use serde::Serialize;

pub mod analysis;

/// A reference to a type stored in the [`TypeArena`](crate::arena::TypeArena).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct TypeRef(pub(crate) usize);

/// A reference to a record definition stored in the [`TypeArena`](crate::arena::TypeArena).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct Record {
    pub(crate) id: usize,
    pub(crate) open: bool,
}

/// A reference to a function argument type list stored in the [`TypeArena`](crate::arena::TypeArena).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct ArgsRef(pub(crate) usize);
/// Represents function argument types with optional parameter support.
///
/// This type allows defining functions that have both required and optional parameters.
/// The `needed` field specifies how many arguments are required, while `values` contains
/// all possible argument types (both required and optional).
///
#[derive(Debug, Serialize, PartialEq, Eq, Hash, Clone, Copy)]
pub struct FunArgs {
    /// All argument types, including both required and optional parameters
    pub values: ArgsRef,
    /// Number of required arguments (must be <= values.len())
    pub needed: usize,
}

/// Type information for expressions.
///
/// This enum represents the type of expressions in the EventQL type system.
/// Types can be inferred during semantic analysis or left as `Unspecified`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
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
    Array(TypeRef),
    /// Record (object) type
    Record(Record),
    /// Subject pattern type
    Subject,
    /// Function type with support for optional parameters.
    ///
    /// The `args` field uses [`FunArgs`] to support both required and optional parameters.
    /// Optional parameters are indicated when `args.needed < args.values.len()`.
    App {
        /// Function argument types, supporting optional parameters
        args: FunArgs,
        /// Return type of the function
        result: TypeRef,
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
}

impl Type {
    /// Returns the inner [`Record`] reference, panicking if this is not a `Type::Record`.
    pub fn as_record_or_panic(&self) -> Record {
        if let Self::Record(r) = self {
            return *r;
        }

        panic!("expected record type, got {:?}", self);
    }
}
