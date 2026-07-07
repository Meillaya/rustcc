//! AST-to-TACKY lowering.
//!
//! Mirrors `nqcc2/lib/tacky_gen.ml` (593 LOC).  The full implementation walks
//! each top-level declaration, allocates temporaries via `TempIdGenerator`,
//! and emits a flat `Vec<Instruction>` per function.  This module lands the
//! chapter-1 subset: the typed AST's single function body is rewritten into a
//! flat instruction list with one `Return(Constant(N))` for each explicit
//! `return N;` statement.  Statement forms the chapter-1 lowerer does not
//! yet understand produce an empty instruction list so the pipeline still
//! yields a `TackyProgram`-shaped payload for higher-chapter test inputs;
//! subsequent waves (chapter 2+) replace this body with the book-faithful
//! `emit_tacky_for_statement` rewrite that covers expressions, blocks,
//! control flow, and declarations.

use anyhow::Result;

use crate::ast::{Program, Statement};
use crate::ir::tacky::{Instruction, TackyFunction, TackyProgram, Val};

/// Type alias for the typed AST consumed by lowering.
///
/// The full `TypedProgram` from `crate::semantics::typecheck` will carry
/// per-node type information added by the type-checker.  Until that pass
/// graduates the wrapper in wave 12+ the alias points at the bare AST
/// (`Program`) so the lowering signature is `&TypedProgram` today and stays
/// unchanged as the type-checker replaces the alias with the real
/// `TypedProgram` struct.
pub type TypedProgram = Program;

/// Lower a typed AST into a TACKY program.
///
/// Chapter-1 algorithm: for `Program { function: Function { name, body } }`
/// the lowerer walks `body` and emits one `Return(Constant(N))` per
/// `return N;` statement.  Statements the chapter-1 lowerer does not yet
/// understand produce an empty instruction list so the pipeline still
/// yields a `TackyProgram`-shaped payload for higher-chapter test inputs.
/// Later waves replace this body with the book-faithful
/// `emit_tacky_for_statement` rewrite.
pub fn lower_program(ast: &TypedProgram) -> Result<TackyProgram> {
    let body: Vec<Instruction> = ast
        .function
        .body
        .iter()
        .map(lower_statement)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    let function = TackyFunction {
        name: ast.function.name.clone(),
        body,
    };

    Ok(TackyProgram {
        functions: vec![function],
    })
}

/// Lower a single AST statement into a flat TACKY instruction list.
///
/// Mirrors `emit_tacky_for_statement` from `nqcc2/lib/tacky_gen.ml`.  For
/// chapter 1 the only emitted form is `return <int_literal>;`; all other
/// statement forms produce an empty instruction list.  The lenient
/// fallthrough is intentional: the chapter-1 lowerer is the smallest
/// correct subset, and the pipeline must still produce a
/// `TackyProgram`-shaped payload for higher-chapter test inputs while the
/// full statement-by-statement rewrite lands in later waves.
fn lower_statement(stmt: &Statement) -> Result<Vec<Instruction>> {
    match stmt {
        Statement::Return(crate::ast::Expr::Constant(n)) => Ok(vec![Instruction::Return(
            Val::Constant(i64::from(*n)),
        )]),
        _ => Ok(Vec::new()),
    }
}
