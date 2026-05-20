use crate::prebuild::prelude::*;

pub fn new_array(array: Rc<RefCell<Prototype>>, content: Vec<JsValue>) -> JsValue {
    let len = content.len();
    let proto = Prototype::new_child(
        array,
        Some("Array instance"),
        content
            .into_iter()
            .enumerate()
            .map(|(i, elem)| (i.into(), elem)),
    );
    proto
        .borrow_mut()
        .properties
        .insert("length".into(), JsValue::BigInt(len as i64));
    JsValue::Prototype(proto)
}

new_class! {
    prebuild_array,
    Array,
    Iterator,;
    constructor, fn_direct,
    |mem, _, arguments: Vec<JsValue>| {
        if let [JsValue::BigInt(nlength)] = arguments[..] && (0..=(u32::MAX as i64)).contains(&nlength)
        {
            return new_array(
                    Prototype::find(mem.clone(), &"Array".into()).1.unwrap_proto(),
                    vec![const { JsValue::Undefined }; nlength as usize],
                );
        }
        new_array(Prototype::find(mem.clone(), &"Array".into()).1.unwrap_proto(), arguments)
    },
    from, fn,
    |mem, _, [items, mapFn, thisArg]| {
        let array = Prototype::find(mem.clone(), &"Array".into()).1.unwrap_proto();
        let JsValue::Symbol(_, iterator) =
            Prototype::find(mem.clone(), &"Symbol".into()).1.find(&"iterator".into()).1
        else {
            panic!("Array not found")
        };
        let iterator = items.find(&iterator).1.unwrap_proto();
        if let JsValue::Undefined = mapFn {
            new_array(
                array,
                run_generator_object(mem.clone(), iterator, JsValue::Undefined, vec![])
                    .collect(),
            )
        } else {
            new_array(
                array,
                run_generator_object(mem.clone(), iterator, JsValue::Undefined, vec![])
                    .map(|value| {
                        run_function_object(
                            mem.clone(),
                            mapFn.unwrap_proto(),
                            thisArg.clone(),
                            vec![value],
                        )
                    })
                    .collect(),
            )
        }
    },
    of, fn_direct,
    |mem, _, arguments: Vec<JsValue>| {
        let array = Prototype::find(mem, &"Array".into()).1.unwrap_proto();
        new_array(array, arguments)
    },
    at, fn,
    |_, this, [at]| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(mut at) = at else {panic!("Array.at at not BigInt")};
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else {panic!("Array.length not BigInt")};
        if at < 0 {
            at = length - at;
        }
        if !(0..length).contains(&at) {
            panic!("index {at} not in array of length {length}");
        }
        Prototype::find(this.clone(), &JsValue::BigInt(at)).1
    },
    push, fn_direct,
    |_, this, arguments: Vec<JsValue>| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else { panic!("Array.length not BigInt") };
        let mut len = length;
        for value in arguments {
            this.borrow_mut().properties.insert(JsValue::BigInt(len), value);
            len += 1;
        }
        this.borrow_mut().properties.insert("length".into(), JsValue::BigInt(len));
        JsValue::BigInt(len)
    },
    pop, fn_direct,
    |_, this, _arguments: Vec<JsValue>| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else { panic!("Array.length not BigInt") };
        if length == 0 {
            return JsValue::Undefined;
        }
        let idx = JsValue::BigInt(length - 1);
        let value = this.borrow_mut().properties.remove(&idx).unwrap_or(JsValue::Undefined);
        this.borrow_mut().properties.insert("length".into(), JsValue::BigInt(length - 1));
        value
    },
    map, fn,
    |mem, this, [callback, thisArg, _]| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else { panic!("Array.length not BigInt") };
        let mut result = Vec::with_capacity(length as usize);
        for i in 0..length {
            let value = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
            let mapped = run_function_object(
                mem.clone(),
                callback.unwrap_proto(),
                thisArg.clone(),
                vec![value, JsValue::BigInt(i), JsValue::Prototype(this.clone())],
            );
            result.push(mapped);
        }
        new_array(mem, result)
    },
    forEach, fn,
    |mem, this, [callback, thisArg]| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else { panic!("Array.length not BigInt") };
        for i in 0..length {
            let value = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
            run_function_object(
                mem.clone(),
                callback.unwrap_proto(),
                thisArg.clone(),
                vec![value, JsValue::BigInt(i), JsValue::Prototype(this.clone())],
            );
        }
        JsValue::Undefined
    },
    filter, fn,
    |mem, this, [callback, thisArg]| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else { panic!("Array.length not BigInt") };
        let mut result = Vec::new();
        for i in 0..length {
            let value = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
            let keep = run_function_object(
                mem.clone(),
                callback.unwrap_proto(),
                thisArg.clone(),
                vec![value.clone(), JsValue::BigInt(i), JsValue::Prototype(this.clone())],
            );
            if keep.is_truthy() {
                result.push(value);
            }
        }
        new_array(mem, result)
    },
    reduce, fn,
    |mem, this, [callback, initialValue, _]| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else { panic!("Array.length not BigInt") };

        let mut accumulator = initialValue.clone();
        let start_idx = if !matches!(initialValue, JsValue::Undefined) { 0 } else {
            if length == 0 { panic!("Reduce of empty array with no initial value"); }
            accumulator = Prototype::find(this.clone(), &JsValue::BigInt(0)).1;
            1
        };

        for i in start_idx..length {
            let value = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
            accumulator = run_function_object(
                mem.clone(),
                callback.unwrap_proto(),
                JsValue::Undefined,
                vec![accumulator, value, JsValue::BigInt(i), JsValue::Prototype(this.clone())],
            );
        }
        accumulator
    },
    find, fn,
    |mem, this, [callback, thisArg]| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else { panic!("Array.length not BigInt") };
        for i in 0..length {
            let value = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
            let found = run_function_object(
                mem.clone(),
                callback.unwrap_proto(),
                thisArg.clone(),
                vec![value.clone(), JsValue::BigInt(i), JsValue::Prototype(this.clone())],
            );
            if found.is_truthy() {
                return value;
            }
        }
        JsValue::Undefined
    },
    includes, fn,
    |_, this, [searchElement, fromIndex]| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else { panic!("Array.length not BigInt") };
        let mut start = 0i64;
        if !matches!(fromIndex, JsValue::Undefined) {
            if let JsValue::BigInt(from) = fromIndex {
                start = if from < 0 { (length + from).max(0) } else { from };
            }
        }
        for i in start..length {
            if Prototype::find(this.clone(), &JsValue::BigInt(i)).1 == searchElement {
                return JsValue::Boolean(true);
            }
        }
        JsValue::Boolean(false)
    },
    indexOf, fn,
    |_, this, [searchElement, fromIndex]| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else { panic!("Array.length not BigInt") };
        let mut start = 0i64;
        if !matches!(fromIndex, JsValue::Undefined) {
            if let JsValue::BigInt(from) = fromIndex {
                start = if from < 0 { (length + from).max(0) } else { from };
            }
        }
        for i in start..length {
            if Prototype::find(this.clone(), &JsValue::BigInt(i)).1 == searchElement {
                return JsValue::BigInt(i);
            }
        }
        JsValue::BigInt(-1)
    },
    slice, fn,
    |mem, this, [start, end]| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else { panic!("Array.length not BigInt") };
        let mut start_idx = 0i64;
        let mut end_idx = length;

        if let JsValue::BigInt(s) = start {
            start_idx = if s < 0 { (length + s).max(0) } else { s.min(length) };
        }
        if let JsValue::BigInt(e) = end {
            end_idx = if e < 0 { (length + e).max(0) } else { e.min(length) };
        }

        let mut result = Vec::new();
        for i in start_idx..end_idx {
            result.push(Prototype::find(this.clone(), &JsValue::BigInt(i)).1);
        }
        new_array(mem, result)
    },
    join, fn,
    |_, this, [separator]| {
        let this = this.unwrap_proto();
        let JsValue::BigInt(length) = Prototype::find(this.clone(), &"length".into()).1 else { panic!("Array.length not BigInt") };
        let sep = match separator {
            JsValue::String(s) => s.clone(),
            JsValue::Undefined => ",".to_owned(),
            other => panic!("not implemented"),
        };

        let strings: Vec<String> = (0..length)
            .map(|i| {
                let val = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
                match val {
                    JsValue::String(s) => s.clone(),
                    JsValue::Null | JsValue::Undefined => "".to_owned(),
                    other => panic!("not implemented"),
                }
            })
            .collect();
        JsValue::String(strings.join(&sep))
    };
    Symbol.iterator, fn_gen, Generator {
        params: vec![],
        excess: None,
        code: Rc::new(vec![
            Box::new(|proto, _, _| {
                proto
                    .borrow_mut()
                    .properties
                    .insert("i".into(), JsValue::BigInt(0));
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
    }
}
