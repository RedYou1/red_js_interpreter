use crate::prebuild::prelude::*;

const STRING_DATA_KEY: &str = "__string_data__";

fn string_receiver(this: &Rc<RefCell<JsValue>>) -> Option<String> {
    match inline_borrow!(this.clone()) {
        JsValue::String(value) => Some(value),
        JsValue::Prototype(object) => {
            let value = object
                .borrow()
                .properties
                .get(&STRING_DATA_KEY.into())?
                .clone();
            match inline_borrow!(value) {
                JsValue::String(value) => Some(value),
                _ => None,
            }
        }
        _ => None,
    }
}

fn to_uint16(value: &JsValue) -> u16 {
    let number = match value {
        JsValue::BigInt(value) => *value as f64,
        JsValue::Number(value) => *value,
        JsValue::Boolean(value) => {
            if *value {
                1.0
            } else {
                0.0
            }
        }
        JsValue::Null => 0.0,
        JsValue::Undefined => f64::NAN,
        JsValue::String(value) if value.trim().is_empty() => 0.0,
        JsValue::String(value) => value.trim().parse().unwrap_or(f64::NAN),
        _ => f64::NAN,
    };
    if number.is_finite() {
        number.trunc().rem_euclid(65536.0) as u16
    } else {
        0
    }
}

