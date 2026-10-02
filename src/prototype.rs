use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    hash::{Hash, Hasher},
    rc::Rc,
};

use crate::{JsValue, PROTO_NAME, inline_borrow};

#[derive(Clone, Eq, PartialEq)]
pub struct Prototype {
    pub name: Option<&'static str>,
    pub properties: HashMap<JsValue, Rc<RefCell<JsValue>>>,
    pub property_order: Vec<JsValue>,
    pub non_enumerable: std::collections::HashSet<JsValue>,
    pub formating: bool,
}

impl Hash for Prototype {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if let Some(name) = self.name {
            return name.hash(state);
        }
        let mut entry_hashes = self
            .properties
            .iter()
            .map(|(key, value)| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                key.hash(&mut hasher);
                value.borrow().hash(&mut hasher);
                hasher.finish()
            })
            .collect::<Vec<_>>();
        entry_hashes.sort_unstable();
        entry_hashes.hash(state);
    }
}

impl Prototype {
    pub fn insert_property(&mut self, key: JsValue, value: Rc<RefCell<JsValue>>) {
        if !self.properties.contains_key(&key) {
            self.property_order.push(key.clone());
        }
        self.properties.insert(key, value);
    }

    pub fn remove_property(&mut self, key: &JsValue) {
        self.properties.remove(key);
        self.property_order.retain(|existing| existing != key);
    }

    pub fn inner_find(
        this: Rc<RefCell<Self>>,
        key: &JsValue,
    ) -> Option<(Rc<RefCell<Prototype>>, Rc<RefCell<JsValue>>)> {
        if let Some(elem) = this.clone().borrow().properties.get(key) {
            Some((this.clone(), elem.clone()))
        } else {
            Self::inner_find(this.borrow().parent()?, key)
        }
    }

    pub fn opt_find(
        this: Rc<RefCell<Self>>,
        key: &JsValue,
    ) -> Option<(Rc<RefCell<Prototype>>, Rc<RefCell<JsValue>>)> {
        Self::inner_find(this, key)
    }

    pub fn find(
        this: Rc<RefCell<Self>>,
        key: &JsValue,
    ) -> (Option<Rc<RefCell<Prototype>>>, Rc<RefCell<JsValue>>) {
        // let receiver_name = this.borrow().name;
        let result = Self::inner_find(this, key)
            .map(|(a, b)| (Some(a), b))
            .unwrap_or((None, Rc::new(RefCell::new(JsValue::Undefined))));
        // if result.0.is_none() {
        //     logln(
        //         LogLevel::Trace,
        //         &format!("Prototype::find miss receiver={receiver_name:?} key={key:?}"),
        //     );
        // }
        result
    }

    pub fn parent(&self) -> Option<Rc<RefCell<Prototype>>> {
        self.properties.get(&PROTO_NAME.into()).and_then(|proto| {
            if let JsValue::Prototype(ref proto) = inline_borrow!(proto) {
                Some(proto.clone())
            } else {
                None
            }
        })
    }

    pub fn new_child(
        this: Rc<RefCell<Self>>,
        name: Option<&'static str>,
        properties: impl IntoIterator<Item = (JsValue, Rc<RefCell<JsValue>>)>,
    ) -> Rc<RefCell<Self>> {
        let properties = properties.into_iter().collect::<Vec<_>>();
        let property_order = properties.iter().map(|(key, _)| key.clone()).collect();
        let mut properties = HashMap::from_iter(properties);
        properties.insert(
            PROTO_NAME.into(),
            Rc::new(RefCell::new(JsValue::Prototype(this))),
        );
        Rc::new(RefCell::new(Prototype {
            name,
            properties,
            property_order,
            non_enumerable: std::collections::HashSet::new(),
            formating: false,
        }))
    }
}

impl Debug for Prototype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.formating {
            f.debug_struct("Prototype")
                .field("ptr", &(self as *const Prototype as *const () as usize))
                .field("already_formated", &true)
                .finish()
        } else {
            unsafe { (self as *const Prototype as *mut Prototype).as_mut_unchecked() }.formating =
                true;
            let t = f
                .debug_struct("Prototype")
                .field("ptr", &(self as *const Prototype as *const () as usize))
                .field("name", &self.name)
                .field(
                    "properties",
                    &self
                        .properties
                        .iter()
                        .map(|(k, v)| {
                            let mut k = k.clone();
                            let mut v = inline_borrow!(v);

                            if let JsValue::Prototype(in_k) = &k {
                                let name = in_k.borrow().name;
                                if let Some(name) = name {
                                    k = JsValue::String(format!("[{}]", name));
                                }
                            }

                            if let JsValue::Prototype(in_v) = &v {
                                let name = in_v.borrow().name;
                                if let Some(name) = name {
                                    v = JsValue::String(format!("[{}]", name));
                                }
                            }
                            (k, v)
                        })
                        .collect::<HashMap<JsValue, JsValue>>(),
                )
                .finish();
            unsafe { (self as *const Prototype as *mut Prototype).as_mut_unchecked() }.formating =
                false;
            t
        }
    }
}
