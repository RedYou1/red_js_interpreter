use crate::{CodeResult, JsValue, inline_borrow, parse, tests::*};

#[test]
pub fn test_new_array() {
    let protos = prebuild_prototypes(default_console_config);
    let array = Prototype::find(protos.clone(), &stringify!(Array).into())
        .1
        .borrow()
        .unwrap_proto("test_new_array for Array");
    let constructor = Prototype::find(array.clone(), &"constructor".into())
        .1
        .borrow()
        .unwrap_proto("test_new_array for Array.constructor");
    let arr = run_function_object(
        constructor.clone(),
        Rc::new(RefCell::new(JsValue::Undefined)),
        vec![],
    );
    assert_eq!(
        arr,
        CodeResult::Return(new_array_with_length(array.clone(), 0))
    );
    let arr = run_function_object(
        constructor.clone(),
        Rc::new(RefCell::new(JsValue::Undefined)),
        vec![Rc::new(RefCell::new(JsValue::BigInt(5)))],
    );
    assert_eq!(
        arr,
        CodeResult::Return(new_array_with_length(array.clone(), 5))
    );
    let content = vec![
        Rc::new(RefCell::new(JsValue::BigInt(1))),
        Rc::new(RefCell::new(JsValue::String("Wow".to_owned()))),
    ];
    let arr = run_function_object(
        constructor.clone(),
        Rc::new(RefCell::new(JsValue::Undefined)),
        content.clone(),
    );
    assert_eq!(
        arr,
        CodeResult::Return(new_array(array.clone(), content.clone()))
    );
    let arr = run_function_object(
        Prototype::find(array.clone(), &"of".into())
            .1
            .borrow()
            .unwrap_proto("test_new_array for Array.of"),
        Rc::new(RefCell::new(JsValue::Undefined)),
        content.clone(),
    );
    assert_eq!(
        arr,
        CodeResult::Return(new_array(array.clone(), content.clone()))
    );
}

#[test]
pub fn test_date_get_year() {
    let protos = prebuild_prototypes(default_console_config);
    let eval = |source| {
        let program = parse(&format!("return {source};")).compile(protos.clone());
        let function = Prototype::find(protos.clone(), &JsValue::String("Function".to_owned()))
            .1
            .borrow()
            .unwrap_proto("test_date_get_year for Function");
        let main = new_runnable(function, "__date_test__", program)
            .borrow()
            .unwrap_proto("test_date_get_year for main");
        let result = run_function_object(main, Rc::new(RefCell::new(JsValue::Undefined)), vec![]);
        inline_borrow!(match result {
            CodeResult::Return(value) | CodeResult::Normal(value) => value,
            result => panic!("unexpected Date result: {result:?}"),
        })
    };

    assert_eq!(eval("new Date(1899, 0).getYear()"), JsValue::Number(-1.0));
    assert_eq!(
        eval("new Date(1899, 11, 31, 23, 59, 59, 999).getYear()"),
        JsValue::Number(-1.0)
    );
    assert_eq!(eval("new Date(1900, 0).getYear()"), JsValue::Number(0.0));
    assert_eq!(
        eval("new Date(1900, 11, 31, 23, 59, 59, 999).getYear()"),
        JsValue::Number(0.0)
    );
    assert_eq!(eval("new Date(1970, 0).getYear()"), JsValue::Number(70.0));
    assert_eq!(eval("new Date(2000, 0).getYear()"), JsValue::Number(100.0));
    assert!(matches!(eval("new Date({}).getYear()"), JsValue::Number(value) if value.is_nan()));
    assert_eq!(
        eval("Date.prototype.getYear.name"),
        JsValue::String("getYear".to_owned())
    );
    assert_eq!(eval("Date.prototype.getYear.length"), JsValue::BigInt(0));
}

#[test]
pub fn test_date_get_year_requires_date() {
    let protos = prebuild_prototypes(default_console_config);
    let program = parse("return Date.prototype.getYear.call({});").compile(protos.clone());
    let function = Prototype::find(protos.clone(), &JsValue::String("Function".to_owned()))
        .1
        .borrow()
        .unwrap_proto("test_date_get_year_requires_date for Function");
    let main = new_runnable(function, "__date_type_test__", program)
        .borrow()
        .unwrap_proto("test_date_get_year_requires_date for main");
    let result = run_function_object(main, Rc::new(RefCell::new(JsValue::Undefined)), vec![]);
    assert!(matches!(result, CodeResult::Error(_)));
}

fn append(logs_ptr: &mut i64, value: String) {
    let logs = unsafe { ((*logs_ptr) as *mut Vec<String>).as_mut_unchecked() };
    logs.push(value);
}

#[test]
pub fn test_console() {
    let mut logs = Vec::new();
    let protos = prebuild_prototypes_test(&mut Loggable::<i64> {
        logger: &(append as fn(&mut i64, String)),
        data: &mut logs as *mut Vec<String> as i64,
    });

    let console = Prototype::find(protos.clone(), &"console".into()).1;
    let console_log = Prototype::find(
        console.borrow().unwrap_proto("test_console for console"),
        &"log".into(),
    )
    .1
    .borrow()
    .unwrap_proto("test_console for console.log");
    let log = run_function_object(
        console_log.clone(),
        console.clone(),
        vec![Rc::new(RefCell::new(JsValue::String(
            "%%Hello World%%".to_owned(),
        )))],
    );
    assert_eq!(
        log,
        CodeResult::Return(Rc::new(RefCell::new(JsValue::Undefined)))
    );

    assert_eq!(logs.as_slice(), ["%Hello World%".to_owned()]);
    logs.clear();

    let _ = run_function_object(
        console_log.clone(),
        console.clone(),
        vec![
            Rc::new(RefCell::new(JsValue::String(
                "my name is %s and i'm %d years old".to_owned(),
            ))),
            Rc::new(RefCell::new(JsValue::String("bloup bloup".to_owned()))),
            Rc::new(RefCell::new(JsValue::BigInt(69))),
        ],
    );

    assert_eq!(
        logs.as_slice(),
        ["my name is bloup bloup and i'm 69 years old".to_owned()]
    );
    logs.clear();

    let _ = run_function_object(
        console_log.clone(),
        console.clone(),
        vec![
            Rc::new(RefCell::new(JsValue::BigInt(69))),
            Rc::new(RefCell::new(JsValue::BigInt(420))),
            Rc::new(RefCell::new(JsValue::Number(69.69))),
            Rc::new(RefCell::new(JsValue::Null)),
        ],
    );

    assert_eq!(logs.as_slice(), ["69 420 69.69 null".to_owned()]);
}
