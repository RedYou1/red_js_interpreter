use serial_test::serial;
pub use std::{cell::RefCell, rc::Rc};
use std::{fs, ptr::null_mut};

use crate::prebuild::console::Loggable;
pub use crate::{
    JsValue, Prototype, new_runnable,
    prebuild::{
        array::new_array,
        array::new_array_with_length,
        console::{CONSOLE_LOGS, default_console_config},
    },
    prebuild_prototypes, run_function_object,
};

mod base;
mod compiler;
mod switch;
mod try_catch;

fn prebuild_prototypes_test<T>(logs: &mut Loggable<T>) -> Rc<RefCell<Prototype>> {
    let protos = prebuild_prototypes(default_console_config);
    let console = Prototype::find(protos.clone(), &JsValue::String("console".to_owned()))
        .1
        .borrow()
        .unwrap_proto("prebuild_prototypes_test for console");
    console.borrow_mut().properties.insert(
        JsValue::String(CONSOLE_LOGS.to_owned()),
        Rc::new(RefCell::new(JsValue::BigInt(
            logs as *mut Loggable<T> as i64,
        ))),
    );
    protos
}

struct AssertResultData<'a> {
    wanted: &'a [String],
    current: usize,
}

fn append(data_ptr: &mut i64, value: String) {
    let data = unsafe { ((*data_ptr) as *mut AssertResultData).as_mut_unchecked() };
    assert!(data.current < data.wanted.len());
    if data.wanted[data.current].ne(&value) {
        panic!(
            "err at result {}:\n{}{}\n{}\n-----------------\n!= {}",
            data.current,
            if data.current > NB_LOGS { "...\n" } else { "" },
            data.wanted[(data.current.saturating_sub(NB_LOGS))..data.current].join("\n"),
            value,
            data.wanted[data.current]
        );
    }
    data.current += 1;
}

const NB_LOGS: usize = 10;
#[macro_export]
macro_rules! assert_result {
    ($name:ident, $src:expr, $($result:expr),*) => {
        #[test]
        fn $name() {
            let wanted = [$($result.to_owned(),)*];
            let mut data = AssertResultData {
                wanted: &wanted,
                current: 0,
            };
            let protos = prebuild_prototypes_test(&mut Loggable::<i64> {
                logger: &(append as fn(&mut i64, String)),
                data: &mut data as *mut AssertResultData as i64,
            });

            let program = $crate::parser::parse($src)
                //.expect("parse failed")
                .compile(protos.clone());
            run_function_object(
                new_runnable(
                    Prototype::find(protos, &JsValue::String("Function".to_owned()))
                        .1.borrow()
                        .unwrap_proto("tests::compiler::assert_result! for Function"),
                    "__main__",
                    program,
                ).borrow()
                .unwrap_proto("tests::compiler::assert_result! for Result"),
                Rc::new(RefCell::new(JsValue::Undefined)),
                vec![],
            );
        }
    }
}

macro_rules! test_only_parse (
    ($fn_name:ident, $name:expr) => {
        #[test]
        #[serial(zzzzzzz)]
        #[ignore]
        fn $fn_name() {
            let protos =
                prebuild_prototypes_test(unsafe { null_mut::<Loggable<()>>().as_mut_unchecked() });

            let _program = crate::parser::parse(
                fs::read_to_string(format!("./src/tests/only_parses/{}.js", $name))
                    .unwrap()
                    .as_str(),
            )
            .compile(protos.clone());
        }
    }
);

test_only_parse!(only_parses_recaptcha, "recaptcha");
