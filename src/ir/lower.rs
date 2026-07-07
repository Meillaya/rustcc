//! AST-to-TACKY lowering.
//!
//! Mirrors `nqcc2/lib/tacky_gen.ml`.  Through chapter 5:
//!
//! - Chapter 1 emits one `Return(Constant(N))` per explicit `return N;`
//!   statement.
//! - Chapter 2 adds the unary form: `return <unop> <int>;` and any nested
//!   combination lower to a sequence of `Copy` + `Negate|Complement`
//!   instructions on a freshly allocated temporary.
//! - Chapter 3 widens with binary arithmetic (`+ - * / %`) and the
//!   bitwise extras (`& | ^ << >>`), all lowered through the two-address
//!   `Copy` + `Binary` shape.
//! - Chapter 4 adds relational / equality operators (`< <= > >= == !=`)
//!   lowered to a single `Cmp` instruction, logical not (`!`) lowered to
//!   `Cmp` against zero, and the short-circuit `&&` / `||` lowered via
//!   `JumpIfZero` / `JumpIfNotZero` with fresh labels.
//! - Chapter 5 adds mutable locals: declarations (no TACKY instruction
//!   for the slot itself), assignments (simple or compound), and
//!   pre/post `++` / `--`.  Lvalues are evaluated exactly once for
//!   compound assignment and pre/post increment so side effects in the
//!   lvalue expression stay well-behaved.

use anyhow::Result;

use crate::ast::{
    AssignOp, BinaryOp, BlockItem, Expr, ForInit, Program, Statement, UnaryOp,
};
use crate::ir::tacky::{ConditionCode, Instruction, TackyFunction, TackyProgram, Val};
use crate::ir::temp::TempIdGenerator;
use crate::util::labels::LabelGenerator;

pub type TypedProgram = Program;

pub fn lower_program(ast: &TypedProgram) -> Result<TackyProgram> {
    let mut ctx = LowerCtx::new();
    let body = lower_block_items(&ast.function.body, &mut ctx)?;
    let body = ensure_trailing_return(body);
    Ok(TackyProgram {
        functions: vec![TackyFunction {
            name: ast.function.name.clone(),
            body,
        }],
    })
}

/// Namespace-prefix user-defined labels so they cannot collide with
/// function names (`main:`) or with the auto-generated labels
/// (`if_end.0`, `while_cond.3`, ...).  The chapter-6 `--goto` extra
/// makes the conflict observable: the assembly emitter writes a
/// top-level `<name>:` for every TACKY `Label(name)`, so leaving a
/// user `main:` label unmangled would shadow the function entry
/// symbol.  Using a fixed prefix keeps the jump / label sides
/// symmetric (both call this helper) and keeps C identifier
/// characters — letters, digits, underscores — valid as the suffix.
fn mangle_user_label(name: &str) -> String {
    format!("user_label.{name}")
}

/// Append `Return(Constant(0))` when the function body does not already
/// end with one.  Mirrors `emit_fun_declaration` in
/// `nqcc2/lib/tacky_gen.ml` which unconditionally appends the same
/// synthetic return so `int main(void) {}` and friends still terminate.
fn ensure_trailing_return(body: Vec<Instruction>) -> Vec<Instruction> {
    let needs_synthetic = match body.last() {
        Some(Instruction::Return(_)) => false,
        _ => true,
    };
    if needs_synthetic {
        let mut body = body;
        body.push(Instruction::Return(Val::Constant(0)));
        return body;
    }
    body
}

#[derive(Debug, Clone)]
struct LowerCtx {
    temps: TempIdGenerator,
    labels: LabelGenerator,
}

impl LowerCtx {
    fn new() -> Self {
        Self {
            temps: TempIdGenerator::new(),
            labels: LabelGenerator::new(),
        }
    }

    fn fresh_tmp(&mut self) -> String {
        format!("tmp.{}", self.temps.next().0)
    }
}

