use crate::{Attrs, Expr, ExprKey, ExprPtr, ExprRef, Value};
use rustc_hash::{FxBuildHasher, FxHashMap};
use serde::Serialize;
use std::hash::BuildHasher;

#[derive(Debug, Serialize)]
struct Slot {
    spans: Vec<Attrs>,
    value: Value,
}

#[derive(Default, Serialize)]
pub struct ExprArena {
    #[serde(skip_serializing)]
    hasher: FxBuildHasher,
    slots: FxHashMap<u64, Slot>,
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

        let slot = self.slots.entry(key.0).or_insert_with(|| Slot {
            spans: vec![],
            value,
        });

        let ptr = ExprPtr(slot.spans.len());
        slot.spans.push(attrs);
        ExprRef { key, ptr }
    }

    pub fn get(&self, node_ref: ExprRef) -> Node<'_> {
        let slot = self
            .slots
            .get(&node_ref.key.0)
            .expect("to be always defined");

        Node {
            attrs: slot.spans[node_ref.ptr.0],
            value: &slot.value,
            node_ref,
        }
    }
}
