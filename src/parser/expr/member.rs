use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, JsValue, LogLevel, PRIMITIVE_BOX_MARKER,
    PRIMITIVE_BOX_VALUE, Prototype, handle_return, inline_borrow, parser::expr::Expr, run_sub,
};

#[derive(Debug)]
pub struct Member {
    pub object: Box<dyn Expr>,
    pub property: Box<dyn Expr>,
}

impl Expr for Member {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let obj = self.object.compile(env.clone());
        let prop = self.property.compile(env);
        let expression = format!("{self:?}");
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Member");
            let obj_value = handle_return!(run_sub(&obj, env.clone(), &mut CodeIndex::new()));
            let obj = match inline_borrow!(obj_value.clone()) {
                JsValue::Prototype(object) => object,
                JsValue::String(_)
                | JsValue::Number(_)
                | JsValue::BigInt(_)
                | JsValue::Boolean(_) => {
                    box_primitive(&env, obj_value.clone()).unwrap_or_else(|| {
                        panic!("expr::Member {expression} could not box {obj_value:?}")
                    })
                }
                other => panic!("expr::Member {expression} received {other:?}"),
            };
            let key = handle_return!(run_sub(&prop, env.clone(), &mut CodeIndex::new()));
            let out = Prototype::find(obj.clone(), &inline_borrow!(key.clone())).1;
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::Member {obj:?}[{key:?}] == {out:?}")
            });
            CodeResult::NormalMember(out, obj, key)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            object: self.object.duplicate(),
            property: self.property.duplicate(),
        })
    }
}

#[derive(Debug)]
pub struct OptionalMember {
    pub object: Box<dyn Expr>,
    pub property: Box<dyn Expr>,
}

impl Expr for OptionalMember {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let object = self.object.compile(env.clone());
        let property = self.property.compile(env);
        vec![Box::new(move |env, _| {
            let object = match run_sub(&object, env.clone(), &mut CodeIndex::new()) {
                CodeResult::Normal(value) | CodeResult::NormalMember(value, _, _) => value,
                result => return result,
            };
            let object = match inline_borrow!(object.clone()) {
                JsValue::Undefined | JsValue::Null => {
                    return CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)));
                }
                JsValue::Prototype(object) => object,
                JsValue::String(_)
                | JsValue::Number(_)
                | JsValue::BigInt(_)
                | JsValue::Boolean(_) => {
                    let Some(object) = box_primitive(&env, object.clone()) else {
                        return CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)));
                    };
                    object
                }
                _ => return CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined))),
            };
            let key = match run_sub(&property, env.clone(), &mut CodeIndex::new()) {
                CodeResult::Normal(key) | CodeResult::NormalMember(key, _, _) => key,
                result => return result,
            };
            let value = Prototype::find(object.clone(), &inline_borrow!(key.clone())).1;
            CodeResult::NormalMember(value, object, key)
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            object: self.object.duplicate(),
            property: self.property.duplicate(),
        })
    }
}

fn box_primitive(
    environment: &Environment,
    value: Rc<RefCell<JsValue>>,
) -> Option<Rc<RefCell<Prototype>>> {
    let (constructor_name, properties) = match inline_borrow!(value.clone()) {
        JsValue::String(text) => {
            let mut properties = vec![
                (
                    "length".into(),
                    Rc::new(RefCell::new(JsValue::BigInt(text.chars().count() as i64))),
                ),
                (
                    PRIMITIVE_BOX_MARKER.into(),
                    Rc::new(RefCell::new(JsValue::Boolean(true))),
                ),
                (PRIMITIVE_BOX_VALUE.into(), value.clone()),
            ];
            for (index, character) in text.chars().enumerate() {
                properties.push((
                    JsValue::BigInt(index as i64),
                    Rc::new(RefCell::new(JsValue::String(character.to_string()))),
                ));
            }
            ("String", properties)
        }
        JsValue::Number(_) | JsValue::BigInt(_) => (
            "Number",
            vec![
                (
                    PRIMITIVE_BOX_MARKER.into(),
                    Rc::new(RefCell::new(JsValue::Boolean(true))),
                ),
                (PRIMITIVE_BOX_VALUE.into(), value.clone()),
            ],
        ),
        JsValue::Boolean(_) => (
            "Boolean",
            vec![
                (
                    PRIMITIVE_BOX_MARKER.into(),
                    Rc::new(RefCell::new(JsValue::Boolean(true))),
                ),
                (PRIMITIVE_BOX_VALUE.into(), value.clone()),
            ],
        ),
        _ => return None,
    };
    let constructor = Prototype::find(environment.mem.clone(), &constructor_name.into()).1;
    let prototype = constructor.borrow().unwrap_proto("primitive constructor");
    Some(Prototype::new_child(prototype, None, properties))
}
