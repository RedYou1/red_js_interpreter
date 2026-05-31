pub use std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

pub use crate::{
    Generator, JsValue, Prototype, Runnable, new_runnable,
    prebuild::{
        array::new_array,
        console::{CONSOLE_LOGS, default_console_config},
    },
    prebuild_prototypes, run_function_object,
};

mod base;
mod compiler;
mod parser;

pub fn prebuild_prototypes_test(logs: &mut Vec<String>) -> Rc<RefCell<Prototype>> {
    let protos = prebuild_prototypes(default_console_config);
    let console = Prototype::find(protos.clone(), &JsValue::String("console".to_owned()))
        .1
        .unwrap_proto();
    console.borrow_mut().properties.insert(
        JsValue::String(CONSOLE_LOGS.to_owned()),
        JsValue::BigInt(logs as *mut Vec<String> as i64),
    );
    protos
}

impl Debug for JsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function(arg0) => f.debug_tuple("Function").field(arg0).finish(),
            Self::Generator(arg0) => f.debug_tuple("Generator").field(arg0).finish(),
            Self::Prototype(arg0) => f.debug_tuple("Prototype").field(arg0).finish(),
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

impl Debug for Prototype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prototype")
            .field(
                "properties",
                &self
                    .properties
                    .iter()
                    .map(|(k, v)| {
                        let mut k = k.clone();
                        let mut v = v.clone();

                        if let JsValue::Prototype(in_k) = &k {
                            let name = in_k.borrow().name;
                            if let Some(name) = name {
                                k = JsValue::String(format!("[{}]", name));
                            }
                        }

                        if let JsValue::Prototype(in_v) = &v {
                            let name = in_v.borrow().name;
                            if let Some(name) = name {
                                v = JsValue::String(format!("[{}]", name));
                            }
                        }
                        (k, v)
                    })
                    .collect::<HashMap<JsValue, JsValue>>(),
            )
            .finish()
    }
}

impl Debug for Runnable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runnable")
            .field("params", &self.params)
            .field("excess", &self.excess)
            .finish()
    }
}

impl Debug for Generator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Generator")
            .field("params", &self.params)
            .field("excess", &self.excess)
            .finish()
    }
}
