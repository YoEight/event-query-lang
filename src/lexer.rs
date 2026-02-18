//! Lexical analysis (tokenization) for EventQL.
//!
//! This module provides the tokenizer that converts raw EventQL query strings
//! into a sequence of tokens. The tokenizer is built using the `nom` parser
//! combinator library.
//!
//! # Main Function
//!
//! - [`tokenize`] - Convert a query string into a vector of tokens
use crate::error::LexerError;
use crate::token::{Operator, Sym, Symbol, Text, Token};
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while};
use nom::character::complete::{alpha1, char, multispace1, satisfy};
use nom::character::one_of;
use nom::combinator::{eof, opt, recognize};
use nom::error::{Error, context};
use nom::multi::many0;
use nom::number::complete::double;
use nom::sequence::{delimited, pair};
use nom::{AsChar, IResult, Parser};

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
pub(crate) fn tokenize(input: &str) -> Result<Vec<Token<'_>>, LexerError> {
    let mut input = Text::new(input);
    let mut tokens = Vec::new();

    loop {
        let (remaining, token) = token(input).map_err(massage_nom_error)?;
        input = remaining;

        tokens.push(token);

        if matches!(token.sym, Sym::Eof) {
            break;
        }
    }

    Ok(tokens)
}

fn massage_nom_error(err: nom::Err<Error<Text>>) -> LexerError {
    match err {
        nom::Err::Incomplete(_) => LexerError::IncompleteInput,
        nom::Err::Error(err) => {
            LexerError::InvalidSymbol(err.input.location_line(), err.input.get_column() as u32)
        }
        nom::Err::Failure(err) => {
            LexerError::InvalidSymbol(err.input.location_line(), err.input.get_column() as u32)
        }
    }
}

fn token(input: Text) -> IResult<Text, Token> {
    delimited(
        skip_whitespace_and_comments,
        parse_token,
        skip_whitespace_and_comments,
    )
    .parse(input)
}

fn skip_whitespace_and_comments(input: Text) -> IResult<Text, Text> {
    recognize(many0(alt((comment, multispace1)))).parse(input)
}

fn comment(input: Text) -> IResult<Text, Text> {
    recognize(pair(
        pair(tag("//"), take_while(|c: char| c != '\n' && c != '\r')),
        opt(alt((tag("\r\n"), tag("\n"), tag("\r")))),
    ))
    .parse(input)
}

fn parse_token(input: Text) -> IResult<Text, Token> {
    alt((end_of_file, symbol, operator, ident, number, string)).parse(input)
}

fn symbol(input: Text) -> IResult<Text, Token> {
    one_of("().,:[]{}")
        .map(|c| match c {
            '(' => Symbol::OpenParen,
            ')' => Symbol::CloseParen,
            '.' => Symbol::Dot,
            ',' => Symbol::Comma,
            ':' => Symbol::Colon,
            '[' => Symbol::OpenBracket,
            ']' => Symbol::CloseBracket,
            '{' => Symbol::OpenBrace,
            '}' => Symbol::CloseBrace,
            _ => unreachable!(),
        })
        .map(move |sym| Token {
            sym: Sym::Symbol(sym),
            line: input.location_line(),
            col: input.get_column() as u32,
        })
        .parse(input)
}

fn end_of_file(input: Text) -> IResult<Text, Token> {
    eof.map(|_| Token {
        sym: Sym::Eof,
        line: input.location_line(),
        col: input.get_column() as u32,
    })
    .parse(input)
}

fn operator(input: Text) -> IResult<Text, Token> {
    alt((operator_1, operator_2)).parse(input)
}

fn operator_1(input: Text) -> IResult<Text, Token> {
    one_of("+-*/^")
        .map(|c| match c {
            '+' => Operator::Add,
            '-' => Operator::Sub,
            '*' => Operator::Mul,
            '/' => Operator::Div,
            _ => unreachable!(),
        })
        .map(move |op| Token {
            sym: Sym::Operator(op),
            line: input.location_line(),
            col: input.get_column() as u32,
        })
        .parse(input)
}

fn operator_2(input: Text) -> IResult<Text, Token> {
    one_of("<>!=")
        .flat_map(|c| {
            context(
                "valid character when parsing an operator",
                opt(char('=')).map_opt(move |eq_opt| match (c, eq_opt.is_some()) {
                    ('<', false) => Some(Operator::Lt),
                    ('<', true) => Some(Operator::Lte),
                    ('>', false) => Some(Operator::Gt),
                    ('>', true) => Some(Operator::Gte),
                    ('!', true) => Some(Operator::Neq),
                    ('=', true) => Some(Operator::Eq),
                    _ => None,
                }),
            )
        })
        .map(move |op| Token {
            sym: Sym::Operator(op),
            line: input.location_line(),
            col: input.get_column() as u32,
        })
        .parse(input)
}

fn ident(input: Text) -> IResult<Text, Token> {
    recognize(pair(
        alpha1,
        many0(satisfy(|c| AsChar::is_alphanum(c) || c == '_')),
    ))
    .map(|value: Text| {
        let sym = if value.fragment().eq_ignore_ascii_case("and") {
            Sym::Operator(Operator::And)
        } else if value.fragment().eq_ignore_ascii_case("or") {
            Sym::Operator(Operator::Or)
        } else if value.fragment().eq_ignore_ascii_case("xor") {
            Sym::Operator(Operator::Xor)
        } else if value.fragment().eq_ignore_ascii_case("not") {
            Sym::Operator(Operator::Not)
        } else if value.fragment().eq_ignore_ascii_case("contains") {
            Sym::Operator(Operator::Contains)
        } else if value.fragment().eq_ignore_ascii_case("as") {
            Sym::Operator(Operator::As)
        } else {
            Sym::Id(value.fragment())
        };

        Token {
            sym,
            line: value.location_line(),
            col: value.get_column() as u32,
        }
    })
    .parse(input)
}

fn number(input: Text) -> IResult<Text, Token> {
    double
        .map(|value| Token {
            sym: Sym::Number(value),
            line: input.location_line(),
            col: input.get_column() as u32,
        })
        .parse(input)
}

fn string(input: Text) -> IResult<Text, Token> {
    delimited(char('"'), take_while(|c| c != '"'), char('"'))
        .map(|value: Text| Token {
            sym: Sym::String(value.fragment()),
            line: input.location_line(),
            col: input.get_column() as u32,
        })
        .parse(input)
}
