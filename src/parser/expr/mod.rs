use std::{cell::RefCell, fmt::Debug, mem::MaybeUninit, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, JsValue, LogLevel, Prototype, handle_return,
    inline_borrow, parser::parser::Parser, run_sub,
};

pub trait Expr: Debug {
    fn compile(&self, env: Environment) -> Vec<Code>;
    fn duplicate(&self) -> Box<dyn Expr>;

    fn is_assignment_target(&self) -> bool {
        false
    }

    fn is_assignment_pattern_element(&self) -> bool {
        self.is_assignment_target()
    }

    fn is_rest_element(&self) -> bool {
        false
    }

    fn is_optional_chain(&self) -> bool {
        false
    }
}

impl Expr for Option<Box<dyn Expr>> {
    fn compile(&self, env: Environment) -> Vec<Code> {
        if let Some(e) = self {
            e.compile(env)
        } else {
            vec![Box::new(|_, _| {
                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            })]
        }
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(self.as_ref().map(|a| Expr::duplicate(a.as_ref())))
    }
}

impl<const LEN: usize> Expr for [Box<dyn Expr>; LEN] {
    fn compile(&self, env: Environment) -> Vec<Code> {
        self.iter()
            .flat_map(|code| code.compile(env.clone()))
            .collect()
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        let mut t = [const { MaybeUninit::<Box<dyn Expr>>::uninit() }; LEN];
        for (v, t) in self.iter().zip(t.iter_mut()) {
            t.write(v.as_ref().duplicate());
        }
        Box::new(unsafe { MaybeUninit::array_assume_init(t) })
    }
}

impl Expr for Vec<Box<dyn Expr>> {
    fn compile(&self, env: Environment) -> Vec<Code> {
        self.iter()
            .flat_map(|stmt| stmt.compile(env.clone()))
            .collect()
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(
            self.iter()
                .map(|a| a.as_ref().duplicate())
                .collect::<Self>(),
        )
    }
}

mod assign;
mod call;
mod class_decl;
mod consts;
mod function_decl;
mod if_decl;
mod loop_decl;
mod member;
mod new;
mod object;
mod operators;
mod postfix;
mod returns;
mod switch_decl;
mod template_str;
mod try_catch;

pub use {
    assign::*, call::*, class_decl::*, consts::*, function_decl::*, if_decl::*, loop_decl::*,
    member::*, new::*, object::*, operators::*, postfix::*, returns::*, switch_decl::*,
    template_str::*, try_catch::*,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
}

const SPREAD_MARKER: &str = "__red_js_spread__";

#[derive(Debug)]
pub struct Spread {
    pub expression: Box<dyn Expr>,
}

#[derive(Debug)]
pub struct DefaultParameter {
    pub name: String,
    pub default: Box<dyn Expr>,
}

pub(crate) fn infer_anonymous_function_name(value: &Rc<RefCell<JsValue>>, name: &str) {
    let JsValue::Prototype(function) = inline_borrow!(value.clone()) else {
        return;
    };
    let Some((_, name_property)) = Prototype::opt_find(function, &"name".into()) else {
        return;
    };
    if matches!(
        inline_borrow!(name_property.clone()),
        JsValue::String(current) if current == "anonymous function"
    ) {
        *name_property.borrow_mut() = JsValue::String(name.to_owned());
    }
}

impl Expr for DefaultParameter {
    fn compile(&self, environment: Environment) -> Vec<Code> {
        let name = self.name.clone();
        let default = self.default.compile(environment.clone());
        vec![Box::new(move |environment, _| {
            let binding = Prototype::find(environment.mem.clone(), &name.as_str().into()).1;
            if matches!(inline_borrow!(binding.clone()), JsValue::Undefined) {
                let value = match run_sub(&default, environment.clone(), &mut CodeIndex::new()) {
                    CodeResult::Normal(value) | CodeResult::NormalMember(value, _, _) => value,
                    result => return result,
                };
                infer_anonymous_function_name(&value, &name);
                environment
                    .mem
                    .borrow_mut()
                    .properties
                    .insert(name.as_str().into(), value);
            }
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            name: self.name.clone(),
            default: self.default.duplicate(),
        })
    }
}

impl Expr for Spread {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let expression = self.expression.compile(env);
        vec![Box::new(move |env, _| {
            let source = match run_sub(&expression, env.clone(), &mut CodeIndex::new()) {
                CodeResult::Normal(value) | CodeResult::NormalMember(value, _, _) => value,
                result => return result,
            };
            let object_prototype = Prototype::find(env.mem.clone(), &stringify!(Object).into())
                .1
                .borrow()
                .unwrap_proto("Spread wrapper for Object");
            let spread = Prototype::new_child(
                object_prototype,
                None,
                [
                    (
                        SPREAD_MARKER.into(),
                        Rc::new(RefCell::new(JsValue::Boolean(true))),
                    ),
                    ("value".into(), source),
                ],
            );
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Prototype(spread))))
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            expression: self.expression.duplicate(),
        })
    }

    fn is_assignment_pattern_element(&self) -> bool {
        self.expression.is_assignment_target()
    }

    fn is_rest_element(&self) -> bool {
        true
    }
}