fn lower_block_items(items: &[BlockItem], ctx: &mut LowerCtx) -> Result<Vec<Instruction>> {
    let mut out = Vec::new();
    for item in items {
        match item {
            BlockItem::Statement(stmt) => out.extend(lower_statement(stmt, ctx)?),
            BlockItem::Declaration { name, init } => {
                if let Some(expr) = init {
                    let (instrs, val) = lower_expr(expr, ctx)?;
                    out.extend(instrs);
                    out.push(Instruction::Copy {
                        src: val,
                        dst: name.clone(),
                    });
                }
            }
        }
    }
    Ok(out)
}

fn lower_statement(stmt: &Statement, ctx: &mut LowerCtx) -> Result<Vec<Instruction>> {
    match stmt {
        Statement::Return(expr) => {
            let (instrs, val) = lower_expr(expr, ctx)?;
            let mut out = instrs;
            out.push(Instruction::Return(val));
            Ok(out)
        }
        Statement::Expr(None) => Ok(Vec::new()),
        Statement::Expr(Some(expr)) => {
            let (instrs, _val) = lower_expr(expr, ctx)?;
            Ok(instrs)
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let end_label = ctx.labels.next_with_prefix("if_end");
            let else_label = ctx.labels.next_with_prefix("if_else");
            let (cond_instrs, cond_val) = lower_expr(condition, ctx)?;
            let mut out = cond_instrs;
            match else_branch {
                None => {
                    out.push(Instruction::JumpIfZero {
                        condition: cond_val,
                        target: end_label.clone(),
                    });
                    out.extend(lower_statement(then_branch, ctx)?);
                    out.push(Instruction::Label(end_label));
                }
                Some(else_branch) => {
                    out.push(Instruction::JumpIfZero {
                        condition: cond_val,
                        target: else_label.clone(),
                    });
                    out.extend(lower_statement(then_branch, ctx)?);
                    out.push(Instruction::Jump {
                        target: end_label.clone(),
                    });
                    out.push(Instruction::Label(else_label));
                    out.extend(lower_statement(else_branch, ctx)?);
                    out.push(Instruction::Label(end_label));
                }
            }
            Ok(out)
        }
        Statement::Block(items) => lower_block_items(items, ctx),
        Statement::While { condition, body } => {
            let cond_label = ctx.labels.next_with_prefix("while_cond");
            let end_label = ctx.labels.next_with_prefix("while_end");
            let (cond_instrs, cond_val) = lower_expr(condition, ctx)?;
            let mut out = Vec::new();
            out.push(Instruction::Label(cond_label.clone()));
            out.extend(cond_instrs);
            out.push(Instruction::JumpIfZero {
                condition: cond_val,
                target: end_label.clone(),
            });
            out.extend(lower_statement(body, ctx)?);
            out.push(Instruction::Jump {
                target: cond_label,
            });
            out.push(Instruction::Label(end_label));
            Ok(out)
        }
        Statement::DoWhile { body, condition } => {
            let start_label = ctx.labels.next_with_prefix("do_start");
            let cond_label = ctx.labels.next_with_prefix("do_cond");
            let (cond_instrs, cond_val) = lower_expr(condition, ctx)?;
            let mut out = Vec::new();
            out.push(Instruction::Label(start_label.clone()));
            out.extend(lower_statement(body, ctx)?);
            out.push(Instruction::Label(cond_label.clone()));
            out.extend(cond_instrs);
            out.push(Instruction::JumpIfNotZero {
                condition: cond_val,
                target: start_label,
            });
            Ok(out)
        }
        Statement::For {
            init,
            condition,
            post,
            body,
        } => {
            let start_label = ctx.labels.next_with_prefix("for_start");
            let end_label = ctx.labels.next_with_prefix("for_end");
            let mut out = Vec::new();
            if let Some(init) = init {
                match init {
                    ForInit::Declaration { name, init } => {
                        if let Some(expr) = init {
                            let (instrs, val) = lower_expr(expr, ctx)?;
                            out.extend(instrs);
                            out.push(Instruction::Copy {
                                src: val,
                                dst: name.clone(),
                            });
                        }
                    }
                    ForInit::Expr(expr) => {
                        let (instrs, _val) = lower_expr(expr, ctx)?;
                        out.extend(instrs);
                    }
                }
            }
            out.push(Instruction::Label(start_label.clone()));
            if let Some(condition) = condition {
                let (cond_instrs, cond_val) = lower_expr(condition, ctx)?;
                out.extend(cond_instrs);
                out.push(Instruction::JumpIfZero {
                    condition: cond_val,
                    target: end_label.clone(),
                });
            }
            out.extend(lower_statement(body, ctx)?);
            if let Some(post) = post {
                let (instrs, _val) = lower_expr(post, ctx)?;
                out.extend(instrs);
            }
            out.push(Instruction::Jump {
                target: start_label,
            });
            out.push(Instruction::Label(end_label));
            Ok(out)
        }
        Statement::Goto(target) => Ok(vec![Instruction::Jump {
            target: mangle_user_label(target),
        }]),
        Statement::Labeled { label, statement } => {
            let mut out = Vec::new();
            out.push(Instruction::Label(mangle_user_label(label)));
            out.extend(lower_statement(statement, ctx)?);
            Ok(out)
        }
        Statement::Break | Statement::Continue => Ok(Vec::new()),
        Statement::Switch { .. } | Statement::Case { .. } | Statement::Default { .. } => {
            Ok(Vec::new())
        }
    }
}

