//! EventQL parser library for parsing event sourcing query language.
//!
//! This library provides a complete lexer and parser for EventQL (EQL), a query language
//! designed for event sourcing systems. It allows you to parse EQL query strings into
//! an abstract syntax tree (AST) that can be analyzed or executed.
pub mod arena;
mod ast;
mod error;
mod lexer;
mod parser;
#[cfg(test)]
mod tests;
mod token;
mod typing;

use crate::arena::Arena;
use crate::lexer::tokenize;
use crate::prelude::{
    Analysis, AnalysisOptions, FunArgs, Scope, Typed, display_type, parse, resolve_type_from_str,
};
use crate::token::Token;
pub use ast::*;
use rustc_hash::FxHashMap;
pub use typing::Type;

/// Convenience module that re-exports all public types and functions.
///
/// This module provides a single import point for all the library's public API,
/// including AST types, error types, lexer, parser, and token types.
pub mod prelude {
    pub use super::arena::*;
    pub use super::ast::*;
    pub use super::error::*;
    pub use super::parser::*;
    pub use super::token::*;
    pub use super::typing::analysis::*;
    pub use super::typing::*;
}

/// Builder for function argument specifications.
///
/// Allows defining function signatures with both required and optional parameters.
/// When `required` equals the length of `args`, all parameters are required.
pub struct FunArgsBuilder<'a> {
    args: &'a [Type],
    required: usize,
}

impl<'a> FunArgsBuilder<'a> {
    /// Creates a new `FunArgsBuilder` with the given argument types and required count.
    pub fn new(args: &'a [Type], required: usize) -> Self {
        Self { args, required }
    }
}

impl<'a> From<&'a [Type]> for FunArgsBuilder<'a> {
    fn from(args: &'a [Type]) -> Self {
        Self {
            args,
            required: args.len(),
        }
    }
}

impl<'a, const N: usize> From<&'a [Type; N]> for FunArgsBuilder<'a> {
    fn from(value: &'a [Type; N]) -> Self {
        Self {
            args: value.as_slice(),
            required: value.len(),
        }
    }
}

/// Builder for configuring type information on a [`SessionBuilder`].
///
/// Obtained by calling [`SessionBuilder::declare_type`]. Use [`define_record`](EventTypeBuilder::define_record)
/// to define a record-shaped type. Call [`done`](EventTypeBuilder::done) to return to the [`SessionBuilder`].
pub struct EventTypeBuilder<'a> {
    parent: &'a mut SessionBuilder,
}

impl<'a> EventTypeBuilder<'a> {
    /// Starts building a record-shaped event type with named fields.
    pub fn define_record(self) -> EventTypeRecordBuilder<'a> {
        EventTypeRecordBuilder {
            inner: self,
            props: Default::default(),
        }
    }

    /// Registers a type for a specific named data source.
    ///
    /// Queries targeting `data_source` will use `tpe` for type checking instead of the default event type.
    /// Data source names are case-insensitive.
    pub fn data_source(self, data_source: &str, tpe: Type) -> Self {
        let data_source = self.parent.arena.strings.alloc_no_case(data_source);

        self.parent.options.data_sources.insert(data_source, tpe);

        self
    }

    /// Finalizes type configuration and returns the [`SessionBuilder`].
    pub fn done(self) {}
}

/// Builder for defining the fields of a record-shaped event type.
///
/// Obtained by calling [`EventTypeBuilder::define_record`]. Add fields with [`prop`](EventTypeRecordBuilder::prop)
/// and finalize with [`as_default_event_type`](EventTypeRecordBuilder::as_default_event_type) or
/// [`for_data_source`](EventTypeRecordBuilder::for_data_source) to return to the [`EventTypeBuilder`].
pub struct EventTypeRecordBuilder<'a> {
    inner: EventTypeBuilder<'a>,
    props: FxHashMap<StrRef, Type>,
}

impl<'a> EventTypeRecordBuilder<'a> {
    /// Conditionally adds a field to the event record type.
    pub fn prop_when(mut self, test: bool, name: &str, tpe: Type) -> Self {
        if test {
            self.props
                .insert(self.inner.parent.arena.strings.alloc(name), tpe);
        }

        self
    }

    /// Adds a field with the given name and type to the event record type.
    pub fn prop(mut self, name: &str, tpe: Type) -> Self {
        self.props
            .insert(self.inner.parent.arena.strings.alloc(name), tpe);
        self
    }

