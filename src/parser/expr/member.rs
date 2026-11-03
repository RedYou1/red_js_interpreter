use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, LogLevel, Prototype, RUNNABLE, handle_return,
    inline_borrow,
    parser::expr::{self, Expr},
    run_function_object, run_sub,
};

fn string_member(value: &str, key: &crate::JsValue) -> crate::JsValue {
    if matches!(key, crate::JsValue::String(name) if name == "length") {
        return crate::JsValue::BigInt(value.encode_utf16().count() as i64);
    }

    let index = match key {
        crate::JsValue::BigInt(index) if *index >= 0 => usize::try_from(*index).ok(),
        crate::JsValue::Number(index)
            if index.is_finite() && *index >= 0.0 && index.fract() == 0.0 =>
        {
            (*index < usize::MAX as f64).then_some(*index as usize)
        }
        crate::JsValue::String(index) => index
            .parse::<usize>()
            .ok()
            .filter(|parsed| parsed.to_string() == *index),
        _ => None,
    };

    index
        .and_then(|index| value.encode_utf16().nth(index))
        .map(|unit| crate::JsValue::String(String::from_utf16_lossy(&[unit])))
        .unwrap_or(crate::JsValue::Undefined)
}

#[derive(Debug)]
pub struct Member {
    pub object: Box<dyn Expr>,
    pub property: Box<dyn Expr>,
}

impl Expr for Member {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let obj = self.object.compile(env.clone());
        let prop = self.property.compile(env);
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Member");
            let object = handle_return!(run_sub(&obj, env.clone(), &mut CodeIndex::new()));
            let key = handle_return!(run_sub(&prop, env.clone(), &mut CodeIndex::new()));
            match inline_borrow!(object.clone()) {
                crate::JsValue::Prototype(object) => {
                    let out = Prototype::find(object.clone(), &inline_borrow!(key.clone())).1;
                    env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                        format!("Exiting Expr::Member {object:?}[{key:?}] == {out:?}")
                    });
                    CodeResult::NormalMember(out, object, key)
                }
                crate::JsValue::String(value) => {
                    let key_value = inline_borrow!(key.clone());
                    let member = string_member(&value, &key_value);
                    if !matches!(member, crate::JsValue::Undefined) {
                        CodeResult::Normal(Rc::new(RefCell::new(member)))
                    } else {
                        let string_prototype =
                            Prototype::find(env.mem.clone(), &stringify!(String).into())
                                .1
                                .borrow()
                                .unwrap_proto("expr::Member for String");
                        if let Some((_, property)) =
                            Prototype::opt_find(string_prototype.clone(), &key_value)
                        {
                            let receiver = Prototype::new_child(
                                string_prototype,
                                None,
                                [
                                    (
                                        "__string_data__".into(),
                                        Rc::new(RefCell::new(crate::JsValue::String(
                                            value.clone(),
                                        ))),
                                    ),
                                    (
                                        "length".into(),
                                        Rc::new(RefCell::new(crate::JsValue::BigInt(
                                            value.encode_utf16().count() as i64,
                                        ))),
                                    ),
                                ],
                            );
                            CodeResult::NormalMember(property, receiver, key)
                        } else {
                            CodeResult::Normal(Rc::new(RefCell::new(crate::JsValue::Undefined)))
                        }
                    }
                }
                value => {
                    value.unwrap_proto("expr::Member for obj");
                    unreachable!()
                }
            }
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            object: self.object.duplicate(),
            property: self.property.duplicate(),
        })
    }
}

#[derive(Debug)]
pub enum OptionalChainStep {
    Member {
        property: Box<dyn Expr>,
        optional: bool,
    },
    Call {
        arguments: Vec<expr::CallArgument>,
        optional: bool,
    },
}

#[derive(Debug)]
pub struct OptionalChain {
    pub object: Box<dyn Expr>,
    pub steps: Vec<OptionalChainStep>,
}

