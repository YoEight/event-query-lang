use crate::token::Symbol;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Lexer(LexerError),

    #[error(transparent)]
    Parser(ParserError),
}

#[derive(Debug, Error)]
pub enum LexerError {
    #[error("unexpected end of input")]
    IncompleteInput,

    #[error("{0}:{1}: invalid character")]
    InvalidSymbol(u32, u32),
}

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("{0}:{1}: expected identifier but got {2}")]
    ExpectedIdent(u32, u32, String),

    #[error("{0}:{1}: expected keyword {2} but got {3}")]
    ExpectedKeyword(u32, u32, &'static str, String),

    #[error("{0}:{1}: expected {2} but got {3}")]
    ExpectedSymbol(u32, u32, Symbol, String),

    #[error("{0}:{1}: unexpected token {2}")]
    UnexpectedToken(u32, u32, String),

    #[error("unexpected end of file")]
    UnexpectedEof,
}
