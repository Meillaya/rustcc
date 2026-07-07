//! AST-to-TACKY lowering.
//!
//! Mirrors `nqcc2/lib/tacky_gen.ml` (593 LOC).  The real implementation walks
//! each top-level declaration, allocates temporaries via `TempIdGenerator`,
//! and emits a flat `Vec<Instruction>` per function.  This stub produces an
//! empty TACKY program so the pipeline compiles against the new
//! book-faithful `TackyProgram` shape while the lowerer is rebuilt in W2-T2.

use anyhow::Result;

use crate::ast::Program;
use crate::ir::tacky::TackyProgram;

/// AST-to-TACKY lowerer.
///
/// Holds per-function state (label, break/continue targets) so the real W2-T2
/// implementation can keep the structured-statement rewrite idempotent.
#[derive(Debug, Default)]
pub(crate) struct Lowerer;

impl Lowerer {
    /// Lower an AST program into a TACKY program.
    ///
    /// Currently returns an empty `TackyProgram`.  The real lowering will
    /// emit a single `TackyFunction` for `int main(void)` and a flat
    /// instruction list per function.
    pub(crate) fn lower_program(&mut self, _program: &Program) -> Result<TackyProgram> {
        Ok(TackyProgram { functions: vec![] })
    }
}
