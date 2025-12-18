use crate::ast::{Query, Source, SourceKind};
use crate::error::ParserError;
use crate::token::{Sym, Symbol, Token};

pub type ParseResult<'a, A> = Result<A, ParserError<'a>>;

struct Parser<'a> {
    input: &'a [Token<'a>],
    offset: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a [Token<'a>]) -> Self {
        Self { input, offset: 0 }
    }

    fn peek<'b>(&'b self) -> Token<'a> {
        self.input[self.offset]
    }

    fn shift<'b>(&'b mut self) -> Token<'a> {
        let res = self.input[self.offset];

        if self.offset + 1 < self.input.len() {
            self.offset += 1;
        }

        res
    }

    fn parse_ident<'b>(&'b mut self) -> ParseResult<'a, &'a str> {
        let token = self.shift();

        if let Sym::Id(id) = token.sym {
            return Ok(id);
        }

        Err(ParserError::ExpectedIdent(token))
    }

    fn parse_source_kind<'b>(&'b mut self) -> ParseResult<'a, SourceKind<'a>> {
        let token = self.shift();
        match token.sym {
            Sym::Id(id) if id.eq_ignore_ascii_case("events") => Ok(SourceKind::Events),
            Sym::String(sub) => Ok(SourceKind::Subject(sub)),
            Sym::Symbol(sym) if matches!(sym, Symbol::OpenParen) => {
                let query = self.parse_query()?;
                expect_symbol(self.shift(), Symbol::CloseParen)?;

                Ok(SourceKind::Subquery(query))
            }
            _ => Err(ParserError::UnexpectedToken(token)),
        }
    }

    fn parse_source<'b>(&'b mut self) -> ParseResult<'a, Source<'a>> {
        let token = self.shift();
        expect_keyword(token, "from")?;
        let binding = self.parse_ident()?;
        expect_keyword(token, "in")?;
        let kind = self.parse_source_kind()?;

        Ok(Source { binding, kind })
    }

    fn parse_query<'b>(&'b mut self) -> ParseResult<'a, Query<'a>> {
        let mut sources = vec![];

        while let Sym::Id(name) = self.peek().sym
            && name.eq_ignore_ascii_case("from")
        {
            sources.push(self.parse_source()?);
        }

        if let Sym::Id(name) = self.peek().sym
            && name.eq_ignore_ascii_case("where")
        {
            todo!("parse where clause")
        }

        todo!()
    }
}

fn expect_keyword<'a, 'b>(token: Token<'a>, keyword: &'static str) -> ParseResult<'a, ()> {
    if let Sym::Id(id) = token.sym
        && id.eq_ignore_ascii_case(keyword)
    {
        return Ok(());
    }

    Err(ParserError::ExpectedKeyword(keyword, token))
}

fn expect_symbol(token: Token, expect: Symbol) -> ParseResult<()> {
    if let Sym::Symbol(sym) = token.sym
        && sym == expect
    {
        return Ok(());
    }

    Err(ParserError::ExpectedSymbol(expect, token))
}

pub fn parse<'a>(input: &[Token<'a>]) -> ParseResult<'a, ()> {
    Ok(())
}
