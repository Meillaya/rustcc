//! AST-to-TACKY lowering.
//!
//! Mirrors `nqcc2/lib/tacky_gen.ml` (593 LOC).  The full implementation walks
//! each top-level declaration, allocates temporaries via `TempIdGenerator`,
//! and emits a flat `Vec<Instruction>` per function.  This module lands the
//! chapter-1/chapter-2/chapter-3/chapter-4 subset: the typed AST's single
//! function body is rewritten into a flat instruction list.
//!
//! - Chapter 1 emits one `Return(Constant(N))` per explicit `return N;`
//!   statement.
//! - Chapter 2 adds the unary form: `return <unop> <int>;` and any nested
//!   combination thereof lower to a sequence of `Copy` +
//!   `Negate|Complement` instructions that compose on a freshly allocated
//!   temporary.
//! - Chapter 3 widens the surface with binary arithmetic (`+ - * / %`) and
//!   the bitwise extras (`& | ^ << >>`), all lowered through the
//!   two-address `Copy` + `Binary` shape the codegen pass expects.
//! - Chapter 4 adds relational / equality operators (`< <= > >= == !=`)
//!   lowered to a TACKY `Cmp` instruction, logical not (`!`) lowered to
//!   a TACKY `Cmp` against zero, and the short-circuit logical operators
//!   `&&` and `||` lowered via `JumpIfZero` / `JumpIfNotZero` with a
//!   fresh end-label per expression.
//!
//! Statements the lowerer does not yet understand produce an empty
//! instruction list so the pipeline still yields a `TackyProgram`-shaped
//! payload for higher-chapter test inputs.

use anyhow::Result;

use crate::ast::{BinaryOp, Expr, Program, Statement, UnaryOp};
use crate::ir::tacky::{ConditionCode, Instruction, TackyFunction, TackyProgram, Val};
use crate::ir::temp::TempIdGenerator;
use crate::util::labels::LabelGenerator;

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
/// For `Program { function: Function { name, body } }` the lowerer walks
/// `body`, allocates a fresh monotonic `TempIdGenerator` for temporary
/// variables and a fresh [`LabelGenerator`] for short-circuit / structured
/// labels, and emits a flat instruction list per `return <expr>;`
/// statement.  Expressions handled through chapter 4 include integer
/// constants, transparent parens, any nesting of `Negate` / `Complement`
/// / `Not`, and any chapter-3 + chapter-4 binary expression.  Statements
/// the lowerer does not yet understand produce an empty instruction list
/// so the pipeline still yields a `TackyProgram`-shaped payload for
/// higher-chapter test inputs.
pub fn lower_program(ast: &TypedProgram) -> Result<TackyProgram> {
    let mut temps = TempIdGenerator::new();
    let mut labels = LabelGenerator::new();
    let body: Vec<Instruction> = ast
        .function
        .body
        .iter()
        .map(|stmt| lower_statement(stmt, &mut temps, &mut labels))
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
/// Mirrors `emit_tacky_for_statement` from `nqcc2/lib/tacky_gen.ml`.
///
/// - `return <int>;` -> `[Return(Constant(N))]`
/// - `return <unop> <int>;` (any nesting, including chapter-4 `!`) ->
///   `[Copy { src: inner_val, dst: tmp }; Negate|Complement { dst:
///   tmp }; Return(Var(tmp))]` for `Negate` / `Complement`, or
///   `[Cmp { left: inner_val, right: 0, dst: tmp, cc: E }; Return(Var(tmp))]`
///   for `Not`.
/// - `return <binary expr>;` (any chapter-3 + chapter-4 binary form, any
///   nesting) -> either the two-address shape `Copy left to tmp;
///   BinaryOp { src: right, dst: tmp }; Return(tmp)` (arithmetic /
///   bitwise / shift) or the Cmp form `Cmp { left, right, dst, cc };
///   Return(Var(tmp))` (relational / equality) or the short-circuit
///   sequence for `&&` / `||`.
fn lower_statement(
    stmt: &Statement,
    temps: &mut TempIdGenerator,
    labels: &mut LabelGenerator,
) -> Result<Vec<Instruction>> {
    match stmt {
        Statement::Return(Expr::Constant(n)) => Ok(vec![Instruction::Return(
            Val::Constant(i64::from(*n)),
        )]),
        Statement::Return(expr) if is_chapter4_expr(expr) => {
            let (instrs, val) = lower_expr(expr, temps, labels);
            let mut instrs = instrs;
            instrs.push(Instruction::Return(val));
            Ok(instrs)
        }
        _ => Ok(Vec::new()),
    }
}

fn is_chapter4_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Constant(_) | Expr::Paren(_) | Expr::Unary { .. } => true,
        Expr::Binary { left, right, .. } => {
            is_chapter4_expr(left) && is_chapter4_expr(right)
        }
        _ => false,
    }
}

