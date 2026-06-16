use std::{cell::RefCell, rc::Rc};

use crate::{JsValue, LogLevel, Prototype};



#[derive(Clone)]
pub enum CodeResult {
    Normal(Rc<RefCell<JsValue>>),
    NormalMember(
        Rc<RefCell<JsValue>>,
        Rc<RefCell<Prototype>>,
        Rc<RefCell<JsValue>>,
    ),
    Break,
    Continue,
    Yield(Rc<RefCell<JsValue>>),
    YieldBreak,
    Return(Rc<RefCell<JsValue>>),
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

pub struct CodeIndex {
    current: usize,
    retry: bool,
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
}

pub type Code = Box<dyn Fn(Rc<RefCell<Prototype>>, &mut CodeIndex) -> CodeResult>;

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

pub fn run_sub(codes: &[Code], mem: Rc<RefCell<Prototype>>, i: &mut CodeIndex) -> CodeResult {
    crate::logln(
        LogLevel::Trace,
        &format!(
            "run_sub: {} codes starting at {} with mem: {:?}",
            codes.len(),
            i.current(),
            mem
        ),
    );
    while i.current() < codes.len() {
        handle_return!(codes[i.current()](mem.clone(), i));
        i.next();
    }
    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
}