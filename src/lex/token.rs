// Mirrors nqcc2/lib/tokens.ml (1-100). Locked to book-faithful token set for
// all 20 chapters + 7 extras (signed, ++, --, +=, -=, *=, /=, %=, &=, |=, ^=
// compound-assignment, <<=, >>= shifts, dot, arrow, question/colon ternary,
// shift operators, etc.). The scanner classifies each token against the
// `Keyword` and `Punct` enums (the book-faithful internal model) and then
// lowers the result to the flat `TokenKind` variants consumed by the parser.
// Splitting the two views keeps the long-term token vocabulary declarative
// while preserving the parser's discriminant-based matching.
//
// The module-level `allow(dead_code)` is intentional: token variants beyond
// the chapter-1 subset are reserved for waves 2-20 (long, unsigned, double,
// char constants, switch/case/default, etc.). Carrying them now avoids
// recursive AST churn once each chapter lands.

//! Book-faithful token vocabulary shared by the lexer and parser.
//!
//! The lexer recognises the 22 keywords in `Keyword`, the operator/punctuation
//! set in `Punct`, and the constant forms collected into the flat `TokenKind`
//! variants consumed by the recursive-descent parser.  Adding a new chapter's
//! tokens is a matter of extending the three enums in one place; the scanner
//! and parser then pick up the new variants automatically.

#![allow(dead_code)]

/// Reserved words introduced by chapters 1-12 of the book.
///
/// Mirrors the keyword table in `nqcc2/lib/lex.ml` (`convert_identifier`).
/// `Signed` is included even though the OCaml reference uses `KWSigned`; the
/// Rust port names it consistently with the other unsigned/signed types so
/// future signed-arithmetic work can reuse the same enum variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    Int,
    Long,
    Unsigned,
    Signed,
    Double,
    Char,
    Void,
    If,
    Else,
    Do,
    While,
    For,
    Break,
    Continue,
    Return,
    Goto,
    Switch,
    Case,
    Default,
    Sizeof,
    Struct,
    Union,
    Static,
    Extern,
}

/// Book-faithful punctuation / operator vocabulary.
///
/// Mirrors the punctuation cases in `nqcc2/lib/tokens.ml`.  Multi-character
/// forms are stored as a single variant so the scanner only needs to peek one
/// or two characters ahead and the parser can pattern-match exhaustively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Punct {
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Comma,
    Semicolon,
    Tilde,
    Bang,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    AmpAmp,
    Pipe,
    PipePipe,
    Caret,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    LtLt,
    GtGt,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    AmpEq,
    PipeEq,
    CaretEq,
    LtLtEq,
    GtGtEq,
    PlusPlus,
    MinusMinus,
    Question,
    Colon,
    Arrow,
    Dot,
}

/// A lexed token paired with its original source spelling.
///
/// The struct shape is preserved from the chapter-1 scanner so the existing
/// recursive-descent parser (which matches on `kind` and reads `lexeme` for
/// identifiers and error messages) keeps compiling without churn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) lexeme: String,
}

/// Chapter 12: integer constant suffix that decides between
/// `unsigned int` and `unsigned long` for an unsigned literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UIntKind {
    UInt,
    ULong,
}

