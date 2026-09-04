use std::{cell::RefCell, mem::MaybeUninit, rc::Rc};

use crate::{ARGUMENTS, CodeResult, Environment, JsValue, Prototype, Runnable, inline_borrow};

pub mod array;
pub mod console;
pub mod date;
pub mod error;
pub mod iterator;
pub mod math;
pub mod number;
mod prelude;
pub mod string;
pub mod symbol;

pub fn prebuild_runnable<const WANTED_PARAM: usize>(
    env: Environment,
    fun: Box<
        dyn Fn(
            Environment,
            Rc<RefCell<JsValue>>,
            [Rc<RefCell<JsValue>>; WANTED_PARAM],
        ) -> CodeResult,
    >,
) -> Runnable {
    Runnable {
        params: Vec::new(),
        excess: None,
        mem: env.mem.clone(),
        code: vec![Box::new(move |env, _| {
            let JsValue::Prototype(array) =
                inline_borrow!(Prototype::find(env.mem.clone(), &ARGUMENTS.into()).1)
            else {
                env.logger.borrow_mut().logln(crate::LogLevel::Fatal, &|| {
                    format!("prebuild_runnable missing prototype argument array key={ARGUMENTS}")
                });
                panic!("{ARGUMENTS} not found prebuild function")
            };

            let mut params = MaybeUninit::<[Rc<RefCell<JsValue>>; WANTED_PARAM]>::uninit();
            let t: &mut [MaybeUninit<Rc<RefCell<JsValue>>>] =
                unsafe { &mut *(params.as_mut() as *mut _ as *mut _) };
            for (i, t) in t.iter_mut().enumerate() {
                t.write(Prototype::find(array.clone(), &i.into()).1);
            }

            fun(
                env.clone(),
                Prototype::find(env.mem, &"this".into()).1,
                unsafe { params.assume_init() },
            )
        })],
    }
}

pub fn prebuild_runnable_direct(
    env: Environment,
    fun: Box<
        dyn Fn(
            Environment,
            Rc<RefCell<JsValue>>,
            Vec<Rc<RefCell<JsValue>>>,
        ) -> CodeResult,
    >,
) -> Runnable {
    Runnable {
        params: Vec::new(),
        excess: None,
        mem: env.mem.clone(),
        code: vec![Box::new(move |env, _| {
            let JsValue::Prototype(array) =
                inline_borrow!(Prototype::find(env.mem.clone(), &ARGUMENTS.into()).1)
            else {
                env.logger.borrow_mut().logln(crate::LogLevel::Fatal, &|| {
                    format!(
                        "prebuild_runnable_direct missing prototype argument array key={ARGUMENTS}"
                    )
                });
                panic!("{ARGUMENTS} not found prebuild function")
            };
            let JsValue::BigInt(length) =
                inline_borrow!(Prototype::find(array.clone(), &"length".into()).1)
            else {
                env.logger.borrow_mut().logln_str(
                    crate::LogLevel::Fatal,
                    "prebuild_runnable_direct arguments length is not BigInt",
                );
                panic!("length in Array")
            };

            let mut params = Vec::with_capacity(length as usize);

            for i in 0..length as usize {
                params.push(Prototype::find(array.clone(), &i.into()).1);
            }

            fun(
                env.clone(),
                Prototype::find(env.mem, &"this".into()).1,
                params,
            )
        })],
    }
}

