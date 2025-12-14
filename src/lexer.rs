use crate::token::{Operator, Pos, Sym, Symbol, Token};
use nom::branch::alt;
use nom::bytes::complete::take_while1;
use nom::character::complete::{char, multispace0};
use nom::character::one_of;
use nom::combinator::{fail, map, opt, success};
use nom::error::{Error, context};
use nom::sequence::delimited;
use nom::{AsChar, IResult, Parser, combinator};
// pub fn next_token(pos: Pos) -> IResult<Pos, Token> {
//     let (pos, _) = take_while(|c: char| c.is_ascii_whitespace())(pos)?;
//     let (_, is_eof) = map(opt(eof), |res| res.is_some()).parse(pos)?;
//
//     if is_eof {
//         return Ok((pos, Token::new(Sym::Eof, pos, "")));
//     }
//
//     let (pos, c) = peek(anychar).parse(pos)?;
//
//     if matches!(c, '(' | ')' | '.' | ',' | ':' | '[' | ']' | '{' | '}') {
//         return map(take(1usize), |v| Token::new(Sym::Symbol, v)).parse(pos);
//     }
//
//     if matches!(c, '+' | '-' | '*' | '/' | '=' | '^') {
//         return map(take(1usize), |v| Token::new(Sym::Operator, v)).parse(pos);
//     }
//
//     if matches!(c, '<' | '>') {
//         let (tmp, _) = anychar(pos)?;
//         let (_, ok) = opt(char('=')).parse(tmp)?;
//
//         let parser = if ok.is_some() {
//             take(2usize)
//         } else {
//             take(1usize)
//         };
//
//         return map(parser, |v| Token::new(Sym::Operator, v)).parse(pos);
//     }
//
//     if matches!(c, '!') {
//         let (pos, _) = anychar(pos)?;
//         let (_, ok) = opt(char('=')).parse(pos)?;
//
//         if ok.is_some() {
//             failure("invalid symbol '='", pos)?;
//         }
//
//         return Ok((pos, Token::new(Sym::Operator, pos, "!")));
//     }
//
//     if c.is_ascii_alphabetic() {
//         return map(take_while(AsChar::is_alpha), |v| Token::new(Sym::Id, v)).parse(pos);
//     }
//
//     if c.is_ascii_digit() {
//         return map(take_while(AsChar::is_dec_digit), |v| {
//             Token::new(Sym::Integer, v)
//         })
//         .parse(pos);
//     }
//
//     failure("unexpected character", pos)
// }

// fn failure<'a>(reason: &'static str, pos: Pos<'a>) -> IResult<Pos<'a>, Token<'a>> {
//     context(reason, fail()).parse(pos)
// }

fn parse_token<'a>() -> impl Parser<Pos<'a>, Output = Token<'a>> {
    delimited(multispace0, token, multispace0)
}

fn token(input: Pos) -> IResult<Pos, Token> {
    alt((
        eof,
        symbol,
        operator,
        id,
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

fn id(input: Pos) -> IResult<Pos, Token> {
    map(take_while1(AsChar::is_alpha), |value: Pos| Token {
        sym: Sym::Id(value.fragment()),
        line: value.location_line(),
        col: input.get_column() as u32,
    })
    .parse(input)
}
