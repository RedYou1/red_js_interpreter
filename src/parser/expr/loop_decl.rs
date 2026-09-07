use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, JsValue, LogLevel, PROTO_NAME, Prototype,
    handle_return, inline_borrow, new_array,
    parser::{
        expr::{self, BinaryOp, Expr},
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
    pub for_in: Option<(String, Box<dyn Expr>)>,
    pub for_of: Option<(String, Box<dyn Expr>)>,
    pub label: Option<String>,
}

impl LoopExpr {
    pub fn parse(parser: &mut Parser) -> Self {
        Self::parse_with_label(parser, None)
    }

    pub fn parse_with_label(parser: &mut Parser, label: Option<String>) -> Self {
        let t = parser.tokens()[parser.index()].clone();
        parser.bump();
        parser
            .env
            .logger
            .borrow_mut()
            .logln_str(LogLevel::Info, "Entering LoopExpr::parse");
        if !matches!(parser.tokens()[parser.index()], Token::LParen) {
            parser.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                format!(
                    "LoopExpr::parse expected '(' after {:?} at index {} but found {:?}",
                    t,
                    parser.index(),
                    parser.tokens()[parser.index()]
                )
            });
            panic!("expected '(' after 'for'");
        }
        parser.bump();

        // Parse init
        let mut for_in = None;
        let mut for_of = None;
        let init: Option<Box<dyn Expr>> =
            if !matches!(t, Token::For) {
                None
            } else if matches!(parser.tokens()[parser.index()], Token::Semicolon) {
                parser.bump();
                None
            } else if matches!(
                parser.tokens()[parser.index()],
                Token::Let | Token::Const | Token::Var
            ) || matches!(parser.tokens()[parser.index() + 1], Token::In | Token::Of)
            {
                if !matches!(parser.tokens()[parser.index() + 1], Token::In | Token::Of) {
                    parser.bump();
                }
                let name = parser.expect_ident();
                let initializer: Option<Box<dyn Expr>> =
                    if let Token::Assign(t) = &parser.tokens()[parser.index()] {
                        assert_eq!(*t, Option::<BinaryOp>::None);
                        parser.bump();
                        Some(Box::new(parser.parse_expression(true)))
                    } else if let Token::Of = parser.tokens()[parser.index()] {
                        parser.bump();
                        for_of = Some((
                            name.clone(),
                            Box::new(parser.parse_expression(true)) as Box<dyn Expr>,
                        ));
                        None
                    } else if let Token::In = parser.tokens()[parser.index()] {
                        parser.bump();
                        for_in = Some((
                            name.clone(),
                            Box::new(parser.parse_expression(true)) as Box<dyn Expr>,
                        ));
                        None
                    } else {
                        None
                    };
                if let Token::Semicolon = parser.tokens()[parser.index()] {
                    parser.bump();
                }
                Some(Box::new(expr::VarDecl {
                    name,
                    initializer,
                    function_scoped: false,
                }))
            } else {
                let expr = Box::new(parser.parse_expression(true));
                if let Token::Semicolon = parser.tokens()[parser.index()] {
                    parser.bump();
                }
                Some(expr)
            };

        // Parse condition
        let condition: Option<Box<dyn Expr>> = if for_in.is_some() || for_of.is_some() {
            None
        } else {
            Some(if let Token::Semicolon = parser.tokens()[parser.index()] {
                Box::new(expr::ConstBoolean { b: true })
            } else {
                Box::new(parser.parse_expression(true))
            })
        };
        if let Token::Semicolon = parser.tokens()[parser.index()] {
            parser.bump();
        }

        // Parse update
        let update: Option<Box<dyn Expr>> =
            if for_in.is_some()
                || for_of.is_some()
                || matches!(parser.tokens()[parser.index()], Token::RParen)
            {
                None
            } else {
                Some(Box::new(parser.parse_expression(true)))
            };

        if !matches!(parser.tokens()[parser.index()], Token::RParen) {
            parser.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                format!(
                    "LoopExpr::parse expected ')' after clauses at index {} but found {:?}",
                    parser.index(),
                    parser.tokens()[parser.index()]
                )
            });
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
            for_in,
            for_of,
            label,
        }
    }

    pub fn parse_do(parser: &mut Parser, label: Option<String>) -> Self {
        assert_eq!(parser.tokens()[parser.index()], Token::Do);
        parser.bump();
        let body = parser.parse_block();

        if parser.tokens()[parser.index()] != Token::While {
            panic!("expected 'while' after do body");
        }
        parser.bump();
        if parser.tokens()[parser.index()] != Token::LParen {
            panic!("expected '(' after while");
        }
        parser.bump();
        let condition = Some(Box::new(parser.parse_expression(true)) as Box<dyn Expr>);
        if parser.tokens()[parser.index()] != Token::RParen {
            panic!("expected ')' after while condition");
        }
        parser.bump();
        if parser.tokens()[parser.index()] == Token::Semicolon {
            parser.bump();
        }

        Self {
            init: None,
            condition,
            update: None,
            body,
            do_first: true,
            for_in: None,
            for_of: None,
            label,
        }
    }
}

