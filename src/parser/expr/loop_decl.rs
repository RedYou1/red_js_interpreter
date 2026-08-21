use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::{
        expr::Expr,
        expr::{self, BinaryOp},
        lexer::Token,
        parser::Parser,
    },
    run_sub,
};

#[derive(Debug)]
pub struct LoopExpr {
    pub init: Option<Box<dyn Expr>>,
    pub condition: Option<Box<dyn Expr>>,
    pub update: Option<Box<dyn Expr>>,
    pub body: Vec<Box<dyn Expr>>,
    pub do_first: bool,
}

impl LoopExpr {
    pub fn parse(parser: &mut Parser) -> Self {
        let t = parser.tokens()[parser.index()].clone();
        parser.bump();
        logln(LogLevel::Info, "Entering LoopExpr::parse");
        if !matches!(parser.tokens()[parser.index()], Token::LParen) {
            logln(
                LogLevel::Fatal,
                &format!(
                    "LoopExpr::parse expected '(' after {:?} at index {} but found {:?}",
                    t,
                    parser.index(),
                    parser.tokens()[parser.index()]
                ),
            );
            panic!("expected '(' after 'for'");
        }
        parser.bump();

        // Parse init
        let mut of = false;
        let (init, of_cond): (Option<Box<dyn Expr>>, Option<Box<dyn Expr>>) =
            if !matches!(t, Token::For)
                || matches!(parser.tokens()[parser.index()], Token::Semicolon)
            {
                (None, None)
            } else if matches!(
                parser.tokens()[parser.index()],
                Token::Let | Token::Const | Token::Var
            ) || matches!(parser.tokens()[parser.index() + 1], Token::Of)
            {
                if !matches!(parser.tokens()[parser.index() + 1], Token::Of) {
                    parser.bump();
                }
                let name = parser.expect_ident();
                let initializer: Option<Box<dyn Expr>> =
                    if let Token::Assign(t) = &parser.tokens()[parser.index()] {
                        assert_eq!(*t, Option::<BinaryOp>::None);
                        parser.bump();
                        Some(Box::new(parser.parse_expression()))
                    } else if let Token::Of = parser.tokens()[parser.index()] {
                        parser.bump();
                        of = true;
                        Some(Box::new(parser.parse_expression()))
                    } else {
                        None
                    };
                if let Token::Semicolon = parser.tokens()[parser.index()] {
                    parser.bump();
                }
                if of {
                    (
                        Some(Box::new([
                            Box::new(expr::VarDecl {
                                name: format!("__for_of_{name}__"),
                                initializer: Some(Box::new(expr::Call {
                                    func: Box::new(expr::Member {
                                        object: initializer.unwrap(),
                                        property: Box::new(expr::Member {
                                            object: Box::new(expr::Identifier {
                                                name: stringify!(Symbol).to_owned(),
                                            }),
                                            property: Box::new(expr::ConstString {
                                                s: "iterator".to_owned(),
                                            }),
                                        }),
                                    }),
                                    args: Vec::new(),
                                })),
                            }) as Box<dyn Expr>,
                            Box::new(expr::VarDecl {
                                name: name.clone(),
                                initializer: None,
                            }),
                        ])),
                        Some(Box::new(expr::Operator {
                            left: Box::new(expr::Assign {
                                value: Box::new(expr::Call {
                                    func: Box::new(expr::Member {
                                        object: Box::new(expr::Identifier {
                                            name: format!("__for_of_{name}__"),
                                        }),
                                        property: Box::new(expr::ConstString {
                                            s: "next".to_owned(),
                                        }),
                                    }),
                                    args: Vec::new(),
                                }),
                                target: Box::new(expr::ConstString { s: name }),
                            }),
                            op: expr::BinaryOp::NotEq,
                            right: Box::new(expr::ConstObj {
                                obj: JsValue::Undefined,
                            }),
                        })),
                    )
                } else {
                    (Some(Box::new(expr::VarDecl { name, initializer })), None)
                }
            } else {
                let expr = Box::new(parser.parse_expression());
                if let Token::Semicolon = parser.tokens()[parser.index()] {
                    parser.bump();
                }
                (Some(expr), None)
            };

        // Parse condition
        let condition: Option<Box<dyn Expr>> = if of {
            of_cond
        } else {
            Some(if let Token::Semicolon = parser.tokens()[parser.index()] {
                Box::new(expr::ConstBoolean { b: true })
            } else {
                Box::new(parser.parse_expression())
            })
        };
        if let Token::Semicolon = parser.tokens()[parser.index()] {
            parser.bump();
        }

        // Parse update
        let update: Option<Box<dyn Expr>> =
            if of || matches!(parser.tokens()[parser.index()], Token::RParen) {
                None
            } else {
                Some(Box::new(parser.parse_expression()))
            };

        if !matches!(parser.tokens()[parser.index()], Token::RParen) {
            logln(
                LogLevel::Fatal,
                &format!(
                    "LoopExpr::parse expected ')' after clauses at index {} but found {:?}",
                    parser.index(),
                    parser.tokens()[parser.index()]
                ),
            );
            panic!("expected ')' after for clauses");
        }
        parser.bump();

        let body = parser.parse_block();

        Self {
            init,
            condition,
            update,
            body,
            do_first: false,
        }
    }
}

impl Expr for LoopExpr {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        logln(
            LogLevel::Info,
            &format!(
                "Entering LoopExpr::compile do_first={} body_len={}",
                self.do_first,
                self.body.len()
            ),
        );
        if self.init.is_none() && self.condition.is_none() && self.update.is_none() {
            logln(
                LogLevel::Fatal,
                "LoopExpr::compile received a loop with no init, condition, or update",
            );
            panic!("loop has no executable clauses");
        }
        let do_first = self.do_first;
        let init: Vec<Code> = self.init.compile(mem.clone());
        let condition: Vec<Code> = self.condition.compile(mem.clone());
        let update: Vec<Code> = self.update.compile(mem.clone());
        let body: Vec<Code> = self.body.compile(mem.clone());

        vec![
            Box::new(move |proto, _i| {
                handle_return!(run_sub(&init, proto.clone(), &mut CodeIndex::new()));

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
                    //TODO handle correctly his label
                    match &res {
                        CodeResult::Normal(_)
                        | CodeResult::NormalMember(_, _, _)
                        | CodeResult::Continue(_) => {}
                        CodeResult::Break(_) | CodeResult::YieldBreak => {
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
                        CodeResult::Error(_) => return res,
                    }
                }

                handle_return!(run_sub(&update, proto.clone(), &mut CodeIndex::new()));

                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            }),
            Box::new(move |proto, _i| {
                let cond =
                    handle_return!(run_sub(&condition, proto.clone(), &mut CodeIndex::new()));
                logln(
                    LogLevel::Trace,
                    &format!(
                        "LoopExpr condition index={} truthy={}",
                        _i.current(),
                        cond.borrow().is_truthy()
                    ),
                );
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
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            init: self.init.as_ref().map(|a| a.as_ref().duplicate()),
            condition: self.condition.as_ref().map(|a| a.duplicate()),
            update: self.update.as_ref().map(|a| a.duplicate()),
            body: self.body.iter().map(|a| a.duplicate()).collect(),
            do_first: self.do_first,
        })
    }
}
