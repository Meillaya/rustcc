//! Reserved-word classification surface.
//!
//! As later chapters add keywords, centralize their classification here so
//! token recognition and parser expectations stay in sync.

#![allow(dead_code)]

/// Placeholder classification for reserved words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Int,
    Void,
    Return,
    If,
    Else,
    While,
    Do,
    For,
    Break,
    Continue,
    Static,
    Extern,
    Long,
    Unsigned,
    Double,
    Char,
    Sizeof,
    Struct,
    Union,
}
