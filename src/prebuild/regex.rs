use redgex::RedGex;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::prebuild::prelude::*;

const REGEX_SOURCE: &str = "__regex_source__";
const REGEX_FLAGS: &str = "__regex_flags__";
const REGEX_MATCHER: &str = "__regex_matcher__";

fn regex_error(env: Environment, name: &str) -> CodeResult {
    let error = Prototype::find(env.mem, &name.into())
        .1
        .borrow()
        .unwrap_proto("regex_error");
    let constructor = Rc::new(RefCell::new(JsValue::Prototype(error.clone())));
    CodeResult::Error(Rc::new(RefCell::new(JsValue::Prototype(
        Prototype::new_child(error, None, [("constructor".into(), constructor)]),
    ))))
}

fn checked_flags(flags: &str) -> Option<String> {
    let mut seen = String::new();
    for flag in flags.chars() {
        if !"dgimsuvy".contains(flag) || seen.contains(flag) {
            return None;
        }
        seen.push(flag);
    }
    Some(
        "dgimsuvy"
            .chars()
            .filter(|flag| seen.contains(*flag))
            .collect(),
    )
}

fn compile_matcher(source: &str) -> Result<Rc<RedGex>, ()> {
    let source = if source == "(?:)" { "" } else { source };
    catch_unwind(AssertUnwindSafe(|| Rc::new(RedGex::new(source)))).map_err(|_| ())
}

fn make_regex(
    env: Environment,
    source: String,
    flags: String,
) -> Result<Rc<RefCell<JsValue>>, CodeResult> {
    let flags = checked_flags(&flags).ok_or_else(|| regex_error(env.clone(), "SyntaxError"))?;
    let matcher = compile_matcher(&source).map_err(|_| regex_error(env.clone(), "SyntaxError"))?;
    let regex = Prototype::find(env.mem.clone(), &stringify!(RegExp).into())
        .1
        .borrow()
        .unwrap_proto("new_regex for Regex");
    let source_value = if source.is_empty() {
        "(?:)".to_owned()
    } else {
        source.clone()
    };
    let object = Prototype::new_child(
        regex,
        None,
        [
            (
                REGEX_SOURCE.into(),
                Rc::new(RefCell::new(JsValue::String(source))),
            ),
            (
                REGEX_FLAGS.into(),
                Rc::new(RefCell::new(JsValue::String(flags.clone()))),
            ),
            (
                REGEX_MATCHER.into(),
                Rc::new(RefCell::new(JsValue::RedGex(matcher))),
            ),
            (
                "source".into(),
                Rc::new(RefCell::new(JsValue::String(source_value))),
            ),
            (
                "flags".into(),
                Rc::new(RefCell::new(JsValue::String(flags.clone()))),
            ),
            (
                "global".into(),
                Rc::new(RefCell::new(JsValue::Boolean(flags.contains('g')))),
            ),
            (
                "ignoreCase".into(),
                Rc::new(RefCell::new(JsValue::Boolean(flags.contains('i')))),
            ),
            (
                "multiline".into(),
                Rc::new(RefCell::new(JsValue::Boolean(flags.contains('m')))),
            ),
            (
                "dotAll".into(),
                Rc::new(RefCell::new(JsValue::Boolean(flags.contains('s')))),
            ),
            (
                "unicode".into(),
                Rc::new(RefCell::new(JsValue::Boolean(flags.contains('u')))),
            ),
            (
                "sticky".into(),
                Rc::new(RefCell::new(JsValue::Boolean(flags.contains('y')))),
            ),
            (
                "lastIndex".into(),
                Rc::new(RefCell::new(JsValue::BigInt(0))),
            ),
        ],
    );
    Ok(Rc::new(RefCell::new(JsValue::Prototype(object))))
}

