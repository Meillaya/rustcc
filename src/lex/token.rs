//! Token definitions reserved for the growing C subset.

#![allow(dead_code)]

use crate::lex::keyword::Keyword;

/// Placeholder token kind surface.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Keyword(Keyword),
    Identifier,
    IntegerLiteral,
    FloatLiteral,
    CharLiteral,
    StringLiteral,
    Punctuation,
    Operator,
    EndOfFile,
}

/// Placeholder token record.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
}
