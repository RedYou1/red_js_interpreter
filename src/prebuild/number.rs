use crate::prebuild::prelude::*;

pub fn prebuild_numeric_parsers(environment: Environment) {
    let function = Prototype::find(environment.mem.clone(), &stringify!(Function).into())
        .1
        .borrow()
        .unwrap_proto("numeric parser Function prototype");
    let parse_integer = new_runnable(
        function.clone(),
        "parseInt",
        prebuild_runnable_direct(
            environment.clone(),
            Box::new(|_, _, arguments| {
                let Some(input) = arguments.first() else {
                    return CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(f64::NAN))));
                };
                let radix = arguments
                    .get(1)
                    .and_then(|radix| value_string(radix).parse::<i64>().ok())
                    .unwrap_or(0);
                let parsed = parse_integer_prefix(&value_string(input), radix);
                CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(parsed))))
            }),
        ),
    );
    let parse_float = new_runnable(
        function,
        "parseFloat",
        prebuild_runnable_direct(
            environment.clone(),
            Box::new(|_, _, arguments| {
                let parsed = arguments
                    .first()
                    .map(value_string)
                    .map_or(f64::NAN, |input| parse_float_prefix(&input));
                CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(parsed))))
            }),
        ),
    );
    environment
        .mem
        .borrow_mut()
        .properties
        .insert("parseInt".into(), parse_integer);
    environment
        .mem
        .borrow_mut()
        .properties
        .insert("parseFloat".into(), parse_float);
}

fn value_string(value: &Rc<RefCell<JsValue>>) -> String {
    match inline_borrow!(value.clone()) {
        JsValue::String(value) => value,
        value => value.print(),
    }
}

fn parse_integer_prefix(input: &str, requested_radix: i64) -> f64 {
    let input = input.trim_start();
    let (sign, input) = if let Some(input) = input.strip_prefix('-') {
        (-1.0, input)
    } else if let Some(input) = input.strip_prefix('+') {
        (1.0, input)
    } else {
        (1.0, input)
    };
    let radix = if requested_radix == 0 {
        if input.starts_with("0x") || input.starts_with("0X") {
            16
        } else {
            10
        }
    } else if (2..=36).contains(&requested_radix) {
        requested_radix as u32
    } else {
        return f64::NAN;
    };
    let input = if radix == 16 && (input.starts_with("0x") || input.starts_with("0X")) {
        &input[2..]
    } else {
        input
    };
    let mut parsed = 0.0;
    let mut found_digit = false;
    for character in input.chars() {
        let Some(digit) = character.to_digit(radix) else {
            break;
        };
        found_digit = true;
        parsed = parsed * f64::from(radix) + f64::from(digit);
    }
    if found_digit { sign * parsed } else { f64::NAN }
}

fn parse_float_prefix(input: &str) -> f64 {
    let input = input.trim_start();
    for (prefix, value) in [
        ("Infinity", f64::INFINITY),
        ("+Infinity", f64::INFINITY),
        ("-Infinity", f64::NEG_INFINITY),
    ] {
        if let Some(rest) = input.strip_prefix(prefix) {
            return if rest.is_empty()
                || rest.starts_with(|character: char| !character.is_alphanumeric())
            {
                value
            } else {
                f64::NAN
            };
        }
    }
    let bytes = input.as_bytes();
    let mut index = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let mut digits = 0;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        digits += 1;
        index += 1;
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            digits += 1;
            index += 1;
        }
    }
    if digits == 0 {
        return f64::NAN;
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        let exponent_start = index;
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let exponent_digits = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == exponent_digits {
            index = exponent_start;
        }
    }
    input[..index].parse().unwrap_or(f64::NAN)
}

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
