#![feature(maybe_uninit_array_assume_init)]

use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    panic::{self, AssertUnwindSafe},
    rc::Rc,
};

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
pub const LOGLEVEL: LogLevel = LogLevel::Info;

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
    date::prebuild_date,
    error::{prebuild_error, prebuild_syntax_error, prebuild_type_error},
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

    let reflect = Prototype::new_child(object.clone(), Some("Reflect"), []);
    reflect.borrow_mut().properties.insert(
        "construct".into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            "Reflect.construct",
            prebuild_runnable(
                prototypes.clone(),
                Box::new(|mem, _, [_target, _arguments, new_target]| {
                    if !is_constructor(&new_target) {
                        return type_error(mem);
                    }
                    CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)))
                }),
            ),
        ),
    );
    prototypes.borrow_mut().properties.insert(
        "Reflect".into(),
        Rc::new(RefCell::new(JsValue::Prototype(reflect))),
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
    let symbol = Prototype::find(prototypes.clone(), &stringify!(Symbol).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_prototypes for Symbol");
    let has_instance = inline_borrow!(Prototype::find(symbol, &"hasInstance".into()).1);
    let has_instance_function = new_runnable(
        function.clone(),
        "Function.[hasInstance]",
        prebuild_runnable(
            prototypes.clone(),
            Box::new(|_, this, [instance]| {
                let JsValue::Prototype(function) = inline_borrow!(this) else {
                    return CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(false))));
                };
                let Some((_, prototype)) = Prototype::opt_find(function, &PROTOTYPE_NAME.into())
                else {
                    return CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(false))));
                };
                let JsValue::Prototype(prototype) = inline_borrow!(prototype) else {
                    return CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(false))));
                };
                let JsValue::Prototype(mut instance) = inline_borrow!(instance) else {
                    return CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(false))));
                };
                loop {
                    let Some(parent) = instance.borrow().parent() else {
                        break;
                    };
                    if Rc::ptr_eq(&parent, &prototype) {
                        return CodeResult::Return(Rc::new(RefCell::new(
                            JsValue::Boolean(true),
                        )));
                    }
                    instance = parent;
                }
                CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(false))))
            }),
        ),
    );
    function
        .borrow_mut()
        .properties
        .insert(has_instance, has_instance_function);
    let dynamic_function_mem = prototypes.clone();
    let dynamic_function = new_runnable_with_object(
        function.clone(),
        object.clone(),
        "Function",
        prebuild_runnable_direct(
            prototypes.clone(),
            Box::new(move |proto, _, arguments| {
                let (parameters, body) = if let Some((body, parameters)) = arguments.split_last() {
                    let parameters = parameters
                        .iter()
                        .map(|value| match &*value.borrow() {
                            JsValue::String(value) => Ok(value.clone()),
                            _ => Err(()),
                        })
                        .collect::<Result<Vec<_>, _>>();
                    let body = match &*body.borrow() {
                        JsValue::String(value) => Ok(value.clone()),
                        _ => Err(()),
                    };
                    match (parameters, body) {
                        (Ok(parameters), Ok(body)) => (parameters.join(","), body),
                        _ => return syntax_error(dynamic_function_mem.clone()),
                    }
                } else {
                    (String::new(), String::new())
                };
                if has_invalid_html_close_comment(&parameters) {
                    return syntax_error(dynamic_function_mem.clone());
                }
                let source = format!("function anonymous({parameters}\n) {{\n{body}\n}}");
                let parsed = panic::catch_unwind(AssertUnwindSafe(|| {
                    let program = parse(&source).compile(dynamic_function_mem.clone());
                    run_sub(&program.code, proto, &mut CodeIndex::new())
                }));
                match parsed {
                    Ok(CodeResult::Normal(result) | CodeResult::NormalMember(result, _, _)) => {
                        CodeResult::Return(result)
                    }
                    Ok(result) => result,
                    Err(_) => syntax_error(dynamic_function_mem.clone()),
                }
            }),
        ),
    );
    function
        .borrow_mut()
        .properties
        .insert(CONSTRUCTOR_NAME.into(), dynamic_function);
    prebuild_iterator(prototypes.clone());
    prebuild_array(prototypes.clone());
    prebuild_date(prototypes.clone());
    prebuild_string(prototypes.clone());
    prebuild_number(prototypes.clone());
    prebuild_math(prototypes.clone());
    prebuild_console(prototypes.clone());
    prebuild_error(prototypes.clone());
    prebuild_type_error(prototypes.clone());
    prebuild_syntax_error(prototypes.clone());
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

fn is_constructor(value: &Rc<RefCell<JsValue>>) -> bool {
    let JsValue::Prototype(object) = inline_borrow!(value.clone()) else {
        return false;
    };
    if Prototype::opt_find(object.clone(), &RUNNABLE.into()).is_none() {
        return false;
    }
    !matches!(
        Prototype::opt_find(object, &"__constructable__".into())
            .map(|(_, value)| inline_borrow!(value)),
        Some(JsValue::Boolean(false))
    )
}

fn type_error(mem: Rc<RefCell<Prototype>>) -> CodeResult {
    let error = Prototype::find(mem, &stringify!(TypeError).into())
        .1
        .borrow()
        .unwrap_proto("Reflect.construct for TypeError");
    let constructor = Rc::new(RefCell::new(JsValue::Prototype(error.clone())));
    CodeResult::Error(Rc::new(RefCell::new(JsValue::Prototype(
        Prototype::new_child(error, None, [("constructor".into(), constructor)]),
    ))))
}

fn syntax_error(mem: Rc<RefCell<Prototype>>) -> CodeResult {
    let error = Prototype::find(mem, &stringify!(SyntaxError).into())
        .1
        .borrow()
        .unwrap_proto("Function constructor for SyntaxError");
    let constructor = Rc::new(RefCell::new(JsValue::Prototype(error.clone())));
    CodeResult::Error(Rc::new(RefCell::new(JsValue::Prototype(
        Prototype::new_child(error, None, [("constructor".into(), constructor)]),
    ))))
}

fn has_invalid_html_close_comment(parameters: &str) -> bool {
    parameters.match_indices("-->").any(|(index, _)| {
        !matches!(
            parameters[..index].chars().next_back(),
            Some('\n' | '\r')
        )
    })
}

const RUNNABLE: &str = "__!@#$%^&*()__";
const ARGUMENTS: &str = "arguments";
