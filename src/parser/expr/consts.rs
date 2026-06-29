use std::{cell::RefCell, rc::Rc};

use crate::{Code, CodeResult, JsValue, Prototype, parser::expr::Expr};

pub struct ConstBigInt {
    pub num: i64,
}

impl Expr for ConstBigInt {
    fn compile_expr(&self, _: Rc<RefCell<Prototype>>) -> Code {
        let num = self.num;
        Box::new(move |_, _| CodeResult::Normal(Rc::new(RefCell::new(JsValue::BigInt(num)))))
    }
}

pub struct ConstNumber {
    pub num: f64,
}

impl Expr for ConstNumber {
    fn compile_expr(&self, _: Rc<RefCell<Prototype>>) -> Code {
        let num = self.num;
        Box::new(move |_, _| CodeResult::Normal(Rc::new(RefCell::new(JsValue::Number(num)))))
    }
}

pub struct ConstString {
    pub s: String,
}

impl Expr for ConstString {
    fn compile_expr(&self, _: Rc<RefCell<Prototype>>) -> Code {
        let s = self.s.clone();
        Box::new(move |_, _| CodeResult::Normal(Rc::new(RefCell::new(JsValue::String(s.clone())))))
    }
}

pub struct ConstBoolean {
    pub b: bool,
}

impl Expr for ConstBoolean {
    fn compile_expr(&self, _: Rc<RefCell<Prototype>>) -> Code {
        let b = self.b;
        Box::new(move |_, _| CodeResult::Normal(Rc::new(RefCell::new(JsValue::Boolean(b)))))
    }
}

pub struct ConstObj {
    pub obj: JsValue,
}

impl Expr for ConstObj {
    fn compile_expr(&self, _: Rc<RefCell<Prototype>>) -> Code {
        let obj = self.obj.clone();
        Box::new(move |_, _| 
            CodeResult::Normal(Rc::new(RefCell::new(obj.clone()))))
    }
}