impl OptionalChain {
    pub fn parse(parser: &mut crate::parser::parser::Parser, object: Box<dyn Expr>) -> Self {
        let mut steps = Vec::new();
        loop {
            let optional = if parser.tokens()[parser.index()] == crate::Token::OptionalChain {
                parser.bump();
                true
            } else {
                false
            };
            match parser.tokens()[parser.index()] {
                crate::Token::LBracket => {
                    parser.bump();
                    let property = Box::new(parser.parse_expression(true));
                    if parser.tokens()[parser.index()] != crate::Token::RBracket {
                        panic!("expected ']' in optional chain");
                    }
                    parser.bump();
                    steps.push(OptionalChainStep::Member { property, optional });
                }
                crate::Token::LParen => {
                    let arguments = parser.parse_call_arguments();
                    steps.push(OptionalChainStep::Call {
                        arguments,
                        optional,
                    });
                }
                crate::Token::Dot => {
                    parser.bump();
                    let name = parser.tokens()[parser.index()].as_keyword().to_owned();
                    parser.bump();
                    steps.push(OptionalChainStep::Member {
                        property: Box::new(expr::ConstString { s: name }),
                        optional,
                    });
                }
                _ if optional => {
                    let name = parser.tokens()[parser.index()].as_keyword().to_owned();
                    parser.bump();
                    steps.push(OptionalChainStep::Member {
                        property: Box::new(expr::ConstString { s: name }),
                        optional: true,
                    });
                }
                _ => break,
            }
            if !matches!(
                parser.tokens()[parser.index()],
                crate::Token::OptionalChain
                    | crate::Token::Dot
                    | crate::Token::LBracket
                    | crate::Token::LParen
            ) {
                break;
            }
        }
        Self { object, steps }
    }
}

