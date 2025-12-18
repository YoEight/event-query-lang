use crate::token::{Operator, Sym, Symbol, Text, Token};
use nom::branch::alt;
use nom::bytes::complete::take_while;
use nom::character::complete::{alpha1, alphanumeric0, char, multispace0};
use nom::character::one_of;
use nom::combinator::{eof, fail, opt, recognize};
use nom::error::{context, Error};
use nom::number::complete::double;
use nom::sequence::{delimited, pair};
use nom::{IResult, Parser};

pub fn tokenize(input: &str) -> Result<Vec<Token<'_>>, nom::Err<Error<Text<'_>>>> {
    let mut input = Text::new(input);
    let mut tokens = Vec::new();

    loop {
        let (remaining, token) = token(input)?;
        input = remaining;

        tokens.push(token);

        if matches!(token.sym, Sym::Eof) {
            break;
        }
    }

    Ok(tokens)
}

fn token(input: Text) -> IResult<Text, Token> {
    delimited(
        multispace0,
        alt((
            end_of_file,
            symbol,
            operator,
            ident,
            number,
            string,
            context("invalid character", fail()),
        )),
        multispace0,
    )
    .parse(input)
}

fn symbol(input: Text) -> IResult<Text, Token> {
    one_of("().,:[]{}")
        .map(|c| match c {
            '(' => Symbol::OpenBrace,
            ')' => Symbol::CloseBrace,
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
    one_of("+-*/=^")
        .map(|c| match c {
            '+' => Operator::Add,
            '-' => Operator::Sub,
            '*' => Operator::Mul,
            '/' => Operator::Div,
            '=' => Operator::Eq,
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
    one_of("<>!")
        .flat_map(|c| {
            context(
                "valid character when parsing an operator",
                opt(char('=')).map_opt(move |eq_opt| match (c, eq_opt.is_some()) {
                    ('<', false) => Some(Operator::Lt),
                    ('<', true) => Some(Operator::Lte),
                    ('>', false) => Some(Operator::Gt),
                    ('>', true) => Some(Operator::Gte),
                    ('!', true) => Some(Operator::Neq),
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
    recognize(pair(alpha1, alphanumeric0))
        .map(|value: Text| Token {
            sym: Sym::Id(value.fragment()),
            line: value.location_line(),
            col: value.get_column() as u32,
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
