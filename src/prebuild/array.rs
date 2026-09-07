use crate::{CodeResult, LogLevel, Logger, handle_error, prebuild::prelude::*};

fn value(value: JsValue) -> Rc<RefCell<JsValue>> {
    Rc::new(RefCell::new(value))
}

fn same_value(left: &JsValue, right: &JsValue) -> bool {
    match (left, right) {
        (JsValue::Prototype(left), JsValue::Prototype(right)) => Rc::ptr_eq(left, right),
        _ => left == right,
    }
}

fn integer(value: &Rc<RefCell<JsValue>>) -> i64 {
    match inline_borrow!(value) {
        JsValue::BigInt(value) => value,
        JsValue::Number(value) if value.is_finite() => value.trunc() as i64,
        _ => 0,
    }
}

fn string_value(value: &Rc<RefCell<JsValue>>) -> String {
    match inline_borrow!(value) {
        JsValue::Undefined => "undefined".to_owned(),
        JsValue::Null => "null".to_owned(),
        JsValue::String(value) => value,
        JsValue::BigInt(value) => value.to_string(),
        JsValue::Number(value) => value.to_string(),
        JsValue::Boolean(value) => value.to_string(),
        value => value.print(),
    }
}

fn array_length(array: &Rc<RefCell<Prototype>>) -> i64 {
    integer(&Prototype::find(array.clone(), &"length".into()).1)
}

fn array_prototype(env: &Environment) -> Rc<RefCell<Prototype>> {
    Prototype::find(env.mem.clone(), &stringify!(Array).into())
        .1
        .borrow()
        .unwrap_proto("Array prototype")
}

fn array_element(array: &Rc<RefCell<Prototype>>, index: i64) -> Rc<RefCell<JsValue>> {
    Prototype::find(array.clone(), &JsValue::BigInt(index)).1
}

fn callback_result(
    env: &Environment,
    callback: &Rc<RefCell<JsValue>>,
    this_arg: Rc<RefCell<JsValue>>,
    value: Rc<RefCell<JsValue>>,
    index: i64,
    array: Rc<RefCell<Prototype>>,
) -> CodeResult {
    run_function_object(
        callback.borrow().unwrap_proto("Array callback"),
        this_arg,
        vec![
            value,
            Rc::new(RefCell::new(JsValue::BigInt(index))),
            Rc::new(RefCell::new(JsValue::Prototype(array))),
        ],
        env.logger.clone(),
    )
}

fn flatten_value(item: Rc<RefCell<JsValue>>, depth: i64, output: &mut Vec<Rc<RefCell<JsValue>>>) {
    if depth > 0
        && let JsValue::Prototype(array) = inline_borrow!(item.clone())
        && Prototype::opt_find(array.clone(), &"length".into()).is_some()
    {
        let length = array_length(&array);
        for index in 0..length {
            flatten_value(array_element(&array, index), depth - 1, output);
        }
    } else {
        output.push(item);
    }
}

