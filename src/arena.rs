use std::collections::hash_map::Entry;
use std::hash::{BuildHasher};
use rustc_hash::{FxBuildHasher, FxHashMap};
use serde::Serialize;
use crate::{ExprKey, ExprRef, Value};


#[derive(Default, Serialize)]
pub struct ExprArena {
    #[serde(skip_serializing)]
    hasher: FxBuildHasher,
    slots: FxHashMap<u64, Value>,
}

impl ExprArena {
    pub fn alloc(&mut self, value: Value) -> ExprRef {
        let key = ExprKey(self.hasher.hash_one(&value));

        if let Entry::Vacant(entry) =self.slots.entry(key.0) {
            entry.insert(value);
        }

        ExprRef {
            key,
        }
    }

    pub fn get(&self, key: ExprRef) -> &Value {
        self.slots.get(&key.key.0).expect("to be always defined")
    }
}