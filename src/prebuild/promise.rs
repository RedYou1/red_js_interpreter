use crate::prebuild::prelude::*;

type JsRef = Rc<RefCell<JsValue>>;
type ObjectRef = Rc<RefCell<Prototype>>;

const PROMISE_MARKER: &str = "__red_js_promise__";
const PROMISE_FULFILLED: &str = "__red_js_promise_fulfilled__";
const PROMISE_RESULT: &str = "__red_js_promise_result__";

pub fn prebuild_promise(environment: Environment) {
    let object_prototype = Prototype::find(environment.mem.clone(), &stringify!(Object).into())
        .1
        .borrow()
        .unwrap_proto("Promise Object prototype");
    let function_prototype = Prototype::find(environment.mem.clone(), &stringify!(Function).into())
        .1
        .borrow()
        .unwrap_proto("Promise Function prototype");
    let promise_prototype = Prototype::new_child(object_prototype.clone(), Some("Promise"), []);

    let then = new_runnable(
        function_prototype.clone(),
        "Promise.then",
        prebuild_runnable_direct(environment.clone(), Box::new(then_method)),
    );
    let catch = new_runnable(
        function_prototype.clone(),
        "Promise.catch",
        prebuild_runnable_direct(environment.clone(), Box::new(catch_method)),
    );
    promise_prototype
        .borrow_mut()
        .properties
        .insert("then".into(), then);
    promise_prototype
        .borrow_mut()
        .properties
        .insert("catch".into(), catch);

    let resolve = new_runnable(
        function_prototype.clone(),
        "Promise.resolve",
        prebuild_runnable_direct(
            environment.clone(),
            Box::new(|environment, _, arguments| {
                let result = arguments.into_iter().next().unwrap_or_else(undefined_value);
                CodeResult::Return(resolved_promise(&environment, result))
            }),
        ),
    );
    let reject = new_runnable(
        function_prototype,
        "Promise.reject",
        prebuild_runnable_direct(
            environment.clone(),
            Box::new(|environment, _, arguments| {
                let result = arguments.into_iter().next().unwrap_or_else(undefined_value);
                CodeResult::Return(rejected_promise(&environment, result))
            }),
        ),
    );
    let promise = Prototype::new_child(
        object_prototype,
        Some("Promise"),
        [
            (
                "prototype".into(),
                value(JsValue::Prototype(promise_prototype)),
            ),
            ("resolve".into(), resolve),
            ("reject".into(), reject),
        ],
    );
    environment
        .mem
        .borrow_mut()
        .properties
        .insert("Promise".into(), value(JsValue::Prototype(promise)));
}

pub fn resolved_promise(environment: &Environment, result: JsRef) -> JsRef {
    if promise_state(&result).is_some() {
        return result;
    }
    new_settled_promise(environment, true, result)
}

pub fn rejected_promise(environment: &Environment, result: JsRef) -> JsRef {
    new_settled_promise(environment, false, result)
}

fn new_settled_promise(environment: &Environment, fulfilled: bool, result: JsRef) -> JsRef {
    let promise_namespace = Prototype::find(environment.mem.clone(), &"Promise".into()).1;
    let promise_prototype = Prototype::find(
        promise_namespace.borrow().unwrap_proto("Promise namespace"),
        &"prototype".into(),
    )
    .1;
    let JsValue::Prototype(promise_prototype) = inline_borrow!(promise_prototype) else {
        unreachable!("Promise.prototype must be an object")
    };
    let promise = Prototype::new_child(
        promise_prototype,
        None,
        [
            (PROMISE_MARKER.into(), value(JsValue::Boolean(true))),
            (PROMISE_FULFILLED.into(), value(JsValue::Boolean(fulfilled))),
            (PROMISE_RESULT.into(), result),
        ],
    );
    promise.borrow_mut().non_enumerable.extend([
        PROMISE_MARKER.into(),
        PROMISE_FULFILLED.into(),
        PROMISE_RESULT.into(),
    ]);
    value(JsValue::Prototype(promise))
}

fn promise_state(promise: &JsRef) -> Option<(bool, JsRef)> {
    let JsValue::Prototype(promise) = inline_borrow!(promise.clone()) else {
        return None;
    };
    if !matches!(
        inline_borrow!(Prototype::find(promise.clone(), &PROMISE_MARKER.into()).1),
        JsValue::Boolean(true)
    ) {
        return None;
    }
    let fulfilled = matches!(
        inline_borrow!(Prototype::find(promise.clone(), &PROMISE_FULFILLED.into()).1),
        JsValue::Boolean(true)
    );
    let result = Prototype::find(promise, &PROMISE_RESULT.into()).1;
    Some((fulfilled, result))
}

fn then_method(environment: Environment, this: JsRef, arguments: Vec<JsRef>) -> CodeResult {
    let Some((fulfilled, result)) = promise_state(&this) else {
        return promise_type_error("Promise.then called on a non-Promise value");
    };
    let callback = if fulfilled {
        arguments.first()
    } else {
        arguments.get(1)
    };
    settle_callback(&environment, fulfilled, result, callback)
}

fn catch_method(environment: Environment, this: JsRef, arguments: Vec<JsRef>) -> CodeResult {
    let Some((fulfilled, result)) = promise_state(&this) else {
        return promise_type_error("Promise.catch called on a non-Promise value");
    };
    let callback = if fulfilled { None } else { arguments.first() };
    settle_callback(&environment, fulfilled, result, callback)
}

fn settle_callback(
    environment: &Environment,
    fulfilled: bool,
    result: JsRef,
    callback: Option<&JsRef>,
) -> CodeResult {
    let Some(callback) = callback.and_then(as_callable) else {
        return CodeResult::Return(if fulfilled {
            resolved_promise(environment, result)
        } else {
            rejected_promise(environment, result)
        });
    };
    let callback_result = run_function_object(
        callback,
        undefined_value(),
        vec![result],
        environment.logger.clone(),
    );
    match callback_result {
        CodeResult::Return(result) | CodeResult::Normal(result) => {
            CodeResult::Return(resolved_promise(environment, result))
        }
        CodeResult::NormalMember(result, _, _) => {
            CodeResult::Return(resolved_promise(environment, result))
        }
        CodeResult::Error(error) => CodeResult::Return(rejected_promise(environment, error)),
        _ => CodeResult::Return(resolved_promise(environment, undefined_value())),
    }
}

fn as_callable(value: &JsRef) -> Option<ObjectRef> {
    let JsValue::Prototype(object) = inline_borrow!(value.clone()) else {
        return None;
    };
    let runnable = Prototype::opt_find(object.clone(), &crate::RUNNABLE.into())?.1;
    matches!(&*runnable.borrow(), JsValue::Function(_)).then_some(object)
}

fn promise_type_error(message: &str) -> CodeResult {
    CodeResult::Error(value(JsValue::String(message.to_owned())))
}

fn value(value: JsValue) -> JsRef {
    Rc::new(RefCell::new(value))
}

fn undefined_value() -> JsRef {
    value(JsValue::Undefined)
}