fn lower_expr(expr: &Expr, ctx: &mut LowerCtx) -> Result<(Vec<Instruction>, Val)> {
    match expr {
        Expr::Constant(n) => Ok((Vec::new(), Val::Constant(i64::from(*n)))),
        Expr::Var(name) => Ok((Vec::new(), Val::Var(name.clone()))),
        Expr::Paren(inner) => lower_expr(inner, ctx),
        Expr::Unary { op, expr: inner } => lower_unary(*op, inner, ctx),
        Expr::Assign { op, target, value } => lower_assign(*op, target, value, ctx),
        Expr::PreInc(inner) => lower_prefix_incdec(inner, true, ctx),
        Expr::PreDec(inner) => lower_prefix_incdec(inner, false, ctx),
        Expr::PostInc(inner) => lower_postfix_incdec(inner, true, ctx),
        Expr::PostDec(inner) => lower_postfix_incdec(inner, false, ctx),
        Expr::Conditional {
            condition,
            then_expr,
            else_expr,
        } => lower_conditional(condition, then_expr, else_expr, ctx),
        Expr::Binary { op, left, right } => lower_binary(*op, left, right, ctx),
    }
}

fn lower_unary(
    op: UnaryOp,
    inner: &Expr,
    ctx: &mut LowerCtx,
) -> Result<(Vec<Instruction>, Val)> {
    let (mut instrs, inner_val) = lower_expr(inner, ctx)?;
    let tmp = ctx.fresh_tmp();
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
        UnaryOp::Not => {
            instrs.push(Instruction::Cmp {
                left: inner_val,
                right: Val::Constant(0),
                dst: tmp.clone(),
                cc: ConditionCode::E,
            });
        }
    }
    Ok((instrs, Val::Var(tmp)))
}

fn lower_assign(
    op: AssignOp,
    target: &Expr,
    value: &Expr,
    ctx: &mut LowerCtx,
) -> Result<(Vec<Instruction>, Val)> {
    let target_name = target
        .lvalue_name()
        .ok_or_else(|| anyhow::anyhow!("lower: invalid lvalue in assignment target"))?
        .to_string();
    if op == AssignOp::Assign {
        let (mut instrs, rhs_val) = lower_expr(value, ctx)?;
        instrs.push(Instruction::Copy {
            src: rhs_val.clone(),
            dst: target_name,
        });
        return Ok((instrs, rhs_val));
    }
    let bin_op = compound_binop(op)
        .ok_or_else(|| anyhow::anyhow!("lower: invalid compound assignment operator"))?;
    let tmp = ctx.fresh_tmp();
    let (mut instrs, rhs_val) = lower_expr(value, ctx)?;
    instrs.push(Instruction::Copy {
        src: Val::Var(target_name.clone()),
        dst: tmp.clone(),
    });
    instrs.push(binary_to_tacky(bin_op, rhs_val, tmp.clone()));
    instrs.push(Instruction::Copy {
        src: Val::Var(tmp.clone()),
        dst: target_name,
    });
    Ok((instrs, Val::Var(tmp)))
}

