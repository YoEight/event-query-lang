//! Token types for lexical analysis.
//!
//! This module defines the types used during tokenization (lexical analysis)
//! of EventQL queries. Tokens are the atomic units produced by the lexer
//! and consumed by the parser.
//!
//! # Core Types
//!
//! - [`Token`] - A token with position information
//! - [`Sym`] - The symbol/category of a token
//! - [`Operator`] - Arithmetic, comparison, and logical operators
//! - [`Symbol`] - Structural symbols (parentheses, brackets, etc.)

use nom_locate::LocatedSpan;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Symbol type representing the category and value of a token.
///
/// This enum encompasses all the different types of tokens that can appear
/// in an EventQL query, from keywords and operators to literals and symbols.
#[derive(Clone, Debug, Copy, Serialize)]
pub enum Sym<'a> {
    /// Identifier (variable names, keywords not yet classified)
    Id(&'a str),
    /// String literal
    String(&'a str),
    /// Numeric literal
    Number(f64),
    /// Keyword (FROM, WHERE, etc.)
    Keyword(&'a str),
    /// Operator (+, -, ==, AND, etc.)
    Operator(Operator),
    /// Structural symbol (parentheses, brackets, etc.)
    Symbol(Symbol),
    /// End of file marker
    Eof,
}

impl Display for Sym<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Sym::Id(id) => write!(f, "{}", id),
            Sym::String(s) => write!(f, "\"{}\"", s),
            Sym::Number(n) => write!(f, "{}", n),
            Sym::Keyword(kw) => write!(f, "{}", kw.to_uppercase()),
            Sym::Operator(op) => write!(f, "{}", op),
            Sym::Symbol(sym) => write!(f, "{}", sym),
            Sym::Eof => write!(f, "<eof>"),
        }
    }
}

/// Operator types for expressions.
///
/// This enum represents all the operators supported in EventQL, including
/// arithmetic, comparison, and logical operators.
///
/// # Operator Precedence
///
/// From highest to lowest precedence:
/// 1. Unary: `+`, `-`, `NOT`
/// 2. Multiplicative: `*`, `/`
/// 3. Additive: `+`, `-`
/// 4. Comparison: `<`, `<=`, `>`, `>=`, `==`, `!=`
/// 5. Logical: `AND`, `OR`, `XOR`
#[derive(Clone, Debug, Copy, PartialEq, Hash, Serialize)]
pub enum Operator {
    /// Addition operator `+`
    Add,
    /// Subtraction operator `-`
    Sub,
    /// Multiplication operator `*`
    Mul,
    /// Division operator `/`
    Div,
    /// Equality operator `==`
    Eq,
    /// Inequality operator `!=`
    Neq,
    /// Less than operator `<`
    Lt,
    /// Less than or equal operator `<=`
    Lte,
    /// Greater than operator `>`
    Gt,
    /// Greater than or equal operator `>=`
    Gte,
    /// Logical AND operator
    And,
    /// Logical OR operator
    Or,
    /// Logical XOR operator
    Xor,
    /// Logical NOT operator
    Not,
    /// Containment (`array CONTAINS value`)
    Contains,
    /// Type conversion (`e.foo as STRING`)
    As,
}

impl Display for Operator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::Add => write!(f, "+"),
            Operator::Sub => write!(f, "-"),
            Operator::Mul => write!(f, "*"),
            Operator::Div => write!(f, "/"),
            Operator::Eq => write!(f, "=="),
            Operator::Neq => write!(f, "!="),
            Operator::Lt => write!(f, "<"),
            Operator::Lte => write!(f, "<="),
            Operator::Gt => write!(f, ">"),
            Operator::Gte => write!(f, ">="),
            Operator::And => write!(f, "AND"),
            Operator::Or => write!(f, "OR"),
            Operator::Xor => write!(f, "XOR"),
            Operator::Not => write!(f, "NOT"),
            Operator::Contains => write!(f, "CONTAINS"),
            Operator::As => write!(f, "AS"),
        }
    }
}

/// Structural symbols used in EventQL syntax.
///
/// These symbols define the structure of expressions and queries,
/// such as parentheses for grouping, brackets for arrays, and braces for records.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Symbol {
    /// Opening parenthesis `(`
    OpenParen,
    /// Closing parenthesis `)`
    CloseParen,
    /// Dot `.` (for field access)
    Dot,
    /// Comma `,` (for separating list items)
    Comma,
    /// Colon `:` (for record field definitions)
    Colon,
    /// Opening bracket `[`
    OpenBracket,
    /// Closing bracket `]`
    CloseBracket,
    /// Opening brace `{`
    OpenBrace,
    /// Closing brace `}`
    CloseBrace,
}

impl Display for Symbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Symbol::OpenParen => write!(f, "("),
            Symbol::CloseParen => write!(f, ")"),
            Symbol::Dot => write!(f, "."),
            Symbol::Comma => write!(f, ","),
            Symbol::Colon => write!(f, ":"),
            Symbol::OpenBracket => write!(f, "["),
            Symbol::CloseBracket => write!(f, "]"),
            Symbol::OpenBrace => write!(f, "{{"),
            Symbol::CloseBrace => write!(f, "}}"),
        }
    }
}

/// Type alias for text with position tracking.
///
/// Used internally by the lexer for tracking positions during tokenization.
pub type Text<'a> = LocatedSpan<&'a str>;

/// A token with position information.
///
/// Tokens are the output of lexical analysis and the input to syntactic analysis.
/// Each token contains a symbol (its type and value) along with its position
/// in the source code.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Token<'a> {
    /// The symbol (type and value) of this token
    pub sym: Sym<'a>,
    /// Line number where this token appears (1-indexed)
    pub line: u32,
    /// Column number where this token appears (1-indexed)
    pub col: u32,
}
