use std::{cell::RefCell, rc::Rc};

use crate::{
    logln, LogLevel, Code, CodeResult, JsValue, Prototype, handle_return,
    parser::{expr::Expr, stmt::Stmt},
};

pub struct IfStmt {
    pub blocks: Vec<(Box<dyn Expr>, Box<dyn Stmt>)>,
}

impl Stmt for IfStmt {
    fn compile_stmt(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        logln(LogLevel::Info, &format!("IfStmt::compile_stmt blocks={}", self.blocks.len()));
        let blocks: Vec<(Code, Vec<Code>)> = self
            .blocks
            .iter()
            .map(|(k, v)| (k.compile_expr(mem.clone()), v.compile_stmt(mem.clone())))
            .collect();
        let total: usize = blocks.iter().map(|(_, v)| v.len() + 2).sum();
        let mut current: usize = 0;
        blocks
            .into_iter()
            .flat_map(|(k, mut v)| {
                current += v.len() + 2;
                let go_to_end = total - current;
                let len = v.len();
                v.insert(
                    0,
                    Box::new(move |proto, i| {
                        let cond = handle_return!(k(proto, i));
                        if cond.borrow().is_fasly() {
                            i.move_amount(true, len + 1);
                            i.reset_retry();
                        }
                        CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                    }),
                );
                v.push(Box::new(move |_, i| {
                    i.move_amount(true, go_to_end);
                    i.reset_retry();
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }));
                v
            })
            .collect()
    }
}