/// Flat token vocabulary consumed by the parser.
///
/// Each variant is the lowered form of the matching `Keyword` / `Punct` /
/// constant shape that the scanner emits.  `Eof` is included as a sentinel
/// variant rather than relying on `Option<Token>`; this keeps the parser's
/// `peek()`/`expect_exact()` pattern uniform across every token kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenKind {
    // Reserved words -- one variant per `Keyword` case so the parser can
    // pattern-match without inspecting a sub-payload.
    Int,
    Void,
    Return,
    If,
    Else,
    Goto,
    Long,
    Unsigned,
    Signed,
    Char,
    Double,
    Struct,
    Union,
    Static,
    Extern,
    While,
    Do,
    For,
    Break,
    Continue,
    Switch,
    Case,
    Default,

    // Identifiers and constants.
    Identifier(String),
    Constant(i64),
    /// Chapter 11: integer constant with a `L` / `l` suffix, typed as
    /// `long` in C.  Carried as an i64 so values larger than 32 bits
    /// (e.g. `4294967290L`) fit without truncation.
    LongConstant(i64),
    /// Chapter 12: integer constant with a `U` / `u` suffix (and
    /// optionally `L` / `l`), typed as `unsigned int` / `unsigned
    /// long`.
    UIntConstant(i64, UIntKind),
    CharLiteral(i32),
    StringLiteral(String),

    // Punctuation / operators -- one variant per `Punct` case.
    Minus,
    Tilde,
    Bang,
    Plus,
    Star,
    Slash,
    Percent,
    ShiftLeft,
    ShiftRight,
    Arrow,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    EqualEqual,
    NotEqual,
    Ampersand,
    Caret,
    Pipe,
    LogicalAnd,
    LogicalOr,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    PercentEqual,
    AmpersandEqual,
    CaretEqual,
    PipeEqual,
    ShiftLeftEqual,
    ShiftRightEqual,
    PlusPlus,
    MinusMinus,
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Dot,
    Question,
    Colon,
    Comma,
    Semicolon,

    /// Sentinel emitted once, at the very end of every token stream.
    Eof,
}

impl TokenKind {
    /// Map a `Keyword` to its flat `TokenKind` representation.
    ///
    /// This keeps the keyword table declarative in `keyword.rs` while giving
    /// the scanner a single conversion point.  The compiler guarantees this
    /// match is exhaustive via the `unreachable!` arm -- adding a `Keyword`
    /// variant without a corresponding `TokenKind` mapping is a compile error.
    pub(crate) fn from_keyword(keyword: Keyword) -> Self {
        match keyword {
            Keyword::Int => Self::Int,
            Keyword::Long => Self::Long,
            Keyword::Unsigned => Self::Unsigned,
            Keyword::Signed => Self::Signed,
            Keyword::Double => Self::Double,
            Keyword::Char => Self::Char,
            Keyword::Void => Self::Void,
            Keyword::If => Self::If,
            Keyword::Else => Self::Else,
            Keyword::Do => Self::Do,
            Keyword::While => Self::While,
            Keyword::For => Self::For,
            Keyword::Break => Self::Break,
            Keyword::Continue => Self::Continue,
            Keyword::Return => Self::Return,
            Keyword::Goto => Self::Goto,
            Keyword::Switch => Self::Switch,
            Keyword::Case => Self::Case,
            Keyword::Default => Self::Default,
            Keyword::Sizeof => unreachable!("sizeof is not a parser token yet"),
            Keyword::Struct => Self::Struct,
            Keyword::Union => Self::Union,
            Keyword::Static => Self::Static,
            Keyword::Extern => Self::Extern,
        }
    }

    /// Map a `Punct` to its flat `TokenKind` representation.
    pub(crate) fn from_punct(punct: Punct) -> Self {
        match punct {
            Punct::OpenParen => Self::OpenParen,
            Punct::CloseParen => Self::CloseParen,
            Punct::OpenBrace => Self::OpenBrace,
            Punct::CloseBrace => Self::CloseBrace,
            Punct::OpenBracket => Self::OpenBracket,
            Punct::CloseBracket => Self::CloseBracket,
            Punct::Comma => Self::Comma,
            Punct::Semicolon => Self::Semicolon,
            Punct::Tilde => Self::Tilde,
            Punct::Bang => Self::Bang,
            Punct::Plus => Self::Plus,
            Punct::Minus => Self::Minus,
            Punct::Star => Self::Star,
            Punct::Slash => Self::Slash,
            Punct::Percent => Self::Percent,
            Punct::Amp => Self::Ampersand,
            Punct::AmpAmp => Self::LogicalAnd,
            Punct::Pipe => Self::Pipe,
            Punct::PipePipe => Self::LogicalOr,
            Punct::Caret => Self::Caret,
            Punct::Eq => Self::Equal,
            Punct::EqEq => Self::EqualEqual,
            Punct::NotEq => Self::NotEqual,
            Punct::Lt => Self::Less,
            Punct::Gt => Self::Greater,
            Punct::LtEq => Self::LessEqual,
            Punct::GtEq => Self::GreaterEqual,
            Punct::LtLt => Self::ShiftLeft,
            Punct::GtGt => Self::ShiftRight,
            Punct::PlusEq => Self::PlusEqual,
            Punct::MinusEq => Self::MinusEqual,
            Punct::StarEq => Self::StarEqual,
            Punct::SlashEq => Self::SlashEqual,
            Punct::PercentEq => Self::PercentEqual,
            Punct::AmpEq => Self::AmpersandEqual,
            Punct::PipeEq => Self::PipeEqual,
            Punct::CaretEq => Self::CaretEqual,
            Punct::LtLtEq => Self::ShiftLeftEqual,
            Punct::GtGtEq => Self::ShiftRightEqual,
            Punct::PlusPlus => Self::PlusPlus,
            Punct::MinusMinus => Self::MinusMinus,
            Punct::Question => Self::Question,
            Punct::Colon => Self::Colon,
            Punct::Arrow => Self::Arrow,
            Punct::Dot => Self::Dot,
        }
    }

