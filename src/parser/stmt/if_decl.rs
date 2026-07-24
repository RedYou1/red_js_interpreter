use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, JsValue, LogLevel, Prototype, handle_return, logln,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::Parser,
        stmt::Stmt,
    },
};

pub struct IfStmt {
    pub blocks: Vec<(Box<dyn Expr>, Vec<Box<dyn Stmt>>)>,
}

impl IfStmt {
    pub fn parse(parser: &mut Parser) -> Self {
        logln(LogLevel::Info, "parse_statement if statement");
        if !matches!(parser.tokens()[parser.index()], Token::LParen) {
            panic!("expected '(' after 'if'");
        }
        parser.bump();
        let condition = parser.parse_expression();
        if !matches!(parser.tokens()[parser.index()], Token::RParen) {
            panic!("expected ')' after if condition");
        }
        parser.bump();

        if !matches!(parser.tokens()[parser.index()], Token::LBrace) {
            panic!("expected '{{' after if");
        }

        let mut blocks = vec![(condition, parser.parse_block_body())];

        while let Token::Else = parser.tokens()[parser.index()] {
            parser.bump();
            let condition = if let Token::If = parser.tokens()[parser.index()] {
                if !matches!(parser.tokens()[parser.index()], Token::LParen) {
                    panic!("expected '(' after 'if'");
                }
                parser.bump();
                let condition = parser.parse_expression();
                if !matches!(parser.tokens()[parser.index()], Token::RParen) {
                    panic!("expected ')' after if condition");
                }
                parser.bump();

                if !matches!(parser.tokens()[parser.index()], Token::LBrace) {
                    panic!("expected '{{' after if");
                }
                condition
            } else {
                Box::new(expr::ConstBoolean { b: true })
            };
            blocks.push((condition, parser.parse_block_body()));
        }

        Self { blocks }
    }
}

impl Stmt for IfStmt {
    fn compile_stmt(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        logln(
            LogLevel::Info,
            &format!("IfStmt::compile_stmt blocks={}", self.blocks.len()),
        );
        let blocks: Vec<(Code, Vec<Code>)> = self
            .blocks
            .iter()
            .map(|(k, v)| (k.compile_expr(mem.clone()), v.compile_stmt(mem.clone())))
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
                        let cond = handle_return!(k(proto, i));
                        if cond.borrow().is_fasly() {
                            i.move_amount(true, len + 1);
                            i.reset_retry();
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
    fn duplicate_stmt(&self) -> Box<dyn Stmt> {
        Box::new(Self {
            blocks: self
                .blocks
                .iter()
                .map(|(a, b)| {
                    (
                        a.duplicate_expr(),
                        b.iter().map(|a| a.duplicate_stmt()).collect(),
                    )
                })
                .collect(),
        })
    }
}
