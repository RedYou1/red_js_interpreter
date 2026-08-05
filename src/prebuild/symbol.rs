use crate::prebuild::prelude::*;

new_class!(
    prebuild_symbol,
    Symbol,
    Object,
    iterator, JsValue::Symbol(rand::random(), Box::new(JsValue::String("Symbol.iterator".to_owned()))),
    hasInstance, JsValue::Symbol(rand::random(), Box::new(JsValue::String("Symbol.hasInstance".to_owned())));
    constructor, fn, |_, _, [value]| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Symbol(rand::random(), Box::new(inline_borrow!(value))))))
    };
);
// Symbol.hasInstance, fn, |_, _, [instance]| {
//     CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(true))))
// }
