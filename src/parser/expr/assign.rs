use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, JsValue, LogLevel, Prototype, handle_return,
    inline_borrow,
    parser::{
        expr::{BinaryOp, Expr},
        lexer::Token,
        parser::Parser,
    },
    run_sub,
};

#[derive(Debug)]
pub struct Assign {
    pub target: Box<dyn Expr>,
    pub value: Box<dyn Expr>,
}

impl Expr for Assign {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let target = self.target.compile(env.clone());
        let value = self.value.compile(env.clone());
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Assign");
            let value = handle_return!(run_sub(&value, env.clone(), &mut CodeIndex::new()));
            let (obj, key) = match run_sub(&target, env.clone(), &mut CodeIndex::new()) {
                CodeResult::Normal(key) => {
                    let key_clone = key.clone();
                    let key_val = inline_borrow!(key_clone.clone());
                    if let Some((found_proto, _)) = Prototype::opt_find(env.mem.clone(), &key_val) {
                        (found_proto, key_clone)
                    } else {
                        (env.mem.clone(), key_clone)
                    }
                }
                CodeResult::NormalMember(_, obj, key) => {
                    // Distinguish identifier targets (obj == current proto) from member targets.
                    let key_clone = key.clone();
                    let key_val = inline_borrow!(key_clone.clone());
                    if std::rc::Rc::ptr_eq(&obj, &env.mem) {
                        // Identifier: update nearest binding on the prototype chain.
                        if let Some((found_proto, _)) = Prototype::opt_find(obj.clone(), &key_val) {
                            (found_proto, key_clone)
                        } else {
                            (obj, key_clone)
                        }
                    } else {
                        // Member access: write to the object itself (create/overwrite own property).
                        (obj, key_clone)
                    }
                }
                _ => {
                    env.logger.borrow_mut().logln_str(
                        LogLevel::Error,
                        "Expr::Assign target did not resolve to a member",
                    );
                    panic!("asign not a member")
                }
            };
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::Assign {obj:?}[{key:?}] = {value:?}")
            });
            obj.borrow_mut()
                .properties
                .insert(inline_borrow!(key), value.clone());
            CodeResult::Normal(value)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            target: self.target.as_ref().duplicate(),
            value: self.value.as_ref().duplicate(),
        })
    }
}

#[derive(Debug)]
pub struct VarDecl {
    pub name: String,
    pub initializer: Option<Box<dyn Expr>>,
}

impl VarDecl {
    pub fn parse(parser: &mut Parser) -> Self {
        let name = parser.expect_ident();
        parser.env.logger.borrow_mut().logln(LogLevel::Info, &|| {
            format!("Entering VarDecl::parse name={}", name)
        });
        let initializer: Option<Box<dyn Expr>> =
            if let Token::Assign(t) = &parser.tokens()[parser.index()] {
                assert_eq!(*t, Option::<BinaryOp>::None);
                parser.bump();
                Some(Box::new(parser.parse_expression(false)))
            } else {
                None
            };
        if let Token::Semicolon = parser.tokens()[parser.index()] {
            parser.bump();
        }
        Self { name, initializer }
    }
}

impl Expr for VarDecl {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let name = self.name.clone();
        let code = self.initializer.compile(env);
        vec![Box::new(move |env, _| {
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Entering Expr::VarDecl name={}", name)
            });
            let value = handle_return!(run_sub(&code, env.clone(), &mut CodeIndex::new()));
            env.mem
                .borrow_mut()
                .properties
                .insert(name.clone().into(), value.clone());
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::VarDecl name={} value={:?}", name, value)
            });
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            name: self.name.clone(),
            initializer: self.initializer.as_ref().map(|a| a.as_ref().duplicate()),
        })
    }
}
