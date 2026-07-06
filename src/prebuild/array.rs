use crate::{CodeResult, LogLevel, logln, prebuild::prelude::*};

pub fn new_array(
    array: Rc<RefCell<Prototype>>,
    content: Vec<Rc<RefCell<JsValue>>>,
) -> Rc<RefCell<JsValue>> {
    let len = content.len();
    logln(LogLevel::Trace, &format!("Array::new_array length={}", len));
    let proto = Prototype::new_child(
        array,
        None,
        content
            .into_iter()
            .enumerate()
            .map(|(i, elem)| (i.into(), elem)),
    );
    proto.borrow_mut().properties.insert(
        "length".into(),
        Rc::new(RefCell::new(JsValue::BigInt(len as i64))),
    );
    Rc::new(RefCell::new(JsValue::Prototype(proto)))
}

new_class! {
    prebuild_array,
    Array,
    Iterator,;
    constructor, fn_direct,
    |mem, _, arguments| {
        let array = Prototype::find(mem.clone(), &stringify!(Array).into()).1.borrow().unwrap_proto("Array.constructor for Array");
        if let [nlength] = &arguments[..] && let JsValue::BigInt(nlength) = inline_borrow!(nlength) && (0..=(u32::MAX as i64)).contains(&nlength)
        {
            new_array(
                array,
                (0..nlength).map(|_| Rc::new(RefCell::new(JsValue::Undefined))).collect(),
            )
        } else {
            new_array(array, arguments)
        }
    },
    from, fn,
    |mem, _, [items, map_fn, this_arg]| {
        let array = Prototype::find(mem.clone(), &stringify!(Array).into()).1.borrow().unwrap_proto("Array.from for Array");
        let JsValue::Symbol(_, iterator) =
            inline_borrow!(Prototype::find(mem.clone(), &stringify!(Symbol).into()).1.borrow().find(&"iterator".into(), "Array.from get Symbol.iterator").1)
        else {
            panic!("Array.from get Symbol.iterator")
        };
        let iterator = items.borrow().find(&iterator, "Array.from get iterator of items").1.borrow().unwrap_proto("Array.from for items's iterator");
        if let JsValue::Undefined = inline_borrow!(map_fn.clone()) {
            new_array(
                array,
                run_generator_object(iterator, Rc::new(RefCell::new(JsValue::Undefined)), vec![])
                    .collect(),
            )
        } else {
            new_array(
                array,
                run_generator_object(iterator, Rc::new(RefCell::new(JsValue::Undefined)), vec![])
                    .map(|value| {
                        run_function_object(
                            map_fn.clone().borrow().unwrap_proto("Array.from for map_fn"),
                            this_arg.clone(),
                            vec![value.clone()],
                        )
                    })
                    .collect(),
            )
        }
    },
    of, fn_direct,
    |mem, _, arguments| {
        let array = Prototype::find(mem, &stringify!(Array).into()).1.borrow().unwrap_proto("Array.of for Array");
        new_array(array, arguments)
    },
    at, fn,
    |_, this, [at]| {
        let this = this.borrow().unwrap_proto("Array.at for this");
        let JsValue::BigInt(mut at) = inline_borrow!(at) else {panic!("Array.at at not BigInt")};
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else {panic!("Array.length not BigInt")};
        if at < 0 {
            at = length - at;
        }
        if !(0..length).contains(&at) {
            panic!("index {at} not in array of length {length}");
        }
        Prototype::find(this.clone(), &JsValue::BigInt(at)).1
    },
    push, fn_direct,
    |_, this, arguments| {
        let this = this.borrow().unwrap_proto("Array.push for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };
        let mut len = length;
        for value in arguments {
            this.borrow_mut().properties.insert(JsValue::BigInt(len), value);
            len += 1;
        }
        this.borrow_mut().properties.insert("length".into(), Rc::new(RefCell::new(JsValue::BigInt(len))));
        Rc::new(RefCell::new(JsValue::BigInt(len)))
    },
    pop, fn_direct,
    |_, this, _arguments| {
        let this = this.borrow().unwrap_proto("Array.pop for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };
        if length == 0 {
            return Rc::new(RefCell::new(JsValue::Undefined));
        }
        let idx = JsValue::BigInt(length - 1);
        let value = this.borrow_mut().properties.remove(&idx).unwrap_or(Rc::new(RefCell::new(JsValue::Undefined)));
        this.borrow_mut().properties.insert("length".into(), Rc::new(RefCell::new(JsValue::BigInt(length - 1))));
        value
    },
    map, fn,
    |mem, this, [callback, this_arg, _]| {
        let this = this.borrow().unwrap_proto("Array.map for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };
        let mut result = Vec::with_capacity(length as usize);
        for i in 0..length {
            let value = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
            let mapped = run_function_object(
                callback.borrow().unwrap_proto("Array.map for callback"),
                this_arg.clone(),
                vec![value, Rc::new(RefCell::new(JsValue::BigInt(i))), Rc::new(RefCell::new(JsValue::Prototype(this.clone())))],
            );
            result.push(mapped);
        }
        new_array(mem, result)
    },
    forEach, fn,
    |_, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.forEach for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };
        for i in 0..length {
            let value = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
            run_function_object(
                callback.borrow().unwrap_proto("Array.forEach for callback"),
                this_arg.clone(),
                vec![value, Rc::new(RefCell::new(JsValue::BigInt(i))), Rc::new(RefCell::new(JsValue::Prototype(this.clone())))],
            );
        }
        Rc::new(RefCell::new(JsValue::Undefined))
    },
    filter, fn,
    |mem, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.filter for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };
        let mut result = Vec::new();
        for i in 0..length {
            let value = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
            let keep = run_function_object(
                callback.borrow().unwrap_proto("Array.filter for callback"),
                this_arg.clone(),
                vec![value.clone(), Rc::new(RefCell::new(JsValue::BigInt(i))), Rc::new(RefCell::new(JsValue::Prototype(this.clone())))],
            );
            if keep.borrow().is_truthy() {
                result.push(value);
            }
        }
        new_array(mem, result)
    },
    reduce, fn,
    |_, this, [callback, initial_value, _]| {
        let this = this.borrow().unwrap_proto("Array.reduce for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };

        let mut accumulator = initial_value.clone();
        let start_idx = if !matches!(inline_borrow!(initial_value), JsValue::Undefined) { 0 } else {
            if length == 0 { panic!("Reduce of empty array with no initial value"); }
            accumulator = Prototype::find(this.clone(), &JsValue::BigInt(0)).1;
            1
        };

        for i in start_idx..length {
            let value = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
            accumulator = run_function_object(
                callback.borrow().unwrap_proto("Array.reduce for callback"),
                Rc::new(RefCell::new(JsValue::Undefined)),
                vec![accumulator, value, Rc::new(RefCell::new(JsValue::BigInt(i))), Rc::new(RefCell::new(JsValue::Prototype(this.clone())))],
            );
        }
        accumulator
    },
    find, fn,
    |_, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.find for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };
        for i in 0..length {
            let value = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
            let found = run_function_object(
                callback.borrow().unwrap_proto("Array.find for callback"),
                this_arg.clone(),
                vec![value.clone(), Rc::new(RefCell::new(JsValue::BigInt(i))), Rc::new(RefCell::new(JsValue::Prototype(this.clone())))],
            );
            if found.borrow().is_truthy() {
                return value;
            }
        }
        Rc::new(RefCell::new(JsValue::Undefined))
    },
    includes, fn,
    |_, this, [search_element, from_index]| {
        let this = this.borrow().unwrap_proto("Array.includes for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };
        let mut start = 0i64;
        if let JsValue::BigInt(from) = inline_borrow!(from_index) {
            start = if from < 0 { (length + from).max(0) } else { from };
        }
        for i in start..length {
            if Prototype::find(this.clone(), &JsValue::BigInt(i)).1 == search_element {
                return Rc::new(RefCell::new(JsValue::Boolean(true)));
            }
        }
        Rc::new(RefCell::new(JsValue::Boolean(false)))
    },
    indexOf, fn,
    |_, this, [search_element, from_index]| {
        let this = this.borrow().unwrap_proto("Array.indexOf for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };
        let mut start = 0i64;
        if let JsValue::BigInt(from) = inline_borrow!(from_index) {
            start = if from < 0 { (length + from).max(0) } else { from };
        }
        for i in start..length {
            if Prototype::find(this.clone(), &JsValue::BigInt(i)).1 == search_element {
                return Rc::new(RefCell::new(JsValue::BigInt(i)));
            }
        }
        Rc::new(RefCell::new(JsValue::BigInt(-1)))
    },
    slice, fn,
    |mem, this, [start, end]| {
        let this = this.borrow().unwrap_proto("Array.slice for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };
        let mut start_idx = 0i64;
        let mut end_idx = length;

        if let JsValue::BigInt(s) = inline_borrow!(start) {
            start_idx = if s < 0 { (length + s).max(0) } else { s.min(length) };
        }
        if let JsValue::BigInt(e) = inline_borrow!(end) {
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
        let this = this.borrow().unwrap_proto("Array.join for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };
        let sep = match inline_borrow!(separator) {
            JsValue::String(s) => s.clone(),
            JsValue::Undefined => ",".to_owned(),
            _ => panic!("not implemented"),
        };

        let strings: Vec<String> = (0..length)
            .map(|i| {
                let val = Prototype::find(this.clone(), &JsValue::BigInt(i)).1;
                match inline_borrow!(val) {
                    JsValue::String(s) => s.clone(),
                    JsValue::Null | JsValue::Undefined => "".to_owned(),
                    _ => panic!("not implemented"),
                }
            })
            .collect();
        Rc::new(RefCell::new(JsValue::String(strings.join(&sep))))
    };
    Symbol.iterator, fn_gen,
    |proto, _| {
        proto.borrow_mut()
            .properties
            .insert("i".into(), Rc::new(RefCell::new(JsValue::BigInt(0))));
        CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
    };
    |proto, _| {
        let JsValue::BigInt(i) = inline_borrow!(Prototype::find(proto.clone(), &"i".into()).1) else {
            panic!("?")
        };
        let this = proto
            .borrow()
            .properties[&"this".into()]
            .borrow()
            .unwrap_proto("Array.iterator this not found");
        let JsValue::BigInt(arr_len) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else {
            panic!("?")
        };
        if i >= arr_len {
            CodeResult::YieldBreak
        } else {
            let obj = Prototype::find(this.clone(), &i.into()).1;
            proto
                .borrow_mut()
                .properties
                .insert("i".into(), Rc::new(RefCell::new(JsValue::BigInt(i + 1))));
            CodeResult::Yield(obj)
        }
    };
    |proto, ind| {
        let JsValue::BigInt(i) = inline_borrow!(Prototype::find(proto.clone(), &"i".into()).1) else {
            panic!("?")
        };
        let this = proto
            .borrow()
            .properties[&"this".into()]
            .borrow()
            .unwrap_proto("Array.iterator this not found");
        let JsValue::BigInt(arr_len) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else {
            panic!("?")
        };
        if i < arr_len {
            ind.move_iamount(-1);
            ind.set_retry();
        }
        CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
    }
}