    /// Stable, kebab-free label used by `--stage lex` output.
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Int => "Int",
            Self::Void => "Void",
            Self::Return => "Return",
            Self::If => "If",
            Self::Else => "Else",
            Self::Goto => "Goto",
            Self::Long => "Long",
            Self::Unsigned => "Unsigned",
            Self::Signed => "Signed",
            Self::Char => "Char",
            Self::Double => "Double",
            Self::Struct => "Struct",
            Self::Union => "Union",
            Self::Static => "Static",
            Self::Extern => "Extern",
            Self::While => "While",
            Self::Do => "Do",
            Self::For => "For",
            Self::Break => "Break",
            Self::Continue => "Continue",
            Self::Switch => "Switch",
            Self::Case => "Case",
            Self::Default => "Default",
            Self::Identifier(_) => "Identifier",
            Self::Constant(_) => "Constant",
            Self::LongConstant(_) => "LongConstant",
            Self::UIntConstant(_, _) => "UIntConstant",
            Self::CharLiteral(_) => "CharLiteral",
            Self::StringLiteral(_) => "StringLiteral",
            Self::Minus => "Minus",
            Self::Tilde => "Tilde",
            Self::Bang => "Bang",
            Self::Plus => "Plus",
            Self::Star => "Star",
            Self::Slash => "Slash",
            Self::Percent => "Percent",
            Self::ShiftLeft => "ShiftLeft",
            Self::ShiftRight => "ShiftRight",
            Self::Arrow => "Arrow",
            Self::Less => "Less",
            Self::LessEqual => "LessEqual",
            Self::Greater => "Greater",
            Self::GreaterEqual => "GreaterEqual",
            Self::Equal => "Equal",
            Self::EqualEqual => "EqualEqual",
            Self::NotEqual => "NotEqual",
            Self::Ampersand => "Ampersand",
            Self::Caret => "Caret",
            Self::Pipe => "Pipe",
            Self::LogicalAnd => "LogicalAnd",
            Self::LogicalOr => "LogicalOr",
            Self::PlusEqual => "PlusEqual",
            Self::MinusEqual => "MinusEqual",
            Self::StarEqual => "StarEqual",
            Self::SlashEqual => "SlashEqual",
            Self::PercentEqual => "PercentEqual",
            Self::AmpersandEqual => "AmpersandEqual",
            Self::CaretEqual => "CaretEqual",
            Self::PipeEqual => "PipeEqual",
            Self::ShiftLeftEqual => "ShiftLeftEqual",
            Self::ShiftRightEqual => "ShiftRightEqual",
            Self::PlusPlus => "PlusPlus",
            Self::MinusMinus => "MinusMinus",
            Self::OpenParen => "OpenParen",
            Self::CloseParen => "CloseParen",
            Self::OpenBrace => "OpenBrace",
            Self::CloseBrace => "CloseBrace",
            Self::OpenBracket => "OpenBracket",
            Self::CloseBracket => "CloseBracket",
            Self::Dot => "Dot",
            Self::Question => "Question",
            Self::Colon => "Colon",
            Self::Comma => "Comma",
            Self::Semicolon => "Semicolon",
            Self::Eof => "Eof",
        }
    }
}