//! Loop-labeling pass.
//!
//! Mirrors `nqcc2/lib/semantic_analysis/label_loops.ml`.  Wave 6+ will rewrite
//! `break`/`continue` and `for`/`while`/`do-while` constructs into explicit
//! jumps targeting generated labels, so later passes can treat control flow as
//! unconditional.  For now the pass is a stub so the pipeline can be wired
//! through the new three-stage semantics facade.

#![allow(dead_code)]

use anyhow::Result;

use crate::ast::Program;

/// Rewrite loops into explicit jumps targeting generated labels.
///
/// The real implementation will mutate the AST to insert labels and rewrite
/// control-flow statements.  For now this is a transparent no-op stub so the
/// pipeline compiles end-to-end while the label-rewriting pass lands in wave 6.
pub fn label_loops(_ast: &mut Program) -> Result<()> {
    Ok(())
}
