//! Shared error surfaces reserved for richer compiler diagnostics.

#![allow(dead_code)]

/// Placeholder compiler-stage tags for future error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerStage {
    Driver,
    Lex,
    Parse,
    Validate,
    Tacky,
    Codegen,
    Link,
}
