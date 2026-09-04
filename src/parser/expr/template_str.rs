use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, JsValue, LogLevel, handle_return,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::Parser,
    },
    run_sub,
};

#[derive(Debug)]
pub enum TemplatePart {
    String(String),
    Expr(Box<dyn Expr>),
}

enum CompiledTemplatePart {
    String(String),
    Expr(Vec<Code>),
}

#[derive(Debug)]
pub struct TemplateLiteral {
    pub parts: Vec<TemplatePart>,
}

impl TemplateLiteral {
    pub fn parse(parser: &mut Parser) -> Self {
        let mut parts = Vec::new();
        while !matches!(
            parser.tokens()[parser.index()],
            Token::TemplateEnd | Token::Eof
        ) {
            match &parser.tokens()[parser.index()] {
                Token::TemplateString(value) => {
                    parts.push(expr::TemplatePart::String(value.clone()));
                    parser.bump();
                }
                Token::TemplateExprStart => {
                    parser.bump();
                    let expr = Box::new(parser.parse_expression(true));
                    if !matches!(parser.tokens()[parser.index()], Token::RBrace) {
                        parser.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                            format!(
                                "TemplateLiteral::parse expected '}}' at index {} but found {:?}",
                                parser.index(),
                                parser.tokens()[parser.index()]
                            )
                        });
                        panic!("expected '}}' after template expression");
                    }
                    parser.bump();
                    parts.push(expr::TemplatePart::Expr(expr));
                }
                _ => {
                    parser.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                        format!(
                            "TemplateLiteral::parse unexpected token at index {}: {:?}",
                            parser.index(),
                            parser.tokens()[parser.index()]
                        )
                    });
                    panic!(
                        "unexpected template token: {:?}",
                        parser.tokens()[parser.index()]
                    );
                }
            }
        }
        if matches!(parser.tokens()[parser.index()], Token::TemplateEnd) {
            parser.bump();
        }
        Self { parts }
    }
}

impl Expr for TemplateLiteral {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let parts: Vec<CompiledTemplatePart> = self
            .parts
            .iter()
            .map(|part| match part {
                TemplatePart::String(s) => CompiledTemplatePart::String(s.clone()),
                TemplatePart::Expr(e) => CompiledTemplatePart::Expr(e.compile(env.clone())),
            })
            .collect();
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::TemplateLiteral");
            let mut result = String::new();
            for part in &parts {
                match part {
                    CompiledTemplatePart::String(value) => result.push_str(value),
                    CompiledTemplatePart::Expr(expr) => {
                        result.push_str(
                            &handle_return!(run_sub(expr, env.clone(), &mut CodeIndex::new()))
                                .borrow()
                                .print(),
                        );
                    }
                }
            }
            let out = Rc::new(RefCell::new(JsValue::String(result)));
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::TemplateLiteral result={:?}", out)
            });
            CodeResult::Normal(out)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            parts: self
                .parts
                .iter()
                .map(|a| match a {
                    TemplatePart::String(a) => TemplatePart::String(a.clone()),
                    TemplatePart::Expr(a) => TemplatePart::Expr(a.duplicate()),
                })
                .collect(),
        })
    }
}
