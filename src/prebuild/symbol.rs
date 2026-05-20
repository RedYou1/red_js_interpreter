use crate::prebuild::prelude::*;

new_class!(
    prebuild_symbol,
    Symbol,
    Object,
    iterator, JsValue::Symbol(rand::random(), Box::new(JsValue::Undefined));
    constructor, fn, |_, _, [value]| {
        JsValue::Symbol(rand::random(), Box::new(value))
    };
);
