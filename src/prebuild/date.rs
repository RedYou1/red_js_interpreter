use std::time::{SystemTime, UNIX_EPOCH};

use crate::prebuild::prelude::*;

const DATE_VALUE: &str = "__date_value__";
const DAY_MS: f64 = 86_400_000.0;

new_class! {
    prebuild_date_class,
    Date,
    Object,;
    constructor, fn_direct,
    |_, this, arguments| {
        let value = date_value(&arguments);
        if let JsValue::Prototype(object) = inline_borrow!(this) {
            object.borrow_mut().properties.insert(
                DATE_VALUE.into(),
                Rc::new(RefCell::new(JsValue::Number(value))),
            );
        }
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)))
    },
    getYear, fn,
    |mem, this, []| {
        let Some(value) = date_value_of(this) else {
            return type_error(mem);
        };
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(if value.is_nan() {
            f64::NAN
        } else {
            civil_from_days((value / DAY_MS).floor() as i64).0 as f64 - 1900.0
        }))))
    },
    getFullYear, fn,
    |mem, this, []| {
        let Some(value) = date_value_of(this) else {
            return type_error(mem);
        };
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(if value.is_nan() {
            f64::NAN
        } else {
            civil_from_days((value / DAY_MS).floor() as i64).0 as f64
        }))))
    },
    getTime, fn,
    |mem, this, []| {
        let Some(value) = date_value_of(this) else {
            return type_error(mem);
        };
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(value))))
    },
    valueOf, fn,
    |mem, this, []| {
        let Some(value) = date_value_of(this) else {
            return type_error(mem);
        };
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(value))))
    },
    setTime, fn,
    |mem, this, [time]| {
        let Some(object) = date_object(this) else {
            return type_error(mem);
        };
        let time = match to_number(mem, time) {
            Ok(time) => time_clip(time),
            Err(error) => return error,
        };
        object.borrow_mut().properties.insert(
            DATE_VALUE.into(),
            Rc::new(RefCell::new(JsValue::Number(time))),
        );
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(time))))
    },
    setYear, fn,
    |mem, this, [year]| {
        let Some(object) = date_object(this) else {
            return type_error(mem);
        };
        let current = date_value_from_object(object.clone());
        let year = match to_number(mem, year) {
            Ok(year) => year,
            Err(error) => return error,
        };
        let value = if year.is_nan() {
            f64::NAN
        } else {
            let time = if current.is_nan() { 0.0 } else { current };
            let (_, month, day) = civil_from_days((time / DAY_MS).floor() as i64);
            let year = year.trunc();
            let year = if (0.0..=99.0).contains(&year) {
                year + 1900.0
            } else {
                year
            };
            let days = days_from_civil(year as i64, month, day);
            time_clip(days as f64 * DAY_MS + time.rem_euclid(DAY_MS))
        };
        object.borrow_mut().properties.insert(
            DATE_VALUE.into(),
            Rc::new(RefCell::new(JsValue::Number(value))),
        );
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Number(value))))
    };
}

