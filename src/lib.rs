use std::{
    cell::RefCell,
    collections::HashMap,
    hash::{Hash, Hasher},
    rc::Rc,
};

mod parser;
mod prebuild;
pub use parser::compile_function;
pub use parser::parse;
pub use prebuild::console::default_console_config;

pub use parser::ast as parser_ast;

#[cfg(test)]
mod tests;

use crate::prebuild::{
    array::{new_array, prebuild_array},
    console::prebuild_console,
    iterator::prebuild_iterator,
    math::prebuild_math,
    number::prebuild_number,
    prebuild_runnable, prebuild_runnable_direct,
    string::prebuild_string,
    symbol::prebuild_symbol,
};

#[derive(Clone)]
pub enum JsValue {
    Function(Rc<Runnable>),
    Generator(Rc<Generator>),

    Prototype(Rc<RefCell<Prototype>>),
    Symbol(u64, Box<JsValue>),
    String(String),
    Number(f64),
    BigInt(i64),
    Boolean(bool),
    Undefined,
    Null,
}

impl Eq for JsValue {}

impl PartialEq for JsValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Prototype(l0), Self::Prototype(r0)) => l0 == r0,
            (Self::Symbol(l0, l1), Self::Symbol(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::Number(l0), Self::Number(r0)) => l0 == r0,
            (Self::BigInt(l0), Self::BigInt(r0)) => l0 == r0,
            (Self::Boolean(l0), Self::Boolean(r0)) => l0 == r0,
            (Self::Undefined, Self::Undefined) => true,
            (Self::Null, Self::Null) => true,
            _ => false,
        }
    }
}

impl Hash for JsValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Prototype(proto) => proto.borrow().hash(state),
            Self::Symbol(id, inner) => {
                id.hash(state);
                inner.hash(state);
            }
            Self::String(value) => value.hash(state),
            Self::Number(value) => value.to_bits().hash(state),
            Self::BigInt(value) => value.hash(state),
            Self::Boolean(value) => value.hash(state),
            Self::Undefined | Self::Null | Self::Function(_) | Self::Generator(_) => {}
        }
    }
}

impl From<&str> for JsValue {
    fn from(value: &str) -> Self {
        JsValue::String(value.to_owned())
    }
}

impl From<String> for JsValue {
    fn from(value: String) -> Self {
        JsValue::String(value)
    }
}

impl From<usize> for JsValue {
    fn from(value: usize) -> Self {
        JsValue::BigInt(value as i64)
    }
}

impl From<i64> for JsValue {
    fn from(value: i64) -> Self {
        JsValue::BigInt(value)
    }
}

impl From<f64> for JsValue {
    fn from(value: f64) -> Self {
        JsValue::Number(value)
    }
}

impl JsValue {
    pub fn opt_find(&self, key: &JsValue) -> Option<(Rc<RefCell<Prototype>>, JsValue)> {
        Prototype::opt_find(self.unwrap_proto(), key)
    }

    pub fn find(&self, key: &JsValue) -> (Option<Rc<RefCell<Prototype>>>, JsValue) {
        Prototype::find(self.unwrap_proto(), key)
    }

    pub fn unwrap_proto(&self) -> Rc<RefCell<Prototype>> {
        let JsValue::Prototype(p) = self else {
            panic!("not proto")
        };
        p.clone()
    }

    pub const fn is_fasly(&self) -> bool {
        match self {
            JsValue::Boolean(false) => true,
            JsValue::String(s) => s.is_empty(),
            JsValue::BigInt(n) if *n == 0 => true,
            JsValue::Number(n) if n.is_nan() || *n == 0. => true,
            JsValue::Null | JsValue::Undefined => true,
            _ => false,
        }
    }

    pub const fn is_truthy(&self) -> bool {
        !self.is_fasly()
    }