/// Lower an expression into the instructions needed to compute it and
/// the resulting TACKY value.
///
/// Chapter 2 + 3 + 4 handles `Constant`, transparent `Paren`, `Unary`,
/// and any binary form (with the children recursed).  Anything else
/// returns an empty instruction list paired with a dummy `Constant(0)`
/// so the pipeline still produces a `TackyProgram`-shaped payload.
///
/// Arithmetic / bitwise / shift binops lower to the two-address form
/// `Copy v1, tmp; BinaryOp { src: v2, dst: tmp }`.  Chapter 4 equality
/// / relational binops lower to a single `Cmp { left, right, dst, cc }`
/// that compares `left` against `right` and writes 0 or 1 into `dst`.
/// The chapter-4 logical `&&` / `||` operators emit short-circuit
/// `JumpIfZero` / `JumpIfNotZero` sequences (mirrors
/// `emit_and_expression` / `emit_or_expression` in
/// `nqcc2/lib/tacky_gen.ml` ~lines 230-269).
fn lower_expr(
    expr: &Expr,
    temps: &mut TempIdGenerator,
    labels: &mut LabelGenerator,
) -> (Vec<Instruction>, Val) {
    match expr {
        Expr::Constant(n) => (Vec::new(), Val::Constant(i64::from(*n))),
        Expr::Paren(inner) => lower_expr(inner, temps, labels),
        Expr::Unary { op, expr: inner } => {
            let (mut instrs, inner_val) = lower_expr(inner, temps, labels);
            let tmp = format!("tmp.{}", temps.next().0);
            match op {
                UnaryOp::Negate => {
                    instrs.push(Instruction::Copy {
                        src: inner_val,
                        dst: tmp.clone(),
                    });
                    instrs.push(Instruction::Negate { dst: tmp.clone() });
                }
                UnaryOp::Complement => {
                    instrs.push(Instruction::Copy {
                        src: inner_val,
                        dst: tmp.clone(),
                    });
                    instrs.push(Instruction::Complement { dst: tmp.clone() });
                }
                // `!e` is logically equivalent to `e == 0`.  Lower to
                // a single TACKY `Cmp` against zero with the equality
                // condition code; codegen expands this into
                // `cmpl $0, src; sete dst; movzbl dst, dst`.
                UnaryOp::Not => {
                    instrs.push(Instruction::Cmp {
                        left: inner_val,
                        right: Val::Constant(0),
                        dst: tmp.clone(),
                        cc: ConditionCode::E,
                    });
                }
            }
            (instrs, Val::Var(tmp))
        }
        Expr::Binary { op, left, right } => match op {
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                // Short-circuit shape (mirrors OCaml `emit_and_expression`
                // / `emit_or_expression`):
                //   `e1 && e2` -> eval e1; JumpIfZero e1, false;
                //                  eval e2; JumpIfZero e2, false;
                //                  Copy 1, dst; Jump end;
                //                  Label false: Copy 0, dst; Label end:
                //   `e1 || e2` -> eval e1; JumpIfNotZero e1, true;
                //                  eval e2; JumpIfNotZero e2, true;
                //                  Copy 0, dst; Jump end;
                //                  Label true: Copy 1, dst; Label end:
                // The labels are unique per expression because the
                // lowerer owns a fresh [`LabelGenerator`] passed by
                // `lower_program`.
                let false_label = match op {
                    BinaryOp::LogicalAnd => labels.next_with_prefix("and_false"),
                    BinaryOp::LogicalOr => labels.next_with_prefix("or_false"),
                    _ => unreachable!("match arm filters this"),
                };
                let end_label = match op {
                    BinaryOp::LogicalAnd => labels.next_with_prefix("and_end"),
                    BinaryOp::LogicalOr => labels.next_with_prefix("or_end"),
                    _ => unreachable!("match arm filters this"),
                };
                let dst = format!("tmp.{}", temps.next().0);
                let dst_var = dst.clone();
                let (mut instrs, left_val) = lower_expr(left, temps, labels);
                let (right_instrs, right_val) = lower_expr(right, temps, labels);

                if matches!(op, BinaryOp::LogicalAnd) {
                    instrs.push(Instruction::JumpIfZero {
                        condition: left_val,
                        target: false_label.clone(),
                    });
                    instrs.extend(right_instrs);
                    instrs.push(Instruction::JumpIfZero {
                        condition: right_val,
                        target: false_label.clone(),
                    });
                    instrs.push(Instruction::Copy {
                        src: Val::Constant(1),
                        dst: dst.clone(),
                    });
                    instrs.push(Instruction::Jump {
                        target: end_label.clone(),
                    });
                    instrs.push(Instruction::Label(false_label));
                    instrs.push(Instruction::Copy {
                        src: Val::Constant(0),
                        dst: dst.clone(),
                    });
                    instrs.push(Instruction::Label(end_label));
                } else {
                    instrs.push(Instruction::JumpIfNotZero {
                        condition: left_val,
                        target: false_label.clone(),
                    });
                    instrs.extend(right_instrs);
                    instrs.push(Instruction::JumpIfNotZero {
                        condition: right_val,
                        target: false_label.clone(),
                    });
                    instrs.push(Instruction::Copy {
                        src: Val::Constant(0),
                        dst: dst.clone(),
                    });
                    instrs.push(Instruction::Jump {
                        target: end_label.clone(),
                    });
                    instrs.push(Instruction::Label(false_label));
                    instrs.push(Instruction::Copy {
                        src: Val::Constant(1),
                        dst: dst.clone(),
                    });
                    instrs.push(Instruction::Label(end_label));
                }
                (instrs, Val::Var(dst_var))
            }
            _ => {
                let (mut instrs, left_val) = lower_expr(left, temps, labels);
                let (right_instrs, right_val) = lower_expr(right, temps, labels);
                let tmp = format!("tmp.{}", temps.next().0);
                instrs.extend(right_instrs);
                // Arithmetic / bitwise / shift ops use the two-address
                // `Copy left, tmp; BinaryOp right, tmp` shape: the
                // destination pre-holds the left operand so a single
                // `<op> right, tmp` suffices.  Chapter-4 equality /
                // relational ops do not need this pre-copy because
                // `Cmp { left, right, dst, cc }` carries both
                // operands explicitly; see `binary_to_tacky`.
                let cmp = is_cmp_op(*op);
                instrs.push(Instruction::Copy {
                    src: left_val.clone(),
                    dst: tmp.clone(),
                });
                let instr = if cmp {
                    binary_to_tacky(*op, Some(left_val), right_val, tmp.clone())
                } else {
                    binary_to_tacky(*op, None, right_val, tmp.clone())
                };
                instrs.push(instr);
                (instrs, Val::Var(tmp))
            }
        },
        _ => (Vec::new(), Val::Constant(0)),
    }
}

