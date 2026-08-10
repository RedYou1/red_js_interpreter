use crate::prebuild::prelude::*;

new_class! {
    prebuild_math,
    Math,
    Object,
    PI, JsValue::Number(std::f64::consts::PI),
    E, JsValue::Number(std::f64::consts::E),
    LN2, JsValue::Number(std::f64::consts::LN_2),
    LN10, JsValue::Number(std::f64::consts::LN_10),
    LOG2E, JsValue::Number(std::f64::consts::LOG2_E),
    LOG10E, JsValue::Number(std::f64::consts::LOG10_E);
    abs, fn,
    |_, _, [num]| {
        CodeResult::Return(Rc::new(RefCell::new(match inline_borrow!(num) {
            JsValue::BigInt(n) => JsValue::BigInt(n.abs()),
            JsValue::Number(n) => JsValue::Number(n.abs()),
            _ => JsValue::BigInt(0)
        })))
    },
    floor, fn,
    |_, _, [num]| {
        CodeResult::Return(Rc::new(RefCell::new(match inline_borrow!(num) {
            JsValue::BigInt(n) => JsValue::BigInt(n),
            JsValue::Number(n) => {let t = n.floor(); if i64::MIN as f64 <= t && t <= i64::MAX as f64 && t.floor() == t { JsValue::BigInt(t as i64) } else {JsValue::Number(t)}},
            _ => JsValue::BigInt(0)
        })))
    },
    ceil, fn,
    |_, _, [num]| {
        CodeResult::Return(Rc::new(RefCell::new(match inline_borrow!(num) {
            JsValue::BigInt(n) => JsValue::BigInt(n),
            JsValue::Number(n) => {let t = n.ceil(); if i64::MIN as f64 <= t && t <= i64::MAX as f64 && t.floor() == t { JsValue::BigInt(t as i64) } else {JsValue::Number(t)}},
            _ => JsValue::BigInt(0)
        })))
    },
    round, fn,
    |_, _, [num]| {
        CodeResult::Return(Rc::new(RefCell::new(match inline_borrow!(num) {
            JsValue::BigInt(n) => JsValue::BigInt(n),
            JsValue::Number(n) => {let t = n.round(); if i64::MIN as f64 <= t && t <= i64::MAX as f64 && t.floor() == t { JsValue::BigInt(t as i64) } else {JsValue::Number(t)}},
            _ => JsValue::BigInt(0)
        })))
    },
    max, fn_direct,
    |_, _, arguments| {
        let mut max = f64::MIN;
        for arg in arguments {
            max = match inline_borrow!(arg) {
                JsValue::BigInt(n) => max.max(n as f64),
                JsValue::Number(n) => max.max(n),
                _ => {max}
            };
        }
        CodeResult::Return(Rc::new(RefCell::new(if i64::MIN as f64 <= max && max <= i64::MAX as f64 && max.floor() == max { JsValue::BigInt(max as i64) } else {JsValue::Number(max)})))
    },
    min, fn_direct,
    |_, _, arguments| {
        let mut min = f64::MIN;
        for arg in arguments {
            min = match inline_borrow!(arg) {
                JsValue::BigInt(n) => min.min(n as f64),
                JsValue::Number(n) => min.min(n),
                _ => {min}
            };
        }
        CodeResult::Return(Rc::new(RefCell::new(if i64::MIN as f64 <= min && min <= i64::MAX as f64 && min.floor() == min { JsValue::BigInt(min as i64) } else {JsValue::Number(min)})))
    },
    pow, fn,
    |_, _, [base, exponent]| {
        let t = match (inline_borrow!(base), inline_borrow!(exponent)) {
            (JsValue::BigInt(b), JsValue::BigInt(e)) if e >= 0 => (b as f64).powf(e as f64),
            (JsValue::BigInt(b), JsValue::Number(e)) if e >= 0.0 => (b as f64).powf(e),
            (JsValue::Number(b), JsValue::BigInt(e)) if e >= 0 => b.powf(e as f64),
            (JsValue::Number(b), JsValue::Number(e)) if e >= 0.0 => b.powf(e),
            _ => 0.0
        };
        CodeResult::Return(Rc::new(RefCell::new(
            if i64::MIN as f64 <= t && t <= i64::MAX as f64 && t.floor() == t { JsValue::BigInt(t as i64) } else {JsValue::Number(t)}
        )))
    },
    sqrt, fn,
    |_, _, [num]| {
        CodeResult::Return(Rc::new(RefCell::new(match inline_borrow!(num) {
            JsValue::BigInt(n) => JsValue::Number((n as f64).sqrt()),
            JsValue::Number(n) => JsValue::Number(n.sqrt()),
            _ => JsValue::BigInt(0)
        })))
    };
}
