use crate::tests::*;

#[test]
pub fn test_new_array() {
    let protos = prebuild_prototypes(default_console_config);
    let array = Prototype::find(protos.clone(), &"Array".into())
        .1
        .unwrap_proto();
    let arr = run_function_object(
        protos.clone(),
        Prototype::find(array.clone(), &"constructor".into())
            .1
            .unwrap_proto(),
        JsValue::Undefined,
        vec![],
    );
    assert_eq!(arr, new_array(array.clone(), vec![]));
    let arr = run_function_object(
        protos.clone(),
        Prototype::find(array.clone(), &"constructor".into())
            .1
            .unwrap_proto(),
        JsValue::Undefined,
        vec![JsValue::BigInt(5)],
    );
    assert_eq!(
        arr,
        new_array(
            array.clone(),
            vec![
                JsValue::Undefined,
                JsValue::Undefined,
                JsValue::Undefined,
                JsValue::Undefined,
                JsValue::Undefined
            ]
        )
    );
    let content = vec![JsValue::BigInt(1), JsValue::String("Wow".to_owned())];
    let arr = run_function_object(
        protos.clone(),
        Prototype::find(array.clone(), &"constructor".into())
            .1
            .unwrap_proto(),
        JsValue::Undefined,
        content.clone(),
    );
    assert_eq!(arr, new_array(array.clone(), content.clone()));
    let arr = run_function_object(
        protos.clone(),
        Prototype::find(array.clone(), &"of".into())
            .1
            .unwrap_proto(),
        JsValue::Undefined,
        content.clone(),
    );
    assert_eq!(arr, new_array(array.clone(), content.clone()));
}

#[test]
pub fn test_console() {
    let mut logs = Vec::new();
    let protos = prebuild_prototypes_test(&mut logs);

    let console = Prototype::find(protos.clone(), &"console".into()).1;
    let console_log = Prototype::find(console.unwrap_proto(), &"log".into())
        .1
        .unwrap_proto();
    let log = run_function_object(
        protos.clone(),
        console_log.clone(),
        console.clone(),
        vec![JsValue::String("%%Hello World%%".to_owned())],
    );
    assert_eq!(log, JsValue::Undefined);

    assert_eq!(logs.as_slice(), ["%Hello World%".to_owned()]);
    logs.clear();

    let _ = run_function_object(
        protos.clone(),
        console_log.clone(),
        console.clone(),
        vec![
            JsValue::String("my name is %s and i'm %d years old".to_owned()),
            JsValue::String("bloup bloup".to_owned()),
            JsValue::BigInt(69),
        ],
    );

    assert_eq!(
        logs.as_slice(),
        ["my name is bloup bloup and i'm 69 years old".to_owned()]
    );
    logs.clear();

    let _ = run_function_object(
        protos.clone(),
        console_log.clone(),
        console.clone(),
        vec![
            JsValue::BigInt(69),
            JsValue::BigInt(420),
            JsValue::Number(69.69),
            JsValue::Null,
        ],
    );

    assert_eq!(logs.as_slice(), ["69 420 69.69 null".to_owned()]);
}