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
    prebuild_iterator(env.clone());
    prebuild_array(env.clone());
    prebuild_date(env.clone());
    prebuild_string(env.clone());
    prebuild_number(env.clone());
    prebuild_math(env.clone());
    prebuild_console(env.clone());
    prebuild_error(env.clone());
    prebuild_type_error(env.clone());
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
