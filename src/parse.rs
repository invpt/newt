use std::{error::Error, fmt, num::NonZeroUsize, ops::Range};

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
    pub span: Range<usize>,
    pub ty: Type<'s>,
    pub kind: ExprKind<'s>,
}

#[derive(Debug)]
pub enum Type<'s> {
    AnyOf(&'s [Type<'s>]),
    Any,
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
    // TODO: consider: should EqChecks also be moved outside Bin?
    //       or should EqAssert be moved inside Bin?
    //       or is current state of affairs okay?
    EqAssert(Box<Expr<'s>>, Box<Expr<'s>>),
    Wildcard(Wildcard, Symbol<'s>, Option<Box<Expr<'s>>>),
    Bin(BinOp, Box<Expr<'s>>, Box<Expr<'s>>),
    Apply(Box<Expr<'s>>, Box<Expr<'s>>),
    Name(Symbol<'s>),
    Literal(Literal),
}

#[derive(Debug)]
pub enum BinOp {
    Or,

    And,

    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,

    BitOr,

    BitXor,

    BitAnd,

    Shl,
    Shr,

    Add,
    Sub,

    Mul,
    Div,
    Rem,
}

#[derive(Debug)]
pub enum Literal {
    Integer(u64),
    String(String),
}

#[derive(Debug)]
pub enum Wildcard {
    Val,
    Var,
}

#[derive(Debug)]
pub struct Def<'s> {
    pub span: Range<usize>,
    pub publish: bool,
    pub name: Symbol<'s>,
    pub value: Expr<'s>,
}

#[derive(Debug)]
pub struct Symbol<'s> {
    pub text: &'s str,
    pub id: Option<SymbolId>,
}

