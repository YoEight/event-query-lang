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
use crate::arena::ExprArena;
use crate::{
    analysis::{AnalysisOptions, Typed, static_analysis},
    error::{AnalysisError, Error},
    token::{Operator, Token},
};
use ordered_float::OrderedFloat;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::{
    fmt::{self, Display},
    mem,
};

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

/// Represents function argument types with optional parameter support.
///
/// This type allows defining functions that have both required and optional parameters.
/// The `needed` field specifies how many arguments are required, while `values` contains
/// all possible argument types (both required and optional).
///
/// # Examples
///
/// ```
/// use eventql_parser::prelude::{FunArgs, Type};
///
/// // Function with all required parameters: (number, string)
/// let required = FunArgs::required(vec![Type::Number, Type::String]);
/// assert_eq!(required.needed, 2);
/// assert_eq!(required.values.len(), 2);
///
/// // Function with optional parameters: (boolean, number?)
/// let optional = FunArgs {
///     values: vec![Type::Bool, Type::Number],
///     needed: 1, // Only first parameter is required
/// };
/// assert!(optional.match_arg_count(1)); // Can call with just boolean
/// assert!(optional.match_arg_count(2)); // Can call with both
/// assert!(!optional.match_arg_count(3)); // Cannot call with 3 args
/// ```
#[derive(Debug, Serialize, Clone)]
pub struct FunArgs {
    /// All argument types, including both required and optional parameters
    pub values: Vec<Type>,
    /// Number of required arguments (must be <= values.len())
    pub needed: usize,
}

impl FunArgs {
    /// Creates a new `FunArgs` where all parameters are required.
    ///
    /// # Examples
    ///
    /// ```
    /// use eventql_parser::prelude::{FunArgs, Type};
    ///
    /// let args = FunArgs::required(vec![Type::Number, Type::String]);
    /// assert_eq!(args.needed, 2);
    /// assert_eq!(args.values.len(), 2);
    /// ```
    pub fn required(args: Vec<Type>) -> Self {
        Self {
            needed: args.len(),
            values: args,
        }
    }

    /// Returns `true` if there are no argument types defined.
    ///
    /// # Examples
    ///
    /// ```
    /// use eventql_parser::prelude::{FunArgs, Type};
    ///
    /// let empty = FunArgs::required(vec![]);
    /// assert!(empty.is_empty());
    ///
    /// let not_empty = FunArgs::required(vec![Type::Number]);
    /// assert!(!not_empty.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Checks if a given argument count is valid for this function signature.
    ///
    /// Returns `true` if the count is between `needed` (inclusive) and
    /// `values.len()` (inclusive), meaning all required arguments are
    /// provided and no extra arguments beyond the optional ones are given.
    ///
    /// # Examples
    ///
    /// ```
    /// use eventql_parser::prelude::{FunArgs, Type};
    ///
    /// let args = FunArgs {
    ///     values: vec![Type::Bool, Type::Number, Type::String],
    ///     needed: 1, // Only first parameter is required
    /// };
    ///
    /// assert!(!args.match_arg_count(0)); // Missing required argument
    /// assert!(args.match_arg_count(1));  // Required argument provided
    /// assert!(args.match_arg_count(2));  // Required + one optional
    /// assert!(args.match_arg_count(3));  // All arguments provided
    /// assert!(!args.match_arg_count(4)); // Too many arguments
    /// ```
    pub fn match_arg_count(&self, cnt: usize) -> bool {
        cnt >= self.needed && cnt <= self.values.len()
    }
}

impl From<Vec<Type>> for FunArgs {
    fn from(value: Vec<Type>) -> Self {
        Self::required(value)
    }
}

