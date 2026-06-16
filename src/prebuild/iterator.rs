use crate::prebuild::prelude::*;

new_class!{
    prebuild_iterator,
    Iterator,
    Object,;;
    map, fn_gen,
    |proto, _| {
        proto
            .borrow_mut()
            .properties
            .insert("i".into(), Rc::new(RefCell::new(JsValue::BigInt(0))));
        run_generator_object(
            Prototype::find(proto.clone(), &"this".into()).1.borrow().unwrap_proto("Iterator.map for this"),
            Rc::new(RefCell::new(JsValue::Undefined)),
            vec![],
        );
        CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
    };
    |proto, _| {
        let JsValue::BigInt(arr_i) = inline_borrow!(Prototype::find(proto.clone(), &"i".into()).1) else {
            panic!("?")
        };
        let JsValue::BigInt(arr_len) = inline_borrow!(Prototype::find(proto.clone(), &"length".into()).1) else {
            panic!("?")
        };
        if arr_i >= arr_len {
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
        } else {
            let obj = Prototype::find(proto.clone(), &arr_i.into()).1;
            proto
                .borrow_mut()
                .properties
                .insert("i".into(), Rc::new(RefCell::new(JsValue::BigInt(arr_i + 1))));
            CodeResult::Yield(obj)
        }
    }
}
