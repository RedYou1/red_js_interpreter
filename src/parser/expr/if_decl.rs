use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, logln,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::Parser,
    },
    run_sub,
};

#[derive(Debug)]
pub struct IfExpr {
    pub blocks: Vec<(Box<dyn Expr>, Vec<Box<dyn Expr>>)>,
}

impl IfExpr {
    fn parse_body(parser: &mut Parser) -> Vec<Box<dyn Expr>> {
        if parser.tokens()[parser.index()] == Token::LBrace {
            parser.parse_block()
        } else {
            let body = parser.parse_expression(false);
            if parser.tokens()[parser.index()] == Token::Semicolon {
                parser.bump();
            }
            body
        }
    }

    pub fn parse(parser: &mut Parser) -> Self {
        logln(LogLevel::Info, "Entering IfExpr::parse");
        if !matches!(parser.tokens()[parser.index()], Token::LParen) {
            logln(
                LogLevel::Fatal,
                &format!(
                    "IfExpr::parse expected '(' at index {} but found {:?}",
                    parser.index(),
                    parser.tokens()[parser.index()]
                ),
            );
            panic!("expected '(' after 'if'");
        }
        parser.bump();
        let condition = Box::new(parser.parse_expression(true));
        if !matches!(parser.tokens()[parser.index()], Token::RParen) {
            logln(
                LogLevel::Fatal,
                &format!(
                    "IfExpr::parse expected ')' at index {} but found {:?}",
                    parser.index(),
                    parser.tokens()[parser.index()]
                ),
            );
            panic!("expected ')' after if condition, {condition:?}");
        }
        parser.bump();

        let mut blocks: Vec<(Box<dyn Expr>, Vec<Box<dyn Expr>>)> =
            vec![(condition, Self::parse_body(parser))];

        while let Token::Else = parser.tokens()[parser.index()] {
            parser.bump();
            let condition: Box<dyn Expr> = if let Token::If = parser.tokens()[parser.index()] {
                parser.bump();
                if !matches!(parser.tokens()[parser.index()], Token::LParen) {
                    logln(
                        LogLevel::Fatal,
                        &format!(
                            "IfExpr::parse expected else-if '(' at index {} but found {:?}",
                            parser.index(),
                            parser.tokens()[parser.index()]
                        ),
                    );
                    panic!("expected '(' after 'if'");
                }
                parser.bump();
                let condition = parser.parse_expression(true);
                if !matches!(parser.tokens()[parser.index()], Token::RParen) {
                    logln(
                        LogLevel::Fatal,
                        &format!(
                            "IfExpr::parse expected else-if ')' at index {} but found {:?}",
                            parser.index(),
                            parser.tokens()[parser.index()]
                        ),
                    );
                    panic!("expected ')' after if condition");
                }
                parser.bump();

                Box::new(condition)
            } else {
                Box::new(expr::ConstBoolean { b: true })
            };
            blocks.push((condition, Self::parse_body(parser)));
        }

        Self { blocks }
    }
}

impl Expr for IfExpr {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        logln(
            LogLevel::Info,
            &format!("Entering IfExpr::compile blocks={}", self.blocks.len()),
        );
        let blocks: Vec<(Vec<Code>, Vec<Code>)> = self
            .blocks
            .iter()
            .map(|(k, v)| (k.compile(mem.clone()), v.compile(mem.clone())))
            .collect();
        let total: usize = blocks.iter().map(|(_, v)| v.len() + 2).sum();
        let mut current: usize = 0;
        blocks
            .into_iter()
            .flat_map(|(k, mut v)| {
                current += v.len() + 2;
                let go_to_end = total - current;
                let len = v.len();
                v.insert(
                    0,
                    Box::new(move |proto, i| {
                        let cond = handle_return!(run_sub(&k, proto, &mut CodeIndex::new()));
                        if cond.borrow().is_fasly() {
                            logln(LogLevel::Trace, "IfExpr condition false; skipping branch");
                            i.move_amount(true, len + 1);
                            i.reset_retry();
                        } else {
                            logln(LogLevel::Trace, "IfExpr condition true; executing branch");
                        }
                        CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                    }),
                );
                v.push(Box::new(move |_, i| {
                    i.move_amount(true, go_to_end);
                    i.reset_retry();
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }));
                v
            })
            .collect()
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            blocks: self
                .blocks
                .iter()
                .map(|(a, b)| (a.duplicate(), b.iter().map(|a| a.duplicate()).collect()))
                .collect(),
        })
    }
}
