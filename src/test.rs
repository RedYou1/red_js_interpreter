use std::{collections::HashMap, fmt::Debug};

use crate::{
    Generator, JsValue, Prototype, Runnable,
    prebuild::{
        array::new_array,
        console::{CONSOLE, default_console_config},
    },
    prebuild_prototypes, run_function_object,
};

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
    let protos = prebuild_prototypes(default_console_config);
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

    #[expect(static_mut_refs)]
    let t = unsafe { CONSOLE.as_slice() };
    assert_eq!(t, ["%Hello World%".to_owned()]);
    #[expect(static_mut_refs)]
    unsafe {
        CONSOLE.clear()
    };

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

    #[expect(static_mut_refs)]
    let t = unsafe { CONSOLE.as_slice() };
    assert_eq!(
        t,
        ["my name is bloup bloup and i'm 69 years old".to_owned()]
    );
    #[expect(static_mut_refs)]
    unsafe {
        CONSOLE.clear()
    };

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

    #[expect(static_mut_refs)]
    let t = unsafe { CONSOLE.as_slice() };
    assert_eq!(t, ["69 420 69.69 null".to_owned()]);
    #[expect(static_mut_refs)]
    unsafe {
        CONSOLE.clear()
    };
}

impl Debug for JsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function(arg0) => f.debug_tuple("Function").field(arg0).finish(),
            Self::Generator(arg0) => f.debug_tuple("Generator").field(arg0).finish(),
            Self::Prototype(arg0) => f.debug_tuple("Prototype").field(arg0).finish(),
            Self::Symbol(arg0, arg1) => f.debug_tuple("Symbol").field(arg0).field(arg1).finish(),
            Self::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
            Self::Number(arg0) => f.debug_tuple("Number").field(arg0).finish(),
            Self::BigInt(arg0) => f.debug_tuple("BigInt").field(arg0).finish(),
            Self::Boolean(arg0) => f.debug_tuple("Boolean").field(arg0).finish(),
            Self::Undefined => write!(f, "Undefined"),
            Self::Null => write!(f, "Null"),
        }
    }
}

impl Debug for Prototype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prototype")
            .field(
                "properties",
                &self
                    .properties
                    .iter()
                    .map(|(k, v)| {
                        let mut k = k.clone();
                        let mut v = v.clone();

                        if let JsValue::Prototype(in_k) = &k {
                            let name = in_k.borrow().name;
                            if let Some(name) = name {
                                k = JsValue::String(format!("[{}]", name));
                            }
                        }

                        if let JsValue::Prototype(in_v) = &v {
                            let name = in_v.borrow().name;
                            if let Some(name) = name {
                                v = JsValue::String(format!("[{}]", name));
                            }
                        }
                        (k, v)
                    })
                    .collect::<HashMap<JsValue, JsValue>>(),
            )
            .finish()
    }
}

impl Debug for Runnable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runnable")
            .field("params", &self.params)
            .field("excess", &self.excess)
            .finish()
    }
}

impl Debug for Generator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Generator")
            .field("params", &self.params)
            .field("excess", &self.excess)
            .finish()
    }
}
