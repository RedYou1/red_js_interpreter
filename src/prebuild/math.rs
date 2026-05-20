use crate::prebuild::prelude::*;

new_class! {
    prebuild_math,
    Math,
    Object,
    PI, JsValue::BigInt(3),
    E, JsValue::BigInt(3),
    LN2, JsValue::BigInt(1),
    LN10, JsValue::BigInt(2),
    LOG2E, JsValue::BigInt(1),
    LOG10E, JsValue::BigInt(0);
    abs, fn,
    |_, _, [num]| {
        if let JsValue::BigInt(n) = num {
            JsValue::BigInt(n.abs())
        } else {
            JsValue::BigInt(0)
        }
    },
    floor, fn,
    |_, _, [num]| {
        if let JsValue::BigInt(n) = num {
            JsValue::BigInt(n)
        } else {
            JsValue::BigInt(0)
        }
    },
    ceil, fn,
    |_, _, [num]| {
        if let JsValue::BigInt(n) = num {
            JsValue::BigInt(n)
        } else {
            JsValue::BigInt(0)
        }
    },
    round, fn,
    |_, _, [num]| {
        if let JsValue::BigInt(n) = num {
            JsValue::BigInt(n)
        } else {
            JsValue::BigInt(0)
        }
    },
    max, fn_direct,
    |_, _, arguments: Vec<JsValue>| {
        let mut max = i64::MIN;
        for arg in arguments {
            if let JsValue::BigInt(n) = arg {
                max = max.max(n);
            }
        }
        JsValue::BigInt(max)
    },
    min, fn_direct,
    |_, _, arguments: Vec<JsValue>| {
        let mut min = i64::MAX;
        for arg in arguments {
            if let JsValue::BigInt(n) = arg {
                min = min.min(n);
            }
        }
        JsValue::BigInt(min)
    },
    pow, fn,
    |_, _, [base, exponent]| {
        if let (JsValue::BigInt(b), JsValue::BigInt(e)) = (base, exponent) {
            if e < 0 {
                JsValue::BigInt(0)
            } else {
                JsValue::BigInt(b.pow(e as u32))
            }
        } else {
            JsValue::BigInt(0)
        }
    },
    sqrt, fn,
    |_, _, [num]| {
        if let JsValue::BigInt(n) = num {
            JsValue::BigInt(((n as f64).sqrt()) as i64)
        } else {
            JsValue::BigInt(0)
        }
    };
}