pub(crate) fn expand_spread(value: &Rc<RefCell<JsValue>>) -> Option<Vec<Rc<RefCell<JsValue>>>> {
    let JsValue::Prototype(wrapper) = inline_borrow!(value.clone()) else {
        return None;
    };
    let marker = Prototype::find(wrapper.clone(), &SPREAD_MARKER.into()).1;
    if !matches!(inline_borrow!(marker), JsValue::Boolean(true)) {
        return None;
    }
    let source = Prototype::find(wrapper, &"value".into()).1;
    match inline_borrow!(source.clone()) {
        JsValue::String(text) => Some(
            text.chars()
                .map(|character| Rc::new(RefCell::new(JsValue::String(character.to_string()))))
                .collect(),
        ),
        JsValue::Prototype(iterable) => {
            let length = Prototype::find(iterable.clone(), &"length".into()).1;
            let JsValue::BigInt(length) = inline_borrow!(length) else {
                panic!("spread value is not an array or string")
            };
            Some(
                (0..length.max(0))
                    .map(|index| Prototype::find(iterable.clone(), &JsValue::BigInt(index)).1)
                    .collect(),
            )
        }
        _ => panic!("spread value is not iterable"),
    }
}

impl Expr for Identifier {
    fn compile(&self, _: Environment) -> Vec<Code> {
        let name = self.name.clone();
        vec![Box::new(move |env, _| {
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Entering Expr::Identifier name={}", name)
            });
            let res = Prototype::find(env.mem.clone(), &name.as_str().into()).1;
            if name == "super" {
                let this = Prototype::find(env.mem, &"this".into()).1;
                env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                    format!("Exiting Expr::Identifier {name:?} result={res:?} of {this:?}")
                });
                CodeResult::NormalMember(
                    res,
                    inline_borrow!(this)
                        .unwrap_proto("expr::Identifier this is suposed to be proto"),
                    Rc::new(RefCell::new("this".into())),
                )
            } else {
                env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                    format!("Exiting Expr::Identifier {name:?} result={res:?}")
                });
                CodeResult::NormalMember(res, env.mem, Rc::new(RefCell::new(name.as_str().into())))
            }
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(self.clone())
    }

    fn is_assignment_target(&self) -> bool {
        !matches!(self.name.as_str(), "this" | "super")
    }
}

#[derive(Debug)]
pub struct Typeof {
    pub obj: Box<dyn Expr>,
}

impl Typeof {
    pub fn parse(parser: &mut Parser) -> Self {
        let expr = parser.parse_call_or_primary(false);
        Self { obj: expr }
    }
}

impl Expr for Typeof {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let obj = self.obj.compile(env);
        vec![Box::new(move |env, _| {
            if obj.len() > 1 {
                handle_return!(run_sub(
                    &obj[..(obj.len() - 1)],
                    env.clone(),
                    &mut CodeIndex::new()
                ));
            }
            let t = handle_return!(obj[obj.len() - 1](env, &mut CodeIndex::new()));
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::String(
                match inline_borrow!(t) {
                    JsValue::Function(_) => "function",
                    JsValue::Generator(_) => "function",
                    JsValue::RedGex(_) => "object",
                    JsValue::Prototype(proto) => {
                        if proto.borrow().name.is_some() {
                            "function"
                        } else {
                            "object"
                        }
                    }
                    JsValue::Symbol(_, _) => "symbol",
                    JsValue::String(_) => "string",
                    JsValue::Number(_) | JsValue::BigInt(_) => "number",
                    JsValue::Boolean(_) => "boolean",
                    JsValue::Undefined => "undefined",
                    JsValue::Null => "object",
                }
                .to_owned(),
            ))))
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            obj: self.obj.duplicate(),
        })
    }
}

#[derive(Debug)]
pub struct Label {
    pub name: String,
    pub code: Vec<Box<dyn Expr>>,
}

impl Expr for Label {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let codes = self.code.compile(env);
        let wanted_name = self.name.clone();
        vec![Box::new(move |env, _| {
            match run_sub(&codes, env, &mut CodeIndex::new()) {
                CodeResult::Break(None) => {
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }
                CodeResult::Break(Some(name)) if wanted_name.eq(name.as_str()) => {
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }
                CodeResult::Continue(None) => {
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }
                CodeResult::Continue(Some(name)) if wanted_name.eq(name.as_str()) => {
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }
                a => a,
            }
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            name: self.name.clone(),
            code: self.code.iter().map(|a| a.duplicate()).collect(),
        })
    }
}
