//! AST-to-TACKY lowering.
//!
//! Mirrors `nqcc2/lib/tacky_gen.ml` (593 LOC).  The full implementation walks
//! each top-level declaration, allocates temporaries via `TempIdGenerator`,
//! and emits a flat `Vec<Instruction>` per function.  This module lands the
//! chapter-1/chapter-2/chapter-3 subset: the typed AST's single function body
//! is rewritten into a flat instruction list.  Chapter 1 emits one
//! `Return(Constant(N))` per explicit `return N;` statement.  Chapter 2
//! adds the unary form: `return <unop> <int>;` and any nested combination
//! thereof lower to a sequence of `Copy` + `Negate|Complement`
//! instructions that compose on a freshly allocated temporary.  Chapter 3
//! widens the surface with binary arithmetic (`+ - * / %`) and the
//! bitwise extras (`& | ^ << >>`), all lowered through the two-address
//! `Copy` + `Binary` shape the codegen pass expects.  Statements the
//! lowerer does not yet understand produce an empty instruction list
//! so the pipeline still yields a `TackyProgram`-shaped payload for
//! higher-chapter test inputs.

use anyhow::Result;

use crate::ast::{BinaryOp, Expr, Program, Statement, UnaryOp};
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
/// Chapter 1+2+3 algorithm: for `Program { function: Function { name, body } }`
/// the lowerer walks `body`, allocates a fresh monotonic `TempIdGenerator`,
/// and emits a flat instruction list per `return <expr>;` statement.
/// Expressions handled today are integer constants, transparent parens,
/// any nesting of `Negate` / `Complement`, and any chapter-3 binary
/// expression.  Statements the lowerer does not yet understand produce
/// an empty instruction list so the pipeline still yields a
/// `TackyProgram`-shaped payload for higher-chapter test inputs.
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
/// chapter-1+2+3 forms are:
/// - `return <int>;` -> `[Return(Constant(N))]`
/// - `return <unop> <int>;` (any nesting) ->
///   `[Copy { src: inner_val, dst: tmp }; Negate|Complement { dst: tmp };
///   Return(Var(tmp))]`
/// - `return <binary expr>;` (any chapter-3 binary form, any nesting) ->
///   `[Copy left to tmp; BinaryOp { src: right, dst: tmp }; Return(tmp)]`
fn lower_statement(
    stmt: &Statement,
    temps: &mut TempIdGenerator,
) -> Result<Vec<Instruction>> {
    match stmt {
        Statement::Return(Expr::Constant(n)) => Ok(vec![Instruction::Return(
            Val::Constant(i64::from(*n)),
        )]),
        Statement::Return(expr) if is_chapter3_expr(expr) => {
            let (instrs, val) = lower_expr(expr, temps);
            let mut instrs = instrs;
            instrs.push(Instruction::Return(val));
            Ok(instrs)
        }
        _ => Ok(Vec::new()),
    }
}

fn is_chapter3_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Constant(_) | Expr::Paren(_) | Expr::Unary { .. } => true,
        Expr::Binary { op: _, left, right } => {
            is_chapter3_expr(left) && is_chapter3_expr(right)
        }
        _ => false,
    }
}

/// Lower an expression into the instructions needed to compute it and
/// the resulting TACKY value.
///
/// Chapter 2+3 handles `Constant`, transparent `Paren`, `Unary`, and any
/// chapter-3 `Binary` form (with the children recursed).  Anything else
/// returns an empty instruction list paired with a dummy `Constant(0)`
/// so the pipeline still produces a `TackyProgram`-shaped payload.
///
/// Binary lowering emits the two-address form `Copy v1, tmp; BinaryOp
/// v2, tmp` because every codegen variant consumes that shape.  This
/// pattern mirrors `emit_binary_expression` in
/// `nqcc2/lib/tacky_gen.ml` (~line 217) modulo the OCaml instruction's
/// `src1`/`src2` fields which our two-address TACKY folds into `dst` and
/// `src`.
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
        Expr::Binary { op, left, right } => {
            let (mut instrs, left_val) = lower_expr(left, temps);
            let (right_instrs, right_val) = lower_expr(right, temps);
            let tmp = format!("tmp.{}", temps.next().0);
            instrs.extend(right_instrs);
            instrs.push(Instruction::Copy {
                src: left_val,
                dst: tmp.clone(),
            });
            let binary = binary_to_tacky(*op, right_val, tmp.clone());
            instrs.push(binary);
            (instrs, Val::Var(tmp))
        }
        _ => (Vec::new(), Val::Constant(0)),
    }
}

fn binary_to_tacky(op: BinaryOp, src: Val, dst: String) -> Instruction {
    match op {
        BinaryOp::Add => Instruction::Add { src, dst },
        BinaryOp::Subtract => Instruction::Sub { src, dst },
        BinaryOp::Multiply => Instruction::Mul { src, dst },
        BinaryOp::Divide => Instruction::DivSigned { src, dst },
        BinaryOp::Remainder => Instruction::RemSigned { src, dst },
        BinaryOp::ShiftLeft => Instruction::BitShiftLeft { src, dst },
        BinaryOp::ShiftRight => Instruction::BitShiftRight { src, dst },
        BinaryOp::BitwiseAnd => Instruction::BitAnd { src, dst },
        BinaryOp::BitwiseXor => Instruction::BitXor { src, dst },
        BinaryOp::BitwiseOr => Instruction::BitOr { src, dst },
    }
}