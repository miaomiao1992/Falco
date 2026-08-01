//! TJS Builtins — Math, console, Date, JSON, Array, Object, parseInt, etc.

use crate::tjs::value::{Value, BuiltinFn, ObjectValue};
use crate::tjs::interpreter::Scope;
use std::rc::Rc;
use std::cell::RefCell;

pub fn register(scope: &mut Scope) {
    // console.log
    scope.declare("console", Value::Object(Rc::new(RefCell::new({
        let mut obj = ObjectValue::new();
        obj.set("log", Value::Builtin(BuiltinFn {
            name: "console.log".to_string(),
            func: Rc::new(|args| {
                let msg: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                eprintln!("[tjs] {}", msg.join(" "));
                Ok(Value::Undefined)
            }),
        }));
        obj.set("error", Value::Builtin(BuiltinFn {
            name: "console.error".to_string(),
            func: Rc::new(|args| {
                let msg: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                eprintln!("[tjs:error] {}", msg.join(" "));
                Ok(Value::Undefined)
            }),
        }));
        obj.set("warn", Value::Builtin(BuiltinFn {
            name: "console.warn".to_string(),
            func: Rc::new(|args| {
                let msg: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                eprintln!("[tjs:warn] {}", msg.join(" "));
                Ok(Value::Undefined)
            }),
        }));
        obj
    }))));

    // Math object
    scope.declare("Math", Value::Object(Rc::new(RefCell::new({
        let mut obj = ObjectValue::new();
        obj.set("PI", Value::Number(std::f64::consts::PI));
        obj.set("E", Value::Number(std::f64::consts::E));
        obj.set("LN2", Value::Number(std::f64::consts::LN_2));
        obj.set("LN10", Value::Number(std::f64::consts::LN_10));
        obj.set("SQRT2", Value::Number(std::f64::consts::SQRT_2));
        obj.set("floor", Value::Builtin(BuiltinFn {
            name: "Math.floor".to_string(),
            func: Rc::new(|args| {
                let n = args.get(0).map(|v| v.to_number()).unwrap_or(f64::NAN);
                Ok(Value::Number(n.floor()))
            }),
        }));
        obj.set("ceil", Value::Builtin(BuiltinFn {
            name: "Math.ceil".to_string(),
            func: Rc::new(|args| {
                let n = args.get(0).map(|v| v.to_number()).unwrap_or(f64::NAN);
                Ok(Value::Number(n.ceil()))
            }),
        }));
        obj.set("round", Value::Builtin(BuiltinFn {
            name: "Math.round".to_string(),
            func: Rc::new(|args| {
                let n = args.get(0).map(|v| v.to_number()).unwrap_or(f64::NAN);
                Ok(Value::Number(n.round()))
            }),
        }));
        obj.set("abs", Value::Builtin(BuiltinFn {
            name: "Math.abs".to_string(),
            func: Rc::new(|args| {
                let n = args.get(0).map(|v| v.to_number()).unwrap_or(f64::NAN);
                Ok(Value::Number(n.abs()))
            }),
        }));
        obj.set("sqrt", Value::Builtin(BuiltinFn {
            name: "Math.sqrt".to_string(),
            func: Rc::new(|args| {
                let n = args.get(0).map(|v| v.to_number()).unwrap_or(f64::NAN);
                Ok(Value::Number(n.sqrt()))
            }),
        }));
        obj.set("pow", Value::Builtin(BuiltinFn {
            name: "Math.pow".to_string(),
            func: Rc::new(|args| {
                let x = args.get(0).map(|v| v.to_number()).unwrap_or(f64::NAN);
                let y = args.get(1).map(|v| v.to_number()).unwrap_or(f64::NAN);
                Ok(Value::Number(x.powf(y)))
            }),
        }));
        obj.set("max", Value::Builtin(BuiltinFn {
            name: "Math.max".to_string(),
            func: Rc::new(|args| {
                let max = args.iter().map(|v| v.to_number()).fold(f64::NEG_INFINITY, f64::max);
                Ok(Value::Number(max))
            }),
        }));
        obj.set("min", Value::Builtin(BuiltinFn {
            name: "Math.min".to_string(),
            func: Rc::new(|args| {
                let min = args.iter().map(|v| v.to_number()).fold(f64::INFINITY, f64::min);
                Ok(Value::Number(min))
            }),
        }));
        obj.set("sin", Value::Builtin(BuiltinFn {
            name: "Math.sin".to_string(),
            func: Rc::new(|args| Ok(Value::Number(args.get(0).map(|v| v.to_number()).unwrap_or(0.0).sin()))),
        }));
        obj.set("cos", Value::Builtin(BuiltinFn {
            name: "Math.cos".to_string(),
            func: Rc::new(|args| Ok(Value::Number(args.get(0).map(|v| v.to_number()).unwrap_or(0.0).cos()))),
        }));
        obj.set("tan", Value::Builtin(BuiltinFn {
            name: "Math.tan".to_string(),
            func: Rc::new(|args| Ok(Value::Number(args.get(0).map(|v| v.to_number()).unwrap_or(0.0).tan()))),
        }));
        obj.set("random", Value::Builtin(BuiltinFn {
            name: "Math.random".to_string(),
            func: Rc::new(|_| {
                // Simple LCG random — not crypto-secure, but deterministic.
                use std::sync::atomic::{AtomicU64, Ordering};
                static SEED: AtomicU64 = AtomicU64::new(12345);
                let mut s = SEED.load(Ordering::Relaxed);
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                SEED.store(s, Ordering::Relaxed);
                Ok(Value::Number((s as f64 / u64::MAX as f64)))
            }),
        }));
        obj.set("log", Value::Builtin(BuiltinFn {
            name: "Math.log".to_string(),
            func: Rc::new(|args| Ok(Value::Number(args.get(0).map(|v| v.to_number()).unwrap_or(0.0).ln()))),
        }));
        obj.set("exp", Value::Builtin(BuiltinFn {
            name: "Math.exp".to_string(),
            func: Rc::new(|args| Ok(Value::Number(args.get(0).map(|v| v.to_number()).unwrap_or(0.0).exp()))),
        }));
        obj
    }))));

    // Global functions
    // String() — converts any value to string (used by template literals).
    scope.declare("String", Value::Builtin(BuiltinFn {
        name: "String".to_string(),
        func: Rc::new(|args| {
            Ok(Value::String(args.get(0).map(|v| v.to_string()).unwrap_or_default()))
        }),
    }));

    // parseInt
    scope.declare("parseInt", Value::Builtin(BuiltinFn {
        name: "parseInt".to_string(),
        func: Rc::new(|args| {
            let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            let n = s.trim().parse::<f64>().unwrap_or(f64::NAN);
            Ok(Value::Number(n.trunc()))
        }),
    }));
    scope.declare("parseFloat", Value::Builtin(BuiltinFn {
        name: "parseFloat".to_string(),
        func: Rc::new(|args| {
            let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            Ok(Value::Number(s.trim().parse::<f64>().unwrap_or(f64::NAN)))
        }),
    }));
    scope.declare("isNaN", Value::Builtin(BuiltinFn {
        name: "isNaN".to_string(),
        func: Rc::new(|args| {
            Ok(Value::Boolean(args.get(0).map(|v| v.to_number()).unwrap_or(f64::NAN).is_nan()))
        }),
    }));
    scope.declare("String", Value::Builtin(BuiltinFn {
        name: "String".to_string(),
        func: Rc::new(|args| Ok(Value::String(args.get(0).map(|v| v.to_string()).unwrap_or_default()))),
    }));
    scope.declare("Number", Value::Builtin(BuiltinFn {
        name: "Number".to_string(),
        func: Rc::new(|args| Ok(Value::Number(args.get(0).map(|v| v.to_number()).unwrap_or(0.0)))),
    }));
    scope.declare("Boolean", Value::Builtin(BuiltinFn {
        name: "Boolean".to_string(),
        func: Rc::new(|args| Ok(Value::Boolean(args.get(0).map(|v| v.is_truthy()).unwrap_or(false)))),
    }));
    scope.declare("Array", Value::Builtin(BuiltinFn {
        name: "Array".to_string(),
        func: Rc::new(|args| {
            if args.len() == 1 {
                if let Value::Number(n) = &args[0] {
                    let len = *n as usize;
                    return Ok(Value::Array(Rc::new(RefCell::new(vec![Value::Undefined; len]))));
                }
            }
            Ok(Value::Array(Rc::new(RefCell::new(args))))
        }),
    }));
    scope.declare("Object", Value::Builtin(BuiltinFn {
        name: "Object".to_string(),
        func: Rc::new(|_| Ok(Value::Object(Rc::new(RefCell::new(ObjectValue::new()))))),
    }));
    scope.declare("alert", Value::Builtin(BuiltinFn {
        name: "alert".to_string(),
        func: Rc::new(|args| {
            let msg = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            eprintln!("[tjs:alert] {}", msg);
            Ok(Value::Undefined)
        }),
    }));
    // Date — YouTube uses both `Date.now()` (static) and `(new Date).getTime()` (instance).
    // We make Date an Object with `now` as a static method and `__new__` as
    // a constructor that returns a date instance object.
    scope.declare("Date", Value::Object(Rc::new(RefCell::new({
        let mut d = ObjectValue::new();
        d.set("now", Value::Builtin(BuiltinFn {
            name: "Date.now".to_string(),
            func: Rc::new(|_| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0);
                Ok(Value::Number(ms))
            }),
        }));
        // __new__ is called by the `new` operator when Date is an Object.
        d.set("__new__", Value::Builtin(BuiltinFn {
            name: "Date.__new__".to_string(),
            func: Rc::new(|_args| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0);
                let mut inst = ObjectValue::new();
                inst.set("getTime", Value::Builtin(BuiltinFn {
                    name: "Date.getTime".to_string(),
                    func: Rc::new(move |_| Ok(Value::Number(ms))),
                }));
                Ok(Value::Object(Rc::new(RefCell::new(inst))))
            }),
        }));
        d
    }))));

    // JSON
    scope.declare("JSON", Value::Object(Rc::new(RefCell::new({
        let mut obj = ObjectValue::new();
        obj.set("stringify", Value::Builtin(BuiltinFn {
            name: "JSON.stringify".to_string(),
            func: Rc::new(|args| {
                let v = args.get(0).cloned().unwrap_or(Value::Undefined);
                Ok(Value::String(json_stringify(&v)))
            }),
        }));
        obj.set("parse", Value::Builtin(BuiltinFn {
            name: "JSON.parse".to_string(),
            func: Rc::new(|args| {
                let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                json_parse(&s).map_err(|e| e.to_string())
            }),
        }));
        obj
    }))));
    // Promise — must be callable as `new Promise(executor)` AND have static methods.
    // YouTube uses: `new Promise(res => window.ytAtN = res)`.
    // We make Promise a Builtin (callable) that returns an object.
    scope.declare("Promise", Value::Builtin(crate::tjs::value::BuiltinFn {
        name: "Promise".to_string(),
        func: Rc::new(|args| {
            // `new Promise(executor)` — executor is a function that receives
            // resolve and reject. We call it immediately (synchronous).
            let executor = args.get(0).cloned().unwrap_or(Value::Undefined);
            // Create a "promise" object with then/catch.
            let mut p = crate::tjs::value::ObjectValue::new();
            p.set("then", Value::Builtin(crate::tjs::value::BuiltinFn {
                name: "Promise.then".to_string(),
                func: Rc::new(|args| {
                    // Simplified: call the callback immediately with undefined.
                    Ok(args.get(0).cloned().unwrap_or(Value::Undefined))
                }),
            }));
            p.set("catch", Value::Builtin(crate::tjs::value::BuiltinFn {
                name: "Promise.catch".to_string(),
                func: Rc::new(|args| {
                    Ok(args.get(0).cloned().unwrap_or(Value::Undefined))
                }),
            }));
            p.set("finally", Value::Builtin(crate::tjs::value::BuiltinFn {
                name: "Promise.finally".to_string(),
                func: Rc::new(|_args| Ok(Value::Undefined)),
            }));
            // Also add static methods on the returned object.
            p.set("resolve", Value::Builtin(crate::tjs::value::BuiltinFn {
                name: "Promise.resolve".to_string(),
                func: Rc::new(|args| Ok(args.get(0).cloned().unwrap_or(Value::Undefined))),
            }));
            p.set("reject", Value::Builtin(crate::tjs::value::BuiltinFn {
                name: "Promise.reject".to_string(),
                func: Rc::new(|args| Ok(args.get(0).cloned().unwrap_or(Value::Undefined))),
            }));
            p.set("all", Value::Builtin(crate::tjs::value::BuiltinFn {
                name: "Promise.all".to_string(),
                func: Rc::new(|_args| Ok(Value::Undefined)),
            }));
            p.set("race", Value::Builtin(crate::tjs::value::BuiltinFn {
                name: "Promise.race".to_string(),
                func: Rc::new(|_args| Ok(Value::Undefined)),
            }));
            let promise_val = Value::Object(Rc::new(RefCell::new(p)));
            // Call the executor with resolve/reject stubs.
            if let Value::Function(_) = &executor {
                // We can't call the function from here (no scope), so we skip.
                // In a real impl, we'd call executor(resolve, reject).
            }
            Ok(promise_val)
        }),
    }));

    // Symbol — stub. YouTube checks `typeof Symbol !== 'undefined'`.
    scope.declare("Symbol", Value::Builtin(BuiltinFn {
        name: "Symbol".to_string(),
        func: Rc::new(|args| {
            let desc = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            Ok(Value::String(format!("Symbol({})", desc)))
        }),
    }));

    // Image — stub constructor (YouTube uses `new Image()` for preloading).
    scope.declare("Image", Value::Builtin(BuiltinFn {
        name: "Image".to_string(),
        func: Rc::new(|_args| {
            Ok(Value::Object(Rc::new(RefCell::new(ObjectValue::new()))))
        }),
    }));

    // Polymer — Web Components framework stub (YouTube uses it).
    scope.declare("Polymer", Value::Builtin(BuiltinFn {
        name: "Polymer".to_string(),
        func: Rc::new(|_args| Ok(Value::Undefined)),
    }));

    // window — the global object (browser environment).
    // YouTube's JS starts with `window.WIZ_global_data = {...}`.
    let win = create_window();
    scope.declare("window", win.clone());
    // Also register common globals as top-level (in browsers, `window` IS
    // the global scope, so `setTimeout`, `document`, etc. are accessible
    // without `window.` prefix).
    if let Value::Object(win_obj) = &win {
        let win_ref = win_obj.borrow();
        for (key, val) in win_ref.properties.iter() {
            scope.declare(key, val.clone());
        }
    }
}

