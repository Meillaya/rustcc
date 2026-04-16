//! Diagnostic-reporting placeholders.

#![allow(dead_code)]

/// Placeholder diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}
