//! Temporary-value placeholders used by lowering and register allocation.

#![allow(dead_code)]

/// Placeholder temporary identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempId(pub usize);