pub fn new_array(
    array: Rc<RefCell<Prototype>>,
    content: Vec<Rc<RefCell<JsValue>>>,
    logger: Rc<RefCell<dyn Logger>>,
) -> Rc<RefCell<JsValue>> {
    let len = content.len();
    logger.borrow_mut().logln(LogLevel::Trace, &|| {
        format!("Array::new_array length={}", len)
    });
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

pub fn new_array_with_length(
    array: Rc<RefCell<Prototype>>,
    length: i64,
    logger: Rc<RefCell<dyn Logger>>,
) -> Rc<RefCell<JsValue>> {
    logger.borrow_mut().logln(LogLevel::Trace, &|| {
        format!("Array::new_array_with_length length={}", length)
    });
    let proto = Prototype::new_child(array, None, []);
    proto.borrow_mut().properties.insert(
        "length".into(),
        Rc::new(RefCell::new(JsValue::BigInt(length))),
    );
    Rc::new(RefCell::new(JsValue::Prototype(proto)))
}

new_class! {
    prebuild_array,
    Array,
    Iterator,;
    constructor, fn_direct,
    |env, _, arguments| {
        let array = Prototype::find(env.mem.clone(), &stringify!(Array).into()).1.borrow().unwrap_proto("Array.constructor for Array");
        if let [nlength] = &arguments[..] && let JsValue::BigInt(nlength) = inline_borrow!(nlength) && (0..=(u32::MAX as i64)).contains(&nlength)
        {
            CodeResult::Return(new_array_with_length(array, nlength, env.logger))
        } else {
            CodeResult::Return(new_array(array, arguments, env.logger))
        }
    },
    from, fn,
    |env, _, [items, map_fn, this_arg]| {
        let array = Prototype::find(env.mem.clone(), &stringify!(Array).into()).1.borrow().unwrap_proto("Array.from for Array");
        let JsValue::Symbol(_, iterator) =
            inline_borrow!(Prototype::find(env.mem.clone(), &stringify!(Symbol).into()).1.borrow().find(&"iterator".into(), "Array.from get Symbol.iterator").1)
        else {
            panic!("Array.from get Symbol.iterator")
        };
        let iterator = items.borrow().find(&iterator, "Array.from get iterator of items").1.borrow().unwrap_proto("Array.from for items's iterator");
        if let JsValue::Undefined = inline_borrow!(map_fn.clone()) {
            let content = run_generator_object(iterator, Rc::new(RefCell::new(JsValue::Undefined)), vec![], env.logger.clone())
                    .map(|f| if let CodeResult::Error(err) = f {Err(err)} else {Ok(f.unwrap_normal())})
                    .collect::<Result<Vec<Rc<_>>, Rc<_>>>();
            match content {
                Ok(content) => CodeResult::Return(new_array(array,content, env.logger)),
                Err(content) => CodeResult::Error(content)
            }
        } else {
            let content = run_generator_object(iterator, Rc::new(RefCell::new(JsValue::Undefined)), vec![], env.logger.clone())
                    .map(|value| {
                        match value {
                            CodeResult::Return(value) => run_function_object(
                                map_fn.clone().borrow().unwrap_proto("Array.from for map_fn"),
                                this_arg.clone(),
                                vec![value.clone()],
                                env.logger.clone(),
                            ),
                            CodeResult::Error(_) => value,
                            _ => panic!("array.from coderesult not handled"),
                        }
                    })
                    .map(|f| if let CodeResult::Error(err) = f {Err(err)} else {Ok(f.unwrap_normal())})
                    .collect::<Result<Vec<Rc<_>>, Rc<_>>>();
            match content {
                Ok(content) => CodeResult::Return(new_array(array,content, env.logger)),
                Err(content) => CodeResult::Error(content)
            }
        }
    },
    of, fn_direct,
    |env, _, arguments| {
        let array = Prototype::find(env.mem, &stringify!(Array).into()).1.borrow().unwrap_proto("Array.of for Array");
        CodeResult::Return(new_array(array, arguments, env.logger))
    },
    at, fn,
    |_, this, [at]| {
        let this = this.borrow().unwrap_proto("Array.at for this");
        let mut at = integer(&at);
        let length = array_length(&this);
        if at < 0 {
            at = length + at;
        }
        if !(0..length).contains(&at) {
            return CodeResult::Return(value(JsValue::Undefined));
        }
        CodeResult::Return(array_element(&this, at))
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
        CodeResult::Return(Rc::new(RefCell::new(JsValue::BigInt(len))))
    },
    pop, fn_direct,
    |_, this, _arguments| {
        let this = this.borrow().unwrap_proto("Array.pop for this");
        let JsValue::BigInt(length) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else { panic!("Array.length not BigInt") };
        if length == 0 {
            return CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)));
        }
        let idx = JsValue::BigInt(length - 1);
        let value = this.borrow_mut().properties.remove(&idx).unwrap_or(Rc::new(RefCell::new(JsValue::Undefined)));
        this.borrow_mut().properties.insert("length".into(), Rc::new(RefCell::new(JsValue::BigInt(length - 1))));
        CodeResult::Return(value)
    },
    map, fn,
    |env, this, [callback, this_arg, _]| {
        let this = this.borrow().unwrap_proto("Array.map for this");
        let length = array_length(&this);
        let mut result = Vec::with_capacity(length as usize);
        for i in 0..length {
            result.push(handle_error!(callback_result(
                &env,
                &callback,
                this_arg.clone(),
                array_element(&this, i),
                i,
                this.clone(),
            )));
        }
        CodeResult::Return(new_array(array_prototype(&env), result, env.logger))
    },
    forEach, fn,
    |env, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.forEach for this");
        let length = array_length(&this);
        for i in 0..length {
            handle_error!(callback_result(
                &env,
                &callback,
                this_arg.clone(),
                array_element(&this, i),
                i,
                this.clone(),
            ));
        }
        CodeResult::Return(value(JsValue::Undefined))
    },
    filter, fn,
    |env, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.filter for this");
        let length = array_length(&this);
        let mut result = Vec::new();
        for i in 0..length {
            let element = array_element(&this, i);
            if handle_error!(callback_result(
                &env,
                &callback,
                this_arg.clone(),
                element.clone(),
                i,
                this.clone(),
            ))
            .borrow()
            .is_truthy()
            {
                result.push(element);
            }
        }
        CodeResult::Return(new_array(array_prototype(&env), result, env.logger))
    },
    reduce, fn,
    |env, this, [callback, initial_value, _]| {
        let this = this.borrow().unwrap_proto("Array.reduce for this");
        let length = array_length(&this);

        let mut accumulator = initial_value.clone();
        let start_idx = if !matches!(inline_borrow!(initial_value), JsValue::Undefined) { 0 } else {
            if length == 0 { return CodeResult::Error(value(JsValue::Undefined)); }
            accumulator = array_element(&this, 0);
            1
        };

        for i in start_idx..length {
            let element = array_element(&this, i);
            accumulator = handle_error!(run_function_object(
                callback.borrow().unwrap_proto("Array.reduce for callback"),
                value(JsValue::Undefined),
                vec![
                    accumulator,
                    element,
                    value(JsValue::BigInt(i)),
                    value(JsValue::Prototype(this.clone())),
                ],
                env.logger.clone(),
            ));
        }
        CodeResult::Return(accumulator)
    },
    find, fn,
    |env, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.find for this");
        let length = array_length(&this);
        for i in 0..length {
            let element = array_element(&this, i);
            if handle_error!(callback_result(
                &env,
                &callback,
                this_arg.clone(),
                element.clone(),
                i,
                this.clone(),
            ))
            .borrow()
            .is_truthy()
            {
                return CodeResult::Return(element);
            }
        }
        CodeResult::Return(value(JsValue::Undefined))
    },
    includes, fn,
    |_, this, [search_element, from_index]| {
        let this = this.borrow().unwrap_proto("Array.includes for this");
        let length = array_length(&this);
        let from = integer(&from_index);
        let start = if from < 0 { (length + from).max(0) } else { from };
        for i in start..length {
            let current = array_element(&this, i);
            let same_value = if let (JsValue::Number(left), JsValue::Number(right)) =
                (inline_borrow!(current.clone()), inline_borrow!(search_element.clone()))
            {
                left == right || (left.is_nan() && right.is_nan())
            } else {
                same_value(
                    &inline_borrow!(current.clone()),
                    &inline_borrow!(search_element.clone()),
                )
            };
            if same_value {
                return CodeResult::Return(value(JsValue::Boolean(true)));
            }
        }
        CodeResult::Return(value(JsValue::Boolean(false)))
    },
    indexOf, fn,
    |_, this, [search_element, from_index]| {
        let this = this.borrow().unwrap_proto("Array.indexOf for this");
        let length = array_length(&this);
        let from = integer(&from_index);
        let start = if from < 0 { (length + from).max(0) } else { from };
        for i in start..length {
            if same_value(
                &inline_borrow!(array_element(&this, i)),
                &inline_borrow!(search_element.clone()),
            ) {
                return CodeResult::Return(value(JsValue::BigInt(i)));
            }
        }
        CodeResult::Return(value(JsValue::BigInt(-1)))
    },
    slice, fn,
    |env, this, [start, end]| {
        let this = this.borrow().unwrap_proto("Array.slice for this");
        let length = array_length(&this);
        let mut start_idx = 0i64;
        let mut end_idx = length;

        let s = integer(&start);
        if !matches!(inline_borrow!(start), JsValue::Undefined) {
            start_idx = if s < 0 { (length + s).max(0) } else { s.min(length) };
        }
        let e = integer(&end);
        if !matches!(inline_borrow!(end), JsValue::Undefined) {
            end_idx = if e < 0 { (length + e).max(0) } else { e.min(length) };
        }

        let mut result = Vec::new();
        for i in start_idx..end_idx {
            result.push(Prototype::find(this.clone(), &JsValue::BigInt(i)).1);
        }
        CodeResult::Return(new_array(array_prototype(&env), result, env.logger))
    },
    concat, fn_direct,
    |env, this, arguments| {
        let this = this.borrow().unwrap_proto("Array.concat for this");
        let array = Prototype::find(env.mem.clone(), &stringify!(Array).into())
            .1
            .borrow()
            .unwrap_proto("Array.concat for Array");
        let mut result = Vec::new();
        for item in std::iter::once(Rc::new(RefCell::new(JsValue::Prototype(this.clone()))))
            .chain(arguments)
        {
            if let JsValue::Prototype(item) = inline_borrow!(item.clone())
                && Prototype::opt_find(item.clone(), &"length".into()).is_some()
            {
                for index in 0..array_length(&item) {
                    result.push(array_element(&item, index));
                }
            } else {
                result.push(item);
            }
        }
        CodeResult::Return(new_array(array, result, env.logger))
    },
    copyWithin, fn,
    |_, this, [target, start, end]| {
        let this = this.borrow().unwrap_proto("Array.copyWithin for this");
        let length = array_length(&this);
        let normalize = |argument: &Rc<RefCell<JsValue>>, default: i64| {
            if matches!(inline_borrow!(argument), JsValue::Undefined) {
                default
            } else {
                let index = integer(argument);
                if index < 0 {
                    (length + index).max(0)
                } else {
                    index.min(length)
                }
            }
        };
        let target = normalize(&target, 0);
        let start = normalize(&start, 0);
        let end = normalize(&end, length);
        let copied = (start..end)
            .map(|index| array_element(&this, index))
            .collect::<Vec<_>>();
        for (offset, item) in copied.into_iter().enumerate() {
            let index = target + offset as i64;
            if index >= length {
                break;
            }
            this.borrow_mut().properties.insert(JsValue::BigInt(index), item);
        }
        CodeResult::Return(value(JsValue::Prototype(this)))
    },
    every, fn,
    |env, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.every for this");
        let length = array_length(&this);
        for index in 0..length {
            if !handle_error!(callback_result(
                &env,
                &callback,
                this_arg.clone(),
                array_element(&this, index),
                index,
                this.clone(),
            ))
            .borrow()
            .is_truthy()
            {
                return CodeResult::Return(value(JsValue::Boolean(false)));
            }
        }
        CodeResult::Return(value(JsValue::Boolean(true)))
    },
    fill, fn,
    |_, this, [fill_value, start, end]| {
        let this = this.borrow().unwrap_proto("Array.fill for this");
        let length = array_length(&this);
        let normalize = |argument: &Rc<RefCell<JsValue>>, default: i64| {
            if matches!(inline_borrow!(argument), JsValue::Undefined) {
                default
            } else {
                let index = integer(argument);
                if index < 0 {
                    (length + index).max(0)
                } else {
                    index.min(length)
                }
            }
        };
        let start = normalize(&start, 0);
        let end = normalize(&end, length);
        for index in start..end {
            this.borrow_mut()
                .properties
                .insert(JsValue::BigInt(index), fill_value.clone());
        }
        CodeResult::Return(value(JsValue::Prototype(this)))
    },
    findIndex, fn,
    |env, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.findIndex for this");
        for index in 0..array_length(&this) {
            if handle_error!(callback_result(
                &env,
                &callback,
                this_arg.clone(),
                array_element(&this, index),
                index,
                this.clone(),
            ))
            .borrow()
            .is_truthy()
            {
                return CodeResult::Return(value(JsValue::BigInt(index)));
            }
        }
        CodeResult::Return(value(JsValue::BigInt(-1)))
    },
    findLast, fn,
    |env, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.findLast for this");
        for index in (0..array_length(&this)).rev() {
            let element = array_element(&this, index);
            if handle_error!(callback_result(
                &env,
                &callback,
                this_arg.clone(),
                element.clone(),
                index,
                this.clone(),
            ))
            .borrow()
            .is_truthy()
            {
                return CodeResult::Return(value(element.borrow().clone()));
            }
        }
        CodeResult::Return(value(JsValue::Undefined))
    },
    findLastIndex, fn,
    |env, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.findLastIndex for this");
        for index in (0..array_length(&this)).rev() {
            if handle_error!(callback_result(
                &env,
                &callback,
                this_arg.clone(),
                array_element(&this, index),
                index,
                this.clone(),
            ))
            .borrow()
            .is_truthy()
            {
                return CodeResult::Return(value(JsValue::BigInt(index)));
            }
        }
        CodeResult::Return(value(JsValue::BigInt(-1)))
    },
    flat, fn,
    |env, this, [depth]| {
        let this = this.borrow().unwrap_proto("Array.flat for this");
        let depth = if matches!(inline_borrow!(depth.clone()), JsValue::Undefined) {
            1
        } else {
            integer(&depth).max(0)
        };
        let mut result = Vec::new();
        for index in 0..array_length(&this) {
            flatten_value(array_element(&this, index), depth, &mut result);
        }
        let array = Prototype::find(env.mem, &stringify!(Array).into())
            .1
            .borrow()
            .unwrap_proto("Array.flat for Array");
        CodeResult::Return(new_array(array, result, env.logger))
    },
    flatMap, fn,
    |env, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.flatMap for this");
        let mut result = Vec::new();
        for index in 0..array_length(&this) {
            let mapped = handle_error!(callback_result(
                &env,
                &callback,
                this_arg.clone(),
                array_element(&this, index),
                index,
                this.clone(),
            ));
            flatten_value(mapped, 1, &mut result);
        }
        let array = Prototype::find(env.mem, &stringify!(Array).into())
            .1
            .borrow()
            .unwrap_proto("Array.flatMap for Array");
        CodeResult::Return(new_array(array, result, env.logger))
    },
    lastIndexOf, fn,
    |_, this, [search_element, from_index]| {
        let this = this.borrow().unwrap_proto("Array.lastIndexOf for this");
        let length = array_length(&this);
        let from = if matches!(inline_borrow!(from_index.clone()), JsValue::Undefined) {
            length - 1
        } else {
            integer(&from_index)
        };
        let start = if from < 0 { length + from } else { from.min(length - 1) };
        for index in (0..=start).rev() {
            if same_value(
                &inline_borrow!(array_element(&this, index)),
                &inline_borrow!(search_element.clone()),
            ) {
                return CodeResult::Return(value(JsValue::BigInt(index)));
            }
        }
        CodeResult::Return(value(JsValue::BigInt(-1)))
    },
    reduceRight, fn,
    |env, this, [callback, initial_value, _]| {
        let this = this.borrow().unwrap_proto("Array.reduceRight for this");
        let length = array_length(&this);
        let mut index = length - 1;
        let mut accumulator = initial_value.clone();
        if matches!(inline_borrow!(initial_value), JsValue::Undefined) {
            if length == 0 {
                return CodeResult::Error(value(JsValue::Undefined));
            }
            accumulator = array_element(&this, index);
            index -= 1;
        }
        while index >= 0 {
            accumulator = handle_error!(run_function_object(
                callback.borrow().unwrap_proto("Array.reduceRight for callback"),
                value(JsValue::Undefined),
                vec![
                    accumulator,
                    array_element(&this, index),
                    value(JsValue::BigInt(index)),
                    value(JsValue::Prototype(this.clone())),
                ],
                env.logger.clone(),
            ));
            index -= 1;
        }
        CodeResult::Return(accumulator)
    },
    reverse, fn,
    |_, this, []| {
        let this = this.borrow().unwrap_proto("Array.reverse for this");
        let length = array_length(&this);
        for index in 0..(length / 2) {
            let other = length - index - 1;
            let left = array_element(&this, index);
            let right = array_element(&this, other);
            this.borrow_mut().properties.insert(JsValue::BigInt(index), right);
            this.borrow_mut().properties.insert(JsValue::BigInt(other), left);
        }
        CodeResult::Return(value(JsValue::Prototype(this)))
    },
    shift, fn_direct,
    |_, this, _| {
        let this = this.borrow().unwrap_proto("Array.shift for this");
        let length = array_length(&this);
        if length == 0 {
            return CodeResult::Return(value(JsValue::Undefined));
        }
        let first = array_element(&this, 0);
        for index in 1..length {
            let item = array_element(&this, index);
            this.borrow_mut()
                .properties
                .insert(JsValue::BigInt(index - 1), item);
        }
        this.borrow_mut().properties.remove(&JsValue::BigInt(length - 1));
        this.borrow_mut()
            .properties
            .insert("length".into(), value(JsValue::BigInt(length - 1)));
        CodeResult::Return(first)
    },
    some, fn,
    |env, this, [callback, this_arg]| {
        let this = this.borrow().unwrap_proto("Array.some for this");
        for index in 0..array_length(&this) {
            if handle_error!(callback_result(
                &env,
                &callback,
                this_arg.clone(),
                array_element(&this, index),
                index,
                this.clone(),
            ))
            .borrow()
            .is_truthy()
            {
                return CodeResult::Return(value(JsValue::Boolean(true)));
            }
        }
        CodeResult::Return(value(JsValue::Boolean(false)))
    },
    join, fn,
    |_, this, [separator]| {
        let this = this.borrow().unwrap_proto("Array.join for this");
        let length = array_length(&this);
        let sep = match inline_borrow!(separator.clone()) {
            JsValue::String(s) => s.clone(),
            JsValue::Undefined => ",".to_owned(),
            _ => string_value(&separator),
        };

        let strings: Vec<String> = (0..length)
            .map(|i| {
                let val = array_element(&this, i);
                match inline_borrow!(val.clone()) {
                    JsValue::Null | JsValue::Undefined => "".to_owned(),
                    _ => string_value(&val),
                }
            })
            .collect();
        CodeResult::Return(value(JsValue::String(strings.join(&sep))))
    };
    keys, fn_gen,
    |env, _| {
        env.mem
            .borrow_mut()
            .properties
            .insert("i".into(), value(JsValue::BigInt(0)));
        CodeResult::Normal(value(JsValue::Undefined))
    };
    |env, _| {
        let index = integer(&Prototype::find(env.mem.clone(), &"i".into()).1);
        let this = Prototype::find(env.mem.clone(), &"this".into())
            .1
            .borrow()
            .unwrap_proto("Array.keys for this");
        if index >= array_length(&this) {
            CodeResult::YieldBreak
        } else {
            env.mem
                .borrow_mut()
                .properties
                .insert("i".into(), value(JsValue::BigInt(index + 1)));
            CodeResult::Yield(value(JsValue::BigInt(index)))
        }
    };
    |_, _| CodeResult::Normal(value(JsValue::Undefined)),
    entries, fn_gen,
    |env, _| {
        env.mem
            .borrow_mut()
            .properties
            .insert("i".into(), value(JsValue::BigInt(0)));
        CodeResult::Normal(value(JsValue::Undefined))
    };
    |env, _| {
        let index = integer(&Prototype::find(env.mem.clone(), &"i".into()).1);
        let this = Prototype::find(env.mem.clone(), &"this".into())
            .1
            .borrow()
            .unwrap_proto("Array.entries for this");
        if index >= array_length(&this) {
            CodeResult::YieldBreak
        } else {
            let array = Prototype::find(env.mem.clone(), &stringify!(Array).into())
                .1
                .borrow()
                .unwrap_proto("Array.entries for Array");
            let pair = new_array(
                array,
                vec![
                    value(JsValue::BigInt(index)),
                    array_element(&this, index),
                ],
                env.logger.clone(),
            );
            env.mem
                .borrow_mut()
                .properties
                .insert("i".into(), value(JsValue::BigInt(index + 1)));
            CodeResult::Yield(pair)
        }
    };
    |_, _| CodeResult::Normal(value(JsValue::Undefined)),
    Symbol.iterator, fn_gen,
    |env, _| {
        env.mem.borrow_mut()
            .properties
            .insert("i".into(), Rc::new(RefCell::new(JsValue::BigInt(0))));
        CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
    };
    |env, _| {
        let JsValue::BigInt(i) = inline_borrow!(Prototype::find(env.mem.clone(), &"i".into()).1) else {
            panic!("Array.iterator next index is not BigInt")
        };
        let this = env.mem
            .borrow()
            .properties[&"this".into()]
            .borrow()
            .unwrap_proto("Array.iterator this not found");
        let JsValue::BigInt(arr_len) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else {
            panic!("Array.iterator next length is not BigInt")
        };
        if i >= arr_len {
            CodeResult::YieldBreak
        } else {
            let obj = Prototype::find(this.clone(), &i.into()).1;
            env.mem
                .borrow_mut()
                .properties
                .insert("i".into(), Rc::new(RefCell::new(JsValue::BigInt(i + 1))));
            CodeResult::Yield(obj)
        }
    };
    |env, ind| {
        let JsValue::BigInt(i) = inline_borrow!(Prototype::find(env.mem.clone(), &"i".into()).1) else {
            panic!("Array.iterator return index is not BigInt")
        };
        let this = env.mem
            .borrow()
            .properties[&"this".into()]
            .borrow()
            .unwrap_proto("Array.iterator this not found");
        let JsValue::BigInt(arr_len) = inline_borrow!(Prototype::find(this.clone(), &"length".into()).1) else {
            panic!("Array.iterator return length is not BigInt")
        };
        if i < arr_len {
            ind.move_iamount(-1);
            ind.set_retry();
        }
        CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
    }
}
