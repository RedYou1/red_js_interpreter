use crate::tests::*;

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
    assert_eq!(arr, new_array(array.clone(), vec![]));
    let arr = run_function_object(
        constructor.clone(),
        Rc::new(RefCell::new(JsValue::Undefined)),
        vec![Rc::new(RefCell::new(JsValue::BigInt(5)))],
    );
    assert_eq!(
        arr,
        new_array(
            array.clone(),
            vec![
                Rc::new(RefCell::new(JsValue::Undefined)),
                Rc::new(RefCell::new(JsValue::Undefined)),
                Rc::new(RefCell::new(JsValue::Undefined)),
                Rc::new(RefCell::new(JsValue::Undefined)),
                Rc::new(RefCell::new(JsValue::Undefined))
            ]
        )
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
    assert_eq!(arr, new_array(array.clone(), content.clone()));
    let arr = run_function_object(
        Prototype::find(array.clone(), &"of".into())
            .1
            .borrow()
            .unwrap_proto("test_new_array for Array.of"),
        Rc::new(RefCell::new(JsValue::Undefined)),
        content.clone(),
    );
    assert_eq!(arr, new_array(array.clone(), content.clone()));
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
    assert_eq!(log, Rc::new(RefCell::new(JsValue::Undefined)));

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