    /// Finalizes the event record type and returns the [`SessionBuilder`].
    pub fn as_default_event_type(self) -> EventTypeBuilder<'a> {
        let ptr = self.inner.parent.arena.types.alloc_record(self.props);
        self.inner.parent.set_default_event_type(Type::Record(ptr));
        self.inner
    }

    /// Finalizes the record type and registers it for a specific named data source.
    ///
    /// Queries targeting `data_source` will use this record type for type checking.
    /// Data source names are case-insensitive. Returns the [`EventTypeBuilder`] to allow
    /// chaining further type declarations.
    pub fn for_data_source(self, data_source: &str) -> EventTypeBuilder<'a> {
        let data_source = self.inner.parent.arena.strings.alloc_no_case(data_source);
        let ptr = self.inner.parent.arena.types.alloc_record(self.props);

        self.inner
            .parent
            .options
            .data_sources
            .insert(data_source, Type::Record(ptr));

        self.inner
    }

    pub fn build(self) -> Type {
        let ptr = self.inner.parent.arena.types.alloc_record(self.props);
        Type::Record(ptr)
    }
}

/// A specialized `Result` type for EventQL parser operations.
pub type Result<A> = std::result::Result<A, error::Error>;

/// `SessionBuilder` is a builder for `Session` objects.
///
/// It allows for the configuration of analysis options, such as declaring
/// functions (both regular and aggregate), and event types before building an `EventQL` parsing session.
#[derive(Default)]
pub struct SessionBuilder {
    arena: Arena,
    options: AnalysisOptions,
}