fn update_regex(
    object: &Rc<RefCell<Prototype>>,
    source: String,
    flags: String,
    matcher: Rc<RedGex>,
) {
    let source_value = if source.is_empty() {
        "(?:)".to_owned()
    } else {
        source.clone()
    };
    let mut object = object.borrow_mut();
    for (key, value) in [
        (REGEX_SOURCE, JsValue::String(source)),
        (REGEX_FLAGS, JsValue::String(flags.clone())),
        (REGEX_MATCHER, JsValue::RedGex(matcher)),
        ("source", JsValue::String(source_value)),
        ("flags", JsValue::String(flags.clone())),
        ("global", JsValue::Boolean(flags.contains('g'))),
        ("ignoreCase", JsValue::Boolean(flags.contains('i'))),
        ("multiline", JsValue::Boolean(flags.contains('m'))),
        ("dotAll", JsValue::Boolean(flags.contains('s'))),
        ("unicode", JsValue::Boolean(flags.contains('u'))),
        ("sticky", JsValue::Boolean(flags.contains('y'))),
        ("lastIndex", JsValue::BigInt(0)),
    ] {
        object
            .properties
            .insert(key.into(), Rc::new(RefCell::new(value)));
    }
}

fn to_string(env: Environment, value: Rc<RefCell<JsValue>>) -> Result<String, CodeResult> {
    match inline_borrow!(value.clone()) {
        JsValue::String(value) => Ok(value),
        JsValue::Symbol(_, _) => Err(regex_error(env, "TypeError")),
        JsValue::Prototype(object) => {
            let method = Prototype::find(object, &"toString".into()).1;
            let JsValue::Prototype(method) = inline_borrow!(method) else {
                return Ok(value.borrow().print());
            };
            let result = run_function_object(method, value, vec![], env.logger.clone());
            let result = match result {
                CodeResult::Return(result)
                | CodeResult::Normal(result)
                | CodeResult::NormalMember(result, _, _) => result,
                CodeResult::Error(error) => return Err(CodeResult::Error(error)),
                _ => return Ok("undefined".to_owned()),
            };
            to_string(env, result)
        }
        value => Ok(value.print()),
    }
}

fn compile_regex(
    env: Environment,
    this: Rc<RefCell<JsValue>>,
    arguments: Vec<Rc<RefCell<JsValue>>>,
) -> CodeResult {
    let Some((object, _, _)) = regex_parts(&this) else {
        return regex_error(env, "TypeError");
    };
    let pattern = arguments
        .first()
        .cloned()
        .unwrap_or_else(|| Rc::new(RefCell::new(JsValue::Undefined)));
    let pattern_regex = regex_parts(&pattern);
    let (source, flags) = if let Some((_, source, flags)) = pattern_regex {
        if arguments
            .get(1)
            .is_some_and(|flags| !matches!(inline_borrow!(flags), JsValue::Undefined))
        {
            return regex_error(env, "TypeError");
        }
        (source, flags)
    } else {
        let source = if matches!(inline_borrow!(pattern.clone()), JsValue::Undefined) {
            String::new()
        } else {
            match to_string(env.clone(), pattern) {
                Ok(source) => source,
                Err(error) => return error,
            }
        };
        let flags = match arguments.get(1) {
            None => String::new(),
            Some(flags) if matches!(inline_borrow!(flags), JsValue::Undefined) => String::new(),
            Some(flags) => match to_string(env.clone(), flags.clone()) {
                Ok(flags) => flags,
                Err(error) => return error,
            },
        };
        (source, flags)
    };
    let flags = match checked_flags(&flags) {
        Some(flags) => flags,
        None => return regex_error(env, "SyntaxError"),
    };
    let matcher = match compile_matcher(&source) {
        Ok(matcher) => matcher,
        Err(()) => return regex_error(env, "SyntaxError"),
    };
    update_regex(&object, source, flags, matcher);
    CodeResult::Return(this)
}

pub fn new_regex(env: Environment, source: String, flags: String) -> CodeResult {
    match make_regex(env.clone(), source, flags) {
        Ok(regex) => CodeResult::Return(regex),
        Err(error) => error,
    }
}

