//! EventQL parser library for parsing event sourcing query language.
//!
//! This library provides a complete lexer and parser for EventQL (EQL), a query language
//! designed for event sourcing systems. It allows you to parse EQL query strings into
//! an abstract syntax tree (AST) that can be analyzed or executed.
mod analysis;
pub mod arena;
mod ast;
mod error;
mod lexer;
mod parser;
#[cfg(test)]
mod tests;
mod token;

use crate::arena::ExprArena;
use crate::lexer::tokenize;
use crate::prelude::{AnalysisOptions, Typed, parse, static_analysis};
use crate::token::Token;
pub use ast::*;
use rustc_hash::FxHashMap;
use unicase::Ascii;

/// Convenience module that re-exports all public types and functions.
///
/// This module provides a single import point for all the library's public API,
/// including AST types, error types, lexer, parser, and token types.
pub mod prelude {
    pub use super::analysis::*;
    pub use super::ast::*;
    pub use super::error::*;
    pub use super::parser::*;
    pub use super::token::*;
}

pub type Result<A> = std::result::Result<A, error::Error>;

pub struct SessionBuilder {
    options: AnalysisOptions,
}

impl SessionBuilder {
    pub fn declare_func(self, name: &str, args: impl Into<FunArgs>, result: Type) -> Self {
        self.declare_func_when(true, name, args, result)
    }

    pub fn declare_func_when(
        mut self,
        test: bool,
        name: &str,
        args: impl Into<FunArgs>,
        result: Type,
    ) -> Self {
        if test {
            self.options.default_scope.entries.insert(
                name,
                Type::App {
                    args: args.into(),
                    result: Box::new(result),
                    aggregate: false,
                },
            );
        }

        self
    }

    pub fn declare_agg_func(self, name: &str, args: impl Into<FunArgs>, result: Type) -> Self {
        self.declare_agg_func_when(true, name, args, result)
    }

    pub fn declare_agg_func_when(
        mut self,
        test: bool,
        name: &str,
        args: impl Into<FunArgs>,
        result: Type,
    ) -> Self {
        if test {
            self.options.default_scope.entries.insert(
                name,
                Type::App {
                    args: args.into(),
                    result: Box::new(result),
                    aggregate: true,
                },
            );
        }

        self
    }

    pub fn declare_event_type_when(mut self, test: bool, tpe: Type) -> Self {
        if test {
            self.options.event_type_info = tpe;
        }

        self
    }

    pub fn declare_event_type(mut self, tpe: Type) -> Self {
        self.options.event_type_info = tpe;
        self
    }

    pub fn declare_custom_type_when(mut self, test: bool, name: &str) -> Self {
        if test {
            self.options
                .custom_types
                .insert(Ascii::new(name.to_owned()));
        }

        self
    }

    pub fn declare_custom_type(mut self, name: &str) -> Self {
        self.options
            .custom_types
            .insert(Ascii::new(name.to_owned()));
        self
    }

    pub fn use_stdlib(self) -> Self {
        self.declare_func("ABS", vec![Type::Number], Type::Number)
            .declare_func("CEIL", vec![Type::Number], Type::Number)
            .declare_func("FLOOR", vec![Type::Number], Type::Number)
            .declare_func("ROUND", vec![Type::Number], Type::Number)
            .declare_func("COS", vec![Type::Number], Type::Number)
            .declare_func("EXP", vec![Type::Number], Type::Number)
            .declare_func("POW", vec![Type::Number, Type::Number], Type::Number)
            .declare_func("SQRT", vec![Type::Number], Type::Number)
            .declare_func("RAND", vec![], Type::Number)
            .declare_func("PI", vec![Type::Number], Type::Number)
            .declare_func("LOWER", vec![Type::String], Type::String)
            .declare_func("UPPER", vec![Type::String], Type::String)
            .declare_func("TRIM", vec![Type::String], Type::String)
            .declare_func("LTRIM", vec![Type::String], Type::String)
            .declare_func("RTRIM", vec![Type::String], Type::String)
            .declare_func("LEN", vec![Type::String], Type::Number)
            .declare_func("INSTR", vec![Type::String], Type::Number)
            .declare_func(
                "SUBSTRING",
                vec![Type::String, Type::Number, Type::Number],
                Type::String,
            )
            .declare_func(
                "REPLACE",
                vec![Type::String, Type::String, Type::String],
                Type::String,
            )
            .declare_func("STARTSWITH", vec![Type::String, Type::String], Type::Bool)
            .declare_func("ENDSWITH", vec![Type::String, Type::String], Type::Bool)
            .declare_func("NOW", vec![], Type::DateTime)
            .declare_func("YEAR", vec![Type::Date], Type::Number)
            .declare_func("MONTH", vec![Type::Date], Type::Number)
            .declare_func("DAY", vec![Type::Date], Type::Number)
            .declare_func("HOUR", vec![Type::Time], Type::Number)
            .declare_func("MINUTE", vec![Type::Time], Type::Number)
            .declare_func("SECOND", vec![Type::Time], Type::Number)
            .declare_func("WEEKDAY", vec![Type::Date], Type::Number)
            .declare_func(
                "IF",
                vec![Type::Bool, Type::Unspecified, Type::Unspecified],
                Type::Unspecified,
            )
            .declare_agg_func(
                "COUNT",
                FunArgs {
                    values: vec![Type::Bool],
                    needed: 0,
                },
                Type::Number,
            )
            .declare_agg_func("SUM", vec![Type::Number], Type::Number)
            .declare_agg_func("AVG", vec![Type::Number], Type::Number)
            .declare_agg_func("MIN", vec![Type::Number], Type::Number)
            .declare_agg_func("MAX", vec![Type::Number], Type::Number)
            .declare_agg_func("MEDIAN", vec![Type::Number], Type::Number)
            .declare_agg_func("STDDEV", vec![Type::Number], Type::Number)
            .declare_agg_func("VARIANCE", vec![Type::Number], Type::Number)
            .declare_agg_func("UNIQUE", vec![Type::Unspecified], Type::Unspecified)
            .declare_event_type(Type::Record(FxHashMap::from_iter([
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
            ])))
    }

    pub fn build(self) -> Session {
        Session {
            arena: ExprArena::default(),
            options: self.options,
        }
    }
}

impl Default for SessionBuilder {
    fn default() -> Self {
        Self {
            options: AnalysisOptions::empty(),
        }
    }
}

pub struct Session {
    arena: ExprArena,
    options: AnalysisOptions,
}

impl Session {
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
    pub fn run_static_analysis(&self, query: Query<Raw>) -> Result<Query<Typed>> {
        Ok(static_analysis(&self.arena, &self.options, query)?)
    }
}
