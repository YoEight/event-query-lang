use crate::ast::{App, Attrs, Binary, Expr, Query, Source, SourceKind, Unary, Value};
use crate::error::ParserError;
use crate::token::{Operator, Sym, Symbol, Token};

pub type ParseResult<'a, A> = Result<A, ParserError<'a>>;

struct Parser<'a> {
    input: &'a [Token<'a>],
    offset: usize,
    scope: u64,
}

impl<'a> Parser<'a> {
    fn new(input: &'a [Token<'a>]) -> Self {
        Self {
            input,
            offset: 0,
            scope: 1,
        }
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
        expect_keyword(self.shift(), "from")?;
        let binding = self.parse_ident()?;
        expect_keyword(self.shift(), "in")?;
        let kind = self.parse_source_kind()?;

        Ok(Source { binding, kind })
    }

    fn parse_where_clause<'b>(&mut self) -> ParseResult<'a, Expr<'a>> {
        expect_keyword(self.shift(), "where")?;
        self.parse_expr()
    }

    fn parse_expr<'b>(&'b mut self) -> ParseResult<'a, Expr<'a>> {
        let token = self.peek();

        match token.sym {
            Sym::Eof => Err(ParserError::UnexpectedEof),

            Sym::Id(_)
            | Sym::String(_)
            | Sym::Number(_)
            | Sym::Symbol(Symbol::OpenParen | Symbol::OpenBracket)
            | Sym::Operator(Operator::Add | Operator::Sub) => self.parse_binary(0),

            _ => Err(ParserError::UnexpectedToken(token)),
        }
    }

    fn parse_primary<'b>(&'b mut self) -> ParseResult<'a, Expr<'a>> {
        let token = self.shift();

        let value = match token.sym {
            Sym::Id(name) => {
                if name.eq_ignore_ascii_case("true") {
                    Value::Bool(true)
                } else if name.eq_ignore_ascii_case("false") {
                    Value::Bool(false)
                } else {
                    if matches!(self.peek().sym, Sym::Symbol(Symbol::OpenParen)) {
                        self.shift();

                        let mut args = vec![];

                        if !matches!(self.peek().sym, Sym::Symbol(Symbol::CloseParen)) {
                            args.push(self.parse_expr()?);

                            while matches!(self.peek().sym, Sym::Symbol(Symbol::Comma)) {
                                self.shift();
                                args.push(self.parse_expr()?);
                            }
                        }

                        expect_symbol(self.shift(), Symbol::CloseParen)?;

                        Value::App(App { func: name, args })
                    } else {
                        Value::Id(name)
                    }
                }
            }

            Sym::String(s) => Value::String(s),
            Sym::Number(n) => Value::Number(n),

            Sym::Symbol(Symbol::OpenParen) => {
                let expr = self.parse_expr()?;
                expect_symbol(self.shift(), Symbol::CloseParen)?;

                Value::Group(Box::new(expr))
            }

            Sym::Symbol(Symbol::OpenBracket) => {
                let mut elems = vec![];

                if !matches!(self.peek().sym, Sym::Symbol(Symbol::CloseBracket)) {
                    elems.push(self.parse_expr()?);

                    while matches!(self.peek().sym, Sym::Symbol(Symbol::Comma)) {
                        self.shift();
                        elems.push(self.parse_expr()?);
                    }
                }

                expect_symbol(self.shift(), Symbol::CloseBracket)?;

                Value::Array(elems)
            }

            Sym::Operator(op) if matches!(op, Operator::Add | Operator::Sub) => {
                Value::Unary(Unary {
                    operator: op,
                    expr: Box::new(self.parse_expr()?),
                })
            }

            _ => return Err(ParserError::UnexpectedToken(token)),
        };

        Ok(Expr {
            attrs: Attrs::new(token.into(), self.scope),
            value,
        })
    }

    fn parse_binary<'b>(&'b mut self, min_bind: u64) -> ParseResult<'a, Expr<'a>> {
        let mut lhs = self.parse_primary()?;

        loop {
            let token = self.peek();
            if matches!(token.sym, Sym::Eof | Sym::Symbol(Symbol::CloseParen)) {
                break;
            }

            let operator = if let Sym::Operator(op) = token.sym {
                op
            } else {
                return Err(ParserError::ExpectedOperator(token));
            };

            let (lhs_bind, rhs_bind) = binding_pow(operator);

            if lhs_bind < min_bind {
                break;
            }

            self.shift();
            let rhs = self.parse_binary(rhs_bind)?;

            lhs = Expr {
                attrs: lhs.attrs,
                value: Value::Binary(Binary {
                    lhs: Box::new(lhs),
                    operator,
                    rhs: Box::new(rhs),
                }),
            };
        }

        Ok(lhs)
    }

    fn parse_query<'b>(&'b mut self) -> ParseResult<'a, Query<'a>> {
        self.scope += 1;
        let mut sources = vec![];

        while let Sym::Id(name) = self.peek().sym
            && name.eq_ignore_ascii_case("from")
        {
            sources.push(self.parse_source()?);
        }

        if let Sym::Id(name) = self.peek().sym
            && name.eq_ignore_ascii_case("where")
        {
            self.parse_where_clause()?;
        }

        self.scope -= 1;
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

fn binding_pow(op: Operator) -> (u64, u64) {
    match op {
        Operator::Add | Operator::Sub => (10, 11),
        Operator::Mul | Operator::Div => (20, 21),
        _ => (1, 2),
    }
}

pub fn parse<'a>(input: &'a [Token<'a>]) -> ParseResult<'a, Query<'a>> {
    let mut parser = Parser::new(input);

    parser.parse_query()
}
