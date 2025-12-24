use std::collections::HashMap;

use crate::{Query, Raw, Source, SourceKind, Type, Typed, error::AnalysisError};

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
    scope: u64,
}

impl<'a> Analysis<'a> {
    fn new(options: &'a AnalysisOptions) -> Self {
        Self {
            options,
            registry: TypeRegistry::default(),
            scope: 1,
        }
    }

    fn analyze_query(&mut self, query: Query<Raw>) -> AnalysisResult<Query<Typed>> {
        for source in query.sources {}
        todo!()
    }

    fn analyze_source(&mut self, source: Source<Raw>) -> AnalysisResult<Source<Typed>> {
        let kind = self.analyze_source_kind(source.kind)?;
        let type_info = match &kind {
            SourceKind::Name(_) | SourceKind::Subject(_) => self.options.event_type_info.clone(),
            SourceKind::Subquery(query) => self.projection_type(query),
        };

        let scope = self.registry.scopes.entry(self.scope).or_default();

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
        todo!()
    }

    fn projection_type(&self, query: &Query<Typed>) -> TypeInfo {
        todo!()
    }
}
