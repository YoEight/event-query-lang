use std::{
    borrow::Cow,
    collections::{BTreeMap, HashSet, btree_map::Entry},
    mem,
};

use case_insensitive_hashmap::CaseInsensitiveHashMap;
use serde::{Serialize, ser::SerializeMap};
use unicase::Ascii;

use crate::arena::ExprArena;
use crate::{
    App, Attrs, Binary, Expr, ExprRef, Field, FunArgs, Query, Raw, Source, SourceKind, Type, Value,
    error::AnalysisError, token::Operator,
};

/// Represents the state of a query that has been statically analyzed.
///
/// This type is used as a marker to indicate that a query has successfully
/// passed static analysis. It contains metadata about the query's type
/// information and variable scope after type checking.
///
/// All variables in a typed query are guaranteed to be:
/// - Properly declared and in scope
/// - Type-safe with sound type assignments
#[derive(Debug, Clone, Serialize)]
pub struct Typed {
    /// The inferred type of the query's projection (PROJECT INTO clause).
    ///
    /// This represents the shape and types of the data that will be
    /// returned by the query.
    pub project: Type,

    /// The variable scope after static analysis.
    ///
    /// Contains all variables that were in scope during type checking,
    /// including bindings from FROM clauses and their associated types.
    #[serde(skip)]
    pub scope: Scope,

    /// Indicates if the query uses aggregate functions.
    pub aggregate: bool,
}

/// Result type for static analysis operations.
///
/// This is a convenience type alias for `Result<A, AnalysisError>` used throughout
/// the static analysis module.
pub type AnalysisResult<A> = std::result::Result<A, AnalysisError>;

/// Configuration options for static analysis.
///
/// This structure contains the type information needed to perform static analysis
/// on EventQL queries, including the default scope with built-in functions and
/// the type information for event records.
pub struct AnalysisOptions {
    /// The default scope containing built-in functions and their type signatures.
    pub default_scope: Scope,
    /// Type information for event records being queried.
    pub event_type_info: Type,
    /// Custom types that are not defined in the EventQL reference.
    ///
    /// This set allows users to register custom type names that can be used
    /// in type conversion expressions (e.g., `field AS CustomType`). Custom
    /// type names are case-insensitive.
    ///
    /// # Examples
    ///
    /// ```
    /// use eventql_parser::prelude::AnalysisOptions;
    ///
    /// let options = AnalysisOptions::default()
    ///     .add_custom_type("Foobar");
    /// ```
    pub custom_types: HashSet<Ascii<String>>,
}

