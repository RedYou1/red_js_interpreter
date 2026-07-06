use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, inline_borrow, logln, new_array,
    parser::expr::Expr,
};

pub struct Object {
    pub properties: Vec<(Box<dyn Expr>, Box<dyn Expr>)>,
}

impl Expr for Object {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let elems: Vec<(Code, Code)> = self
            .properties
            .iter()
            .map(|(key, value)| {
                (
                    key.compile_expr(mem.clone()),
                    value.compile_expr(mem.clone()),
                )
            })
            .collect();
        Box::new(move |proto, _| {
            logln(LogLevel::Trace, "Entering Expr::Object");
            let object_proto = Prototype::find(mem.clone(), &stringify!(Object).into())
                .1
                .borrow()
                .unwrap_proto("expr::Object for Object");
            let props: Vec<(JsValue, Rc<RefCell<JsValue>>)> = elems
                .iter()
                .map(|(key, value)| {
                    (
                        inline_borrow!(key(proto.clone(), &mut CodeIndex::new()).unwrap_normal()),
                        value(proto.clone(), &mut CodeIndex::new()).unwrap_normal(),
                    )
                })
                .collect();
            let out = Rc::new(RefCell::new(JsValue::Prototype(Prototype::new_child(
                object_proto,
                None,
                props,
            ))));
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Object result={:?}", out),
            );
            CodeResult::Normal(out)
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            properties: self
                .properties
                .iter()
                .map(|(a, b)| (a.duplicate_expr(), b.duplicate_expr()))
                .collect(),
        })
    }
}

pub struct Array {
    pub elems: Vec<Box<dyn Expr>>,
}

impl Expr for Array {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let elems: Vec<Code> = self
            .elems
            .iter()
            .map(|elem| elem.compile_expr(mem.clone()))
            .collect();
        Box::new(move |proto, _| {
            logln(LogLevel::Trace, "Entering Expr::Array");
            let array_proto = Prototype::find(mem.clone(), &stringify!(Array).into())
                .1
                .borrow()
                .unwrap_proto("expr::Array for Array");
            let values = elems
                .iter()
                .map(|elem| elem(proto.clone(), &mut CodeIndex::new()).unwrap_normal())
                .collect();
            let out = new_array(array_proto, values);
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Array result={:?}", out),
            );
            CodeResult::Normal(out)
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            elems: self.elems.iter().map(|a| a.duplicate_expr()).collect(),
        })
    }
}
