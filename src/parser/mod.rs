use std::{cell::RefCell, rc::Rc};

use crate::{JsValue, Prototype, Runnable, new_runnable};

pub fn end_stack(text: &str) -> usize {
    let mut index: usize = 0;
    let mut in_str = false;
    let mut stack: usize = 0;
    let mut escaped = false;
    while index < text.len() {
        if escaped {
            escaped = !escaped;
        } else if text[index..=index] == *"\\" {
            escaped = true;
        } else if text[index..=index] == *"\"" || text[index..=index] == *"'" {
            in_str = !in_str;
        } else if in_str {
        } else if text[index..=index] == *"{" {
            stack += 1;
        } else if text[index..=index] == *"}" {
            if stack == 0 {
                return index;
            }
            stack -= 1;
        }
        index += 1;
    }
    index
}

pub fn end_ident(text: &str) -> usize {
    let mut index = 0;
    while index < text.len() && text.chars().nth(index).unwrap().is_alphanumeric() {
        index += 1;
    }
    index
}

pub fn parse(
    mem: Rc<RefCell<Prototype>>,
    parents: Vec<String>,
    mut file_content: &str,
) -> Option<Vec<Box<dyn Fn(Rc<RefCell<Prototype>>, &mut usize) -> (JsValue, Option<JsValue>)>>> {
    let mut codes: Vec<
        Box<dyn Fn(Rc<RefCell<Prototype>>, &mut usize) -> (JsValue, Option<JsValue>)>,
    > = Vec::new();

    file_content = file_content.trim_start();
    while !file_content.is_empty() {
        if file_content.starts_with("function ") {
            file_content = &file_content["function ".len()..];
            let end_name = end_ident(file_content);
            let name = file_content[..end_name].to_owned();
            file_content = &file_content[(file_content.find('(').unwrap() + 1)..];
            let mut params: Vec<String> = file_content[..file_content.find(')').unwrap()]
                .split(',')
                .into_iter()
                .map(|param| param.trim().to_owned())
                .collect();
            let excess = if let Some(param) = params.last()
                && param.starts_with("...")
            {
                Some(params.pop().unwrap()[3..].to_owned())
            } else {
                None
            };
            file_content = &file_content[(file_content.find('{').unwrap() + 1)..];
            let end_block = end_stack(file_content);
            let mut new_parents = parents.clone();
            new_parents.push(name.clone());
            let run_name = new_parents.join(".");
            mem.clone()
                .borrow_mut()
                .properties
                .insert(JsValue::String(name.clone()), JsValue::Undefined);
            let code = parse(
                Prototype::new_child(mem.clone(), None, []),
                new_parents,
                &file_content[..end_block],
            )
            .unwrap();
            file_content = &file_content[(end_block + 1)..];
            mem.clone().borrow_mut().properties.insert(
                JsValue::String(name),
                new_runnable(
                    Prototype::find(mem.clone(), &"Function".into())
                        .1
                        .unwrap_proto(),
                    Some(run_name.leak()),
                    Runnable {
                        params,
                        excess,
                        code,
                    },
                ),
            );
        } else if file_content.starts_with("let ") {
            file_content = &file_content["let ".len()..];
            add_let(&mem, file_content, &mut codes);
        } else if file_content.starts_with("const ") {
            file_content = &file_content["const ".len()..];
            add_let(&mem, file_content, &mut codes);
        } else if file_content.starts_with("var ") {
            file_content = &file_content["var ".len()..];
            let end_name = end_ident(file_content);
            let name = file_content[..end_name].to_owned();
            file_content = &file_content[(file_content.find('=').unwrap() + 1)..];
            let (end, code) = parse_expr(mem.clone(), file_content);
            file_content = &file_content[(end + 1)..];
            codes.push(Box::new(move |mem: Rc<RefCell<Prototype>>, i| {
                let (a, b) = code(mem.clone(), i);
                assert!(b.is_none(), "return in let?");
                let mut mem = mem;
                loop {
                    if let Some("stack memory") = mem.borrow().name {
                        break;
                    }
                    let t = mem.borrow().parent().unwrap();
                    mem = t;
                }
                mem.borrow_mut()
                    .properties
                    .insert(JsValue::String(name.clone()), a);
                (JsValue::Undefined, None)
            }));
        }
        file_content = file_content.trim_start();
    }

    if codes.is_empty() { None } else { Some(codes) }
}

fn add_let(
    mem: &Rc<RefCell<Prototype>>,
    mut file_content: &str,
    codes: &mut Vec<
        Box<dyn Fn(Rc<RefCell<Prototype>>, &mut usize) -> (JsValue, Option<JsValue>) + 'static>,
    >,
) {
    let end_name = end_ident(file_content);
    let name = file_content[..end_name].to_owned();
    file_content = &file_content[(file_content.find('=').unwrap() + 1)..];
    let (end, code) = parse_expr(mem.clone(), file_content);
    file_content = &file_content[(end + 1)..];
    codes.push(Box::new(move |mem: Rc<RefCell<Prototype>>, i| {
        let (a, b) = code(mem.clone(), i);
        assert!(b.is_none(), "return in let?");
        mem.borrow_mut()
            .properties
            .insert(JsValue::String(name.clone()), a);
        (JsValue::Undefined, None)
    }));
}

pub fn parse_expr(
    mem: Rc<RefCell<Prototype>>,
    mut file_content: &str,
) -> (
    usize,
    Box<dyn Fn(Rc<RefCell<Prototype>>, &mut usize) -> (JsValue, Option<JsValue>)>,
) {
    todo!()
}
