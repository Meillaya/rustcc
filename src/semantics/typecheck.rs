//! Type-checking pass.
//!
//! Mirrors `nqcc2/lib/semantic_analysis/typecheck.ml`.  Wave 12+ will introduce
//! the book's full type system (chapters 4-12 grow this pass incrementally).
//! For now the pass is a stub so the pipeline can be wired through the new
//! three-stage semantics facade.

use anyhow::Result;

use crate::ast::Program;

use super::resolve::ResolvedProgram;

/// Thin wrapper that carries a `Program` after type checking.
///
/// Concrete fields (e.g. resolved type annotations, expression types) will be
/// added as the type-checking pass lands.  The wrapper exists today so the
/// `lower_to_tacky` stage can take `&TypedProgram` and not silently regress
/// to the bare AST.
#[derive(Debug, Clone)]
pub struct TypedProgram {
    pub program: Program,
}

/// Type-check the program and attach per-node type information.
///
/// Returns `TypedProgram` so downstream TACKY generation sees typed expressions.
/// The real implementation will walk the AST, resolve declaration types, and
/// enforce C's type compatibility rules.  For now this is a transparent
/// pass-through stub so the pipeline compiles end-to-end while the type
/// checker lands in wave 12+.
pub fn typecheck(ast: &ResolvedProgram) -> Result<TypedProgram> {
    Ok(TypedProgram {
        program: ast.program.clone(),
    })
}
