use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, JsValue, LogLevel, handle_return, inline_borrow,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::Parser,
    },
    run_sub,
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
        let mut lhs = match parser.tokens()[parser.index()] {
            Token::Plus => {
                parser.bump();
                Self::parse_binary(parser, 26)
            }
            Token::Minus => {
                parser.bump();
                Box::new(expr::Operator {
                    left: Box::new(expr::ConstNumber { num: 0.0 }),
                    op: expr::BinaryOp::Sub,
                    right: Self::parse_binary(parser, 26),
                }) as Box<dyn Expr>
            }
            Token::PlusPlus => {
                parser.bump();
                Box::new(expr::Prefix {
                    expr: Self::parse_binary(parser, 26),
                    inc: true,
                }) as Box<dyn Expr>
            }
            Token::MinusMinus => {
                parser.bump();
                Box::new(expr::Prefix {
                    expr: Self::parse_binary(parser, 26),
                    inc: false,
                }) as Box<dyn Expr>
            }
            Token::Not => {
                parser.bump();
                Box::new(expr::Not {
                    expr: Self::parse_binary(parser, 26),
                }) as Box<dyn Expr>
            }
            _ => parser.parse_call_or_primary(min_bp == 0),
        };
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
                Token::InstanceOf => {
                    parser.bump();
                    let class = Self::parse_binary(parser, 19);
                    lhs = Box::new(expr::Call {
                        args: vec![lhs],
                        func: Box::new(expr::Member {
                            object: class,
                            property: Box::new(expr::Member {
                                object: Box::new(expr::Identifier {
                                    name: stringify!(Symbol).to_owned(),
                                }),
                                property: Box::new(expr::ConstString {
                                    s: "hasInstance".to_owned(),
                                }),
                            }),
                        }),
                    });
                    continue;
                }
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
                    lhs = Box::new(expr::ConditionalOp::parse(parser, lhs));
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
            (Token::InstanceOf | Token::Lt | Token::Gt | Token::LtEq | Token::GtEq, _) => {
                Some((18, 19))
            }
            (Token::ShiftL | Token::ShiftR, _) => Some((20, 21)),
            (Token::Plus | Token::Minus, _) => Some((22, 23)),
            (Token::Star | Token::Slash | Token::Mod, _) => Some((24, 25)),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Not {
    pub expr: Box<dyn Expr>,
}

impl Expr for Not {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let expr = self.expr.compile(env);
        vec![Box::new(move |env, _| {
            let value = handle_return!(run_sub(&expr, env, &mut CodeIndex::new()));
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Boolean(
                !inline_borrow!(value).is_truthy(),
            ))))
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            expr: self.expr.duplicate(),
        })
    }
}

#[derive(Debug)]
pub struct Delete {
    pub expr: Box<dyn Expr>,
}

impl Expr for Delete {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let expr = self.expr.compile(env);
        vec![Box::new(move |env, _| {
            let result = run_sub(&expr, env.clone(), &mut CodeIndex::new());
            let (object, key) = match result {
                CodeResult::NormalMember(_, object, key) => (object, key),
                _ => {
                    return CodeResult::Normal(Rc::new(RefCell::new(JsValue::Boolean(true))));
                }
            };
            object.borrow_mut().properties.remove(&inline_borrow!(key));
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Boolean(true))))
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            expr: self.expr.duplicate(),
        })
    }
}

