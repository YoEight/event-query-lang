use std::num::{ParseFloatError, ParseIntError};

use crate::token::{Symbol, Token};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LexerError {
    #[error("unexpected end of query")]
    UnexpectedEOF,

    #[error("unexpected symbol '{0}'")]
    UnexpectedSymbol(String),

    #[error("malformed flaoting number: {0:?}")]
    MalformedFloatingNumber(Option<ParseFloatError>),

    #[error("malformed integral number: {0}")]
    MalformedIntegralNumber(ParseIntError),

    #[error("string literal is not closed properly")]
    StringLiteralNotClosed,
}

#[derive(Debug)]
pub enum ParserError<'a> {
    ExpectedIdent(Token<'a>),
    ExpectedKeyword(&'static str, Token<'a>),
    ExpectedSymbol(Symbol, Token<'a>),
    ExpectedOperator(Token<'a>),
    UnexpectedToken(Token<'a>),
    UnexpectedEof,
}
