use std::{cell::RefCell, fmt::Debug, rc::Rc};

use crate::{
    ARGUMENTS, CONSTRUCTOR_NAME, Code, CodeIndex, CodeResult, JsValue, LogLevel, PROTO_NAME,
    PROTOTYPE_NAME, Prototype, RUNNABLE, inline_borrow, new_array, run_sub,
};

pub struct Runnable {
    pub params: Vec<String>,
    pub excess: Option<String>,
    pub code: Vec<Code>,
    pub mem: Rc<RefCell<Prototype>>,
}

pub fn new_runnable_with_object(
    function: Rc<RefCell<Prototype>>,
    object_proto: Rc<RefCell<Prototype>>,
    name: &'static str,
    runnable: Runnable,
) -> Rc<RefCell<JsValue>> {
    let function_obj = Rc::new(RefCell::new(Prototype {
        name: Some(name),
        properties: std::collections::HashMap::new(),
        formating: false,
    }));
    let prototype_obj = Prototype::new_child(
        object_proto,
        None,
        [(
            CONSTRUCTOR_NAME.into(),
            Rc::new(RefCell::new(JsValue::Prototype(function_obj.clone()))),
        )],
    );
    function_obj.borrow_mut().properties.insert(
        PROTO_NAME.into(),
        Rc::new(RefCell::new(JsValue::Prototype(function.clone()))),
    );
    function_obj.borrow_mut().properties.insert(
        RUNNABLE.into(),
        Rc::new(RefCell::new(JsValue::Function(Rc::new(runnable)))),
    );
    function_obj.borrow_mut().properties.insert(
        PROTOTYPE_NAME.into(),
        Rc::new(RefCell::new(JsValue::Prototype(prototype_obj))),
    );
    Rc::new(RefCell::new(JsValue::Prototype(function_obj)))
}

pub fn new_runnable(
    function: Rc<RefCell<Prototype>>,
    name: &'static str,
    runnable: Runnable,
) -> Rc<RefCell<JsValue>> {
    let object_proto = function
        .borrow()
        .parent()
        .expect("Function prototype should inherit from Object");
    new_runnable_with_object(function, object_proto, name, runnable)
}

pub fn run_function_object(
    func: Rc<RefCell<Prototype>>,
    this: Rc<RefCell<JsValue>>,
    params: Vec<Rc<RefCell<JsValue>>>,
) -> Rc<RefCell<JsValue>> {
    let JsValue::Function(ref runnable) =
        inline_borrow!(inline_borrow!(func.clone()).properties[&RUNNABLE.into()].clone())
    else {
        panic!("func not runnable")
    };
    crate::logln(
        LogLevel::Info,
        &format!(
            "run_function_object: {}({:?})",
            func.borrow().name.unwrap_or("anonymous"),
            params
        ),
    );
    let mem = runnable.mem.clone();

    let proto = Rc::new(RefCell::new(Prototype {
        name: Some("stack memory"),
        properties: params
            .iter()
            .take(runnable.params.len())
            .enumerate()
            .map(|(i, param)| (runnable.params[i].as_str().into(), param.clone()))
            .collect(),
        formating: false,
    }));
    proto.borrow_mut().properties.insert(
        PROTO_NAME.into(),
        Rc::new(RefCell::new(JsValue::Prototype(mem.clone()))),
    );
    let JsValue::Prototype(array) =
        inline_borrow!(Prototype::find(mem.clone(), &stringify!(Array).into()).1)
    else {
        panic!("Array not found")
    };
    if let Some(excess) = &runnable.excess {
        proto.borrow_mut().properties.insert(
            excess.as_str().into(),
            new_array(
                array,
                params.iter().skip(runnable.params.len()).cloned().collect(),
            ),
        );
    }
    proto.borrow_mut().properties.insert(
        ARGUMENTS.into(),
        new_array(
            Prototype::find(mem.clone(), &stringify!(Array).into())
                .1
                .borrow()
                .unwrap_proto("run_function_object get Array"),
            params,
        ),
    );
    proto
        .borrow_mut()
        .properties
        .insert("this".into(), this.clone());
    if let JsValue::Prototype(obj) = inline_borrow!(this)
        && let JsValue::Prototype(class) =
            inline_borrow!(Prototype::find(obj.clone(), &PROTO_NAME.into()).1)
        && let JsValue::Prototype(super_val) =
            inline_borrow!(Prototype::find(class.clone(), &PROTO_NAME.into()).1)
        && let JsValue::Prototype(super_constructor) =
            inline_borrow!(Prototype::find(super_val.clone(), &"constructor".into()).1)
    {
        proto.borrow_mut().properties.insert(
            "super".into(),
            Rc::new(RefCell::new(JsValue::Prototype(super_constructor.clone()))),
        );
    }

    let res = run_sub(&runnable.code, proto, &mut CodeIndex::new());
    if let CodeResult::Return(res) = res {
        res
    } else {
        Rc::new(RefCell::new(JsValue::Undefined))
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