fn json_stringify(v: &Value) -> String {
    match v {
        Value::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e21 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Value::Boolean(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Undefined => "null".to_string(),
        Value::Object(obj) => {
            let obj = obj.borrow();
            let pairs: Vec<String> = obj.properties.iter()
                .map(|(k, v)| format!("\"{}\":{}", k, json_stringify(v)))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        Value::Array(arr) => {
            let arr = arr.borrow();
            let items: Vec<String> = arr.iter().map(json_stringify).collect();
            format!("[{}]", items.join(","))
        }
        _ => "null".to_string(),
    }
}

fn json_parse(s: &str) -> Result<Value, String> {
    let s = s.trim();
    if s.is_empty() { return Err("empty JSON".to_string()); }
    if s == "null" { return Ok(Value::Null); }
    if s == "true" { return Ok(Value::Boolean(true)); }
    if s == "false" { return Ok(Value::Boolean(false)); }
    if s.starts_with('"') && s.ends_with('"') {
        return Ok(Value::String(s[1..s.len()-1].to_string()));
    }
    if let Ok(n) = s.parse::<f64>() {
        return Ok(Value::Number(n));
    }
    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len()-1];
        let items: Vec<Value> = if inner.trim().is_empty() {
            Vec::new()
        } else {
            inner.split(',').map(|item| json_parse(item.trim()).unwrap_or(Value::Null)).collect()
        };
        return Ok(Value::Array(Rc::new(RefCell::new(items))));
    }
    if s.starts_with('{') && s.ends_with('}') {
        let inner = &s[1..s.len()-1];
        let mut obj = ObjectValue::new();
        // Very simplified JSON object parsing.
        for pair in inner.split(',') {
            let parts: Vec<&str> = pair.splitn(2, ':').collect();
            if parts.len() == 2 {
                let key = parts[0].trim().trim_matches('"');
                let val = json_parse(parts[1].trim()).unwrap_or(Value::Null);
                obj.set(key, val);
            }
        }
        return Ok(Value::Object(Rc::new(RefCell::new(obj))));
    }
    Err(format!("cannot parse JSON: {}", s))
}