    pub fn print(&self) -> String {
        match self {
            JsValue::Function(_) => "[Function (anonymous)]".to_owned(),
            JsValue::Generator(_) => "[GeneratorFunction (anonymous)]".to_owned(),
            JsValue::Prototype(ref_cell) => {
                let mut entries = vec![];
                for (key, value) in ref_cell.borrow().properties.iter() {
                    if key == &PROTO_NAME.into() {
                        continue;
                    }
                    let k = match key {
                        JsValue::String(s) => s.clone(),
                        JsValue::Prototype(key) if let Some(name) = key.borrow().name => {
                            format!("[{name}]")
                        }
                        _ => key.print(),
                    };
                    let v = match value {
                        JsValue::String(s) => format!("'{}'", s),
                        JsValue::Prototype(v) if let Some(name) = v.borrow().name => {
                            format!("[{name}]")
                        }
                        _ => value.print(),
                    };
                    entries.push((key.clone(), k, v));
                }
                const fn key_order(k: &JsValue) -> u8 {
                    match k {
                        JsValue::String(_) => 0,
                        JsValue::Number(_) => 1,
                        JsValue::Prototype(_) => 2,
                        _ => 3,
                    }
                }
                entries.sort_by(|a, b| {
                    let ao = key_order(&a.0);
                    let bo = key_order(&b.0);
                    if ao != bo { ao.cmp(&bo) } else { a.1.cmp(&b.1) }
                });
                format!(
                    "{{ {} }}",
                    entries
                        .into_iter()
                        .map(|(_, k, v)| format!("{}: {}", k, v))
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            JsValue::Symbol(_, js_value) => format!("Symbol({})", js_value.print()),
            JsValue::String(s) => s.clone(),
            JsValue::Number(n) => format!("{n}"),
            JsValue::BigInt(n) => n.to_string(),
            JsValue::Boolean(b) => if *b { "true" } else { "false" }.to_owned(),
            JsValue::Undefined => "undefined".to_owned(),
            JsValue::Null => "null".to_owned(),
        }
    }
}

#[derive(Eq, PartialEq)]
pub struct Prototype {
    name: Option<&'static str>,
    properties: HashMap<JsValue, JsValue>,
}

impl Hash for Prototype {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut entry_hashes = self
            .properties
            .iter()
            .map(|(key, value)| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                key.hash(&mut hasher);
                value.hash(&mut hasher);
                hasher.finish()
            })
            .collect::<Vec<_>>();
        entry_hashes.sort_unstable();
        entry_hashes.hash(state);
    }
}

impl Prototype {
    pub fn inner_find(
        this: Rc<RefCell<Self>>,
        key: &JsValue,
    ) -> Option<(Rc<RefCell<Prototype>>, JsValue)> {
        if let Some(elem) = this.borrow().properties.get(key) {
            Some((this.clone(), elem.clone()))
        } else {
            Self::inner_find(this.borrow().parent()?, key)
        }
    }

    pub fn opt_find(
        this: Rc<RefCell<Self>>,
        key: &JsValue,
    ) -> Option<(Rc<RefCell<Prototype>>, JsValue)> {
        Self::inner_find(this, key)
    }

    pub fn find(
        this: Rc<RefCell<Self>>,
        key: &JsValue,
    ) -> (Option<Rc<RefCell<Prototype>>>, JsValue) {
        Self::inner_find(this, key)
            .map(|(a, b)| (Some(a), b))
            .unwrap_or((None, JsValue::Undefined))
    }

    pub fn parent(&self) -> Option<Rc<RefCell<Prototype>>> {
        if let Some(JsValue::Prototype(proto)) = self.properties.get(&PROTO_NAME.into()) {
            Some(proto.clone())
        } else {
            None
        }
    }

    pub fn new_child(
        this: Rc<RefCell<Self>>,
        name: Option<&'static str>,
        properties: impl IntoIterator<Item = (JsValue, JsValue)>,
    ) -> Rc<RefCell<Self>> {
        let mut properties = HashMap::from_iter(properties);
        properties.insert(PROTO_NAME.into(), JsValue::Prototype(this));
        Rc::new(RefCell::new(Prototype { name, properties }))
    }
}

/*
https://miro.medium.com/v2/resize:fit:2000/1*rGkuqfPZUEQw9hvJLV0Gig.png
https://i.sstatic.net/uy5ce.png

https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Function
*/

const PROTO_NAME: &str = "__proto__";
const PROTOTYPE_NAME: &str = "prototype";
const CONSTRUCTOR_NAME: &str = "constructor";

