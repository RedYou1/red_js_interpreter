use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, LogLevel, Prototype, handle_return, inline_borrow,
    parser::expr::Expr, run_sub,
};

fn string_member(value: &str, key: &crate::JsValue) -> crate::JsValue {
    if matches!(key, crate::JsValue::String(name) if name == "length") {
        return crate::JsValue::BigInt(value.encode_utf16().count() as i64);
    }

    let index = match key {
        crate::JsValue::BigInt(index) if *index >= 0 => usize::try_from(*index).ok(),
        crate::JsValue::Number(index)
            if index.is_finite() && *index >= 0.0 && index.fract() == 0.0 =>
        {
            (*index < usize::MAX as f64).then_some(*index as usize)
        }
        crate::JsValue::String(index) => index
            .parse::<usize>()
            .ok()
            .filter(|parsed| parsed.to_string() == *index),
        _ => None,
    };

    index
        .and_then(|index| value.encode_utf16().nth(index))
        .map(|unit| crate::JsValue::String(String::from_utf16_lossy(&[unit])))
        .unwrap_or(crate::JsValue::Undefined)
}

#[derive(Debug)]
pub struct Member {
    pub object: Box<dyn Expr>,
    pub property: Box<dyn Expr>,
}

impl Expr for Member {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let obj = self.object.compile(env.clone());
        let prop = self.property.compile(env);
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Member");
            let object = handle_return!(run_sub(&obj, env.clone(), &mut CodeIndex::new()));
            let key = handle_return!(run_sub(&prop, env.clone(), &mut CodeIndex::new()));
            match inline_borrow!(object.clone()) {
                crate::JsValue::Prototype(object) => {
                    let out = Prototype::find(object.clone(), &inline_borrow!(key.clone())).1;
                    env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                        format!("Exiting Expr::Member {object:?}[{key:?}] == {out:?}")
                    });
                    CodeResult::NormalMember(out, object, key)
                }
                crate::JsValue::String(value) => CodeResult::Normal(Rc::new(RefCell::new(
                    string_member(&value, &inline_borrow!(key)),
                ))),
                value => {
                    value.unwrap_proto("expr::Member for obj");
                    unreachable!()
                }
            }
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            object: self.object.duplicate(),
            property: self.property.duplicate(),
        })
    }
}
