use std::{cell::RefCell, fmt::Debug, hash::Hash, rc::Rc};

use crate::{Generator, PROTO_NAME, Prototype, Runnable, inline_borrow};

#[derive(Clone)]
pub enum JsValue {
    Function(Rc<Runnable>),
    Generator(Rc<Generator>),

    Prototype(Rc<RefCell<Prototype>>),
    Symbol(u64, Box<JsValue>),
    String(String),
    Number(f64),
    BigInt(i64),
    Boolean(bool),
    Undefined,
    Null,
}

impl Eq for JsValue {}

impl PartialEq for JsValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Prototype(l0), Self::Prototype(r0)) => l0.eq(r0),
            (Self::Symbol(l0, l1), Self::Symbol(r0, r1)) => *l0 == *r0 && l1.eq(r1),
            (Self::String(l0), Self::String(r0)) => l0.eq(r0),
            (Self::Number(l0), Self::Number(r0)) => *l0 == *r0,
            (Self::BigInt(l0), Self::BigInt(r0)) => *l0 == *r0,
            (Self::Boolean(l0), Self::Boolean(r0)) => *l0 == *r0,
            (Self::Undefined, Self::Undefined) => true,
            (Self::Null, Self::Null) => true,
            _ => false,
        }
    }
}

impl Hash for JsValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Prototype(proto) => proto.borrow().hash(state),
            Self::Symbol(id, inner) => {
                id.hash(state);
                inner.hash(state);
            }
            Self::String(value) => value.hash(state),
            Self::Number(value) => value.to_bits().hash(state),
            Self::BigInt(value) => value.hash(state),
            Self::Boolean(value) => value.hash(state),
            Self::Undefined | Self::Null | Self::Function(_) | Self::Generator(_) => {}
        }
    }
}

impl From<&str> for JsValue {
    fn from(value: &str) -> Self {
        JsValue::String(value.to_owned())
    }
}

impl From<String> for JsValue {
    fn from(value: String) -> Self {
        JsValue::String(value)
    }
}

impl From<usize> for JsValue {
    fn from(value: usize) -> Self {
        JsValue::BigInt(value as i64)
    }
}

impl From<i64> for JsValue {
    fn from(value: i64) -> Self {
        JsValue::BigInt(value)
    }
}

impl From<f64> for JsValue {
    fn from(value: f64) -> Self {
        JsValue::Number(value)
    }
}

impl JsValue {
    pub fn opt_find(
        &self,
        key: &JsValue,
        message: &str,
    ) -> Option<(Rc<RefCell<Prototype>>, Rc<RefCell<JsValue>>)> {
        Prototype::opt_find(self.unwrap_proto(message), key)
    }

    pub fn find(
        &self,
        key: &JsValue,
        message: &str,
    ) -> (Option<Rc<RefCell<Prototype>>>, Rc<RefCell<JsValue>>) {
        Prototype::find(self.unwrap_proto(message), key)
    }

    pub fn unwrap_proto(&self, message: &str) -> Rc<RefCell<Prototype>> {
        let JsValue::Prototype(p) = self else {
            panic!("panick in unwrap_proto: {message} with value of {self:?}")
        };
        p.clone()
    }

    pub const fn is_fasly(&self) -> bool {
        match self {
            JsValue::Boolean(false) => true,
            JsValue::String(s) => s.is_empty(),
            JsValue::BigInt(n) if *n == 0 => true,
            JsValue::Number(n) if n.is_nan() || *n == 0. => true,
            JsValue::Null | JsValue::Undefined => true,
            _ => false,
        }
    }

    pub const fn is_truthy(&self) -> bool {
        !self.is_fasly()
    }

    pub fn print(&self) -> String {
        match self {
            JsValue::Function(_) => "[Function (anonymous)]".to_owned(),
            JsValue::Generator(_) => "[GeneratorFunction (anonymous)]".to_owned(),
            JsValue::Prototype(ref_cell) => {
                let mut entries = vec![];
                for (key, value) in ref_cell.borrow().properties.iter() {
                    if key == &PROTO_NAME.into() {
                        continue;
                    }
                    let k = match key {
                        JsValue::String(s) => s.clone(),
                        JsValue::Prototype(key) if let Some(name) = key.borrow().name => {
                            format!("[{name}]")
                        }
                        _ => key.print(),
                    };
                    let v = match inline_borrow!(value) {
                        JsValue::String(s) => format!("'{}'", s),
                        JsValue::Prototype(ref v) if let Some(name) = v.borrow().name => {
                            format!("[{name}]")
                        }
                        _ => value.borrow().print(),
                    };
                    entries.push((key.clone(), k, v));
                }
                const fn key_order(k: &JsValue) -> u8 {
                    match k {
                        JsValue::String(_) => 0,
                        JsValue::Number(_) => 1,
                        JsValue::Prototype(_) => 2,
                        _ => 3,
                    }
                }
                entries.sort_by(|a, b| {
                    let ao = key_order(&a.0);
                    let bo = key_order(&b.0);
                    if ao != bo { ao.cmp(&bo) } else { a.1.cmp(&b.1) }
                });
                format!(
                    "{{ {} }}",
                    entries
                        .into_iter()
                        .map(|(_, k, v)| format!("{}: {}", k, v))
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            JsValue::Symbol(_, js_value) => format!("Symbol({})", js_value.print()),
            JsValue::String(s) => s.clone(),
            JsValue::Number(n) => format!("{n}"),
            JsValue::BigInt(n) => n.to_string(),
            JsValue::Boolean(b) => if *b { "true" } else { "false" }.to_owned(),
            JsValue::Undefined => "undefined".to_owned(),
            JsValue::Null => "null".to_owned(),
        }
    }
}

impl Debug for JsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function(arg0) => f.debug_tuple("Function").field(arg0).finish(),
            Self::Generator(arg0) => f.debug_tuple("Generator").field(arg0).finish(),
            Self::Prototype(arg0) => f.debug_tuple("Prototype").field(arg0.as_ref()).finish(),
            Self::Symbol(arg0, arg1) => f.debug_tuple("Symbol").field(arg0).field(arg1).finish(),
            Self::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
            Self::Number(arg0) => f.debug_tuple("Number").field(arg0).finish(),
            Self::BigInt(arg0) => f.debug_tuple("BigInt").field(arg0).finish(),
            Self::Boolean(arg0) => f.debug_tuple("Boolean").field(arg0).finish(),
            Self::Undefined => write!(f, "Undefined"),
            Self::Null => write!(f, "Null"),
        }
    }
}
