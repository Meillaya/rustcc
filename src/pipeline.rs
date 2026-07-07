//! Compiler pipeline stage stubs.
//!
//! This module mirrors the chapter progression from the book: lex, parse,
//! resolve, label loops, typecheck, lower to TACKY, optimize TACKY, generate
//! assembly, fix up assembly, replace pseudoregisters, and emit text.  Most
//! stages are intentionally thin placeholders today; they wire the facade to
//! existing helpers while real implementations land in later Wave-0 tasks.

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

    use crate::semantics::label_loops as label_loops_pass;
    use crate::semantics::ResolvedProgram;

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

    use crate::ir::lower::Lowerer;
    use crate::ir::tacky::TackyProgram;
    use crate::semantics::TypedProgram;

    pub(crate) fn generate_tacky(program: &TypedProgram) -> Result<TackyProgram> {
        let mut lowerer = Lowerer::default();
        lowerer.lower_program(&program.program)
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
    use anyhow::{anyhow, Result};

    use crate::ir::tacky::TackyProgram;
    use crate::semantics::TypedProgram;

    pub(crate) fn convert_tacky_to_asm(
        _tacky: &TackyProgram,
        _program: &TypedProgram,
    ) -> Result<String> {
        Err(anyhow!(
            "chapter 1+ codegen wired in W2-T3; this pipeline stage is now a real generator"
        ))
    }
}

pub mod asm_fixup {
    use anyhow::Result;

    pub(crate) fn fixup_asm(assembly: String) -> Result<String> {
        Ok(assembly)
    }
}

pub mod replace_pseudos {
    use anyhow::Result;

    pub(crate) fn replace_pseudos(assembly: String) -> Result<String> {
        Ok(assembly)
    }
}

pub mod emit {
    use anyhow::Result;

    pub(crate) fn emit(assembly: String) -> Result<String> {
        Ok(assembly)
    }
}