pub fn prebuild_date(mem: Rc<RefCell<Prototype>>) {
    prebuild_date_class(mem.clone());
    let date = Prototype::find(mem.clone(), &stringify!(Date).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_date for Date");
    let object = Prototype::find(mem, &stringify!(Object).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_date for Object");
    let date_prototype = Prototype::new_child(object, None, []);
    let get_year = Prototype::find(date.clone(), &"getYear".into()).1;
    date_prototype
        .borrow_mut()
        .properties
        .insert("getYear".into(), get_year);
    for name in ["getFullYear", "getTime", "valueOf", "setTime", "setYear"] {
        let value = Prototype::find(date.clone(), &name.into()).1;
        date_prototype.borrow_mut().properties.insert(name.into(), value);
    }
    Prototype::find(date.clone(), &"setYear".into())
        .1
        .borrow()
        .unwrap_proto("prebuild_date setYear")
        .borrow_mut()
        .properties
        .insert("length".into(), Rc::new(RefCell::new(JsValue::BigInt(1))));
    date_prototype.borrow_mut().properties.insert(
        "constructor".into(),
        Rc::new(RefCell::new(JsValue::Prototype(date.clone()))),
    );
    date.borrow_mut().properties.insert(
        "prototype".into(),
        Rc::new(RefCell::new(JsValue::Prototype(date_prototype))),
    );
}

fn date_value(arguments: &[Rc<RefCell<JsValue>>]) -> f64 {
    if arguments.is_empty() {
        return SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(f64::NAN, |duration| duration.as_millis() as f64);
    }
    if arguments.len() == 1 {
        return match inline_borrow!(arguments[0].clone()) {
            JsValue::BigInt(value) => value as f64,
            JsValue::Number(value) => value,
            _ => f64::NAN,
        };
    }

    let Some(year) = number_value(&arguments[0]) else {
        return f64::NAN;
    };
    let Some(month) = number_value(&arguments[1]) else {
        return f64::NAN;
    };
    let date = arguments.get(2).and_then(number_value).unwrap_or(1.0);
    let hours = arguments.get(3).and_then(number_value).unwrap_or(0.0);
    let minutes = arguments.get(4).and_then(number_value).unwrap_or(0.0);
    let seconds = arguments.get(5).and_then(number_value).unwrap_or(0.0);
    let milliseconds = arguments.get(6).and_then(number_value).unwrap_or(0.0);
    if ![year, month, date, hours, minutes, seconds, milliseconds]
        .iter()
        .all(|value| value.is_finite())
    {
        return f64::NAN;
    }

    let year = year.trunc() as i64;
    let year = if (0..=99).contains(&year) {
        year + 1900
    } else {
        year
    };
    let month = month.trunc() as i64;
    let date = date.trunc() as i64;
    let hours = hours.trunc() as i64;
    let minutes = minutes.trunc() as i64;
    let seconds = seconds.trunc() as i64;
    let milliseconds = milliseconds.trunc() as i64;
    let month_year = year + month.div_euclid(12);
    let month = month.rem_euclid(12) + 1;
    let days = days_from_civil(month_year, month, date);
    let value = days as f64 * DAY_MS
        + hours as f64 * 3_600_000.0
        + minutes as f64 * 60_000.0
        + seconds as f64 * 1_000.0
        + milliseconds as f64;
    if value.abs() > 8_640_000_000_000_000.0 {
        f64::NAN
    } else {
        value
    }
}

fn date_object(this: Rc<RefCell<JsValue>>) -> Option<Rc<RefCell<Prototype>>> {
    let JsValue::Prototype(object) = inline_borrow!(this) else {
        return None;
    };
    if Prototype::opt_find(object.clone(), &DATE_VALUE.into()).is_some() {
        Some(object)
    } else {
        None
    }
}

fn date_value_of(this: Rc<RefCell<JsValue>>) -> Option<f64> {
    date_object(this).map(date_value_from_object)
}

fn date_value_from_object(object: Rc<RefCell<Prototype>>) -> f64 {
    match Prototype::find(object, &DATE_VALUE.into()).1.borrow().clone() {
        JsValue::Number(value) => value,
        _ => f64::NAN,
    }
}

fn number_value(value: &Rc<RefCell<JsValue>>) -> Option<f64> {
    match inline_borrow!(value.clone()) {
        JsValue::BigInt(value) => Some(value as f64),
        JsValue::Number(value) => Some(value),
        _ => None,
    }
}

fn to_number(
    mem: Rc<RefCell<Prototype>>,
    value: Rc<RefCell<JsValue>>,
) -> Result<f64, CodeResult> {
    match inline_borrow!(value.clone()) {
        JsValue::BigInt(value) => Ok(value as f64),
        JsValue::Number(value) => Ok(value),
        JsValue::String(value) => Ok(value.trim().parse().unwrap_or(f64::NAN)),
        JsValue::Boolean(value) => Ok(if value { 1.0 } else { 0.0 }),
        JsValue::Null => Ok(0.0),
        JsValue::Undefined => Ok(f64::NAN),
        JsValue::Symbol(_, _) => Err(type_error(mem)),
        JsValue::Prototype(object) => {
            let value_of = Prototype::find(object.clone(), &"valueOf".into()).1;
            let value_of = inline_borrow!(value_of);
            let JsValue::Prototype(value_of) = value_of else {
                return Ok(f64::NAN);
            };
            let result = crate::run_function_object(value_of, value.clone(), vec![]);
            let result = match result {
                CodeResult::Normal(value) | CodeResult::NormalMember(value, _, _) => value,
                CodeResult::Return(value) => value,
                error => return Err(error),
            };
            match inline_borrow!(result) {
                JsValue::BigInt(value) => Ok(value as f64),
                JsValue::Number(value) => Ok(value),
                JsValue::String(value) => Ok(value.trim().parse().unwrap_or(f64::NAN)),
                JsValue::Boolean(value) => Ok(if value { 1.0 } else { 0.0 }),
                JsValue::Null => Ok(0.0),
                JsValue::Undefined | JsValue::Prototype(_) | JsValue::Symbol(_, _)
                | JsValue::Function(_) | JsValue::Generator(_) => Ok(f64::NAN),
            }
        }
        JsValue::Function(_) | JsValue::Generator(_) => Ok(f64::NAN),
    }
}

fn time_clip(value: f64) -> f64 {
    if value.is_finite() && value.abs() <= 8_640_000_000_000_000.0 {
        value.trunc()
    } else {
        f64::NAN
    }
}

fn type_error(mem: Rc<RefCell<Prototype>>) -> CodeResult {
    let error = Prototype::find(mem, &stringify!(TypeError).into())
        .1
        .borrow()
        .unwrap_proto("Date.getYear for TypeError");
    let constructor = Rc::new(RefCell::new(JsValue::Prototype(error.clone())));
    CodeResult::Error(Rc::new(RefCell::new(JsValue::Prototype(
        Prototype::new_child(error, None, [("constructor".into(), constructor)]),
    ))))
}

fn days_from_civil(mut year: i64, month: i64, day: i64) -> i64 {
    year -= i64::from(month <= 2);
    let era = (if year >= 0 { year } else { year - 399 }) / 400;
    let year_of_era = year - era * 400;
    let month_index = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(mut days: i64) -> (i64, i64, i64) {
    days += 719_468;
    let era = (if days >= 0 { days } else { days - 146_096 }) / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = month_index + if month_index < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}
