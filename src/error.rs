//! Error types for lexical analysis and parsing.
//!
//! This module defines the error types that can occur during tokenization
//! and parsing of EventQL queries. All errors include position information
//! (line and column numbers) to help diagnose issues in query strings.

use crate::token::Symbol;
use thiserror::Error;

/// Top-level error type for the EventQL parser.
///
/// This enum wraps both lexer and parser errors, providing a unified
/// error type for the entire parsing pipeline.
#[derive(Debug, Error)]
pub enum Error {
    /// Error during lexical analysis (tokenization).
    #[error(transparent)]
    Lexer(LexerError),

    /// Error during syntactic analysis (parsing).
    #[error(transparent)]
    Parser(ParserError),

    /// Error during static analysis.
    #[error(transparent)]
    Analysis(AnalysisError),
}

/// Errors that can occur during lexical analysis.
///
/// These errors are produced by the tokenizer when the input string
/// contains invalid characters or ends unexpectedly.
#[derive(Debug, Error)]
pub enum LexerError {
    /// The input ended unexpectedly while parsing a token.
    ///
    /// This typically occurs when a string literal or other multi-character
    /// token is not properly closed.
    #[error("unexpected end of input")]
    IncompleteInput,

    /// An invalid character was encountered at the specified position.
    ///
    /// The tuple contains `(line_number, column_number)`.
    #[error("{0}:{1}: invalid character")]
    InvalidSymbol(u32, u32),
}

/// Errors that can occur during syntactic analysis.
///
/// These errors are produced by the parser when the token sequence
/// does not match the expected grammar of EventQL.
#[derive(Debug, Error)]
pub enum ParserError {
    /// Expected an identifier but found something else.
    ///
    /// Fields: `(line, column, found_token)`
    #[error("{0}:{1}: expected identifier but got {2}")]
    ExpectedIdent(u32, u32, String),

    /// Expected a specific keyword but found something else.
    ///
    /// Fields: `(line, column, expected_keyword, found_token)`
    #[error("{0}:{1}: expected keyword {2} but got {3}")]
    ExpectedKeyword(u32, u32, &'static str, String),

    /// Expected a specific symbol but found something else.
    ///
    /// Fields: `(line, column, expected_symbol, found_token)`
    /// ```
    #[error("{0}:{1}: expected {2} but got {3}")]
    ExpectedSymbol(u32, u32, Symbol, String),

    /// An unexpected token was encountered.
    ///
    /// Fields: `(line, column, found_token)`
    ///
    /// This is a general error for tokens that don't fit the current parse context.
    #[error("{0}:{1}: unexpected token {2}")]
    UnexpectedToken(u32, u32, String),

    /// The input ended unexpectedly while parsing.
    ///
    /// This occurs when the parser expects more tokens but encounters
    /// the end of the token stream.
    #[error("unexpected end of file")]
    UnexpectedEof,
}

#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("{0}:{1}: binding '{2}' already exists")]
    BindingAlreadyExists(u32, u32, String),
}