#[macro_export]
macro_rules! new_class {
    ($func_name:ident, $name:ident, $parent:ident, $($var_static_name:ident, $var_static:expr),*; $($fn_name:ident, $fn_type:tt, $fn_block:expr),*; $($fn_name2:expr, $fn_type2:tt, $($fn_block2:expr);+),*) => {
        pub fn $func_name(env: Environment) {
            let function = Prototype::find(env.mem.clone(), &stringify!(Function).into()).1.borrow().unwrap_proto("new_class! for Function");
            let class = JsValue::Prototype(Rc::new(RefCell::new(Prototype {
                name: Some(stringify!($name)),
                properties: HashMap::from([
                    (
                        PROTO_NAME.into(),
                        Prototype::find(env.mem.clone(), &stringify!($parent).into()).1,
                    ),
                    $(
                        (stringify!($var_static_name).into(), Rc::new(RefCell::new($var_static))),
                    )*
                    $(
                        class_fn!(env, function, stringify!($fn_name).into(), format!("{}.{}", stringify!($name), stringify!($fn_name)).leak(), $fn_type, $fn_block),
                    )*
                    $(
                        class_fn!(env, function, {
                            let mut temp = Rc::new(RefCell::new(JsValue::Prototype(env.mem.clone())));
                            stringify!($fn_name2).split('.').for_each(|x| temp = Prototype::find(temp.clone().borrow().unwrap_proto("new_class! recusive name"), &x.into()).1);
                            inline_borrow!(temp)
                        }, format!("{}.{{{}}}", stringify!($name), stringify!($fn_name2)).leak(), $fn_type2, $($fn_block2);+),
                    )*
                ]),
                formating: false,
            })));
            if stringify!(Symbol).ne(stringify!($name)) {
                let mem2 = Rc::new(RefCell::new(JsValue::Prototype(env.mem.clone())));
                let symbol = Prototype::find(mem2.clone().borrow().unwrap_proto("new_class! Symbol"), &stringify!(Symbol).into()).1;
                let has_instance = Prototype::find(symbol.clone().borrow().unwrap_proto("new_class! hasInstance"), &"hasInstance".into()).1;
                let name = stringify!($name);
                class.unwrap_proto("new_class!").borrow_mut().properties.insert(inline_borrow!(has_instance),
                    new_runnable(
                        function.clone(),
                        format!("{}.[hasInstance]", stringify!($name)).leak(),
                        prebuild_runnable(env.clone(), Box::new(|_, _, [instance]| {
                            CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean({
                                if let JsValue::Prototype(proto) = inline_borrow!(instance.clone()) {
                                    if let JsValue::Prototype(proto) = inline_borrow!(proto.borrow().properties[&"__proto__".into()].clone()) {
                                        proto.borrow().name.eq(&Some(name))
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            }))))
                        })),
                    ));
            }
            env.mem.borrow_mut().properties.insert(stringify!($name).into(), Rc::new(RefCell::new(class)));
        }
    };
}

#[macro_export]
macro_rules! class_fn {
    ($env:ident, $function:ident, $fn_key:expr, $fn_name:expr, fn, $fn_block:expr) => {
        (
            $fn_key,
            new_runnable(
                $function.clone(),
                $fn_name,
                prebuild_runnable($env.clone(), Box::new($fn_block)),
            ),
        )
    };
    ($env:ident, $function:ident, $fn_key:expr, $fn_name:expr, fn_direct, $fn_block:expr) => {
        (
            $fn_key,
            new_runnable(
                $function.clone(),
                $fn_name,
                prebuild_runnable_direct($env.clone(), Box::new($fn_block)),
            ),
        )
    };
    ($env:ident, $function:ident, $fn_key:expr, $fn_name:expr, fn_gen, $($generator:expr);+) => {
        (
            $fn_key,
            // new_runnable(
            //     $function.clone(),
            //     $fn_name,
            //     prebuild_runnable_direct($env.clone(), Box::new(|proto,_,_|{
                    new_generator(
                        Prototype::find($env.mem.clone(), &stringify!(Function).into()).1.borrow().unwrap_proto("new_class! for Function for fn_gen"),
                        $fn_name,
                        Generator {
                            params: vec![],
                            excess: None,
                            mem: $env.mem.clone(),
                            code: Rc::new([$(Box::new($generator)),+]),
                        },
                    )
            //     })),
            // ),
        )
    };
}