fn regex_parts(this: &Rc<RefCell<JsValue>>) -> Option<(Rc<RefCell<Prototype>>, String, String)> {
    let JsValue::Prototype(object) = inline_borrow!(this.clone()) else {
        return None;
    };
    let source = Prototype::opt_find(object.clone(), &REGEX_SOURCE.into())?.1;
    let flags = Prototype::opt_find(object.clone(), &REGEX_FLAGS.into())?.1;
    let JsValue::String(source) = inline_borrow!(source) else {
        return None;
    };
    let JsValue::String(flags) = inline_borrow!(flags) else {
        return None;
    };
    Some((object, source, flags))
}

fn string_value(value: &Rc<RefCell<JsValue>>) -> String {
    inline_borrow!(value.clone()).print()
}

fn regex_matcher(object: &Rc<RefCell<Prototype>>) -> Option<Rc<RedGex>> {
    let matcher = Prototype::opt_find(object.clone(), &REGEX_MATCHER.into())?.1;
    let JsValue::RedGex(matcher) = inline_borrow!(matcher) else {
        return None;
    };
    Some(matcher)
}

fn array_prototype(env: &Environment) -> Rc<RefCell<Prototype>> {
    Prototype::find(env.mem.clone(), &stringify!(Array).into())
        .1
        .borrow()
        .unwrap_proto("Regex for Array")
}

fn match_array(
    env: &Environment,
    captures: Vec<&str>,
    text: &str,
    index: usize,
) -> Rc<RefCell<JsValue>> {
    let values = captures
        .into_iter()
        .map(|capture| Rc::new(RefCell::new(JsValue::String(capture.to_owned()))))
        .collect();
    let array = new_array(array_prototype(env), values, env.logger.clone());
    let JsValue::Prototype(array) = inline_borrow!(array.clone()) else {
        unreachable!();
    };
    array.borrow_mut().properties.insert(
        "index".into(),
        Rc::new(RefCell::new(JsValue::BigInt(index as i64))),
    );
    array.borrow_mut().properties.insert(
        "input".into(),
        Rc::new(RefCell::new(JsValue::String(text.to_owned()))),
    );
    Rc::new(RefCell::new(JsValue::Prototype(array)))
}

fn set_last_index(object: &Rc<RefCell<Prototype>>, index: i64) {
    object.borrow_mut().properties.insert(
        "lastIndex".into(),
        Rc::new(RefCell::new(JsValue::BigInt(index))),
    );
}

