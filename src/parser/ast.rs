use std::{cell::RefCell, rc::Rc};

use crate::{LogLevel, Prototype, Runnable, logln, parser::stmt::Stmt};

pub struct Program {
    pub body: Vec<Box<dyn Stmt>>,
}

impl Program {
    pub fn compile(self, prebuild: Rc<RefCell<Prototype>>) -> Runnable {
        logln(
            LogLevel::Info,
            &format!("Program::compile body_size={}", self.body.len()),
        );
        Runnable {
            params: Vec::new(),
            excess: None,
            code: self.body.compile_stmt(prebuild.clone()),
            mem: prebuild,
        }
    }
}