fn for_in_property_names(value: Rc<RefCell<JsValue>>) -> Vec<String> {
    let JsValue::Prototype(mut current) = inline_borrow!(value) else {
        return Vec::new();
    };
    let mut names = Vec::new();

    loop {
        let parent = {
            let current_ref = current.borrow();
            let is_array =
                current_ref.parent().and_then(|parent| parent.borrow().name) == Some("Array");
            if current_ref.name.is_none() {
                names.extend(
                    current_ref
                        .properties
                        .keys()
                        .filter(|key| !current_ref.non_enumerable.contains(*key))
                        .filter_map(|key| match key {
                            JsValue::String(key)
                                if key != PROTO_NAME && (!is_array || key != "length") =>
                            {
                                Some(key.clone())
                            }
                            JsValue::BigInt(key) => Some(key.to_string()),
                            JsValue::Number(key) if key.is_finite() && key.fract() == 0.0 => {
                                Some((*key as i64).to_string())
                            }
                            _ => None,
                        }),
                );
            }
            current_ref.parent()
        };
        let Some(parent) = parent else {
            break;
        };
        if Rc::ptr_eq(&parent, &current) {
            break;
        }
        current = parent;
    }

    names.sort();
    names.dedup();
    names
}

fn is_generator_iterator(iterator: &Rc<RefCell<Prototype>>) -> bool {
    matches!(
        inline_borrow!(Prototype::find(iterator.clone(), &crate::RUNNABLE.into()).1),
        JsValue::Generator(_)
    ) || iterator
        .borrow()
        .properties
        .contains_key(&"__Generator_CodeIndex_current__".into())
}

