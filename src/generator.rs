use std::{cell::RefCell, fmt::Debug, rc::Rc};

use crate::{ARGUMENTS, Code, CodeIndex, CodeResult, JsValue, LogLevel, PROTOTYPE_NAME, Prototype, RUNNABLE, inline_borrow, new_array};


pub struct Generator {
    pub params: Vec<String>,
    pub excess: Option<String>,
    pub code: Rc<Vec<Code>>,
    pub mem: Rc<RefCell<Prototype>>,
}

pub struct IterGenerator {
    index: CodeIndex,
    proto: Rc<RefCell<Prototype>>,
    code: Rc<Vec<Code>>,
}
impl Iterator for IterGenerator {
    type Item = Rc<RefCell<JsValue>>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index.current() < self.code.len() {
            let res = self.code[self.index.current()](self.proto.clone(), &mut self.index);
            match res {
                CodeResult::Return(_) => panic!("return in generator?"),
                CodeResult::Yield(res) => {
                    self.index.set_retry();
                    return Some(res);
                }
                CodeResult::YieldBreak => {
                    self.index.goto_end();
                    return None;
                }
                _ => {}
            }
            self.index.next();
        }
        None
    }
}

pub fn new_generator(
    function: Rc<RefCell<Prototype>>,
    name: &'static str,
    runnable: Generator,
) -> Rc<RefCell<JsValue>> {
    Rc::new(RefCell::new(JsValue::Prototype(Prototype::new_child(
        function.clone(),
        Some(name),
        [
            (
                PROTOTYPE_NAME.into(),
                Rc::new(RefCell::new(JsValue::Prototype(function.clone()))),
            ),
            (
                RUNNABLE.into(),
                Rc::new(RefCell::new(JsValue::Generator(Rc::new(runnable)))),
            ),
        ],
    ))))
}

pub fn run_generator_object(
    func: Rc<RefCell<Prototype>>,
    this: Rc<RefCell<JsValue>>,
    params: Vec<Rc<RefCell<JsValue>>>,
) -> IterGenerator {
    let JsValue::Generator(ref runnable) =
        inline_borrow!(inline_borrow!(func.clone()).properties[&RUNNABLE.into()].clone())
    else {
        panic!("func not runnable")
    };
    crate::logln(
        LogLevel::Info,
        &format!(
            "run_generator_object: {}({:?})",
            func.clone().borrow().name.unwrap_or("anonymous"),
            params
        ),
    );
    let mem = runnable.mem.clone();

    let proto = Prototype::new_child(
        mem.clone(),
        Some("stack memory"),
        params
            .iter()
            .take(runnable.params.len())
            .enumerate()
            .map(|(i, param)| (runnable.params[i].as_str().into(), param.clone())),
    );
    let JsValue::Prototype(array) = inline_borrow!(Prototype::find(mem.clone(), &"Array".into()).1)
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
            Prototype::find(mem.clone(), &"Array".into())
                .1
                .borrow()
                .unwrap_proto("run_generator_object get Array"),
            params,
        ),
    );
    proto.borrow_mut().properties.insert("this".into(), this);

    IterGenerator {
        index: CodeIndex::new(),
        proto,
        code: runnable.code.clone(),
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