new_class! {
    prebuild_string_class,
    String,
    Object,;
    constructor, fn,
    |env, this, [arg]| {
        let arg = match inline_borrow!(arg) {
            JsValue::Symbol(_, ref t) =>  *t.clone(),
            v => v
        };
        let result = match &arg {
            JsValue::Prototype(proto) => run_function_object(Prototype::find(proto.clone(), &"toString".into()).1.borrow().unwrap_proto("String.constructor for toString"), Rc::new(RefCell::new(arg.clone())), vec![], env.logger),
            JsValue::String(s) => CodeResult::Return(Rc::new(RefCell::new(JsValue::String(s.clone())))),
            JsValue::Null | JsValue::Undefined => CodeResult::Return(Rc::new(RefCell::new(JsValue::String("".to_owned())))),
            JsValue::BigInt(o) => CodeResult::Return(Rc::new(RefCell::new(JsValue::String(format!("{}", *o))))),
            JsValue::Number(o) => CodeResult::Return(Rc::new(RefCell::new(JsValue::String(format!("{}", *o))))),
            JsValue::Boolean(o) => CodeResult::Return(Rc::new(RefCell::new(JsValue::String(format!("{}", *o))))),
            JsValue::Symbol(_, _) | JsValue::Function(_) | JsValue::Generator(_) | JsValue::RedGex(_) => panic!("not implemented"),
        };
        if let CodeResult::Return(value) = &result
            && let JsValue::String(value) = inline_borrow!(value.clone())
            && let JsValue::Prototype(receiver) = inline_borrow!(this.clone())
        {
            let length = value.encode_utf16().count() as i64;
            let mut receiver = receiver.borrow_mut();
            receiver.properties.insert(
                STRING_DATA_KEY.into(),
                Rc::new(RefCell::new(JsValue::String(value))),
            );
            receiver.properties.insert(
                "length".into(),
                Rc::new(RefCell::new(JsValue::BigInt(length))),
            );
        }
        result
    },
    fromCharCode, fn_direct,
    |env, _, arguments| {
        let code_units = arguments
            .iter()
            .map(|argument| to_uint16(&inline_borrow!(argument.clone())))
            .collect::<Vec<_>>();
        match String::from_utf16(&code_units) {
            Ok(value) => CodeResult::Return(Rc::new(RefCell::new(JsValue::String(value)))),
            Err(_) => {
                let array = Prototype::find(env.mem.clone(), &stringify!(Array).into())
                    .1
                    .borrow()
                    .unwrap_proto("String.fromCharCode for Array");
                let elements = code_units
                    .into_iter()
                    .map(|unit| Rc::new(RefCell::new(JsValue::BigInt(i64::from(unit)))))
                    .collect();
                CodeResult::Return(new_array(array, elements, env.logger))
            }
        }
    },
    length, fn,
    |_, this, []| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::BigInt(
            string_receiver(&this)
                .map_or(0, |value| value.encode_utf16().count() as i64),
        ))))
    },
    charAt, fn,
    |_, this, [index]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) && let JsValue::BigInt(idx) = inline_borrow!(index) && idx >= 0 && (idx as usize) < s.len() {
            JsValue::String(s.chars().nth(idx as usize).unwrap().to_string())
        } else {
            JsValue::String("".to_owned())
        })))
    },
    charCodeAt, fn,
    |_, this, [index]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) && let JsValue::BigInt(idx) = inline_borrow!(index) && idx >= 0 && (idx as usize) < s.len() {
            JsValue::BigInt(s.chars().nth(idx as usize).unwrap() as i64)
        } else {
            JsValue::BigInt(-1)
        })))
    },
    substring, fn,
    |_, this, [start, end]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) {
            let len = s.len() as i64;
            let mut start_idx = 0usize;
            let mut end_idx = len as usize;

            if let JsValue::BigInt(st) = inline_borrow!(start) {
                start_idx = ((st.max(0)).min(len)) as usize;
            }
            if let JsValue::BigInt(en) = inline_borrow!(end) {
                end_idx = ((en.max(0)).min(len)) as usize;
            }

            if start_idx > end_idx {
                std::mem::swap(&mut start_idx, &mut end_idx);
            }

            JsValue::String(s.chars().skip(start_idx).take(end_idx - start_idx).collect())
        } else {
            JsValue::String("".to_owned())
        })))
    },
    slice, fn,
    |_, this, [start, end]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) {
            let len = s.len() as i64;
            let mut start_idx = 0i64;
            let mut end_idx = len;

            if let JsValue::BigInt(st) = inline_borrow!(start) {
                start_idx = if st < 0 { (len + st).max(0) } else { st.min(len) };
            }
            if let JsValue::BigInt(en) = inline_borrow!(end) {
                end_idx = if en < 0 { (len + en).max(0) } else { en.min(len) };
            }

            if start_idx >= end_idx || start_idx < 0 {
                JsValue::String("".to_owned())
            }
            else{
            JsValue::String(s.chars().skip(start_idx as usize).take((end_idx - start_idx) as usize).collect())
            }
        } else {
            JsValue::String("".to_owned())
        })))
    },
    indexOf, fn,
    |_, this, [search, from_index]| {
        if let Some(s) = string_receiver(&this)
            && let JsValue::String(search_str) = inline_borrow!(search) {
            let mut start = 0usize;
            if let JsValue::BigInt(from) = inline_borrow!(from_index) {
                start = from.max(0) as usize;
            }
            if let Some(pos) = s[start..].find(search_str.as_str()) {
                return CodeResult::Return(Rc::new(RefCell::new(JsValue::BigInt((start + pos) as i64))));
            }
        }
        CodeResult::Return(Rc::new(RefCell::new(JsValue::BigInt(-1))))
    },
    includes, fn,
    |_, this, [search]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) &&
            let JsValue::String(search_str) = inline_borrow!(search) {
                JsValue::Boolean(s.contains(search_str.as_str()))
        }else{
            JsValue::Boolean(false)
        })))
    },
    startsWith, fn,
    |_, this, [search]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) &&
            let JsValue::String(search_str) = inline_borrow!(search) {
                JsValue::Boolean(s.starts_with(search_str.as_str()))

        } else {
            JsValue::Boolean(false)
        })))
    },
    endsWith, fn,
    |_, this, [search]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) &&
            let JsValue::String(search_str) = inline_borrow!(search) {
             JsValue::Boolean(s.ends_with(search_str.as_str()))
        } else {
            JsValue::Boolean(false)
        })))
    },
    toUpperCase, fn,
    |_, this, []| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) {
            JsValue::String(s.to_uppercase())
        } else {
            JsValue::String("".to_owned())
        })))
    },
    toLowerCase, fn,
    |_, this, []| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) {
            JsValue::String(s.to_lowercase())
        } else {
            JsValue::String("".to_owned())
        })))
    },
    trim, fn,
    |_, this, []| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) {
            JsValue::String(s.trim().to_owned())
        } else {
            JsValue::String("".to_owned())
        })))
    },
    split, fn,
    |env, this, [separator]| {
        let array = Prototype::find(env.mem, &stringify!(Array).into()).1.borrow().unwrap_proto("String.split for Array");
        if let JsValue::String(s) = inline_borrow!(this) {
            let parts: Vec<Rc<RefCell<JsValue>>> = match inline_borrow!(separator) {
                JsValue::String(sep) => {
                    if sep.is_empty() {
                        s.chars().map(|c| Rc::new(RefCell::new(JsValue::String(c.to_string())))).collect()
                    } else {
                        s.split(sep.as_str()).map(|p| Rc::new(RefCell::new(JsValue::String(p.to_owned())))).collect()
                    }
                },
                JsValue::Undefined => vec![Rc::new(RefCell::new(JsValue::String(s.clone())))],
                _ => vec![Rc::new(RefCell::new(JsValue::String(s.clone())))],
            };
            CodeResult::Return(new_array(array, parts, env.logger))
        } else {
            CodeResult::Return(new_array(array, vec![], env.logger))
        }
    },
    repeat, fn,
    |_, this, [count]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) && let JsValue::BigInt(n) = inline_borrow!(count) && n > 0 {
            JsValue::String(s.repeat(n as usize))
        } else {
            JsValue::String("".to_owned())
        })))
    },
    replace, fn,
    |_, this, [search, replacement]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) {
            if let (JsValue::String(search_str), JsValue::String(replace_str)) = (inline_borrow!(search), inline_borrow!(replacement)) {
                JsValue::String(s.replacen(search_str.as_str(), replace_str.as_str(), 1))
            } else {
                JsValue::String(s.clone())
            }
        } else {
            JsValue::String("".to_owned())
        })))
    },
    match_, fn_direct,
    |env, this, arguments| {
        let pattern = arguments
            .first()
            .cloned()
            .unwrap_or_else(|| Rc::new(RefCell::new(JsValue::Undefined)));
        crate::prebuild::regex::string_match(env, this, pattern)
    };
}

pub fn prebuild_string(env: Environment) {
    prebuild_string_class(env.clone());
    let string = Prototype::find(env.mem, &stringify!(String).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_string for String");
    let method = string
        .borrow_mut()
        .properties
        .remove(&"match_".into())
        .expect("String.match method");
    string
        .borrow_mut()
        .properties
        .insert("match".into(), method);
}
