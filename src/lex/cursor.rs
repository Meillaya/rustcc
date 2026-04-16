//! Cursor-level source traversal helpers for the lexer.

#![allow(dead_code)]

/// Placeholder for a character-oriented cursor over source text.
#[derive(Debug, Default, Clone)]
pub struct LexCursor;

impl LexCursor {
    /// Construct a new cursor for future lexing work.
    pub fn new() -> Self {
        Self
    }
}
