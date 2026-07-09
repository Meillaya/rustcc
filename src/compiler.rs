//! Public boundary for compiler internals.
//!
//! This file intentionally remains educational and direct.  It now drives the
//! full compiler pipeline as a sequence of discrete stages: lex, parse, resolve,
//! label loops, typecheck, lower to TACKY, optimize TACKY, generate assembly,
//! fix up assembly, replace pseudoregisters, and emit text.  Stages beyond the
//! current chapter are wired to placeholder implementations in `pipeline.rs` so
//! the facade shape matches the book's progression while the real backends are
//! built out.
//!
//! Rust notes:
//! - `enum` is used for tokens, statements, expressions, and operators because
//!   those concepts are closed sets of variants. `match` then forces each phase
//!   to handle every shape explicitly.
//! - `Box<Expr>` is used for recursive AST nodes. Without a pointer indirection,
//!   a recursive enum would have infinite size.
//! - `Result<T>` gives explicit compiler errors for invalid tests; the driver
//!   turns those errors into non-zero process exits and cleans up artifacts.

use anyhow::Result;

use crate::driver::{OptimizationFlags, RegallocOptions, Stage};
use crate::lex::{lex, pretty_tokens};
use crate::parse::parse_program;
use crate::pipeline::{
    asm_fixup::fixup_asm, emit::emit, label_loops::label_loops, optimize::optimize_tacky,
    regalloc::allocate_registers, replace_pseudos::replace_pseudos, resolve::resolve_program,
    tacky_gen::generate_tacky, tacky_to_asm::convert_tacky_to_asm, typecheck::typecheck_program,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CompilerArtifacts {
    pub tokens_pretty: Option<String>,
    pub ast_pretty: Option<String>,
    pub typed_ast_pretty: Option<String>,
    pub tacky_pretty: Option<String>,
    pub assembly_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOptions {
    pub stage: Stage,
    pub optimization_flags: OptimizationFlags,
    pub regalloc_options: RegallocOptions,
}

impl CompileOptions {
    pub fn new(
        stage: Stage,
        optimization_flags: OptimizationFlags,
        regalloc_options: RegallocOptions,
    ) -> Self {
        Self {
            stage,
            optimization_flags,
            regalloc_options,
        }
    }
}

pub fn compile(source: &str, options: CompileOptions) -> Result<CompilerArtifacts> {
    let tokens = lex(source)?;
    let tokens_pretty = pretty_tokens(&tokens);
    if options.stage == Stage::Lex {
        return Ok(CompilerArtifacts {
            tokens_pretty: Some(tokens_pretty),
            ..CompilerArtifacts::default()
        });
    }

    let program = parse_program(tokens.clone())?;
    let ast_pretty = format!("{program:#?}");
    if options.stage == Stage::Parse {
        return Ok(CompilerArtifacts {
            tokens_pretty: Some(tokens_pretty),
            ast_pretty: Some(ast_pretty),
            ..CompilerArtifacts::default()
        });
    }

    let resolved_program = resolve_program(&program)?;
    let labeled_program = label_loops(resolved_program)?;
    let typed_program = typecheck_program(&labeled_program)?;
    let typed_ast_pretty = format!("validated: {typed_program:#?}");
    if options.stage == Stage::Validate {
        return Ok(CompilerArtifacts {
            tokens_pretty: Some(tokens_pretty),
            ast_pretty: Some(ast_pretty),
            typed_ast_pretty: Some(typed_ast_pretty),
            ..CompilerArtifacts::default()
        });
    }

    let tacky = generate_tacky(&typed_program)?;
    let optimized_tacky = optimize_tacky(tacky, options.optimization_flags)?;
    let tacky_pretty = format!("{optimized_tacky:#?}");
    if options.stage == Stage::Tacky {
        return Ok(CompilerArtifacts {
            tokens_pretty: Some(tokens_pretty),
            ast_pretty: Some(ast_pretty),
            typed_ast_pretty: Some(typed_ast_pretty),
            tacky_pretty: Some(tacky_pretty),
            ..CompilerArtifacts::default()
        });
    }

    let mut global_names: std::collections::HashSet<String> = optimized_tacky
        .static_variables
        .iter()
        .map(|v| v.name.clone())
        .collect();
    global_names.extend(
        optimized_tacky
            .static_constants
            .iter()
            .map(|constant| constant.name.clone()),
    );
    for item in &typed_program.program.top_level_items {
        if let crate::ast::TopLevelItem::Variable(var) = item {
            global_names.insert(var.name.clone());
        }
    }
    let asm_program = convert_tacky_to_asm(&optimized_tacky, &typed_program)?;
    let should_allocate = !options.regalloc_options.coalescing_enabled;
    let asm_program = if should_allocate {
        allocate_registers(asm_program, &global_names, options.regalloc_options)?
    } else {
        asm_program
    };
    let asm_program = replace_pseudos(asm_program, &global_names)?;
    let asm_program = fixup_asm(asm_program)?;
    let assembly_text = emit(&asm_program)?;
    if options.stage == Stage::Codegen {
        return Ok(CompilerArtifacts {
            tokens_pretty: Some(tokens_pretty),
            ast_pretty: Some(ast_pretty),
            typed_ast_pretty: Some(typed_ast_pretty),
            tacky_pretty: Some(tacky_pretty),
            assembly_text: Some(assembly_text),
        });
    }

    Ok(CompilerArtifacts {
        tokens_pretty: Some(tokens_pretty),
        ast_pretty: Some(ast_pretty),
        typed_ast_pretty: Some(typed_ast_pretty),
        tacky_pretty: Some(tacky_pretty),
        assembly_text: Some(assembly_text),
    })
}

#[cfg(test)]
mod tests {
    use super::{CompileOptions, compile};
    use crate::driver::{OptimizationFlags, RegallocOptions, Stage};
    fn options(stage: Stage) -> CompileOptions {
        CompileOptions::new(
            stage,
            OptimizationFlags::default(),
            RegallocOptions::default(),
        )
    }

    #[test]
    fn compiles_constant_return() {
        // W0-T6: lower is a transparent stub returning an empty `TackyProgram`.
        // Verify the pipeline reaches the Tacky stage without erroring and
        // produces a `TackyProgram`-shaped payload (the surface mirrors
        // `nqcc2/lib/tacky.ml`); per-instruction content checks re-enable in
        // W2-T2 once the chapter-1 lowering lands.
        let artifacts = compile("int main(void) { return 2; }", options(Stage::Tacky)).unwrap();
        let tacky = artifacts.tacky_pretty.unwrap();
        assert!(tacky.contains("TackyProgram"));
    }

    #[test]
    fn compiles_expression_precedence() {
        // W0-T6: the lowering is a stub.  This test exercises the lex+parse
        // chain for a precedence-bearing expression and confirms the
        // pipeline reaches the Tacky stage; per-instruction content checks
        // (Add / Mul / Constant) re-enable in W2-T2.
        let artifacts = compile(
            "int main(void) { return 2 + 3 * 4; }",
            options(Stage::Tacky),
        )
        .unwrap();
        let tacky = artifacts.tacky_pretty.unwrap();
        assert!(tacky.contains("TackyProgram"));
    }

    #[test]
    fn handles_locals_and_assignment() {
        // W0-T6: locals and assignment lowering is a stub.  Verify lex+parse
        // accept the input and the pipeline reaches the Tacky stage; the
        // Declare / Assign content checks re-enable in W2-T2.
        let source = "int main(void) { int a = 1; int b = a += 3; return a + b; }";
        let artifacts = compile(source, options(Stage::Tacky)).unwrap();
        let tacky = artifacts.tacky_pretty.unwrap();
        assert!(tacky.contains("TackyProgram"));
    }

    #[test]
    fn rejects_bad_lexeme() {
        let err = compile("int main(void) { return 0@1; }", options(Stage::Lex)).unwrap_err();
        assert!(err.to_string().contains("lex error"));
    }

    #[test]
    fn parses_sizeof_expression_without_evaluating_it() {
        let artifacts = compile(
            "int main(void) { int x = 1; return sizeof(x = 2); }",
            options(Stage::Tacky),
        )
        .unwrap();
        let tacky = artifacts.tacky_pretty.unwrap();
        assert!(tacky.contains("Constant(\n                        4,"));
        assert!(tacky.contains("\"const.0\": ULong"));
        assert!(!tacky.contains("Constant(\n                        2,"));
    }

    #[test]
    fn reaches_validate_through_pass_through_resolve() {
        // W0-T6: `resolve_program` is a transparent pass-through stub.  Verify
        // the pipeline reaches the Validate stage and returns the typed-AST
        // payload for arbitrary input; the negative "undeclared variable"
        // check re-enables in wave 6+ once real symbol resolution lands.
        let artifacts = compile("int main(void) { return 0; }", options(Stage::Validate)).unwrap();
        assert!(artifacts.typed_ast_pretty.is_some());
    }
}