/// Create a `window` global object — the global scope in browsers.
/// YouTube's JS starts with `window.WIZ_global_data = {...}` so we need
/// `window` to be an object that can have properties set on it.
fn create_window() -> Value {
    let mut win = ObjectValue::new();
    // window references itself (window.window === window).
    // We can't set it yet because we need the Rc — we'll set it after creation.
    // For now, add common browser globals as stubs.
    win.set("location", Value::Object(Rc::new(RefCell::new({
        let mut loc = ObjectValue::new();
        loc.set("href", Value::String("https://www.youtube.com".to_string()));
        loc.set("hostname", Value::String("www.youtube.com".to_string()));
        loc.set("origin", Value::String("https://www.youtube.com".to_string()));
        loc.set("pathname", Value::String("/".to_string()));
        loc.set("search", Value::String("".to_string()));
        loc.set("hash", Value::String("".to_string()));
        loc
    }))));
    win.set("navigator", Value::Object(Rc::new(RefCell::new({
        let mut nav = ObjectValue::new();
        nav.set("userAgent", Value::String("Mozilla/5.0 (Falco/TJS)".to_string()));
        nav.set("platform", Value::String("Linux".to_string()));
        nav.set("language", Value::String("en-US".to_string()));
        nav
    }))));
    win.set("document", Value::Object(Rc::new(RefCell::new({
        let mut doc = ObjectValue::new();
        doc.set("readyState", Value::String("complete".to_string()));
        doc.set("title", Value::String("".to_string()));
        doc.set("cookie", Value::String("".to_string()));
        // document.createElement — returns an element-like object with
        // innerHTML setter, appendChild, setAttribute, etc.
        doc.set("createElement", Value::Builtin(BuiltinFn {
            name: "document.createElement".to_string(),
            func: Rc::new(|args| {
                let tag = args.get(0).map(|v| v.to_string()).unwrap_or("div".to_string());
                let mut el = ObjectValue::new();
                el.set("tagName", Value::String(tag.to_uppercase()));
                el.set("nodeName", Value::String(tag.to_uppercase()));
                el.set("nodeType", Value::Number(1.0));
                el.set("id", Value::String(String::new()));
                el.set("className", Value::String(String::new()));
                el.set("innerHTML", Value::String(String::new()));
                el.set("textContent", Value::String(String::new()));
                el.set("style", Value::Object(Rc::new(RefCell::new(ObjectValue::new()))));
                el.set("setAttribute", Value::Builtin(BuiltinFn {
                    name: "setAttribute".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                el.set("getAttribute", Value::Builtin(BuiltinFn {
                    name: "getAttribute".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                el.set("appendChild", Value::Builtin(BuiltinFn {
                    name: "appendChild".to_string(),
                    func: Rc::new(|args| Ok(args.get(0).cloned().unwrap_or(Value::Undefined))),
                }));
                el.set("removeChild", Value::Builtin(BuiltinFn {
                    name: "removeChild".to_string(),
                    func: Rc::new(|args| Ok(args.get(0).cloned().unwrap_or(Value::Undefined))),
                }));
                // addEventListener — stores the callback but doesn't fire
                // (no real event loop in TJS). At least doesn't crash.
                el.set("addEventListener", Value::Builtin(BuiltinFn {
                    name: "addEventListener".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                el.set("removeEventListener", Value::Builtin(BuiltinFn {
                    name: "removeEventListener".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                // dispatchEvent — no-op.
                el.set("dispatchEvent", Value::Builtin(BuiltinFn {
                    name: "dispatchEvent".to_string(),
                    func: Rc::new(|_args| Ok(Value::Boolean(true))),
                }));
                // click() — triggers onclick if set.
                el.set("click", Value::Builtin(BuiltinFn {
                    name: "click".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                // focus() / blur() — no-op.
                el.set("focus", Value::Builtin(BuiltinFn {
                    name: "focus".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                el.set("blur", Value::Builtin(BuiltinFn {
                    name: "blur".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                // contains() — simplified.
                el.set("contains", Value::Builtin(BuiltinFn {
                    name: "contains".to_string(),
                    func: Rc::new(|_args| Ok(Value::Boolean(false))),
                }));
                // querySelector / querySelectorAll on elements.
                el.set("querySelector", Value::Builtin(BuiltinFn {
                    name: "querySelector".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                el.set("querySelectorAll", Value::Builtin(BuiltinFn {
                    name: "querySelectorAll".to_string(),
                    func: Rc::new(|_args| Ok(Value::Array(Rc::new(RefCell::new(Vec::new()))))),
                }));
                // getElementsByClassName / getElementsByTagName.
                el.set("getElementsByClassName", Value::Builtin(BuiltinFn {
                    name: "getElementsByClassName".to_string(),
                    func: Rc::new(|_args| Ok(Value::Array(Rc::new(RefCell::new(Vec::new()))))),
                }));
                el.set("getElementsByTagName", Value::Builtin(BuiltinFn {
                    name: "getElementsByTagName".to_string(),
                    func: Rc::new(|_args| Ok(Value::Array(Rc::new(RefCell::new(Vec::new()))))),
                }));
                // cloneNode.
                el.set("cloneNode", Value::Builtin(BuiltinFn {
                    name: "cloneNode".to_string(),
                    func: Rc::new(|_args| {
                        Ok(Value::Object(Rc::new(RefCell::new(ObjectValue::new()))))
                    }),
                }));
                el.set("removeEventListener", Value::Builtin(BuiltinFn {
                    name: "removeEventListener".to_string(),
                    func: Rc::new(|_args| Ok(Value::Undefined)),
                }));
                Ok(Value::Object(Rc::new(RefCell::new(el))))
            }),
        }));
        // document.createTextNode — returns a text node.
        doc.set("createTextNode", Value::Builtin(BuiltinFn {
            name: "document.createTextNode".to_string(),
            func: Rc::new(|args| {
                let text = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                let mut node = ObjectValue::new();
                node.set("nodeType", Value::Number(3.0));
                node.set("textContent", Value::String(text));
                Ok(Value::Object(Rc::new(RefCell::new(node))))
            }),
        }));
        // document.getElementById — stub returns undefined.
        doc.set("getElementById", Value::Builtin(BuiltinFn {
            name: "document.getElementById".to_string(),
            func: Rc::new(|_args| Ok(Value::Undefined)),
        }));
        // document.querySelector — stub.
        doc.set("querySelector", Value::Builtin(BuiltinFn {
            name: "document.querySelector".to_string(),
            func: Rc::new(|_args| Ok(Value::Undefined)),
        }));
        // document.querySelectorAll — returns empty array.
        doc.set("querySelectorAll", Value::Builtin(BuiltinFn {
            name: "document.querySelectorAll".to_string(),
            func: Rc::new(|_args| Ok(Value::Array(Rc::new(RefCell::new(Vec::new()))))),
        }));
        // document.addEventListener — no-op.
        doc.set("addEventListener", Value::Builtin(BuiltinFn {
            name: "document.addEventListener".to_string(),
            func: Rc::new(|_args| Ok(Value::Undefined)),
        }));
        // document.body — an element-like object.
        let mut body = ObjectValue::new();
        body.set("tagName", Value::String("BODY".to_string()));
        body.set("appendChild", Value::Builtin(BuiltinFn {
            name: "body.appendChild".to_string(),
            func: Rc::new(|args| Ok(args.get(0).cloned().unwrap_or(Value::Undefined))),
        }));
        body.set("innerHTML", Value::String(String::new()));
        doc.set("body", Value::Object(Rc::new(RefCell::new(body))));
        // document.head — an element-like object.
        let mut head = ObjectValue::new();
        head.set("tagName", Value::String("HEAD".to_string()));
        doc.set("head", Value::Object(Rc::new(RefCell::new(head))));
        // document.documentElement.
        let mut html_el = ObjectValue::new();
        html_el.set("tagName", Value::String("HTML".to_string()));
        doc.set("documentElement", Value::Object(Rc::new(RefCell::new(html_el))));
        doc
    }))));
    // window.setTimeout — stub (would need event loop integration).
    win.set("setTimeout", Value::Builtin(BuiltinFn {
        name: "setTimeout".to_string(),
        func: Rc::new(|_args| Ok(Value::Number(0.0))),
    }));
    // window.setInterval — stub.
    win.set("setInterval", Value::Builtin(BuiltinFn {
        name: "setInterval".to_string(),
        func: Rc::new(|_args| Ok(Value::Number(0.0))),
    }));
    // window.clearTimeout / clearInterval — stubs.
    win.set("clearTimeout", Value::Builtin(BuiltinFn {
        name: "clearTimeout".to_string(),
        func: Rc::new(|_args| Ok(Value::Undefined)),
    }));
    win.set("clearInterval", Value::Builtin(BuiltinFn {
        name: "clearInterval".to_string(),
        func: Rc::new(|_args| Ok(Value::Undefined)),
    }));
    // window.addEventListener — no-op.
    win.set("addEventListener", Value::Builtin(BuiltinFn {
        name: "window.addEventListener".to_string(),
        func: Rc::new(|_args| Ok(Value::Undefined)),
    }));
    // window.requestAnimationFrame — YouTube calls this.
    win.set("requestAnimationFrame", Value::Builtin(BuiltinFn {
        name: "requestAnimationFrame".to_string(),
        func: Rc::new(|_args| Ok(Value::Number(1.0))),
    }));
    win.set("cancelAnimationFrame", Value::Builtin(BuiltinFn {
        name: "cancelAnimationFrame".to_string(),
        func: Rc::new(|_args| Ok(Value::Undefined)),
    }));
    // window.performance.now() — returns 0.
    win.set("performance", Value::Object(Rc::new(RefCell::new({
        let mut perf = ObjectValue::new();
        perf.set("now", Value::Builtin(BuiltinFn {
            name: "performance.now".to_string(),
            func: Rc::new(|_args| Ok(Value::Number(0.0))),
        }));
        perf
    }))));
    Value::Object(Rc::new(RefCell::new(win)))
}
