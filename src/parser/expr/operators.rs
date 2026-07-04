use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::{ParseError, Parser},
    },
};

#[derive(Clone, Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
}

pub struct Operator {
    pub left: Box<dyn Expr>,
    pub op: BinaryOp,
    pub right: Box<dyn Expr>,
}

impl Operator {
    pub fn parse(parser: &mut Parser) -> Result<Box<dyn Expr>, ParseError> {
        Self::parse_binary(parser, 0)
    }

    fn parse_binary(parser: &mut Parser, min_bp: u8) -> Result<Box<dyn Expr>, ParseError> {
        let mut lhs = parser.parse_call_or_primary()?;
        // advance tokens for loop
        while let Some((l_bp, r_bp)) = Self::precedence(parser.current()) {
            if l_bp < min_bp {
                break;
            }
            let op = match parser.current() {
                Token::Plus => expr::BinaryOp::Add,
                Token::Minus => expr::BinaryOp::Sub,
                Token::Star => expr::BinaryOp::Mul,
                Token::Slash => expr::BinaryOp::Div,
                Token::Eq => expr::BinaryOp::Eq,
                Token::NotEq => expr::BinaryOp::NotEq,
                Token::Lt => expr::BinaryOp::Lt,
                Token::Gt => expr::BinaryOp::Gt,
                Token::LtEq => expr::BinaryOp::LtEq,
                Token::GtEq => expr::BinaryOp::GtEq,
                _ => unreachable!(),
            };
            // consume operator
            parser.bump();
            let rhs = Self::parse_binary(parser, r_bp)?;
            lhs = Box::new(expr::Operator {
                left: lhs,
                op,
                right: rhs,
            });
        }
        Ok(lhs)
    }

    const fn precedence(tok: &Token) -> Option<(u8, u8)> {
        match tok {
            Token::Eq | Token::NotEq | Token::Lt | Token::Gt | Token::LtEq | Token::GtEq => {
                Some((8, 9))
            }
            Token::Plus | Token::Minus => Some((10, 11)),
            Token::Star | Token::Slash => Some((12, 13)),
            _ => None,
        }
    }
}

impl Expr for Operator {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let left = self.left.compile_expr(mem.clone());
        let right = self.right.compile_expr(mem.clone());
        let op = self.op.clone();
        Box::new(move |proto, i| {
            logln(
                LogLevel::Trace,
                &format!("Entering Expr::Operator op={:?}", op),
            );
            let l = inline_borrow!(handle_return!(left(proto.clone(), i)));
            let r = inline_borrow!(handle_return!(right(proto.clone(), i)));
            let value = match op {
                BinaryOp::Add => match (l, r) {
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => JsValue::BigInt(a + b),
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Number(a as f64 + b),
                    (JsValue::Number(a), JsValue::BigInt(b)) => JsValue::Number(a + b as f64),
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Number(a + b),
                    (JsValue::String(a), b) => JsValue::String(a + &b.print()),
                    (a, JsValue::String(b)) => JsValue::String(a.print() + &b),
                    _ => JsValue::Undefined,
                },
                BinaryOp::Sub => match (l, r) {
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => JsValue::BigInt(a - b),
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Number(a as f64 - b),
                    (JsValue::Number(a), JsValue::BigInt(b)) => JsValue::Number(a - b as f64),
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Number(a - b),
                    _ => JsValue::Undefined,
                },
                BinaryOp::Mul => match (l, r) {
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => JsValue::BigInt(a * b),
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Number(a as f64 * b),
                    (JsValue::Number(a), JsValue::BigInt(b)) => JsValue::Number(a * b as f64),
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Number(a * b),
                    _ => JsValue::Undefined,
                },
                BinaryOp::Div => match (l, r) {
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => {
                        if a % b == 0 {
                            JsValue::BigInt(a / b)
                        } else {
                            JsValue::Number(a as f64 / b as f64)
                        }
                    }
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Number(a as f64 / b),
                    (JsValue::Number(a), JsValue::BigInt(b)) => JsValue::Number(a / b as f64),
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Number(a / b),
                    _ => JsValue::Undefined,
                },
                BinaryOp::Eq => JsValue::Boolean(l.eq(&r)),
                BinaryOp::NotEq => JsValue::Boolean(l.ne(&r)),
                BinaryOp::Lt => match (l, r) {
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => JsValue::Boolean(a < b),
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Boolean((a as f64) < b),
                    (JsValue::Number(a), JsValue::BigInt(b)) => JsValue::Boolean(a < b as f64),
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a < b),
                    _ => JsValue::Boolean(false),
                },
                BinaryOp::Gt => match (l, r) {
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => JsValue::Boolean(a > b),
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Boolean((a as f64) > b),
                    (JsValue::Number(a), JsValue::BigInt(b)) => JsValue::Boolean(a > b as f64),
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a > b),
                    _ => JsValue::Boolean(false),
                },
                BinaryOp::LtEq => match (l, r) {
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => JsValue::Boolean(a <= b),
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Boolean((a as f64) <= b),
                    (JsValue::Number(a), JsValue::BigInt(b)) => JsValue::Boolean(a <= b as f64),
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a <= b),
                    _ => JsValue::Boolean(false),
                },
                BinaryOp::GtEq => match (l, r) {
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => JsValue::Boolean(a >= b),
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Boolean((a as f64) >= b),
                    (JsValue::Number(a), JsValue::BigInt(b)) => JsValue::Boolean(a >= b as f64),
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a >= b),
                    _ => JsValue::Boolean(false),
                },
            };
            let out = Rc::new(RefCell::new(value));
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Operator result={:?}", out),
            );
            CodeResult::Normal(out)
        })
    }
}