impl SessionBuilder {
    /// Declares a new function with the given name, arguments, and return type.
    ///
    /// This function adds a new entry to the session's default scope, allowing
    /// the parser to recognize and type-check calls to this function.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the function.
    /// * `args` - The arguments the function accepts, which can be converted into `FunArgs`.
    /// * `result` - The return type of the function.
    pub fn declare_func<'a>(
        &mut self,
        name: &'a str,
        args: impl Into<FunArgsBuilder<'a>>,
        result: Type,
    ) {
        let builder = args.into();
        let name = self.arena.strings.alloc_no_case(name);
        let args = self.arena.types.alloc_args(builder.args);

        self.options.default_scope.declare(
            name,
            Type::App {
                args: FunArgs {
                    values: args,
                    needed: builder.required,
                },
                result: self.arena.types.register_type(result),
                aggregate: false,
            },
        );
    }

    /// Declares a new aggregate function with the given name, arguments, and return type.
    ///
    /// Similar to `declare_func`, but marks the function as an aggregate function.
    /// Aggregate functions have specific rules for where they can be used in an EQL query.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the aggregate function.
    /// * `args` - The arguments the aggregate function accepts.
    /// * `result` - The return type of the aggregate function.
    pub fn declare_agg_func<'a>(
        &mut self,
        name: &'a str,
        args: impl Into<FunArgsBuilder<'a>>,
        result: Type,
    ) {
        let builder = args.into();
        let name = self.arena.strings.alloc_no_case(name);
        let args = self.arena.types.alloc_args(builder.args);

        self.options.default_scope.declare(
            name,
            Type::App {
                args: FunArgs {
                    values: args,
                    needed: builder.required,
                },
                result: self.arena.types.register_type(result),
                aggregate: true,
            },
        );
    }

    /// Conditionally declares the expected type of event records.
    ///
    /// This type information is crucial for type-checking event properties
    /// accessed in EQL queries (e.g., `e.id`, `e.data.value`).
    /// The declaration only happens if `test` is `true`.
    ///
    /// # Arguments
    ///
    /// * `test` - A boolean indicating whether to declare the event type.
    /// * `tpe` - The `Type` representing the structure of event records.
    pub fn set_default_event_type(&mut self, tpe: Type) {
        self.options.default_event_type = tpe;
    }

    /// Declares the expected type of event records.
    ///
    /// This type information is crucial for type-checking event properties
    /// accessed in EQL queries (e.g., `e.id`, `e.data.value`).
    ///
    /// # Arguments
    ///
    /// * `tpe` - The `Type` representing the structure of event records.
    pub fn declare_type(&mut self) -> EventTypeBuilder<'_> {
        EventTypeBuilder { parent: self }
    }

    /// Includes the standard library of functions and event types in the session.
    ///
    /// This method pre-configures the `SessionBuilder` with a set of commonly
    /// used functions (e.g., mathematical, string, date/time) and a default
    /// event type definition. Calling this method is equivalent to calling
    /// `declare_func` and `declare_agg_func` for all standard library functions,
    /// and `declare_event_type` for the default event structure.
    pub fn use_stdlib(mut self) -> Self {
        self.declare_func("abs", &[Type::Number], Type::Number);
        self.declare_func("ceil", &[Type::Number], Type::Number);
        self.declare_func("floor", &[Type::Number], Type::Number);
        self.declare_func("round", &[Type::Number], Type::Number);
        self.declare_func("cos", &[Type::Number], Type::Number);
        self.declare_func("exp", &[Type::Number], Type::Number);
        self.declare_func("pow", &[Type::Number, Type::Number], Type::Number);
        self.declare_func("sqrt", &[Type::Number], Type::Number);
        self.declare_func("rand", &[], Type::Number);
        self.declare_func("pi", &[Type::Number], Type::Number);
        self.declare_func("lower", &[Type::String], Type::String);
        self.declare_func("upper", &[Type::String], Type::String);
        self.declare_func("trim", &[Type::String], Type::String);
        self.declare_func("ltrim", &[Type::String], Type::String);
        self.declare_func("rtrim", &[Type::String], Type::String);
        self.declare_func("len", &[Type::String], Type::Number);
        self.declare_func("instr", &[Type::String], Type::Number);
        self.declare_func(
            "substring",
            &[Type::String, Type::Number, Type::Number],
            Type::String,
        );
        self.declare_func(
            "replace",
            &[Type::String, Type::String, Type::String],
            Type::String,
        );
        self.declare_func("startswith", &[Type::String, Type::String], Type::Bool);
        self.declare_func("endswith", &[Type::String, Type::String], Type::Bool);
        self.declare_func("now", &[], Type::DateTime);
        self.declare_func("year", &[Type::Date], Type::Number);
        self.declare_func("month", &[Type::Date], Type::Number);
        self.declare_func("day", &[Type::Date], Type::Number);
        self.declare_func("hour", &[Type::Time], Type::Number);
        self.declare_func("minute", &[Type::Time], Type::Number);
        self.declare_func("second", &[Type::Time], Type::Number);
        self.declare_func("weekday", &[Type::Date], Type::Number);
        self.declare_func(
            "IF",
            &[Type::Bool, Type::Unspecified, Type::Unspecified],
            Type::Unspecified,
        );
        self.declare_agg_func(
            "count",
            FunArgsBuilder {
                args: &[Type::Bool],
                required: 0,
            },
            Type::Number,
        );
        self.declare_agg_func("sum", &[Type::Number], Type::Number);
        self.declare_agg_func("avg", &[Type::Number], Type::Number);
        self.declare_agg_func("min", &[Type::Number], Type::Number);
        self.declare_agg_func("max", &[Type::Number], Type::Number);
        self.declare_agg_func("median", &[Type::Number], Type::Number);
        self.declare_agg_func("stddev", &[Type::Number], Type::Number);
        self.declare_agg_func("variance", &[Type::Number], Type::Number);
        self.declare_agg_func("unique", &[Type::Unspecified], Type::Unspecified);
        self.declare_type()
            .data_source("eventtypes", Type::String)
            .data_source("subjects", Type::String)
            .define_record()
            .prop("specversion", Type::String)
            .prop("id", Type::String)
            .prop("time", Type::DateTime)
            .prop("source", Type::String)
            .prop("subject", Type::Subject)
            .prop("type", Type::String)
            .prop("datacontenttype", Type::String)
            .prop("data", Type::Unspecified)
            .prop("predecessorhash", Type::String)
            .prop("hash", Type::String)
            .prop("traceparent", Type::String)
            .prop("tracestate", Type::String)
            .prop("signature", Type::String)
            .as_default_event_type();

        self
    }

    /// Builds the `Session` object with the configured analysis options.
    ///
    /// This consumes the `SessionBuilder` and returns a `Session` instance
    /// ready for tokenizing, parsing, and analyzing EventQL queries.
    pub fn build(mut self) -> Session {
        self.arena.types.freeze();

        Session {
            arena: self.arena,
            options: self.options,
        }
    }
}

