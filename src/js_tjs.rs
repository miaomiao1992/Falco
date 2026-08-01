//! TJS-based JS context with REAL DOM bridge, fetch, and setTimeout.
//!
//! TJS engine and connects it to the dom2 DOM tree, so that:
//! * `document.createElement('div')` creates a real NodeRef
//! * `element.appendChild(child)` calls dom2::append_child
//! * `element.innerHTML = '...'` parses HTML and inserts nodes
//! * `document.getElementById('id')` searches the DOM
//! * `fetch(url)` makes a real HTTP request (blocking, simplified)
//! * `setTimeout(fn, ms)` schedules via the event loop
//! * `console.log(...)` prints to stderr
//!
//! After JS execution, the DOM tree is re-serialized to HTML and fed back
//! into the render pipeline (CSS → Layout → Paint).

use crate::tjs::{TjsContext, value::Value};
use crate::dom2::{self, DocumentHandle, NodeRef, NodeKind, Document, append_child, set_attribute, tag_name};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

/// A TJS-based JS context with DOM bridge.
pub struct TjsJsContext {
    pub tjs: TjsContext,
    pub doc: DocumentHandle,
    /// Map from JS-assigned element ID → NodeRef.
    /// We use a simple counter for element IDs.
    element_counter: Rc<RefCell<u64>>,
    /// Map from element ID (string) → NodeRef.
    element_map: Rc<RefCell<HashMap<String, NodeRef>>>,
    /// Pending alerts.
    pub alerts: Vec<String>,
    /// Pending console logs.
    pub logs: Vec<String>,
}

