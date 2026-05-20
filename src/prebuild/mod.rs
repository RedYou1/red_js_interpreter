use std::{cell::RefCell, rc::Rc};

use crate::{ARGUMENTS, JsValue, Prototype, Runnable};

pub mod array;
pub mod console;
pub mod iterator;
pub mod math;
pub mod number;
mod prelude;
pub mod string;
pub mod symbol;

pub fn prebuild_runnable<const WANTED_PARAM: usize>(
    mem: Rc<RefCell<Prototype>>,
    fun: Box<dyn Fn(Rc<RefCell<Prototype>>, JsValue, [JsValue; WANTED_PARAM]) -> JsValue>,
) -> Runnable {
    Runnable {
        params: Vec::new(),
        excess: None,
        code: vec![Box::new(move |proto, _| {
            let JsValue::Prototype(array) = Prototype::find(proto.clone(), &ARGUMENTS.into()).1
            else {
                panic!("{ARGUMENTS} not found prebuild function")
            };

            let mut params = [const { JsValue::Undefined }; WANTED_PARAM];

            for (i, param) in params.iter_mut().enumerate() {
                *param = Prototype::find(array.clone(), &i.into()).1;
            }

            (
                JsValue::Null,
                Some(fun(
                    mem.clone(),
                    Prototype::find(proto, &"this".into()).1,
                    params,
                )),
            )
        })],
    }
}

pub fn prebuild_runnable_direct(
    mem: Rc<RefCell<Prototype>>,
    fun: Box<dyn Fn(Rc<RefCell<Prototype>>, JsValue, Vec<JsValue>) -> JsValue>,
) -> Runnable {
    Runnable {
        params: Vec::new(),
        excess: None,
        code: vec![Box::new(move |proto, _| {
            let JsValue::Prototype(array) = Prototype::find(proto.clone(), &ARGUMENTS.into()).1
            else {
                panic!("{ARGUMENTS} not found prebuild function")
            };
            let JsValue::BigInt(length) = Prototype::find(array.clone(), &"length".into()).1 else {
                panic!("length in Array")
            };

            let mut params = Vec::with_capacity(length as usize);

            for i in 0..length as usize {
                params.push(Prototype::find(array.clone(), &i.into()).1);
            }

            (
                JsValue::Null,
                Some(fun(
                    mem.clone(),
                    Prototype::find(proto, &"this".into()).1,
                    params,
                )),
            )
        })],
    }
}

#[macro_export]
macro_rules! new_class {
    ($func_name:ident, $name:ident, $parent:ident, $($var_static_name:ident, $var_static:expr),*; $($fn_name:ident, $fn_type:tt, $fn_block:expr),*; $($fn_name2:expr, $fn_type2:tt, $fn_block2:expr),*) => {
        pub fn $func_name(mem: Rc<RefCell<Prototype>>) {
            let function = Prototype::find(mem.clone(), &"Function".into()).1.unwrap_proto();
            let class = JsValue::Prototype(Rc::new(RefCell::new(Prototype {
                name: Some(stringify!($name)),
                properties: HashMap::from([
                    (
                        PROTO_NAME.into(),
                        Prototype::find(mem.clone(), &stringify!($parent).into()).1,
                    ),
                    $(
                        (stringify!($var_static_name).into(), $var_static),
                    )*
                    $(
                        class_fn!(mem, function, $name, stringify!($fn_name).into(), $fn_type, $fn_block),
                    )*
                    $(
                        class_fn!(mem, function, $name, {
                            let mut temp = JsValue::Prototype(mem.clone());
                            stringify!($fn_name2).split('.').for_each(|x| temp = Prototype::find(temp.unwrap_proto(), &x.into()).1);
                            temp
                        }, $fn_type2, $fn_block2),
                    )*
                ]),
            })));
            mem.borrow_mut().properties.insert(stringify!($name).into(), class);
        }
    };
}

#[macro_export]
macro_rules! class_fn {
    ($mem:ident, $function:ident, $class_name:ident, $fn_name:expr, fn, $fn_block:expr) => {
        (
            $fn_name,
            new_runnable(
                $function.clone(),
                Some(format!("{}.{}", stringify!($class_name), stringify!($fn_name)).leak()),
                prebuild_runnable($mem.clone(), Box::new($fn_block)),
            ),
        )
    };
    ($mem:ident, $function:ident, $class_name:ident, $fn_name:expr, fn_direct, $fn_block:expr) => {
        (
            $fn_name,
            new_runnable(
                $function.clone(),
                Some(format!("{}.{}", stringify!($class_name), stringify!($fn_name)).leak()),
                prebuild_runnable_direct($mem.clone(), Box::new($fn_block)),
            ),
        )
    };
    ($mem:ident, $function:ident, $class_name:ident, $fn_name:expr, fn_gen, $generator:expr) => {
        (
            $fn_name,
            new_generator(
                $function.clone(),
                Some(format!("{}.{}", stringify!($class_name), stringify!($fn_name)).leak()),
                $generator,
            ),
        )
    };
}
