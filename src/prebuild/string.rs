use crate::prebuild::prelude::*;

new_class! {
    prebuild_string,
    String,
    Object,;
    constructor, fn,
    |env, _, [arg]| {
        let arg = match inline_borrow!(arg) {
            JsValue::Symbol(_, ref t) =>  *t.clone(),
            v => v
        };
        match &arg {
            JsValue::Prototype(proto) => run_function_object(Prototype::find(proto.clone(), &"toString".into()).1.borrow().unwrap_proto("String.constructor for toString"), Rc::new(RefCell::new(arg.clone())), vec![], env.logger),
            JsValue::String(s) => CodeResult::Return(Rc::new(RefCell::new(JsValue::String(s.clone())))),
            JsValue::Null | JsValue::Undefined => CodeResult::Return(Rc::new(RefCell::new(JsValue::String("".to_owned())))),
            JsValue::BigInt(o) => CodeResult::Return(Rc::new(RefCell::new(JsValue::String(format!("{}", *o))))),
            JsValue::Number(o) => CodeResult::Return(Rc::new(RefCell::new(JsValue::String(format!("{}", *o))))),
            JsValue::Boolean(o) => CodeResult::Return(Rc::new(RefCell::new(JsValue::String(format!("{}", *o))))),
            JsValue::Symbol(_, _) | JsValue::Function(_) | JsValue::Generator(_) => panic!("not implemented"),
        }
    },
    length, fn,
    |_, this, []| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::String(s) = inline_borrow!(this) {
            JsValue::BigInt(s.len() as i64)
        } else {
            JsValue::BigInt(0)
        })))
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
        if let JsValue::String(s) = inline_borrow!(this)
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
    };
}
