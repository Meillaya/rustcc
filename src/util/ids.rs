//! Placeholder identifiers for AST, IR, and symbol-table entities.

#![allow(dead_code)]

/// Placeholder generic identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(pub usize);
