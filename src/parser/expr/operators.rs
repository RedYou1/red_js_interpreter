use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::Parser,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    BinAnd,
    LogAnd,
    BinOr,
    LogOr,
    XOr,
    ShiftL,
    ShiftR,
}

#[derive(Debug)]
pub struct Operator {
    pub left: Box<dyn Expr>,
    pub op: BinaryOp,
    pub right: Box<dyn Expr>,
}

impl Operator {
    pub fn parse(parser: &mut Parser) -> Box<dyn Expr> {
        Self::parse_binary(parser, 0)
    }

    fn parse_binary(parser: &mut Parser, min_bp: u8) -> Box<dyn Expr> {
        let mut lhs = parser.parse_call_or_primary(min_bp == 0);
        // advance tokens for loop
        while let Some((l_bp, r_bp)) = Self::precedence(
            &parser.tokens()[parser.index()],
            parser.tokens().get(parser.index() + 1),
        ) {
            if l_bp < min_bp {
                break;
            }
            let op = match parser.tokens()[parser.index()] {
                Token::Plus => expr::BinaryOp::Add,
                Token::Minus => expr::BinaryOp::Sub,
                Token::Star => expr::BinaryOp::Mul,
                Token::Slash => expr::BinaryOp::Div,
                Token::Mod => expr::BinaryOp::Mod,
                Token::Eq => expr::BinaryOp::Eq,
                Token::NotEq => expr::BinaryOp::NotEq,
                Token::Lt => expr::BinaryOp::Lt,
                Token::Gt => expr::BinaryOp::Gt,
                Token::LtEq => expr::BinaryOp::LtEq,
                Token::GtEq => expr::BinaryOp::GtEq,
                Token::And if let Token::And = parser.tokens()[parser.index() + 1] => {
                    parser.bump();
                    expr::BinaryOp::LogAnd
                }
                Token::And => expr::BinaryOp::BinAnd,
                Token::Or if let Token::Or = parser.tokens()[parser.index() + 1] => {
                    parser.bump();
                    expr::BinaryOp::LogOr
                }
                Token::Or => expr::BinaryOp::BinOr,
                Token::XOr => expr::BinaryOp::XOr,
                Token::ShiftL => expr::BinaryOp::ShiftL,
                Token::ShiftR => expr::BinaryOp::ShiftR,
                Token::QMark if min_bp == 0 => {
                    parser.bump();
                    let t = parser.parse_expression();
                    assert_eq!(parser.tokens()[parser.index()], Token::Colon);
                    parser.bump();
                    let f = parser.parse_expression();
                    lhs = Box::new(expr::ConditionalOp { cond: lhs, t, f });
                    continue;
                }
                Token::QMark | Token::Colon => break,
                _ => unreachable!(),
            };
            // consume operator
            parser.bump();
            let rhs = Self::parse_binary(parser, r_bp);
            lhs = Box::new(expr::Operator {
                left: lhs,
                op,
                right: rhs,
            });
        }
        lhs
    }

    const fn precedence(tok: &Token, next: Option<&Token>) -> Option<(u8, u8)> {
        match (tok, next) {
            // (Token::Comma, _) => Some((2, 3)),
            (Token::QMark | Token::Colon, _) => Some((4, 5)),
            (Token::Or, Some(Token::Or)) => Some((6, 7)),
            (Token::And, Some(Token::And)) => Some((8, 9)),
            (Token::Or, _) => Some((10, 11)),
            (Token::XOr, _) => Some((12, 13)),
            (Token::And, _) => Some((14, 15)),
            (Token::Eq | Token::NotEq, _) => Some((16, 17)),
            (Token::Lt | Token::Gt | Token::LtEq | Token::GtEq, _) => Some((18, 19)),
            (Token::ShiftL | Token::ShiftR, _) => Some((20, 21)),
            (Token::Plus | Token::Minus, _) => Some((22, 23)),
            (Token::Star | Token::Slash | Token::Mod, _) => Some((24, 25)),
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
                BinaryOp::Mod => match (l, r) {
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => JsValue::BigInt(a % b),
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Number(a as f64 % b),
                    (JsValue::Number(a), JsValue::BigInt(b)) => JsValue::Number(a % b as f64),
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Number(a % b),
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
                BinaryOp::BinAnd => todo!(),
                BinaryOp::BinOr => todo!(),
                BinaryOp::LogAnd => JsValue::Boolean(l.is_truthy() && r.is_truthy()),
                BinaryOp::LogOr => JsValue::Boolean(l.is_truthy() || r.is_truthy()),
                BinaryOp::XOr => todo!(),
                BinaryOp::ShiftL => todo!(),
                BinaryOp::ShiftR => todo!(),
            };
            let out = Rc::new(RefCell::new(value));
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Operator result={:?}", out),
            );
            CodeResult::Normal(out)
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            left: self.left.duplicate_expr(),
            op: self.op.clone(),
            right: self.right.duplicate_expr(),
        })
    }
}

#[derive(Debug)]
pub struct ConditionalOp {
    pub cond: Box<dyn Expr>,
    pub t: Box<dyn Expr>,
    pub f: Box<dyn Expr>,
}

impl Expr for ConditionalOp {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let cond = self.cond.compile_expr(mem.clone());
        let t = self.t.compile_expr(mem.clone());
        let f = self.f.compile_expr(mem);
        Box::new(move |prop, _| {
            logln(LogLevel::Trace, "Entering Expr::ConditionalOp");
            let expr = if handle_return!(cond(prop.clone(), &mut CodeIndex::new()))
                .borrow()
                .is_truthy()
            {
                &t
            } else {
                &f
            };
            let res = expr(prop, &mut CodeIndex::new());
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::ConditionalOp res={:?}", res),
            );
            res
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            cond: self.cond.duplicate_expr(),
            t: self.t.duplicate_expr(),
            f: self.f.duplicate_expr(),
        })
    }
}
