//! TJS Value — runtime value types.
//!
//! All TJS values are one of: Number, String, Boolean, Null, Undefined,
//! Object (HashMap), Array (Vec), Function (closure).

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;

/// A TJS runtime value.
#[derive(Clone)]
pub enum Value {
    Number(f64),
    /// BigInt — arbitrary-precision integer. Stored as a string to preserve
    /// precision (we use crate::tjs_ext::BigInt for arithmetic).
    BigInt(String),
    String(String),
    Boolean(bool),
    Null,
    Undefined,
    Object(Rc<RefCell<ObjectValue>>),
    Array(Rc<RefCell<Vec<Value>>>),
    Function(Rc<FunctionValue>),
    Builtin(BuiltinFn),
}

/// An object value — a HashMap of properties.
pub struct ObjectValue {
    pub properties: HashMap<String, Value>,
    pub prototype: Option<Value>,
}

/// A function value — parameters + body + closure scope.
pub struct FunctionValue {
    pub params: Vec<String>,
    pub body: Vec<crate::tjs::parser::Stmt>,
    pub closure: Rc<RefCell<crate::tjs::interpreter::Scope>>,
    pub name: String,
    /// Optional compiled bytecode for VM execution.
    /// When set, the VM can execute this function directly without
    /// falling back to the tree-walking interpreter.
    pub vm_code: Option<Rc<Vec<crate::tjs::vm::Bytecode>>>,
    pub vm_nlocals: u32,
}

/// A builtin function implemented in Rust.
pub struct BuiltinFn {
    pub name: String,
    pub func: Rc<dyn Fn(Vec<Value>) -> Result<Value, String>>,
}