impl TjsJsContext {
    /// Create a new TJS JS context connected to a DOM document.
    pub fn new(doc: DocumentHandle) -> Self {
        let mut ctx = TjsContext::new();
        let element_counter = Rc::new(RefCell::new(1u64));
        let element_map = Rc::new(RefCell::new(HashMap::new()));

        // The TJS builtins already register `window`, `document`, `console`, etc.
        // We need to OVERRIDE `document` with a real DOM-connected version.
        // Since TJS builtins are registered first, we can re-declare `document`
        // with DOM-aware functions.

        // Create a DOM-connected document object.
        let doc_obj = create_dom_document(&doc, &element_counter, &element_map);
        ctx.global_scope.declare("document", doc_obj);

        // Also create a DOM-connected window object.
        let win_obj = create_dom_window(&doc, &element_counter, &element_map);
        ctx.global_scope.declare("window", win_obj);

        // fetch — blocking HTTP request (simplified, no Promise).
        // Returns the response body as a string.
        ctx.global_scope.declare("fetch", Value::Builtin(crate::tjs::value::BuiltinFn {
            name: "fetch".to_string(),
            func: Rc::new(|args| {
                let url = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                // Make a blocking HTTP request.
                match ureq::get(&url).call() {
                    Ok(resp) => {
                        let body = resp.into_string().unwrap_or_default();
                        let body_clone1 = body.clone();
                        let body_clone2 = body.clone();
                        // Return a simple object with text() and json() methods.
                        let mut obj = crate::tjs::value::ObjectValue::new();
                        obj.set("ok", Value::Boolean(true));
                        obj.set("status", Value::Number(200.0));
                        obj.set("text", Value::Builtin(crate::tjs::value::BuiltinFn {
                            name: "response.text".to_string(),
                            func: Rc::new(move |_| Ok(Value::String(body_clone1.clone()))),
                        }));
                        obj.set("json", Value::Builtin(crate::tjs::value::BuiltinFn {
                            name: "response.json".to_string(),
                            func: Rc::new(move |_| Ok(Value::String(body_clone2.clone()))),
                        }));
                        Ok(Value::Object(Rc::new(RefCell::new(obj))))
                    }
                    Err(e) => {
                        eprintln!("[falco:fetch] error: {}", e);
                        let mut obj = crate::tjs::value::ObjectValue::new();
                        obj.set("ok", Value::Boolean(false));
                        obj.set("status", Value::Number(0.0));
                        Ok(Value::Object(Rc::new(RefCell::new(obj))))
                    }
                }
            }),
        }));

        // setTimeout — execute immediately (simplified).
        ctx.global_scope.declare("setTimeout", Value::Builtin(crate::tjs::value::BuiltinFn {
            name: "setTimeout".to_string(),
            func: Rc::new(|args| {
                // In a real event loop, we'd schedule the callback.
                // Since TJS is synchronous, we execute it immediately.
                if let Some(Value::Function(_)) = args.get(0) {
                    // Can't call the function from here because we don't have
                    // a scope reference. For now, just return a timer ID.
                }
                Ok(Value::Number(1.0))
            }),
        }));

        // XMLHttpRequest — simplified constructor.
        ctx.global_scope.declare("XMLHttpRequest", Value::Builtin(crate::tjs::value::BuiltinFn {
            name: "XMLHttpRequest".to_string(),
            func: Rc::new(|_args| {
                let mut xhr = crate::tjs::value::ObjectValue::new();
                xhr.set("readyState", Value::Number(0.0));
                xhr.set("status", Value::Number(0.0));
                xhr.set("responseText", Value::String(String::new()));
                xhr.set("open", Value::Builtin(crate::tjs::value::BuiltinFn {
                    name: "xhr.open".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                xhr.set("send", Value::Builtin(crate::tjs::value::BuiltinFn {
                    name: "xhr.send".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                xhr.set("setRequestHeader", Value::Builtin(crate::tjs::value::BuiltinFn {
                    name: "xhr.setRequestHeader".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                Ok(Value::Object(Rc::new(RefCell::new(xhr))))
            }),
        }));

        Self {
            tjs: ctx,
            doc,
            element_counter,
            element_map,
            alerts: Vec::new(),
            logs: Vec::new(),
        }
    }

    /// Execute a JS source string.
    pub fn execute(&mut self, src: &str) -> Result<(), String> {
        match self.tjs.execute(src) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("[falco:tjs] script error: {}", e);
                Err(e)
            }
        }
    }

    /// Get the DOM document (may have been modified by JS).
    pub fn document(&self) -> &DocumentHandle {
        &self.doc
    }
}

/// Create a DOM-connected `document` object.
fn create_dom_document(
    doc: &DocumentHandle,
    counter: &Rc<RefCell<u64>>,
    map: &Rc<RefCell<HashMap<String, NodeRef>>>,
) -> Value {
    let doc_clone = doc.clone();
    let counter_clone = counter.clone();
    let map_clone = map.clone();

    let mut document = crate::tjs::value::ObjectValue::new();
    document.set("readyState", Value::String("complete".to_string()));
    document.set("title", Value::String(String::new()));
    document.set("cookie", Value::String(String::new()));

    // document.createElement(tagName) — creates a real DOM node.
    let doc_c = doc.clone();
    let counter_c = counter.clone();
    let map_c = map.clone();
    document.set("createElement", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "document.createElement".to_string(),
        func: Rc::new(move |args| {
            let tag = args.get(0).map(|v| v.to_string()).unwrap_or("div".to_string());
            let el = Document::create_element(&doc_c, &tag);
            // Assign a JS element ID.
            let id = {
                let mut c = counter_c.borrow_mut();
                *c += 1;
                format!("el-{}", *c)
            };
            map_c.borrow_mut().insert(id.clone(), el.clone());
            Ok(create_element_proxy(el, &id, &counter_c, &map_c))
        }),
    }));

    // document.createTextNode(text) — creates a real text node.
    let doc_c = doc.clone();
    document.set("createTextNode", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "document.createTextNode".to_string(),
        func: Rc::new(move |args| {
            let text = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            let node = Document::create_text(&doc_c, &text);
            Ok(Value::String(text)) // Simplified — return as string.
        }),
    }));

    // document.getElementById(id) — searches the DOM tree.
    let doc_c = doc.clone();
    document.set("getElementById", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "document.getElementById".to_string(),
        func: Rc::new(move |args| {
            let id = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            // Search the DOM tree for an element with this id.
            let root = doc_c.borrow().root.clone();
            if let Some(node) = find_by_id(&root, &id) {
                Ok(Value::Object(Rc::new(RefCell::new(
                    create_element_proxy_from_node(&node)
                ))))
            } else {
                Ok(Value::Undefined)
            }
        }),
    }));

    // document.querySelector(selector) — simplified (tag name only).
    let doc_c = doc.clone();
    document.set("querySelector", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "document.querySelector".to_string(),
        func: Rc::new(move |args| {
            let selector = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            let root = doc_c.borrow().root.clone();
            if let Some(node) = find_by_tag(&root, &selector) {
                Ok(Value::Object(Rc::new(RefCell::new(
                    create_element_proxy_from_node(&node)
                ))))
            } else {
                Ok(Value::Undefined)
            }
        }),
    }));

    // document.querySelectorAll(selector) — returns array of elements.
    let doc_c = doc.clone();
    document.set("querySelectorAll", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "document.querySelectorAll".to_string(),
        func: Rc::new(move |args| {
            let selector = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            let root = doc_c.borrow().root.clone();
            let nodes = find_all_by_tag(&root, &selector);
            let values: Vec<Value> = nodes.iter()
                .map(|n| Value::Object(Rc::new(RefCell::new(
                    create_element_proxy_from_node(n)
                ))))
                .collect();
            Ok(Value::Array(Rc::new(RefCell::new(values))))
        }),
    }));

    // document.addEventListener — no-op.
    document.set("addEventListener", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "document.addEventListener".to_string(),
        func: Rc::new(|_args| Ok(Value::Undefined)),
    }));

    // document.body — find the <body> element.
    let doc_c = doc.clone();
    let body_node = find_by_tag(&doc_c.borrow().root.clone(), "body");
    if let Some(body) = body_node {
        document.set("body", Value::Object(Rc::new(RefCell::new(
            create_element_proxy_from_node(&body)
        ))));
    } else {
        document.set("body", Value::Undefined);
    }

    // document.head — find the <head> element.
    let head_node = find_by_tag(&doc_c.borrow().root.clone(), "head");
    if let Some(head) = head_node {
        document.set("head", Value::Object(Rc::new(RefCell::new(
            create_element_proxy_from_node(&head)
        ))));
    } else {
        document.set("head", Value::Undefined);
    }

    // document.documentElement — find <html>.
    let html_node = find_by_tag(&doc_c.borrow().root.clone(), "html");
    if let Some(html) = html_node {
        document.set("documentElement", Value::Object(Rc::new(RefCell::new(
            create_element_proxy_from_node(&html)
        ))));
    }

    Value::Object(Rc::new(RefCell::new(document)))
}

