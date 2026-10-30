use std::{cell::RefCell, collections::HashSet, rc::Rc};

use serde_json::{Map, Number, Value};

use crate::prebuild::prelude::*;

const MAX_JSON_DEPTH: usize = 128;
type JsRef = Rc<RefCell<JsValue>>;

pub fn prebuild_json(environment: Environment) {
    let object_prototype = Prototype::find(environment.mem.clone(), &stringify!(Object).into())
        .1
        .borrow()
        .unwrap_proto("JSON Object prototype");
    let function_prototype = Prototype::find(environment.mem.clone(), &stringify!(Function).into())
        .1
        .borrow()
        .unwrap_proto("JSON Function prototype");
    let parse = new_runnable(
        function_prototype.clone(),
        "JSON.parse",
        prebuild_runnable(environment.clone(), Box::new(parse_json)),
    );
    let stringify = new_runnable(
        function_prototype,
        "JSON.stringify",
        prebuild_runnable(environment.clone(), Box::new(stringify_json)),
    );
    let json = Prototype::new_child(
        object_prototype,
        Some("JSON"),
        [("parse".into(), parse), ("stringify".into(), stringify)],
    );
    environment.mem.borrow_mut().properties.insert(
        "JSON".into(),
        Rc::new(RefCell::new(JsValue::Prototype(json))),
    );
}

fn parse_json(environment: Environment, _this: JsRef, [input]: [JsRef; 1]) -> CodeResult {
    let source = inline_borrow!(input).print();
    match serde_json::from_str::<Value>(&source)
        .map_err(|error| error.to_string())
        .and_then(|value| json_to_js(value, &environment, 0))
    {
        Ok(value) => CodeResult::Return(value),
        Err(error) => CodeResult::Error(Rc::new(RefCell::new(JsValue::String(error)))),
    }
}

fn stringify_json(_environment: Environment, _this: JsRef, [input]: [JsRef; 1]) -> CodeResult {
    let mut ancestors = HashSet::new();
    match js_to_json(&input, &mut ancestors, 0) {
        Ok(Some(value)) => match serde_json::to_string(&value) {
            Ok(value) => CodeResult::Return(Rc::new(RefCell::new(JsValue::String(value)))),
            Err(error) => {
                CodeResult::Error(Rc::new(RefCell::new(JsValue::String(error.to_string()))))
            }
        },
        Ok(None) => CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined))),
        Err(error) => CodeResult::Error(Rc::new(RefCell::new(JsValue::String(error)))),
    }
}

fn json_to_js(value: Value, environment: &Environment, depth: usize) -> Result<JsRef, String> {
    if depth > MAX_JSON_DEPTH {
        return Err("JSON nesting limit exceeded".to_owned());
    }
    match value {
        Value::Null => Ok(Rc::new(RefCell::new(JsValue::Null))),
        Value::Bool(value) => Ok(Rc::new(RefCell::new(JsValue::Boolean(value)))),
        Value::Number(value) => Ok(Rc::new(RefCell::new(js_number(value)))),
        Value::String(value) => Ok(Rc::new(RefCell::new(JsValue::String(value)))),
        Value::Array(values) => {
            let array = Prototype::find(environment.mem.clone(), &stringify!(Array).into())
                .1
                .borrow()
                .unwrap_proto("JSON.parse Array prototype");
            let values = values
                .into_iter()
                .map(|value| json_to_js(value, environment, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(new_array(array, values, environment.logger.clone()))
        }
        Value::Object(values) => {
            let object = Prototype::find(environment.mem.clone(), &stringify!(Object).into())
                .1
                .borrow()
                .unwrap_proto("JSON.parse Object prototype");
            let values = values
                .into_iter()
                .map(|(key, value)| {
                    json_to_js(value, environment, depth + 1).map(|value| (key.into(), value))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Rc::new(RefCell::new(JsValue::Prototype(
                Prototype::new_child(object, None, values),
            ))))
        }
    }
}

fn js_to_json(
    value: &Rc<RefCell<JsValue>>,
    ancestors: &mut HashSet<usize>,
    depth: usize,
) -> Result<Option<Value>, String> {
    if depth > MAX_JSON_DEPTH {
        return Err("JSON nesting limit exceeded".to_owned());
    }
    match inline_borrow!(value.clone()) {
        JsValue::Null => Ok(Some(Value::Null)),
        JsValue::Undefined
        | JsValue::Function(_)
        | JsValue::Generator(_)
        | JsValue::Symbol(_, _) => Ok(None),
        JsValue::Boolean(value) => Ok(Some(Value::Bool(value))),
        JsValue::String(value) => Ok(Some(Value::String(value))),
        JsValue::BigInt(_) => Err("JSON.stringify cannot serialize BigInt".to_owned()),
        JsValue::Number(value) => Ok(Some(json_number(value))),
        JsValue::RedGex(_) => Ok(Some(Value::Object(Map::new()))),
        JsValue::Prototype(object) => {
            let pointer = Rc::as_ptr(&object) as usize;
            if !ancestors.insert(pointer) {
                return Err("JSON.stringify cannot serialize cyclic values".to_owned());
            }
            let result = if let Some(length) = array_length(&object) {
                stringify_array(&object, length, ancestors, depth)
            } else {
                stringify_object(&object, ancestors, depth)
            };
            ancestors.remove(&pointer);
            result.map(Some)
        }
    }
}

fn array_length(object: &Rc<RefCell<Prototype>>) -> Option<usize> {
    let length = Prototype::opt_find(object.clone(), &"length".into())?.1;
    match inline_borrow!(length) {
        JsValue::BigInt(length) if length >= 0 => Some(length as usize),
        _ => None,
    }
}

fn stringify_array(
    array: &Rc<RefCell<Prototype>>,
    length: usize,
    ancestors: &mut HashSet<usize>,
    depth: usize,
) -> Result<Value, String> {
    let mut values = Vec::with_capacity(length);
    for index in 0..length {
        let value = Prototype::find(array.clone(), &JsValue::BigInt(index as i64)).1;
        values.push(js_to_json(&value, ancestors, depth + 1)?.unwrap_or(Value::Null));
    }
    Ok(Value::Array(values))
}

fn stringify_object(
    object: &Rc<RefCell<Prototype>>,
    ancestors: &mut HashSet<usize>,
    depth: usize,
) -> Result<Value, String> {
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
    let mut result = Map::with_capacity(properties.len());
    for (key, value) in properties {
        if let Some(value) = js_to_json(&value, ancestors, depth + 1)? {
            result.insert(key, value);
        }
    }
    Ok(Value::Object(result))
}

fn js_number(value: Number) -> JsValue {
    if let Some(integer) = value.as_i64() {
        JsValue::Number(integer as f64)
    } else {
        JsValue::Number(value.as_f64().unwrap_or(f64::NAN))
    }
}

fn json_number(value: f64) -> Value {
    if value.is_finite()
        && value.fract() == 0.0
        && value >= i64::MIN as f64
        && value <= i64::MAX as f64
    {
        Value::Number((value as i64).into())
    } else {
        Number::from_f64(value).map_or(Value::Null, Value::Number)
    }
}