/// `Session` is the main entry point for parsing and analyzing EventQL queries.
///
/// It holds the necessary context, such as the expression arena and analysis options,
/// to perform lexical analysis, parsing, and static analysis of EQL query strings.
pub struct Session {
    arena: Arena,
    options: AnalysisOptions,
}

impl Session {
    /// Creates a new `SessionBuilder` for configuring and building a `Session`.
    ///
    /// This is the recommended way to create a `Session` instance, allowing
    /// for customization of functions, and event types.
    ///
    /// # Returns
    ///
    /// A new `SessionBuilder` instance.
    pub fn builder() -> SessionBuilder {
        SessionBuilder::default()
    }

    /// Tokenize an EventQL query string.
    ///
    /// This function performs lexical analysis on the input string, converting it
    /// into a sequence of tokens. Each token includes position information (line
    /// and column numbers) for error reporting.
    /// # Recognized Tokens
    ///
    /// - **Identifiers**: Alphanumeric names starting with a letter (e.g., `events`, `e`)
    /// - **Keywords**: Case-insensitive SQL-like keywords detected by the parser
    /// - **Numbers**: Floating-point literals (e.g., `42`, `3.14`)
    /// - **Strings**: Double-quoted string literals (e.g., `"hello"`)
    /// - **Operators**: Arithmetic (`+`, `-`, `*`, `/`), comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`), logical (`AND`, `OR`, `XOR`, `NOT`)
    /// - **Symbols**: Structural characters (`(`, `)`, `[`, `]`, `{`, `}`, `.`, `,`, `:`)
    pub fn tokenize<'a>(&self, input: &'a str) -> Result<Vec<Token<'a>>> {
        let tokens = tokenize(input)?;
        Ok(tokens)
    }

    /// Parse an EventQL query string into an abstract syntax tree.
    ///
    /// This is the main entry point for parsing EventQL queries. It performs both
    /// lexical analysis (tokenization) and syntactic analysis (parsing) in a single call.
    /// # Examples
    ///
    /// ```
    /// use eventql_parser::Session;
    ///
    /// // Parse a simple query
    /// let mut session = Session::builder().use_stdlib().build();
    /// let query = session.parse("FROM e IN events WHERE e.id == \"1\" PROJECT INTO e").unwrap();
    /// assert!(query.predicate.is_some());
    /// ```
    pub fn parse(&mut self, input: &str) -> Result<Query<Raw>> {
        let tokens = self.tokenize(input)?;
        Ok(parse(&mut self.arena, tokens.as_slice())?)
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
    pub fn run_static_analysis(&mut self, query: Query<Raw>) -> Result<Query<Typed>> {
        let mut analysis = self.analysis();
        Ok(analysis.analyze_query(query)?)
    }

    /// Converts a type name string to its corresponding [`Type`] variant.
    ///
    /// This function performs case-insensitive matching for built-in type names defined
    /// in the analysis options.
    ///
    /// # Returns
    ///
    /// * `Some(Type)` - If the name matches a built-in type
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
    pub fn resolve_type(&self, name: &str) -> Option<Type> {
        resolve_type_from_str(name)
    }

    /// Provides human-readable string formatting for types.
    ///
    /// Function types display optional parameters with a `?` suffix. For example,
    /// a function with signature `(boolean, number?) -> string` accepts 1 or 2 arguments.
    /// Aggregate functions use `=>` instead of `->` in their signature.
    pub fn display_type(&self, tpe: Type) -> String {
        display_type(&self.arena, tpe)
    }

    /// Creates an [`Analysis`] instance for fine-grained control over static analysis.
    ///
    /// Use this when you need to analyze individual expressions or manage scopes manually,
    /// rather than using [`run_static_analysis`](Session::run_static_analysis) for whole queries.
    pub fn analysis(&mut self) -> Analysis<'_> {
        Analysis::new(&mut self.arena, &self.options)
    }

    /// Returns a reference to the underlying [`Arena`].
    pub fn arena(&self) -> &Arena {
        &self.arena
    }

    /// Returns a mutable reference to the underlying [`Arena`].
    pub fn arena_mut(&mut self) -> &mut Arena {
        &mut self.arena
    }

    /// Returns the global [`Scope`]
    pub fn global_scope(&self) -> &Scope {
        &self.options.default_scope
    }
}