fn lower_prefix_incdec(
    inner: &Expr,
    increment: bool,
    _ctx: &mut LowerCtx,
) -> Result<(Vec<Instruction>, Val)> {
    let target_name = inner
        .lvalue_name()
        .ok_or_else(|| anyhow::anyhow!("lower: invalid lvalue in ++/--"))?
        .to_string();
    let mut instrs = Vec::new();
    let instr = if increment {
        Instruction::Add {
            src: Val::Constant(1),
            dst: target_name.clone(),
        }
    } else {
        Instruction::Sub {
            src: Val::Constant(1),
            dst: target_name.clone(),
        }
    };
    instrs.push(instr);
    Ok((instrs, Val::Var(target_name)))
}

fn lower_postfix_incdec(
    inner: &Expr,
    increment: bool,
    ctx: &mut LowerCtx,
) -> Result<(Vec<Instruction>, Val)> {
    let target_name = inner
        .lvalue_name()
        .ok_or_else(|| anyhow::anyhow!("lower: invalid lvalue in ++/--"))?
        .to_string();
    let old = ctx.fresh_tmp();
    let mut instrs = Vec::new();
    instrs.push(Instruction::Copy {
        src: Val::Var(target_name.clone()),
        dst: old.clone(),
    });
    let instr = if increment {
        Instruction::Add {
            src: Val::Constant(1),
            dst: target_name,
        }
    } else {
        Instruction::Sub {
            src: Val::Constant(1),
            dst: target_name,
        }
    };
    instrs.push(instr);
    Ok((instrs, Val::Var(old)))
}

fn lower_conditional(
    condition: &Expr,
    then_expr: &Expr,
    else_expr: &Expr,
    ctx: &mut LowerCtx,
) -> Result<(Vec<Instruction>, Val)> {
    let result = ctx.fresh_tmp();
    let else_label = ctx.labels.next_with_prefix("cond_else");
    let end_label = ctx.labels.next_with_prefix("cond_end");
    let (cond_instrs, cond_val) = lower_expr(condition, ctx)?;
    let (mut then_instrs, then_val) = lower_expr(then_expr, ctx)?;
    let (mut else_instrs, else_val) = lower_expr(else_expr, ctx)?;

    let mut out = cond_instrs;
    out.push(Instruction::JumpIfZero {
        condition: cond_val,
        target: else_label.clone(),
    });
    then_instrs.push(Instruction::Copy {
        src: then_val,
        dst: result.clone(),
    });
    then_instrs.push(Instruction::Jump {
        target: end_label.clone(),
    });
    out.extend(then_instrs);
    out.push(Instruction::Label(else_label));
    else_instrs.push(Instruction::Copy {
        src: else_val,
        dst: result.clone(),
    });
    out.extend(else_instrs);
    out.push(Instruction::Label(end_label));
    Ok((out, Val::Var(result)))
}

fn lower_binary(
    op: BinaryOp,
    left: &Expr,
    right: &Expr,
    ctx: &mut LowerCtx,
) -> Result<(Vec<Instruction>, Val)> {
    match op {
        BinaryOp::LogicalAnd => emit_short_circuit(left, right, false, ctx, "and"),
        BinaryOp::LogicalOr => emit_short_circuit(left, right, true, ctx, "or"),
        _ => {
            let (mut instrs, left_val) = lower_expr(left, ctx)?;
            let (right_instrs, right_val) = lower_expr(right, ctx)?;
            instrs.extend(right_instrs);
            if is_cmp_op(op) {
                let tmp = ctx.fresh_tmp();
                instrs.push(Instruction::Copy {
                    src: left_val.clone(),
                    dst: tmp.clone(),
                });
                instrs.push(cmp_to_tacky(op, left_val, right_val, tmp.clone()));
                Ok((instrs, Val::Var(tmp)))
            } else {
                let tmp = ctx.fresh_tmp();
                instrs.push(Instruction::Copy {
                    src: left_val,
                    dst: tmp.clone(),
                });
                instrs.push(binary_to_tacky(op, right_val, tmp.clone()));
                Ok((instrs, Val::Var(tmp)))
            }
        }
    }
}

