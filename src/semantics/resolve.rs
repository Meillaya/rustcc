//! Identifier resolution pass.
//!
//! Mirrors `nqcc2/lib/semantic_analysis/resolve.ml`.  Wave 6 will move the
//! scope-tracking logic from the old monolithic `validate.rs` into this
//! module.  For now the pass is a stub so the pipeline can be wired through
//! the new three-stage semantics facade.

use anyhow::Result;

use crate::ast::Program;

/// Thin wrapper that carries a `Program` after resolution.
///
/// Concrete fields (e.g. resolved symbol tables, scopes) will be added as the
/// resolution pass lands.  The wrapper exists today so downstream stages can
/// take `&ResolvedProgram` and not silently regress to the bare AST.
#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub program: Program,
}

/// Resolve identifiers, declarations, and goto/label consistency.
///
/// Returns `ResolvedProgram` so downstream passes operate on the post-resolution
/// view.  The real implementation will populate symbol tables, validate scopes,
/// and reject undeclared names; for now this is a transparent pass-through stub
/// so the pipeline compiles end-to-end while the resolution pass lands in
/// wave 6.
pub fn resolve_program(ast: &Program) -> Result<ResolvedProgram> {
    Ok(ResolvedProgram {
        program: ast.clone(),
    })
}
