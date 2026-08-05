#![feature(maybe_uninit_array_assume_init)]

use std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

mod code_manipulation;
mod generator;
mod jsvalue;
mod parser;
mod prebuild;
mod prototype;
mod runnable;
pub use code_manipulation::*;
pub use generator::*;
pub use jsvalue::JsValue;
pub use parser::parse;
pub use prebuild::array::new_array;
pub use prebuild::console::default_console_config;
pub use prototype::Prototype;
pub use runnable::*;

pub use parser::ast as parser_ast;

#[derive(Debug, PartialEq, PartialOrd)]
pub enum LogLevel {
    Trace = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
    Fatal = 4,
}
#[cfg(test)]
pub const LOGLEVEL: LogLevel = LogLevel::Trace;
#[cfg(not(test))]
pub const LOGLEVEL: LogLevel = LogLevel::Trace;

pub fn logln(level: LogLevel, message: &str) {
    if level >= LOGLEVEL {
        println!("{level:?}: {message}");
    }
}

#[cfg(test)]
mod tests;

use crate::prebuild::{
    array::prebuild_array,
    console::prebuild_console,
    error::{prebuild_error, prebuild_type_error},
    iterator::{prebuild_iterator, prebuild_itergen},
    math::prebuild_math,
    number::prebuild_number,
    prebuild_runnable, prebuild_runnable_direct,
    string::prebuild_string,
    symbol::prebuild_symbol,
};

#[macro_export]
macro_rules! inline_borrow {
    ($obj:expr) => {{
        let t = $obj;
        Clone::clone(&*t.borrow())
    }};
}

/*
https://miro.medium.com/v2/resize:fit:2000/1*rGkuqfPZUEQw9hvJLV0Gig.png
https://i.sstatic.net/uy5ce.png

https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Function
*/

const PROTO_NAME: &str = "__proto__";
const PROTOTYPE_NAME: &str = "prototype";
const CONSTRUCTOR_NAME: &str = "constructor";

pub fn prebuild_prototypes(
    console_config: impl Fn(Rc<RefCell<Prototype>>) -> Rc<RefCell<Prototype>>,
) -> Rc<RefCell<Prototype>> {
    let object = Rc::new(RefCell::new(Prototype {
        name: Some("Object"),
        properties: HashMap::from([(PROTO_NAME.into(), Rc::new(RefCell::new(JsValue::Null)))]),
        formating: false,
    }));

    let function = Prototype::new_child(object.clone(), Some("Function"), []);

    let prototypes = Rc::new(RefCell::new(Prototype {
        name: None,
        properties: HashMap::from([
            (
                stringify!(Object).into(),
                Rc::new(RefCell::new(JsValue::Prototype(object.clone()))),
            ),
            (
                stringify!(Function).into(),
                Rc::new(RefCell::new(JsValue::Prototype(function.clone()))),
            ),
        ]),
        formating: false,
    }));

    let obj = object.clone();
    object.borrow_mut().properties.insert(
        CONSTRUCTOR_NAME.into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            "Object.constructor",
            prebuild_runnable(
                prototypes.clone(),
                Box::new(move |_mem, _this, [value]| {
                    if let JsValue::Undefined | JsValue::Null = inline_borrow!(value.clone()) {
                        CodeResult::Return(Rc::new(RefCell::new(JsValue::Prototype(
                            Prototype::new_child(
                                obj.clone(),
                                None,
                                [(
                                    PROTOTYPE_NAME.into(),
                                    Rc::new(RefCell::new(JsValue::Prototype(obj.clone()))),
                                )],
                            ),
                        ))))
                    } else if let JsValue::Prototype(_) = inline_borrow!(value.clone()) {
                        CodeResult::Return(value.clone())
                    } else {
                        todo!() // return as an object of primitive wrapper
                    }
                }),
            ),
        ),
    );

    object.borrow_mut().properties.insert(
        "create".into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            "Object.create",
            prebuild_runnable_direct(
                prototypes.clone(),
                Box::new(|_mem, _this, arguments| {
                    CodeResult::Return(Rc::new(RefCell::new(
                        if let Some(proto) = arguments.first() {
                            if let JsValue::Prototype(ref proto_obj) = inline_borrow!(proto) {
                                JsValue::Prototype(Prototype::new_child(
                                    proto_obj.clone(),
                                    None,
                                    [],
                                ))
                            } else {
                                JsValue::Undefined
                            }
                        } else {
                            JsValue::Undefined
                        },
                    )))
                }),
            ),
        ),
    );

    function.borrow_mut().properties.insert(
        "call".into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            "Function.call",
            prebuild_runnable_direct(
                prototypes.clone(),
                Box::new(|_, this, arguments| {
                    if let JsValue::Prototype(ref func_proto) = inline_borrow!(this) {
                        let this_arg = arguments
                            .first()
                            .cloned()
                            .unwrap_or_else(|| Rc::new(RefCell::new(JsValue::Undefined)));
                        let params: Vec<Rc<RefCell<JsValue>>> =
                            arguments.iter().skip(1).cloned().collect();
                        crate::run_function_object(func_proto.clone(), this_arg, params)
                    } else {
                        CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)))
                    }
                }),
            ),
        ),
    );

    prebuild_symbol(prototypes.clone());
    prebuild_iterator(prototypes.clone());
    prebuild_array(prototypes.clone());
    prebuild_string(prototypes.clone());
    prebuild_number(prototypes.clone());
    prebuild_math(prototypes.clone());
    prebuild_console(prototypes.clone());
    prebuild_error(prototypes.clone());
    prebuild_type_error(prototypes.clone());
    prototypes.borrow().properties[&JsValue::String("console".to_owned())]
        .borrow()
        .unwrap_proto("prebuild_prototypes adding config to console")
        .borrow_mut()
        .properties
        .insert(
            JsValue::String("__config__".to_owned()),
            Rc::new(RefCell::new(JsValue::Prototype(console_config(
                prototypes.clone(),
            )))),
        );
    prebuild_itergen(prototypes.clone());
    prototypes.borrow_mut().properties.insert(
        "NaN".into(),
        Rc::new(RefCell::new(JsValue::Number(f64::NAN))),
    );
    prototypes.borrow_mut().properties.insert(
        "Infinity".into(),
        Rc::new(RefCell::new(JsValue::Number(f64::INFINITY))),
    );
    prototypes
}

const RUNNABLE: &str = "__!@#$%^&*()__";
const ARGUMENTS: &str = "arguments";
