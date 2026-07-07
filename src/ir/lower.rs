//! AST-to-TACKY lowering.
//!
//! Mirrors `nqcc2/lib/tacky_gen.ml` (593 LOC).  The full implementation walks
//! each top-level declaration, allocates temporaries via `TempIdGenerator`,
//! and emits a flat `Vec<Instruction>` per function.  This module lands the
//! chapter-1/chapter-2 subset: the typed AST's single function body is
//! rewritten into a flat instruction list.  Chapter 1 emits one
//! `Return(Constant(N))` per explicit `return N;` statement.  Chapter 2
//! adds the unary form: `return <unop> <int>;` and any nested combination
//! thereof lower to a sequence of `Copy` + `Negate|Complement`
//! instructions that compose on a freshly allocated temporary.  Statements
//! the lowerer does not yet understand produce an empty instruction list
//! so the pipeline still yields a `TackyProgram`-shaped payload for
//! higher-chapter test inputs.

use anyhow::Result;

use crate::ast::{Expr, Program, Statement, UnaryOp};
use crate::ir::tacky::{Instruction, TackyFunction, TackyProgram, Val};
use crate::ir::temp::TempIdGenerator;

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
/// Chapter 1+2 algorithm: for `Program { function: Function { name, body } }`
/// the lowerer walks `body`, allocates a fresh monotonic `TempIdGenerator`,
/// and emits a flat instruction list per `return <expr>;` statement.
/// Expressions handled today are integer constants and any nesting of
/// `Negate` / `Complement` applied to such constants.  Statements the
/// lowerer does not yet understand produce an empty instruction list so
/// the pipeline still yields a `TackyProgram`-shaped payload for
/// higher-chapter test inputs.
pub fn lower_program(ast: &TypedProgram) -> Result<TackyProgram> {
    let mut temps = TempIdGenerator::new();
    let body: Vec<Instruction> = ast
        .function
        .body
        .iter()
        .map(|stmt| lower_statement(stmt, &mut temps))
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
/// Mirrors `emit_tacky_for_statement` from `nqcc2/lib/tacky_gen.ml`.  The
/// chapter-1+2 forms are:
/// - `return <int>;` -> `[Return(Constant(N))]`
/// - `return <unop> <int>;` (any nesting) ->
///   `[Copy { src: inner_val, dst: tmp }; Negate|Complement { dst: tmp };
///   Return(Var(tmp))]`
fn lower_statement(
    stmt: &Statement,
    temps: &mut TempIdGenerator,
) -> Result<Vec<Instruction>> {
    match stmt {
        Statement::Return(Expr::Constant(n)) => Ok(vec![Instruction::Return(
            Val::Constant(i64::from(*n)),
        )]),
        Statement::Return(expr) if is_chapter2_expr(expr) => {
            let (instrs, val) = lower_expr(expr, temps);
            let mut instrs = instrs;
            instrs.push(Instruction::Return(val));
            Ok(instrs)
        }
        _ => Ok(Vec::new()),
    }
}

fn is_chapter2_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Constant(_) | Expr::Paren(_) | Expr::Unary { .. } => true,
        _ => false,
    }
}

/// Lower an expression into the instructions needed to compute it and
/// the resulting TACKY value.
///
/// Chapter 2 only recognizes `Constant`, transparent `Paren`, and
/// `Unary { op, expr }` (with the inner recursed).  Anything else
/// returns an empty instruction list paired with a dummy `Constant(0)`
/// so the pipeline still produces a `TackyProgram`-shaped payload.
fn lower_expr(expr: &Expr, temps: &mut TempIdGenerator) -> (Vec<Instruction>, Val) {
    match expr {
        Expr::Constant(n) => (Vec::new(), Val::Constant(i64::from(*n))),
        Expr::Paren(inner) => lower_expr(inner, temps),
        Expr::Unary { op, expr: inner } => {
            let (mut instrs, inner_val) = lower_expr(inner, temps);
            let tmp = format!("tmp.{}", temps.next().0);
            instrs.push(Instruction::Copy {
                src: inner_val,
                dst: tmp.clone(),
            });
            let unary = match op {
                UnaryOp::Negate => Instruction::Negate { dst: tmp.clone() },
                UnaryOp::Complement => Instruction::Complement { dst: tmp.clone() },
            };
            instrs.push(unary);
            (instrs, Val::Var(tmp))
        }
        _ => (Vec::new(), Val::Constant(0)),
    }
}
