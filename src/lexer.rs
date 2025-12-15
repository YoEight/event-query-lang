use crate::token::{Operator, Pos, Sym, Symbol, Token};
use nom::branch::alt;
use nom::character::complete::{alpha1, alphanumeric0, char, multispace0};
use nom::character::{digit1, one_of};
use nom::combinator::{fail, map, opt, recognize};
use nom::error::{Error, context};
use nom::number::double;
use nom::sequence::{delimited, pair};
use nom::{IResult, Parser, combinator};

fn parse_token<'a>() -> impl Parser<Pos<'a>, Output = Token<'a>> {
    delimited(multispace0, token, multispace0)
}

fn token(input: Pos) -> IResult<Pos, Token> {
    alt((
        eof,
        symbol,
        operator,
        ident,
        number,
        context("invalid character", fail()),
    ))
    .parse(input)
}

fn eof(input: Pos) -> IResult<Pos, Token> {
    (combinator::eof::<_, Error<Pos>>)
        .map(|pos| Token {
            sym: Sym::Eof,
            line: pos.location_line(),
            col: pos.get_column() as u32,
        })
        .parse(input)
}

fn symbol(input: Pos) -> IResult<Pos, Token> {
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

fn operator(input: Pos) -> IResult<Pos, Token> {
    alt((operator_1, operator_2)).parse(input)
}

fn operator_1(input: Pos) -> IResult<Pos, Token> {
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

fn operator_2(input: Pos) -> IResult<Pos, Token> {
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

fn ident(input: Pos) -> IResult<Pos, Token> {
    recognize(pair(alpha1, alphanumeric0))
        .map(|value: Pos| Token {
            sym: Sym::Id(value.fragment()),
            line: value.location_line(),
            col: value.get_column() as u32,
        })
        .parse(input)
}

fn number(input: Pos) -> IResult<Pos, Token> {
    alt((
        map(double(), |value| Sym::Float(value)),
        map(digit1(), |value: Pos| {
            Sym::Integer(value.fragment().parse().unwrap())
        }),
    ))
    .map(|sym| Token {
        sym,
        line: input.location_line(),
        col: input.get_column() as u32,
    })
    .parse(input)
}
