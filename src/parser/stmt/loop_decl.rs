use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::{expr::Expr, stmt::Stmt},
    run_sub,
};

pub struct LoopStmt {
    pub init: Option<Box<dyn Expr>>,
    pub condition: Option<Box<dyn Expr>>,
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
        assert!(self.init.is_some() || self.condition.is_some() || self.update.is_some());
        let do_first = self.do_first;
        let init: Code = self.init.compile_expr(mem.clone());
        let condition: Code = self.condition.compile_expr(mem.clone());
        let update: Code = self.update.compile_expr(mem.clone());
        let body: Vec<Code> = self
            .body
            .iter()
            .flat_map(|c| c.compile_stmt(mem.clone()))
            .collect();

        vec![
            Box::new(move |proto, _i| {
                handle_return!(init(proto.clone(), &mut CodeIndex::new()));

                let sub = Prototype::new_child(proto.clone(), None, []);
                proto.borrow_mut().properties.insert(
                    "__forloop_sub__".into(),
                    Rc::new(RefCell::new(JsValue::Prototype(sub.clone()))),
                );
                CodeIndex::new().save_into(sub.clone(), "forloop_i");

                if !do_first {
                    _i.skip(1);
                }
                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            }),
            Box::new(move |proto, _i| {
                let sub =
                    inline_borrow!(proto.borrow().properties[&"__forloop_sub__".into()].clone())
                        .unwrap_proto("sub not proto in loop body?");
                let mut i = CodeIndex::load_from(sub.clone(), "forloop_i");
                if i.current < body.len() {
                    let res = run_sub(&body, sub.clone(), &mut i);
                    match &res {
                        CodeResult::Normal(_)
                        | CodeResult::NormalMember(_, _, _)
                        | CodeResult::Continue => {}
                        CodeResult::Break | CodeResult::YieldBreak => {
                            _i.move_iamount(1);
                            _i.reset_retry();
                            i.reset();
                            i.save_into(sub, "forloop_i");
                            return CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)));
                        }
                        CodeResult::Return(_) => return res,
                        CodeResult::Yield(res) => {
                            i.next();
                            i.set_retry();
                            i.save_into(sub, "forloop_i");
                            _i.set_retry();
                            return CodeResult::Yield(res.clone());
                        }
                    }
                }

                handle_return!(update(proto.clone(), &mut CodeIndex::new()));

                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            }),
            Box::new(move |proto, _i| {
                let cond = handle_return!(condition(proto.clone(), _i));
                if cond.borrow().is_truthy() {
                    let sub = Prototype::new_child(proto.clone(), None, []);
                    proto.borrow_mut().properties.insert(
                        "__forloop_sub__".into(),
                        Rc::new(RefCell::new(JsValue::Prototype(sub.clone()))),
                    );
                    CodeIndex::new().save_into(sub.clone(), "forloop_i");

                    _i.move_iamount(-1);
                    _i.set_retry();
                }
                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            }),
        ]
    }
}
