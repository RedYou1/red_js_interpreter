use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::{ParseError, Parser},
        stmt::Stmt,
    },
    run_sub,
};

pub struct LoopStmt {
    pub init: Option<Box<dyn Expr>>,
    pub condition: Option<Box<dyn Expr>>,
    pub update: Option<Box<dyn Expr>>,
    pub body: Vec<Box<dyn Stmt>>,
    pub do_first: bool,
}

impl LoopStmt {
    pub fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let t = parser.current().clone();
        parser.bump();
        logln(LogLevel::Info, "parse_statement for loop");
        if !matches!(parser.current(), Token::LParen) {
            return Err(ParseError("expected '(' after 'for'".into()));
        }
        parser.bump();

        // Parse init
        let mut of = false;
        let (init, of_cond): (Option<Box<dyn Expr>>, Option<Box<dyn Expr>>) =
            if !matches!(t, Token::For) || matches!(parser.current(), Token::Semicolon) {
                (None, None)
            } else if matches!(parser.current(), Token::Let | Token::Const | Token::Var)
                || matches!(parser.peek(), Token::Of)
            {
                if !matches!(parser.peek(), Token::Of) {
                    parser.bump();
                }
                let name = parser.expect_ident()?;
                let initializer = if let Token::Assign = parser.current() {
                    parser.bump();
                    Some(parser.parse_expression()?)
                } else if let Token::Of = parser.current() {
                    parser.bump();
                    of = true;
                    Some(parser.parse_expression()?)
                } else {
                    None
                };
                if let Token::Semicolon = parser.current() {
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
                let expr = parser.parse_expression()?;
                if let Token::Semicolon = parser.current() {
                    parser.bump();
                }
                (Some(expr), None)
            };

        // Parse condition
        let condition: Option<Box<dyn Expr>> = if of {
            of_cond
        } else {
            Some(if let Token::Semicolon = parser.current() {
                Box::new(expr::ConstBoolean { b: true })
            } else {
                parser.parse_expression()?
            })
        };
        if let Token::Semicolon = parser.current() {
            parser.bump();
        }

        // Parse update
        let update = if of || matches!(parser.current(), Token::RParen) {
            None
        } else {
            Some(parser.parse_expression()?)
        };

        if !matches!(parser.current(), Token::RParen) {
            return Err(ParseError("expected ')' after for clauses".into()));
        }
        parser.bump();

        if !matches!(parser.current(), Token::LBrace) {
            return Err(ParseError("expected '{' for for body".into()));
        }
        let body = parser.parse_block_body()?;

        Ok(Self {
            init,
            condition,
            update,
            body,
            do_first: false,
        })
    }
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
