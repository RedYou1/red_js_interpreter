use std::{cell::RefCell, rc::Rc};

use super::expand_spread;
use crate::{
    Code, CodeIndex, CodeResult, Environment, JsValue, LogLevel, Prototype, inline_borrow,
    new_array,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::Parser,
    },
    run_sub,
};

const OBJECT_SPREAD_MARKER: &str = "__red_js_object_spread__";

#[derive(Debug)]
struct BindingProperty {
    property: String,
    binding: String,
    default: Option<Box<dyn Expr>>,
    nested: Option<Box<dyn Expr>>,
}

#[derive(Debug)]
pub struct ObjectDestructure {
    properties: Vec<BindingProperty>,
    rest: Option<String>,
    initializer: Box<dyn Expr>,
}

impl ObjectDestructure {
    pub fn parse(parser: &mut Parser) -> Self {
        let (properties, rest) = parse_binding_pattern(parser);
        if parser.tokens()[parser.index()] != Token::Assign(None) {
            panic!("object destructuring declaration requires an initializer");
        }
        parser.bump();
        let initializer = Box::new(parser.parse_expression(false));
        Self {
            properties,
            rest,
            initializer,
        }
    }

    pub fn parse_parameter(parser: &mut Parser, parameter: String) -> Self {
        let (properties, rest) = parse_binding_pattern(parser);
        Self {
            properties,
            rest,
            initializer: Box::new(expr::Identifier { name: parameter }),
        }
    }
}

fn parse_binding_pattern(parser: &mut Parser) -> (Vec<BindingProperty>, Option<String>) {
    if parser.tokens()[parser.index()] != Token::LBrace {
        panic!("expected '{{' in object destructuring pattern");
    }
    parser.bump();
    let mut properties = Vec::new();
    let mut rest = None;
    while !matches!(parser.tokens()[parser.index()], Token::RBrace | Token::Eof) {
        if parser.tokens()[parser.index()] == Token::Spread {
            parser.bump();
            rest = Some(parser.expect_ident());
            if parser.tokens()[parser.index()] == Token::Comma {
                parser.bump();
            }
            break;
        }
        properties.push(parse_binding_property(parser));
        if parser.tokens()[parser.index()] == Token::Comma {
            parser.bump();
        } else {
            break;
        }
    }
    if parser.tokens()[parser.index()] != Token::RBrace {
        panic!("expected '}}' in object destructuring pattern");
    }
    parser.bump();
    (properties, rest)
}

fn parse_binding_property(parser: &mut Parser) -> BindingProperty {
    let property = parser.tokens()[parser.index()].as_keyword().to_owned();
    parser.bump();
    let mut nested = None;
    let binding = if parser.tokens()[parser.index()] == Token::Colon {
        parser.bump();
        match parser.tokens()[parser.index()] {
            Token::LBrace => {
                let name = format!("__red_nested_destructure_{}__", parser.index());
                nested = Some(
                    Box::new(ObjectDestructure::parse_parameter(parser, name.clone()))
                        as Box<dyn Expr>,
                );
                name
            }
            Token::LBracket => {
                let name = format!("__red_nested_destructure_{}__", parser.index());
                nested = Some(Box::new(expr::ArrayDestructure::parse_parameter(
                    parser,
                    name.clone(),
                )) as Box<dyn Expr>);
                name
            }
            _ => parser.expect_ident(),
        }
    } else {
        property.clone()
    };
    let default: Option<Box<dyn Expr>> = if parser.tokens()[parser.index()] == Token::Assign(None) {
        parser.bump();
        Some(Box::new(parser.parse_expression(false)))
    } else {
        None
    };
    BindingProperty {
        property,
        binding,
        default,
        nested,
    }
}