new_class! {
    prebuild_regex_class,
    RegExp,
    Object,;
    constructor, fn_direct,
    |env, _, arguments| {
        let source = match arguments.first() {
            None => String::new(),
            Some(value) if matches!(inline_borrow!(value), JsValue::Undefined) => String::new(),
            Some(value) => match to_string(env.clone(), value.clone()) {
                Ok(source) => source,
                Err(error) => return error,
            },
        };
        let flags = match arguments.get(1) {
            None => String::new(),
            Some(value) if matches!(inline_borrow!(value), JsValue::Undefined) => String::new(),
            Some(value) => match to_string(env.clone(), value.clone()) {
                Ok(flags) => flags,
                Err(error) => return error,
            },
        };
        new_regex(env, source, flags)
    },
    compile, fn_direct,
    |env, this, arguments| compile_regex(env, this, arguments),
    test, fn,
    |_, this, [text]| {
        let Some((object, _source, flags)) = regex_parts(&this) else {
            return CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(false))));
        };
        let Some(matcher) = regex_matcher(&object) else {
            return CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(false))));
        };
        let input = string_value(&text);
        let start = if flags.contains('g') || flags.contains('y') {
            match inline_borrow!(Prototype::find(object.clone(), &"lastIndex".into()).1) {
                JsValue::BigInt(index) if index >= 0 => index as usize,
                _ => 0,
            }
        } else {
            0
        };
        let Some(search_text) = input.get(start..) else {
            set_last_index(&object, 0);
            return CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(false))));
        };
        let matched = matcher.first_match(search_text);
        if let Some(captures) = matched {
            let match_start = search_text.find(captures[0]).unwrap_or(0);
            if flags.contains('g') || flags.contains('y') {
                set_last_index(&object, (start + match_start + captures[0].len()) as i64);
            }
            CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(true))))
        } else {
            if flags.contains('g') || flags.contains('y') {
                set_last_index(&object, 0);
            }
            CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(false))))
        }
    },
    exec, fn,
    |env, this, [text]| {
        let Some((object, _source, flags)) = regex_parts(&this) else {
            return CodeResult::Return(Rc::new(RefCell::new(JsValue::Null)));
        };
        let Some(matcher) = regex_matcher(&object) else {
            return CodeResult::Return(Rc::new(RefCell::new(JsValue::Null)));
        };
        let input = string_value(&text);
        let start = if flags.contains('g') || flags.contains('y') {
            match inline_borrow!(Prototype::find(object.clone(), &"lastIndex".into()).1) {
                JsValue::BigInt(index) if index >= 0 => index as usize,
                _ => 0,
            }
        } else {
            0
        };
        let Some(search_text) = input.get(start..) else {
            set_last_index(&object, 0);
            return CodeResult::Return(Rc::new(RefCell::new(JsValue::Null)));
        };
        let Some(captures) = matcher.first_match(search_text) else {
            if flags.contains('g') || flags.contains('y') {
                set_last_index(&object, 0);
            }
            return CodeResult::Return(Rc::new(RefCell::new(JsValue::Null)));
        };
        let match_start = search_text.find(captures[0]).unwrap_or(0);
        if flags.contains('g') || flags.contains('y') {
            set_last_index(&object, (start + match_start + captures[0].len()) as i64);
        }
        CodeResult::Return(match_array(&env, captures, &input, start + match_start))
    },
    toString, fn,
    |_, this, []| {
        let Some((_, source, flags)) = regex_parts(&this) else {
            return CodeResult::Return(Rc::new(RefCell::new(JsValue::String("/ /".to_owned()))));
        };
        CodeResult::Return(Rc::new(RefCell::new(JsValue::String(format!("/{source}/{flags}")))))
    },
    allMatchs, fn,
    |env, this, [text]| {
        let Some((object, _, _)) = regex_parts(&this) else {
            return CodeResult::Return(new_array(array_prototype(&env), vec![], env.logger));
        };
        let Some(matcher) = regex_matcher(&object) else {
            return CodeResult::Return(new_array(array_prototype(&env), vec![], env.logger));
        };
        let input = string_value(&text);
        let matches = matcher
            .all_matchs(&input)
            .map(|captures| match_array(&env, captures, &input, 0))
            .collect();
        CodeResult::Return(new_array(array_prototype(&env), matches, env.logger))
    },
    allMatches, fn,
    |env, this, [text]| {
        let Some((object, _, _)) = regex_parts(&this) else {
            return CodeResult::Return(new_array(array_prototype(&env), vec![], env.logger));
        };
        let Some(matcher) = regex_matcher(&object) else {
            return CodeResult::Return(new_array(array_prototype(&env), vec![], env.logger));
        };
        let input = string_value(&text);
        let matches = matcher
            .all_matchs(&input)
            .map(|captures| match_array(&env, captures, &input, 0))
            .collect();
        CodeResult::Return(new_array(array_prototype(&env), matches, env.logger))
    };
}

pub fn prebuild_regex(env: Environment) {
    prebuild_regex_class(env.clone());
    let regex = Prototype::find(env.mem.clone(), &stringify!(RegExp).into())
        .1
        .borrow()
        .unwrap_proto("prebuild_regex for RegExp");
    regex.borrow_mut().properties.insert(
        "prototype".into(),
        Rc::new(RefCell::new(JsValue::Prototype(regex.clone()))),
    );
    let compile = Prototype::find(regex, &"compile".into())
        .1
        .borrow()
        .unwrap_proto("prebuild_regex for compile");
    compile
        .borrow_mut()
        .properties
        .insert("length".into(), Rc::new(RefCell::new(JsValue::BigInt(2))));
}
