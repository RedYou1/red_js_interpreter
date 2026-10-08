use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, JsValue, LogLevel, Prototype, handle_return,
    inline_borrow,
    parser::{
        expr::{self, Expr, FunctionDecl},
        lexer::Token,
        parser::Parser,
    },
    run_sub,
};

#[derive(Debug)]
pub struct ClassDecl {
    pub name: &'static str,
    pub super_class: Option<Box<dyn Expr>>,
    pub methods: Rc<[(FunctionDecl, bool)]>,
}

impl ClassDecl {
    pub fn parse(parser: &mut Parser) -> Self {
        let name = if let Token::Ident(_) = parser.tokens()[parser.index()] {
            parser.expect_ident().leak()
        } else {
            parser.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                format!(
                    "ClassDecl::parse expected class name at index {} but found {:?}",
                    parser.index(),
                    parser.tokens()[parser.index()]
                )
            });
            panic!("expected class name");
        };
        parser.env.logger.borrow_mut().logln(LogLevel::Info, &|| {
            format!("Entering ClassDecl::parse name={}", name)
        });
        let super_class = if let Token::Ident(ident) = &parser.tokens()[parser.index()] {
            if ident.eq("extends") {
                parser.bump();
                Some(parser.parse_call_or_primary(true))
            } else {
                None
            }
        } else {
            None
        };
        if !matches!(parser.tokens()[parser.index()], Token::LBrace) {
            parser.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                format!(
                    "ClassDecl::parse expected '{{' at index {} but found {:?}",
                    parser.index(),
                    parser.tokens()[parser.index()]
                )
            });
            panic!("expected '{{' after class name");
        }
        parser.bump();
        let mut methods = Vec::new();
        while !matches!(parser.tokens()[parser.index()], Token::RBrace | Token::Eof) {
            let is_static = matches!(
                (&parser.tokens()[parser.index()], parser.tokens().get(parser.index() + 1)),
                (Token::Ident(name), Some(Token::LParen)) if name == "static"
            ) == false
                && matches!(&parser.tokens()[parser.index()], Token::Ident(name) if name == "static");
            if is_static {
                parser.bump();
            }
            let generator = if matches!(parser.tokens()[parser.index()], Token::Star) {
                parser.bump();
                true
            } else {
                false
            };
            let name = match parser.tokens()[parser.index()].clone() {
                Token::Ident(name) | Token::Str(name) => {
                    parser.bump();
                    name
                }
                Token::Number(name) => {
                    parser.bump();
                    name.to_string()
                }
                Token::BigInt(name) => {
                    parser.bump();
                    name.to_string()
                }
                Token::LBracket => {
                    parser.bump();
                    let name = match parser.tokens()[parser.index()].clone() {
                        Token::Ident(name) | Token::Str(name) => name,
                        Token::Number(name) => name.to_string(),
                        Token::BigInt(name) => name.to_string(),
                        _ => {
                            parser.bump();
                            continue;
                        }
                    };
                    parser.bump();
                    parser.skip_to(Token::RBracket);
                    if matches!(parser.tokens()[parser.index()], Token::RBracket) {
                        parser.bump();
                    }
                    name
                }
                _ => {
                    parser.bump();
                    continue;
                }
            };
            let name = name.leak();
            if matches!(parser.tokens()[parser.index()], Token::LParen) {
                methods.push((
                    expr::FunctionDecl::parse_named(parser, false, name, generator),
                    is_static,
                ));
            }
        }
        let class = Self {
            name,
            super_class,
            methods: Rc::from(methods),
        };
        if let Token::RBrace = parser.tokens()[parser.index()] {
            parser.bump();
        }
        class
    }
}

impl Expr for ClassDecl {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let super_constructor: Vec<Code> = if let Some(ref expr) = self.super_class {
            expr.compile(env.clone())
        } else {
            vec![Box::new(|_, _| {
                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            })]
        };

        let constructor_method = self
            .methods
            .iter()
            .find(|(m, is_static)| !is_static && m.name == "constructor")
            .map(|(m, _)| m);

        let constructor_runnable = if let Some(constructor) = constructor_method {
            constructor.compile(env.clone())
        } else {
            FunctionDecl {
                name: "constructor",
                params: vec![],
                body: Rc::new([]),
                generator: false,
                insert: false,
                lexical_this: false,
            }
            .compile(env.clone())
        };

        let name = self.name;
        let methodes: Vec<(&'static str, Vec<Code>)> = self
            .methods
            .iter()
            .map(|(methode, _)| (methode.name, methode.compile(env.clone())))
            .collect();
        let mem = env.mem.clone();
        vec![Box::new(move |env, _| {
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Entering Expr::ClassDecl name={}", name)
            });

            let class_proto = Prototype::new_child(
                if let JsValue::Prototype(super_func_proto) = inline_borrow!(handle_return!(
                    run_sub(&super_constructor, env.clone(), &mut CodeIndex::new())
                )) && let JsValue::Prototype(super_proto_obj) =
                    inline_borrow!(Prototype::find(super_func_proto.clone(), &"prototype".into()).1)
                {
                    super_proto_obj
                } else {
                    Prototype::find(env.mem.clone(), &stringify!(Object).into())
                        .1
                        .borrow()
                        .unwrap_proto("expr::ClassDecl for Object")
                },
                Some(name),
                [],
            );
            class_proto.borrow_mut().insert_property(
                "constructor".into(),
                handle_return!(run_sub(
                    &constructor_runnable,
                    env.clone(),
                    &mut CodeIndex::new()
                )),
            );
            class_proto.borrow_mut().insert_property(
                "prototype".into(),
                Rc::new(RefCell::new(JsValue::Prototype(class_proto.clone()))),
            );

            for (methode_name, methode_code) in methodes.iter() {
                class_proto.borrow_mut().insert_property(
                    (*methode_name).into(),
                    handle_return!(run_sub(methode_code, env.clone(), &mut CodeIndex::new())),
                );
            }

            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::ClassDecl {class_proto:?}")
            });
            let res = Rc::new(RefCell::new(JsValue::Prototype(class_proto)));
            mem.borrow_mut().properties.insert(name.into(), res.clone());
            CodeResult::Normal(res)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            name: self.name,
            super_class: self.super_class.as_ref().map(|t| t.as_ref().duplicate()),
            methods: self.methods.clone(),
        })
    }
}
