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
pub struct Expr<'a> {
    pub attrs: Attrs,
    pub value: Value<'a>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Access<'a> {
    pub target: Box<Expr<'a>>,
    pub field: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct App<'a> {
    pub func: &'a str,
    pub args: Vec<Expr<'a>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Field<'a> {
    pub name: &'a str,
    pub value: Expr<'a>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Binary<'a> {
    pub lhs: Box<Expr<'a>>,
    pub operator: Operator,
    pub rhs: Box<Expr<'a>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Unary<'a> {
    pub operator: Operator,
    pub expr: Box<Expr<'a>>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Value<'a> {
    Number(f64),
    String(&'a str),
    Bool(bool),
    Id(&'a str),
    Array(Vec<Expr<'a>>),
    Record(Vec<Field<'a>>),
    Access(Access<'a>),
    App(App<'a>),
    Binary(Binary<'a>),
    Unary(Unary<'a>),
    Group(Box<Expr<'a>>),
}

#[derive(Debug, Clone, Serialize)]
pub struct Source<'a> {
    pub binding: &'a str,
    pub kind: SourceKind<'a>,
}

#[derive(Debug, Clone, Serialize)]
pub enum SourceKind<'a> {
    Name(&'a str),
    Subject(&'a str),
    Subquery(Query<'a>),
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderBy<'a> {
    pub expr: Expr<'a>,
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
pub struct Query<'a> {
    pub attrs: Attrs,
    pub sources: Vec<Source<'a>>,
    pub predicate: Option<Expr<'a>>,
    pub group_by: Option<Expr<'a>>,
    pub order_by: Option<OrderBy<'a>>,
    pub limit: Option<Limit>,
    pub projection: Expr<'a>,
}