pub fn prebuild_prototypes(
    console_config: impl Fn(Rc<RefCell<Prototype>>) -> Rc<RefCell<Prototype>>,
) -> Rc<RefCell<Prototype>> {
    let object = Rc::new(RefCell::new(Prototype {
        name: Some("Object"),
        properties: HashMap::from([(PROTO_NAME.into(), JsValue::Null)]),
    }));

    let function = Prototype::new_child(object.clone(), Some("Function"), []);

    let prototypes = Rc::new(RefCell::new(Prototype {
        name: Some("root memory"),
        properties: HashMap::from([
            ("Object".into(), JsValue::Prototype(object.clone())),
            ("Function".into(), JsValue::Prototype(function.clone())),
        ]),
    }));

    let obj = object.clone();
    object.borrow_mut().properties.insert(
        CONSTRUCTOR_NAME.into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            Some("Object.constructor"),
            prebuild_runnable(
                prototypes.clone(),
                Box::new(move |_mem, _this, [value]| {
                    if let JsValue::Undefined | JsValue::Null = value {
                        JsValue::Prototype(Prototype::new_child(
                            obj.clone(),
                            None,
                            [(PROTOTYPE_NAME.into(), JsValue::Prototype(obj.clone()))],
                        ))
                    } else if let JsValue::Prototype(_) = value {
                        value
                    } else {
                        todo!() // return as an object of primitive wrapper
                    }
                }),
            ),
        ),
    );

    object.borrow_mut().properties.insert(
        "create".into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            Some("Object.create"),
            prebuild_runnable_direct(
                prototypes.clone(),
                Box::new(|_mem, _this, arguments| {
                    if let Some(proto) = arguments.get(0) {
                        if let JsValue::Prototype(proto_obj) = proto.clone() {
                            JsValue::Prototype(Prototype::new_child(proto_obj, None, []))
                        } else {
                            JsValue::Undefined
                        }
                    } else {
                        JsValue::Undefined
                    }
                }),
            ),
        ),
    );

    function.borrow_mut().properties.insert(
        "call".into(),
        new_runnable_with_object(
            function.clone(),
            object.clone(),
            Some("Function.call"),
            prebuild_runnable_direct(
                prototypes.clone(),
                Box::new(|mem, this, arguments| {
                    if let JsValue::Prototype(ref func_proto) = this {
                        let this_arg = arguments.get(0).cloned().unwrap_or(JsValue::Undefined);
                        let params: Vec<JsValue> = arguments.iter().skip(1).cloned().collect();
                        crate::run_function_object(mem.clone(), func_proto.clone(), this_arg, params)
                    } else {
                        JsValue::Undefined
                    }
                }),
            ),
        ),
    );

    prebuild_symbol(prototypes.clone());
    prebuild_iterator(prototypes.clone());
    prebuild_array(prototypes.clone());
    prebuild_string(prototypes.clone());
    prebuild_number(prototypes.clone());
    prebuild_math(prototypes.clone());
    prebuild_console(prototypes.clone());
    prototypes.borrow().properties[&JsValue::String("console".to_owned())]
        .unwrap_proto()
        .borrow_mut()
        .properties
        .insert(
            JsValue::String("__config__".to_owned()),
            JsValue::Prototype(console_config(prototypes.clone())),
        );
    prototypes
}

const RUNNABLE: &str = "__!@#$%^&*()__";
const ARGUMENTS: &str = "arguments";

pub struct Runnable {
    params: Vec<String>,
    excess: Option<String>,
    code: Vec<Box<dyn Fn(Rc<RefCell<Prototype>>, &mut usize) -> (JsValue, Option<JsValue>)>>,
}

pub fn new_runnable_with_object(
    function: Rc<RefCell<Prototype>>,
    object_proto: Rc<RefCell<Prototype>>,
    name: Option<&'static str>,
    runnable: Runnable,
) -> JsValue {
    let function_obj = Rc::new(RefCell::new(Prototype {
        name,
        properties: std::collections::HashMap::new(),
    }));
    let prototype_obj = Prototype::new_child(
        object_proto,
        None,
        [(CONSTRUCTOR_NAME.into(), JsValue::Prototype(function_obj.clone()))],
    );
    function_obj.borrow_mut().properties.insert(
        PROTO_NAME.into(),
        JsValue::Prototype(function.clone()),
    );
    function_obj.borrow_mut().properties.insert(
        RUNNABLE.into(),
        JsValue::Function(Rc::new(runnable)),
    );
    function_obj.borrow_mut().properties.insert(
        PROTOTYPE_NAME.into(),
        JsValue::Prototype(prototype_obj),
    );
    JsValue::Prototype(function_obj)
}

