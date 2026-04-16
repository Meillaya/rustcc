//! Optimization-pass placeholders.

#![allow(dead_code)]

/// Placeholder optimization family names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationPass {
    ConstantFolding,
    UnreachableCodeElimination,
    CopyPropagation,
    DeadStoreElimination,
}