impl Expr for ObjectDestructure {
    fn compile(&self, environment: Environment) -> Vec<Code> {
        let initializer = self.initializer.compile(environment.clone());
        let properties = self
            .properties
            .iter()
            .map(|property| {
                (
                    property.property.clone(),
                    property.binding.clone(),
                    property
                        .default
                        .as_ref()
                        .map(|value| value.compile(environment.clone())),
                    property
                        .nested
                        .as_ref()
                        .map(|value| value.compile(environment.clone())),
                )
            })
            .collect::<Vec<_>>();
        let rest = self.rest.clone();
        vec![Box::new(move |environment, _| {
            let source = match run_sub(&initializer, environment.clone(), &mut CodeIndex::new()) {
                CodeResult::Normal(value) | CodeResult::NormalMember(value, _, _) => value,
                result => return result,
            };
            let source_object = match inline_borrow!(source.clone()) {
                JsValue::Prototype(source) => source,
                JsValue::Null | JsValue::Undefined => {
                    return CodeResult::Error(Rc::new(RefCell::new(JsValue::String(
                        "cannot destructure null or undefined".to_owned(),
                    ))));
                }
                _ => {
                    return CodeResult::Error(Rc::new(RefCell::new(JsValue::String(
                        "object destructuring source is not an object".to_owned(),
                    ))));
                }
            };
            let mut used_properties = Vec::new();
            for (property, binding, default, nested) in &properties {
                let mut value = Prototype::find(source_object.clone(), &property.as_str().into()).1;
                if matches!(inline_borrow!(value.clone()), JsValue::Undefined)
                    && let Some(default) = default
                {
                    value = match run_sub(default, environment.clone(), &mut CodeIndex::new()) {
                        CodeResult::Normal(value) | CodeResult::NormalMember(value, _, _) => value,
                        result => return result,
                    };
                }
                environment
                    .mem
                    .borrow_mut()
                    .properties
                    .insert(binding.as_str().into(), value);
                if let Some(nested) = nested {
                    match run_sub(nested, environment.clone(), &mut CodeIndex::new()) {
                        CodeResult::Normal(_) | CodeResult::NormalMember(_, _, _) => {}
                        result => return result,
                    }
                }
                used_properties.push(JsValue::String(property.clone()));
            }
            if let Some(rest) = &rest {
                let object_prototype =
                    Prototype::find(environment.mem.clone(), &stringify!(Object).into())
                        .1
                        .borrow()
                        .unwrap_proto("object destructuring rest prototype");
                let rest_properties = source_object
                    .borrow()
                    .properties
                    .iter()
                    .filter(|(key, _)| {
                        !used_properties.contains(key)
                            && !source_object.borrow().non_enumerable.contains(*key)
                            && key != &&"__proto__".into()
                    })
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<Vec<_>>();
                let rest_object = Prototype::new_child(object_prototype, None, rest_properties);
                environment.mem.borrow_mut().properties.insert(
                    rest.as_str().into(),
                    Rc::new(RefCell::new(JsValue::Prototype(rest_object))),
                );
            }
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            properties: self
                .properties
                .iter()
                .map(|property| BindingProperty {
                    property: property.property.clone(),
                    binding: property.binding.clone(),
                    default: property.default.as_ref().map(|value| value.duplicate()),
                    nested: property.nested.as_ref().map(|value| value.duplicate()),
                })
                .collect(),
            rest: self.rest.clone(),
            initializer: self.initializer.duplicate(),
        })
    }
}

#[derive(Debug)]
struct ArrayBinding {
    name: String,
    default: Option<Box<dyn Expr>>,
}

type CompiledArrayBinding = Option<(String, Option<Vec<Code>>)>;

#[derive(Debug)]
pub struct ArrayDestructure {
    elements: Vec<Option<ArrayBinding>>,
    rest: Option<String>,
    initializer: Box<dyn Expr>,
}

impl ArrayDestructure {
    pub fn parse(parser: &mut Parser) -> Self {
        let (elements, rest) = parse_array_pattern(parser);
        if parser.tokens()[parser.index()] != Token::Assign(None) {
            panic!("array destructuring declaration requires an initializer");
        }
        parser.bump();
        Self {
            elements,
            rest,
            initializer: Box::new(parser.parse_expression(false)),
        }
    }

