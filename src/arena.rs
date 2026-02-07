use serde::Serialize;
use crate::{Attrs, Expr, ExprRef, Value};

#[derive(Serialize)]
struct ExprNode {
    attrs: Attrs,
    value: Value,
}

#[derive(Default, Serialize)]
pub struct ExprArena {
    exprs: Vec<Expr>,
}

impl ExprArena {
    pub fn alloc(&mut self, attrs: Attrs, value: Value) -> ExprRef {
        todo!()
    }
}