fn emit_short_circuit(
    left: &Expr,
    right: &Expr,
    is_or: bool,
    ctx: &mut LowerCtx,
    prefix: &str,
) -> Result<(Vec<Instruction>, Val)> {
    let dst = ctx.fresh_tmp();
    let false_label = ctx.labels.next_with_prefix(&format!("{prefix}_false"));
    let end_label = ctx.labels.next_with_prefix(&format!("{prefix}_end"));
    let (mut instrs, left_val) = lower_expr(left, ctx)?;
    let (right_instrs, right_val) = lower_expr(right, ctx)?;
    let jump = if is_or {
        Instruction::JumpIfNotZero {
            condition: left_val,
            target: false_label.clone(),
        }
    } else {
        Instruction::JumpIfZero {
            condition: left_val,
            target: false_label.clone(),
        }
    };
    instrs.push(jump);
    instrs.extend(right_instrs);
    let second_jump = if is_or {
        Instruction::JumpIfNotZero {
            condition: right_val,
            target: false_label.clone(),
        }
    } else {
        Instruction::JumpIfZero {
            condition: right_val,
            target: false_label.clone(),
        }
    };
    instrs.push(second_jump);
    // After both jumps fall through, neither operand short-circuited; the
    // combined result is the operator's long-form value (0 for `||`,
    // 1 for `&&`).  The short-circuit label holds the opposite value.
    let (short_circuit_value, long_form_value) = if is_or { (1, 0) } else { (0, 1) };
    instrs.push(Instruction::Copy {
        src: Val::Constant(long_form_value),
        dst: dst.clone(),
    });
    instrs.push(Instruction::Jump {
        target: end_label.clone(),
    });
    instrs.push(Instruction::Label(false_label));
    instrs.push(Instruction::Copy {
        src: Val::Constant(short_circuit_value),
        dst: dst.clone(),
    });
    instrs.push(Instruction::Label(end_label));
    Ok((instrs, Val::Var(dst)))
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
        _ => unreachable!("non-binary op in binary_to_tacky: {op:?}"),
    }
}

fn cmp_to_tacky(op: BinaryOp, left: Val, right: Val, dst: String) -> Instruction {
    let cc = match op {
        BinaryOp::Equal => ConditionCode::E,
        BinaryOp::NotEqual => ConditionCode::NE,
        BinaryOp::LessThan => ConditionCode::L,
        BinaryOp::LessOrEqual => ConditionCode::LE,
        BinaryOp::GreaterThan => ConditionCode::G,
        BinaryOp::GreaterOrEqual => ConditionCode::GE,
        _ => unreachable!("non-cmp op in cmp_to_tacky: {op:?}"),
    };
    Instruction::Cmp {
        left,
        right,
        dst,
        cc,
    }
}

fn compound_binop(op: AssignOp) -> Option<BinaryOp> {
    Some(match op {
        AssignOp::Assign => return None,
        AssignOp::Add => BinaryOp::Add,
        AssignOp::Subtract => BinaryOp::Subtract,
        AssignOp::Multiply => BinaryOp::Multiply,
        AssignOp::Divide => BinaryOp::Divide,
        AssignOp::Remainder => BinaryOp::Remainder,
        AssignOp::ShiftLeft => BinaryOp::ShiftLeft,
        AssignOp::ShiftRight => BinaryOp::ShiftRight,
        AssignOp::BitwiseAnd => BinaryOp::BitwiseAnd,
        AssignOp::BitwiseXor => BinaryOp::BitwiseXor,
        AssignOp::BitwiseOr => BinaryOp::BitwiseOr,
    })
}