pub fn new_runnable(
    function: Rc<RefCell<Prototype>>,
    name: Option<&'static str>,
    runnable: Runnable,
) -> JsValue {
    let object_proto = function
        .borrow()
        .parent()
        .expect("Function prototype should inherit from Object");
    new_runnable_with_object(function, object_proto, name, runnable)
}

pub fn run_function_object(
    mem: Rc<RefCell<Prototype>>,
    func: Rc<RefCell<Prototype>>,
    this: JsValue,
    params: Vec<JsValue>,
) -> JsValue {
    let JsValue::Function(ref runnable) = func.borrow().properties[&RUNNABLE.into()] else {
        panic!("func not runnable")
    };

    let proto = Rc::new(RefCell::new(Prototype {
        name: Some("stack memory"),
        properties: params
            .iter()
            .take(runnable.params.len())
            .enumerate()
            .map(|(i, param)| (runnable.params[i].as_str().into(), param.clone()))
            .collect(),
    }));
    let JsValue::Prototype(array) = Prototype::find(mem.clone(), &"Array".into()).1 else {
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
            Prototype::find(mem.clone(), &"Array".into())
                .1
                .unwrap_proto(),
            params,
        ),
    );
    proto.borrow_mut().properties.insert("this".into(), this);
    if let Some(super_val) = func.borrow().properties.get(&"super".into()) {
        proto.borrow_mut().properties.insert("super".into(), super_val.clone());
    }

    let mut i = 0;
    while i < runnable.code.len() {
        let temp = i;
        let res = runnable.code[i](proto.clone(), &mut i);
        if i == temp {
            i += 1;
        }
        if let (_, Some(res)) = res {
            return res;
        }
    }

    JsValue::Undefined
}

pub struct Generator {
    params: Vec<String>,
    excess: Option<String>,
    code: Rc<
        Vec<
            Box<
                dyn Fn(Rc<RefCell<Prototype>>, &mut bool, &mut usize) -> (JsValue, Option<JsValue>),
            >,
        >,
    >,
}

pub struct IterGenerator {
    index: usize,
    proto: Rc<RefCell<Prototype>>,
    code: Rc<
        Vec<
            Box<
                dyn Fn(Rc<RefCell<Prototype>>, &mut bool, &mut usize) -> (JsValue, Option<JsValue>),
            >,
        >,
    >,
}
impl Iterator for IterGenerator {
    type Item = JsValue;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.code.len() {
            let mut go_next = true;
            let res = self.code[self.index](self.proto.clone(), &mut go_next, &mut self.index);
            if go_next {
                self.index += 1;
            }
            if let (_, Some(res)) = res {
                return Some(res);
            }
        }
        None
    }
}

pub fn new_generator(
    function: Rc<RefCell<Prototype>>,
    name: Option<&'static str>,
    runnable: Generator,
) -> JsValue {
    JsValue::Prototype(Prototype::new_child(
        function.clone(),
        name,
        [
            (PROTOTYPE_NAME.into(), JsValue::Prototype(function.clone())),
            (RUNNABLE.into(), JsValue::Generator(Rc::new(runnable))),
        ],
    ))
}

pub fn run_generator_object(
    mem: Rc<RefCell<Prototype>>,
    func: Rc<RefCell<Prototype>>,
    this: JsValue,
    params: Vec<JsValue>,
) -> IterGenerator {
    let JsValue::Generator(ref runnable) = func.borrow().properties[&RUNNABLE.into()] else {
        panic!("func not runnable")
    };

    let proto = Rc::new(RefCell::new(Prototype {
        name: Some("stack memory"),
        properties: params
            .iter()
            .take(runnable.params.len())
            .enumerate()
            .map(|(i, param)| (runnable.params[i].as_str().into(), param.clone()))
            .collect(),
    }));
    let JsValue::Prototype(array) = Prototype::find(mem.clone(), &"Array".into()).1 else {
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
            Prototype::find(mem.clone(), &"Array".into())
                .1
                .unwrap_proto(),
            params,
        ),
    );
    proto.borrow_mut().properties.insert("this".into(), this);

    IterGenerator {
        index: 0,
        proto,
        code: runnable.code.clone(),
    }
}
