#[cfg(test)]
use std::cell::LazyCell;

use crate::prebuild::prelude::*;

#[cfg(test)]
pub static mut CONSOLE: LazyCell<Vec<String>> = LazyCell::new(|| Vec::new());

/// %o Outputs a JavaScript object.<br>
/// %d or %i Outputs an integer.<br>
/// %s Outputs a string.<br>
/// %f Outputs a floating-point value.<br>
pub fn default_console_config(mem: Rc<RefCell<Prototype>>) -> Rc<RefCell<Prototype>> {
    let function = Prototype::find(mem.clone(), &"Function".into())
        .1
        .unwrap_proto();
    let simple = new_runnable(
        function.clone(),
        None,
        prebuild_runnable(
            mem.clone(),
            Box::new(|_, _, [value]| JsValue::String(value.print())),
        ),
    );
    let digit = new_runnable(
        function.clone(),
        None,
        prebuild_runnable(
            mem.clone(),
            Box::new(|_, _, [value]| {
                JsValue::String(match value {
                    JsValue::BigInt(d) => d.to_string(),
                    JsValue::Number(n) => format!("{:.0}", n.floor()),
                    JsValue::Boolean(b) => {
                        if b {
                            "1".to_owned()
                        } else {
                            "0".to_owned()
                        }
                    }
                    value => panic!("to_string format not an integer: {}", value.print()),
                })
            }),
        ),
    );
    Rc::new(RefCell::new(Prototype {
        name: None,
        properties: HashMap::from([
            (JsValue::String("o".to_owned()), simple.clone()),
            (JsValue::String("d".to_owned()), digit.clone()),
            (JsValue::String("i".to_owned()), digit),
            (JsValue::String("s".to_owned()), simple),
            (
                JsValue::String("f".to_owned()),
                new_runnable(
                    function.clone(),
                    None,
                    prebuild_runnable(
                        mem.clone(),
                        Box::new(|_, _, [value]| {
                            JsValue::String(match value {
                                JsValue::BigInt(d) => (d as f64).to_string(),
                                JsValue::Number(n) => n.to_string(),
                                JsValue::Boolean(b) => {
                                    if b {
                                        "1.0".to_owned()
                                    } else {
                                        "0.0".to_owned()
                                    }
                                }
                                value => {
                                    panic!("to_string format not an integer: {}", value.print())
                                }
                            })
                        }),
                    ),
                ),
            ),
        ]),
    }))
}

new_class!(
    prebuild_console,
    console,
    Object,
    __config__,JsValue::Null;
    log, fn_direct, |mem, this, arguments: Vec<JsValue>| {
        if arguments.is_empty() {
            println!();

            #[cfg(test)]
            unsafe{
                #[expect(static_mut_refs)]
                CONSOLE.push("".to_owned());
            }
        }else if let JsValue::String(format) = arguments.first().unwrap() {
            let config = Prototype::find(this.unwrap_proto(), &JsValue::String("__config__".to_owned())).1.unwrap_proto();
            let mut text = String::new();
            let mut argi:usize = 1;

            let mut format = format.as_str();
            while !format.is_empty(){
                let i = format.find('%');
                if let Some(i) = i {
                    if i > 0 {
                        text += &format[..i];
                    }
                    format = &format[(i+1)..];
                }else {
                    text += format;
                    break;
                }
                if format.is_empty() {
                    panic!("console format end with %");
                }
                let formater = format[0..=0].to_owned();
                format = &format[1..];

                if formater == "%" {
                    text += "%";
                    continue;
                }

                let JsValue::Prototype(formater) = Prototype::find(config.clone(), &JsValue::String(formater.clone())).1 else {panic!("console formater {formater} not found")};
                let JsValue::String(ref res) = run_function_object(mem.clone(), formater, JsValue::Undefined, vec![arguments[argi].clone()]) else {panic!("console formater didnt returned a string")};
                argi += 1;
                text += res;
            }

            println!("{text}");

            #[cfg(test)]
            unsafe{
                #[expect(static_mut_refs)]
                CONSOLE.push(text.to_owned());
            }
        } else {
            let text = arguments.iter().map(JsValue::print).collect::<Vec<String>>().join(" ");

            println!("{text}");

            #[cfg(test)]
            unsafe{
                #[expect(static_mut_refs)]
                CONSOLE.push(text);
            }
        };
        JsValue::Undefined
    };
);
