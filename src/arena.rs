use crate::{Attrs, Expr, ExprKey, ExprPtr, ExprRef, Value};
use rustc_hash::FxBuildHasher;
use serde::Serialize;
use std::hash::BuildHasher;

#[derive(Debug, Serialize)]
struct Slot {
    attrs: Attrs,
    value: Value,
}

#[derive(Default, Serialize)]
pub struct ExprArena {
    #[serde(skip_serializing)]
    hasher: FxBuildHasher,
    slots: Vec<Slot>,
}

#[derive(Debug, Copy, Clone)]
pub struct Node<'a> {
    pub attrs: Attrs,
    pub value: &'a Value,
    pub node_ref: ExprRef,
}

impl<'a> Node<'a> {
    pub fn as_expr(&self) -> Expr {
        Expr {
            attrs: self.attrs,
            node_ref: self.node_ref,
        }
    }
}

impl ExprArena {
    pub fn alloc(&mut self, attrs: Attrs, value: Value) -> ExprRef {
        let key = ExprKey(self.hasher.hash_one(&value));

        let ptr = ExprPtr(self.slots.len());
        self.slots.push(Slot { attrs, value });

        ExprRef { key, ptr }
    }

    pub fn get(&self, node_ref: ExprRef) -> Node<'_> {
        let slot = &self.slots[node_ref.ptr.0];
        Node {
            attrs: slot.attrs,
            value: &slot.value,
            node_ref,
        }
    }
}