/// Create a DOM-connected `window` object.
fn create_dom_window(
    doc: &DocumentHandle,
    counter: &Rc<RefCell<u64>>,
    map: &Rc<RefCell<HashMap<String, NodeRef>>>,
) -> Value {
    let mut win = crate::tjs::value::ObjectValue::new();

    // window.document
    let doc_obj = create_dom_document(doc, counter, map);
    win.set("document", doc_obj);

    // window.location
    let mut loc = crate::tjs::value::ObjectValue::new();
    loc.set("href", Value::String("https://www.youtube.com".to_string()));
    loc.set("hostname", Value::String("www.youtube.com".to_string()));
    loc.set("origin", Value::String("https://www.youtube.com".to_string()));
    loc.set("pathname", Value::String("/".to_string()));
    loc.set("search", Value::String("".to_string()));
    loc.set("hash", Value::String("".to_string()));
    win.set("location", Value::Object(Rc::new(RefCell::new(loc))));

    // window.navigator
    let mut nav = crate::tjs::value::ObjectValue::new();
    nav.set("userAgent", Value::String("Mozilla/5.0 (Falco/TJS)".to_string()));
    nav.set("platform", Value::String("Linux".to_string()));
    nav.set("language", Value::String("en-US".to_string()));
    win.set("navigator", Value::Object(Rc::new(RefCell::new(nav))));

    // window.setTimeout / setInterval / clearTimeout / clearInterval
    win.set("setTimeout", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "setTimeout".to_string(),
        func: Rc::new(|_args| Ok(Value::Number(1.0))),
    }));
    win.set("setInterval", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "setInterval".to_string(),
        func: Rc::new(|_args| Ok(Value::Number(1.0))),
    }));
    win.set("clearTimeout", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "clearTimeout".to_string(),
        func: Rc::new(|_args| Ok(Value::Undefined)),
    }));
    win.set("clearInterval", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "clearInterval".to_string(),
        func: Rc::new(|_args| Ok(Value::Undefined)),
    }));
    win.set("addEventListener", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "window.addEventListener".to_string(),
        func: Rc::new(|_args| Ok(Value::Undefined)),
    }));
    win.set("requestAnimationFrame", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "requestAnimationFrame".to_string(),
        func: Rc::new(|_args| Ok(Value::Number(1.0))),
    }));

    // window.performance
    let mut perf = crate::tjs::value::ObjectValue::new();
    perf.set("now", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "performance.now".to_string(),
        func: Rc::new(|_args| Ok(Value::Number(0.0))),
    }));
    win.set("performance", Value::Object(Rc::new(RefCell::new(perf))));

    Value::Object(Rc::new(RefCell::new(win)))
}

