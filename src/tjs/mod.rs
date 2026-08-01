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

pub mod lexer;
pub mod parser;
pub mod value;
pub mod interpreter;
pub mod builtins;
pub mod vm;
pub mod jit;

use std::collections::HashMap;

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
        Self { global_scope: global, use_vm: false }
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
        tjs.execute("var x = 10; var y = 20; console.log(x + y)").unwrap();
    }

    #[test]
    fn tjs_functions() {
        let mut tjs = TjsContext::new();
        let result = tjs.execute("function add(a, b) { return a + b } add(3, 4)").unwrap();
        assert_eq!(result, value::Value::Number(7.0));
    }

    #[test]
    fn tjs_closures() {
        let mut tjs = TjsContext::new();
        let result = tjs.execute(
            "function counter() { var n = 0; return function() { return ++n } }
             var c = counter(); c(); c(); c()"
        ).unwrap();
        assert_eq!(result, value::Value::Number(3.0));
    }

    #[test]
    fn tjs_objects() {
        let mut tjs = TjsContext::new();
        let result = tjs.execute("var obj = { x: 1, y: 2 }; obj.x + obj.y").unwrap();
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
        let result = tjs.execute("var sum = 0; for (var i = 1; i <= 100; i++) { sum += i } sum").unwrap();
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
        let result = tjs.execute("var n = 10; var f = 1; while (n > 1) { f *= n; n-- } f").unwrap();
        assert_eq!(result, value::Value::Number(3628800.0));
    }

    #[test]
    fn tjs_if_else() {
        let mut tjs = TjsContext::new();
        let result = tjs.execute("var x = 5; if (x > 3) { 'big' } else { 'small' }").unwrap();
        assert_eq!(result, value::Value::String("big".to_string()));
    }
}
