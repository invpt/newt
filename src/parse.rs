use std::ops::Range;

use crate::lex::Lexer;

pub struct Expr<'s> {
    pub kind: ExprKind<'s>,
    pub span: Range<usize>,
}

pub enum ExprKind<'s> {
    Name(&'s str),
}

pub struct Parser<'s> {
    lex: Lexer<'s>,
}