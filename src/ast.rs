//! Abstract syntax tree (AST) types for EventQL.
//!
//! This module defines the structure of parsed EventQL queries as an abstract
//! syntax tree. The AST represents the semantic structure of a query, making it
//! easy to analyze, transform, or execute queries.
//!
//! # Core Types
//!
//! - [`Query`] - The root of the AST, representing a complete query
//! - [`Expr`] - Expressions with position and type information
//! - [`Value`] - The various kinds of expression values (literals, operators, etc.)
//! - [`Source`] - Data sources in FROM clauses
//!
use crate::token::{Operator, Token};
use ordered_float::OrderedFloat;
use serde::Serialize;
use std::hash::{Hash, Hasher};

/// Position information for source code locations.
///
/// This struct tracks the line and column number of tokens and AST nodes,
/// which is useful for error reporting and debugging.
///
/// # Examples
///
/// ```
/// use eventql_parser::Pos;
///
/// let pos = Pos { line: 1, col: 10 };
/// assert_eq!(pos.line, 1);
/// assert_eq!(pos.col, 10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Pos {
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
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

/// Attributes attached to each expression node.
///
/// These attributes provide metadata about an expression, including its
/// position in the source code, scope information, and type information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Attrs {
    /// Source position of this expression
    pub pos: Pos,
}

impl Attrs {
    /// Create new attributes with unspecified type.
    pub fn new(pos: Pos) -> Self {
        Self { pos }
    }
}

impl<'a> From<Token<'a>> for Attrs {
    fn from(value: Token<'a>) -> Self {
        Self { pos: value.into() }
    }
}

/// A reference to a string stored in the [`StringArena`](crate::arena::StringArena).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct StrRef(pub(crate) usize);

/// A reference to a vector of expressions stored in the [`ExprArena`](crate::arena::ExprArena).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct VecRef(pub(crate) usize);

/// A reference to a vector of record fields stored in the [`ExprArena`](crate::arena::ExprArena).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct RecRef(pub(crate) usize);

/// Internal pointer to an expression in the arena.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ExprPtr(pub(crate) usize);

/// Internal hash key for an expression to provide structural equality.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct ExprKey(pub(crate) u64);

/// A reference to an expression stored in an [`ExprArena`](crate::arena::ExprArena).
///
/// This is a lightweight handle that combines a hash key for fast comparison
/// and a pointer for fast lookup.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ExprRef {
    pub(crate) key: ExprKey,
    pub(crate) ptr: ExprPtr,
}

impl Hash for ExprRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

/// Field access expression (e.g., `e.data.price`).
///
/// Represents accessing a field of a record or object using dot notation.
/// Can be chained for nested field access.
///
/// # Examples
///
/// In the query `WHERE e.data.user.id == 1`, the expression `e.data.user.id`
/// is parsed as nested `Access` nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Access {
    /// The target expression being accessed
    pub target: ExprRef,
    /// The name of the field being accessed
    pub field: StrRef,
}

/// Function application (e.g., `sum(e.price)`, `count()`).
///
/// Represents a function call with zero or more arguments.
///
/// # Examples
///
/// In the query `WHERE count(e.items) > 5`, the `count(e.items)` is an `App` node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct App {
    /// Name of the function being called
    pub func: StrRef,
    /// Arguments passed to the function
    pub args: VecRef,
}

/// A field in a record literal (e.g., `{name: "Alice", age: 30}`).
///
/// Represents a key-value pair in a record construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Field {
    /// Field attributes
    pub attrs: Attrs,
    /// Field name
    pub name: StrRef,
    /// Field value expression
    pub expr: ExprRef,
}

/// Binary operation (e.g., `a + b`, `x == y`, `p AND q`).
///
/// Represents operations that take two operands, including arithmetic,
/// comparison, and logical operators.
///
/// # Examples
///
/// In `WHERE e.price > 100 AND e.active == true`, there are multiple
/// binary operations: `>`, `==`, and `AND`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Binary {
    /// Left-hand side operand
    pub lhs: ExprRef,
    /// The operator
    pub operator: Operator,
    /// Right-hand side operand
    pub rhs: ExprRef,
}

/// Unary operation (e.g., `-x`, `NOT active`).
///
/// Represents operations that take a single operand.
///
/// # Examples
///
/// In `WHERE NOT e.deleted`, the `NOT e.deleted` is a unary operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Unary {
    /// The operator (Add for +, Sub for -, Not for NOT)
    pub operator: Operator,
    /// The operand expression
    pub expr: ExprRef,
}

/// The kind of value an expression represents.
///
/// This enum contains all the different types of expressions that can appear
/// in an EventQL query, from simple literals to complex operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Value {
    /// Numeric literal (e.g., `42`, `3.14`)
    Number(OrderedFloat<f64>),
    /// String literal (e.g., `"hello"`)
    String(StrRef),
    /// Boolean literal (`true` or `false`)
    Bool(bool),
    /// Identifier (e.g., variable name `e`, `x`)
    Id(StrRef),
    /// Array literal (e.g., `[1, 2, 3]`)
    Array(VecRef),
    /// Record literal (e.g., `{name: "Alice", age: 30}`)
    Record(RecRef),
    /// Field access (e.g., `e.data.price`)
    Access(Access),
    /// Function application (e.g., `sum(e.price)`)
    App(App),
    /// Binary operation (e.g., `a + b`, `x == y`)
    Binary(Binary),
    /// Unary operation (e.g., `-x`, `NOT active`)
    Unary(Unary),
    /// Grouped/parenthesized expression (e.g., `(a + b)`)
    Group(ExprRef),
}

