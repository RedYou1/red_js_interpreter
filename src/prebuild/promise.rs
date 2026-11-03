use crate::RUNNABLE;
use crate::prebuild::prelude::*;

const STATE: &str = "__promise_state__";
const RESULT: &str = "__promise_result__";

#[derive(Clone, Copy)]
enum PromiseState {
    Fulfilled,
    Rejected,
}

pub fn prebuild_promise(env: Environment) {
    let object = Prototype::find(env.mem.clone(), &stringify!(Object).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_promise for Object");
    let function = Prototype::find(env.mem.clone(), &stringify!(Function).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_promise for Function");
    let prototype = Prototype::new_child(object.clone(), None, []);
    let promise = Prototype::new_child(object, None, []);
    promise.borrow_mut().properties.insert(
        "prototype".into(),
        Rc::new(RefCell::new(JsValue::Prototype(prototype.clone()))),
    );

    for (name, state) in [
        ("resolve", PromiseState::Fulfilled),
        ("reject", PromiseState::Rejected),
    ] {
        let runnable = prebuild_runnable_direct(
            env.clone(),
            Box::new(move |env, _, arguments| {
                let value = arguments
                    .first()
                    .cloned()
                    .unwrap_or_else(|| Rc::new(RefCell::new(JsValue::Undefined)));
                CodeResult::Return(match state {
                    PromiseState::Fulfilled => promise_resolve(&env, value),
                    PromiseState::Rejected => promise_reject(&env, value),
                })
            }),
        );
        promise
            .borrow_mut()
            .properties
            .insert(name.into(), new_runnable(function.clone(), name, runnable));
    }

    let then = new_runnable(
        function.clone(),
        "Promise.then",
        prebuild_runnable(
            env.clone(),
            Box::new(|env, this, [on_fulfilled, on_rejected]| {
                promise_then(env, this, on_fulfilled, on_rejected)
            }),
        ),
    );
    prototype
        .borrow_mut()
        .properties
        .insert("then".into(), then);
    let catch = new_runnable(
        function,
        "Promise.catch",
        prebuild_runnable(
            env.clone(),
            Box::new(|env, this, [on_rejected]| {
                promise_then(
                    env,
                    this,
                    Rc::new(RefCell::new(JsValue::Undefined)),
                    on_rejected,
                )
            }),
        ),
    );
    prototype
        .borrow_mut()
        .properties
        .insert("catch".into(), catch);
    env.mem.borrow_mut().properties.insert(
        "Promise".into(),
        Rc::new(RefCell::new(JsValue::Prototype(promise))),
    );
}

/// Creates an already-fulfilled Promise value for a host API result.
pub fn promise_resolve(env: &Environment, value: Rc<RefCell<JsValue>>) -> Rc<RefCell<JsValue>> {
    if promise_state(&value).is_some() {
        return value;
    }
    new_promise(env, PromiseState::Fulfilled, value)
}

/// Creates an already-rejected Promise value for a host API failure.
pub fn promise_reject(env: &Environment, reason: Rc<RefCell<JsValue>>) -> Rc<RefCell<JsValue>> {
    new_promise(env, PromiseState::Rejected, reason)
}

fn new_promise(
    env: &Environment,
    state: PromiseState,
    result: Rc<RefCell<JsValue>>,
) -> Rc<RefCell<JsValue>> {
    let promise = Prototype::find(env.mem.clone(), &"Promise".into())
        .1
        .borrow()
        .unwrap_proto("new_promise for Promise");
    let prototype = Prototype::find(promise, &"prototype".into())
        .1
        .borrow()
        .unwrap_proto("new_promise for Promise.prototype");
    let state = match state {
        PromiseState::Fulfilled => "fulfilled",
        PromiseState::Rejected => "rejected",
    };
    Rc::new(RefCell::new(JsValue::Prototype(Prototype::new_child(
        prototype,
        None,
        [
            (
                STATE.into(),
                Rc::new(RefCell::new(JsValue::String(state.into()))),
            ),
            (RESULT.into(), result),
        ],
    ))))
}

fn promise_state(value: &Rc<RefCell<JsValue>>) -> Option<(PromiseState, Rc<RefCell<JsValue>>)> {
    let JsValue::Prototype(promise) = inline_borrow!(value.clone()) else {
        return None;
    };
    let promise = promise.borrow();
    let state = promise.properties.get(&STATE.into()).cloned()?;
    let state = match inline_borrow!(state) {
        JsValue::String(state) if state == "fulfilled" => PromiseState::Fulfilled,
        JsValue::String(state) if state == "rejected" => PromiseState::Rejected,
        _ => return None,
    };
    Some((state, promise.properties.get(&RESULT.into())?.clone()))
}

fn promise_then(
    env: Environment,
    this: Rc<RefCell<JsValue>>,
    on_fulfilled: Rc<RefCell<JsValue>>,
    on_rejected: Rc<RefCell<JsValue>>,
) -> CodeResult {
    let Some((state, result)) = promise_state(&this) else {
        return promise_type_error(env);
    };
    let handler = match state {
        PromiseState::Fulfilled => on_fulfilled,
        PromiseState::Rejected => on_rejected,
    };
    let Some(handler) = callable_handler(handler) else {
        return CodeResult::Return(new_promise(&env, state, result));
    };
    match run_function_object(
        handler,
        Rc::new(RefCell::new(JsValue::Undefined)),
        vec![result],
        env.logger.clone(),
    ) {
        CodeResult::Return(value)
        | CodeResult::Normal(value)
        | CodeResult::NormalMember(value, _, _) => CodeResult::Return(promise_resolve(&env, value)),
        CodeResult::Error(reason) => CodeResult::Return(promise_reject(&env, reason)),
        _ => CodeResult::Return(promise_resolve(
            &env,
            Rc::new(RefCell::new(JsValue::Undefined)),
        )),
    }
}

fn callable_handler(handler: Rc<RefCell<JsValue>>) -> Option<Rc<RefCell<Prototype>>> {
    let JsValue::Prototype(handler) = inline_borrow!(handler) else {
        return None;
    };
    let runnable = Prototype::opt_find(handler.clone(), &RUNNABLE.into())?.1;
    matches!(inline_borrow!(runnable), JsValue::Function(_)).then_some(handler)
}

fn promise_type_error(env: Environment) -> CodeResult {
    let prototype = Prototype::find(env.mem, &stringify!(TypeError).into())
        .1
        .borrow()
        .unwrap_proto("Promise TypeError");
    let constructor = Rc::new(RefCell::new(JsValue::Prototype(prototype.clone())));
    CodeResult::Error(Rc::new(RefCell::new(JsValue::Prototype(
        Prototype::new_child(prototype, None, [("constructor".into(), constructor)]),
    ))))
}