    pub fn parse_parameter(parser: &mut Parser, parameter: String) -> Self {
        let (elements, rest) = parse_array_pattern(parser);
        Self {
            elements,
            rest,
            initializer: Box::new(expr::Identifier { name: parameter }),
        }
    }
}

fn parse_array_pattern(parser: &mut Parser) -> (Vec<Option<ArrayBinding>>, Option<String>) {
    if parser.tokens()[parser.index()] != Token::LBracket {
        panic!("expected '[' in array destructuring pattern");
    }
    parser.bump();
    let mut elements = Vec::new();
    let mut rest = None;
    while !matches!(
        parser.tokens()[parser.index()],
        Token::RBracket | Token::Eof
    ) {
        if parser.tokens()[parser.index()] == Token::Comma {
            elements.push(None);
            parser.bump();
            continue;
        }
        if parser.tokens()[parser.index()] == Token::Spread {
            parser.bump();
            rest = Some(parser.expect_ident());
            if parser.tokens()[parser.index()] == Token::Comma {
                parser.bump();
            }
            break;
        }
        let name = parser.expect_ident();
        let default = if parser.tokens()[parser.index()] == Token::Assign(None) {
            parser.bump();
            Some(Box::new(parser.parse_expression(false)) as Box<dyn Expr>)
        } else {
            None
        };
        elements.push(Some(ArrayBinding { name, default }));
        if parser.tokens()[parser.index()] == Token::Comma {
            parser.bump();
        } else {
            break;
        }
    }
    if parser.tokens()[parser.index()] != Token::RBracket {
        panic!("expected ']' in array destructuring pattern");
    }
    parser.bump();
    (elements, rest)
}

impl Expr for ArrayDestructure {
    fn compile(&self, environment: Environment) -> Vec<Code> {
        let initializer = self.initializer.compile(environment.clone());
        let elements = self
            .elements
            .iter()
            .map(|element| {
                element.as_ref().map(|binding| {
                    (
                        binding.name.clone(),
                        binding
                            .default
                            .as_ref()
                            .map(|value| value.compile(environment.clone())),
                    )
                })
            })
            .collect::<Vec<CompiledArrayBinding>>();
        let rest = self.rest.clone();
        vec![Box::new(move |environment, _| {
            let source = match run_sub(&initializer, environment.clone(), &mut CodeIndex::new()) {
                CodeResult::Normal(value) | CodeResult::NormalMember(value, _, _) => value,
                result => return result,
            };
            let values = match array_destructure_values(&source) {
                Ok(values) => values,
                Err(message) => {
                    return CodeResult::Error(Rc::new(RefCell::new(JsValue::String(message))));
                }
            };
            if let Some(result) = bind_array_elements(&environment, &elements, &values) {
                return result;
            }
            if let Some(rest) = &rest {
                let array_prototype =
                    Prototype::find(environment.mem.clone(), &stringify!(Array).into())
                        .1
                        .borrow()
                        .unwrap_proto("array destructuring rest prototype");
                let remaining = values.into_iter().skip(elements.len()).collect();
                let rest_array = new_array(array_prototype, remaining, environment.logger.clone());
                environment
                    .mem
                    .borrow_mut()
                    .properties
                    .insert(rest.as_str().into(), rest_array);
            }
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            elements: self
                .elements
                .iter()
                .map(|element| {
                    element.as_ref().map(|binding| ArrayBinding {
                        name: binding.name.clone(),
                        default: binding.default.as_ref().map(|value| value.duplicate()),
                    })
                })
                .collect(),
            rest: self.rest.clone(),
            initializer: self.initializer.duplicate(),
        })
    }
}

fn bind_array_elements(
    environment: &Environment,
    elements: &[CompiledArrayBinding],
    values: &[Rc<RefCell<JsValue>>],
) -> Option<CodeResult> {
    for (index, element) in elements.iter().enumerate() {
        let Some((name, default)) = element else {
            continue;
        };
        let mut value = values
            .get(index)
            .cloned()
            .unwrap_or_else(|| Rc::new(RefCell::new(JsValue::Undefined)));
        if matches!(inline_borrow!(value.clone()), JsValue::Undefined)
            && let Some(default) = default
        {
            value = match run_sub(default, environment.clone(), &mut CodeIndex::new()) {
                CodeResult::Normal(value) | CodeResult::NormalMember(value, _, _) => value,
                result => return Some(result),
            };
        }
        environment
            .mem
            .borrow_mut()
            .properties
            .insert(name.as_str().into(), value);
    }
    None
}

fn array_destructure_values(
    source: &Rc<RefCell<JsValue>>,
) -> Result<Vec<Rc<RefCell<JsValue>>>, String> {
    match inline_borrow!(source.clone()) {
        JsValue::String(text) => Ok(text
            .chars()
            .map(|character| Rc::new(RefCell::new(JsValue::String(character.to_string()))))
            .collect()),
        JsValue::Prototype(array) => {
            let length = Prototype::find(array.clone(), &"length".into()).1;
            let JsValue::BigInt(length) = inline_borrow!(length) else {
                return Err("array destructuring source has no length".to_owned());
            };
            Ok((0..length.max(0))
                .map(|index| Prototype::find(array.clone(), &JsValue::BigInt(index)).1)
                .collect())
        }
        JsValue::Null | JsValue::Undefined => {
            Err("cannot destructure null or undefined".to_owned())
        }
        _ => Err("array destructuring source is not iterable".to_owned()),
    }
}

#[derive(Debug)]
pub struct Object {
    pub properties: Vec<(Box<dyn Expr>, Box<dyn Expr>)>,
}

impl Object {
    pub fn parse(parser: &mut Parser) -> Self {
        parser.bump();
        let mut properties: Vec<(Box<dyn Expr>, Box<dyn Expr>)> = Vec::new();
        while !matches!(parser.tokens()[parser.index()], Token::RBrace | Token::Eof) {
            if parser.tokens()[parser.index()] == Token::Spread {
                parser.bump();
                let source = Box::new(parser.parse_expression(false));
                properties.push((
                    Box::new(expr::ConstString {
                        s: OBJECT_SPREAD_MARKER.to_owned(),
                    }),
                    source,
                ));
                if parser.tokens()[parser.index()] == Token::Comma {
                    parser.bump();
                }
                continue;
            }
            let start = if let Token::Star = parser.tokens()[parser.index()] {
                parser.bump();
                true
            } else {
                false
            };
            let key: Box<dyn Expr> = if let Token::Ident(value) = &parser.tokens()[parser.index()] {
                let key = Box::new(expr::ConstString { s: value.clone() });
                parser.bump();
                key
            } else {
                //false
                parser.parse_primary()
            };
            match parser.tokens()[parser.index()] {
                Token::Colon => {
                    parser.bump();
                    let value = Box::new(parser.parse_expression(false));
                    properties.push((key, value));
                }
                Token::LParen => {
                    let mut func = expr::FunctionDecl::parse(parser, false);
                    func.generator = start;
                    properties.push((key, Box::new(func)));
                }
                _ => {
                    parser.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                        format!(
                            "Object::parse expected ':' or '(' at index {} but found {:?}",
                            parser.index(),
                            parser.tokens()[parser.index()]
                        )
                    });
                    panic!(
                        "expected ':' or '(' in object literal, got {:?}",
                        parser.tokens()[parser.index()]
                    )
                }
            }
            if let Token::Comma = parser.tokens()[parser.index()] {
                parser.bump();
            }
        }
        if parser.tokens()[parser.index()] != Token::RBrace {
            parser.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                format!(
                    "Object::parse expected '}}' at index {} but found {:?}",
                    parser.index(),
                    parser.tokens()[parser.index()]
                )
            });
            panic!("expected '}}' at end of object literal");
        }
        parser.bump();
        Self { properties }
    }
}

