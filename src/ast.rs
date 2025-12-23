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
use std::{collections::BTreeMap, marker::PhantomData};

use crate::{
    analysis::{AnalysisOptions, static_analysis},
    error::Error,
    token::{Operator, Token},
};
use serde::Serialize;

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

/// Type information for expressions.
///
/// This enum represents the type of an expression in the EventQL type system.
/// Types can be inferred during semantic analysis or left as `Unspecified`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub enum Type {
    /// Type has not been determined yet
    Unspecified,
    /// Numeric type (f64)
    Number,
    /// String type
    String,
    /// Boolean type
    Bool,
    /// Array type
    Array(Vec<Type>),
    /// Record (object) type
    Record(BTreeMap<String, Type>),
    /// Subject pattern type
    Subject,
}

/// Attributes attached to each expression node.
///
/// These attributes provide metadata about an expression, including its
/// position in the source code, scope information, and type information.
#[derive(Debug, Clone, Serialize)]
pub struct Attrs {
    /// Source position of this expression
    pub pos: Pos,
    /// Scope level (0 for top-level, incremented for subqueries)
    pub scope: u64,
    /// Type of this expression
    pub tpe: Type,
}

impl Attrs {
    /// Create new attributes with unspecified type.
    pub fn new(pos: Pos, scope: u64) -> Self {
        Self {
            pos,
            scope,
            tpe: Type::Unspecified,
        }
    }
}

/// An expression with metadata.
///
/// This is the fundamental building block of the AST. Every expression
/// carries attributes (position, scope, type) and a value that determines
/// what kind of expression it is.
#[derive(Debug, Clone, Serialize)]
pub struct Expr {
    /// Metadata about this expression
    pub attrs: Attrs,
    /// The value/kind of this expression
    pub value: Value,
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
#[derive(Debug, Clone, Serialize)]
pub struct Access {
    /// The target expression being accessed
    pub target: Box<Expr>,
    /// The name of the field being accessed
    pub field: String,
}

/// Function application (e.g., `sum(e.price)`, `count()`).
///
/// Represents a function call with zero or more arguments.
///
/// # Examples
///
/// In the query `WHERE count(e.items) > 5`, the `count(e.items)` is an `App` node.
#[derive(Debug, Clone, Serialize)]
pub struct App {
    /// Name of the function being called
    pub func: String,
    /// Arguments passed to the function
    pub args: Vec<Expr>,
}

/// A field in a record literal (e.g., `{name: "Alice", age: 30}`).
///
/// Represents a key-value pair in a record construction.
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    /// Field name
    pub name: String,
    /// Field value expression
    pub value: Expr,
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
#[derive(Debug, Clone, Serialize)]
pub struct Binary {
    /// Left-hand side operand
    pub lhs: Box<Expr>,
    /// The operator
    pub operator: Operator,
    /// Right-hand side operand
    pub rhs: Box<Expr>,
}

/// Unary operation (e.g., `-x`, `NOT active`).
///
/// Represents operations that take a single operand.
///
/// # Examples
///
/// In `WHERE NOT e.deleted`, the `NOT e.deleted` is a unary operation.
#[derive(Debug, Clone, Serialize)]
pub struct Unary {
    /// The operator (Add for +, Sub for -, Not for NOT)
    pub operator: Operator,
    /// The operand expression
    pub expr: Box<Expr>,
}

/// The kind of value an expression represents.
///
/// This enum contains all the different types of expressions that can appear
/// in an EventQL query, from simple literals to complex operations.
#[derive(Debug, Clone, Serialize)]
pub enum Value {
    /// Numeric literal (e.g., `42`, `3.14`)
    Number(f64),
    /// String literal (e.g., `"hello"`)
    String(String),
    /// Boolean literal (`true` or `false`)
    Bool(bool),
    /// Identifier (e.g., variable name `e`, `x`)
    Id(String),
    /// Array literal (e.g., `[1, 2, 3]`)
    Array(Vec<Expr>),
    /// Record literal (e.g., `{name: "Alice", age: 30}`)
    Record(Vec<Field>),
    /// Field access (e.g., `e.data.price`)
    Access(Access),
    /// Function application (e.g., `sum(e.price)`)
    App(App),
    /// Binary operation (e.g., `a + b`, `x == y`)
    Binary(Binary),
    /// Unary operation (e.g., `-x`, `NOT active`)
    Unary(Unary),
    /// Grouped/parenthesized expression (e.g., `(a + b)`)
    Group(Box<Expr>),
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
    pub binding: String,
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
    Name(String),
    /// Subject pattern (string literal used as event subject pattern)
    Subject(String),
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
#[derive(Debug, Clone, Serialize)]
pub struct OrderBy {
    /// Expression to sort by
    pub expr: Expr,
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
#[derive(Debug, Clone, Serialize)]
pub struct GroupBy {
    /// Expression to group by
    pub expr: Expr,

    /// Predicate to filter groups after aggregation
    pub predicate: Option<Expr>,
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

/// Represents the state of a query that has been statically analysed. It includes all variables are defined
/// and the types are sound.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Typed;

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
/// use eventql_parser::parse_query;
///
/// let query = parse_query(
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
    pub predicate: Option<Expr>,
    /// Optional GROUP BY clause expression
    pub group_by: Option<GroupBy>,
    /// Optional ORDER BY clause
    pub order_by: Option<OrderBy>,
    /// Optional LIMIT clause (TOP or SKIP)
    pub limit: Option<Limit>,
    /// PROJECT INTO clause expression (required)
    pub projection: Expr,
    /// Remove duplicate rows from the query's results
    pub distinct: bool,

    #[serde(skip)]
    pub(crate) _marker: PhantomData<A>,
}

impl Query<Raw> {
    pub fn run_static_analysis(self, options: &AnalysisOptions) -> crate::Result<Query<Typed>> {
        static_analysis(options, self).map_err(Error::Analysis)
    }
}
