use crate::arena::Arena;
use crate::ast::{Binding, Limit, Order, Query};
use crate::prelude::{Type, Typed};
use crate::token::Operator;
use crate::{Attrs, ExprRef, Raw, SourceKind, Value};
use ordered_float::OrderedFloat;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Debug;

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
    pub attrs: Attrs,
    pub name: String,
    pub value: ExprView,
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

#[derive(Debug, Serialize)]
pub struct QueryView<A> {
    pub attrs: Attrs,
    pub sources: Vec<SourceView<A>>,
    pub predicate: Option<ExprView>,
    pub group_by: Option<GroupByView>,
    pub order_by: Option<OrderByView>,
    pub limit: Option<Limit>,
    pub projection: ExprView,
    pub distinct: bool,
    pub meta: A,
}

#[derive(Debug, Serialize)]
pub struct SourceView<A> {
    pub binding: Binding,
    pub kind: SourceKindView<A>,
}

#[derive(Debug, Serialize)]
pub enum SourceKindView<A> {
    Name(String),
    Subject(String),
    Subquery(Box<QueryView<A>>),
}

#[derive(Debug, Serialize)]
pub struct GroupByView {
    pub expr: ExprView,
    pub predicate: Option<ExprView>,
}

#[derive(Debug, Serialize)]
pub struct OrderByView {
    pub expr: ExprView,
    pub order: Order,
}

impl ExprRef {
    pub fn view(self, arena: &Arena) -> ExprView {
        let node = arena.exprs.get(self);
        let value = match node.value {
            Value::Number(n) => ValueView::Number(n),
            Value::String(s) => ValueView::String(arena.strings.get(s).to_owned()),
            Value::Bool(b) => ValueView::Bool(b),
            Value::Id(id) => ValueView::Id(arena.strings.get(id).to_owned()),
            Value::Array(arr) => {
                let mut values = Vec::with_capacity(arena.exprs.vec(arr).len());
                for idx in arena.exprs.vec_idxes(arr) {
                    let expr = arena.exprs.vec_get(arr, idx);
                    values.push(expr.view(arena));
                }

                ValueView::Array(values)
            }
            Value::Record(fields) => {
                let mut values = Vec::with_capacity(arena.exprs.rec(fields).len());

                for idx in arena.exprs.rec_idxes(fields) {
                    let field = arena.exprs.rec_get(fields, idx);
                    values.push(FieldView {
                        attrs: field.attrs,
                        name: arena.strings.get(field.name).to_owned(),
                        value: field.expr.view(arena),
                    })
                }

                ValueView::Record(values)
            }
            Value::Access(access) => ValueView::Access(AccessView {
                target: Box::new(access.target.view(arena)),
                field: arena.strings.get(access.field).to_owned(),
            }),
            Value::App(app) => {
                let mut args = Vec::with_capacity(arena.exprs.vec(app.args).len());

                for idx in arena.exprs.vec_idxes(app.args) {
                    let expr = arena.exprs.vec_get(app.args, idx);
                    args.push(expr.view(arena));
                }

                ValueView::App(AppView {
                    func: arena.strings.get(app.func).to_owned(),
                    args,
                })
            }
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

        ExprView::new(node.attrs, value)
    }
}

impl<A> Query<A> {
    pub fn view(self, arena: &Arena) -> QueryView<<A as ProjectMeta>::View>
    where
        A: ProjectMeta,
    {
        QueryView {
            attrs: self.attrs,
            sources: self
                .sources
                .into_iter()
                .map(|s| SourceView {
                    binding: s.binding.clone(),
                    kind: match s.kind {
                        SourceKind::Name(name) => {
                            SourceKindView::Name(arena.strings.get(name).to_owned())
                        }
                        SourceKind::Subject(subject) => {
                            SourceKindView::Subject(arena.strings.get(subject).to_owned())
                        }
                        SourceKind::Subquery(subquery) => {
                            SourceKindView::Subquery(Box::new(subquery.view(arena)))
                        }
                    },
                })
                .collect(),
            predicate: self.predicate.map(|e| e.view(arena)),
            group_by: self.group_by.map(|g| GroupByView {
                expr: g.expr.view(arena),
                predicate: g.predicate.map(|e| e.view(arena)),
            }),
            order_by: self.order_by.map(|o| OrderByView {
                expr: o.expr.view(arena),
                order: o.order,
            }),
            limit: self.limit,
            projection: self.projection.view(arena),
            meta: self.meta.project_meta(arena),
            distinct: self.distinct,
        }
    }
}

#[derive(Debug, Serialize)]
pub enum TypeView {
    Unspecified,
    Number,
    String,
    Bool,
    Subject,
    Date,
    Time,
    DateTime,
    Custom(String),
    Array(Box<TypeView>),
    Record(BTreeMap<String, TypeView>),
    App {
        args: Vec<TypeView>,
        result: Box<TypeView>,
        aggregate: bool,
    },
}

pub trait ProjectMeta {
    type View: Debug + Serialize;
    fn project_meta(&self, arena: &Arena) -> Self::View;
}

impl ProjectMeta for Raw {
    type View = Raw;

    fn project_meta(&self, _arena: &Arena) -> Self::View {
        *self
    }
}

#[derive(Debug, Serialize)]
pub struct MetaView {
    project: TypeView,
    aggregate: bool,
}

impl ProjectMeta for Typed {
    type View = MetaView;

    fn project_meta(&self, arena: &Arena) -> Self::View {
        MetaView {
            project: project_type(arena, self.project),
            aggregate: self.aggregate,
        }
    }
}

fn project_type(arena: &Arena, tpe: Type) -> TypeView {
    match tpe {
        Type::Unspecified => TypeView::Unspecified,
        Type::Number => TypeView::Number,
        Type::String => TypeView::String,
        Type::Bool => TypeView::Bool,
        Type::Subject => TypeView::Subject,
        Type::Date => TypeView::Date,
        Type::Time => TypeView::Time,
        Type::DateTime => TypeView::DateTime,
        Type::Custom(key) => TypeView::Custom(arena.strings.get(key).to_owned()),

        Type::Array(arr) => {
            TypeView::Array(Box::new(project_type(arena, arena.types.get_type(arr))))
        }

        Type::Record(rec) => {
            let mut props = BTreeMap::new();

            for (key, val) in arena.types.get_record(rec) {
                props.insert(
                    arena.strings.get(*key).to_owned(),
                    project_type(arena, *val),
                );
            }

            TypeView::Record(props)
        }

        Type::App {
            args,
            result,
            aggregate,
        } => {
            let mut args_view = Vec::new();

            for val in arena.types.get_args(args.values) {
                args_view.push(project_type(arena, *val));
            }

            TypeView::App {
                args: args_view,
                result: Box::new(project_type(arena, arena.types.get_type(result))),
                aggregate,
            }
        }
    }
}