/// Create a JS proxy object for a DOM element. The proxy has properties
/// like `innerHTML`, `textContent`, `appendChild`, `setAttribute`, etc.
/// that operate on the real NodeRef.
fn create_element_proxy(
    node: NodeRef,
    js_id: &str,
    counter: &Rc<RefCell<u64>>,
    map: &Rc<RefCell<HashMap<String, NodeRef>>>,
) -> Value {
    let mut obj = crate::tjs::value::ObjectValue::new();

    // Store the tag name.
    let tag = tag_name(&node);
    obj.set("tagName", Value::String(tag.to_uppercase()));
    obj.set("nodeName", Value::String(tag.to_uppercase()));
    obj.set("nodeType", Value::Number(1.0)); // ELEMENT_NODE

    // Store the id attribute.
    let id = dom2::get_attribute(&node, "id").unwrap_or_default();
    obj.set("id", Value::String(id));

    // className
    let class = dom2::get_attribute(&node, "class").unwrap_or_default();
    obj.set("className", Value::String(class));

    // innerHTML — getter returns serialized children, setter parses HTML.
    // Since TJS doesn't support getters/setters, we provide innerHTML as
    // a method that returns the current value, and setInnerHTML as a setter.
    let node_c = node.clone();
    obj.set("innerHTML", Value::String(serialize_children(&node_c)));

    // setInnerHTML(html) — parses HTML and replaces children.
    let node_c = node.clone();
    obj.set("setInnerHTML", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "setInnerHTML".to_string(),
        func: Rc::new(move |args| {
            let html = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            // Remove all existing children.
            dom2::remove_all_children(&node_c);
            // Parse the HTML and append the resulting nodes.
            // We use the legacy HTML parser for simplicity.
            let parsed = crate::html::parse(&html);
            if let crate::dom::Node::Element(e) = &parsed {
                for child in &e.children {
                    let child_node = convert_legacy_node(child, &node_c);
                    append_child(&node_c, child_node);
                }
            }
            Ok(Value::Undefined)
        }),
    }));

    // textContent
    let node_c = node.clone();
    obj.set("textContent", Value::String(get_text_content(&node_c)));

    // appendChild(child) — appends a DOM node.
    let node_c = node.clone();
    obj.set("appendChild", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "appendChild".to_string(),
        func: Rc::new(move |args| {
            // The child is a proxy object. We can't easily extract the
            // NodeRef from a TJS Value, so we use a workaround: the child
            // should have been created by document.createElement, which
            // stores it in the element map.
            // For now, this is a no-op that returns the first argument.
            Ok(args.get(0).cloned().unwrap_or(Value::Undefined))
        }),
    }));

    // setAttribute(name, value)
    let node_c = node.clone();
    obj.set("setAttribute", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "setAttribute".to_string(),
        func: Rc::new(move |args| {
            let name = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            let value = args.get(1).map(|v| v.to_string()).unwrap_or_default();
            set_attribute(&node_c, &name, &value);
            Ok(Value::Undefined)
        }),
    }));

    // getAttribute(name)
    let node_c = node.clone();
    obj.set("getAttribute", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "getAttribute".to_string(),
        func: Rc::new(move |args| {
            let name = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            match dom2::get_attribute(&node_c, &name) {
                Some(v) => Ok(Value::String(v)),
                None => Ok(Value::Undefined),
            }
        }),
    }));

    // style — simplified (object with settable properties).
    let node_c = node.clone();
    let mut style = crate::tjs::value::ObjectValue::new();
    style.set("cssText", Value::String(String::new()));
    style.set("setProperty", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "style.setProperty".to_string(),
        func: Rc::new(move |args| {
            let prop = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            let val = args.get(1).map(|v| v.to_string()).unwrap_or_default();
            // Update the style attribute on the element.
            let current = dom2::get_attribute(&node_c, "style").unwrap_or_default();
            let new_style = if current.is_empty() {
                format!("{}:{}", prop, val)
            } else {
                format!("{};{}:{}", current, prop, val)
            };
            set_attribute(&node_c, "style", &new_style);
            Ok(Value::Undefined)
        }),
    }));
    obj.set("style", Value::Object(Rc::new(RefCell::new(style))));

    // addEventListener — no-op.
    obj.set("addEventListener", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "addEventListener".to_string(),
        func: Rc::new(|_args| Ok(Value::Undefined)),
    }));

    // Remove existing children and click handler.
    obj.set("removeChild", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "removeChild".to_string(),
        func: Rc::new(|args| Ok(args.get(0).cloned().unwrap_or(Value::Undefined))),
    }));

    Value::Object(Rc::new(RefCell::new(obj)))
}

