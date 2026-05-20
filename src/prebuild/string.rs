use crate::prebuild::prelude::*;

new_class! {
    prebuild_string,
    String,
    Object,;
    constructor, fn,
    |mem, _, [arg]| {
        let arg = if let JsValue::Symbol(_, ref t) = arg {t} else {&arg};
        match arg {
            JsValue::Prototype(proto) => run_function_object(mem, Prototype::find(proto.clone(), &"toString".into()).1.unwrap_proto(), arg.clone(), vec![]),
            JsValue::String(s) => JsValue::String(s.clone()),
            JsValue::Null | JsValue::Undefined => JsValue::String("".to_owned()),
            JsValue::BigInt(o) => JsValue::String(format!("{}", *o)),
            JsValue::Number(o) => JsValue::String(format!("{}", *o)),
            JsValue::Boolean(o) => JsValue::String(format!("{}", *o)),
            JsValue::Symbol(_, _) | JsValue::Function(_) | JsValue::Generator(_) => panic!("not implemented"),
        }
    },
    length, fn,
    |_, this, []| {
        if let JsValue::String(s) = this {
            JsValue::BigInt(s.len() as i64)
        } else {
            JsValue::BigInt(0)
        }
    },
    charAt, fn,
    |_, this, [index]| {
        if let JsValue::String(s) = this && let JsValue::BigInt(idx) = index && idx >= 0 && (idx as usize) < s.len() {
            JsValue::String(s.chars().nth(idx as usize).unwrap().to_string())
        } else {
            JsValue::String("".to_owned())
        }
    },
    charCodeAt, fn,
    |_, this, [index]| {
        if let JsValue::String(s) = this && let JsValue::BigInt(idx) = index && idx >= 0 && (idx as usize) < s.len() {
            JsValue::BigInt(s.chars().nth(idx as usize).unwrap() as i64)
        } else {
            JsValue::BigInt(-1)
        }
    },
    substring, fn,
    |_, this, [start, end]| {
        if let JsValue::String(s) = this {
            let len = s.len() as i64;
            let mut start_idx = 0usize;
            let mut end_idx = len as usize;

            if let JsValue::BigInt(st) = start {
                start_idx = ((st.max(0)).min(len)) as usize;
            }
            if let JsValue::BigInt(en) = end {
                end_idx = ((en.max(0)).min(len)) as usize;
            }

            if start_idx > end_idx {
                std::mem::swap(&mut start_idx, &mut end_idx);
            }

            JsValue::String(s.chars().skip(start_idx).take(end_idx - start_idx).collect())
        } else {
            JsValue::String("".to_owned())
        }
    },
    slice, fn,
    |_, this, [start, end]| {
        if let JsValue::String(s) = this {
            let len = s.len() as i64;
            let mut start_idx = 0i64;
            let mut end_idx = len;

            if let JsValue::BigInt(st) = start {
                start_idx = if st < 0 { (len + st).max(0) } else { st.min(len) };
            }
            if let JsValue::BigInt(en) = end {
                end_idx = if en < 0 { (len + en).max(0) } else { en.min(len) };
            }

            if start_idx >= end_idx || start_idx < 0 {
                return JsValue::String("".to_owned());
            }

            JsValue::String(s.chars().skip(start_idx as usize).take((end_idx - start_idx) as usize).collect())
        } else {
            JsValue::String("".to_owned())
        }
    },
    indexOf, fn,
    |_, this, [search, from_index]| {
        if let JsValue::String(s) = this {
            if let JsValue::String(search_str) = search {
                let mut start = 0usize;
                if let JsValue::BigInt(from) = from_index {
                    start = from.max(0) as usize;
                }
                if let Some(pos) = s[start..].find(search_str.as_str()) {
                    return JsValue::BigInt((start + pos) as i64);
                }
            }
            JsValue::BigInt(-1)
        } else {
            JsValue::BigInt(-1)
        }
    },
    includes, fn,
    |_, this, [search]| {
        if let JsValue::String(s) = this {
            if let JsValue::String(search_str) = search {
                return JsValue::Boolean(s.contains(search_str.as_str()));
            }
            JsValue::Boolean(false)
        } else {
            JsValue::Boolean(false)
        }
    },
    startsWith, fn,
    |_, this, [search]| {
        if let JsValue::String(s) = this {
            if let JsValue::String(search_str) = search {
                return JsValue::Boolean(s.starts_with(search_str.as_str()));
            }
            JsValue::Boolean(false)
        } else {
            JsValue::Boolean(false)
        }
    },
    endsWith, fn,
    |_, this, [search]| {
        if let JsValue::String(s) = this {
            if let JsValue::String(search_str) = search {
                return JsValue::Boolean(s.ends_with(search_str.as_str()));
            }
            JsValue::Boolean(false)
        } else {
            JsValue::Boolean(false)
        }
    },
    toUpperCase, fn,
    |_, this, []| {
        if let JsValue::String(s) = this {
            JsValue::String(s.to_uppercase())
        } else {
            JsValue::String("".to_owned())
        }
    },
    toLowerCase, fn,
    |_, this, []| {
        if let JsValue::String(s) = this {
            JsValue::String(s.to_lowercase())
        } else {
            JsValue::String("".to_owned())
        }
    },
    trim, fn,
    |_, this, []| {
        if let JsValue::String(s) = this {
            JsValue::String(s.trim().to_owned())
        } else {
            JsValue::String("".to_owned())
        }
    },
    split, fn,
    |mem, this, [separator]| {
        if let JsValue::String(s) = this {
            let parts: Vec<JsValue> = match separator {
                JsValue::String(sep) => {
                    if sep.is_empty() {
                        s.chars().map(|c| JsValue::String(c.to_string())).collect()
                    } else {
                        s.split(sep.as_str()).map(|p| JsValue::String(p.to_owned())).collect()
                    }
                },
                JsValue::Undefined => vec![JsValue::String(s.clone())],
                _ => vec![JsValue::String(s.clone())],
            };
            let array = Prototype::find(mem, &"Array".into()).1.unwrap_proto();
            new_array(array, parts)
        } else {
            let array = Prototype::find(mem, &"Array".into()).1.unwrap_proto();
            new_array(array, vec![])
        }
    },
    repeat, fn,
    |_, this, [count]| {
        if let JsValue::String(s) = this && let JsValue::BigInt(n) = count && n > 0 {
            JsValue::String(s.repeat(n as usize))
        } else {
            JsValue::String("".to_owned())
        }
    },
    replace, fn,
    |_, this, [search, replacement]| {
        if let JsValue::String(s) = this {
            if let (JsValue::String(search_str), JsValue::String(replace_str)) = (search, replacement) {
                JsValue::String(s.replacen(search_str.as_str(), replace_str.as_str(), 1))
            } else {
                JsValue::String(s.clone())
            }
        } else {
            JsValue::String("".to_owned())
        }
    };
}