fn is_cmp_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::LessThan
            | BinaryOp::LessOrEqual
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterOrEqual
    )
}

fn binary_to_tacky(
    op: BinaryOp,
    left: Option<Val>,
    right: Val,
    dst: String,
) -> Instruction {
    match op {
        BinaryOp::Add => Instruction::Add { src: right, dst },
        BinaryOp::Subtract => Instruction::Sub { src: right, dst },
        BinaryOp::Multiply => Instruction::Mul { src: right, dst },
        BinaryOp::Divide => Instruction::DivSigned { src: right, dst },
        BinaryOp::Remainder => Instruction::RemSigned { src: right, dst },
        BinaryOp::ShiftLeft => Instruction::BitShiftLeft { src: right, dst },
        BinaryOp::ShiftRight => Instruction::BitShiftRight { src: right, dst },
        BinaryOp::BitwiseAnd => Instruction::BitAnd { src: right, dst },
        BinaryOp::BitwiseXor => Instruction::BitXor { src: right, dst },
        BinaryOp::BitwiseOr => Instruction::BitOr { src: right, dst },
        // Chapter 4 — equality / relational lower to a TACKY `Cmp`.
        // The two-address Copy-then-BinaryOp shape used by the
        // arithmetic / bitwise ops would clobber `left` before the
        // comparison needs it, so Cmp takes both `left` and `right`
        // and writes the boolean result (0 or 1) into `dst`.  The
        // pre-emitted `Copy left, tmp` above is harmless here (it
        // just moves the value into `tmp`, and `Cmp` reads it from
        // there).
        BinaryOp::Equal => Instruction::Cmp {
            left: expect_cmp_left(left),
            right,
            dst,
            cc: ConditionCode::E,
        },
        BinaryOp::NotEqual => Instruction::Cmp {
            left: expect_cmp_left(left),
            right,
            dst,
            cc: ConditionCode::NE,
        },
        BinaryOp::LessThan => Instruction::Cmp {
            left: expect_cmp_left(left),
            right,
            dst,
            cc: ConditionCode::L,
        },
        BinaryOp::LessOrEqual => Instruction::Cmp {
            left: expect_cmp_left(left),
            right,
            dst,
            cc: ConditionCode::LE,
        },
        BinaryOp::GreaterThan => Instruction::Cmp {
            left: expect_cmp_left(left),
            right,
            dst,
            cc: ConditionCode::G,
        },
        BinaryOp::GreaterOrEqual => Instruction::Cmp {
            left: expect_cmp_left(left),
            right,
            dst,
            cc: ConditionCode::GE,
        },
        // Logical `&&` / `||` are short-circuit-lowered by
        // `lower_expr` above and never reach this conversion.
        BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
            unreachable!("logical && / || are lowered with short-circuit jumps")
        }
    }
}

/// Unwrap the `Option<Val>` that the arithmetic / comparison split
/// uses to carry `left` only when the binary op is a comparison.
/// Panics if the caller forgot to provide `left` for a comparison.
fn expect_cmp_left(left: Option<Val>) -> Val {
    left.expect("comparison binary op must carry its left operand")
}