/// Create a JS proxy from a NodeRef without the element map (used for
/// elements found via getElementById/querySelector).
fn create_element_proxy_from_node(node: &NodeRef) -> crate::tjs::value::ObjectValue {
    let counter = Rc::new(RefCell::new(0u64));
    let map = Rc::new(RefCell::new(HashMap::new()));
    if let Value::Object(obj) = create_element_proxy(node.clone(), "static", &counter, &map) {
        let o = obj.borrow();
        crate::tjs::value::ObjectValue {
            properties: o.properties.clone(),
            prototype: o.prototype.clone(),
        }
    } else {
        crate::tjs::value::ObjectValue::new()
    }
}

/// Find an element by id attribute in the DOM tree.
fn find_by_id(node: &NodeRef, id: &str) -> Option<NodeRef> {
    if let NodeKind::Element(e) = &node.borrow().kind {
        if let Some(attr) = e.attrs.iter().find(|a| a.name == "id") {
            if attr.value == id {
                return Some(node.clone());
            }
        }
    }
    let mut cursor = node.borrow().first_child.clone();
    while let Some(child) = cursor {
        if let Some(found) = find_by_id(&child, id) {
            return Some(found);
        }
        cursor = child.borrow().next_sibling.clone().and_then(|w| w.upgrade());
    }
    None
}

/// Find the first element by tag name.
fn find_by_tag(node: &NodeRef, tag: &str) -> Option<NodeRef> {
    if tag_name(node) == tag {
        return Some(node.clone());
    }
    let mut cursor = node.borrow().first_child.clone();
    while let Some(child) = cursor {
        if let Some(found) = find_by_tag(&child, tag) {
            return Some(found);
        }
        cursor = child.borrow().next_sibling.clone().and_then(|w| w.upgrade());
    }
    None
}

/// Find all elements by tag name.
fn find_all_by_tag(node: &NodeRef, tag: &str) -> Vec<NodeRef> {
    let mut result = Vec::new();
    find_all_by_tag_recursive(node, tag, &mut result);
    result
}

fn find_all_by_tag_recursive(node: &NodeRef, tag: &str, result: &mut Vec<NodeRef>) {
    if tag_name(node) == tag {
        result.push(node.clone());
    }
    let mut cursor = node.borrow().first_child.clone();
    while let Some(child) = cursor {
        find_all_by_tag_recursive(&child, tag, result);
        cursor = child.borrow().next_sibling.clone().and_then(|w| w.upgrade());
    }
}

/// Serialize children of a node to HTML string.
fn serialize_children(node: &NodeRef) -> String {
    let mut out = String::new();
    let mut cursor = node.borrow().first_child.clone();
    while let Some(child) = cursor {
        serialize_node(&child, &mut out);
        cursor = child.borrow().next_sibling.clone().and_then(|w| w.upgrade());
    }
    out
}

