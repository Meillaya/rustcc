//! Compile-time execution for the early native backend.
//!
//! Chapters 1-8 can be reduced to a deterministic integer return value.  This
//! evaluator walks the lowered control-flow instruction stream, using a `HashMap`
//! for local variables and labels because names are discovered dynamically during
//! lowering.  The public entry point keeps the interpreter private while giving
//! `compiler.rs` one behavior-preserving call.

use std::collections::HashMap;

use anyhow::{Result, bail};

use crate::ast::{AssignOp, BinaryOp, BlockItem, Expr, ForInit, Program, Statement};
use crate::ir::lower::Lowerer;
use crate::ir::tacky::Instr;

pub(crate) fn evaluate_program(program: &Program) -> Result<i32> {
    Evaluator::default().eval_program(program)
}

#[derive(Default)]
struct Evaluator {
    variables: HashMap<String, i32>,
}

impl Evaluator {
    fn eval_program(&mut self, program: &Program) -> Result<i32> {
        // Chapter 6's goto tests exercise C's awkward rule that a jump may skip
        // a declaration initializer while the declared identifier is still in
        // scope after the label.  To model that simply, the interpreter first
        // allocates every local slot with value 0, then executes declaration
        // initializers only when control reaches that declaration instruction.
        self.predeclare_locals(program)?;

        let instrs = Lowerer::default().lower_program(program);
        let mut labels = HashMap::new();
        for (index, instr) in instrs.iter().enumerate() {
            if let Instr::Label(label) = instr {
                labels.insert(label.clone(), index);
            }
        }

        let mut pc = 0usize;
        let mut steps = 0usize;
        while pc < instrs.len() {
            steps += 1;
            if steps > 100_000 {
                bail!("runtime error: probable infinite loop during compile-time evaluation");
            }
            match &instrs[pc] {
                Instr::Declare { name, init } => {
                    if let Some(init) = init {
                        let value = self.eval_expr(init)?;
                        self.variables.insert(name.clone(), value);
                    }
                    pc += 1;
                }
                Instr::Expr(Some(expr)) => {
                    self.eval_expr(expr)?;
                    pc += 1;
                }
                Instr::Expr(None) | Instr::Label(_) => pc += 1,
                Instr::Return(expr) => return self.eval_expr(expr),
                Instr::Jump(label) => {
                    pc = *labels
                        .get(label)
                        .ok_or_else(|| anyhow::anyhow!("semantic error: missing label {label}"))?;
                }
                Instr::JumpIfZero { condition, target } => {
                    if self.eval_expr(condition)? == 0 {
                        pc = *labels.get(target).ok_or_else(|| {
                            anyhow::anyhow!("internal error: missing generated label {target}")
                        })?;
                    } else {
                        pc += 1;
                    }
                }
                Instr::Switch {
                    expr,
                    cases,
                    default,
                    end,
                } => {
                    let value = self.eval_expr(expr)?;
                    let target = cases
                        .iter()
                        .find_map(|(case_value, label)| {
                            (*case_value == value).then_some(label.as_str())
                        })
                        .or(default.as_deref())
                        .unwrap_or(end);
                    pc = *labels.get(target).ok_or_else(|| {
                        anyhow::anyhow!("internal error: missing switch label {target}")
                    })?;
                }
            }
        }
        Ok(0)
    }

    fn predeclare_locals(&mut self, program: &Program) -> Result<()> {
        for item in &program.body {
            self.predeclare_from_block_item(item)?;
        }
        Ok(())
    }

    fn predeclare_from_block_item(&mut self, item: &BlockItem) -> Result<()> {
        match item {
            BlockItem::Declaration { name, init } => {
                if self.variables.contains_key(name) {
                    bail!("semantic error: redefinition of {name}");
                }
                let _ = init;
                self.variables.insert(name.clone(), 0);
            }
            BlockItem::Statement(statement) => self.predeclare_from_statement(statement)?,
        }
        Ok(())
    }