/// A source binding. A name attached to a source of events.
///
/// # Examples
/// in `FROM e IN events`, `e` is the binding.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Binding {
    /// Name attached to a source of events
    pub name: StrRef,
    /// Position in the source code where that binding was introduced
    pub pos: Pos,
}

/// A data source in a FROM clause.
///
/// Sources specify where data comes from in a query. Each source has a binding
/// (the variable name) and a kind (what it binds to).
///
/// # Examples
///
/// In `FROM e IN events`, the source has:
/// - `binding`: `"e"`
/// - `kind`: `SourceKind::Name("events")`
#[derive(Debug, Clone, Serialize)]
pub struct Source<A> {
    /// Variable name bound to this source
    pub binding: Binding,
    /// What this source represents
    pub kind: SourceKind<A>,
}

/// The kind of data source.
///
/// EventQL supports three types of sources:
/// - Named sources (e.g., `FROM e IN events`)
/// - Subject patterns (e.g., `FROM e IN "users/john"`)
/// - Subqueries (e.g., `FROM e IN (SELECT ...)`)
#[derive(Debug, Clone, Serialize)]
pub enum SourceKind<A> {
    /// Named source (identifier)
    Name(StrRef),
    /// Subject pattern (string literal used as event subject pattern)
    Subject(StrRef),
    /// Nested subquery
    Subquery(Box<Query<A>>),
}

/// ORDER BY clause specification.
///
/// Defines how query results should be sorted.
///
/// # Examples
///
/// In `ORDER BY e.timestamp DESC`, this would be represented as:
/// - `expr`: expression for `e.timestamp`
/// - `order`: `Order::Desc`
#[derive(Debug, Clone, Copy, Serialize)]
pub struct OrderBy {
    /// Expression to sort by
    pub expr: ExprRef,
    /// Sort direction (ascending or descending)
    pub order: Order,
}

/// Sort order direction.
///
/// Specifies whether sorting is ascending or descending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Order {
    /// Ascending order (smallest to largest)
    Asc,
    /// Descending order (largest to smallest)
    Desc,
}

/// GROUP BY clause specification
///
/// Defines how query results should be order by.
/// # Examples
///
/// In `GROUP BY e.age HAVING age > 123`, this would be represented as:
/// - `expr`: expression for `e.age`
/// - `predicate`: `age > 123`
#[derive(Debug, Clone, Copy, Serialize)]
pub struct GroupBy {
    /// Expression to group by
    pub expr: ExprRef,

    /// Predicate to filter groups after aggregation
    pub predicate: Option<ExprRef>,
}

/// Result set limit specification.
///
/// EventQL supports two types of limits:
/// - `TOP n` - Take the first n results
/// - `SKIP n` - Skip the first n results
///
/// # Examples
///
/// - `TOP 10` limits to first 10 results
/// - `SKIP 20` skips first 20 results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Limit {
    /// Skip the first n results
    Skip(u64),
    /// Take only the first n results
    Top(u64),
}

/// Represents the state of a query that only has a valid syntax. There are no guarantee that all
/// the variables exists or that the query is sound. For example, if the user is asking for an event
/// that has field that should be a string or a number at the same time.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Raw;

/// A complete EventQL query.
///
/// This is the root node of the AST, representing a full query with all its clauses.
/// A query must have at least one source and a projection; other clauses are optional.
///
/// # Structure
///
/// ```text
/// FROM <alias> <source>
/// [FROM <alias> <source>] ...
/// [WHERE <condition>]
/// [GROUP BY <field> [HAVING <condition>]]
/// [ORDER BY <field> ASC|DESC]
/// [TOP|SKIP <n>]
/// PROJECT INTO [DISTINCT] <projection>
/// ```
///
/// # Examples
///
/// ```
/// use eventql_parser::Session;
///
/// let mut session = Session::builder().use_stdlib().build();
/// let query = session.parse(
///     "FROM e IN events \
///      WHERE e.price > 100 \
///      ORDER BY e.timestamp DESC \
///      TOP 10 \
///      PROJECT INTO {id: e.id, price: e.price}"
/// ).unwrap();
///
/// assert_eq!(query.sources.len(), 1);
/// assert!(query.predicate.is_some());
/// assert!(query.order_by.is_some());
/// assert!(query.limit.is_some());
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct Query<A> {
    /// Metadata about this query
    pub attrs: Attrs,
    /// FROM clause sources (must have at least one)
    pub sources: Vec<Source<A>>,
    /// Optional WHERE clause filter predicate
    pub predicate: Option<ExprRef>,
    /// Optional GROUP BY clause expression
    pub group_by: Option<GroupBy>,
    /// Optional ORDER BY clause
    pub order_by: Option<OrderBy>,
    /// Optional LIMIT clause (TOP or SKIP)
    pub limit: Option<Limit>,
    /// PROJECT INTO clause expression (required)
    pub projection: ExprRef,
    /// Remove duplicate rows from the query's results
    pub distinct: bool,
    /// Type-level metadata about the query's analysis state.
    ///
    /// This field uses a generic type parameter to track whether the query
    /// is in a raw (unparsed/untyped) state or has been statically analyzed:
    /// - `Query<Raw>`: Query parsed but not yet type-checked
    /// - `Query<Typed>`: Query that has passed static analysis with validated
    ///   types and variable scopes
    ///
    /// This provides compile-time guarantees about the query's type safety.
    pub meta: A,
}