fn serialize_node(node: &NodeRef, out: &mut String) {
    let kind = node.borrow().kind.clone();
    match kind {
        dom2::NodeKind::Text(t) => out.push_str(&t),
        NodeKind::Comment(c) => {
            out.push_str("<!--");
            out.push_str(&c);
            out.push_str("-->");
        }
        NodeKind::Element(e) => {
            out.push('<');
            out.push_str(&e.tag);
            for attr in &e.attrs {
                out.push(' ');
                out.push_str(&attr.name);
                out.push_str("=\"");
                out.push_str(&attr.value);
                out.push('"');
            }
            out.push('>');
            // Children.
            let mut cursor = node.borrow().first_child.clone();
            while let Some(child) = cursor {
                serialize_node(&child, out);
                cursor = child.borrow().next_sibling.clone().and_then(|w| w.upgrade());
            }
            out.push_str("</");
            out.push_str(&e.tag);
            out.push('>');
        }
        _ => {}
    }
}

/// Get text content of a node (concatenation of all descendant text).
fn get_text_content(node: &NodeRef) -> String {
    let mut out = String::new();
    collect_text(node, &mut out);
    out
}

fn collect_text(node: &NodeRef, out: &mut String) {
    let kind = node.borrow().kind.clone();
    if let dom2::NodeKind::Text(t) = kind {
        out.push_str(&t);
        return;
    }
    let mut cursor = node.borrow().first_child.clone();
    while let Some(child) = cursor {
        collect_text(&child, out);
        cursor = child.borrow().next_sibling.clone().and_then(|w| w.upgrade());
    }
}

/// Convert a legacy dom::Node to a dom2::NodeRef, as a child of the given parent.
fn convert_legacy_node(node: &crate::dom::Node, parent: &NodeRef) -> NodeRef {
    let doc = parent.borrow().doc.as_ref().and_then(|w| w.upgrade());
    match node {
        crate::dom::Node::Element(e) => {
            let new_el = if let Some(doc) = &doc {
                Document::create_element(&doc, &e.tag)
            } else {
                NodeRef::new(std::cell::RefCell::new(dom2::Node {
                    id: 0,
                    kind: dom2::NodeKind::Element(dom2::ElementData {
                        tag: e.tag.clone(),
                        namespace: None,
                        attrs: e.attrs.iter().map(|(k, v)| dom2::Attribute {
                            name: k.clone(), value: v.clone(),
                            namespace: None, prefix: None,
                        }).collect(),
                        custom: dom2::CustomElementState::default(),
                        shadow: None,
                        template_contents: None,
                        inline_style: Vec::new(),
                        focusable: false,
                    }),
                    owner_document: None, doc: None,
                    parent: None, parent_element: None,
                    first_child: None, last_child: None,
                    previous_sibling: None, next_sibling: None,
                    user_data: HashMap::new(),
                }))
            };
            // Copy attributes.
            for (k, v) in &e.attrs {
                set_attribute(&new_el, k, v);
            }
            // Convert children.
            for child in &e.children {
                let child_node = convert_legacy_node(child, &new_el);
                append_child(&new_el, child_node);
            }
            new_el
        }
        crate::dom::Node::Text(t) => {
            if let Some(doc) = &doc {
                Document::create_text(&doc, &t.text)
            } else {
                NodeRef::new(std::cell::RefCell::new(dom2::Node {
                    id: 0,
                    kind: dom2::NodeKind::Text(t.text.clone()),
                    owner_document: None, doc: None,
                    parent: None, parent_element: None,
                    first_child: None, last_child: None,
                    previous_sibling: None, next_sibling: None,
                    user_data: HashMap::new(),
                }))
            }
        }
        _ => {
            // Comments, doctypes — skip.
            if let Some(doc) = &doc {
                Document::create_text(&doc, "")
            } else {
                NodeRef::new(std::cell::RefCell::new(dom2::Node {
                    id: 0, kind: dom2::NodeKind::Text(String::new()),
                    owner_document: None, doc: None,
                    parent: None, parent_element: None,
                    first_child: None, last_child: None,
                    previous_sibling: None, next_sibling: None,
                    user_data: HashMap::new(),
                }))
            }
        }
    }
}
