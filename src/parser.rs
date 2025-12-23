//! Syntactic analysis (parsing) for EventQL.
//!
//! This module provides the parser that converts a sequence of tokens into
//! an abstract syntax tree (AST). The parser implements a recursive descent
//! parser with operator precedence for expressions.
//!
//! # Main Function
//!
//! - [`parse`] - Convert a slice of tokens into a Query AST
use std::marker::PhantomData;

use crate::ast::{
    Access, App, Attrs, Binary, Expr, Field, Limit, Order, OrderBy, Query, Source, SourceKind,
    Unary, Value,
};
use crate::error::ParserError;
use crate::token::{Operator, Sym, Symbol, Token};
use crate::{GroupBy, Raw};

/// Result type for parser operations.
///
/// This is a convenience alias for `Result<T, ParserError>`.
pub type ParseResult<A> = Result<A, ParserError>;

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
            scope: 0,
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

    fn parse_ident(&mut self) -> ParseResult<String> {
        let token = self.shift();

        if let Sym::Id(id) = token.sym {
            return Ok(id.to_owned());
        }

        Err(ParserError::ExpectedIdent(
            token.line,
            token.col,
            token.sym.to_string(),
        ))
    }

    fn parse_source_kind(&mut self) -> ParseResult<SourceKind<Raw>> {
        let token = self.shift();
        match token.sym {
            Sym::Id(id) => Ok(SourceKind::Name(id.to_owned())),
            Sym::String(sub) => Ok(SourceKind::Subject(sub.to_owned())),
            Sym::Symbol(Symbol::OpenParen) => {
                let query = self.parse_query()?;
                expect_symbol(self.shift(), Symbol::CloseParen)?;

                Ok(SourceKind::Subquery(Box::new(query)))
            }
            _ => Err(ParserError::UnexpectedToken(
                token.line,
                token.col,
                token.sym.to_string(),
            )),
        }
    }

    fn parse_source(&mut self) -> ParseResult<Source<Raw>> {
        expect_keyword(self.shift(), "from")?;
        let binding = self.parse_ident()?;
        expect_keyword(self.shift(), "in")?;
        let kind = self.parse_source_kind()?;

        Ok(Source { binding, kind })
    }

    fn parse_where_clause(&mut self) -> ParseResult<Expr> {
        expect_keyword(self.shift(), "where")?;
        self.parse_expr()
    }

    fn parse_group_by(&mut self) -> ParseResult<GroupBy> {
        expect_keyword(self.shift(), "group")?;
        expect_keyword(self.shift(), "by")?;

        let expr = self.parse_expr()?;

        let predicate = if let Sym::Id(name) = self.peek().sym
            && name.eq_ignore_ascii_case("having")
        {
            self.shift();

            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(GroupBy { expr, predicate })
    }

    fn parse_order_by(&mut self) -> ParseResult<OrderBy> {
        expect_keyword(self.shift(), "order")?;
        expect_keyword(self.shift(), "by")?;

        let expr = self.parse_expr()?;
        let token = self.shift();

        if let Sym::Id(name) = token.sym {
            let order = if name.eq_ignore_ascii_case("asc") {
                Order::Asc
            } else if name.eq_ignore_ascii_case("desc") {
                Order::Desc
            } else {
                return Err(ParserError::UnexpectedToken(
                    token.line,
                    token.col,
                    name.to_owned(),
                ));
            };

            return Ok(OrderBy { expr, order });
        }

        Err(ParserError::UnexpectedToken(
            token.line,
            token.col,
            token.sym.to_string(),
        ))
    }

    fn parse_limit(&mut self) -> ParseResult<Limit> {
        let token = self.shift();
        let limit = expect_keyword(token, "top")
            .map(|_| "top")
            .or_else(|_| expect_keyword(token, "skip").map(|_| "skip"))
            .map_err(|_| {
                ParserError::UnexpectedToken(token.line, token.col, token.sym.to_string())
            })?;

        let token = self.shift();
        if let Sym::Number(value) = token.sym
            && value.fract() == 0.0
        {
            return match limit {
                "top" => Ok(Limit::Top(value as u64)),
                "skip" => Ok(Limit::Skip(value as u64)),
                _ => unreachable!(),
            };
        }

        Err(ParserError::UnexpectedToken(
            token.line,
            token.col,
            token.sym.to_string(),
        ))
    }

    fn parse_expr(&mut self) -> ParseResult<Expr> {
        let token = self.peek();

        match token.sym {
            Sym::Eof => Err(ParserError::UnexpectedEof),

            Sym::Id(_)
            | Sym::String(_)
            | Sym::Number(_)
            | Sym::Symbol(Symbol::OpenParen | Symbol::OpenBracket | Symbol::OpenBrace)
            | Sym::Operator(Operator::Add | Operator::Sub | Operator::Not) => self.parse_binary(0),

            _ => Err(ParserError::UnexpectedToken(
                token.line,
                token.col,
                token.sym.to_string(),
            )),
        }
    }

    fn parse_primary(&mut self) -> ParseResult<Expr> {
        let token = self.shift();

        let value = match token.sym {
            Sym::Id(name) => {
                if name.eq_ignore_ascii_case("true") {
                    Value::Bool(true)
                } else if name.eq_ignore_ascii_case("false") {
                    Value::Bool(false)
                } else if matches!(self.peek().sym, Sym::Symbol(Symbol::OpenParen)) {
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

                    Value::App(App {
                        func: name.to_owned(),
                        args,
                    })
                } else if matches!(self.peek().sym, Sym::Symbol(Symbol::Dot)) {
                    self.shift();
                    let mut access = Access {
                        target: Box::new(Expr {
                            attrs: Attrs::new(token.into(), self.scope),
                            value: Value::Id(name.to_owned()),
                        }),
                        field: self.parse_ident()?,
                    };

                    while matches!(self.peek().sym, Sym::Symbol(Symbol::Dot)) {
                        self.shift();
                        access = Access {
                            target: Box::new(Expr {
                                attrs: access.target.attrs,
                                value: Value::Access(access),
                            }),
                            field: self.parse_ident()?,
                        };
                    }

                    Value::Access(access)
                } else {
                    Value::Id(name.to_owned())
                }
            }

            Sym::String(s) => Value::String(s.to_owned()),
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

            Sym::Symbol(Symbol::OpenBrace) => {
                let mut fields = vec![];

                if !matches!(self.peek().sym, Sym::Symbol(Symbol::CloseBrace)) {
                    let name = self.parse_ident()?;
                    expect_symbol(self.shift(), Symbol::Colon)?;
                    let value = self.parse_expr()?;

                    fields.push(Field { name, value });

                    while matches!(self.peek().sym, Sym::Symbol(Symbol::Comma)) {
                        self.shift();

                        let name = self.parse_ident()?;
                        expect_symbol(self.shift(), Symbol::Colon)?;
                        let value = self.parse_expr()?;

                        fields.push(Field { name, value });
                    }
                }

                expect_symbol(self.shift(), Symbol::CloseBrace)?;

                Value::Record(fields)
            }

            Sym::Operator(op) if matches!(op, Operator::Add | Operator::Sub | Operator::Not) => {
                Value::Unary(Unary {
                    operator: op,
                    expr: Box::new(self.parse_expr()?),
                })
            }

            _ => {
                return Err(ParserError::UnexpectedToken(
                    token.line,
                    token.col,
                    token.sym.to_string(),
                ));
            }
        };

        Ok(Expr {
            attrs: Attrs::new(token.into(), self.scope),
            value,
        })
    }

    fn parse_binary(&mut self, min_bind: u64) -> ParseResult<Expr> {
        let mut lhs = self.parse_primary()?;

        loop {
            let token = self.peek();
            let operator = if let Sym::Operator(op) = token.sym {
                op
            } else {
                break;
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

    fn parse_query(&mut self) -> ParseResult<Query<Raw>> {
        self.scope += 1;
        let scope = self.scope;

        let mut sources = vec![];
        let pos = self.peek().into();

        while let Sym::Id(name) = self.peek().sym
            && name.eq_ignore_ascii_case("from")
        {
            sources.push(self.parse_source()?);
        }

        let predicate = if let Sym::Id(name) = self.peek().sym
            && name.eq_ignore_ascii_case("where")
        {
            Some(self.parse_where_clause()?)
        } else {
            None
        };

        let group_by = if let Sym::Id(name) = self.peek().sym
            && name.eq_ignore_ascii_case("group")
        {
            Some(self.parse_group_by()?)
        } else {
            None
        };

        let order_by = if let Sym::Id(name) = self.peek().sym
            && name.eq_ignore_ascii_case("order")
        {
            Some(self.parse_order_by()?)
        } else {
            None
        };

        let limit = if let Sym::Id(name) = self.peek().sym
            && (name.eq_ignore_ascii_case("skip") || name.eq_ignore_ascii_case("top"))
        {
            Some(self.parse_limit()?)
        } else {
            None
        };

        expect_keyword(self.shift(), "project")?;
        expect_keyword(self.shift(), "into")?;

        let distinct = if let Sym::Id(name) = self.peek().sym
            && name.eq_ignore_ascii_case("distinct")
        {
            self.shift();
            true
        } else {
            false
        };

        let projection = self.parse_expr()?;

        self.scope -= 1;

        Ok(Query {
            attrs: Attrs::new(pos, scope),
            sources,
            predicate,
            group_by,
            order_by,
            limit,
            projection,
            distinct,
            _marker: PhantomData,
        })
    }
}

fn expect_keyword(token: Token, keyword: &'static str) -> ParseResult<()> {
    if let Sym::Id(id) = token.sym
        && id.eq_ignore_ascii_case(keyword)
    {
        return Ok(());
    }

    Err(ParserError::ExpectedKeyword(
        token.line,
        token.col,
        keyword,
        token.sym.to_string(),
    ))
}

fn expect_symbol(token: Token, expect: Symbol) -> ParseResult<()> {
    if let Sym::Symbol(sym) = token.sym
        && sym == expect
    {
        return Ok(());
    }

    Err(ParserError::ExpectedSymbol(
        token.line,
        token.col,
        expect,
        token.sym.to_string(),
    ))
}

// Pratt parser operator binding power (precedence)
fn binding_pow(op: Operator) -> (u64, u64) {
    match op {
        Operator::Add | Operator::Sub => (20, 21),
        Operator::Mul | Operator::Div => (30, 31),
        Operator::Eq
        | Operator::Neq
        | Operator::Gt
        | Operator::Lt
        | Operator::Gte
        | Operator::Lte => (10, 11),
        Operator::And | Operator::Or | Operator::Xor | Operator::Not => (1, 2),
    }
}

/// Parse a sequence of tokens into a Query AST.
///
/// This function performs syntactic analysis on the token stream, constructing
/// an abstract syntax tree that represents the structure of the EventQL query.
///
/// # Grammar
///
/// The parser recognizes the following EventQL grammar:
///
/// ```text
/// Query     := FROM+ WHERE? GROUP_BY? ORDER_BY? LIMIT? PROJECT
/// FROM      := "FROM" Id "IN" SourceKind
/// SourceKind := Id | String | "(" Query ")"
/// WHERE     := "WHERE" Expr
/// GROUP_BY  := "GROUP" "BY" Expr
/// ORDER_BY  := "ORDER" "BY" Expr ("ASC" | "DESC")
/// LIMIT     := ("TOP" | "SKIP") Number
/// PROJECT   := "PROJECT" "INTO" Expr
/// Expr      := Binary | Unary | Primary
/// Primary   := Number | String | Bool | Id | Array | Record | Access | App | "(" Expr ")"
/// ```
///
/// # Expression Precedence
///
/// The parser uses a Pratt parser for expressions with the following precedence
/// (from highest to lowest):
///
/// 1. Unary operators (`+`, `-`, `NOT`)
/// 2. Multiplicative (`*`, `/`)
/// 3. Additive (`+`, `-`)
/// 4. Comparison (`<`, `<=`, `>`, `>=`, `==`, `!=`)
/// 5. Logical (`AND`, `OR`, `XOR`)
pub fn parse<'a>(input: &'a [Token<'a>]) -> ParseResult<Query<Raw>> {
    let mut parser = Parser::new(input);

    parser.parse_query()
}