    fn predeclare_from_statement(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                self.predeclare_from_statement(body)?;
            }
            Statement::For { init, body, .. } => {
                if let Some(ForInit::Declaration { name, .. }) = init {
                    if self.variables.contains_key(name) {
                        bail!("semantic error: redefinition of {name}");
                    }
                    self.variables.insert(name.clone(), 0);
                }
                self.predeclare_from_statement(body)?;
            }
            Statement::Switch { body, .. }
            | Statement::Case {
                statement: body, ..
            }
            | Statement::Default { statement: body }
            | Statement::Labeled {
                statement: body, ..
            } => self.predeclare_from_statement(body)?,
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.predeclare_from_statement(then_branch)?;
                if let Some(else_branch) = else_branch {
                    self.predeclare_from_statement(else_branch)?;
                }
            }
            Statement::Block(items) => {
                for item in items {
                    self.predeclare_from_block_item(item)?;
                }
            }
            Statement::Return(_)
            | Statement::Break
            | Statement::Continue
            | Statement::Goto(_)
            | Statement::Expr(_) => {}
        }
        Ok(())
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<i32> {
        match expr {
            Expr::Constant(value) => Ok(*value),
            Expr::Var(name) => self
                .variables
                .get(name)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("semantic error: undeclared variable {name}")),
            Expr::Paren(inner) => self.eval_expr(inner),
            Expr::Negate(inner) => Ok(self.eval_expr(inner)?.wrapping_neg()),
            Expr::Complement(inner) => Ok(!self.eval_expr(inner)?),
            Expr::LogicalNot(inner) => Ok(i32::from(self.eval_expr(inner)? == 0)),
            Expr::PreInc(inner) => self.bump_lvalue(inner, 1, true),
            Expr::PreDec(inner) => self.bump_lvalue(inner, -1, true),
            Expr::PostInc(inner) => self.bump_lvalue(inner, 1, false),
            Expr::PostDec(inner) => self.bump_lvalue(inner, -1, false),
            Expr::Assign { op, target, value } => self.eval_assign(*op, target, value),
            Expr::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                if self.eval_expr(condition)? != 0 {
                    self.eval_expr(then_expr)
                } else {
                    self.eval_expr(else_expr)
                }
            }
            Expr::Binary { op, left, right } => self.eval_binary(*op, left, right),
        }
    }

    fn eval_binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr) -> Result<i32> {
        match op {
            BinaryOp::LogicalAnd => {
                if self.eval_expr(left)? == 0 {
                    Ok(0)
                } else {
                    Ok(i32::from(self.eval_expr(right)? != 0))
                }
            }
            BinaryOp::LogicalOr => {
                if self.eval_expr(left)? != 0 {
                    Ok(1)
                } else {
                    Ok(i32::from(self.eval_expr(right)? != 0))
                }
            }
            _ => Ok(op.eval_values(self.eval_expr(left)?, self.eval_expr(right)?)),
        }
    }

    fn eval_assign(&mut self, op: AssignOp, target: &Expr, value: &Expr) -> Result<i32> {
        let name = target
            .lvalue_name()
            .ok_or_else(|| anyhow::anyhow!("semantic error: invalid assignment target"))?
            .to_string();
        let rhs = self.eval_expr(value)?;
        let old = self
            .variables
            .get(&name)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("semantic error: undeclared variable {name}"))?;
        let new_value = match op {
            AssignOp::Assign => rhs,
            AssignOp::Add => BinaryOp::Add.eval_values(old, rhs),
            AssignOp::Subtract => BinaryOp::Subtract.eval_values(old, rhs),
            AssignOp::Multiply => BinaryOp::Multiply.eval_values(old, rhs),
            AssignOp::Divide => BinaryOp::Divide.eval_values(old, rhs),
            AssignOp::Remainder => BinaryOp::Remainder.eval_values(old, rhs),
            AssignOp::ShiftLeft => BinaryOp::ShiftLeft.eval_values(old, rhs),
            AssignOp::ShiftRight => BinaryOp::ShiftRight.eval_values(old, rhs),
            AssignOp::BitwiseAnd => BinaryOp::BitwiseAnd.eval_values(old, rhs),
            AssignOp::BitwiseXor => BinaryOp::BitwiseXor.eval_values(old, rhs),
            AssignOp::BitwiseOr => BinaryOp::BitwiseOr.eval_values(old, rhs),
        };
        self.variables.insert(name, new_value);
        Ok(new_value)
    }

    fn bump_lvalue(&mut self, target: &Expr, delta: i32, prefix: bool) -> Result<i32> {
        let name = target
            .lvalue_name()
            .ok_or_else(|| anyhow::anyhow!("semantic error: increment target is not an lvalue"))?
            .to_string();
        let old = self
            .variables
            .get(&name)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("semantic error: undeclared variable {name}"))?;
        let new_value = old.wrapping_add(delta);
        self.variables.insert(name, new_value);
        Ok(if prefix { new_value } else { old })
    }
}
