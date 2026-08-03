//! TJS — Talons JavaScript Engine
//!
//! A from-scratch JavaScript engine for Falco. Pure Rust, no external
//! dependencies. Tree-walking interpreter with planned bytecode VM.
//!
//! Pipeline: Source → Lexer → Tokens → Parser → AST → Interpreter → Result
//!
//! Supported features:
//!   - Variables: var, let, const
//!   - Functions: declarations, expressions, arrow functions, closures
//!   - Control flow: if/else, for, while, do/while, break, continue
//!   - Types: number, string, boolean, null, undefined, object, array
//!   - Operators: + - * / % == != < > <= >= && || ! ++ -- += -= etc.
//!   - Builtins: Math, console.log, Date.now, Array, Object, JSON, parseInt
//!   - Template literals (basic)
//!   - try/catch (basic)

pub mod builtins;
pub mod interpreter;
pub mod jit;
pub mod lexer;
pub mod parser;
pub mod value;
pub mod vm;

/// TJS execution context — owns the global scope and builtin functions.
pub struct TjsContext {
    pub global_scope: interpreter::Scope,
    use_vm: bool,
}

impl TjsContext {
    /// Create a new TJS context with builtins registered.
    pub fn new() -> Self {
        let mut global = interpreter::Scope::new(None);
        builtins::register(&mut global);
        Self {
            global_scope: global,
            use_vm: false,
        }
    }

    /// Enable the bytecode VM for faster execution.
    pub fn with_vm(mut self) -> Self {
        self.use_vm = true;
        self
    }

