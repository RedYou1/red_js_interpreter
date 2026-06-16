use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::expr::Expr,
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

impl Expr for Operator {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let left = self.left.compile_expr(mem.clone());
        let right = self.right.compile_expr(mem.clone());
        let op = self.op.clone();
        Box::new(move |proto, i| {
            logln(LogLevel::Trace, &format!("Entering Expr::Operator op={:?}", op));
            let l = inline_borrow!(handle_return!(left(proto.clone(), i)));
            let r = inline_borrow!(handle_return!(right(proto.clone(), i)));
            let value = match op {
                BinaryOp::Add => match (l, r) {
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Number(a + b),
                    (JsValue::String(a), b) => JsValue::String(a + &b.print()),
                    (a, JsValue::String(b)) => JsValue::String(a.print() + &b),
                    _ => JsValue::Undefined,
                },
                BinaryOp::Sub => {
                    if let (JsValue::Number(a), JsValue::Number(b)) = (l, r) {
                        JsValue::Number(a - b)
                    } else {
                        JsValue::Undefined
                    }
                }
                BinaryOp::Mul => {
                    if let (JsValue::Number(a), JsValue::Number(b)) = (l, r) {
                        JsValue::Number(a * b)
                    } else {
                        JsValue::Undefined
                    }
                }
                BinaryOp::Div => {
                    if let (JsValue::Number(a), JsValue::Number(b)) = (l, r) {
                        JsValue::Number(a / b)
                    } else {
                        JsValue::Undefined
                    }
                }
                BinaryOp::Eq => JsValue::Boolean(l.eq(&r)),
                BinaryOp::NotEq => JsValue::Boolean(l.eq(&r)),
                BinaryOp::Lt => match (l, r) {
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a < b),
                    _ => JsValue::Boolean(false),
                },
                BinaryOp::Gt => match (l, r) {
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a > b),
                    _ => JsValue::Boolean(false),
                },
                BinaryOp::LtEq => match (l, r) {
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a <= b),
                    _ => JsValue::Boolean(false),
                },
                BinaryOp::GtEq => match (l, r) {
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a >= b),
                    _ => JsValue::Boolean(false),
                },
            };
            let out = Rc::new(RefCell::new(value));
            logln(LogLevel::Trace, &format!("Exiting Expr::Operator result={:?}", out));
            CodeResult::Normal(out)
        })
    }
}
