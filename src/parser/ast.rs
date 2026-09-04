use crate::{Environment, LogLevel, Runnable, parser::expr::Expr};

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
        }
    }
}
