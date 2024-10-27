use std::{default, ops::Range};

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
    Fast,
    #[token(".")]
    Period,
    #[regex("[a-zA-Z_][a-zA-Z_0-9]*", |lex| lex.slice())]
    Text(&'s str),
    #[regex("[0-9]+", |lex| lex.slice().parse().ok())]
    Number(u64),
}

pub struct Lexer<'s> {
    lex: logos::Lexer<'s, TokenKind<'s>>,
    peek: Option<Token<'s>>,
}

impl<'s> Lexer<'s> {
    pub fn new(src: &'s str) -> Lexer<'s> {
        Lexer {
            lex: logos::Lexer::new(src),
            peek: None,
        }
    }

    pub fn next(&mut self) -> Result<Option<Token<'s>>> {
        if let Some(peek) = self.peek.take() {
            return Ok(Some(peek));
        }

        match self.lex.next() {
            Some(result) => match result {
                Ok(kind) => Ok(Some(Token {
                    kind,
                    span: self.lex.span(),
                })),
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
            self.peek = self.next()?;
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
        match self.next()? {
            Some(token) => match pred(&token) {
                Some(value) => Ok(Some(value)),
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
                _ => None,
            }
        }
    };
}

pub use pred;