    /// Execute a JavaScript source string.
    pub fn execute(&mut self, src: &str) -> Result<value::Value, String> {
        let tokens = lexer::tokenize(src)?;
        let ast = parser::parse(&tokens)?;

        if self.use_vm {
            // Create a VM with builtins registered.
            let mut machine = vm::Vm::new();

            // Create a compiler that knows about the same global names as the VM.
            let mut compiler = vm::Compiler::new();
            // Register the same builtins as global names so indices match.
            for name in machine.global_names() {
                compiler.register_global_name(name);
            }

            let bytecode = compiler.compile_program(&ast);
            match machine.run(&bytecode) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    eprintln!("[tjs:vm] falling back to interpreter: {}", e);
                }
            }
        }

        // Tree-walking interpreter.
        let result = interpreter::interpret(&ast, &mut self.global_scope)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tjs_arithmetic() {
        let mut tjs = TjsContext::new();
        let result = tjs.execute("1 + 2 * 3").unwrap();
        assert_eq!(result, value::Value::Number(7.0));
    }

    #[test]
    fn tjs_variables() {
        let mut tjs = TjsContext::new();
        tjs.execute("var x = 10; var y = 20; console.log(x + y)")
            .unwrap();
    }

    #[test]
    fn tjs_functions() {
        let mut tjs = TjsContext::new();
        let result = tjs
            .execute("function add(a, b) { return a + b } add(3, 4)")
            .unwrap();
        assert_eq!(result, value::Value::Number(7.0));
    }

    #[test]
    fn tjs_closures() {
        let mut tjs = TjsContext::new();
        let result = tjs
            .execute(
                "function counter() { var n = 0; return function() { return ++n } }
             var c = counter(); c(); c(); c()",
            )
            .unwrap();
        assert_eq!(result, value::Value::Number(3.0));
    }

    #[test]
    fn tjs_objects() {
        let mut tjs = TjsContext::new();
        let result = tjs
            .execute("var obj = { x: 1, y: 2 }; obj.x + obj.y")
            .unwrap();
        assert_eq!(result, value::Value::Number(3.0));
    }

    #[test]
    fn tjs_arrays() {
        let mut tjs = TjsContext::new();
        let result = tjs.execute("var arr = [1, 2, 3, 4, 5]; arr[2]").unwrap();
        assert_eq!(result, value::Value::Number(3.0));
    }

    #[test]
    fn tjs_loops() {
        let mut tjs = TjsContext::new();
        let result = tjs
            .execute("var sum = 0; for (var i = 1; i <= 100; i++) { sum += i } sum")
            .unwrap();
        assert_eq!(result, value::Value::Number(5050.0));
    }

    #[test]
    fn tjs_strings() {
        let mut tjs = TjsContext::new();
        let result = tjs.execute("'Hello' + ' ' + 'World'").unwrap();
        assert_eq!(result, value::Value::String("Hello World".to_string()));
    }

    #[test]
    fn tjs_while() {
        let mut tjs = TjsContext::new();
        let result = tjs
            .execute("var n = 10; var f = 1; while (n > 1) { f *= n; n-- } f")
            .unwrap();
        assert_eq!(result, value::Value::Number(3628800.0));
    }

    #[test]
    fn tjs_if_else() {
        let mut tjs = TjsContext::new();
        let result = tjs
            .execute("var x = 5; if (x > 3) { 'big' } else { 'small' }")
            .unwrap();
        assert_eq!(result, value::Value::String("big".to_string()));
    }

    // --- Regression tests: Array.prototype methods (were stubs / missing) ---
    // See value.rs `Value::Array` arm. push/pop/join were stubs returning
    // Undefined/empty; map/filter/reduce/forEach/find/some/every were absent.
    //
    // NOTE: Value::Array equality is Rc pointer identity, so we assert via
    // JSON.stringify (output) for array results and via scalar Value for the
    // rest.

    #[test]
    fn tjs_array_map() {
        let mut tjs = TjsContext::new();
        let r = tjs
            .execute("JSON.stringify([1,2,3,4,5].map(x => x*x))")
            .unwrap();
        assert_eq!(r, value::Value::String("[1,4,9,16,25]".to_string()));
    }

    #[test]
    fn tjs_array_filter() {
        let mut tjs = TjsContext::new();
        let r = tjs
            .execute("JSON.stringify([1,2,3,4,5].filter(x => x % 2 == 0))")
            .unwrap();
        assert_eq!(r, value::Value::String("[2,4]".to_string()));
    }

    #[test]
    fn tjs_array_reduce_with_init() {
        let mut tjs = TjsContext::new();
        let r = tjs.execute("[1,2,3,4,5].reduce((a,b) => a+b, 0)").unwrap();
        assert_eq!(r, value::Value::Number(15.0));
    }

    #[test]
    fn tjs_array_reduce_no_init() {
        let mut tjs = TjsContext::new();
        let r = tjs.execute("[1,2,3,4,5].reduce((a,b) => a+b)").unwrap();
        assert_eq!(r, value::Value::Number(15.0));
    }

    #[test]
    fn tjs_array_chain() {
        let mut tjs = TjsContext::new();
        let r = tjs
            .execute("[1,2,3,4,5].map(x=>x+1).filter(x=>x>2).reduce((a,b)=>a*b,1)")
            .unwrap();
        assert_eq!(r, value::Value::Number(360.0));
    }

    #[test]
    fn tjs_array_push_join() {
        let mut tjs = TjsContext::new();
        let r = tjs
            .execute("var a=[10,20]; a.push(30); a.push(40); a.join('-')")
            .unwrap();
        assert_eq!(r, value::Value::String("10-20-30-40".to_string()));
    }

    #[test]
    fn tjs_array_slice_concat_includes_indexof() {
        let mut tjs = TjsContext::new();
        assert_eq!(
            tjs.execute("JSON.stringify([1,2,3,4,5].slice(1,3))")
                .unwrap(),
            value::Value::String("[2,3]".to_string())
        );
        assert_eq!(
            tjs.execute("JSON.stringify([1,2].concat([3,4],[5]))")
                .unwrap(),
            value::Value::String("[1,2,3,4,5]".to_string())
        );
        assert_eq!(
            tjs.execute("[1,2,3].includes(3)").unwrap(),
            value::Value::Boolean(true)
        );
        assert_eq!(
            tjs.execute("[1,2,3,4].indexOf(4)").unwrap(),
            value::Value::Number(3.0)
        );
    }

    #[test]
    fn tjs_array_find_some_every() {
        let mut tjs = TjsContext::new();
        assert_eq!(
            tjs.execute("[1,2,3,4,5].find(x => x > 3)").unwrap(),
            value::Value::Number(4.0)
        );
        assert_eq!(
            tjs.execute("[1,2,3,4,5].some(x => x > 4)").unwrap(),
            value::Value::Boolean(true)
        );
        assert_eq!(
            tjs.execute("[1,2,3,4,5].every(x => x < 10)").unwrap(),
            value::Value::Boolean(true)
        );
    }

    #[test]
    fn tjs_array_callback_closure_capture() {
        let mut tjs = TjsContext::new();
        let r = tjs
            .execute("JSON.stringify((function(){ var f=5; return [1,2,3].map(x => x*f); })())")
            .unwrap();
        assert_eq!(r, value::Value::String("[5,10,15]".to_string()));
    }

    #[test]
    fn tjs_string_repeat() {
        let mut tjs = TjsContext::new();
        let r = tjs.execute(r#""abc".repeat(3)"#).unwrap();
        assert_eq!(r, value::Value::String("abcabcabc".to_string()));
    }

    #[test]
    fn tjs_string_pad_start() {
        let mut tjs = TjsContext::new();
        let r = tjs.execute(r#""5".padStart(3, "0")"#).unwrap();
        assert_eq!(r, value::Value::String("005".to_string()));
    }

    #[test]
    fn tjs_string_pad_end() {
        let mut tjs = TjsContext::new();
        let r = tjs.execute(r#""hi".padEnd(5, "!")"#).unwrap();
        assert_eq!(r, value::Value::String("hi!!!".to_string()));
    }

    #[test]
    fn tjs_string_trim_start_end() {
        let mut tjs = TjsContext::new();
        let r = tjs.execute(r#""  hello  ".trimStart()"#).unwrap();
        assert_eq!(r, value::Value::String("hello  ".to_string()));
        let r = tjs.execute(r#""  hello  ".trimEnd()"#).unwrap();
        assert_eq!(r, value::Value::String("  hello".to_string()));
    }

    #[test]
    fn tjs_string_includes() {
        let mut tjs = TjsContext::new();
        let r = tjs.execute(r#""hello world".includes("world")"#).unwrap();
        assert_eq!(r, value::Value::Boolean(true));
        let r = tjs.execute(r#""hello world".includes("xyz")"#).unwrap();
        assert_eq!(r, value::Value::Boolean(false));
    }

    #[test]
    fn tjs_string_starts_ends_with() {
        let mut tjs = TjsContext::new();
        let r = tjs.execute(r#""hello".startsWith("he")"#).unwrap();
        assert_eq!(r, value::Value::Boolean(true));
        let r = tjs.execute(r#""hello".endsWith("lo")"#).unwrap();
        assert_eq!(r, value::Value::Boolean(true));
    }

    #[test]
    fn tjs_string_slice_substring() {
        let mut tjs = TjsContext::new();
        let r = tjs.execute(r#""hello".slice(1, 4)"#).unwrap();
        assert_eq!(r, value::Value::String("ell".to_string()));
        let r = tjs.execute(r#""hello".substring(1, 4)"#).unwrap();
        assert_eq!(r, value::Value::String("ell".to_string()));
    }

    #[test]
    fn tjs_string_replace() {
        let mut tjs = TjsContext::new();
        let r = tjs
            .execute(r#""hello world".replace("world", "rust")"#)
            .unwrap();
        assert_eq!(r, value::Value::String("hello rust".to_string()));
    }
}
