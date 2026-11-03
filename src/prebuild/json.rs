use std::collections::HashSet;

use serde_json::{Map, Number, Value};

use crate::prebuild::prelude::*;

pub fn prebuild_json(env: Environment) {
    let object = Prototype::find(env.mem.clone(), &stringify!(Object).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_json for Object");
    let function = Prototype::find(env.mem.clone(), &stringify!(Function).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_json for Function");
    let json = Prototype::new_child(object, Some("JSON"), []);
    let parse = new_runnable(
        function.clone(),
        "JSON.parse",
        prebuild_runnable_direct(
            env.clone(),
            Box::new(|env, _, arguments| {
                let source = arguments
                    .first()
                    .map(|value| inline_borrow!(value.clone()).print())
                    .unwrap_or_else(|| "undefined".to_owned());
                match serde_json::from_str::<Value>(&source) {
                    Ok(value) => CodeResult::Return(json_to_js(value, &env)),
                    Err(_) => json_error(env, "SyntaxError"),
                }
            }),
        ),
    );
    let stringify = new_runnable(
        function,
        "JSON.stringify",
        prebuild_runnable_direct(
            env.clone(),
            Box::new(|env, _, arguments| {
                let Some(value) = arguments.first() else {
                    return CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)));
                };
                match js_to_json(value, false, &mut HashSet::new()) {
                    Ok(Some(value)) => match serde_json::to_string(&value) {
                        Ok(value) => {
                            CodeResult::Return(Rc::new(RefCell::new(JsValue::String(value))))
                        }
                        Err(_) => json_error(env, "TypeError"),
                    },
                    Ok(None) => CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined))),
                    Err(()) => json_error(env, "TypeError"),
                }
            }),
        ),
    );
    json.borrow_mut().properties.insert("parse".into(), parse);
    json.borrow_mut()
        .properties
        .insert("stringify".into(), stringify);
    env.mem.borrow_mut().properties.insert(
        "JSON".into(),
        Rc::new(RefCell::new(JsValue::Prototype(json))),
    );
}

fn json_to_js(value: Value, env: &Environment) -> Rc<RefCell<JsValue>> {
    let value = match value {
        Value::Null => JsValue::Null,
        Value::Bool(value) => JsValue::Boolean(value),
        Value::Number(value) => value
            .as_i64()
            .map(JsValue::BigInt)
            .or_else(|| value.as_f64().map(JsValue::Number))
            .unwrap_or(JsValue::Number(f64::NAN)),
        Value::String(value) => JsValue::String(value),
        Value::Array(values) => {
            let array = Prototype::find(env.mem.clone(), &stringify!(Array).into())
                .1
                .borrow()
                .unwrap_proto("JSON.parse for Array");
            return new_array(
                array,
                values
                    .into_iter()
                    .map(|value| json_to_js(value, env))
                    .collect(),
                env.logger.clone(),
            );
        }
        Value::Object(values) => {
            let object = Prototype::find(env.mem.clone(), &stringify!(Object).into())
                .1
                .borrow()
                .unwrap_proto("JSON.parse for Object");
            return Rc::new(RefCell::new(JsValue::Prototype(Prototype::new_child(
                object,
                None,
                values
                    .into_iter()
                    .map(|(key, value)| (key.into(), json_to_js(value, env))),
            ))));
        }
    };
    Rc::new(RefCell::new(value))
}

fn js_to_json(
    value: &Rc<RefCell<JsValue>>,
    in_array: bool,
    ancestors: &mut HashSet<usize>,
) -> Result<Option<Value>, ()> {
    let value = inline_borrow!(value.clone());
    match value {
        JsValue::Undefined
        | JsValue::Function(_)
        | JsValue::Generator(_)
        | JsValue::Symbol(_, _) => Ok(in_array.then_some(Value::Null)),
        JsValue::Null => Ok(Some(Value::Null)),
        JsValue::Boolean(value) => Ok(Some(Value::Bool(value))),
        JsValue::String(value) => Ok(Some(Value::String(value))),
        JsValue::Number(value) => Ok(Some(
            Number::from_f64(value)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        )),
        JsValue::BigInt(_) => Err(()),
        JsValue::RedGex(_) => Ok(Some(Value::Object(Map::new()))),
        JsValue::Prototype(object) => {
            let pointer = Rc::as_ptr(&object) as usize;
            if !ancestors.insert(pointer) {
                return Err(());
            }
            let is_array = object
                .borrow()
                .parent()
                .is_some_and(|parent| parent.borrow().name == Some("Array"));
            let result = if is_array {
                let length = object
                    .borrow()
                    .properties
                    .get(&JsValue::from("length"))
                    .and_then(|value| match inline_borrow!(value.clone()) {
                        JsValue::BigInt(length) => Some(length.max(0) as usize),
                        _ => None,
                    })
                    .unwrap_or(0);
                let mut values = Vec::with_capacity(length);
                for index in 0..length {
                    let value = object
                        .borrow()
                        .properties
                        .get(&JsValue::from(index))
                        .cloned()
                        .unwrap_or_else(|| Rc::new(RefCell::new(JsValue::Undefined)));
                    values.push(js_to_json(&value, true, ancestors)?.unwrap_or(Value::Null));
                }
                Some(Value::Array(values))
            } else {
                let properties = {
                    let object = object.borrow();
                    object
                        .properties
                        .iter()
                        .filter_map(|(key, value)| match key {
                            JsValue::String(key)
                                if key != "__proto__"
                                    && !object
                                        .non_enumerable
                                        .contains(&JsValue::String(key.clone())) =>
                            {
                                Some((key.clone(), value.clone()))
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                };
                let mut values = Map::new();
                for (key, value) in properties {
                    if let Some(value) = js_to_json(&value, false, ancestors)? {
                        values.insert(key, value);
                    }
                }
                Some(Value::Object(values))
            };
            ancestors.remove(&pointer);
            Ok(result)
        }
    }
}

fn json_error(env: Environment, name: &str) -> CodeResult {
    let prototype = Prototype::find(env.mem, &name.into())
        .1
        .borrow()
        .unwrap_proto("JSON error prototype");
    let constructor = Rc::new(RefCell::new(JsValue::Prototype(prototype.clone())));
    CodeResult::Error(Rc::new(RefCell::new(JsValue::Prototype(
        Prototype::new_child(prototype, None, [("constructor".into(), constructor)]),
    ))))
}
