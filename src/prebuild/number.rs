use crate::prebuild::prelude::*;
use crate::{new_runnable, prebuild_runnable_direct};

new_class! {
    prebuild_number_class,
    Number,
    Object,
    MAX_VALUE, JsValue::BigInt(i64::MAX),
    MIN_VALUE, JsValue::BigInt(i64::MIN),
    MAX_SAFE_INTEGER, JsValue::BigInt(9007199254740991),
    MIN_SAFE_INTEGER, JsValue::BigInt(-9007199254740991);
    constructor, fn,
    |_, _, [arg]| {
        CodeResult::Return(Rc::new(RefCell::new(match inline_borrow!(arg) {
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
            JsValue::Prototype(_) | JsValue::Symbol(_, _) | JsValue::Function(_) | JsValue::Generator(_) | JsValue::RedGex(_) => JsValue::BigInt(0),
        })))
    },
    isNaN, fn,
    |_, _, [arg]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::BigInt(_) = inline_borrow!(arg) {
            JsValue::Boolean(false)
        } else {
            JsValue::Boolean(true)
        })))
    },
    isFinite, fn,
    |_, _, [arg]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::BigInt(_) = inline_borrow!(arg) {
            JsValue::Boolean(true)
        } else {
            JsValue::Boolean(false)
        })))
    },
    isInteger, fn,
    |_, _, [arg]| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(matches!(inline_borrow!(arg), JsValue::BigInt(_))))))
    },
    toFixed, fn,
    |_, this, [digits]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::BigInt(n) = inline_borrow!(this) {
            let d = if let JsValue::BigInt(d) = inline_borrow!(digits) { d as usize } else { 0 };
            if d == 0 {
                JsValue::String(format!("{}", n))
            } else {
                JsValue::String(format!("{:.prec$}", n, prec = d))
            }
        } else {
            JsValue::String("0".to_owned())
        })))
    },
    toString, fn,
    |_, this, [radix]| {
        CodeResult::Return(Rc::new(RefCell::new(if let JsValue::BigInt(n) = inline_borrow!(this) {
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
        })))
    };
}

pub fn prebuild_number(env: Environment) {
    prebuild_number_class(env.clone());
    let function = Prototype::find(env.mem.clone(), &stringify!(Function).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_number for Function");
    for (name, parse) in [
        ("parseInt", parse_int as fn(&[Rc<RefCell<JsValue>>]) -> f64),
        (
            "parseFloat",
            parse_float as fn(&[Rc<RefCell<JsValue>>]) -> f64,
        ),
    ] {
        let runnable = prebuild_runnable_direct(
            env.clone(),
            Box::new(move |_, _, arguments| {
                CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(parse(&arguments)))))
            }),
        );
        env.mem
            .borrow_mut()
            .properties
            .insert(name.into(), new_runnable(function.clone(), name, runnable));
    }
}

fn parse_int(arguments: &[Rc<RefCell<JsValue>>]) -> f64 {
    let value = arguments
        .first()
        .map(|value| inline_borrow!(value.clone()).print())
        .unwrap_or_else(|| "undefined".to_owned());
    let radix = arguments
        .get(1)
        .map_or(0, |value| match inline_borrow!(value.clone()) {
            JsValue::BigInt(value) => value as i32,
            JsValue::Number(value) if value.is_finite() => value.trunc() as i32,
            JsValue::Boolean(value) => i32::from(value),
            JsValue::Null => 0,
            JsValue::String(value) => value.trim().parse::<f64>().unwrap_or(f64::NAN) as i32,
            _ => i32::MAX,
        });
    let value = value.trim_start();
    let (negative, value) = match value.strip_prefix('-') {
        Some(value) => (true, value),
        None => match value.strip_prefix('+') {
            Some(value) => (false, value),
            None => (false, value),
        },
    };
    let (radix, value) = if radix == 0 {
        if value.starts_with("0x") || value.starts_with("0X") {
            (16, &value[2..])
        } else {
            (10, value)
        }
    } else if radix == 16 && (value.starts_with("0x") || value.starts_with("0X")) {
        (radix, &value[2..])
    } else {
        (radix, value)
    };
    if !(2..=36).contains(&radix) {
        return f64::NAN;
    }
    let mut result = 0.0;
    let mut digits = 0;
    for character in value.chars() {
        let Some(digit) = character.to_digit(radix as u32) else {
            break;
        };
        result = result * radix as f64 + f64::from(digit);
        digits += 1;
    }
    if digits == 0 {
        f64::NAN
    } else if negative {
        -result
    } else {
        result
    }
}

fn parse_float(arguments: &[Rc<RefCell<JsValue>>]) -> f64 {
    let value = arguments
        .first()
        .map(|value| inline_borrow!(value.clone()).print())
        .unwrap_or_else(|| "undefined".to_owned());
    let value = value.trim_start();
    let sign_end = usize::from(value.starts_with(['+', '-']));
    let unsigned = &value[sign_end..];
    for (prefix, result) in [
        ("Infinity", f64::INFINITY),
        ("-Infinity", f64::NEG_INFINITY),
        ("+Infinity", f64::INFINITY),
    ] {
        if value.starts_with(prefix) {
            return result;
        }
    }
    let bytes = unsigned.as_bytes();
    let mut index = 0;
    let mut digits = 0;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
        digits += 1;
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return f64::NAN;
    }
    let exponent_start = index;
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let exponent_digits = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if exponent_digits == index {
            index = exponent_start;
        }
    }
    value[..sign_end + index].parse().unwrap_or(f64::NAN)
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