impl<'s> Symbol<'s> {
    pub fn unknown(text: &'s str) -> Symbol<'s> {
        Symbol { text, id: None }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SymbolId(pub NonZeroUsize);

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
                name: Symbol::unknown(name),
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
            ty: Type::Any,
            kind: ExprKind::Dict(defs.into_boxed_slice()),
        })
    }

    fn tuple(&mut self, end_pred: impl Fn(&Token<'s>) -> Option<()>) -> Result<Expr<'s>> {
        let start = self.lex.offset();
        let mut end = self.lex.offset();

        if self.lex.has_peek(&end_pred)? {
            return Ok(Expr {
                span: start..end,
                ty: Type::Any,
                kind: ExprKind::Tuple(Box::new([])),
            });
        }

        let first = self.seq(either(&end_pred, pred!(TokenKind::Comma)))?;
        if self.lex.has_peek(&end_pred)? {
            return Ok(Expr {
                span: start..end,
                ty: Type::Any,
                kind: first.kind,
            });
        }

        let mut exprs = Vec::from([first]);
        loop {
            if self.lex.has_peek(&end_pred)? {
                break;
            }

            self.lex.require(pred!(TokenKind::Comma))?;

            if self.lex.has_peek(&end_pred)? {
                break;
            }

            let expr = self.seq(either(&end_pred, pred!(TokenKind::Comma)))?;
            end = expr.span.end;
            exprs.push(expr);
        }

        Ok(Expr {
            span: start..end,
            ty: Type::Any,
            kind: ExprKind::Tuple(exprs.into_boxed_slice()),
        })
    }

    fn seq(&mut self, end_pred: impl Fn(&Token<'s>) -> Option<()>) -> Result<Expr<'s>> {
        let start = self.lex.offset();
        let mut end = self.lex.offset();

        if self.lex.has_peek(&end_pred)? {
            return Ok(Expr {
                span: start..end,
                ty: Type::Any,
                kind: ExprKind::Tuple(Box::new([])),
            });
        }

        let mut exprs = Vec::new();
        let term = loop {
            let (expr, expr_term) = self.expr_termination()?;
            if exprs.is_empty() && self.lex.has_peek(&end_pred)? {
                return Ok(Expr {
                    span: start..end,
                    ty: Type::Any,
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
            ty: Type::Any,
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
        let (lhs, term) = self.big_termination()?;
        if term == Terminated || self.lex.eat(pred!(TokenKind::Eq))?.is_none() {
            Ok((lhs, term))
        } else {
            let (rhs, term) = self.big_termination()?;
            Ok((
                Expr {
                    span: lhs.span.start..rhs.span.end,
                    ty: Type::Any,
                    kind: ExprKind::EqAssert(Box::new(lhs), Box::new(rhs)),
                },
                term,
            ))
        }
    }

    fn big_termination(&mut self) -> Result<(Expr<'s>, Termination)> {
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
                        ty: Type::Any,
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
                        ty: Type::Any,
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
                        ty: Type::Any,
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
                ty: Type::Any,
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
                    ty: Type::Any,
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
        self.or()
    }

    fn or(&mut self) -> Result<Expr<'s>> {
        self.left_associative(Self::and, pred!(TokenKind::Or => BinOp::Or))
    }

    fn and(&mut self) -> Result<Expr<'s>> {
        self.left_associative(Self::comparison, pred!(TokenKind::And => BinOp::And))
    }

    fn comparison(&mut self) -> Result<Expr<'s>> {
        self.unassociative(
            Self::wildcards,
            pred!(
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::Neq,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::LtEq => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::GtEq => BinOp::Ge,
            ),
        )
    }

    fn wildcards(&mut self) -> Result<Expr<'s>> {
        if let Some((wildcard, start)) = self.lex.eat(pred!(@t
            TokenKind::Val => (Wildcard::Val, t.span.start),
            TokenKind::Var => (Wildcard::Var, t.span.start),
        ))? {
            let (name, name_span) = self
                .lex
                .require(pred!(@t TokenKind::Name(name) => (name, t.span.clone())))?;

            let ty = self.maybe_ty()?;

            Ok(Expr {
                span: start..ty.as_ref().map(|ty| ty.span.end).unwrap_or(name_span.end),
                ty: Type::Any,
                kind: ExprKind::Wildcard(wildcard, Symbol::unknown(name), ty.map(Box::new)),
            })
        } else {
            self.bit_or()
        }
    }

    fn bit_or(&mut self) -> Result<Expr<'s>> {
        self.left_associative(Self::bit_xor, pred!(TokenKind::Pipe => BinOp::BitOr))
    }

    fn bit_xor(&mut self) -> Result<Expr<'s>> {
        self.left_associative(Self::bit_and, pred!(TokenKind::Tilde => BinOp::BitXor))
    }

    fn bit_and(&mut self) -> Result<Expr<'s>> {
        self.left_associative(Self::shift, pred!(TokenKind::Amp => BinOp::BitAnd))
    }

    fn shift(&mut self) -> Result<Expr<'s>> {
        self.unassociative(
            Self::arith,
            pred!(
                TokenKind::LtLt => BinOp::Shl,
                TokenKind::GtGt => BinOp::Shr,
            ),
        )
    }

    fn arith(&mut self) -> Result<Expr<'s>> {
        self.left_associative(
            Self::term,
            pred!(
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
            ),
        )
    }

    fn term(&mut self) -> Result<Expr<'s>> {
        self.left_associative(
            Self::jux,
            pred!(
                TokenKind::Star => BinOp::Mul,
                TokenKind::ForwardSlash => BinOp::Div,
                TokenKind::Percent => BinOp::Rem,
            ),
        )
    }

    fn ty(&mut self) -> Result<Expr<'s>> {
        if let Some(ty) = self.maybe_ty()? {
            Ok(ty)
        } else {
            todo!("{:?}", self.lex.next()?)
        }
    }

    fn maybe_ty(&mut self) -> Result<Option<Expr<'s>>> {
        // TODO: this will be more complicated!
        self.maybe_jux()
    }

    fn jux(&mut self) -> Result<Expr<'s>> {
        if let Some(jux) = self.maybe_jux()? {
            Ok(jux)
        } else {
            todo!("{:?}", self.lex.next()?)
        }
    }

    fn maybe_jux(&mut self) -> Result<Option<Expr<'s>>> {
        let Some(mut expr) = self.maybe_atom()? else {
            return Ok(None);
        };

        while let Some(suff) = self.maybe_atom()? {
            expr = Expr {
                span: expr.span.start..suff.span.end,
                ty: Type::Any,
                kind: ExprKind::Apply(Box::new(expr), Box::new(suff)),
            }
        }

        Ok(Some(expr))
    }

    fn maybe_atom(&mut self) -> Result<Option<Expr<'s>>> {
        if let Some(start) = self
            .lex
            .eat(pred!(@t TokenKind::OpenParen => t.span.start))?
        {
            let body = self.tuple(pred!(TokenKind::CloseParen))?;
            let end = self
                .lex
                .require(pred!(@t TokenKind::CloseParen => t.span.end))?;

            Ok(Some(Expr {
                span: start..end,
                ty: Type::Any,
                kind: body.kind,
            }))
        } else if let Some((n, span)) = self
            .lex
            .eat(pred!(@t TokenKind::Number(n) => (n, t.span.clone())))?
        {
            Ok(Some(Expr {
                span,
                ty: Type::Any,
                kind: ExprKind::Literal(Literal::Integer(n)),
            }))
        } else if let Some((name, span)) = self
            .lex
            .eat(pred!(@t TokenKind::Name(name) => (name, t.span.clone())))?
        {
            Ok(Some(Expr {
                span,
                ty: Type::Any,
                kind: ExprKind::Name(Symbol::unknown(name)),
            }))
        } else {
            Ok(None)
        }
    }

    fn left_associative(
        &mut self,
        below: impl Fn(&mut Self) -> Result<Expr<'s>>,
        pred: impl Fn(&Token<'s>) -> Option<BinOp>,
    ) -> Result<Expr<'s>> {
        let mut lhs = below(self)?;

        while let Some(op) = self.lex.eat(&pred)? {
            let rhs = below(self)?;

            lhs = Expr {
                span: lhs.span.start..rhs.span.end,
                ty: Type::Any,
                kind: ExprKind::Bin(op, Box::new(lhs), Box::new(rhs)),
            }
        }

        Ok(lhs)
    }

    fn unassociative(
        &mut self,
        below: impl Fn(&mut Self) -> Result<Expr<'s>>,
        pred: impl Fn(&Token<'s>) -> Option<BinOp>,
    ) -> Result<Expr<'s>> {
        let lhs = below(self)?;

        if let Some(op) = self.lex.eat(&pred)? {
            let rhs = below(self)?;

            Ok(Expr {
                span: lhs.span.start..rhs.span.end,
                ty: Type::Any,
                kind: ExprKind::Bin(op, Box::new(lhs), Box::new(rhs)),
            })
        } else {
            Ok(lhs)
        }
    }
}
