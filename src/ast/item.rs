//! Top-level translation-unit items.

#![allow(dead_code)]

/// Placeholder top-level item kinds.
#[derive(Debug, Clone)]
pub enum Item {
    Function,
    GlobalDeclaration,
    StructDeclaration,
}
