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

pub mod prelude {
    pub use super::ast::*;
    pub use super::error::*;
    pub use super::lexer::*;
    pub use super::parser::*;
    pub use super::token::*;
}

pub type Result<A> = std::result::Result<A, error::Error>;

pub fn parse_query(input: &str) -> Result<Query> {
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
