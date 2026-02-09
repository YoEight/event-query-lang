use crate::{Attrs, Expr, ExprKey, ExprPtr, ExprRef, Value};
use rustc_hash::FxBuildHasher;
use serde::Serialize;
use std::hash::BuildHasher;

#[derive(Debug, Serialize)]
struct Slot {
    attrs: Attrs,
    value: Value,
}

/// An arena-based allocator for EventQL expressions.
///
/// The `ExprArena` provides a memory-efficient way to store and manage AST nodes
/// by using a flat vector and returning lightweight [`ExprRef`] handles.
#[derive(Default, Serialize)]
pub struct ExprArena {
    #[serde(skip_serializing)]
    hasher: FxBuildHasher,
    slots: Vec<Slot>,
}

/// A view into a single node within an [`ExprArena`].
///
/// This struct provides access to the attributes and value of a node
/// without transferring ownership. It's typically obtained by calling [`ExprArena::get`].
#[derive(Debug, Copy, Clone)]
pub struct Node<'a> {
    /// Metadata about this expression (e.g., source position)
    pub attrs: Attrs,
    /// The actual kind and value of the expression
    pub value: &'a Value,
    /// The stable reference to this node in the arena
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
    /// Allocates a new expression in the arena.
    ///
    /// This method takes an expression's attributes and value, hashes the value
    /// to create a stable [`ExprKey`], and stores it in the arena. It returns
    /// an [`ExprRef`] which can be used to retrieve the expression later.
    pub fn alloc(&mut self, attrs: Attrs, value: Value) -> ExprRef {
        let key = ExprKey(self.hasher.hash_one(&value));

        let ptr = ExprPtr(self.slots.len());
        self.slots.push(Slot { attrs, value });

        ExprRef { key, ptr }
    }

    /// Retrieves a node from the arena using an [`ExprRef`].
    ///
    /// # Panics
    ///
    /// Panics if the [`ExprRef`] contains an invalid pointer that is out of bounds
    /// of the arena's internal storage.
    pub fn get(&self, node_ref: ExprRef) -> Node<'_> {
        let slot = &self.slots[node_ref.ptr.0];
        Node {
            attrs: slot.attrs,
            value: &slot.value,
            node_ref,
        }
    }
}