/// Type information for expressions.
///
/// This enum represents the type of an expression in the E
#[derive(Clone, Debug, Default, Serialize)]
pub enum Type {
    /// Type has not been determined yet
    #[default]
    Unspecified,
    /// Numeric type (f64)
    Number,
    /// String type
    String,
    /// Boolean type
    Bool,
    /// Array type
    Array(Box<Type>),
    /// Record (object) type
    Record(FxHashMap<String, Type>),
    /// Subject pattern type
    Subject,
    /// Function type with support for optional parameters.
    ///
    /// The `args` field uses [`FunArgs`] to support both required and optional parameters.
    /// Optional parameters are indicated when `args.needed < args.values.len()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use eventql_parser::prelude::{Type, FunArgs};
    ///
    /// // Function with all required parameters: (number, string) -> boolean
    /// let all_required = Type::App {
    ///     args: vec![Type::Number, Type::String].into(),
    ///     result: Box::new(Type::Bool),
    ///     aggregate: false,
    /// };
    ///
    /// // Aggregate function with optional parameter: (boolean?) => number
    /// let with_optional = Type::App {
    ///     args: FunArgs {
    ///         values: vec![Type::Bool],
    ///         needed: 0, // All parameters are optional
    ///     },
    ///     result: Box::new(Type::Number),
    ///     aggregate: true,
    /// };
    /// ```
    App {
        /// Function argument types, supporting optional parameters
        args: FunArgs,
        /// Return type of the function
        result: Box<Type>,
        /// Whether this is an aggregate function (operates on grouped data)
        aggregate: bool,
    },
    /// Date type (e.g., `2026-01-03`)
    ///
    /// Used when a field is explicitly converted to a date using the `AS DATE` syntax.
    Date,
    /// Time type (e.g., `13:45:39`)
    ///
    /// Used when a field is explicitly converted to a time using the `AS TIME` syntax.
    Time,
    /// DateTime type (e.g., `2026-01-01T13:45:39Z`)
    ///
    /// Used when a field is explicitly converted to a datetime using the `AS DATETIME` syntax.
    DateTime,
    /// Custom type not defined in the EventQL reference
    ///
    /// Used when a field is converted to a custom type registered in [`AnalysisOptions::custom_types`].
    /// The string contains the custom type name as it appears in the query.
    ///
    /// # Examples
    ///
    /// ```
    /// use eventql_parser::{parse_query, prelude::AnalysisOptions};
    ///
    /// let query = parse_query("FROM e IN events PROJECT INTO { ts: e.data.timestamp as CustomTimestamp }").unwrap();
    /// let options = AnalysisOptions::default().add_custom_type("CustomTimestamp");
    /// let typed_query = query.run_static_analysis(&options).unwrap();
    /// ```
    Custom(String),
}

/// Provides human-readable string formatting for types.
///
/// Function types display optional parameters with a `?` suffix. For example,
/// a function with signature `(boolean, number?) -> string` accepts 1 or 2 arguments.
/// Aggregate functions use `=>` instead of `->` in their signature.
///
/// # Examples
///
/// ```
/// use eventql_parser::prelude::{Type, FunArgs};
///
/// // Basic types
/// assert_eq!(Type::Number.to_string(), "number");
/// assert_eq!(Type::String.to_string(), "string");
/// assert_eq!(Type::Bool.to_string(), "boolean");
///
/// // Array type
/// let arr = Type::Array(Box::new(Type::Number));
/// assert_eq!(arr.to_string(), "[]number");
///
/// // Function with all required parameters
/// let func = Type::App {
///     args: vec![Type::Number, Type::String].into(),
///     result: Box::new(Type::Bool),
///     aggregate: false,
/// };
/// assert_eq!(func.to_string(), "(number, string) -> boolean");
///
/// // Function with optional parameters
/// let func_optional = Type::App {
///     args: FunArgs {
///         values: vec![Type::Bool, Type::Number],
///         needed: 1,
///     },
///     result: Box::new(Type::String),
///     aggregate: false,
/// };
/// assert_eq!(func_optional.to_string(), "(boolean, number?) -> string");
///
/// // Aggregate function
/// let agg = Type::App {
///     args: vec![Type::Number].into(),
///     result: Box::new(Type::Number),
///     aggregate: true,
/// };
/// assert_eq!(agg.to_string(), "(number) => number");
/// ```
impl Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Unspecified => write!(f, "any"),
            Type::Number => write!(f, "number"),
            Type::String => write!(f, "string"),
            Type::Bool => write!(f, "boolean"),
            Type::Array(tpe) => write!(f, "[]{tpe}"),
            Type::Record(map) => {
                write!(f, "{{ ")?;

                for (idx, (name, value)) in map.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{name}: {value}")?;
                }

                write!(f, " }}")
            }
            Type::Subject => write!(f, "subject"),
            Type::App {
                args,
                result,
                aggregate,
            } => {
                write!(f, "(")?;

                for (idx, arg) in args.values.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{arg}")?;

                    if idx + 1 > args.needed {
                        write!(f, "?")?;
                    }
                }

                write!(f, ")")?;

                if *aggregate {
                    write!(f, " => ")?;
                } else {
                    write!(f, " -> ")?;
                }

                write!(f, "{result}")
            }
            Type::Date => write!(f, "date"),
            Type::Time => write!(f, "time"),
            Type::DateTime => write!(f, "datetime"),
            Type::Custom(n) => write!(f, "{}", n.to_lowercase()),
        }
    }
}

