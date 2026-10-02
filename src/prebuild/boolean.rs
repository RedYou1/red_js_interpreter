use crate::prebuild::prelude::*;

new_class! {
    prebuild_boolean,
    Boolean,
    Object,;
    constructor, fn,
    |_, _, [arg]| {
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Boolean(
            inline_borrow!(arg).is_truthy(),
        ))))
    };
}
