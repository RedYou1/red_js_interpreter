use std::ptr;

use crate::{Code, CodeIndex, IterGenerator, prebuild::prelude::*, run_sub};

fn close_generator(this: &Rc<RefCell<Prototype>>, code: &[Code]) {
    this.borrow_mut().properties.insert(
        "__done__".into(),
        Rc::new(RefCell::new(JsValue::Boolean(true))),
    );
    this.borrow_mut()
        .properties
        .insert("__code__".into(), Rc::new(RefCell::new(JsValue::BigInt(0))));
    drop(unsafe { Rc::from_raw(code as *const [Code]) });
}

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
                    Rc::new(RefCell::new(JsValue::Prototype(self.env.mem))),
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
                (
                    "__done__".into(),
                    Rc::new(RefCell::new(JsValue::Boolean(false))),
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
    next, fn, |env, this, []| {
        let this = this.borrow().unwrap_proto("Generator.next this not proto");
        if matches!(
            inline_borrow!(Prototype::find(this.clone(), &"__done__".into()).1),
            JsValue::Boolean(true)
        ) {
            return CodeResult::Return(iterator_result(
                &env,
                Rc::new(RefCell::new(JsValue::Undefined)),
                true,
            ));
        }
        let JsValue::Prototype(proto) = inline_borrow!(Prototype::find(this.clone(), &"__mem__".into()).1) else {panic!("Generator.next parse __mem__ not proto {this:?}")};
        let JsValue::BigInt(code_len) = inline_borrow!(Prototype::find(this.clone(), &"__code__len".into()).1) else {panic!("Generator.next parse __code__len not BigInt {this:?}")};
        let code_ptr = if let JsValue::BigInt(ptr) = inline_borrow!(Prototype::find(this.clone(), &"__code__".into()).1) {
            ptr
        } else {panic!("Generator.next parse __code__ not BigInt {this:?}")};
        if code_ptr == 0 {
            return CodeResult::Return(iterator_result(
                &env,
                Rc::new(RefCell::new(JsValue::Undefined)),
                true,
            ));
        }
        let code = unsafe { ptr::slice_from_raw_parts(code_ptr as *const Code, code_len as usize).as_ref_unchecked()};
        let mut code_index = CodeIndex::load_from(this.clone(), "Generator_CodeIndex");
        if code_index.current >= code.len() {
            close_generator(&this, code);
            return CodeResult::Return(iterator_result(
                &env,
                Rc::new(RefCell::new(JsValue::Undefined)),
                true,
            ));
        }
        let res = run_sub(code, env.with_mem(proto.clone()), &mut code_index);
        match res {
            CodeResult::Normal(r) | CodeResult::Return(r) => {
                close_generator(&this, code);
                CodeResult::Return(iterator_result(&env, r, true))
            },
            CodeResult::YieldBreak => {
                close_generator(&this, code);
                CodeResult::Return(iterator_result(
                    &env,
                    Rc::new(RefCell::new(JsValue::Undefined)),
                    true,
                ))
            },
            CodeResult::Yield(r) => {
                code_index.next();
                code_index.save_into(this.clone(), "Generator_CodeIndex");
                CodeResult::Return(iterator_result(&env, r, false))
            },
            CodeResult::Error(_) => {
                close_generator(&this, code);
                res
            }
            _ => {
                panic!("got wrong codeResult from iterator in Generator obj {res:?}");
            }
        }
    };
    Symbol.iterator, fn, |_, this, []| { CodeResult::Return(this) }
}

pub(crate) fn iterator_result(
    env: &Environment,
    value: Rc<RefCell<JsValue>>,
    done: bool,
) -> Rc<RefCell<JsValue>> {
    let object = Prototype::find(env.mem.clone(), &"Object".into())
        .1
        .borrow()
        .unwrap_proto("IteratorResult Object prototype");
    Rc::new(RefCell::new(JsValue::Prototype(Prototype::new_child(
        object,
        None,
        [
            ("value".into(), value),
            ("done".into(), Rc::new(RefCell::new(JsValue::Boolean(done)))),
        ],
    ))))
}
