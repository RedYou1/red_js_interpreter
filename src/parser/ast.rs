use std::{cell::RefCell, rc::Rc};

use crate::{
    CodeResult, Environment, JsValue, LogLevel, Prototype, Runnable, new_runnable,
    parser::expr::Expr, run_function_object,
};

pub struct Program {
    pub body: Vec<Box<dyn Expr>>,
}

impl Program {
    pub fn compile(self, env: Environment) -> Runnable {
        env.logger.borrow_mut().logln(LogLevel::Info, &|| {
            format!("Entering Program::compile body_size={}", self.body.len())
        });
        Runnable {
            params: Vec::new(),
            excess: None,
            code: self.body.compile(env.clone()),
            mem: env.mem,
            lexical_this: None,
        }
    }

    pub fn run(self, env: Environment) -> CodeResult {
        let runnable = self.compile(env.clone());
        Self::run_compiled(runnable, env)
    }

    pub fn run_compiled(runnable: Runnable, env: Environment) -> CodeResult {
        let function = Prototype::find(env.mem.clone(), &"Function".into())
            .1
            .borrow()
            .unwrap_proto("Program::run for Function");
        let main = new_runnable(function, "__main__", runnable);
        run_function_object(
            main.borrow().unwrap_proto("Program::run for main function"),
            Rc::new(RefCell::new(JsValue::Undefined)),
            vec![],
            env.logger,
        )
    }
}
