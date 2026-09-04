use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{Environment, JsValue, LogLevel, Prototype, inline_borrow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeResult {
    Normal(Rc<RefCell<JsValue>>),
    NormalMember(
        Rc<RefCell<JsValue>>,
        Rc<RefCell<Prototype>>,
        Rc<RefCell<JsValue>>,
    ),
    Break(Option<String>),
    Continue(Option<String>),
    Yield(Rc<RefCell<JsValue>>),
    YieldBreak,
    Return(Rc<RefCell<JsValue>>),
    Error(Rc<RefCell<JsValue>>),
}

impl CodeResult {
    pub fn unwrap_normal(self) -> Rc<RefCell<JsValue>> {
        if let CodeResult::Normal(res) = self {
            res
        } else if let CodeResult::NormalMember(res, _, _) = self {
            res
        } else {
            panic!("unwrap_normal")
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct CodeIndex {
    pub(crate) current: usize,
    pub(crate) retry: bool,
}

impl CodeIndex {
    pub const fn new() -> Self {
        Self {
            current: 0,
            retry: false,
        }
    }

    pub const fn reset(&mut self) {
        self.current = 0;
        self.retry = false;
    }

    pub const fn next(&mut self) -> usize {
        if self.retry {
            self.retry = false;
        } else {
            self.current += 1;
        }
        self.current
    }

    pub const fn current(&self) -> usize {
        self.current
    }

    pub const fn set_retry(&mut self) {
        self.retry = true;
    }

    pub const fn reset_retry(&mut self) {
        self.retry = false;
    }

    pub const fn move_iamount(&mut self, amount: isize) {
        if amount < 0 {
            self.current -= amount.abs() as usize;
        } else {
            self.current += amount as usize;
        }
    }

    pub const fn move_amount(&mut self, dir: bool, amount: usize) {
        if dir {
            self.current += amount;
        } else {
            self.current -= amount;
        }
    }

    pub const fn skip(&mut self, amount: usize) {
        self.current += amount;
    }

    pub const fn goto_end(&mut self) {
        self.current = usize::MAX;
    }

    pub fn load_from(proto: Rc<RefCell<Prototype>>, name: &str) -> Self {
        let JsValue::BigInt(i) =
            inline_borrow!(Prototype::find(proto.clone(), &format!("__{name}_current__").into()).1)
        else {
            panic!("CodeIndex.load_from parse current not BigInt {proto:?}")
        };
        let JsValue::Boolean(r) =
            inline_borrow!(Prototype::find(proto.clone(), &format!("__{name}_retry__").into()).1)
        else {
            panic!("CodeIndex.load_from parse retry not Boolean {proto:?}")
        };
        Self {
            current: i as usize,
            retry: r,
        }
    }

    pub fn save_into(&self, proto: Rc<RefCell<Prototype>>, name: &str) {
        let props: &mut HashMap<JsValue, Rc<RefCell<JsValue>>> = &mut proto.borrow_mut().properties;
        props.insert(
            format!("__{name}_current__").into(),
            Rc::new(RefCell::new(JsValue::BigInt(self.current as i64))),
        );
        props.insert(
            format!("__{name}_retry__").into(),
            Rc::new(RefCell::new(JsValue::Boolean(self.retry))),
        );
    }
}

pub type Code = Box<dyn Fn(Environment, &mut CodeIndex) -> CodeResult>;

#[macro_export]
macro_rules! handle_return {
    ($code:expr) => {{
        let __res__ = $code;
        if let CodeResult::Normal(res) = __res__ {
            res
        } else if let CodeResult::NormalMember(res, _, _) = __res__ {
            res
        } else {
            return __res__;
        }
    }};
}

#[macro_export]
macro_rules! handle_error {
    ($code:expr) => {{
        let __res__ = $code;
        match __res__ {
            CodeResult::Normal(res) => res,
            CodeResult::NormalMember(res, _, _) => res,
            CodeResult::Return(res) => res,
            CodeResult::Error(_) => return __res__,
            _ => {
                panic!("unhandled code result in handle_error")
            }
        }
    }};
}

pub fn run_sub(codes: &[Code], env: Environment, i: &mut CodeIndex) -> CodeResult {
    env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
        format!(
            "run_sub: {} codes starting at {} with mem: {:?}",
            codes.len(),
            i.current(),
            env.mem
        )
    });
    let mut result = CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)));
    while i.current() < codes.len() {
        result = codes[i.current()](env.clone(), i);
        match result {
            CodeResult::Normal(_) | CodeResult::NormalMember(_, _, _) => {}
            _ => {
                env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                    format!(
                        "run_sub stopped at code index {} with {:?}",
                        i.current(),
                        result
                    )
                });
                return result;
            }
        }
        i.next();
    }
    result
}
