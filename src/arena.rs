use std::collections::HashMap;
use std::hash::{BuildHasher, RandomState};
use serde::Serialize;
use crate::{Attrs, Expr, ExprKey, ExprPtr, ExprRef, Value};

#[derive(Serialize)]
pub struct ExprNode {
    pub attrs: Attrs,
    pub value: Value,
    pub key: ExprKey,
}

#[derive(Default, Serialize)]
pub struct ExprArena {
    #[serde(skip_serializing)]
    hasher: RandomState,
    slots: Vec<ExprNode>,
}

impl ExprArena {
    pub fn alloc(&mut self, attrs: Attrs, value: Value) -> ExprRef {
        let key = ExprKey(self.hasher.hash_one(&value));
        let ptr = ExprPtr(self.slots.len());

        self.slots.push(ExprNode {
            attrs,
            value,
            key,
        });

        ExprRef {
            ptr,
            key,
        }
    }

    pub fn get(&self, key: ExprRef) -> &ExprNode {
        &self.slots[key.ptr.0]
    }

    pub fn get_value(&self, key: ExprRef) -> &Value {
        &self.get(key).value
    }
}