impl Expr for Object {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let elems: Vec<(Vec<Code>, Vec<Code>)> = self
            .properties
            .iter()
            .map(|(key, value)| (key.compile(env.clone()), value.compile(env.clone())))
            .collect();
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Object");
            let object_proto = Prototype::find(env.mem.clone(), &stringify!(Object).into())
                .1
                .borrow()
                .unwrap_proto("expr::Object for Object");
            let mut props: Vec<(JsValue, Rc<RefCell<JsValue>>)> = Vec::new();
            for (key, value) in elems.iter() {
                let key_val = match run_sub(key, env.clone(), &mut CodeIndex::new()) {
                    CodeResult::Normal(res) => inline_borrow!(res),
                    CodeResult::NormalMember(res, _, _) => inline_borrow!(res),
                    e => return e,
                };
                let value_val = match run_sub(value, env.clone(), &mut CodeIndex::new()) {
                    CodeResult::Normal(res) => res,
                    CodeResult::NormalMember(res, _, _) => res,
                    e => return e,
                };
                if key_val == JsValue::String(OBJECT_SPREAD_MARKER.to_owned()) {
                    let JsValue::Prototype(source) = inline_borrow!(value_val) else {
                        continue;
                    };
                    let source = source.borrow();
                    for (key, value) in &source.properties {
                        if key != &"__proto__".into() && !source.non_enumerable.contains(key) {
                            props.push((key.clone(), value.clone()));
                        }
                    }
                } else {
                    props.push((key_val, value_val));
                }
            }
            let out = Rc::new(RefCell::new(JsValue::Prototype(Prototype::new_child(
                object_proto,
                None,
                props,
            ))));
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::Object result={:?}", out)
            });
            CodeResult::Normal(out)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            properties: self
                .properties
                .iter()
                .map(|(a, b)| (a.duplicate(), b.duplicate()))
                .collect(),
        })
    }
}

#[derive(Debug)]
pub struct Array {
    pub elems: Vec<Box<dyn Expr>>,
}

impl Expr for Array {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let elems: Vec<Vec<Code>> = self
            .elems
            .iter()
            .map(|elem| elem.compile(env.clone()))
            .collect();
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Array");
            let array_proto = Prototype::find(env.mem.clone(), &stringify!(Array).into())
                .1
                .borrow()
                .unwrap_proto("expr::Array for Array");
            let mut values: Vec<Rc<RefCell<JsValue>>> = Vec::new();
            for elem in elems.iter() {
                match run_sub(elem, env.clone(), &mut CodeIndex::new()) {
                    CodeResult::Normal(res) | CodeResult::NormalMember(res, _, _) => {
                        if let Some(spread) = expand_spread(&res) {
                            values.extend(spread);
                        } else {
                            values.push(res);
                        }
                    }
                    e => return e,
                }
            }
            let out = new_array(array_proto, values, env.logger.clone());
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::Array result={:?}", out)
            });
            CodeResult::Normal(out)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            elems: self.elems.iter().map(|a| a.duplicate()).collect(),
        })
    }
}
