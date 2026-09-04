#![feature(maybe_uninit_array_assume_init)]

use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt::Debug,
    num::NonZero,
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

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub enum LogLevel {
    Trace = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
    Fatal = 4,
}

pub trait Logger {
    fn logln(&mut self, level: LogLevel, message: &dyn Fn() -> String);
    fn logln_str(&mut self, level: LogLevel, message: &str) {
        self.logln(level, &|| message.to_owned());
    }
}

pub trait DataLogger: Logger {
    type LogData;
    fn prepare(&self, level: LogLevel, message: &dyn Fn() -> String) -> Self::LogData;
    fn logln_data(&mut self, data: Self::LogData);
}

impl<T: DataLogger> Logger for T {
    fn logln(&mut self, level: LogLevel, message: &dyn Fn() -> String) {
        self.logln_data(self.prepare(level, message));
    }
}

pub struct StdOutLogger;
impl DataLogger for StdOutLogger {
    type LogData = String;
    fn prepare(&self, level: LogLevel, message: &dyn Fn() -> String) -> Self::LogData {
        format!("{level:?}: {}", message())
    }
    fn logln_data(&mut self, data: Self::LogData) {
        println!("{data}");
    }
}

pub struct FilterLogs<T: DataLogger> {
    logger: T,
    min_level: LogLevel,
}
impl<T: DataLogger> FilterLogs<T> {
    pub const fn new(logger: T, min_level: LogLevel) -> Self {
        Self { logger, min_level }
    }
}
impl<T: DataLogger> DataLogger for FilterLogs<T> {
    type LogData = Option<T::LogData>;
    fn prepare(&self, level: LogLevel, message: &dyn Fn() -> String) -> Self::LogData {
        if level >= self.min_level {
            Some(self.logger.prepare(level, message))
        } else {
            None
        }
    }
    fn logln_data(&mut self, data: Self::LogData) {
        if let Some(data) = data {
            self.logger.logln_data(data);
        }
    }
}

