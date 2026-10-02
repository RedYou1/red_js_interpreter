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
    LOG10E, JsValue::Number(std::f64::consts::LOG10_E),
    SQRT1_2, JsValue::Number(std::f64::consts::FRAC_1_SQRT_2),
    SQRT2, JsValue::Number(std::f64::consts::SQRT_2);
    abs, fn,
    |_, _, [num]| {
        CodeResult::Return(Rc::new(RefCell::new(match inline_borrow!(num) {
            JsValue::BigInt(n) => JsValue::BigInt(n.abs()),
            JsValue::Number(n) => JsValue::Number(n.abs()),
            _ => JsValue::BigInt(0)
        })))
    },
    acos, fn,
    |_, num, []| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(match inline_borrow!(num) {
            JsValue::BigInt(n) => (n as f64).acos(),
            JsValue::Number(n) => n.acos(),
            _ => f64::NAN,
        }))))
    },
    asin, fn,
    |_, num, []| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(match inline_borrow!(num) {
            JsValue::BigInt(n) => (n as f64).asin(),
            JsValue::Number(n) => n.asin(),
            _ => f64::NAN,
        }))))
    },
    atan, fn,
    |_, num, []| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(match inline_borrow!(num) {
            JsValue::BigInt(n) => (n as f64).atan(),
            JsValue::Number(n) => n.atan(),
            _ => f64::NAN,
        }))))
    },
    atan2, fn,
    |_, y, [x]| {
        let y = match inline_borrow!(y) {
            JsValue::BigInt(n) => n as f64,
            JsValue::Number(n) => n,
            _ => f64::NAN,
        };
        let x = match inline_borrow!(x) {
            JsValue::BigInt(n) => n as f64,
            JsValue::Number(n) => n,
            _ => f64::NAN,
        };
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(y.atan2(x)))))
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
    },
    log, fn,
    |_, num, []| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(match inline_borrow!(num) {
            JsValue::BigInt(n) => (n as f64).ln(),
            JsValue::Number(n) => n.ln(),
            _ => f64::NAN,
        }))))
    },
    random, fn,
    |_, _, []| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(0.5))))
    },
    sin, fn,
    |_, num, []| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(match inline_borrow!(num) {
            JsValue::BigInt(n) => (n as f64).sin(),
            JsValue::Number(n) => n.sin(),
            _ => f64::NAN,
        }))))
    },
    cos, fn,
    |_, num, []| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(match inline_borrow!(num) {
            JsValue::BigInt(n) => (n as f64).cos(),
            JsValue::Number(n) => n.cos(),
            _ => f64::NAN,
        }))))
    },
    exp, fn,
    |_, num, []| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(match inline_borrow!(num) {
            JsValue::BigInt(n) => (n as f64).exp(),
            JsValue::Number(n) => n.exp(),
            _ => f64::NAN,
        }))))
    },
    tan, fn,
    |_, num, []| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(match inline_borrow!(num) {
            JsValue::BigInt(n) => (n as f64).tan(),
            JsValue::Number(n) => n.tan(),
            _ => f64::NAN,
        }))))
    };
}
