use std::ops::Range;

use logos::Logos;

type Result<T> = std::result::Result<T, LexerError>;

#[derive(Clone, Debug, PartialEq)]
pub struct LexerError {
    pub kind: LexerErrorKind,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LexerErrorKind {
    InvalidData,
    UnexpectedEof,
    UnexpectedToken,
}

#[derive(Debug, Clone)]
pub struct Token<'s> {
    pub kind: TokenKind<'s>,
    pub span: Range<usize>,
}

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
pub enum TokenKind<'s> {
    #[token("pub")]
    Pub,
    #[token("def")]
    Def,
    #[token("type")]
    Type,
    #[token("trait")]
    Trait,
    #[token("effect")]
    Effect,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("=")]
    Eq,
    #[token("val")]
    Val,
    #[token("var")]
    Var,
    #[token("or")]
    Or,
    #[token("and")]
    And,
    #[token("==")]
    EqEq,
    #[token("!=")]
    BangEq,
    #[token("<")]
    Lt,
    #[token("<=")]
    LtEq,
    #[token(">")]
    Gt,
    #[token(">=")]
    GtEq,
    #[token("|")]
    Pipe,
    #[token("~")]
    Tilde,
    #[token("&")]
    Amp,
    #[token("<<")]
    LtLt,
    #[token(">>")]
    GtGt,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    ForwardSlash,
    #[token("%")]
    Percent,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("(")]
    OpenParen,
    #[token(")")]
    CloseParen,
    #[token("{")]
    OpenCurly,
    #[token("}")]
    CloseCurly,
    #[regex("[a-zA-Z_][a-zA-Z_0-9]*", |lex| lex.slice())]
    Name(&'s str),
    #[regex("[0-9]+", |lex| lex.slice().parse().ok())]
    Number(u64),
}

pub struct Lexer<'s, 'd> {
    diag: Diagnostics<'d>,
    lex: logos::Lexer<'s, TokenKind<'s>>,
    offset: usize,
    peek: Option<Token<'s>>,
}

impl<'s, 'd> Lexer<'s, 'd> {
    pub fn new(diag: Diagnostics<'d>, src: &'s str) -> Lexer<'s, 'd> {
        Lexer {
            diag,
            lex: logos::Lexer::new(src),
            offset: 0,
            peek: None,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn next(&mut self) -> Result<Option<Token<'s>>> {
        if let Some(peek) = self.peek.take() {
            self.offset = peek.span.end;
            return Ok(Some(peek));
        }

        match self.lex.next() {
            Some(result) => match result {
                Ok(kind) => {
                    self.offset = self.lex.span().end;
                    Ok(Some(Token {
                        kind,
                        span: self.lex.span(),
                    }))
                }
                Err(()) => Err(LexerError {
                    kind: LexerErrorKind::InvalidData,
                    span: self.lex.span(),
                }),
            },
            None => Ok(None),
        }
    }

    pub fn peek(&mut self) -> Result<Option<&Token<'s>>> {
        if self.peek.is_none() {
            let old_offset = self.offset;
            self.peek = self.next()?;
            self.offset = old_offset;
        }

        Ok(self.peek.as_ref())
    }

    pub fn require<T>(&mut self, pred: impl FnOnce(&Token<'s>) -> Option<T>) -> Result<T> {
        match self.next()? {
            Some(token) => match pred(&token) {
                Some(value) => Ok(value),
                None => Err(LexerError {
                    kind: LexerErrorKind::UnexpectedToken,
                    span: token.span,
                }),
            },
            None => Err(LexerError {
                kind: LexerErrorKind::UnexpectedEof,
                span: self.lex.span(),
            }),
        }
    }

    pub fn eat<T>(&mut self, pred: impl FnOnce(&Token<'s>) -> Option<T>) -> Result<Option<T>> {
        match self.peek()? {
            Some(token) => match pred(&token) {
                Some(value) => {
                    self.next()?;
                    Ok(Some(value))
                }
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    pub fn has_peek<T>(&mut self, pred: impl FnOnce(&Token<'s>) -> Option<T>) -> Result<bool> {
        match self.peek()? {
            Some(token) => match pred(&token) {
                Some(_) => Ok(true),
                None => Ok(false),
            },
            None => Ok(false),
        }
    }
}

#[macro_export]
macro_rules! pred {
    ($(@$t:ident)? $($p:pat $(=> $e:expr)?),*$(,)?) => {
        |token| {
            $(let $t = token;)?
            match token.kind {
                $($p => Some(($($e)?)),)*
                #[allow(unreachable_patterns)]
                _ => None,
            }
        }
    };
}

pub use pred;

use crate::diagnostics::Diagnostics;

pub fn either<'s, T>(
    a: impl Fn(&Token<'s>) -> Option<T>,
    b: impl Fn(&Token<'s>) -> Option<T>,
) -> impl Fn(&Token<'s>) -> Option<T> {
    move |token| a(token).or_else(|| b(token))
}
