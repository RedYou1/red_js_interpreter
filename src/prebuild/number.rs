use crate::prebuild::prelude::*;

new_class! {
    prebuild_number,
    Number,
    Object,
    MAX_VALUE, JsValue::BigInt(i64::MAX),
    MIN_VALUE, JsValue::BigInt(i64::MIN),
    MAX_SAFE_INTEGER, JsValue::BigInt(9007199254740991),
    MIN_SAFE_INTEGER, JsValue::BigInt(-9007199254740991);
    constructor, fn,
    |_, _, [arg]| {
        Rc::new(RefCell::new(match inline_borrow!(arg) {
            JsValue::BigInt(n) => JsValue::BigInt(n),
            JsValue::Number(n) => JsValue::Number(n),
            JsValue::String(s) => {
                if let Ok(n) = s.parse::<i64>() {
                    JsValue::BigInt(n)
                } else {
                    JsValue::BigInt(0)
                }
            },
            JsValue::Boolean(b) => JsValue::BigInt(if b { 1 } else { 0 }),
            JsValue::Null | JsValue::Undefined => JsValue::BigInt(0),
            JsValue::Prototype(_) | JsValue::Symbol(_, _) | JsValue::Function(_) | JsValue::Generator(_) => JsValue::BigInt(0),
        }))
    },
    isNaN, fn,
    |_, _, [arg]| {
        Rc::new(RefCell::new(if let JsValue::BigInt(_) = inline_borrow!(arg) {
            JsValue::Boolean(false)
        } else {
            JsValue::Boolean(true)
        }))
    },
    isFinite, fn,
    |_, _, [arg]| {
        Rc::new(RefCell::new(if let JsValue::BigInt(_) = inline_borrow!(arg) {
            JsValue::Boolean(true)
        } else {
            JsValue::Boolean(false)
        }))
    },
    isInteger, fn,
    |_, _, [arg]| {
        Rc::new(RefCell::new(JsValue::Boolean(matches!(inline_borrow!(arg), JsValue::BigInt(_)))))
    },
    toFixed, fn,
    |_, this, [digits]| {
        Rc::new(RefCell::new(if let JsValue::BigInt(n) = inline_borrow!(this) {
            let d = if let JsValue::BigInt(d) = inline_borrow!(digits) { d as usize } else { 0 };
            if d == 0 {
                JsValue::String(format!("{}", n))
            } else {
                JsValue::String(format!("{:.prec$}", n, prec = d))
            }
        } else {
            JsValue::String("0".to_owned())
        }))
    },
    toString, fn,
    |_, this, [radix]| {
        Rc::new(RefCell::new(if let JsValue::BigInt(n) = inline_borrow!(this) {
            let base = if let JsValue::BigInt(r) = inline_borrow!(radix) {
                if (2..=36).contains(&r) { r as u32 } else { 10 }
            } else {
                10
            };

            if base == 10 {
                JsValue::String(format!("{}", n))
            } else {
                JsValue::String(radix_fmt(n, base).to_string())
            }
        } else {
            JsValue::String("0".to_owned())
        }))
    };
}

fn radix_fmt(mut n: i64, base: u32) -> String {
    if n == 0 {
        return "0".to_owned();
    }
    let is_negative = n < 0;
    n = n.abs();
    let digits = "0123456789abcdefghijklmnopqrstuvwxyz";
    let mut result = String::new();
    while n > 0 {
        result.insert(0, digits.chars().nth((n % base as i64) as usize).unwrap());
        n /= base as i64;
    }
    if is_negative {
        result.insert(0, '-');
    }
    result
}
