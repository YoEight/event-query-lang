use std::{collections::HashMap, ops::ControlFlow};

use crate::{
    Attrs, Expr, Query, Raw, Source, SourceKind, Type, Typed, Value, error::AnalysisError,
};

pub type AnalysisResult<A> = std::result::Result<A, AnalysisError>;

#[derive(Default)]
pub struct AnalysisOptions {
    default_scope: Scope,
    event_type_info: TypeInfo,
}

pub fn static_analysis(
    options: &AnalysisOptions,
    query: Query<Raw>,
) -> AnalysisResult<Query<Typed>> {
    let mut analysis = Analysis::new(options);

    analysis.analyze_query(query)
}

#[derive(Default)]
pub struct TypeRegistry {
    pub scopes: HashMap<u64, Scope>,
}

#[derive(Default)]
pub struct Scope {
    pub entries: HashMap<String, TypeInfo>,
}

#[derive(Default, Clone)]
pub struct TypeInfo {
    pub tpe: Type,
    pub props: HashMap<String, TypeInfo>,
}

struct Analysis<'a> {
    options: &'a AnalysisOptions,
    registry: TypeRegistry,
}

impl<'a> Analysis<'a> {
    fn new(options: &'a AnalysisOptions) -> Self {
        Self {
            options,
            registry: TypeRegistry::default(),
        }
    }

    fn analyze_query(&mut self, query: Query<Raw>) -> AnalysisResult<Query<Typed>> {
        let scope = query.scope;
        let mut typed_sources = Vec::with_capacity(query.sources.len());

        for source in query.sources {
            typed_sources.push(self.analyze_source(scope, source)?);
        }

        if let Some(expr) = &query.predicate {}

        todo!()
    }

    fn analyze_source(
        &mut self,
        scope_id: u64,
        source: Source<Raw>,
    ) -> AnalysisResult<Source<Typed>> {
        let kind = self.analyze_source_kind(source.kind)?;
        let type_info = match &kind {
            SourceKind::Name(_) | SourceKind::Subject(_) => self.options.event_type_info.clone(),
            SourceKind::Subquery(query) => self.projection_type(query),
        };

        let scope = self.registry.scopes.entry(scope_id).or_default();

        if scope
            .entries
            .insert(source.binding.name.clone(), type_info)
            .is_some()
        {
            return Err(AnalysisError::BindingAlreadyExists(
                source.binding.pos.line,
                source.binding.pos.col,
                source.binding.name,
            ));
        }

        Ok(Source {
            binding: source.binding,
            kind,
        })
    }

    fn analyze_source_kind(&mut self, kind: SourceKind<Raw>) -> AnalysisResult<SourceKind<Typed>> {
        match kind {
            SourceKind::Name(n) => Ok(SourceKind::Name(n)),
            SourceKind::Subject(s) => Ok(SourceKind::Subject(s)),
            SourceKind::Subquery(query) => {
                let query = self.analyze_query(*query)?;
                Ok(SourceKind::Subquery(Box::new(query)))
            }
        }
    }

    fn analyze_expr(&mut self, expr: &Expr, expect: Type) -> AnalysisResult<()> {
        self.analyze_value(&expr.attrs, &expr.value, expect)
    }

    fn analyze_value(
        &mut self,
        attrs: &Attrs,
        value: &Value,
        mut expect: Type,
    ) -> AnalysisResult<()> {
        let result = match value {
            Value::Number(_) => expect.check_mut(Type::Number),
            Value::String(_) => expect.check_mut(Type::String),
            Value::Bool(_) => expect.check_mut(Type::Bool),
            Value::Id(id) => {
                if let Some(info) = self.options.default_scope.entries.get(id) {
                    expect.check_mut(info.tpe.clone())
                } else {
                    todo!()
                }
            }
            Value::Array(exprs) => todo!(),
            Value::Record(fields) => todo!(),
            Value::Access(access) => todo!(),
            Value::App(app) => todo!(),
            Value::Binary(binary) => todo!(),
            Value::Unary(unary) => todo!(),
            Value::Group(expr) => todo!(),
        };

        if let ControlFlow::Break((expect, actual)) = result {
            return Err(AnalysisError::TypeMismatch(
                attrs.pos.line,
                attrs.pos.col,
                expect,
                actual,
            ));
        }

        todo!()
    }

    fn get_id_type_info_mut(&mut self, id: &str) -> Option<&mut Type> {
        todo!()
    }

    fn projection_type(&self, query: &Query<Typed>) -> TypeInfo {
        todo!()
    }
}
