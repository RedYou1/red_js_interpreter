pub use std::{cell::RefCell, rc::Rc};

use crate::prebuild::console::Loggable;
pub use crate::{
    JsValue, Prototype, new_runnable,
    prebuild::{
        array::new_array,
        console::{CONSOLE_LOGS, default_console_config},
    },
    prebuild_prototypes, run_function_object,
};

mod base;
mod compiler;

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
