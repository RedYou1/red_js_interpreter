use std::{cell::RefCell, rc::Rc};

use crate::{LogLevel, Prototype, Runnable, logln, parser::expr::Expr};

pub struct Program {
    pub body: Vec<Box<dyn Expr>>,
}

impl Program {
    pub fn compile(self, prebuild: Rc<RefCell<Prototype>>) -> Runnable {
        logln(
            LogLevel::Info,
            &format!("Entering Program::compile body_size={}", self.body.len()),
        );
        Runnable {
            params: Vec::new(),
            excess: None,
            code: self.body.compile(prebuild.clone()),
            mem: prebuild,
        }
    }
}
