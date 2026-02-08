use crate::arena::ExprArena;
use crate::ast::Expr;
use crate::token::Operator;
use crate::{Attrs, Value};
use ordered_float::OrderedFloat;
use serde::Serialize;

mod analysis;
mod lexer;
mod parser;

#[derive(Debug, Serialize)]
pub struct ExprView {
    pub attrs: Attrs,
    pub value: ValueView,
}

impl ExprView {
    pub fn new(attrs: Attrs, value: ValueView) -> Self {
        Self { attrs, value }
    }
}

#[derive(Debug, Serialize)]
pub enum ValueView {
    Number(OrderedFloat<f64>),
    String(String),
    Bool(bool),
    Id(String),
    Array(Vec<ExprView>),
    Record(Vec<FieldView>),
    Access(AccessView),
    App(AppView),
    Binary(BinaryView),
    Unary(UnaryView),
    Group(Box<ExprView>),
}

#[derive(Debug, Serialize)]
pub struct FieldView {
    pub name: String,
    pub expr: ExprView,
}

#[derive(Debug, Serialize)]
pub struct AccessView {
    pub target: Box<ExprView>,
    pub field: String,
}

#[derive(Debug, Serialize)]
pub struct AppView {
    pub func: String,
    pub args: Vec<ExprView>,
}

#[derive(Debug, Serialize)]
pub struct BinaryView {
    pub lhs: Box<ExprView>,
    pub operator: Operator,
    pub rhs: Box<ExprView>,
}

#[derive(Debug, Serialize)]
pub struct UnaryView {
    pub operator: Operator,
    pub expr: Box<ExprView>,
}

impl Expr {
    pub fn view(&self, arena: &ExprArena) -> ExprView {
        let value = match arena.get(self.node_ref) {
            Value::Number(n) => ValueView::Number(*n),
            Value::String(s) => ValueView::String(s.clone()),
            Value::Bool(b) => ValueView::Bool(*b),
            Value::Id(id) => ValueView::Id(id.clone()),
            Value::Array(arr) => ValueView::Array(arr.iter().map(|e| e.view(arena)).collect()),
            Value::Record(fields) => ValueView::Record(
                fields
                    .iter()
                    .map(|f| FieldView {
                        name: f.name.clone(),
                        expr: f.expr.view(arena),
                    })
                    .collect(),
            ),
            Value::Access(access) => ValueView::Access(AccessView {
                target: Box::new(access.target.view(arena)),
                field: access.field.clone(),
            }),
            Value::App(app) => ValueView::App(AppView {
                func: app.func.clone(),
                args: app.args.iter().map(|e| e.view(arena)).collect(),
            }),
            Value::Binary(binary) => ValueView::Binary(BinaryView {
                lhs: Box::new(binary.lhs.view(arena)),
                operator: binary.operator,
                rhs: Box::new(binary.rhs.view(arena)),
            }),
            Value::Unary(unary) => ValueView::Unary(UnaryView {
                operator: unary.operator,
                expr: Box::new(unary.expr.view(arena)),
            }),
            Value::Group(expr) => ValueView::Group(Box::new(expr.view(arena))),
        };

        ExprView::new(self.attrs, value)
    }
}
