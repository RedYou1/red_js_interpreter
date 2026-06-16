use crate::{logln, LogLevel};
use crate::prebuild::prelude::*;

#[cfg(test)]
pub const CONSOLE_LOGS: &str = "__$G%RH^&$%E$WG#ESOVBT__";

/// %o Outputs a JavaScript object.<br>
/// %d or %i Outputs an integer.<br>
/// %s Outputs a string.<br>
/// %f Outputs a floating-point value.<br>
pub fn default_console_config(mem: Rc<RefCell<Prototype>>) -> Rc<RefCell<Prototype>> {
    let function = Prototype::find(mem.clone(), &"Function".into())
        .1
        .borrow()
        .unwrap_proto("default_console_config for Function");
    let simple = new_runnable(
        function.clone(),
        "console.format.simple",
        prebuild_runnable(
            mem.clone(),
            Box::new(|_, _, [value]| {
                Rc::new(RefCell::new(JsValue::String(value.borrow().print())))
            }),
        ),
    );
    let digit = new_runnable(
        function.clone(),
        "console.format.digit",
        prebuild_runnable(
            mem.clone(),
            Box::new(|_, _, [value]| {
                Rc::new(RefCell::new(JsValue::String(match inline_borrow!(value) {
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
                })))
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
                    "console.format.float",
                    prebuild_runnable(
                        mem.clone(),
                        Box::new(|_, _, [value]| {
                            Rc::new(RefCell::new(JsValue::String(match inline_borrow!(value) {
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
                            })))
                        }),
                    ),
                ),
            ),
        ]),
    }))
}

#[cfg(test)]
fn push_to_logs(console: Rc<RefCell<Prototype>>, text: String) {
    let JsValue::BigInt(vec_ptr) =
        *Prototype::find(console, &JsValue::String(CONSOLE_LOGS.to_owned()))
            .1
            .borrow()
    else {
        return;
    };
    let vec: &mut Vec<String> = unsafe { (vec_ptr as *mut Vec<String>).as_mut_unchecked() };
    vec.push(text);
}

new_class!(
    prebuild_console,
    console,
    Object,
    __config__,JsValue::Null;
    log, fn_direct, |_, this, arguments| {
        if arguments.is_empty() {
            println!();

            #[cfg(test)]
            push_to_logs(this.borrow().unwrap_proto("Console.log for this"), "".to_owned());
        }else if let Some(JsValue::String(format)) = arguments.first().map(|t| inline_borrow!(t)) {
            let config = Prototype::find(this.borrow().unwrap_proto("Console.log for this"), &JsValue::String("__config__".to_owned())).1.borrow().unwrap_proto("Console.log for __config__");
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

                let JsValue::Prototype(ref formater) = inline_borrow!(Prototype::find(config.clone(), &JsValue::String(formater.clone())).1) else {panic!("console formater {formater} not found")};
                let JsValue::String(ref res) = inline_borrow!(run_function_object(formater.clone(), Rc::new(RefCell::new(JsValue::Undefined)), vec![arguments[argi].clone()])) else {panic!("console formater didnt returned a string")};
                argi += 1;
                text += res;
            }

            logln(LogLevel::Trace, &format!("console.log formatted output={text}"));
            println!("{text}");

            #[cfg(test)]
            push_to_logs(this.borrow().unwrap_proto("Console.log for this"), text);
        } else {
            let text = arguments.iter().map(|t| t.borrow().print()).collect::<Vec<String>>().join(" ");

            logln(LogLevel::Trace, &format!("console.log output={text}"));
            println!("{text}");

            #[cfg(test)]
            push_to_logs(this.borrow().unwrap_proto("Console.log for this"), text);
        };
        Rc::new(RefCell::new(JsValue::Undefined))
    };
);
