use rustc_hash::FxHashMap;
use serde::Serialize;
use std::{collections::HashSet, mem};

use crate::arena::Arena;
use crate::typing::{Record, Type};
use crate::{
    App, Attrs, Binary, ExprRef, Field, Query, Raw, RecRef, Source, SourceKind, StrRef, Value,
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
pub type AnalysisResult<A> = Result<A, AnalysisError>;

/// Configuration options for static analysis.
///
/// This structure contains the type information needed to perform static analysis
/// on EventQL queries, including the default scope with built-in functions and
/// the type information for event records.
#[derive(Default)]
pub struct AnalysisOptions {
    /// The default scope containing built-in functions and their type signatures.
    pub default_scope: Scope,
    /// Type information for event records being queried.
    pub default_event_type: Type,
    /// Custom types that are not defined in the EventQL reference.
    ///
    /// This set allows users to register custom type names that can be used
    /// in type conversion expressions (e.g., `field AS CustomType`). Custom
    /// type names are case-insensitive.
    pub custom_types: HashSet<StrRef>,

    /// Per-data-source type overrides.
    ///
    /// When a query targets a named data source, this map is checked first. If a match is
    /// found, the associated type is used instead of [`default_event_type`](AnalysisOptions::default_event_type).
    /// Keys are case-insensitive data source names.
    pub data_sources: FxHashMap<StrRef, Type>,
}

/// Represents a variable scope during static analysis.
///
/// A scope tracks the variables and their types that are currently in scope
/// during type checking. This is used to resolve variable references and
/// ensure type correctness.
#[derive(Default, Clone, Serialize, Debug)]
pub struct Scope {
    #[serde(skip_serializing)]
    entries: FxHashMap<StrRef, Type>,
}

impl Scope {
    /// Checks if the scope contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Declares a new variable binding in this scope.
    ///
    /// Returns `true` if the binding was newly inserted, or `false` if a binding
    /// with the same name already existed (in which case the old value is replaced).
    pub fn declare(&mut self, name: StrRef, tpe: Type) -> bool {
        self.entries.insert(name, tpe).is_none()
    }

    /// Looks up the type of a variable by name.
    ///
    /// Returns `None` if the variable is not declared in this scope.
    pub fn get(&self, name: StrRef) -> Option<Type> {
        self.entries.get(&name).copied()
    }

    /// Returns a mutable reference to the type of a variable, allowing in-place updates.
    ///
    /// Returns `None` if the variable is not declared in this scope.
    pub fn get_mut(&mut self, name: StrRef) -> Option<&mut Type> {
        self.entries.get_mut(&name)
    }

    /// Returns `true` if a variable with the given name is declared in this scope.
    pub fn exists(&self, name: StrRef) -> bool {
        self.entries.contains_key(&name)
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
    arena: &'a mut Arena,
    /// The analysis options containing type information for functions and event types.
    options: &'a AnalysisOptions,
    /// Stack of previous scopes for nested scope handling.
    prev_scopes: Vec<Scope>,
    /// The current scope containing variable bindings and their types.
    scope: Scope,
}

impl<'a> Analysis<'a> {
    /// Creates a new analysis instance with the given options.
    pub fn new(arena: &'a mut Arena, options: &'a AnalysisOptions) -> Self {
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

    #[cfg(test)]
    pub fn test_declare(&mut self, name: &str, tpe: Type) -> bool {
        let name = self.arena.strings.alloc_no_case(name);
        self.scope.declare(name, tpe)
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
    /// use eventql_parser::Session;
    ///
    /// let mut session = Session::builder().build();
    /// let query = session.parse("FROM e IN events WHERE [1,2,3] CONTAINS e.data.price PROJECT INTO e").unwrap();
    ///
    /// let typed_query = session.run_static_analysis(query);
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
            let node = self.arena.exprs.get(group_by.expr);
            if !matches!(node.value, Value::Access(_) | Value::Id(_)) {
                return Err(AnalysisError::ExpectFieldLiteral(
                    node.attrs.pos.line,
                    node.attrs.pos.col,
                ));
            }

            self.analyze_expr(&mut ctx, group_by.expr, Type::Unspecified)?;

            if let Some(expr) = group_by.predicate {
                ctx.allow_agg_func = true;
                ctx.use_agg_funcs = true;

                self.analyze_expr(&mut ctx, expr, Type::Bool)?;

                let node = self.arena.exprs.get(expr);
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
            let node = self.arena.exprs.get(order_by.expr);
            if query.group_by.is_none() && !matches!(node.value, Value::Access(_) | Value::Id(_)) {
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
            SourceKind::Name(name) => {
                let tpe = if let Some(tpe) = self.options.data_sources.get(name).copied() {
                    tpe
                } else {
                    self.options.default_event_type
                };

                self.arena.types.alloc_type(tpe)
            }

            SourceKind::Subject(_) => self.arena.types.alloc_type(self.options.default_event_type),

            SourceKind::Subquery(query) => self.projection_type(query),
        };

        if !self.scope.declare(source.binding.name, tpe) {
            return Err(AnalysisError::BindingAlreadyExists(
                source.binding.pos.line,
                source.binding.pos.col,
                self.arena.strings.get(source.binding.name).to_owned(),
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
        let node = self.arena.exprs.get(expr);
        match node.value {
            Value::Record(record) => {
                if self.arena.exprs.rec(record).is_empty() {
                    return Err(AnalysisError::EmptyRecord(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                    ));
                }

                ctx.allow_agg_func = true;
                let tpe = self.analyze_expr(ctx, expr, Type::Unspecified)?;
                let mut chk_ctx = CheckContext {
                    use_agg_func: ctx.use_agg_funcs,
                    ..Default::default()
                };

                self.check_projection_on_record(&mut chk_ctx, record)?;
                Ok(tpe)
            }

            Value::App(app) => {
                ctx.allow_agg_func = true;

                let tpe = self.analyze_expr(ctx, expr, Type::Unspecified)?;

                if ctx.use_agg_funcs {
                    let mut chk_ctx = CheckContext {
                        use_agg_func: ctx.use_agg_funcs,
                        ..Default::default()
                    };

                    self.check_projection_on_field_expr(&mut chk_ctx, expr)?;
                } else {
                    self.reject_constant_func(node.attrs, &app)?;
                }

                Ok(tpe)
            }

            Value::Id(_) if ctx.use_agg_funcs => Err(AnalysisError::ExpectAggExpr(
                node.attrs.pos.line,
                node.attrs.pos.col,
            )),

            Value::Id(id) => {
                if let Some(tpe) = self.scope.get(id) {
                    Ok(tpe)
                } else {
                    Err(AnalysisError::VariableUndeclared(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                        self.arena.strings.get(id).to_owned(),
                    ))
                }
            }

            Value::Access(_) if ctx.use_agg_funcs => Err(AnalysisError::ExpectAggExpr(
                node.attrs.pos.line,
                node.attrs.pos.col,
            )),

            Value::Access(access) => {
                let mut current = self.arena.exprs.get(access.target);

                loop {
                    match current.value {
                        Value::Id(name) => {
                            if !self.scope.exists(name) {
                                return Err(AnalysisError::VariableUndeclared(
                                    current.attrs.pos.line,
                                    current.attrs.pos.col,
                                    self.arena.strings.get(name).to_owned(),
                                ));
                            }

                            break;
                        }

                        Value::Access(next) => current = self.arena.exprs.get(next.target),
                        _ => unreachable!(),
                    }
                }

                self.analyze_expr(ctx, expr, Type::Unspecified)
            }

            _ => {
                let tpe = self.project_type(expr);

                Err(AnalysisError::ExpectRecordOrSourcedProperty(
                    node.attrs.pos.line,
                    node.attrs.pos.col,
                    display_type(self.arena, tpe),
                ))
            }
        }
    }

    fn check_projection_on_record(
        &mut self,
        ctx: &mut CheckContext,
        record: RecRef,
    ) -> AnalysisResult<()> {
        for idx in 0..self.arena.exprs.rec(record).len() {
            let field = self.arena.exprs.rec_get(record, idx);

            self.check_projection_on_field(ctx, &field)?;
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
        let node = self.arena.exprs.get(expr);
        match node.value {
            Value::Number(_) | Value::String(_) | Value::Bool(_) => Ok(()),

            Value::Id(id) => {
                if self.scope.exists(id) {
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
                for idx in self.arena.exprs.vec_idxes(exprs) {
                    let expr = self.arena.exprs.vec_get(exprs, idx);

                    self.check_projection_on_field_expr(ctx, expr)?;
                }

                Ok(())
            }

            Value::Record(fields) => {
                for idx in self.arena.exprs.rec_idxes(fields) {
                    let field = self.arena.exprs.rec_get(fields, idx);

                    self.check_projection_on_field(ctx, &field)?;
                }

                Ok(())
            }

            Value::Access(access) => self.check_projection_on_field_expr(ctx, access.target),

            Value::App(app) => {
                if let Some(Type::App { aggregate, .. }) = self.options.default_scope.get(app.func)
                {
                    ctx.use_agg_func |= aggregate;

                    if ctx.use_agg_func && ctx.use_source_based {
                        return Err(AnalysisError::UnallowedAggFuncUsageWithSrcField(
                            node.attrs.pos.line,
                            node.attrs.pos.col,
                        ));
                    }

                    if aggregate {
                        return self.expect_agg_func(expr);
                    }

                    for idx in self.arena.exprs.vec_idxes(app.args) {
                        let arg = self.arena.exprs.vec_get(app.args, idx);

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
            Value::Group(expr) => self.check_projection_on_field_expr(ctx, expr),
        }
    }

    fn expect_agg_func(&self, expr: ExprRef) -> AnalysisResult<()> {
        let node = self.arena.exprs.get(expr);
        if let Value::App(app) = node.value
            && let Some(Type::App {
                aggregate: true, ..
            }) = self.options.default_scope.get(app.func)
        {
            for idx in 0..self.arena.exprs.vec(app.args).len() {
                let arg = self.arena.exprs.vec_get(app.args, idx);

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
        let node = self.arena.exprs.get(expr);
        match node.value {
            Value::Id(id) => {
                if self.scope.exists(id) {
                    return Err(AnalysisError::UnallowedAggFuncUsageWithSrcField(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                    ));
                }

                Ok(false)
            }
            Value::Group(expr) => self.expect_agg_expr(expr),
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
        let node = self.arena.exprs.get(expr);
        match node.value {
            Value::Id(id) if !self.options.default_scope.exists(id) => Ok(()),
            Value::Access(access) => self.ensure_agg_param_is_source_bound(access.target),
            Value::Binary(binary) => self.ensure_agg_binary_op_is_source_bound(node.attrs, binary),
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
        let node = self.arena.exprs.get(expr);
        match node.value {
            Value::Id(id) => !self.options.default_scope.exists(id),
            Value::Array(exprs) => {
                if self.arena.exprs.vec(exprs).is_empty() {
                    return false;
                }

                for idx in 0..self.arena.exprs.vec(exprs).len() {
                    let expr = self.arena.exprs.vec_get(exprs, idx);

                    if !self.ensure_agg_binary_op_branch_is_source_bound(expr) {
                        return false;
                    }
                }

                true
            }
            Value::Record(fields) => {
                if self.arena.exprs.rec(fields).is_empty() {
                    return false;
                }

                for idx in 0..self.arena.exprs.rec(fields).len() {
                    let field = self.arena.exprs.rec_get(fields, idx);

                    if !self.ensure_agg_binary_op_branch_is_source_bound(field.expr) {
                        return false;
                    }
                }

                true
            }

            Value::Access(access) => {
                self.ensure_agg_binary_op_branch_is_source_bound(access.target)
            }

            Value::Binary(binary) => self
                .ensure_agg_binary_op_is_source_bound(node.attrs, binary)
                .is_ok(),
            Value::Unary(unary) => self.ensure_agg_binary_op_branch_is_source_bound(unary.expr),
            Value::Group(expr) => self.ensure_agg_binary_op_branch_is_source_bound(expr),

            Value::Number(_) | Value::String(_) | Value::Bool(_) | Value::App(_) => false,
        }
    }

    fn invalidate_agg_func_usage(&self, expr: ExprRef) -> AnalysisResult<()> {
        let node = self.arena.exprs.get(expr);
        match node.value {
            Value::Number(_)
            | Value::String(_)
            | Value::Bool(_)
            | Value::Id(_)
            | Value::Access(_) => Ok(()),

            Value::Array(exprs) => {
                for idx in 0..self.arena.exprs.vec(exprs).len() {
                    let expr = self.arena.exprs.vec_get(exprs, idx);

                    self.invalidate_agg_func_usage(expr)?;
                }

                Ok(())
            }

            Value::Record(fields) => {
                for idx in 0..self.arena.exprs.rec(fields).len() {
                    let field = self.arena.exprs.rec_get(fields, idx);

                    self.invalidate_agg_func_usage(field.expr)?;
                }

                Ok(())
            }

            Value::App(app) => {
                if let Some(Type::App { aggregate, .. }) = self.options.default_scope.get(app.func)
                    && aggregate
                {
                    return Err(AnalysisError::WrongAggFunUsage(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                        self.arena.strings.get(app.func).to_owned(),
                    ));
                }

                for idx in 0..self.arena.exprs.vec(app.args).len() {
                    let arg = self.arena.exprs.vec_get(app.args, idx);
                    self.invalidate_agg_func_usage(arg)?;
                }

                Ok(())
            }

            Value::Binary(binary) => {
                self.invalidate_agg_func_usage(binary.lhs)?;
                self.invalidate_agg_func_usage(binary.rhs)
            }

            Value::Unary(unary) => self.invalidate_agg_func_usage(unary.expr),
            Value::Group(expr) => self.invalidate_agg_func_usage(expr),
        }
    }

    fn reject_constant_func(&self, attrs: Attrs, app: &App) -> AnalysisResult<()> {
        if self.arena.exprs.vec(app.args).is_empty() {
            return Err(AnalysisError::ConstantExprInProjectIntoClause(
                attrs.pos.line,
                attrs.pos.col,
            ));
        }

        let mut errored = None;
        for idx in 0..self.arena.exprs.vec(app.args).len() {
            let arg = self.arena.exprs.vec_get(app.args, idx);

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
        let node = self.arena.exprs.get(expr);
        match node.value {
            Value::Id(id) if self.scope.exists(id) => Ok(()),
            Value::Array(exprs) => {
                let mut errored = None;
                for idx in 0..self.arena.exprs.vec(exprs).len() {
                    let expr = self.arena.exprs.vec_get(exprs, idx);

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
                for idx in 0..self.arena.exprs.rec(fields).len() {
                    let field = self.arena.exprs.rec_get(fields, idx);

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
            Value::App(app) => self.reject_constant_func(node.attrs, &app),
            Value::Unary(unary) => self.reject_constant_expr(unary.expr),
            Value::Group(expr) => self.reject_constant_expr(expr),

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
    /// use eventql_parser::Session;
    ///
    /// let mut session = Session::builder().build();
    /// let query = session.parse("FROM e IN events PROJECT INTO { price: 1 + 2 }").unwrap();
    ///
    /// let result = session.run_static_analysis(query);
    /// assert!(result.is_ok());
    /// ```
    pub fn analyze_expr(
        &mut self,
        ctx: &mut AnalysisContext,
        expr: ExprRef,
        mut expect: Type,
    ) -> AnalysisResult<Type> {
        let node = self.arena.exprs.get(expr);
        match node.value {
            Value::Number(_) => self.arena.type_check(node.attrs, expect, Type::Number),
            Value::String(_) => self.arena.type_check(node.attrs, expect, Type::String),
            Value::Bool(_) => self.arena.type_check(node.attrs, expect, Type::Bool),

            Value::Id(id) => {
                if let Some(tpe) = self.options.default_scope.get(id) {
                    self.arena.type_check(node.attrs, expect, tpe)
                } else if let Some(tpe) = self.scope.get_mut(id) {
                    *tpe = self.arena.type_check(node.attrs, mem::take(tpe), expect)?;

                    Ok(*tpe)
                } else {
                    Err(AnalysisError::VariableUndeclared(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                        self.arena.strings.get(id).to_owned(),
                    ))
                }
            }

            Value::Array(exprs) => {
                if matches!(expect, Type::Unspecified) {
                    for idx in self.arena.exprs.vec_idxes(exprs) {
                        let expr = self.arena.exprs.vec_get(exprs, idx);

                        expect = self.analyze_expr(ctx, expr, expect)?;
                    }

                    return Ok(self.arena.types.alloc_array_of(expect));
                }

                match expect {
                    Type::Array(expect) => {
                        let mut expect = self.arena.types.get_type(expect);
                        for idx in 0..self.arena.exprs.vec(exprs).len() {
                            let expr = self.arena.exprs.vec_get(exprs, idx);
                            expect = self.analyze_expr(ctx, expr, expect)?;
                        }

                        Ok(self.arena.types.alloc_array_of(expect))
                    }

                    expect => {
                        let tpe = self.project_type(expr);

                        Err(AnalysisError::TypeMismatch(
                            node.attrs.pos.line,
                            node.attrs.pos.col,
                            display_type(self.arena, expect),
                            display_type(self.arena, tpe),
                        ))
                    }
                }
            }

            Value::Record(fields) => {
                if matches!(expect, Type::Unspecified) {
                    let mut record = FxHashMap::default();

                    for idx in 0..self.arena.exprs.rec(fields).len() {
                        let field = self.arena.exprs.rec_get(fields, idx);

                        record.insert(
                            field.name,
                            self.analyze_expr(ctx, field.expr, Type::Unspecified)?,
                        );
                    }

                    return Ok(Type::Record(self.arena.types.alloc_record(record)));
                }

                if let Type::Record(rec) = expect
                    && self.arena.types.record_len(rec) == self.arena.exprs.rec(fields).len()
                {
                    for idx in self.arena.exprs.rec_idxes(fields) {
                        let field = self.arena.exprs.rec_get(fields, idx);

                        if let Some(tpe) = self.arena.types.record_get(rec, field.name) {
                            let new_tpe = self.analyze_expr(ctx, field.expr, tpe)?;
                            self.arena.types.record_set(rec, field.name, new_tpe);
                            continue;
                        }

                        return Err(AnalysisError::FieldUndeclared(
                            field.attrs.pos.line,
                            field.attrs.pos.col,
                            self.arena.strings.get(field.name).to_owned(),
                        ));
                    }

                    return Ok(expect);
                }

                let tpe = self.project_type(expr);

                Err(AnalysisError::TypeMismatch(
                    node.attrs.pos.line,
                    node.attrs.pos.col,
                    display_type(self.arena, expect),
                    display_type(self.arena, tpe),
                ))
            }

            Value::Access(_) => Ok(self.analyze_access(node.attrs, expr, expect)?),

            Value::App(app) => {
                if let Some(tpe) = self.options.default_scope.get(app.func)
                    && let Type::App {
                        args,
                        result,
                        aggregate,
                    } = tpe
                {
                    let args_actual_len = self.arena.exprs.vec(app.args).len();
                    let args_decl_len = self.arena.types.get_args(args.values).len();

                    if !(args_actual_len >= args.needed && args_actual_len <= args_decl_len) {
                        return Err(AnalysisError::FunWrongArgumentCount(
                            node.attrs.pos.line,
                            node.attrs.pos.col,
                            self.arena.strings.get(app.func).to_owned(),
                        ));
                    }

                    if aggregate && !ctx.allow_agg_func {
                        return Err(AnalysisError::WrongAggFunUsage(
                            node.attrs.pos.line,
                            node.attrs.pos.col,
                            self.arena.strings.get(app.func).to_owned(),
                        ));
                    }

                    if aggregate && ctx.allow_agg_func {
                        ctx.use_agg_funcs = true;
                    }

                    let arg_types = self.arena.types.args_idxes(args.values);
                    let args_idxes = self.arena.exprs.vec_idxes(app.args);
                    for (val_idx, tpe_idx) in args_idxes.zip(arg_types) {
                        let arg = self.arena.exprs.vec_get(app.args, val_idx);
                        let tpe = self.arena.types.args_get(args.values, tpe_idx);

                        self.analyze_expr(ctx, arg, tpe)?;
                    }

                    if matches!(expect, Type::Unspecified) {
                        Ok(self.arena.types.get_type(result))
                    } else {
                        self.arena
                            .type_check(node.attrs, expect, self.arena.types.get_type(result))
                    }
                } else {
                    Err(AnalysisError::FuncUndeclared(
                        node.attrs.pos.line,
                        node.attrs.pos.col,
                        self.arena.strings.get(app.func).to_owned(),
                    ))
                }
            }

            Value::Binary(binary) => match binary.operator {
                Operator::Add | Operator::Sub | Operator::Mul | Operator::Div => {
                    self.analyze_expr(ctx, binary.lhs, Type::Number)?;
                    self.analyze_expr(ctx, binary.rhs, Type::Number)?;
                    self.arena.type_check(node.attrs, expect, Type::Number)
                }

                Operator::Eq
                | Operator::Neq
                | Operator::Lt
                | Operator::Lte
                | Operator::Gt
                | Operator::Gte => {
                    let lhs_expect = self.analyze_expr(ctx, binary.lhs, Type::Unspecified)?;
                    let rhs_expect = self.analyze_expr(ctx, binary.rhs, lhs_expect)?;

                    // If the left side didn't have enough type information while the other did,
                    // we replay another typecheck pass on the left side if the right side was conclusive
                    if matches!(lhs_expect, Type::Unspecified)
                        && !matches!(rhs_expect, Type::Unspecified)
                    {
                        self.analyze_expr(ctx, binary.lhs, rhs_expect)?;
                    }

                    self.arena.type_check(node.attrs, expect, Type::Bool)
                }

                Operator::Contains => {
                    let new_expect = self.arena.types.alloc_array_of(Type::Unspecified);
                    let lhs_expect = self.analyze_expr(ctx, binary.lhs, new_expect)?;

                    let lhs_assumption = match lhs_expect {
                        Type::Array(inner) => self.arena.types.get_type(inner),
                        other => {
                            return Err(AnalysisError::ExpectArray(
                                node.attrs.pos.line,
                                node.attrs.pos.col,
                                display_type(self.arena, other),
                            ));
                        }
                    };

                    let rhs_expect = self.analyze_expr(ctx, binary.rhs, lhs_assumption)?;

                    // If the left side didn't have enough type information while the other did,
                    // we replay another typecheck pass on the left side if the right side was conclusive
                    if matches!(lhs_assumption, Type::Unspecified)
                        && !matches!(rhs_expect, Type::Unspecified)
                    {
                        let new_expect = self.arena.types.alloc_array_of(rhs_expect);
                        self.analyze_expr(ctx, binary.lhs, new_expect)?;
                    }

                    self.arena.type_check(node.attrs, expect, Type::Bool)
                }

                Operator::And | Operator::Or | Operator::Xor => {
                    self.analyze_expr(ctx, binary.lhs, Type::Bool)?;
                    self.analyze_expr(ctx, binary.rhs, Type::Bool)?;
                    self.arena.type_check(node.attrs, expect, Type::Bool)
                }

                Operator::As => {
                    let rhs = self.arena.exprs.get(binary.rhs);
                    if let Value::Id(name) = rhs.value {
                        return if let Some(tpe) = resolve_type(self.arena, self.options, name) {
                            // NOTE - we could check if it's safe to convert the left branch to that type
                            Ok(tpe)
                        } else {
                            Err(AnalysisError::UnsupportedCustomType(
                                rhs.attrs.pos.line,
                                rhs.attrs.pos.col,
                                self.arena.strings.get(name).to_owned(),
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
                    self.arena.type_check(node.attrs, expect, Type::Number)
                }

                Operator::Not => {
                    self.analyze_expr(ctx, unary.expr, Type::Bool)?;
                    self.arena.type_check(node.attrs, expect, Type::Bool)
                }

                _ => unreachable!(),
            },

            Value::Group(expr) => Ok(self.analyze_expr(ctx, expr, expect)?),
        }
    }

    fn analyze_access(
        &mut self,
        attrs: Attrs,
        access: ExprRef,
        expect: Type,
    ) -> AnalysisResult<Type> {
        struct State {
            depth: u8,
            /// When true means we are into dynamically type object.
            dynamic: bool,
            definition: Def,
        }

        impl State {
            fn new(definition: Def) -> Self {
                Self {
                    depth: 0,
                    dynamic: false,
                    definition,
                }
            }
        }

        #[derive(Copy, Clone)]
        struct Parent {
            record: Record,
            field: Option<StrRef>,
        }

        enum Def {
            User { parent: Parent, tpe: Type },
            System(Type),
        }

        fn go<'global>(
            scope: &mut Scope,
            arena: &'global mut Arena,
            sys: &'global AnalysisOptions,
            expr: ExprRef,
        ) -> AnalysisResult<State> {
            let node = arena.exprs.get(expr);
            match node.value {
                Value::Id(id) => {
                    if let Some(tpe) = sys.default_scope.get(id) {
                        if matches!(tpe, Type::Record(_)) {
                            Ok(State::new(Def::System(tpe)))
                        } else {
                            Err(AnalysisError::ExpectRecord(
                                node.attrs.pos.line,
                                node.attrs.pos.col,
                                display_type(arena, tpe),
                            ))
                        }
                    } else if let Some(tpe) = scope.get_mut(id) {
                        if matches!(tpe, Type::Unspecified) {
                            let record = arena.types.instantiate_record();
                            *tpe = Type::Record(record);

                            Ok(State::new(Def::User {
                                parent: Parent {
                                    record,
                                    field: None,
                                },
                                tpe: Type::Record(record),
                            }))
                        } else if let Type::Record(record) = *tpe {
                            Ok(State::new(Def::User {
                                parent: Parent {
                                    record,
                                    field: None,
                                },
                                tpe: *tpe,
                            }))
                        } else {
                            Err(AnalysisError::ExpectRecord(
                                node.attrs.pos.line,
                                node.attrs.pos.col,
                                display_type(arena, *tpe),
                            ))
                        }
                    } else {
                        Err(AnalysisError::VariableUndeclared(
                            node.attrs.pos.line,
                            node.attrs.pos.col,
                            arena.strings.get(id).to_owned(),
                        ))
                    }
                }
                Value::Access(access) => {
                    let mut state = go(scope, arena, sys, access.target)?;

                    // TODO - we should consider make that field and depth configurable.
                    let is_data_field =
                        state.depth == 0 && arena.strings.get(access.field) == "data";

                    // TODO - we should consider make that behavior configurable.
                    // the `data` property is where the JSON payload is located, which means
                    // we should be lax if a property is not defined yet.
                    if !state.dynamic && is_data_field {
                        state.dynamic = true;
                    }

                    match state.definition {
                        Def::User { parent, tpe } => {
                            if matches!(tpe, Type::Unspecified) && state.dynamic {
                                let record = arena.types.instantiate_record();
                                arena
                                    .types
                                    .record_set(record, access.field, Type::Unspecified);

                                // TODO - this is impossible. Should return a proper error instead of panicking
                                if let Some(field) = parent.field {
                                    arena.types.record_set(
                                        parent.record,
                                        field,
                                        Type::Record(record),
                                    );
                                }

                                return Ok(State {
                                    depth: state.depth + 1,
                                    definition: Def::User {
                                        parent: Parent {
                                            record,
                                            field: Some(access.field),
                                        },
                                        tpe: Type::Unspecified,
                                    },
                                    ..state
                                });
                            } else if let Type::Record(record) = tpe {
                                return if let Some(tpe) =
                                    arena.types.record_get(record, access.field)
                                {
                                    Ok(State {
                                        depth: state.depth + 1,
                                        definition: Def::User {
                                            parent: Parent {
                                                record,
                                                field: Some(access.field),
                                            },
                                            tpe,
                                        },
                                        ..state
                                    })
                                } else {
                                    // TODO - that test seems useless because it can't be the data field and not be dynamic
                                    if state.dynamic || is_data_field {
                                        arena.types.record_set(
                                            record,
                                            access.field,
                                            Type::Unspecified,
                                        );
                                        return Ok(State {
                                            depth: state.depth + 1,
                                            definition: Def::User {
                                                parent: Parent {
                                                    record,
                                                    field: Some(access.field),
                                                },
                                                tpe: Type::Unspecified,
                                            },
                                            ..state
                                        });
                                    }

                                    Err(AnalysisError::FieldUndeclared(
                                        node.attrs.pos.line,
                                        node.attrs.pos.col,
                                        arena.strings.get(access.field).to_owned(),
                                    ))
                                };
                            }

                            Err(AnalysisError::ExpectRecord(
                                node.attrs.pos.line,
                                node.attrs.pos.col,
                                display_type(arena, tpe),
                            ))
                        }

                        Def::System(tpe) => {
                            if matches!(tpe, Type::Unspecified) && state.dynamic {
                                return Ok(State {
                                    depth: state.depth + 1,
                                    definition: Def::System(Type::Unspecified),
                                    ..state
                                });
                            }

                            if let Type::Record(rec) = tpe {
                                if let Some(field) = arena.types.record_get(rec, access.field) {
                                    return Ok(State {
                                        depth: state.depth + 1,
                                        definition: Def::System(field),
                                        ..state
                                    });
                                }

                                return Err(AnalysisError::FieldUndeclared(
                                    node.attrs.pos.line,
                                    node.attrs.pos.col,
                                    arena.strings.get(access.field).to_owned(),
                                ));
                            }

                            Err(AnalysisError::ExpectRecord(
                                node.attrs.pos.line,
                                node.attrs.pos.col,
                                display_type(arena, tpe),
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
            Def::User { parent, tpe } => {
                let new_tpe = self.arena.type_check(attrs, tpe, expect)?;

                if let Some(field) = parent.field {
                    self.arena.types.record_set(parent.record, field, new_tpe);
                }

                Ok(new_tpe)
            }

            Def::System(tpe) => self.arena.type_check(attrs, tpe, expect),
        }
    }

    fn projection_type(&mut self, query: &Query<Typed>) -> Type {
        self.project_type(query.projection)
    }

    fn project_type(&mut self, node: ExprRef) -> Type {
        match self.arena.exprs.get(node).value {
            Value::Number(_) => Type::Number,
            Value::String(_) => Type::String,
            Value::Bool(_) => Type::Bool,
            Value::Id(id) => {
                if let Some(tpe) = self.options.default_scope.get(id) {
                    tpe
                } else if let Some(tpe) = self.scope.get(id) {
                    tpe
                } else {
                    Type::Unspecified
                }
            }
            Value::Array(exprs) => {
                let mut project = Type::Unspecified;

                for idx in self.arena.exprs.vec_idxes(exprs) {
                    let expr = self.arena.exprs.vec_get(exprs, idx);
                    let tmp = self.project_type(expr);

                    if !matches!(tmp, Type::Unspecified) {
                        project = tmp;
                        break;
                    }
                }

                self.arena.types.alloc_array_of(project)
            }
            Value::Record(fields) => {
                let mut props = FxHashMap::default();

                for idx in self.arena.exprs.rec_idxes(fields) {
                    let field = self.arena.exprs.rec_get(fields, idx);
                    let tpe = self.project_type(field.expr);
                    props.insert(field.name, tpe);
                }

                Type::Record(self.arena.types.alloc_record(props))
            }
            Value::Access(access) => {
                let tpe = self.project_type(access.target);
                if let Type::Record(record) = tpe {
                    self.arena
                        .types
                        .record_get(record, access.field)
                        .unwrap_or_default()
                } else {
                    Type::Unspecified
                }
            }
            Value::App(app) => self.options.default_scope.get(app.func).unwrap_or_default(),
            Value::Binary(binary) => match binary.operator {
                Operator::Add | Operator::Sub | Operator::Mul | Operator::Div => Type::Number,
                Operator::As => {
                    if let Value::Id(n) = self.arena.exprs.get(binary.rhs).value
                        && let Some(tpe) = resolve_type(self.arena, self.options, n)
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
            Value::Group(expr) => self.project_type(expr),
        }
    }
}

impl Arena {
    /// Checks if two types are the same.
    ///
    /// * If `this` is `Type::Unspecified` then `self` is updated to the more specific `Type`.
    /// * If `this` is `Type::Subject` and is checked against a `Type::String` then `self` is updated to `Type::String`
    fn type_check(&mut self, attrs: Attrs, this: Type, other: Type) -> Result<Type, AnalysisError> {
        match (this, other) {
            (Type::Unspecified, other) => Ok(other),
            (this, Type::Unspecified) => Ok(this),
            (Type::Subject, Type::Subject) => Ok(Type::Subject),

            // Subjects are strings so there is no reason to reject a type
            // when compared to a string. However, when it happens, we demote
            // a subject to a string.
            (Type::Subject, Type::String) => Ok(Type::String),
            (Type::String, Type::Subject) => Ok(Type::String),

            (Type::Number, Type::Number) => Ok(Type::Number),
            (Type::String, Type::String) => Ok(Type::String),
            (Type::Bool, Type::Bool) => Ok(Type::Bool),
            (Type::Date, Type::Date) => Ok(Type::Date),
            (Type::Time, Type::Time) => Ok(Type::Time),
            (Type::DateTime, Type::DateTime) => Ok(Type::DateTime),

            // `DateTime` can be implicitly cast to `Date` or `Time`
            (Type::DateTime, Type::Date) => Ok(Type::Date),
            (Type::Date, Type::DateTime) => Ok(Type::Date),
            (Type::DateTime, Type::Time) => Ok(Type::Time),
            (Type::Time, Type::DateTime) => Ok(Type::Time),
            (Type::Custom(a), Type::Custom(b)) if self.strings.eq_ignore_ascii_case(a, b) => {
                Ok(Type::Custom(a))
            }
            (Type::Array(a), Type::Array(b)) => {
                let a = self.types.get_type(a);
                let b = self.types.get_type(b);
                let tpe = self.type_check(attrs, a, b)?;

                Ok(self.types.alloc_array_of(tpe))
            }

            (Type::Record(a), Type::Record(b)) if self.types.records_have_same_keys(a, b) => {
                let mut map_a = mem::take(&mut self.types.records[a.0]);
                let mut map_b = mem::take(&mut self.types.records[b.0]);

                for (bk, bv) in map_b.iter_mut() {
                    let av = map_a.get_mut(bk).unwrap();
                    let new_tpe = self.type_check(attrs, *av, *bv)?;

                    *av = new_tpe;
                    *bv = new_tpe;
                }

                self.types.records[a.0] = map_a;
                self.types.records[b.0] = map_b;

                Ok(Type::Record(a))
            }

            (
                Type::App {
                    args: a_args,
                    result: a_res,
                    aggregate: a_agg,
                },
                Type::App {
                    args: b_args,
                    result: b_res,
                    aggregate: b_agg,
                },
            ) if self.types.get_args(a_args.values).len()
                == self.types.get_args(b_args.values).len()
                && a_agg == b_agg =>
            {
                if self.types.get_args(a_args.values).is_empty() {
                    let a = self.types.get_type(a_res);
                    let b = self.types.get_type(b_res);
                    let new_res = self.type_check(attrs, a, b)?;

                    return Ok(Type::App {
                        args: a_args,
                        result: self.types.register_type(new_res),
                        aggregate: a_agg,
                    });
                }

                let mut vec_a = mem::take(&mut self.types.args[a_args.values.0]);
                let mut vec_b = mem::take(&mut self.types.args[b_args.values.0]);

                for (a, b) in vec_a.iter_mut().zip(vec_b.iter_mut()) {
                    let new_tpe = self.type_check(attrs, *a, *b)?;
                    *a = new_tpe;
                    *b = new_tpe;
                }

                self.types.args[a_args.values.0] = vec_a;
                self.types.args[b_args.values.0] = vec_b;

                let res_a = self.types.get_type(a_res);
                let res_b = self.types.get_type(b_res);
                let new_tpe = self.type_check(attrs, res_a, res_b)?;

                Ok(Type::App {
                    args: a_args,
                    result: self.types.register_type(new_tpe),
                    aggregate: a_agg,
                })
            }

            (this, other) => Err(AnalysisError::TypeMismatch(
                attrs.pos.line,
                attrs.pos.col,
                display_type(self, this),
                display_type(self, other),
            )),
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
/// note: Registered custom types are also recognized (case-insensitive).
pub(crate) fn resolve_type_from_str(
    arena: &Arena,
    opts: &AnalysisOptions,
    name: &str,
) -> Option<Type> {
    if name.eq_ignore_ascii_case("string") {
        Some(Type::String)
    } else if name.eq_ignore_ascii_case("int")
        || name.eq_ignore_ascii_case("float64")
        || name.eq_ignore_ascii_case("number")
    {
        Some(Type::Number)
    } else if name.eq_ignore_ascii_case("boolean") || name.eq_ignore_ascii_case("bool") {
        Some(Type::Bool)
    } else if name.eq_ignore_ascii_case("date") {
        Some(Type::Date)
    } else if name.eq_ignore_ascii_case("time") {
        Some(Type::Time)
    } else if name.eq_ignore_ascii_case("datetime") {
        Some(Type::DateTime)
    } else if let Some(str_ref) = arena.strings.str_ref_no_case(name)
        && opts.custom_types.contains(&str_ref)
    {
        Some(Type::Custom(str_ref))
    } else {
        None
    }
}

pub(crate) fn resolve_type(
    arena: &Arena,
    opts: &AnalysisOptions,
    name_ref: StrRef,
) -> Option<Type> {
    let name = arena.strings.get(name_ref);
    resolve_type_from_str(arena, opts, name)
}

pub(crate) fn display_type(arena: &Arena, tpe: Type) -> String {
    fn go(buffer: &mut String, arena: &Arena, tpe: Type) {
        match tpe {
            Type::Unspecified => buffer.push_str("Any"),
            Type::Number => buffer.push_str("Number"),
            Type::String => buffer.push_str("String"),
            Type::Bool => buffer.push_str("Bool"),
            Type::Subject => buffer.push_str("Subject"),
            Type::Date => buffer.push_str("Date"),
            Type::Time => buffer.push_str("Time"),
            Type::DateTime => buffer.push_str("DateTime"),
            Type::Custom(n) => buffer.push_str(arena.strings.get(n)),

            Type::Array(tpe) => {
                buffer.push_str("[]");
                go(buffer, arena, arena.types.get_type(tpe));
            }

            Type::Record(map) => {
                let map = arena.types.get_record(map);

                buffer.push_str("{ ");

                for (idx, (name, value)) in map.iter().enumerate() {
                    if idx != 0 {
                        buffer.push_str(", ");
                    }

                    buffer.push_str(arena.strings.get(*name));
                    buffer.push_str(": ");

                    go(buffer, arena, *value);
                }

                buffer.push_str(" }");
            }

            Type::App {
                args,
                result,
                aggregate,
            } => {
                let fun_args = arena.types.get_args(args.values);
                buffer.push('(');

                for (idx, arg) in fun_args.iter().copied().enumerate() {
                    if idx != 0 {
                        buffer.push_str(", ");
                    }

                    go(buffer, arena, arg);

                    if idx + 1 > args.needed {
                        buffer.push('?');
                    }
                }

                buffer.push(')');

                if aggregate {
                    buffer.push_str(" => ");
                } else {
                    buffer.push_str(" -> ");
                }

                go(buffer, arena, arena.types.get_type(result));
            }
        }
    }

    let mut buffer = String::new();
    go(&mut buffer, arena, tpe);

    buffer
}
