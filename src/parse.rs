use std::{error::Error, fmt, ops::Range};

use crate::{
    lex::{either, Lexer, LexerError, Token, TokenKind},
    pred,
};

type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    Lex(LexerError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl Error for ParseError {}

impl From<LexerError> for ParseError {
    fn from(value: LexerError) -> Self {
        ParseError::Lex(value)
    }
}

#[derive(Debug)]
pub struct Expr<'s> {
    pub kind: ExprKind<'s>,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub enum ExprKind<'s> {
    Dict(Box<[Def<'s>]>),
    Lambda(
        Option<Box<Expr<'s>>>,
        Option<Box<Expr<'s>>>,
        Option<Box<Expr<'s>>>,
    ),
    If(Box<Expr<'s>>, Box<Expr<'s>>, Option<Box<Expr<'s>>>),
    Tuple(Box<[Expr<'s>]>),
    Seq(Box<[Expr<'s>]>, Termination),
    Name(&'s str),
    Literal(Literal),
}

#[derive(Debug)]
pub enum Literal {
    Integer(u64),
    String(String),
}

#[derive(Debug)]
pub struct Def<'s> {
    pub publish: bool,
    pub name: &'s str,
    pub value: Expr<'s>,
    pub span: Range<usize>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Termination {
    Unterminated,
    Terminated,
}

use Termination::*;

pub struct Parser<'s> {
    lex: Lexer<'s>,
}

impl<'s> Parser<'s> {
    pub fn parse(lex: Lexer<'s>) -> Result<Expr<'s>> {
        Parser { lex }.dict(pred!())
    }

    fn dict(&mut self, end_pred: impl Fn(&Token<'s>) -> Option<()>) -> Result<Expr<'s>> {
        let mut defs = Vec::new();

        while !self.lex.has_peek(&end_pred)? && self.lex.peek()?.is_some() {
            let (start, publish) = self.lex.require(pred!(@t
                TokenKind::Def => (t.span.start, false),
                TokenKind::Pub => (t.span.start, true),
            ))?;

            let name = self.lex.require(pred!(TokenKind::Name(name) => name))?;

            let value = self.expr(Terminated)?;

            match &value.kind {
                // these constructs are allowed as def values.
                ExprKind::Dict(..) | ExprKind::Lambda(..) => (),
                // these constructs are not.
                _ => panic!(""), // TODO: error not panic
            }

            defs.push(Def {
                publish,
                name,
                span: start..value.span.end,
                value,
            });
        }

        let start = defs
            .iter()
            .map(|def| def.span.start)
            .min()
            .unwrap_or(self.lex.offset());
        let end = defs
            .iter()
            .map(|def| def.span.end)
            .max()
            .unwrap_or(self.lex.offset());

        Ok(Expr {
            span: start..end,
            kind: ExprKind::Dict(defs.into_boxed_slice()),
        })
    }

    fn tuple(&mut self, end_pred: impl Fn(&Token<'s>) -> Option<()>) -> Result<Expr<'s>> {
        let start = self.lex.offset();
        let mut end = self.lex.offset();

        if self.lex.has_peek(&end_pred)? {
            return Ok(Expr {
                span: start..end,
                kind: ExprKind::Tuple(Box::new([])),
            });
        }

        let first = self.seq(either(&end_pred, pred!(TokenKind::Comma)))?;
        if self.lex.has_peek(&end_pred)? {
            return Ok(Expr {
                span: start..end,
                kind: first.kind,
            });
        }

        let mut exprs = Vec::from([first]);
        loop {
            let expr = self.seq(either(&end_pred, pred!(TokenKind::Comma)))?;
            end = expr.span.end;
            exprs.push(expr);

            if self.lex.has_peek(&end_pred)? {
                break;
            }

            self.lex.require(pred!(TokenKind::Comma))?;
        }

        Ok(Expr {
            span: start..end,
            kind: ExprKind::Tuple(exprs.into_boxed_slice()),
        })
    }

    fn seq(&mut self, end_pred: impl Fn(&Token<'s>) -> Option<()>) -> Result<Expr<'s>> {
        let start = self.lex.offset();
        let mut end = self.lex.offset();

        if self.lex.has_peek(&end_pred)? {
            return Ok(Expr {
                span: start..end,
                kind: ExprKind::Tuple(Box::new([])),
            });
        }

        let mut exprs = Vec::new();
        let term = loop {
            let (expr, expr_term) = self.expr_termination()?;
            if exprs.is_empty() && self.lex.has_peek(&end_pred)? {
                return Ok(Expr {
                    span: start..end,
                    kind: expr.kind,
                });
            }
            end = expr.span.end;
            exprs.push(expr);

            let explicit_term = self.lex.eat(pred!(TokenKind::Semicolon))?.is_some();

            if self.lex.has_peek(&end_pred)? || self.lex.peek()?.is_none() {
                if explicit_term {
                    break Terminated;
                } else {
                    break Unterminated;
                }
            } else if expr_term == Unterminated && !explicit_term {
                // BAD: the seq is not over, the expr is not terminated, and there was no terminating semicolon.
                //      the user has probably forgotten a semicolon?
                panic!("Expected semicolon, found something else") // TODO: error, not panic
            }
        };

        Ok(Expr {
            span: start..end,
            kind: ExprKind::Seq(exprs.into_boxed_slice(), term),
        })
    }

    fn expr(&mut self, desired_term: Termination) -> Result<Expr<'s>> {
        let (expr, actual_term) = self.expr_termination()?;
        if actual_term < desired_term {
            self.lex.require(pred!(TokenKind::Semicolon))?;
        }

        Ok(expr)
    }

    fn expr_termination(&mut self) -> Result<(Expr<'s>, Termination)> {
        if self.lex.eat(pred!(TokenKind::Type))?.is_some() {
            todo!("Type literals")
        } else if self.lex.eat(pred!(TokenKind::Trait))?.is_some() {
            todo!("Trait literals")
        } else if self.lex.eat(pred!(TokenKind::Effect))?.is_some() {
            todo!("Effect literals")
        } else if self.lex.eat(pred!(TokenKind::If))?.is_some() {
            self.conditional()
        } else {
            let output = self.output()?;
            let thunk = self.thunk()?;

            let expr = if let (None, None) = (&output, &thunk) {
                Some(self.below()?)
            } else {
                None
            };
            let output = if let None = &expr {
                output
            } else {
                self.output()?
            };
            let thunk = if let None = &expr {
                thunk
            } else {
                self.thunk()?
            };

            Ok(match (output, thunk) {
                (Some(output), Some((thunk, thunk_term))) => (
                    Expr {
                        span: expr
                            .as_ref()
                            .map(|expr| expr.span.start)
                            .unwrap_or(output.span.start)
                            ..thunk.span.end,
                        kind: ExprKind::Lambda(
                            expr.map(Box::new),
                            Some(Box::new(output)),
                            Some(Box::new(thunk)),
                        ),
                    },
                    thunk_term,
                ),
                (Some(output), None) => (
                    Expr {
                        span: expr
                            .as_ref()
                            .map(|expr| expr.span.start)
                            .unwrap_or(output.span.start)
                            ..output.span.end,
                        kind: ExprKind::Lambda(expr.map(Box::new), Some(Box::new(output)), None),
                    },
                    Unterminated,
                ),
                (None, Some((thunk, thunk_term))) => (
                    Expr {
                        span: expr
                            .as_ref()
                            .map(|expr| expr.span.start)
                            .unwrap_or(thunk.span.start)
                            ..thunk.span.end,
                        kind: ExprKind::Lambda(expr.map(Box::new), None, Some(Box::new(thunk))),
                    },
                    thunk_term,
                ),
                (None, None) => (
                    expr.expect("Wanted expr, found nothing"), // TODO: error, not panic
                    Unterminated,
                ),
            })
        }
    }

    fn conditional(&mut self) -> Result<(Expr<'s>, Termination)> {
        let cond = self.below()?;
        let (body, body_term) = self
            .thunk()?
            .expect("If statements must have a body using => or {}"); // TODO: error not panic
        let (alt, alt_term) = if self.lex.eat(pred!(TokenKind::Else))?.is_some() {
            let (alt, alt_term) = if self.lex.eat(pred!(TokenKind::If))?.is_some() {
                self.conditional()?
            } else {
                self.thunk()?
                    .expect("Else statements must have a body using => or {}") // TODO: error not panic
            };
            (Some(alt), Some(alt_term))
        } else {
            (None, None)
        };

        Ok((
            Expr {
                span: cond.span.start
                    ..alt
                        .as_ref()
                        .map(|alt| alt.span.end)
                        .unwrap_or(body.span.end),
                kind: ExprKind::If(Box::new(cond), Box::new(body), alt.map(Box::new)),
            },
            alt_term.unwrap_or(body_term),
        ))
    }

    fn thunk(&mut self) -> Result<Option<(Expr<'s>, Termination)>> {
        if self.lex.eat(pred!(TokenKind::FatArrow))?.is_some() {
            Ok(Some(self.expr_termination()?))
        } else if let Some(start) = self
            .lex
            .eat(pred!(@t TokenKind::OpenCurly => t.span.start))?
        {
            let body = self.seq(pred!(TokenKind::CloseCurly))?;
            let end = self
                .lex
                .require(pred!(@t TokenKind::CloseCurly => t.span.end))?;

            Ok(Some((
                Expr {
                    span: start..end,
                    kind: body.kind,
                },
                Terminated,
            )))
        } else {
            Ok(None)
        }
    }

    fn output(&mut self) -> Result<Option<Expr<'s>>> {
        if self.lex.eat(pred!(TokenKind::Arrow))?.is_some() {
            Ok(Some(self.below()?))
        } else {
            Ok(None)
        }
    }

    fn below(&mut self) -> Result<Expr<'s>> {
        self.atom()
    }

    fn atom(&mut self) -> Result<Expr<'s>> {
        if let Some(start) = self
            .lex
            .eat(pred!(@t TokenKind::OpenParen => t.span.start))?
        {
            let body = self.tuple(pred!(TokenKind::CloseParen))?;
            let end = self
                .lex
                .require(pred!(@t TokenKind::CloseParen => t.span.end))?;

            Ok(Expr {
                span: start..end,
                kind: body.kind,
            })
        } else if let Some((n, span)) = self
            .lex
            .eat(pred!(@t TokenKind::Number(n) => (n, t.span.clone())))?
        {
            Ok(Expr {
                span,
                kind: ExprKind::Literal(Literal::Integer(n)),
            })
        } else if let Some((name, span)) = self
            .lex
            .eat(pred!(@t TokenKind::Name(name) => (name, t.span.clone())))?
        {
            Ok(Expr {
                span,
                kind: ExprKind::Name(name),
            })
        } else {
            todo!("{:?}", self.lex.next()?)
        }
    }
}
