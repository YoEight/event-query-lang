//! EventQL parser library for parsing event sourcing query language.
//!
//! This library provides a complete lexer and parser for EventQL (EQL), a query language
//! designed for event sourcing systems. It allows you to parse EQL query strings into
//! an abstract syntax tree (AST) that can be analyzed or executed.
pub mod analysis;
mod ast;
mod error;
mod lexer;
mod parser;
#[cfg(test)]
mod tests;
mod token;

use crate::error::{Error, LexerError};
use crate::prelude::{parse, tokenize};
pub use ast::*;
use nom::Err;

/// Convenience module that re-exports all public types and functions.
///
/// This module provides a single import point for all the library's public API,
/// including AST types, error types, lexer, parser, and token types.
pub mod prelude {
    pub use super::ast::*;
    pub use super::error::*;
    pub use super::lexer::*;
    pub use super::parser::*;
    pub use super::token::*;
}

pub type Result<A> = std::result::Result<A, error::Error>;

/// Parse an EventQL query string into an abstract syntax tree.
///
/// This is the main entry point for parsing EventQL queries. It performs both
/// lexical analysis (tokenization) and syntactic analysis (parsing) in a single call.
/// # Examples
///
/// ```
/// use eventql_parser::parse_query;
///
/// // Parse a simple query
/// let query = parse_query("FROM e IN events WHERE e.id == 1 PROJECT INTO e").unwrap();
/// assert!(query.predicate.is_some());
///
/// // Parse with multiple clauses
/// let complex = parse_query(
///     "FROM e IN events \
///      WHERE e.price > 100 \
///      ORDER BY e.timestamp DESC \
///      TOP 10 \
///      PROJECT INTO {id: e.id, price: e.price}"
/// ).unwrap();
/// assert!(complex.order_by.is_some());
/// assert!(complex.limit.is_some());
///
/// // Handle errors
/// match parse_query("FROM e IN events WHERE") {
///     Ok(_) => println!("Parsed successfully"),
///     Err(e) => println!("Parse error: {}", e),
/// }
/// ```
pub fn parse_query(input: &str) -> Result<Query<Raw>> {
    let tokens = tokenize(input).map_err(|e| match e {
        Err::Incomplete(_) => Error::Lexer(LexerError::IncompleteInput),
        Err::Error(x) => Error::Lexer(LexerError::InvalidSymbol(
            x.input.location_line(),
            x.input.get_column() as u32,
        )),
        Err::Failure(x) => Error::Lexer(LexerError::InvalidSymbol(
            x.input.location_line(),
            x.input.get_column() as u32,
        )),
    })?;

    parse(tokens.as_slice()).map_err(Error::Parser)
}
