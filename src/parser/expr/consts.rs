use std::{cell::RefCell, rc::Rc};

use crate::{Code, CodeResult, JsValue, LogLevel, Prototype, logln, parser::expr::Expr};

#[derive(Debug, Clone)]
pub struct ConstBigInt {
    pub num: i64,
}

impl Expr for ConstBigInt {
    fn compile(&self, _: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let num = self.num;
        vec![Box::new(move |_, _| {
            logln(
                LogLevel::Trace,
                &format!("Entering Expr::ConstBigInt num={:?}", num),
            );
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::BigInt(num))))
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ConstNumber {
    pub num: f64,
}

impl Expr for ConstNumber {
    fn compile(&self, _: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let num = self.num;
        vec![Box::new(move |_, _| {
            logln(
                LogLevel::Trace,
                &format!("Entering Expr::ConstNumber num={:?}", num),
            );
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Number(num))))
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ConstString {
    pub s: String,
}

impl Expr for ConstString {
    fn compile(&self, _: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let s = self.s.clone();
        vec![Box::new(move |_, _| {
            logln(
                LogLevel::Trace,
                &format!("Entering Expr::ConstString s={:?}", s),
            );
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::String(s.clone()))))
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ConstBoolean {
    pub b: bool,
}

impl Expr for ConstBoolean {
    fn compile(&self, _: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let b = self.b;
        vec![Box::new(move |_, _| {
            logln(
                LogLevel::Trace,
                &format!("Entering Expr::ConstBoolean b={:?}", b),
            );
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Boolean(b))))
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ConstObj {
    pub obj: JsValue,
}

impl Expr for ConstObj {
    fn compile(&self, _: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let obj = self.obj.clone();
        vec![Box::new(move |_, _| {
            logln(
                LogLevel::Trace,
                &format!("Entering Expr::ConstObj obj={:?}", obj),
            );
            CodeResult::Normal(Rc::new(RefCell::new(obj.clone())))
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(self.clone())
    }
}