impl Expr for OptionalChain {
    fn compile(&self, env: Environment) -> Vec<Code> {
        enum CompiledStep {
            Member {
                property: Vec<Code>,
                optional: bool,
            },
            Call {
                arguments: Vec<(Vec<Code>, bool)>,
                optional: bool,
            },
        }

        let object = self.object.compile(env.clone());
        let steps = self
            .steps
            .iter()
            .map(|step| match step {
                OptionalChainStep::Member { property, optional } => CompiledStep::Member {
                    property: property.compile(env.clone()),
                    optional: *optional,
                },
                OptionalChainStep::Call {
                    arguments,
                    optional,
                } => CompiledStep::Call {
                    arguments: arguments
                        .iter()
                        .map(|argument| match argument {
                            expr::CallArgument::Value(argument) => {
                                (argument.compile(env.clone()), false)
                            }
                            expr::CallArgument::Spread(argument) => {
                                (argument.compile(env.clone()), true)
                            }
                        })
                        .collect(),
                    optional: *optional,
                },
            })
            .collect::<Vec<_>>();
        vec![Box::new(move |env, _| {
            let mut value = handle_return!(run_sub(&object, env.clone(), &mut CodeIndex::new()));
            let mut receiver: Option<Rc<RefCell<Prototype>>> = None;
            for step in &steps {
                match step {
                    CompiledStep::Member { property, optional } => {
                        if *optional && is_nullish(&value) {
                            return CodeResult::Normal(Rc::new(RefCell::new(
                                crate::JsValue::Undefined,
                            )));
                        }
                        let key = inline_borrow!(handle_return!(run_sub(
                            property,
                            env.clone(),
                            &mut CodeIndex::new()
                        )));
                        match inline_borrow!(value.clone()) {
                            crate::JsValue::Prototype(object) => {
                                value = Prototype::find(object.clone(), &key).1;
                                receiver = Some(object);
                            }
                            crate::JsValue::String(string) => {
                                let member = string_member(&string, &key);
                                if !matches!(member, crate::JsValue::Undefined) {
                                    value = Rc::new(RefCell::new(member));
                                    receiver = None;
                                } else {
                                    let string_prototype = Prototype::find(
                                        env.mem.clone(),
                                        &stringify!(String).into(),
                                    )
                                    .1
                                    .borrow()
                                    .unwrap_proto("OptionalChain for String");
                                    if let Some((_, property)) =
                                        Prototype::opt_find(string_prototype.clone(), &key)
                                    {
                                        let wrapper = Prototype::new_child(
                                            string_prototype,
                                            None,
                                            [(
                                                "__string_data__".into(),
                                                Rc::new(RefCell::new(crate::JsValue::String(
                                                    string.clone(),
                                                ))),
                                            )],
                                        );
                                        value = property;
                                        receiver = Some(wrapper);
                                    } else {
                                        value = Rc::new(RefCell::new(crate::JsValue::Undefined));
                                        receiver = None;
                                    }
                                }
                            }
                            crate::JsValue::Null | crate::JsValue::Undefined => {
                                return type_error(&env);
                            }
                            _ => {
                                value = Rc::new(RefCell::new(crate::JsValue::Undefined));
                                receiver = None;
                            }
                        }
                    }
                    CompiledStep::Call {
                        arguments,
                        optional,
                    } => {
                        if *optional && is_nullish(&value) {
                            return CodeResult::Normal(Rc::new(RefCell::new(
                                crate::JsValue::Undefined,
                            )));
                        }
                        let mut values = Vec::with_capacity(arguments.len());
                        for (argument, spread) in arguments {
                            let value = handle_return!(run_sub(
                                argument,
                                env.clone(),
                                &mut CodeIndex::new()
                            ));
                            if *spread {
                                match crate::prebuild::array::iterable_values(&env, value) {
                                    Ok(spread_values) => values.extend(spread_values),
                                    Err(error) => return error,
                                }
                            } else {
                                values.push(value);
                            }
                        }
                        let crate::JsValue::Prototype(function) = inline_borrow!(value.clone())
                        else {
                            return type_error(&env);
                        };
                        let Some(runnable) =
                            Prototype::opt_find(function.clone(), &RUNNABLE.into())
                                .map(|(_, runnable)| inline_borrow!(runnable))
                        else {
                            return type_error(&env);
                        };
                        let this = receiver
                            .take()
                            .map(|receiver| {
                                Rc::new(RefCell::new(crate::JsValue::Prototype(receiver)))
                            })
                            .unwrap_or_else(|| Rc::new(RefCell::new(crate::JsValue::Undefined)));
                        let result = match runnable {
                            crate::JsValue::Function(_) => {
                                run_function_object(function, this, values, env.logger.clone())
                            }
                            _ => return type_error(&env),
                        };
                        value = match result {
                            CodeResult::Normal(value)
                            | CodeResult::NormalMember(value, _, _)
                            | CodeResult::Return(value) => value,
                            other => return other,
                        };
                    }
                }
            }
            CodeResult::Normal(value)
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            object: self.object.duplicate(),
            steps: self
                .steps
                .iter()
                .map(|step| match step {
                    OptionalChainStep::Member { property, optional } => OptionalChainStep::Member {
                        property: property.duplicate(),
                        optional: *optional,
                    },
                    OptionalChainStep::Call {
                        arguments,
                        optional,
                    } => OptionalChainStep::Call {
                        arguments: arguments
                            .iter()
                            .map(|argument| match argument {
                                expr::CallArgument::Value(argument) => {
                                    expr::CallArgument::Value(argument.duplicate())
                                }
                                expr::CallArgument::Spread(argument) => {
                                    expr::CallArgument::Spread(argument.duplicate())
                                }
                            })
                            .collect(),
                        optional: *optional,
                    },
                })
                .collect(),
        })
    }
}

fn is_nullish(value: &Rc<RefCell<crate::JsValue>>) -> bool {
    matches!(
        inline_borrow!(value.clone()),
        crate::JsValue::Null | crate::JsValue::Undefined
    )
}

fn type_error(env: &Environment) -> CodeResult {
    let prototype = Prototype::find(env.mem.clone(), &stringify!(TypeError).into())
        .1
        .borrow()
        .unwrap_proto("OptionalChain TypeError");
    let constructor = Rc::new(RefCell::new(crate::JsValue::Prototype(prototype.clone())));
    CodeResult::Error(Rc::new(RefCell::new(crate::JsValue::Prototype(
        Prototype::new_child(prototype, None, [("constructor".into(), constructor)]),
    ))))
}
