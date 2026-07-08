//! Compiler pipeline stages.
//!
//! This module mirrors the chapter progression from the book: lex, parse,
//! resolve, label loops, typecheck, lower to TACKY, optimize TACKY, generate
//! assembly, fix up assembly, replace pseudoregisters, and emit text.
//!
//! The TACKY-to-text path is stage-typed:
//!   - `tacky_to_asm` consumes TACKY and produces an `AsmProgram` (the
//!     typed assembly AST). It calls `codegen::generate` for the
//!     TACKY-walk.
//!   - `asm_fixup`, `replace_pseudos` are `AsmProgram -> AsmProgram` so
//!     they layer on top of codegen's output; chapter 1 wires them to
//!     identity bodies (`Ok(asm)`).
//!   - `emit` is the boundary into text: `AsmProgram -> String`. The
//!     driver writes this string to `.s` or feeds it directly to the
//!     assembler.
//!
//! Earlier stages (lex, parse, resolve, label_loops, typecheck,
//! tacky_gen, optimize) are thin wrappers over the module-level entry
//! points because the `pipeline.rs` shape mirrors the public facade
//! the task description expects: `lex -> parse -> resolve ->
//! label_loops -> typecheck -> ast_to_tacky -> optimize -> generate ->
//! fixup -> replace_pseudos -> emit`.

pub mod resolve {
    use anyhow::Result;

    use crate::ast::Program;
    use crate::semantics::{ResolvedProgram, resolve_program as resolve_pass};

    pub(crate) fn resolve_program(program: &Program) -> Result<ResolvedProgram> {
        resolve_pass(program)
    }
}

pub mod label_loops {
    use anyhow::Result;

    use crate::semantics::ResolvedProgram;
    use crate::semantics::label_loops as label_loops_pass;

    pub(crate) fn label_loops(mut resolved: ResolvedProgram) -> Result<ResolvedProgram> {
        let program = &mut resolved.program;
        label_loops_pass(program)?;
        Ok(resolved)
    }
}

pub mod typecheck {
    use anyhow::Result;

    use crate::semantics::{ResolvedProgram, TypedProgram, typecheck as typecheck_pass};

    pub(crate) fn typecheck_program(resolved: &ResolvedProgram) -> Result<TypedProgram> {
        typecheck_pass(resolved)
    }
}

pub mod tacky_gen {
    use anyhow::Result;

    use crate::ir::tacky::{TackyProgram, ast_to_tacky};
    use crate::semantics::TypedProgram;

    pub(crate) fn generate_tacky(program: &TypedProgram) -> Result<TackyProgram> {
        ast_to_tacky(program)
    }
}

pub mod optimize {
    use anyhow::Result;

    use crate::driver::OptimizationFlags;
    use crate::ir::opt::run_opt as run_pass;
    use crate::ir::tacky::TackyProgram;

    pub(crate) fn optimize_tacky(
        tacky: TackyProgram,
        _optimization_flags: OptimizationFlags,
    ) -> Result<TackyProgram> {
        // Until wave 20 lands each optimization pass we run no passes; the
        // pure identity function keeps the pipeline deterministic.
        let _ = run_pass;
        Ok(tacky)
    }
}

pub mod tacky_to_asm {
    //! TACKY -> AsmProgram.
    //!
    //! Wraps `crate::codegen::generate`. Frames are not yet computed by
    //! any chapter-1 codegen pass, so the slice is empty; the `&[Frame]`
    //! parameter is preserved so the function signature stays put when
    //! chapter 7+ fills in frame layouts.
    use anyhow::Result;

    use crate::codegen::{AsmProgram, generate};
    use crate::ir::tacky::TackyProgram;
    use crate::semantics::TypedProgram;

    pub(crate) fn convert_tacky_to_asm(
        tacky: &TackyProgram,
        _program: &TypedProgram,
    ) -> Result<AsmProgram> {
        // ch.1: no per-function frame layouts, so pass an empty slice.
        generate(tacky, &[])
    }
}

pub mod asm_fixup {
    //! AsmProgram -> AsmProgram.
    //!
    //! Wrapper over `crate::codegen::fixup`. The current body is the
    //! identity pass per the W2-T3 plan; chapter 9+ (W10) adds the
    //! real rewriter for two-operand mov/binary/div forms.
    use anyhow::Result;

    use crate::codegen::{AsmProgram, fixup as fixup_pass};

    pub(crate) fn fixup_asm(asm: AsmProgram) -> Result<AsmProgram> {
        fixup_pass(asm, &[])
    }
}

pub mod replace_pseudos {
    //! AsmProgram -> AsmProgram.
    use std::collections::HashSet;

    use anyhow::Result;

    use crate::codegen::{AsmProgram, replace_pseudos as replace_pseudos_pass};

    pub(crate) fn replace_pseudos(
        asm: AsmProgram,
        globals: &HashSet<String>,
    ) -> Result<AsmProgram> {
        replace_pseudos_pass(asm, globals)
    }
}

pub mod emit {
    //! AsmProgram -> String.
    //!
    //! Wrapper over `crate::codegen::emit`. This is the boundary
    //! between typed assembly and `.s` text: the driver writes the
    //! returned string to the assembly file or hands it to the
    //! assembler.
    use anyhow::Result;

    use crate::codegen::{AsmProgram, emit as emit_pass};

    pub(crate) fn emit(asm: &AsmProgram) -> Result<String> {
        emit_pass(asm)
    }
}
