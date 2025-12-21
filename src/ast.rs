use crate::token::{Operator, Token};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
}

impl From<Token<'_>> for Pos {
    fn from(value: Token<'_>) -> Self {
        Self {
            line: value.line,
            col: value.col,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize)]
pub enum Type {
    Unspecified,
    Number,
    String,
    Bool,
    Array,
    Record,
    Subject,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Attrs {
    pub pos: Pos,
    pub scope: u64,
    pub tpe: Type,
}

impl Attrs {
    pub fn new(pos: Pos, scope: u64) -> Self {
        Self {
            pos,
            scope,
            tpe: Type::Unspecified,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Expr {
    pub attrs: Attrs,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct Access {
    pub target: Box<Expr>,
    pub field: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct App {
    pub func: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone, Serialize)]
pub struct Binary {
    pub lhs: Box<Expr>,
    pub operator: Operator,
    pub rhs: Box<Expr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Unary {
    pub operator: Operator,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Id(String),
    Array(Vec<Expr>),
    Record(Vec<Field>),
    Access(Access),
    App(App),
    Binary(Binary),
    Unary(Unary),
    Group(Box<Expr>),
}

#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub binding: String,
    pub kind: SourceKind,
}

#[derive(Debug, Clone, Serialize)]
pub enum SourceKind {
    Name(String),
    Subject(String),
    Subquery(Query),
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderBy {
    pub expr: Expr,
    pub order: Order,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Limit {
    Skip(u64),
    Top(u64),
}

#[derive(Debug, Clone, Serialize)]
pub struct Query {
    pub attrs: Attrs,
    pub sources: Vec<Source>,
    pub predicate: Option<Expr>,
    pub group_by: Option<Expr>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<Limit>,
    pub projection: Expr,
}
