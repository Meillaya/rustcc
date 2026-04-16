//! Source-location placeholders.

#![allow(dead_code)]

/// Placeholder byte-span structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}