impl Expr for LoopExpr {
    fn compile(&self, env: Environment) -> Vec<Code> {
        env.logger.borrow_mut().logln(LogLevel::Info, &|| {
            format!(
                "Entering LoopExpr::compile do_first={} body_len={}",
                self.do_first,
                self.body.len()
            )
        });
        if self.init.is_none() && self.condition.is_none() && self.update.is_none() {
            env.logger.borrow_mut().logln_str(
                LogLevel::Fatal,
                "LoopExpr::compile received a loop with no init, condition, or update",
            );
            panic!("loop has no executable clauses");
        }
        if let Some((for_of_name, for_of_expr)) = &self.for_of {
            let iterator_expr = expr::Call {
                func: Box::new(expr::Member {
                    object: for_of_expr.duplicate(),
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
            };
            let iterator = iterator_expr.compile(env.clone());
            let body: Vec<Code> = self.body.compile(env.clone());
            let for_of_name = for_of_name.clone();
            let label = self.label.clone();

            return vec![
                Box::new(move |env, _i| {
                    let iterator =
                        handle_return!(run_sub(&iterator, env.clone(), &mut CodeIndex::new()));
                    let sub = Prototype::new_child(
                        env.mem.clone(),
                        None,
                        [("__forloop_scope__".into(), Rc::new(RefCell::new(JsValue::Boolean(true))))],
                    );
                    sub.borrow_mut()
                        .properties
                        .insert("__for_of_iterator__".into(), iterator);
                    CodeIndex::new().save_into(sub.clone(), "forloop_i");
                    env.mem.borrow_mut().properties.insert(
                        "__forloop_sub__".into(),
                        Rc::new(RefCell::new(JsValue::Prototype(sub))),
                    );
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }),
                Box::new(move |env, _i| {
                    let sub = inline_borrow!(
                        env.mem.borrow().properties[&"__forloop_sub__".into()].clone()
                    )
                    .unwrap_proto("for-of sub not proto");
                    let iterator = inline_borrow!(
                        Prototype::find(sub.clone(), &"__for_of_iterator__".into()).1
                    )
                    .unwrap_proto("for-of iterator not proto");
                    let next = Prototype::find(iterator.clone(), &"next".into()).1;
                    let next = inline_borrow!(next).unwrap_proto("for-of next not function");
                    let result = match crate::run_function_object(
                        next,
                        Rc::new(RefCell::new(JsValue::Prototype(iterator.clone()))),
                        vec![],
                        env.logger.clone(),
                    ) {
                        CodeResult::Return(value) => value,
                        CodeResult::Error(error) => return CodeResult::Error(error),
                        other => return other,
                    };

                    let legacy_generator = is_generator_iterator(&iterator);
                    let (done, value) = if legacy_generator {
                        if matches!(inline_borrow!(result.clone()), JsValue::Undefined) {
                            (true, Rc::new(RefCell::new(JsValue::Undefined)))
                        } else {
                            (false, result)
                        }
                    } else {
                        match inline_borrow!(result.clone()) {
                            JsValue::Prototype(result) => {
                                let done = Prototype::find(result.clone(), &"done".into()).1;
                                let value = Prototype::find(result, &"value".into()).1;
                                (done.borrow().is_truthy(), value)
                            }
                            _ => {
                                let type_error = Prototype::new_child(
                                    Prototype::find(env.mem.clone(), &"TypeError".into())
                                        .1
                                        .borrow()
                                        .unwrap_proto("for-of TypeError"),
                                    None,
                                    [(
                                        "message".into(),
                                        Rc::new(RefCell::new(JsValue::String(
                                            "iterator result is not an object".to_owned(),
                                        ))),
                                    )],
                                );
                                return CodeResult::Error(Rc::new(RefCell::new(
                                    JsValue::Prototype(type_error),
                                )));
                            }
                        }
                    };
                    if done {
                        _i.skip(1);
                        return CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)));
                    }
                    sub.borrow_mut()
                        .properties
                        .insert(for_of_name.clone().into(), value);

                    let mut i = CodeIndex::load_from(sub.clone(), "forloop_i");
                    if i.current >= body.len() {
                        i.reset();
                    }
                    let res = run_sub(&body, env.with_mem(sub.clone()), &mut i);
                    match res {
                        CodeResult::Normal(_)
                        | CodeResult::NormalMember(_, _, _)
                        | CodeResult::Continue(None)
                        | CodeResult::Continue(Some(_)) => {}
                        CodeResult::Break(None) | CodeResult::YieldBreak => {
                            _i.move_iamount(1);
                            _i.reset_retry();
                            return CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)));
                        }
                        CodeResult::Break(Some(name)) if label.as_deref() == Some(name.as_str()) => {
                            _i.move_iamount(1);
                            _i.reset_retry();
                            return CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)));
                        }
                        CodeResult::Yield(value) => {
                            i.next();
                            i.set_retry();
                            i.save_into(sub, "forloop_i");
                            _i.set_retry();
                            return CodeResult::Yield(value);
                        }
                        other => return other,
                    }
                    i.reset();
                    i.save_into(sub, "forloop_i");
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }),
                Box::new(move |_, _i| {
                    _i.move_iamount(-1);
                    _i.set_retry();
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }),
            ];
        }

        if let Some((for_in_name, for_in_expr)) = &self.for_in {
            let init: Vec<Code> = self.init.compile(env.clone());
            let target: Vec<Code> = for_in_expr.compile(env.clone());
            let body: Vec<Code> = self.body.compile(env.clone());
            let for_in_name = for_in_name.clone();
            let label = self.label.clone();

            return vec![
                Box::new(move |env, _i| {
                    handle_return!(run_sub(&init, env.clone(), &mut CodeIndex::new()));
                    let target =
                        handle_return!(run_sub(&target, env.clone(), &mut CodeIndex::new()));
                    let names = for_in_property_names(target)
                        .into_iter()
                        .map(|name| Rc::new(RefCell::new(JsValue::String(name))))
                        .collect();
                    let array = Prototype::find(env.mem.clone(), &"Array".into())
                        .1
                        .borrow()
                        .unwrap_proto("for-in keys Array prototype");
                    env.mem.borrow_mut().properties.insert(
                        "__forin_keys__".into(),
                        new_array(array, names, env.logger.clone()),
                    );

                    let sub = Prototype::new_child(
                        env.mem.clone(),
                        None,
                        [("__forloop_scope__".into(), Rc::new(RefCell::new(JsValue::Boolean(true))))],
                    );
                    sub.borrow_mut().properties.insert(
                        "__forin_i__".into(),
                        Rc::new(RefCell::new(JsValue::BigInt(0))),
                    );
                    CodeIndex::new().save_into(sub.clone(), "forloop_i");
                    env.mem.borrow_mut().properties.insert(
                        "__forloop_sub__".into(),
                        Rc::new(RefCell::new(JsValue::Prototype(sub))),
                    );

                    let keys = Prototype::find(env.mem.clone(), &"__forin_keys__".into())
                        .1
                        .borrow()
                        .unwrap_proto("for-in keys");
                    let JsValue::BigInt(length) =
                        inline_borrow!(Prototype::find(keys, &"length".into()).1)
                    else {
                        panic!("for-in keys length is not BigInt");
                    };
                    if length == 0 {
                        _i.skip(2);
                    }
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }),
                Box::new(move |env, _i| {
                    let sub = inline_borrow!(
                        env.mem.borrow().properties[&"__forloop_sub__".into()].clone()
                    )
                    .unwrap_proto("for-in sub not proto");
                    let JsValue::BigInt(index) =
                        inline_borrow!(Prototype::find(sub.clone(), &"__forin_i__".into()).1)
                    else {
                        panic!("for-in index is not BigInt");
                    };
                    let keys = Prototype::find(env.mem.clone(), &"__forin_keys__".into())
                        .1
                        .borrow()
                        .unwrap_proto("for-in keys");
                    let key = Prototype::find(keys, &JsValue::BigInt(index)).1;
                    env.mem
                        .borrow_mut()
                        .properties
                        .insert(for_in_name.clone().into(), key);

                    sub.borrow_mut().properties.insert(
                        "__forin_i__".into(),
                        Rc::new(RefCell::new(JsValue::BigInt(index + 1))),
                    );

                    let mut i = CodeIndex::load_from(sub.clone(), "forloop_i");
                    if i.current < body.len() {
                        let res = run_sub(&body, env.with_mem(sub.clone()), &mut i);
                        match &res {
                            CodeResult::Normal(_)
                            | CodeResult::NormalMember(_, _, _)
                            | CodeResult::Continue(None) => {}
                            CodeResult::Continue(Some(_)) => return res,
                            CodeResult::Break(None) | CodeResult::YieldBreak => {
                                _i.move_iamount(1);
                                _i.reset_retry();
                                i.reset();
                                i.save_into(sub, "forloop_i");
                                return CodeResult::Normal(Rc::new(RefCell::new(
                                    JsValue::Undefined,
                                )));
                            }
                            CodeResult::Break(Some(name))
                                if label.as_deref() == Some(name.as_str()) =>
                            {
                                _i.move_iamount(1);
                                _i.reset_retry();
                                i.reset();
                                i.save_into(sub, "forloop_i");
                                return CodeResult::Normal(Rc::new(RefCell::new(
                                    JsValue::Undefined,
                                )));
                            }
                            CodeResult::Break(Some(_)) => return res,
                            CodeResult::Return(_) | CodeResult::Error(_) => return res,
                            CodeResult::Yield(res) => {
                                i.next();
                                i.set_retry();
                                i.save_into(sub, "forloop_i");
                                _i.set_retry();
                                return CodeResult::Yield(res.clone());
                            }
                        }
                    }
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }),
                Box::new(move |env, _i| {
                    let sub = inline_borrow!(
                        env.mem.borrow().properties[&"__forloop_sub__".into()].clone()
                    )
                    .unwrap_proto("for-in sub not proto");
                    let JsValue::BigInt(index) =
                        inline_borrow!(Prototype::find(sub, &"__forin_i__".into()).1)
                    else {
                        panic!("for-in index is not BigInt");
                    };
                    let keys = Prototype::find(env.mem, &"__forin_keys__".into())
                        .1
                        .borrow()
                        .unwrap_proto("for-in keys");
                    let JsValue::BigInt(length) =
                        inline_borrow!(Prototype::find(keys, &"length".into()).1)
                    else {
                        panic!("for-in keys length is not BigInt");
                    };
                    if index < length {
                        _i.move_iamount(-1);
                        _i.set_retry();
                    }
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }),
            ];
        }
        let do_first = self.do_first;
        let label = self.label.clone();
        let init: Vec<Code> = self.init.compile(env.clone());
        let condition: Vec<Code> = self.condition.compile(env.clone());
        let update: Vec<Code> = self.update.compile(env.clone());
        let body: Vec<Code> = self.body.compile(env.clone());

        vec![
            Box::new(move |env, _i| {
                handle_return!(run_sub(&init, env.clone(), &mut CodeIndex::new()));

                let sub = Prototype::new_child(
                    env.mem.clone(),
                    None,
                    [("__forloop_scope__".into(), Rc::new(RefCell::new(JsValue::Boolean(true))))],
                );
                env.mem.borrow_mut().properties.insert(
                    "__forloop_sub__".into(),
                    Rc::new(RefCell::new(JsValue::Prototype(sub.clone()))),
                );
                CodeIndex::new().save_into(sub.clone(), "forloop_i");

                if !do_first {
                    _i.skip(1);
                }
                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            }),
            Box::new(move |env, _i| {
                let sub =
                    inline_borrow!(env.mem.borrow().properties[&"__forloop_sub__".into()].clone())
                        .unwrap_proto("sub not proto in loop body?");
                let mut i = CodeIndex::load_from(sub.clone(), "forloop_i");
                if i.current < body.len() {
                    let res = run_sub(&body, env.with_mem(sub.clone()), &mut i);
                    //TODO handle correctly his label
                    match &res {
                        CodeResult::Normal(_)
                        | CodeResult::NormalMember(_, _, _)
                        | CodeResult::Continue(None) => {}
                        CodeResult::Continue(Some(_)) => return res,
                        CodeResult::Break(None) | CodeResult::YieldBreak => {
                            _i.move_iamount(1);
                            _i.reset_retry();
                            i.reset();
                            i.save_into(sub, "forloop_i");
                            return CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)));
                        }
                        CodeResult::Break(Some(name))
                            if label.as_deref() == Some(name.as_str()) =>
                        {
                            _i.move_iamount(1);
                            _i.reset_retry();
                            i.reset();
                            i.save_into(sub, "forloop_i");
                            return CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)));
                        }
                        CodeResult::Break(Some(_)) => return res,
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

                handle_return!(run_sub(&update, env.clone(), &mut CodeIndex::new()));

                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            }),
            Box::new(move |env, _i| {
                let cond = handle_return!(run_sub(&condition, env.clone(), &mut CodeIndex::new()));
                env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                    format!(
                        "LoopExpr condition index={} truthy={}",
                        _i.current(),
                        cond.borrow().is_truthy()
                    )
                });
                if cond.borrow().is_truthy() {
                    let sub = Prototype::new_child(env.mem.clone(), None, []);
                    env.mem.borrow_mut().properties.insert(
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
            for_in: self
                .for_in
                .as_ref()
                .map(|(name, expr)| (name.clone(), expr.duplicate())),
            for_of: self
                .for_of
                .as_ref()
                .map(|(name, expr)| (name.clone(), expr.duplicate())),
            label: self.label.clone(),
        })
    }
}
