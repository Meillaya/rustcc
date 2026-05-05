//! Reserved-word classification for the lexer.
//!
//! Keeping keyword recognition in one table avoids scattering language growth
//! across the scanner.  The function returns `Option<TokenKind>` so identifiers
//! remain the scanner's fallback case without allocating an intermediate enum.

use crate::lex::token::TokenKind;

pub(crate) fn keyword_kind(lexeme: &str) -> Option<TokenKind> {
    Some(match lexeme {
        "int" => TokenKind::Int,
        "void" => TokenKind::Void,
        "return" => TokenKind::Return,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "goto" => TokenKind::Goto,
        "long" => TokenKind::Long,
        "unsigned" => TokenKind::Unsigned,
        "signed" => TokenKind::Signed,
        "char" => TokenKind::Char,
        "double" => TokenKind::Double,
        "struct" => TokenKind::Struct,
        "union" => TokenKind::Union,
        "while" => TokenKind::While,
        "do" => TokenKind::Do,
        "for" => TokenKind::For,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "switch" => TokenKind::Switch,
        "case" => TokenKind::Case,
        "default" => TokenKind::Default,
        _ => return None,
    })
}
