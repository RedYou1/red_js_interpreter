use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, JsValue, LogLevel, Prototype, handle_return, logln,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::Parser,
    },
};

pub enum TemplatePart {
    String(String),
    Expr(Box<dyn Expr>),
}

enum CompiledTemplatePart {
    String(String),
    Expr(Code),
}

pub struct TemplateLiteral {
    pub parts: Vec<TemplatePart>,
}

impl TemplateLiteral {
    pub fn parse(parser: &mut Parser) -> Self {
        let mut parts = Vec::new();
        while !matches!(parser.tokens()[parser.index()], Token::TemplateEnd | Token::Eof) {
            match &parser.tokens()[parser.index()] {
                Token::TemplateString(value) => {
                    parts.push(expr::TemplatePart::String(value.clone()));
                    parser.bump();
                }
                Token::TemplateExprStart => {
                    parser.bump();
                    let expr = parser.parse_expression();
                    if !matches!(parser.tokens()[parser.index()], Token::RBrace) {
                        panic!("expected '}}' after template expression");
                    }
                    parser.bump();
                    parts.push(expr::TemplatePart::Expr(expr));
                }
                _ => {
                    panic!("unexpected template token: {:?}", parser.tokens()[parser.index()]);
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
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let parts: Vec<CompiledTemplatePart> = self
            .parts
            .iter()
            .map(|part| match part {
                TemplatePart::String(s) => CompiledTemplatePart::String(s.clone()),
                TemplatePart::Expr(e) => CompiledTemplatePart::Expr(e.compile_expr(mem.clone())),
            })
            .collect();
        Box::new(move |proto, i| {
            logln(LogLevel::Trace, "Entering Expr::TemplateLiteral");
            let mut result = String::new();
            for part in &parts {
                match part {
                    CompiledTemplatePart::String(value) => result.push_str(value),
                    CompiledTemplatePart::Expr(expr) => {
                        result.push_str(&handle_return!(expr(proto.clone(), i)).borrow().print());
                    }
                }
            }
            let out = Rc::new(RefCell::new(JsValue::String(result)));
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::TemplateLiteral result={:?}", out),
            );
            CodeResult::Normal(out)
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            parts: self
                .parts
                .iter()
                .map(|a| match a {
                    TemplatePart::String(a) => TemplatePart::String(a.clone()),
                    TemplatePart::Expr(a) => TemplatePart::Expr(a.duplicate_expr()),
                })
                .collect(),
        })
    }
}