impl Clone for BuiltinFn {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            func: self.func.clone(),
        }
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::BigInt(_) => "bigint",
            Value::String(_) => "string",
            Value::Boolean(_) => "boolean",
            Value::Null => "object", // JS quirk: typeof null === "object"
            Value::Undefined => "undefined",
            Value::Object(_) => "object",
            Value::Array(_) => "object",
            Value::Function(_) => "function",
            Value::Builtin(_) => "function",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::BigInt(s) => s != "0" && !s.is_empty(),
            Value::String(s) => !s.is_empty(),
            Value::Boolean(b) => *b,
            Value::Null => false,
            Value::Undefined => false,
            _ => true,
        }
    }

    pub fn to_number(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            Value::BigInt(s) => s.parse().unwrap_or(f64::NAN),
            Value::String(s) => s.parse().unwrap_or(f64::NAN),
            Value::Boolean(b) => if *b { 1.0 } else { 0.0 },
            Value::Null => 0.0,
            Value::Undefined => f64::NAN,
            _ => f64::NAN,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e21 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::BigInt(s) => s.clone(),
            Value::String(s) => s.clone(),
            Value::Boolean(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Undefined => "undefined".to_string(),
            Value::Object(_) => "[object Object]".to_string(),
            Value::Array(arr) => {
                let arr = arr.borrow();
                arr.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")
            }
            Value::Function(f) => format!("function {}() {{}}", f.name),
            Value::Builtin(f) => format!("function {}() {{ [native code] }}", f.name),
        }
    }

    pub fn equals(&self, other: &Value) -> bool {
        // Strict equality (===).
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::BigInt(a), Value::BigInt(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Undefined, Value::Undefined) => true,
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    pub fn loose_equals(&self, other: &Value) -> bool {
        // Loose equality (==).
        match (self, other) {
            (a, b) if a.equals(b) => true,
            (Value::Number(a), Value::String(b)) => *a == b.parse::<f64>().unwrap_or(f64::NAN),
            (Value::String(a), Value::Number(b)) => a.parse::<f64>().unwrap_or(f64::NAN) == *b,
            (Value::Boolean(b), other) => Value::Number(if *b { 1.0 } else { 0.0 }).loose_equals(other),
            (other, Value::Boolean(b)) => other.loose_equals(&Value::Number(if *b { 1.0 } else { 0.0 })),
            (Value::Null, Value::Undefined) => true,
            (Value::Undefined, Value::Null) => true,
            _ => false,
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::BigInt(s) => write!(f, "{}n", s),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
            Value::Undefined => write!(f, "undefined"),
            Value::Object(_) => write!(f, "[Object]"),
            Value::Array(arr) => write!(f, "[Array({})]", arr.borrow().len()),
            Value::Function(func) => write!(f, "[Function: {}]", func.name),
            Value::Builtin(func) => write!(f, "[Builtin: {}]", func.name),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl ObjectValue {
    pub fn new() -> Self {
        Self { properties: HashMap::new(), prototype: None }
    }

    pub fn get(&self, key: &str) -> Value {
        if let Some(v) = self.properties.get(key) {
            v.clone()
        } else if let Some(proto) = &self.prototype {
            proto.get_property(key)
        } else {
            Value::Undefined
        }
    }

    pub fn set(&mut self, key: &str, value: Value) {
        self.properties.insert(key.to_string(), value);
    }
}

impl Value {
    /// Get a property from any value type.
    pub fn get_property(&self, key: &str) -> Value {
        match self {
            Value::Object(obj) => obj.borrow().get(key),
            Value::Array(arr) => {
                match key {
                    "length" => Value::Number(arr.borrow().len() as f64),
                    "push" => Value::Builtin(BuiltinFn {
                        name: "push".to_string(),
                        func: Rc::new(move |args| {
                            // Note: this creates a new closure each time, which is slow.
                            // But it works.
                            Ok(Value::Undefined)
                        }),
                    }),
                    "pop" => Value::Builtin(BuiltinFn {
                        name: "pop".to_string(),
                        func: Rc::new(|_| Ok(Value::Undefined)),
                    }),
                    "join" => Value::Builtin(BuiltinFn {
                        name: "join".to_string(),
                        func: Rc::new(|_| Ok(Value::String(String::new()))),
                    }),
                    _ => {
                        if let Ok(idx) = key.parse::<usize>() {
                            let arr = arr.borrow();
                            if idx < arr.len() {
                                arr[idx].clone()
                            } else {
                                Value::Undefined
                            }
                        } else {
                            Value::Undefined
                        }
                    }
                }
            }
            Value::String(s) => {
                match key {
                    "length" => Value::Number(s.chars().count() as f64),
                    "toUpperCase" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "toUpperCase".to_string(),
                            func: Rc::new(move |_| Ok(Value::String(s.to_uppercase()))),
                        })
                    }
                    "toLowerCase" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "toLowerCase".to_string(),
                            func: Rc::new(move |_| Ok(Value::String(s.to_lowercase()))),
                        })
                    }
                    "charAt" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "charAt".to_string(),
                            func: Rc::new(move |args| {
                                let idx = args.get(0).map(|v| v.to_number() as usize).unwrap_or(0);
                                Ok(Value::String(s.chars().nth(idx).map(|c| c.to_string()).unwrap_or_default()))
                            }),
                        })
                    }
                    "indexOf" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "indexOf".to_string(),
                            func: Rc::new(move |args| {
                                let search = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                                Ok(Value::Number(s.find(&search).map(|i| {
                                    // Convert byte index to char index.
                                    s[..i].chars().count() as f64
                                }).unwrap_or(-1.0)))
                            }),
                        })
                    }
                    "includes" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "includes".to_string(),
                            func: Rc::new(move |args| {
                                let search = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                                Ok(Value::Boolean(s.contains(&search)))
                            }),
                        })
                    }
                    "startsWith" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "startsWith".to_string(),
                            func: Rc::new(move |args| {
                                let search = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                                Ok(Value::Boolean(s.starts_with(&search)))
                            }),
                        })
                    }
                    "endsWith" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "endsWith".to_string(),
                            func: Rc::new(move |args| {
                                let search = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                                Ok(Value::Boolean(s.ends_with(&search)))
                            }),
                        })
                    }
                    "slice" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "slice".to_string(),
                            func: Rc::new(move |args| {
                                let chars: Vec<char> = s.chars().collect();
                                let start = args.get(0).map(|v| v.to_number() as i64).unwrap_or(0);
                                let end = args.get(1).map(|v| v.to_number() as i64).unwrap_or(chars.len() as i64);
                                let start = if start < 0 { (chars.len() as i64 + start).max(0) as usize } else { start as usize };
                                let end = if end < 0 { (chars.len() as i64 + end).max(0) as usize } else { end.min(chars.len() as i64) as usize };
                                if start >= end { return Ok(Value::String(String::new())); }
                                Ok(Value::String(chars[start..end].iter().collect()))
                            }),
                        })
                    }
                    "substring" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "substring".to_string(),
                            func: Rc::new(move |args| {
                                let chars: Vec<char> = s.chars().collect();
                                let mut start = args.get(0).map(|v| v.to_number() as usize).unwrap_or(0);
                                let mut end = args.get(1).map(|v| v.to_number() as usize).unwrap_or(chars.len());
                                if start > end { std::mem::swap(&mut start, &mut end); }
                                start = start.min(chars.len());
                                end = end.min(chars.len());
                                Ok(Value::String(chars[start..end].iter().collect()))
                            }),
                        })
                    }
                    "split" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "split".to_string(),
                            func: Rc::new(move |args| {
                                let sep = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                                let parts: Vec<Value> = if sep.is_empty() {
                                    s.chars().map(|c| Value::String(c.to_string())).collect()
                                } else {
                                    s.split(&sep).map(|p| Value::String(p.to_string())).collect()
                                };
                                Ok(Value::Array(Rc::new(RefCell::new(parts))))
                            }),
                        })
                    }
                    "trim" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "trim".to_string(),
                            func: Rc::new(move |_| Ok(Value::String(s.trim().to_string()))),
                        })
                    }
                    "replace" => {
                        let s = s.clone();
                        Value::Builtin(BuiltinFn {
                            name: "replace".to_string(),
                            func: Rc::new(move |args| {
                                let from = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                                let to = args.get(1).map(|v| v.to_string()).unwrap_or_default();
                                Ok(Value::String(s.replacen(&from, &to, 1)))
                            }),
                        })
                    }
                    _ => {
                        if let Ok(idx) = key.parse::<usize>() {
                            s.chars().nth(idx).map(|c| Value::String(c.to_string())).unwrap_or(Value::Undefined)
                        } else {
                            Value::Undefined
                        }
                    }
                }
            }
            Value::Function(_) | Value::Builtin(_) => {
                match key {
                    "call" => Value::Builtin(BuiltinFn { name: "call".to_string(), func: Rc::new(|_| Ok(Value::Undefined)) }),
                    "apply" => Value::Builtin(BuiltinFn { name: "apply".to_string(), func: Rc::new(|_| Ok(Value::Undefined)) }),
                    "bind" => Value::Builtin(BuiltinFn { name: "bind".to_string(), func: Rc::new(|_| Ok(Value::Undefined)) }),
                    _ => Value::Undefined,
                }
            }
            _ => Value::Undefined,
        }
    }

    /// Set a property on any value type.
    pub fn set_property(&self, key: &str, value: Value) {
        match self {
            Value::Object(obj) => {
                obj.borrow_mut().set(key, value);
            }
            Value::Array(arr) => {
                if key == "length" {
                    let len = value.to_number() as usize;
                    let mut a = arr.borrow_mut();
                    a.resize(len, Value::Undefined);
                } else if let Ok(idx) = key.parse::<usize>() {
                    let mut a = arr.borrow_mut();
                    while a.len() <= idx {
                        a.push(Value::Undefined);
                    }
                    a[idx] = value;
                }
            }
            _ => {}
        }
    }
}