pub struct LastNLogs<T: DataLogger> {
    logger: T,
    logs: VecDeque<T::LogData>,
    max_amount: NonZero<usize>,
}
impl<T: DataLogger> LastNLogs<T> {
    pub const fn new_empty(logger: T, max_amount: NonZero<usize>) -> Self {
        Self {
            logger,
            logs: VecDeque::new(),
            max_amount,
        }
    }
    pub const fn new_custom(
        logger: T,
        max_amount: NonZero<usize>,
        logs: VecDeque<T::LogData>,
    ) -> Self {
        Self {
            logger,
            logs,
            max_amount,
        }
    }
}
impl<T: DataLogger> DataLogger for LastNLogs<T> {
    type LogData = T::LogData;
    fn prepare(&self, level: LogLevel, message: &dyn Fn() -> String) -> Self::LogData {
        self.logger.prepare(level, message)
    }
    fn logln_data(&mut self, data: Self::LogData) {
        if self.logs.len() == self.max_amount.get() {
            self.logs.pop_front();
        }
        self.logs.push_back(data);
    }
}
impl<T: DataLogger> LastNLogs<T> {
    pub fn flush(&mut self) {
        for data in self.logs.drain(..) {
            self.logger.logln_data(data);
        }
    }
}
impl<T: DataLogger> Drop for LastNLogs<T> {
    fn drop(&mut self) {
        self.flush();
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
    regex::prebuild_regex,
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

#[derive(Clone)]
pub struct Environment {
    pub mem: Rc<RefCell<Prototype>>,
    pub logger: Rc<RefCell<dyn Logger>>,
}
impl Environment {
    pub fn with_mem(&self, mem: Rc<RefCell<Prototype>>) -> Self {
        Self {
            mem,
            logger: self.logger.clone(),
        }
    }
}

const PROTO_NAME: &str = "__proto__";
const PROTOTYPE_NAME: &str = "prototype";
const CONSTRUCTOR_NAME: &str = "constructor";

pub fn prebuild_prototypes(
    console_config: impl Fn(Environment) -> Rc<RefCell<Prototype>>,
    logger: Rc<RefCell<dyn Logger>>,
) -> Rc<RefCell<Prototype>> {
    let object = Rc::new(RefCell::new(Prototype {
        name: Some("Object"),
        properties: HashMap::from([(PROTO_NAME.into(), Rc::new(RefCell::new(JsValue::Null)))]),
        formating: false,
    }));

    let function = Prototype::new_child(object.clone(), Some("Function"), []);
    function.borrow_mut().properties.insert(
        PROTOTYPE_NAME.into(),
        Rc::new(RefCell::new(JsValue::Prototype(function.clone()))),
    );

    let env = Environment {
        mem: Rc::new(RefCell::new(Prototype {
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
        })),
        logger,
    };

    let obj = object.clone();
    object.borrow_mut().properties.insert(
        CONSTRUCTOR_NAME.into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            "Object.constructor",
            prebuild_runnable(
                env.clone(),
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

    let descriptor_object = object.clone();
    object.borrow_mut().properties.insert(
        "getOwnPropertyDescriptor".into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            "Object.getOwnPropertyDescriptor",
            prebuild_runnable(
                env.clone(),
                Box::new(move |_env, _, [value, key]| {
                    let JsValue::Prototype(target) = inline_borrow!(value) else {
                        return CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)));
                    };
                    let key = inline_borrow!(key);
                    let Some(property) = target.borrow().properties.get(&key).cloned() else {
                        return CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)));
                    };
                    let writable = !matches!(key, JsValue::String(ref key) if key == "name" || key == "length");
                    let descriptor = Prototype::new_child(
                        descriptor_object.clone(),
                        None,
                        [
                            ("value".into(), property),
                            ("writable".into(), Rc::new(RefCell::new(JsValue::Boolean(writable)))),
                            ("enumerable".into(), Rc::new(RefCell::new(JsValue::Boolean(false)))),
                            ("configurable".into(), Rc::new(RefCell::new(JsValue::Boolean(true)))),
                        ],
                    );
                    descriptor.borrow_mut().properties.remove(&PROTO_NAME.into());
                    CodeResult::Return(Rc::new(RefCell::new(JsValue::Prototype(descriptor))))
                }),
            ),
        ),
    );

    object.borrow_mut().properties.insert(
        "getOwnPropertyNames".into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            "Object.getOwnPropertyNames",
            prebuild_runnable(
                env.clone(),
                Box::new(|env, _, [value]| {
                    let JsValue::Prototype(target) = inline_borrow!(value) else {
                        return CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)));
                    };
                    let names = target
                        .borrow()
                        .properties
                        .keys()
                        .filter_map(|key| match key {
                            JsValue::String(key) => {
                                Some(Rc::new(RefCell::new(JsValue::String(key.clone()))))
                            }
                            _ => None,
                        })
                        .collect();
                    let array = Prototype::find(env.mem.clone(), &stringify!(Array).into())
                        .1
                        .borrow()
                        .unwrap_proto("Object.getOwnPropertyNames for Array");
                    CodeResult::Return(new_array(array, names, env.logger))
                }),
            ),
        ),
    );

    let own_property_object = object.clone();
    object.borrow_mut().properties.insert(
        "hasOwnProperty".into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            "Object.prototype.hasOwnProperty",
            prebuild_runnable(
                env.clone(),
                Box::new(move |_env, this, [key]| {
                    let JsValue::Prototype(target) = inline_borrow!(this) else {
                        return CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(false))));
                    };
                    let key = inline_borrow!(key);
                    CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(
                        target.borrow().properties.contains_key(&key),
                    ))))
                }),
            ),
        ),
    );

    object.borrow_mut().properties.insert(
        "propertyIsEnumerable".into(),
        new_runnable_with_object(
            function.clone(),
            own_property_object,
            "Object.prototype.propertyIsEnumerable",
            prebuild_runnable(
                env.clone(),
                Box::new(|_env, _this, [_key]| {
                    CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(false))))
                }),
            ),
        ),
    );
    object.borrow_mut().properties.insert(
        "toString".into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            "Object.prototype.toString",
            prebuild_runnable(
                env.clone(),
                Box::new(|_env, this, []| {
                    let tag = match inline_borrow!(this) {
                        JsValue::Null => "Null",
                        JsValue::Undefined => "Undefined",
                        JsValue::Prototype(object) => object
                            .borrow()
                            .name
                            .map(|name| match name {
                                "Array" => "Array",
                                "Function" => "Function",
                                "RegExp" => "RegExp",
                                "Error" | "TypeError" | "SyntaxError" => "Error",
                                _ => "Object",
                            })
                            .unwrap_or("Object"),
                        JsValue::Function(_) => "Function",
                        _ => "Object",
                    };
                    CodeResult::Return(Rc::new(RefCell::new(JsValue::String(format!(
                        "[object {tag}]"
                    )))))
                }),
            ),
        ),
    );
    object.borrow_mut().properties.insert(
        PROTOTYPE_NAME.into(),
        Rc::new(RefCell::new(JsValue::Prototype(object.clone()))),
    );

    let reflect = Prototype::new_child(object.clone(), Some("Reflect"), []);
    reflect.borrow_mut().properties.insert(
        "construct".into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            "Reflect.construct",
            prebuild_runnable(
                env.clone(),
                Box::new(|mem, _, [_target, _arguments, new_target]| {
                    if !is_constructor(&new_target) {
                        return type_error(mem);
                    }
                    CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)))
                }),
            ),
        ),
    );
    env.mem.borrow_mut().properties.insert(
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
                env.clone(),
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
                env.clone(),
                Box::new(|env, this, arguments| {
                    if let JsValue::Prototype(ref func_proto) = inline_borrow!(this) {
                        let this_arg = arguments
                            .first()
                            .cloned()
                            .unwrap_or_else(|| Rc::new(RefCell::new(JsValue::Undefined)));
                        let params: Vec<Rc<RefCell<JsValue>>> =
                            arguments.iter().skip(1).cloned().collect();
                        crate::run_function_object(
                            func_proto.clone(),
                            this_arg,
                            params,
                            env.logger.clone(),
                        )
                    } else {
                        CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)))
                    }
                }),
            ),
        ),
    );

    prebuild_symbol(env.clone());
    let symbol = Prototype::find(env.mem.clone(), &stringify!(Symbol).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_prototypes for Symbol");
    let has_instance = inline_borrow!(Prototype::find(symbol, &"hasInstance".into()).1);
    let has_instance_function = new_runnable(
        function.clone(),
        "Function.[hasInstance]",
        prebuild_runnable(
            env.clone(),
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
                        return CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(true))));
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
    let function_for_bind = function.clone();
    let object_for_bind = object.clone();
    function.borrow_mut().properties.insert(
        "bind".into(),
        new_runnable_with_object(
            function_for_bind.clone(),
            object_for_bind.clone(),
            "Function.bind",
            prebuild_runnable_direct(
                env.clone(),
                Box::new(move |env, this, arguments| {
                    let JsValue::Prototype(target) = inline_borrow!(this) else {
                        return CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)));
                    };
                    let bound_this = arguments
                        .first()
                        .cloned()
                        .unwrap_or_else(|| Rc::new(RefCell::new(JsValue::Undefined)));
                    let target = target.clone();
                    let bound_arguments = arguments.into_iter().skip(1).collect::<Vec<_>>();
                    CodeResult::Return(new_runnable_with_object(
                        function_for_bind.clone(),
                        object_for_bind.clone(),
                        "Function.bound",
                        prebuild_runnable_direct(
                            env.clone(),
                            Box::new(move |env, _, arguments| {
                                let mut call_arguments = bound_arguments.clone();
                                call_arguments.extend(arguments);
                                run_function_object(
                                    target.clone(),
                                    bound_this.clone(),
                                    call_arguments,
                                    env.logger.clone(),
                                )
                            }),
                        ),
                    ))
                }),
            ),
        ),
    );
    prebuild_iterator(env.clone());
    prebuild_array(env.clone());
    let array = Prototype::find(env.mem.clone(), &stringify!(Array).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_prototypes for Array");
    array.borrow_mut().properties.insert(
        PROTOTYPE_NAME.into(),
        Rc::new(RefCell::new(JsValue::Prototype(array.clone()))),
    );
    prebuild_date(env.clone());
    prebuild_string(env.clone());
    prebuild_number(env.clone());
    prebuild_math(env.clone());
    prebuild_console(env.clone());
    prebuild_error(env.clone());
    prebuild_type_error(env.clone());
    prebuild_syntax_error(env.clone());
    env.mem.borrow().properties[&JsValue::String("console".to_owned())]
        .borrow()
        .unwrap_proto("prebuild_prototypes adding config to console")
        .borrow_mut()
        .properties
        .insert(
            JsValue::String("__config__".to_owned()),
            Rc::new(RefCell::new(JsValue::Prototype(console_config(
                env.clone(),
            )))),
        );
    prebuild_itergen(env.clone());
    env.mem.borrow_mut().properties.insert(
        "NaN".into(),
        Rc::new(RefCell::new(JsValue::Number(f64::NAN))),
    );
    env.mem.borrow_mut().properties.insert(
        "Infinity".into(),
        Rc::new(RefCell::new(JsValue::Number(f64::INFINITY))),
    );
    prebuild_regex(env.clone());
    env.mem
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

fn type_error(env: Environment) -> CodeResult {
    let error = Prototype::find(env.mem, &stringify!(TypeError).into())
        .1
        .borrow()
        .unwrap_proto("Reflect.construct for TypeError");
    let constructor = Rc::new(RefCell::new(JsValue::Prototype(error.clone())));
    CodeResult::Error(Rc::new(RefCell::new(JsValue::Prototype(
        Prototype::new_child(error, None, [("constructor".into(), constructor)]),
    ))))
}

const RUNNABLE: &str = "__!@#$%^&*()__";
const ARGUMENTS: &str = "arguments";
