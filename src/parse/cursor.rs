//! Token-stream traversal helpers for the parser.

#![allow(dead_code)]

/// Placeholder token cursor for recursive-descent parsing.
#[derive(Debug, Default, Clone)]
pub struct ParseCursor;

impl ParseCursor {
    /// Construct a parser cursor once token streams are wired in.
    pub fn new() -> Self {
        Self
    }
}
