use crate::prebuild::prelude::*;

new_class!(
    prebuild_iterator,
    Iterator,
    Object,;
    map, fn_gen, Generator {
        params: vec![],
        excess: None,
        code: Rc::new(vec![
            Box::new(|proto, _, _| {
                proto
                    .borrow_mut()
                    .properties
                    .insert("i".into(), JsValue::BigInt(0));
                run_generator_object(
                    proto.clone(),
                    Prototype::find(proto.clone(), &"this".into()).1.unwrap_proto(),
                    JsValue::Undefined,
                    vec![],
                );
                (JsValue::Undefined, None)
            }),
            Box::new(|proto, go_next, _| {
                let JsValue::BigInt(arr_i) = Prototype::find(proto.clone(), &"i".into()).1 else {
                    panic!("?")
                };
                let JsValue::BigInt(arr_len) = Prototype::find(proto.clone(), &"length".into()).1 else {
                    panic!("?")
                };
                if arr_i >= arr_len {
                    (JsValue::Undefined, None)
                } else {
                    let obj = Prototype::find(proto.clone(), &arr_i.into()).1;
                    proto
                        .borrow_mut()
                        .properties
                        .insert("i".into(), JsValue::BigInt(arr_i + 1));
                    *go_next = false;
                    (JsValue::Undefined, Some(obj))
                }
            }),
        ]),
    };
);