impl Expr for Operator {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let left = self.left.compile(env.clone());
        let right = self.right.compile(env.clone());
        let op = self.op.clone();
        vec![Box::new(move |env, _| {
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Entering Expr::Operator op={:?}", op)
            });
            let left_value = handle_return!(run_sub(&left, env.clone(), &mut CodeIndex::new()));
            if op == BinaryOp::LogAnd || op == BinaryOp::LogOr {
                let left_truthy = left_value.borrow().is_truthy();
                let should_return_left =
                    (op == BinaryOp::LogAnd && !left_truthy)
                        || (op == BinaryOp::LogOr && left_truthy);
                if should_return_left {
                    return CodeResult::Normal(left_value);
                }
                return CodeResult::Normal(handle_return!(run_sub(
                    &right,
                    env.clone(),
                    &mut CodeIndex::new()
                )));
            }
            let l = inline_borrow!(left_value);
            let r = inline_borrow!(handle_return!(run_sub(
                &right,
                env.clone(),
                &mut CodeIndex::new()
            )));
            let value = match op {
                BinaryOp::Add => match (l, r) {
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => JsValue::BigInt(a + b),
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Number(a as f64 + b),
                    (JsValue::BigInt(a), JsValue::Boolean(b)) => {
                        JsValue::BigInt(a + if b { 1 } else { 0 })
                    }
                    (JsValue::Number(a), JsValue::BigInt(b)) => JsValue::Number(a + b as f64),
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Number(a + b),
                    (JsValue::Number(a), JsValue::Boolean(b)) => {
                        JsValue::Number(a + if b { 1.0 } else { 0.0 })
                    }
                    (JsValue::Boolean(a), JsValue::BigInt(b)) => {
                        JsValue::BigInt(if a { 1 } else { 0 } + b)
                    }
                    (JsValue::Boolean(a), JsValue::Number(b)) => {
                        JsValue::Number(if a { 1.0 } else { 0.0 } + b)
                    }
                    (JsValue::Boolean(a), JsValue::Boolean(b)) => {
                        JsValue::BigInt(if a { 1 } else { 0 } + if b { 1 } else { 0 })
                    }
                    (JsValue::String(a), b) => JsValue::String(a + &b.print()),
                    (a, JsValue::String(b)) => JsValue::String(a.print() + &b),
                    _ => JsValue::Undefined,
                },
                BinaryOp::Sub => match (
                    if let JsValue::String(s) = l {
                        if let Ok(n) = s.parse::<i64>() {
                            JsValue::BigInt(n)
                        } else if let Ok(n) = s.parse::<f64>() {
                            JsValue::Number(n)
                        } else if s.trim().to_lowercase().eq("true") {
                            JsValue::Boolean(true)
                        } else if s.trim().to_lowercase().eq("false") {
                            JsValue::Boolean(false)
                        } else {
                            JsValue::String(s)
                        }
                    } else {
                        l
                    },
                    if let JsValue::String(s) = r {
                        if let Ok(n) = s.parse::<i64>() {
                            JsValue::BigInt(n)
                        } else if let Ok(n) = s.parse::<f64>() {
                            JsValue::Number(n)
                        } else if s.trim().to_lowercase().eq("true") {
                            JsValue::Boolean(true)
                        } else if s.trim().to_lowercase().eq("false") {
                            JsValue::Boolean(false)
                        } else {
                            JsValue::String(s)
                        }
                    } else {
                        r
                    },
                ) {
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => JsValue::BigInt(a - b),
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Number(a as f64 - b),
                    (JsValue::BigInt(a), JsValue::Boolean(b)) => {
                        JsValue::BigInt(a - if b { 1 } else { 0 })
                    }
                    (JsValue::Number(a), JsValue::BigInt(b)) => JsValue::Number(a - b as f64),
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Number(a - b),
                    (JsValue::Number(a), JsValue::Boolean(b)) => {
                        JsValue::Number(a - if b { 1.0 } else { 0.0 })
                    }
                    (JsValue::Boolean(a), JsValue::BigInt(b)) => {
                        JsValue::BigInt(if a { 1 } else { 0 } - b)
                    }
                    (JsValue::Boolean(a), JsValue::Number(b)) => {
                        JsValue::Number(if a { 1.0 } else { 0.0 } - b)
                    }
                    (JsValue::Boolean(a), JsValue::Boolean(b)) => {
                        JsValue::BigInt(if a { 1 } else { 0 } - if b { 1 } else { 0 })
                    }
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
                    (JsValue::BigInt(0), JsValue::BigInt(0)) => JsValue::Number(f64::NAN),
                    (JsValue::BigInt(a), JsValue::BigInt(0)) => {
                        JsValue::BigInt(if a > 0 { i64::MAX } else { i64::MIN })
                    }
                    (JsValue::BigInt(a), JsValue::BigInt(b)) => {
                        if a % b == 0 {
                            JsValue::BigInt(a / b)
                        } else {
                            JsValue::Number(a as f64 / b as f64)
                        }
                    }
                    (JsValue::BigInt(0), JsValue::Number(0.0)) => JsValue::Number(f64::NAN),
                    (JsValue::BigInt(a), JsValue::Number(0.0)) => {
                        JsValue::BigInt(if a > 0 { i64::MAX } else { i64::MIN })
                    }
                    (JsValue::BigInt(a), JsValue::Number(b)) => JsValue::Number(a as f64 / b),
                    (JsValue::Number(a), JsValue::BigInt(0))
                    | (JsValue::Number(a), JsValue::Number(0.0)) => JsValue::Number(if a == 0.0 {
                        f64::NAN
                    } else if a > 0.0 {
                        f64::INFINITY
                    } else {
                        f64::NEG_INFINITY
                    }),
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
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::Operator result={:?}", out)
            });
            CodeResult::Normal(out)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            left: self.left.duplicate(),
            op: self.op.clone(),
            right: self.right.duplicate(),
        })
    }
}

#[derive(Debug)]
pub struct ConditionalOp {
    pub cond: Box<dyn Expr>,
    pub t: Box<dyn Expr>,
    pub f: Box<dyn Expr>,
}

impl ConditionalOp {
    pub fn parse(parser: &mut Parser, cond: Box<dyn Expr>) -> ConditionalOp {
        parser.bump();
        let mut t = parser.parse_expression(false);
        let t: Box<dyn Expr> = if t.len() == 1 {
            t.pop().expect("len == 1")
        } else if t.is_empty() {
            Box::new(expr::ConstObj {
                obj: JsValue::Undefined,
            })
        } else {
            Box::new(t)
        };

        let f = if parser.tokens()[parser.index()] != Token::Colon {
            Box::new(expr::ConstObj {
                obj: JsValue::Undefined,
            }) as Box<dyn Expr>
        } else {
            parser.bump();
            let mut f = parser.parse_expression(false);
            if f.len() == 1 {
                f.pop().expect("len == 1")
            } else if f.is_empty() {
                Box::new(expr::ConstObj {
                    obj: JsValue::Undefined,
                })
            } else {
                Box::new(f) as Box<dyn Expr>
            }
        };
        expr::ConditionalOp { cond, t, f }
    }
}

impl Expr for ConditionalOp {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let cond = self.cond.compile(env.clone());
        let t = self.t.compile(env.clone());
        let f = self.f.compile(env);
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::ConditionalOp");
            let expr = if handle_return!(run_sub(&cond, env.clone(), &mut CodeIndex::new()))
                .borrow()
                .is_truthy()
            {
                &t
            } else {
                &f
            };
            let res = run_sub(expr, env.clone(), &mut CodeIndex::new());
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::ConditionalOp res={:?}", res)
            });
            res
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            cond: self.cond.duplicate(),
            t: self.t.duplicate(),
            f: self.f.duplicate(),
        })
    }
}