impl Type {
    pub fn as_record_or_panic_mut(&mut self) -> &mut FxHashMap<String, Type> {
        if let Self::Record(r) = self {
            return r;
        }

        panic!("expected record type, got {:?}", self);
    }

    /// Checks if two types are the same.
    ///
    /// * If `self` is `Type::Unspecified` then `self` is updated to the more specific `Type`.
    /// * If `self` is `Type::Subject` and is checked against a `Type::String` then `self` is updated to `Type::String`
    pub fn check(self, attrs: Attrs, other: Type) -> Result<Type, AnalysisError> {
        match (self, other) {
            (Self::Unspecified, other) => Ok(other),
            (this, Self::Unspecified) => Ok(this),
            (Self::Subject, Self::Subject) => Ok(Self::Subject),

            // Subjects are strings so there is no reason to reject a type
            // when compared to a string. However, when it happens, we demote
            // a subject to a string.
            (Self::Subject, Self::String) => Ok(Self::String),
            (Self::String, Self::Subject) => Ok(Self::String),

            (Self::Number, Self::Number) => Ok(Self::Number),
            (Self::String, Self::String) => Ok(Self::String),
            (Self::Bool, Self::Bool) => Ok(Self::Bool),
            (Self::Date, Self::Date) => Ok(Self::Date),
            (Self::Time, Self::Time) => Ok(Self::Time),
            (Self::DateTime, Self::DateTime) => Ok(Self::DateTime),

            // `DateTime` can be implicitly cast to `Date` or `Time`
            (Self::DateTime, Self::Date) => Ok(Self::Date),
            (Self::Date, Self::DateTime) => Ok(Self::Date),
            (Self::DateTime, Self::Time) => Ok(Self::Time),
            (Self::Time, Self::DateTime) => Ok(Self::Time),
            (Self::Custom(a), Self::Custom(b)) if a.eq_ignore_ascii_case(b.as_str()) => {
                Ok(Self::Custom(a))
            }
            (Self::Array(mut a), Self::Array(b)) => {
                *a = a.as_ref().clone().check(attrs, *b)?;
                Ok(Self::Array(a))
            }

            (Self::Record(mut a), Self::Record(b)) if a.len() == b.len() => {
                if a.is_empty() {
                    return Ok(Self::Record(a));
                }

                for bk in b.keys() {
                    if !a.contains_key(bk) {
                        return Err(AnalysisError::TypeMismatch(
                            attrs.pos.line,
                            attrs.pos.col,
                            Self::Record(a),
                            Self::Record(b),
                        ));
                    }
                }

                for (bk, bv) in b.into_iter() {
                    let av = a.get_mut(&bk).unwrap();
                    let a = mem::take(av);
                    *av = a.check(attrs, bv)?;
                }

                Ok(Self::Record(a))
            }

            (
                Self::App {
                    args: mut a_args,
                    result: mut a_res,
                    aggregate: a_agg,
                },
                Self::App {
                    args: b_args,
                    result: b_res,
                    aggregate: b_agg,
                },
            ) if a_args.values.len() == b_args.values.len() && a_agg == b_agg => {
                if a_args.is_empty() {
                    let tmp = mem::take(a_res.as_mut());
                    *a_res = tmp.check(attrs, *b_res)?;
                    return Ok(Self::App {
                        args: a_args,
                        result: a_res,
                        aggregate: a_agg,
                    });
                }

                for (a, b) in a_args.values.iter_mut().zip(b_args.values.into_iter()) {
                    let tmp = mem::take(a);
                    *a = tmp.check(attrs, b)?;
                }

                let tmp = mem::take(a_res.as_mut());
                *a_res = tmp.check(attrs, *b_res)?;

                Ok(Self::App {
                    args: a_args,
                    result: a_res,
                    aggregate: a_agg,
                })
            }

            (this, other) => Err(AnalysisError::TypeMismatch(
                attrs.pos.line,
                attrs.pos.col,
                this,
                other,
            )),
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

/// Internal pointer to an expression in the arena.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ExprPtr(pub(crate) usize);

/// Internal hash key for an expression to provide structural equality.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct ExprKey(pub(crate) u64);

/// A reference to an expression stored in an [`ExprArena`].
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

/// An expression with metadata.
///
/// This is the fundamental building block of the AST. Every expression
/// carries attributes (position, scope, type) and a value that determines
/// what kind of expression it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Expr {
    /// Metadata about this expression
    pub attrs: Attrs,
    /// The value/kind of this expression
    pub node_ref: ExprRef,
}

impl Hash for Expr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node_ref.hash(state);
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Access {
    /// The target expression being accessed
    pub target: ExprRef,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct App {
    /// Name of the function being called
    pub func: String,
    /// Arguments passed to the function
    pub args: Vec<ExprRef>,
}

/// A field in a record literal (e.g., `{name: "Alice", age: 30}`).
///
/// Represents a key-value pair in a record construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Field {
    /// Field attributes
    pub attrs: Attrs,
    /// Field name
    pub name: String,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Value {
    /// Numeric literal (e.g., `42`, `3.14`)
    Number(OrderedFloat<f64>),
    /// String literal (e.g., `"hello"`)
    String(String),
    /// Boolean literal (`true` or `false`)
    Bool(bool),
    /// Identifier (e.g., variable name `e`, `x`)
    Id(String),
    /// Array literal (e.g., `[1, 2, 3]`)
    Array(Vec<ExprRef>),
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
    Group(ExprRef),
}

/// A source binding. A name attached to a source of events.
///
/// # Examples
/// in `FROM e IN events`, `e` is the binding.
#[derive(Debug, Clone, Serialize)]
pub struct Binding {
    /// Name attached to a source of events
    pub name: String,
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
#[derive(Debug, Clone, Serialize)]
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

impl Query<Raw> {
    /// Performs static analysis on this raw query.
    ///
    /// This is a convenience method that runs type checking and variable scoping
    /// analysis on the query, converting it from a raw (untyped) query to a
    /// typed query.
    ///
    /// The analysis validates:
    /// - Variable declarations and scoping
    /// - Type compatibility in expressions and operations
    /// - Valid field accesses on record types
    /// - Correct function argument types and counts
    /// - Aggregate function usage restrictions (only in PROJECT INTO)
    /// - No mixing of aggregate functions with source-bound fields
    /// - Aggregate function arguments are source-bound fields
    /// - Non-empty record literals in projections
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration containing type information and default scope
    ///
    /// # Returns
    ///
    /// Returns a typed query on success, or an error if type checking fails.
    pub fn run_static_analysis(
        self,
        arena: &ExprArena,
        options: &AnalysisOptions,
    ) -> crate::Result<Query<Typed>> {
        static_analysis(arena, options, self).map_err(Error::Analysis)
    }
}
