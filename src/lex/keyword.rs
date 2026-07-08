//! Reserved-word classification for the lexer.
//!
//! The scanner asks `keyword_from_str` whenever an identifier is consumed; a
//! `Some` answer means the lexeme is a reserved word (and should be lowered
//! into the parser's flat `TokenKind`), while `None` means the lexeme is a
//! regular identifier and stays as `Identifier(String)`.  Centralising the
//! keyword table here keeps the scanner's main loop free of branching on
//! lexeme text and lets later chapters extend the reserved-word set without
//! editing the scanner.
//!
//! The table mirrors `convert_identifier` in `nqcc2/lib/lex.ml`.  The Rust
//! port additionally reserves `signed` because the OCaml reference defines
//! `KWSigned`; this keeps the parity deliberate and explicit even though the
//! book does not introduce signed integer types.

use crate::lex::token::Keyword;

/// Classify an identifier-shaped lexeme as a reserved word, if any.
///
/// Returns `Some(Keyword)` when the lexeme is one of the 22 book keywords
/// (plus `signed`, which the OCaml reference includes in its keyword table).
/// Anything else falls through to `None` so the caller can wrap it as an
/// `Identifier` token.
pub(crate) fn keyword_from_str(lexeme: &str) -> Option<Keyword> {
    Some(match lexeme {
        "int" => Keyword::Int,
        "long" => Keyword::Long,
        "unsigned" => Keyword::Unsigned,
        "signed" => Keyword::Signed,
        "double" => Keyword::Double,
        "char" => Keyword::Char,
        "void" => Keyword::Void,
        "if" => Keyword::If,
        "else" => Keyword::Else,
        "do" => Keyword::Do,
        "while" => Keyword::While,
        "for" => Keyword::For,
        "break" => Keyword::Break,
        "continue" => Keyword::Continue,
        "return" => Keyword::Return,
        "goto" => Keyword::Goto,
        "switch" => Keyword::Switch,
        "case" => Keyword::Case,
        "default" => Keyword::Default,
        "sizeof" => Keyword::Sizeof,
        "struct" => Keyword::Struct,
        "union" => Keyword::Union,
        "static" => Keyword::Static,
        "extern" => Keyword::Extern,
        _ => return None,
    })
}
