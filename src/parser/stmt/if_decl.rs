use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, JsValue, LogLevel, Prototype, handle_return, logln,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::{ParseError, Parser},
        stmt::Stmt,
    },
};

pub struct IfStmt {
    pub blocks: Vec<(Box<dyn Expr>, Box<dyn Stmt>)>,
}

impl IfStmt {
    pub fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        logln(LogLevel::Info, "parse_statement if statement");
        if !matches!(parser.current(), Token::LParen) {
            return Err(ParseError("expected '(' after 'if'".into()));
        }
        parser.bump();
        let condition = parser.parse_expression()?;
        if !matches!(parser.current(), Token::RParen) {
            return Err(ParseError("expected ')' after if condition".into()));
        }
        parser.bump();

        if !matches!(parser.current(), Token::LBrace) {
            return Err(ParseError("expected '{' after if".into()));
        }

        let mut consequent = parser.parse_block_body()?;
        if consequent.len() != 1 {
            return Err(ParseError("Multiple bloc for same if".to_owned()));
        };

        let mut blocks = vec![(condition, consequent.pop().unwrap())];

        if let Token::Else = parser.current() {
            parser.bump();
            let mut consequent = parser.parse_block_body()?;
            if consequent.len() != 1 {
                return Err(ParseError("Multiple bloc for same if".to_owned()));
            };
            blocks.push((
                Box::new(expr::ConstBoolean { b: true }),
                consequent.pop().unwrap(),
            ));
        }

        Ok(Self { blocks })
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
}