impl AnalysisOptions {
    /// Adds a custom type name to the analysis options.
    ///
    /// Custom types allow you to use type conversion syntax with types that are
    /// not part of the standard EventQL type system. The type name is stored
    /// case-insensitively.
    ///
    /// # Arguments
    ///
    /// * `value` - The custom type name to register
    ///
    /// # Returns
    ///
    /// Returns `self` to allow for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use eventql_parser::prelude::AnalysisOptions;
    ///
    /// let options = AnalysisOptions::default()
    ///     .add_custom_type("Timestamp")
    ///     .add_custom_type("UUID");
    /// ```
    pub fn add_custom_type<'a>(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        match value.into() {
            Cow::Borrowed(t) => self.custom_types.insert(Ascii::new(t.to_owned())),
            Cow::Owned(t) => self.custom_types.insert(Ascii::new(t)),
        };

        self
    }
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            default_scope: Scope {
                entries: CaseInsensitiveHashMap::from_iter([
                    (
                        "ABS",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "CEIL",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "FLOOR",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "ROUND",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "COS",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "EXP",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "POW",
                        Type::App {
                            args: vec![Type::Number, Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "SQRT",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "RAND",
                        Type::App {
                            args: vec![].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "PI",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "LOWER",
                        Type::App {
                            args: vec![Type::String].into(),
                            result: Box::new(Type::String),
                            aggregate: false,
                        },
                    ),
                    (
                        "UPPER",
                        Type::App {
                            args: vec![Type::String].into(),
                            result: Box::new(Type::String),
                            aggregate: false,
                        },
                    ),
                    (
                        "TRIM",
                        Type::App {
                            args: vec![Type::String].into(),
                            result: Box::new(Type::String),
                            aggregate: false,
                        },
                    ),
                    (
                        "LTRIM",
                        Type::App {
                            args: vec![Type::String].into(),
                            result: Box::new(Type::String),
                            aggregate: false,
                        },
                    ),
                    (
                        "RTRIM",
                        Type::App {
                            args: vec![Type::String].into(),
                            result: Box::new(Type::String),
                            aggregate: false,
                        },
                    ),
                    (
                        "LEN",
                        Type::App {
                            args: vec![Type::String].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "INSTR",
                        Type::App {
                            args: vec![Type::String].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "SUBSTRING",
                        Type::App {
                            args: vec![Type::String, Type::Number, Type::Number].into(),
                            result: Box::new(Type::String),
                            aggregate: false,
                        },
                    ),
                    (
                        "REPLACE",
                        Type::App {
                            args: vec![Type::String, Type::String, Type::String].into(),
                            result: Box::new(Type::String),
                            aggregate: false,
                        },
                    ),
                    (
                        "STARTSWITH",
                        Type::App {
                            args: vec![Type::String, Type::String].into(),
                            result: Box::new(Type::Bool),
                            aggregate: false,
                        },
                    ),
                    (
                        "ENDSWITH",
                        Type::App {
                            args: vec![Type::String, Type::String].into(),
                            result: Box::new(Type::Bool),
                            aggregate: false,
                        },
                    ),
                    (
                        "NOW",
                        Type::App {
                            args: vec![].into(),
                            result: Box::new(Type::DateTime),
                            aggregate: false,
                        },
                    ),
                    (
                        "YEAR",
                        Type::App {
                            args: vec![Type::Date].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "MONTH",
                        Type::App {
                            args: vec![Type::Date].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "DAY",
                        Type::App {
                            args: vec![Type::Date].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "HOUR",
                        Type::App {
                            args: vec![Type::Time].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "MINUTE",
                        Type::App {
                            args: vec![Type::Time].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "SECOND",
                        Type::App {
                            args: vec![Type::Time].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "WEEKDAY",
                        Type::App {
                            args: vec![Type::Date].into(),
                            result: Box::new(Type::Number),
                            aggregate: false,
                        },
                    ),
                    (
                        "IF",
                        Type::App {
                            args: vec![Type::Bool, Type::Unspecified, Type::Unspecified].into(),
                            result: Box::new(Type::Unspecified),
                            aggregate: false,
                        },
                    ),
                    (
                        "COUNT",
                        Type::App {
                            args: FunArgs {
                                values: vec![Type::Bool],
                                needed: 0,
                            },
                            result: Box::new(Type::Number),
                            aggregate: true,
                        },
                    ),
                    (
                        "SUM",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: true,
                        },
                    ),
                    (
                        "AVG",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: true,
                        },
                    ),
                    (
                        "MIN",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: true,
                        },
                    ),
                    (
                        "MAX",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: true,
                        },
                    ),
                    (
                        "MEDIAN",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: true,
                        },
                    ),
                    (
                        "STDDEV",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: true,
                        },
                    ),
                    (
                        "VARIANCE",
                        Type::App {
                            args: vec![Type::Number].into(),
                            result: Box::new(Type::Number),
                            aggregate: true,
                        },
                    ),
                    (
                        "UNIQUE",
                        Type::App {
                            args: vec![Type::Unspecified].into(),
                            result: Box::new(Type::Unspecified),
                            aggregate: true,
                        },
                    ),
                ]),
            },
            event_type_info: Type::Record(BTreeMap::from([
                ("specversion".to_owned(), Type::String),
                ("id".to_owned(), Type::String),
                ("time".to_owned(), Type::DateTime),
                ("source".to_owned(), Type::String),
                ("subject".to_owned(), Type::Subject),
                ("type".to_owned(), Type::String),
                ("datacontenttype".to_owned(), Type::String),
                ("data".to_owned(), Type::Unspecified),
                ("predecessorhash".to_owned(), Type::String),
                ("hash".to_owned(), Type::String),
                ("traceparent".to_owned(), Type::String),
                ("tracestate".to_owned(), Type::String),
                ("signature".to_owned(), Type::String),
            ])),
            custom_types: HashSet::default(),
        }
    }
}

/// Performs static analysis on an EventQL query.
///
/// This function takes a raw (untyped) query and performs type checking and
/// variable scoping analysis. It validates that:
/// - All variables are properly declared
/// - Types match expected types in expressions and operations
/// - Field accesses are valid for their record types
/// - Function calls have the correct argument types
/// - Aggregate functions are only used in PROJECT INTO clauses
/// - Aggregate functions are not mixed with source-bound fields in projections
/// - Aggregate function arguments are source-bound fields (not constants or function results)
/// - Record literals are non-empty in projection contexts
///
/// # Arguments
///
/// * `options` - Configuration containing type information and default scope
/// * `query` - The raw query to analyze
///
/// # Returns
///
/// Returns a typed query on success, or an `AnalysisError` if type checking fails.
pub fn static_analysis(
    arena: &ExprArena,
    options: &AnalysisOptions,
    query: Query<Raw>,
) -> AnalysisResult<Query<Typed>> {
    let mut analysis = Analysis::new(arena, options);

    analysis.analyze_query(query)
}

/// Represents a variable scope during static analysis.
///
/// A scope tracks the variables and their types that are currently in scope
/// during type checking. This is used to resolve variable references and
/// ensure type correctness.
#[derive(Default, Clone, Debug)]
pub struct Scope {
    /// Map of variable names to their types.
    pub entries: CaseInsensitiveHashMap<Type>,
}

impl Serialize for Scope {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;

        for (key, value) in self.entries.iter() {
            map.serialize_entry(key.as_str(), value)?;
        }

        map.end()
    }
}

impl Scope {
    /// Checks if the scope contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Default)]
struct CheckContext {
    use_agg_func: bool,
    use_source_based: bool,
}

/// Context for controlling analysis behavior.
///
/// This struct allows you to configure how expressions are analyzed,
/// such as whether aggregate functions are allowed in the current context.
#[derive(Default)]
pub struct AnalysisContext {
    /// Controls whether aggregate functions (like COUNT, SUM, AVG) are allowed
    /// in the current analysis context.
    ///
    /// Set to `true` to allow aggregate functions, `false` to reject them.
    /// Defaults to `false`.
    pub allow_agg_func: bool,

    /// Indicates if the query uses aggregate functions.
    pub use_agg_funcs: bool,
}

/// A type checker and static analyzer for EventQL expressions.
///
/// This struct maintains the analysis state including scopes and type information.
/// It can be used to perform type checking on individual expressions or entire queries.
pub struct Analysis<'a> {
    arena: &'a ExprArena,
    /// The analysis options containing type information for functions and event types.
    options: &'a AnalysisOptions,
    /// Stack of previous scopes for nested scope handling.
    prev_scopes: Vec<Scope>,
    /// The current scope containing variable bindings and their types.
    scope: Scope,
}

impl<'a> Analysis<'a> {
    /// Creates a new analysis instance with the given options.
    pub fn new(arena: &'a ExprArena, options: &'a AnalysisOptions) -> Self {
        Self {
            arena,
            options,
            prev_scopes: Default::default(),
            scope: Scope::default(),
        }
    }

    /// Returns a reference to the current scope.
    ///
    /// The scope contains variable bindings and their types for the current
    /// analysis context. Note that this only includes local variable bindings
    /// and does not include global definitions such as built-in functions
    /// (e.g., `COUNT`, `NOW`) or event type information, which are stored
    /// in the `AnalysisOptions`.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Returns a mutable reference to the current scope.
    ///
    /// This allows you to modify the scope by adding or removing variable bindings.
    /// This is useful when you need to set up custom type environments before
    /// analyzing expressions. Note that this only provides access to local variable
    /// bindings; global definitions like built-in functions are managed through
    /// `AnalysisOptions` and cannot be modified via the scope.
    pub fn scope_mut(&mut self) -> &mut Scope {
        &mut self.scope
    }

    fn enter_scope(&mut self) {
        if self.scope.is_empty() {
            return;
        }

        let prev = mem::take(&mut self.scope);
        self.prev_scopes.push(prev);
    }

    fn exit_scope(&mut self) -> Scope {
        if let Some(prev) = self.prev_scopes.pop() {
            mem::replace(&mut self.scope, prev)
        } else {
            mem::take(&mut self.scope)
        }
    }

    /// Performs static analysis on a parsed query.
    ///
    /// This method analyzes an entire EventQL query, performing type checking on all
    /// clauses including sources, predicates, group by, order by, and projections.
    /// It returns a typed version of the query with type information attached.
    ///
    /// # Arguments
    ///
    /// * `query` - A parsed query in its raw (untyped) form
    ///
    /// # Returns
    ///
    /// Returns a typed query with all type information resolved, or an error if
    /// type checking fails for any part of the query.
    ///
    /// # Example
    ///
    /// ```rust
    /// use eventql_parser::{parse_query, prelude::{Analysis, AnalysisOptions}};
    ///
    /// let query = parse_query("FROM e IN events WHERE [1,2,3] CONTAINS e.data.price PROJECT INTO e").unwrap();
    ///
    /// let options = AnalysisOptions::default();
    /// let mut analysis = Analysis::new(&options);
    ///
    /// let typed_query = analysis.analyze_query(query);
    /// assert!(typed_query.is_ok());
    /// ```
    pub fn analyze_query(&mut self, query: Query<Raw>) -> AnalysisResult<Query<Typed>> {
        self.enter_scope();

        let mut sources = Vec::with_capacity(query.sources.len());
        let mut ctx = AnalysisContext::default();

        for source in query.sources {
            sources.push(self.analyze_source(source)?);
        }

        if let Some(expr) = query.predicate.as_ref().copied() {
            self.analyze_expr(&mut ctx, expr, Type::Bool)?;
        }

        if let Some(group_by) = &query.group_by {
            let node = self.arena.get(group_by.expr);
            if !matches!(node.value, Value::Access(_)) {
                return Err(AnalysisError::ExpectFieldLiteral(
                    node.attrs.pos.line,
                    node.attrs.pos.col,
                ));
            }

            self.analyze_expr(&mut ctx, group_by.expr, Type::Unspecified)?;

            if let Some(expr) = group_by.predicate.as_ref().copied() {
                let node = self.arena.get(expr);
                ctx.allow_agg_func = true;
                ctx.use_agg_funcs = true;

                self.analyze_expr(&mut ctx, expr, Type::Bool)?;
                if !self.expect_agg_expr(expr)? {
                    return Err(AnalysisError::ExpectAggExpr(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                    ));
                }
            }

            ctx.allow_agg_func = true;
            ctx.use_agg_funcs = true;
        }

        let project = self.analyze_projection(&mut ctx, query.projection)?;

        if let Some(order_by) = &query.order_by {
            self.analyze_expr(&mut ctx, order_by.expr, Type::Unspecified)?;
            let node = self.arena.get(order_by.expr);
            if query.group_by.is_none() && !matches!(node.value, Value::Access(_)) {
                return Err(AnalysisError::ExpectFieldLiteral(
                    node.attrs.pos.line,
                    node.attrs.pos.col,
                ));
            } else if query.group_by.is_some() {
                self.expect_agg_func(order_by.expr)?;
            }
        }

        let scope = self.exit_scope();

        Ok(Query {
            attrs: query.attrs,
            sources,
            predicate: query.predicate,
            group_by: query.group_by,
            order_by: query.order_by,
            limit: query.limit,
            projection: query.projection,
            distinct: query.distinct,
            meta: Typed {
                project,
                scope,
                aggregate: ctx.use_agg_funcs,
            },
        })
    }

    fn analyze_source(&mut self, source: Source<Raw>) -> AnalysisResult<Source<Typed>> {
        let kind = self.analyze_source_kind(source.kind)?;
        let tpe = match &kind {
            SourceKind::Name(_) | SourceKind::Subject(_) => self.options.event_type_info.clone(),
            SourceKind::Subquery(query) => self.projection_type(query),
        };

        if self
            .scope
            .entries
            .insert(source.binding.name.clone(), tpe)
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

    fn analyze_projection(
        &mut self,
        ctx: &mut AnalysisContext,
        expr: ExprRef,
    ) -> AnalysisResult<Type> {
        let node = self.arena.get(expr);
        match node.value {
            Value::Record(record) => {
                if record.is_empty() {
                    return Err(AnalysisError::EmptyRecord(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                    ));
                }

                ctx.allow_agg_func = true;
                let tpe = self.analyze_expr(ctx, node.node_ref, Type::Unspecified)?;
                let mut chk_ctx = CheckContext {
                    use_agg_func: ctx.use_agg_funcs,
                    ..Default::default()
                };

                self.check_projection_on_record(&mut chk_ctx, record.as_slice())?;
                Ok(tpe)
            }

            Value::App(app) => {
                ctx.allow_agg_func = true;

                let tpe = self.analyze_expr(ctx, node.node_ref, Type::Unspecified)?;

                if ctx.use_agg_funcs {
                    let mut chk_ctx = CheckContext {
                        use_agg_func: ctx.use_agg_funcs,
                        ..Default::default()
                    };

                    self.check_projection_on_field_expr(&mut chk_ctx, expr)?;
                } else {
                    self.reject_constant_func(node.attrs, app)?;
                }

                Ok(tpe)
            }

            Value::Id(_) if ctx.use_agg_funcs => Err(AnalysisError::ExpectAggExpr(
                node.attrs.pos.line,
                node.attrs.pos.col,
            )),

            Value::Id(id) => {
                if let Some(tpe) = self.scope.entries.get(id.as_str()).cloned() {
                    Ok(tpe)
                } else {
                    Err(AnalysisError::VariableUndeclared(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                        id.clone(),
                    ))
                }
            }

            Value::Access(_) if ctx.use_agg_funcs => Err(AnalysisError::ExpectAggExpr(
                node.attrs.pos.line,
                node.attrs.pos.col,
            )),

            Value::Access(access) => {
                let mut current = self.arena.get(access.target);

                loop {
                    match current.value {
                        Value::Id(name) => {
                            if !self.scope.entries.contains_key(name.as_str()) {
                                return Err(AnalysisError::VariableUndeclared(
                                    current.attrs.pos.line,
                                    current.attrs.pos.col,
                                    name.clone(),
                                ));
                            }

                            break;
                        }

                        Value::Access(next) => current = self.arena.get(next.target),
                        _ => unreachable!(),
                    }
                }

                self.analyze_expr(ctx, expr, Type::Unspecified)
            }

            _ => Err(AnalysisError::ExpectRecordOrSourcedProperty(
                node.attrs.pos.line,
                node.attrs.pos.col,
                self.project_type(expr),
            )),
        }
    }

    fn check_projection_on_record(
        &mut self,
        ctx: &mut CheckContext,
        record: &[Field],
    ) -> AnalysisResult<()> {
        for field in record {
            self.check_projection_on_field(ctx, field)?;
        }

        Ok(())
    }

    fn check_projection_on_field(
        &mut self,
        ctx: &mut CheckContext,
        field: &Field,
    ) -> AnalysisResult<()> {
        self.check_projection_on_field_expr(ctx, field.expr)
    }

    fn check_projection_on_field_expr(
        &mut self,
        ctx: &mut CheckContext,
        expr: ExprRef,
    ) -> AnalysisResult<()> {
        let node = self.arena.get(expr);
        match node.value {
            Value::Number(_) | Value::String(_) | Value::Bool(_) => Ok(()),

            Value::Id(id) => {
                if self.scope.entries.contains_key(id.as_str()) {
                    if ctx.use_agg_func {
                        return Err(AnalysisError::UnallowedAggFuncUsageWithSrcField(
                            node.attrs.pos.line,
                            node.attrs.pos.col,
                        ));
                    }

                    ctx.use_source_based = true;
                }

                Ok(())
            }

            Value::Array(exprs) => {
                for expr in exprs.iter().copied() {
                    self.check_projection_on_field_expr(ctx, expr)?;
                }

                Ok(())
            }

            Value::Record(fields) => {
                for field in fields {
                    self.check_projection_on_field(ctx, field)?;
                }

                Ok(())
            }

            Value::Access(access) => self.check_projection_on_field_expr(ctx, access.target),

            Value::App(app) => {
                if let Some(Type::App { aggregate, .. }) =
                    self.options.default_scope.entries.get(app.func.as_str())
                {
                    ctx.use_agg_func |= *aggregate;

                    if ctx.use_agg_func && ctx.use_source_based {
                        return Err(AnalysisError::UnallowedAggFuncUsageWithSrcField(
                            node.attrs.pos.line,
                            node.attrs.pos.col,
                        ));
                    }

                    if *aggregate {
                        return self.expect_agg_func(expr);
                    }

                    for arg in app.args.iter().copied() {
                        self.invalidate_agg_func_usage(arg)?;
                    }
                }

                Ok(())
            }

            Value::Binary(binary) => {
                self.check_projection_on_field_expr(ctx, binary.lhs)?;
                self.check_projection_on_field_expr(ctx, binary.rhs)
            }

            Value::Unary(unary) => self.check_projection_on_field_expr(ctx, unary.expr),
            Value::Group(expr) => self.check_projection_on_field_expr(ctx, *expr),
        }
    }

    fn expect_agg_func(&self, expr: ExprRef) -> AnalysisResult<()> {
        let node = self.arena.get(expr);
        if let Value::App(app) = node.value
            && let Some(Type::App {
                aggregate: true, ..
            }) = self.options.default_scope.entries.get(app.func.as_str())
        {
            for arg in app.args.iter().copied() {
                self.ensure_agg_param_is_source_bound(arg)?;
                self.invalidate_agg_func_usage(arg)?;
            }

            return Ok(());
        }

        Err(AnalysisError::ExpectAggExpr(
            node.attrs.pos.line,
            node.attrs.pos.col,
        ))
    }

    fn expect_agg_expr(&self, expr: ExprRef) -> AnalysisResult<bool> {
        let node = self.arena.get(expr);
        match node.value {
            Value::Id(id) => {
                if self.scope.entries.contains_key(id.as_str()) {
                    return Err(AnalysisError::UnallowedAggFuncUsageWithSrcField(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                    ));
                }

                Ok(false)
            }
            Value::Group(expr) => self.expect_agg_expr(*expr),
            Value::Binary(binary) => {
                let lhs = self.expect_agg_expr(binary.lhs)?;
                let rhs = self.expect_agg_expr(binary.rhs)?;

                if !lhs && !rhs {
                    return Err(AnalysisError::ExpectAggExpr(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                    ));
                }

                Ok(true)
            }
            Value::Unary(unary) => self.expect_agg_expr(unary.expr),
            Value::App(_) => {
                self.expect_agg_func(expr)?;
                Ok(true)
            }

            _ => Ok(false),
        }
    }

    fn ensure_agg_param_is_source_bound(&self, expr: ExprRef) -> AnalysisResult<()> {
        let node = self.arena.get(expr);
        match node.value {
            Value::Id(id) if !self.options.default_scope.entries.contains_key(id.as_str()) => {
                Ok(())
            }
            Value::Access(access) => self.ensure_agg_param_is_source_bound(access.target),
            Value::Binary(binary) => self.ensure_agg_binary_op_is_source_bound(node.attrs, *binary),
            Value::Unary(unary) => self.ensure_agg_param_is_source_bound(unary.expr),

            _ => Err(AnalysisError::ExpectSourceBoundProperty(
                node.attrs.pos.line,
                node.attrs.pos.col,
            )),
        }
    }

    fn ensure_agg_binary_op_is_source_bound(
        &self,
        attrs: Attrs,
        binary: Binary,
    ) -> AnalysisResult<()> {
        if !self.ensure_agg_binary_op_branch_is_source_bound(binary.lhs)
            && !self.ensure_agg_binary_op_branch_is_source_bound(binary.rhs)
        {
            return Err(AnalysisError::ExpectSourceBoundProperty(
                attrs.pos.line,
                attrs.pos.col,
            ));
        }

        Ok(())
    }

    fn ensure_agg_binary_op_branch_is_source_bound(&self, expr: ExprRef) -> bool {
        let node = self.arena.get(expr);
        match node.value {
            Value::Id(id) => !self.options.default_scope.entries.contains_key(id.as_str()),
            Value::Array(exprs) => {
                if exprs.is_empty() {
                    return false;
                }

                exprs
                    .iter()
                    .copied()
                    .all(|expr| self.ensure_agg_binary_op_branch_is_source_bound(expr))
            }
            Value::Record(fields) => {
                if fields.is_empty() {
                    return false;
                }

                fields
                    .iter()
                    .all(|field| self.ensure_agg_binary_op_branch_is_source_bound(field.expr))
            }
            Value::Access(access) => {
                self.ensure_agg_binary_op_branch_is_source_bound(access.target)
            }

            Value::Binary(binary) => self
                .ensure_agg_binary_op_is_source_bound(node.attrs, *binary)
                .is_ok(),
            Value::Unary(unary) => self.ensure_agg_binary_op_branch_is_source_bound(unary.expr),
            Value::Group(expr) => self.ensure_agg_binary_op_branch_is_source_bound(*expr),

            Value::Number(_) | Value::String(_) | Value::Bool(_) | Value::App(_) => false,
        }
    }

    fn invalidate_agg_func_usage(&self, expr: ExprRef) -> AnalysisResult<()> {
        let node = self.arena.get(expr);
        match node.value {
            Value::Number(_)
            | Value::String(_)
            | Value::Bool(_)
            | Value::Id(_)
            | Value::Access(_) => Ok(()),

            Value::Array(exprs) => {
                for expr in exprs.iter().copied() {
                    self.invalidate_agg_func_usage(expr)?;
                }

                Ok(())
            }

            Value::Record(fields) => {
                for field in fields {
                    self.invalidate_agg_func_usage(field.expr)?;
                }

                Ok(())
            }

            Value::App(app) => {
                if let Some(Type::App { aggregate, .. }) =
                    self.options.default_scope.entries.get(app.func.as_str())
                    && *aggregate
                {
                    return Err(AnalysisError::WrongAggFunUsage(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                        app.func.clone(),
                    ));
                }

                for arg in app.args.iter().copied() {
                    self.invalidate_agg_func_usage(arg)?;
                }

                Ok(())
            }

            Value::Binary(binary) => {
                self.invalidate_agg_func_usage(binary.lhs)?;
                self.invalidate_agg_func_usage(binary.rhs)
            }

            Value::Unary(unary) => self.invalidate_agg_func_usage(unary.expr),
            Value::Group(expr) => self.invalidate_agg_func_usage(*expr),
        }
    }

    fn reject_constant_func(&self, attrs: Attrs, app: &App) -> AnalysisResult<()> {
        if app.args.is_empty() {
            return Err(AnalysisError::ConstantExprInProjectIntoClause(
                attrs.pos.line,
                attrs.pos.col,
            ));
        }

        let mut errored = None;
        for arg in app.args.iter().copied() {
            if let Err(e) = self.reject_constant_expr(arg) {
                if errored.is_none() {
                    errored = Some(e);
                }

                continue;
            }

            // if at least one arg is sourced-bound is ok
            return Ok(());
        }

        Err(errored.expect("to be defined at that point"))
    }

    fn reject_constant_expr(&self, expr: ExprRef) -> AnalysisResult<()> {
        let node = self.arena.get(expr);
        match node.value {
            Value::Id(id) if self.scope.entries.contains_key(id.as_str()) => Ok(()),

            Value::Array(exprs) => {
                let mut errored = None;
                for expr in exprs.iter().copied() {
                    if let Err(e) = self.reject_constant_expr(expr) {
                        if errored.is_none() {
                            errored = Some(e);
                        }

                        continue;
                    }

                    // if at least one arg is sourced-bound is ok
                    return Ok(());
                }

                Err(errored.expect("to be defined at that point"))
            }

            Value::Record(fields) => {
                let mut errored = None;
                for field in fields {
                    if let Err(e) = self.reject_constant_expr(field.expr) {
                        if errored.is_none() {
                            errored = Some(e);
                        }

                        continue;
                    }

                    // if at least one arg is sourced-bound is ok
                    return Ok(());
                }

                Err(errored.expect("to be defined at that point"))
            }

            Value::Binary(binary) => self
                .reject_constant_expr(binary.lhs)
                .or_else(|e| self.reject_constant_expr(binary.rhs).map_err(|_| e)),

            Value::Access(access) => self.reject_constant_expr(access.target),
            Value::App(app) => self.reject_constant_func(node.attrs, app),
            Value::Unary(unary) => self.reject_constant_expr(unary.expr),
            Value::Group(expr) => self.reject_constant_expr(*expr),

            _ => Err(AnalysisError::ConstantExprInProjectIntoClause(
                node.attrs.pos.line,
                node.attrs.pos.col,
            )),
        }
    }

    /// Analyzes an expression and checks it against an expected type.
    ///
    /// This method performs type checking on an expression, verifying that all operations
    /// are type-safe and that the expression's type is compatible with the expected type.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The analysis context controlling analysis behavior
    /// * `expr` - The expression to analyze
    /// * `expect` - The expected type of the expression
    ///
    /// # Returns
    ///
    /// Returns the actual type of the expression after checking compatibility with the expected type,
    /// or an error if type checking fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use eventql_parser::prelude::{tokenize, Parser, Analysis, AnalysisContext, AnalysisOptions, Type};
    ///
    /// let tokens = tokenize("1 + 2").unwrap();
    /// let expr = Parser::new(tokens.as_slice()).parse_expr().unwrap();
    /// let options = AnalysisOptions::default();
    /// let mut analysis = Analysis::new(&options);
    ///
    /// let result = analysis.analyze_expr(&mut AnalysisContext::default(), &expr, Type::Number);
    /// assert!(result.is_ok());
    /// ```
    pub fn analyze_expr(
        &mut self,
        ctx: &mut AnalysisContext,
        expr: ExprRef,
        mut expect: Type,
    ) -> AnalysisResult<Type> {
        let node = self.arena.get(expr);
        match node.value {
            Value::Number(_) => expect.check(node.attrs, Type::Number),
            Value::String(_) => expect.check(node.attrs, Type::String),
            Value::Bool(_) => expect.check(node.attrs, Type::Bool),

            Value::Id(id) => {
                if let Some(tpe) = self.options.default_scope.entries.get(id.as_str()) {
                    expect.check(node.attrs, tpe.clone())
                } else if let Some(tpe) = self.scope.entries.get_mut(id.as_str()) {
                    let tmp = mem::take(tpe);
                    *tpe = tmp.check(node.attrs, expect)?;

                    Ok(tpe.clone())
                } else {
                    Err(AnalysisError::VariableUndeclared(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                        id.to_owned(),
                    ))
                }
            }

            Value::Array(exprs) => {
                if matches!(expect, Type::Unspecified) {
                    for expr in exprs.iter().copied() {
                        expect = self.analyze_expr(ctx, expr, expect)?;
                    }

                    return Ok(Type::Array(Box::new(expect)));
                }

                match expect {
                    Type::Array(mut expect) => {
                        for expr in exprs.iter().copied() {
                            *expect = self.analyze_expr(ctx, expr, expect.as_ref().clone())?;
                        }

                        Ok(Type::Array(expect))
                    }

                    expect => Err(AnalysisError::TypeMismatch(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                        expect,
                        self.project_type(expr),
                    )),
                }
            }

            Value::Record(fields) => {
                if matches!(expect, Type::Unspecified) {
                    let mut record = BTreeMap::new();

                    for field in fields {
                        record.insert(
                            field.name.clone(),
                            self.analyze_expr(ctx, field.expr, Type::Unspecified)?,
                        );
                    }

                    return Ok(Type::Record(record));
                }

                match expect {
                    Type::Record(mut types) if fields.len() == types.len() => {
                        for field in fields {
                            if let Some(tpe) = types.remove(field.name.as_str()) {
                                types.insert(
                                    field.name.clone(),
                                    self.analyze_expr(ctx, field.expr, tpe)?,
                                );
                            } else {
                                return Err(AnalysisError::FieldUndeclared(
                                    field.attrs.pos.line,
                                    field.attrs.pos.col,
                                    field.name.clone(),
                                ));
                            }
                        }

                        Ok(Type::Record(types))
                    }

                    expect => Err(AnalysisError::TypeMismatch(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                        expect,
                        self.project_type(expr),
                    )),
                }
            }

            Value::Access(access) => Ok(self.analyze_access(node.attrs, access.target, expect)?),

            Value::App(app) => {
                if let Some(tpe) = self.options.default_scope.entries.get(app.func.as_str())
                    && let Type::App {
                        args,
                        result,
                        aggregate,
                    } = tpe
                {
                    if !args.match_arg_count(app.args.len()) {
                        return Err(AnalysisError::FunWrongArgumentCount(
                            node.attrs.pos.line,
                            node.attrs.pos.col,
                            app.func.clone(),
                        ));
                    }

                    if *aggregate && !ctx.allow_agg_func {
                        return Err(AnalysisError::WrongAggFunUsage(
                            node.attrs.pos.line,
                            node.attrs.pos.col,
                            app.func.clone(),
                        ));
                    }

                    if *aggregate && ctx.allow_agg_func {
                        ctx.use_agg_funcs = true;
                    }

                    for (arg, tpe) in app.args.iter().copied().zip(args.values.iter().cloned()) {
                        self.analyze_expr(ctx, arg, tpe)?;
                    }

                    if matches!(expect, Type::Unspecified) {
                        Ok(result.as_ref().clone())
                    } else {
                        expect.check(node.attrs, result.as_ref().clone())
                    }
                } else {
                    Err(AnalysisError::FuncUndeclared(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                        app.func.clone(),
                    ))
                }
            }

            Value::Binary(binary) => match binary.operator {
                Operator::Add | Operator::Sub | Operator::Mul | Operator::Div => {
                    self.analyze_expr(ctx, binary.lhs, Type::Number)?;
                    self.analyze_expr(ctx, binary.rhs, Type::Number)?;
                    expect.check(node.attrs, Type::Number)
                }

                Operator::Eq
                | Operator::Neq
                | Operator::Lt
                | Operator::Lte
                | Operator::Gt
                | Operator::Gte => {
                    let lhs_expect = self.analyze_expr(ctx, binary.lhs, Type::Unspecified)?;
                    let rhs_expect = self.analyze_expr(ctx, binary.rhs, lhs_expect.clone())?;

                    // If the left side didn't have enough type information while the other did,
                    // we replay another typecheck pass on the left side if the right side was conclusive
                    if matches!(lhs_expect, Type::Unspecified)
                        && !matches!(rhs_expect, Type::Unspecified)
                    {
                        self.analyze_expr(ctx, binary.lhs, rhs_expect)?;
                    }

                    expect.check(node.attrs, Type::Bool)
                }

                Operator::Contains => {
                    let lhs_expect = self.analyze_expr(
                        ctx,
                        binary.lhs,
                        Type::Array(Box::new(Type::Unspecified)),
                    )?;

                    let lhs_assumption = match lhs_expect {
                        Type::Array(inner) => *inner,
                        other => {
                            return Err(AnalysisError::ExpectArray(
                                node.attrs.pos.line,
                                node.attrs.pos.col,
                                other,
                            ));
                        }
                    };

                    let rhs_expect = self.analyze_expr(ctx, binary.rhs, lhs_assumption.clone())?;

                    // If the left side didn't have enough type information while the other did,
                    // we replay another typecheck pass on the left side if the right side was conclusive
                    if matches!(lhs_assumption, Type::Unspecified)
                        && !matches!(rhs_expect, Type::Unspecified)
                    {
                        self.analyze_expr(ctx, binary.lhs, Type::Array(Box::new(rhs_expect)))?;
                    }

                    expect.check(node.attrs, Type::Bool)
                }

                Operator::And | Operator::Or | Operator::Xor => {
                    self.analyze_expr(ctx, binary.lhs, Type::Bool)?;
                    self.analyze_expr(ctx, binary.rhs, Type::Bool)?;

                    expect.check(node.attrs, Type::Bool)
                }

                Operator::As => {
                    let rhs = self.arena.get(binary.rhs);
                    if let Value::Id(name) = rhs.value {
                        return if let Some(tpe) = name_to_type(self.options, name) {
                            // NOTE - we could check if it's safe to convert the left branch to that type
                            Ok(tpe)
                        } else {
                            Err(AnalysisError::UnsupportedCustomType(
                                rhs.attrs.pos.line,
                                rhs.attrs.pos.col,
                                name.clone(),
                            ))
                        };
                    }

                    unreachable!(
                        "we already made sure during parsing that we can only have an ID symbol at this point"
                    )
                }

                Operator::Not => unreachable!(),
            },

            Value::Unary(unary) => match unary.operator {
                Operator::Add | Operator::Sub => {
                    self.analyze_expr(ctx, unary.expr, Type::Number)?;
                    expect.check(node.attrs, Type::Number)
                }

                Operator::Not => {
                    self.analyze_expr(ctx, unary.expr, Type::Bool)?;
                    expect.check(node.attrs, Type::Bool)
                }

                _ => unreachable!(),
            },

            Value::Group(expr) => Ok(self.analyze_expr(ctx, *expr, expect)?),
        }
    }

    fn analyze_access(
        &mut self,
        attrs: Attrs,
        access: ExprRef,
        expect: Type,
    ) -> AnalysisResult<Type> {
        struct State<A, B> {
            depth: u8,
            /// When true means we are into dynamically type object.
            dynamic: bool,
            definition: Def<A, B>,
        }

        impl<A, B> State<A, B> {
            fn new(definition: Def<A, B>) -> Self {
                Self {
                    depth: 0,
                    dynamic: false,
                    definition,
                }
            }
        }

        enum Def<A, B> {
            User(A),
            System(B),
        }

        fn go<'a>(
            scope: &'a mut Scope,
            arena: &'a ExprArena,
            sys: &'a AnalysisOptions,
            expr: ExprRef,
        ) -> AnalysisResult<State<&'a mut Type, &'a Type>> {
            let node = arena.get(expr);
            match node.value {
                Value::Id(id) => {
                    if let Some(tpe) = sys.default_scope.entries.get(id.as_str()) {
                        Ok(State::new(Def::System(tpe)))
                    } else if let Some(tpe) = scope.entries.get_mut(id.as_str()) {
                        Ok(State::new(Def::User(tpe)))
                    } else {
                        Err(AnalysisError::VariableUndeclared(
                            node.attrs.pos.line,
                            node.attrs.pos.col,
                            id.clone(),
                        ))
                    }
                }
                Value::Access(access) => {
                    let mut state = go(scope, arena, sys, access.target)?;

                    // TODO - we should consider make that field and depth configurable.
                    let is_data_field = state.depth == 0 && access.field == "data";

                    // TODO - we should consider make that behavior configurable.
                    // the `data` property is where the JSON payload is located, which means
                    // we should be lax if a property is not defined yet.
                    if !state.dynamic && is_data_field {
                        state.dynamic = true;
                    }

                    match state.definition {
                        Def::User(tpe) => {
                            if matches!(tpe, Type::Unspecified) && state.dynamic {
                                *tpe = Type::Record(BTreeMap::from([(
                                    access.field.clone(),
                                    Type::Unspecified,
                                )]));
                                return Ok(State {
                                    depth: state.depth + 1,
                                    definition: Def::User(
                                        tpe.as_record_or_panic_mut()
                                            .get_mut(access.field.as_str())
                                            .unwrap(),
                                    ),
                                    ..state
                                });
                            }

                            if let Type::Record(fields) = tpe {
                                return match fields.entry(access.field.clone()) {
                                    Entry::Vacant(entry) => {
                                        if state.dynamic || is_data_field {
                                            return Ok(State {
                                                depth: state.depth + 1,
                                                definition: Def::User(
                                                    entry.insert(Type::Unspecified),
                                                ),
                                                ..state
                                            });
                                        }

                                        Err(AnalysisError::FieldUndeclared(
                                            node.attrs.pos.line,
                                            node.attrs.pos.col,
                                            access.field.clone(),
                                        ))
                                    }

                                    Entry::Occupied(entry) => {
                                        return Ok(State {
                                            depth: state.depth + 1,
                                            definition: Def::User(entry.into_mut()),
                                            ..state
                                        });
                                    }
                                };
                            }

                            Err(AnalysisError::ExpectRecord(
                                node.attrs.pos.line,
                                node.attrs.pos.col,
                                tpe.clone(),
                            ))
                        }

                        Def::System(tpe) => {
                            if matches!(tpe, Type::Unspecified) && state.dynamic {
                                return Ok(State {
                                    depth: state.depth + 1,
                                    definition: Def::System(&Type::Unspecified),
                                    ..state
                                });
                            }

                            if let Type::Record(fields) = tpe {
                                if let Some(field) = fields.get(access.field.as_str()) {
                                    return Ok(State {
                                        depth: state.depth + 1,
                                        definition: Def::System(field),
                                        ..state
                                    });
                                }

                                return Err(AnalysisError::FieldUndeclared(
                                    node.attrs.pos.line,
                                    node.attrs.pos.col,
                                    access.field.clone(),
                                ));
                            }

                            Err(AnalysisError::ExpectRecord(
                                node.attrs.pos.line,
                                node.attrs.pos.col,
                                tpe.clone(),
                            ))
                        }
                    }
                }
                Value::Number(_)
                | Value::String(_)
                | Value::Bool(_)
                | Value::Array(_)
                | Value::Record(_)
                | Value::App(_)
                | Value::Binary(_)
                | Value::Unary(_)
                | Value::Group(_) => unreachable!(),
            }
        }

        let state = go(&mut self.scope, self.arena, self.options, access)?;

        match state.definition {
            Def::User(tpe) => {
                let tmp = mem::take(tpe);
                *tpe = tmp.check(attrs, expect)?;

                Ok(tpe.clone())
            }

            Def::System(tpe) => tpe.clone().check(attrs, expect),
        }
    }

    fn projection_type(&self, query: &Query<Typed>) -> Type {
        self.project_type(query.projection)
    }

    fn project_type(&self, node: ExprRef) -> Type {
        match self.arena.get(node).value {
            Value::Number(_) => Type::Number,
            Value::String(_) => Type::String,
            Value::Bool(_) => Type::Bool,
            Value::Id(id) => {
                if let Some(tpe) = self.options.default_scope.entries.get(id.as_str()) {
                    tpe.clone()
                } else if let Some(tpe) = self.scope.entries.get(id.as_str()) {
                    tpe.clone()
                } else {
                    Type::Unspecified
                }
            }
            Value::Array(exprs) => {
                let mut project = Type::Unspecified;

                for expr in exprs.iter().copied() {
                    let tmp = self.project_type(expr);

                    if !matches!(tmp, Type::Unspecified) {
                        project = tmp;
                        break;
                    }
                }

                Type::Array(Box::new(project))
            }
            Value::Record(fields) => Type::Record(
                fields
                    .iter()
                    .map(|field| (field.name.clone(), self.project_type(field.expr)))
                    .collect(),
            ),
            Value::Access(access) => {
                let tpe = self.project_type(access.target);
                if let Type::Record(fields) = tpe {
                    fields
                        .get(access.field.as_str())
                        .cloned()
                        .unwrap_or_default()
                } else {
                    Type::Unspecified
                }
            }
            Value::App(app) => self
                .options
                .default_scope
                .entries
                .get(app.func.as_str())
                .cloned()
                .unwrap_or_default(),
            Value::Binary(binary) => match binary.operator {
                Operator::Add | Operator::Sub | Operator::Mul | Operator::Div => Type::Number,
                Operator::As => {
                    if let Value::Id(n) = self.arena.get(binary.rhs).value
                        && let Some(tpe) = name_to_type(self.options, n.as_str())
                    {
                        tpe
                    } else {
                        Type::Unspecified
                    }
                }
                Operator::Eq
                | Operator::Neq
                | Operator::Lt
                | Operator::Lte
                | Operator::Gt
                | Operator::Gte
                | Operator::And
                | Operator::Or
                | Operator::Xor
                | Operator::Not
                | Operator::Contains => Type::Bool,
            },
            Value::Unary(unary) => match unary.operator {
                Operator::Add | Operator::Sub => Type::Number,
                Operator::Mul
                | Operator::Div
                | Operator::Eq
                | Operator::Neq
                | Operator::Lt
                | Operator::Lte
                | Operator::Gt
                | Operator::Gte
                | Operator::And
                | Operator::Or
                | Operator::Xor
                | Operator::Not
                | Operator::Contains
                | Operator::As => unreachable!(),
            },
            Value::Group(expr) => self.project_type(*expr),
        }
    }
}

/// Converts a type name string to its corresponding [`Type`] variant.
///
/// This function performs case-insensitive matching for built-in type names and checks
/// against custom types defined in the analysis options.
///
/// # Returns
///
/// * `Some(Type)` - If the name matches a built-in type or custom type
/// * `None` - If the name doesn't match any known type
///
/// # Built-in Type Mappings
///
/// The following type names are recognized (case-insensitive):
/// - `"string"` → [`Type::String`]
/// - `"int"` or `"float64"` → [`Type::Number`]
/// - `"boolean"` → [`Type::Bool`]
/// - `"date"` → [`Type::Date`]
/// - `"time"` → [`Type::Time`]
/// - `"datetime"` → [`Type::DateTime`]
///
/// # Examples
///
/// ```
/// use eventql_parser::Type;
/// use eventql_parser::prelude::{AnalysisOptions, name_to_type};
///
/// let opts = AnalysisOptions::default();
/// assert!(matches!(name_to_type(&opts, "String"), Some(Type::String)));
/// assert!(matches!(name_to_type(&opts, "INT"), Some(Type::Number)));
/// assert!(name_to_type(&opts, "unknown").is_none());
/// ```
pub fn name_to_type(opts: &AnalysisOptions, name: &str) -> Option<Type> {
    if name.eq_ignore_ascii_case("string") {
        Some(Type::String)
    } else if name.eq_ignore_ascii_case("int") || name.eq_ignore_ascii_case("float64") {
        Some(Type::Number)
    } else if name.eq_ignore_ascii_case("boolean") {
        Some(Type::Bool)
    } else if name.eq_ignore_ascii_case("date") {
        Some(Type::Date)
    } else if name.eq_ignore_ascii_case("time") {
        Some(Type::Time)
    } else if name.eq_ignore_ascii_case("datetime") {
        Some(Type::DateTime)
    } else if opts.custom_types.contains(&Ascii::new(name.to_owned())) {
        // ^ Sad we have to allocate here for no reason
        Some(Type::Custom(name.to_owned()))
    } else {
        None
    }
}
