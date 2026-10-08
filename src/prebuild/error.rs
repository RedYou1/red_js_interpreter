use crate::prebuild::prelude::*;

new_class! {
    prebuild_error,
    Error,
    Object,;
    constructor, fn,
    |env, _, [arg]| {
        let error = Prototype::find(env.mem.clone(), &stringify!(Error).into()).1.borrow().unwrap_proto("Error.constructor for Error");
        let proto = Prototype::new_child(
            error,
            None,
            vec![("message".into(), arg)],
        );
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Prototype(proto))))
    };
}

new_class! {
    prebuild_type_error,
    TypeError,
    Error,;
    constructor, fn,
    |env, _, [arg]| {
        let error = Prototype::find(env.mem.clone(), &stringify!(TypeError).into()).1.borrow().unwrap_proto("TypeError.constructor for TypeError");
        let proto = Prototype::new_child(
            error,
            None,
            vec![("message".into(), arg)],
        );
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Prototype(proto))))
    };
}

new_class! {
    prebuild_reference_error,
    ReferenceError,
    Error,;
    constructor, fn,
    |env, _, [arg]| {
        let error = Prototype::find(env.mem.clone(), &stringify!(ReferenceError).into())
            .1
            .borrow()
            .unwrap_proto("ReferenceError.constructor for ReferenceError");
        let proto = Prototype::new_child(error, None, vec![("message".into(), arg)]);
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Prototype(proto))))
    };
}

new_class! {
    prebuild_syntax_error,
    SyntaxError,
    Error,;
    constructor, fn,
    |env, _, [arg]| {
        let error = Prototype::find(env.mem.clone(), &stringify!(SyntaxError).into()).1.borrow().unwrap_proto("SyntaxError.constructor for SyntaxError");
        let proto = Prototype::new_child(
            error,
            None,
            vec![("message".into(), arg)],
        );
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Prototype(proto))))
    };
}

pub(crate) fn error_result(env: Environment, name: &str, message: String) -> CodeResult {
    let error = Prototype::find(env.mem, &name.into())
        .1
        .borrow()
        .unwrap_proto("error_result");
    let constructor = Rc::new(RefCell::new(JsValue::Prototype(error.clone())));
    CodeResult::Error(Rc::new(RefCell::new(JsValue::Prototype(
        Prototype::new_child(
            error,
            None,
            [
                (
                    "message".into(),
                    Rc::new(RefCell::new(JsValue::String(message))),
                ),
                ("constructor".into(), constructor),
            ],
        ),
    ))))
}
