use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, logln,
    parser::{expr::Expr, stmt::Stmt},
    run_sub,
};

pub struct LoopStmt {
    pub init: Option<Box<dyn Expr>>,
    pub condition: Box<dyn Expr>,
    pub update: Option<Box<dyn Expr>>,
    pub body: Vec<Box<dyn Stmt>>,
    pub do_first: bool,
}

impl Stmt for LoopStmt {
    fn compile_stmt(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        logln(
            LogLevel::Info,
            &format!(
                "LoopStmt::compile_stmt do_first={} body_len={}",
                self.do_first,
                self.body.len()
            ),
        );
        let do_first = self.do_first;
        let init: Code = self.init.compile_expr(mem.clone());
        let condition: Code = self.condition.compile_expr(mem.clone());
        let update: Code = self.update.compile_expr(mem.clone());
        let body: Vec<Code> = self
            .body
            .iter()
            .flat_map(|c| c.compile_stmt(mem.clone()))
            .collect();

        let body_init = Rc::new(RefCell::new(CodeIndex::new()));
        let body_code = body_init.clone();
        let body_end = body_init.clone();
        vec![
            Box::new(move |proto, _i| {
                body_init.borrow_mut().reset();
                handle_return!(init(proto, _i));
                if !do_first {
                    _i.skip(1);
                }
                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            }),
            Box::new(move |proto, _i| {
                let res = run_sub(&body, proto.clone(), &mut body_code.borrow_mut());
                match &res {
                    CodeResult::Normal(_) | CodeResult::NormalMember(_, _, _) | CodeResult::Continue => {}
                    CodeResult::Break | CodeResult::YieldBreak => {
                        _i.move_iamount(1);
                        _i.reset_retry();
                    }
                    CodeResult::Return(_) => return res,
                    CodeResult::Yield(_) => {
                        _i.set_retry();
                        return res;
                    }
                }

                handle_return!(update(proto.clone(), _i));

                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            }),
            Box::new(move |proto, _i| {
                let cond = handle_return!(condition(proto, _i));
                if cond.borrow().is_truthy() {
                    body_end.borrow_mut().reset();
                    _i.move_iamount(-1);
                    _i.set_retry();
                }
                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            }),
        ]
    }
}
