use crate::{CodeResult, JsValue, assert_result, tests::*};

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

assert_result!(
    test_date_get_year,
    r#"
    console.log(new Date(1899, 0).getYear());
    console.log(new Date(1899, 11, 31, 23, 59, 59, 999).getYear());
    console.log(new Date(1900, 0).getYear());
    console.log(new Date(1900, 11, 31, 23, 59, 59, 999).getYear());
    console.log(new Date(1970, 0).getYear());
    console.log(new Date(1970, 11, 31, 23, 59, 59, 999).getYear());
    console.log(new Date(2000, 0).getYear());
    console.log(new Date(2000, 11, 31, 23, 59, 59, 999).getYear());
    console.log(new Date(0, 0).getYear());
    console.log(new Date(99, 0).getYear());
    console.log(new Date(100, 0).getYear());
    console.log(new Date(1970, 12, 1).getYear());
    console.log(new Date({}).getYear());
    console.log(Date.prototype.getYear.name);
    console.log(Date.prototype.getYear.length);
    "#,
    "-1",
    "-1",
    "0",
    "0",
    "70",
    "70",
    "100",
    "100",
    "0",
    "99",
    "-1800",
    "71",
    "NaN",
    "getYear",
    "0"
);

assert_result!(
    test_date_get_year_requires_date,
    r#"
    var getYear = Date.prototype.getYear;
    var objectThrew = false;
    var undefinedThrew = false;
    var nullThrew = false;
    try { getYear.call({}); } catch (_) { objectThrew = true; }
    try { getYear.call(undefined); } catch (_) { undefinedThrew = true; }
    try { getYear.call(null); } catch (_) { nullThrew = true; }
    console.log(objectThrew);
    console.log(undefinedThrew);
    console.log(nullThrew);
    "#,
    "true",
    "true",
    "true"
);

assert_result!(
    test_date_get_year_is_not_a_constructor,
    r#"
    var date = new Date(Date.now());
    var constructable = true;
    var threw = false;
    try { Reflect.construct(function(){}, [], Date.prototype.getYear); } catch (_) { constructable = false; }
    var callbackType = "";
    function assertThrows(expected, callback) {
        callbackType = typeof callback;
        try { callback(); } catch (error) { threw = error.constructor === expected; }
    }
    assertThrows(TypeError, () => { new date.getYear(); });
    console.log(constructable);
    console.log(callbackType);
    console.log(threw);
    "#,
    "false",
    "function",
    "true"
);

assert_result!(
    test_date_set_year,
    r#"
    var date = new Date(1970, 1, 2, 3, 4, 5);
    console.log(date.setYear(71));
    console.log(date.getYear());
    console.log(date.getFullYear());
    console.log(date.valueOf());
    console.log(Date.prototype.setYear.name);
    console.log(Date.prototype.setYear.length);

    date = new Date({});
    console.log(date.setYear(71));
    console.log(date.getFullYear());

    date = new Date(0);
    console.log(date.setYear());
    console.log(date.valueOf());
    console.log(date.setYear(Infinity));
    console.log(Date.prototype.setYear(1));
    "#,
    "34311845000",
    "71",
    "1971",
    "34311845000",
    "setYear",
    "1",
    "31536000000",
    "1971",
    "NaN",
    "NaN",
    "NaN",
    "-2177452800000"
);

assert_result!(
    test_date_set_year_to_number_order,
    r#"
    var date = new Date(0);
    var value = {
        valueOf: function() {
            date.setTime(NaN);
            return 1;
        }
    };
    date.setYear(value);
    console.log(date.getYear());
    console.log(date.getFullYear());
    "#,
    "1",
    "1901"
);

assert_result!(
    test_date_set_year_is_not_a_constructor,
    r#"
    var constructable = true;
    try { Reflect.construct(function(){}, [], Date.prototype.setYear); } catch (_) { constructable = false; }
    var threw = false;
    try { var date = new Date(Date.now()); new date.setYear(); } catch (error) { threw = error.constructor === TypeError; }
    console.log(constructable);
    console.log(threw);
    "#,
    "false",
    "true"
);

fn append_console_log(logs_ptr: &mut i64, value: String) {
    let logs = unsafe { ((*logs_ptr) as *mut Vec<String>).as_mut_unchecked() };
    logs.push(value);
}

#[test]
pub fn test_console() {
    let mut logs = Vec::new();
    let protos = prebuild_prototypes_test(&mut Loggable::<i64> {
        logger: &(append_console_log as fn(&mut i64, String)),
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
