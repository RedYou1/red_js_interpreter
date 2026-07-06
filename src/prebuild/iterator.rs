use std::ptr;

use crate::{Code, CodeIndex, IterGenerator, prebuild::prelude::*, run_sub};

new_class! {
    prebuild_iterator,
    Iterator,
    Object,;;
}

impl IterGenerator {
    pub fn into_proto(self, generator: Rc<RefCell<Prototype>>) -> Rc<RefCell<Prototype>> {
        let t = Prototype::new_child(
            generator,
            None,
            [
                (
                    "__mem__".into(),
                    Rc::new(RefCell::new(JsValue::Prototype(self.proto))),
                ),
                (
                    "__code__len".into(),
                    Rc::new(RefCell::new(JsValue::BigInt(self.code.len() as i64))),
                ),
                (
                    "__code__".into(),
                    Rc::new(RefCell::new(JsValue::BigInt(
                        Rc::into_raw(self.code) as *const () as i64,
                    ))),
                ),
            ],
        );
        self.index.save_into(t.clone(), "Generator_CodeIndex");
        t
    }
}

new_class! {
    prebuild_itergen,
    Generator,
    Iterator,;
    next, fn, |_, this, []| {
        let this = this.borrow().unwrap_proto("Generator.next this not proto");
        let JsValue::Prototype(proto) = inline_borrow!(Prototype::find(this.clone(), &"__mem__".into()).1) else {panic!("Generator.next parse __mem__ not proto {this:?}")};
        let JsValue::BigInt(code_len) = inline_borrow!(Prototype::find(this.clone(), &"__code__len".into()).1) else {panic!("Generator.next parse __code__len not BigInt {this:?}")};
        let code = if let JsValue::BigInt(ptr) = inline_borrow!(Prototype::find(this.clone(), &"__code__".into()).1) {
            unsafe { ptr::slice_from_raw_parts(ptr as *const Code, code_len as usize).as_ref_unchecked()}
        } else {panic!("Generator.next parse __code__ not BigInt {this:?}")};
        let mut code_index = CodeIndex::load_from(this.clone(), "Generator_CodeIndex");
        if code_index.current >= code.len() {
            drop(unsafe{ Rc::from_raw(code) });
            return Rc::new(RefCell::new(JsValue::Undefined));
        }
        let res = run_sub(code, proto.clone(), &mut code_index);
        match res {
            CodeResult::Normal(r) | CodeResult::Return(r) => {
                drop(unsafe{ Rc::from_raw(code) });
                r
            },
            CodeResult::YieldBreak => {
                drop(unsafe{ Rc::from_raw(code) });
                Rc::new(RefCell::new(JsValue::Undefined))
            },
            CodeResult::Yield(r) => {
                code_index.next();
                code_index.save_into(this.clone(), "Generator_CodeIndex");
                r
            },
            _ => {
                panic!("got wrong codeResult from iterator in Generator obj {res:?}");
            }
        }
    };
    Symbol.iterator, fn, |_, this, []| { this }
}
