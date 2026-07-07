//! Label/goto validation pass.
//!
//! Mirrors the validation responsibilities of `nqcc2/lib/semantic_analysis/label_loops.ml`.
//! The OCaml reference rewrites `break`/`continue`/`while`/`for`/`do-while`
//! with generated labels (Wave 8/9 work); the chapter-6 subset keeps the
//! pass small but adds the new responsibilities from the `--goto` extra:
//!
//! 1. Track every user-defined label in the function body.
//! 2. Reject duplicate labels (same name declared twice in one function).
//! 3. Reject `goto label;` whose target is not a label in the same
//!    function (which catches `goto variable;` because variables and
//!    labels are tracked in separate namespaces).
//! 4. Reject `goto` that crosses a function boundary (a chapter-9 problem
//!    once multi-function programs exist; the chapter-6 subset only has
//!    `main`, but the walker stays function-scoped so the rule generalises).
//!
//! The pass is intentionally read-only on the AST: it walks the function
//! body twice — once to collect labels, once to verify gotos — and bails
//! out with `anyhow::Error` on the first violation.  Lowering stays
//! responsible for translating `Labeled { name, .. }` into a TACKY
//! `Label(name)` and `Goto(name)` into a `Jump { target: name }`; the
//! validation here just guarantees those names line up.

use std::collections::HashSet;

use anyhow::{Result, bail};

use crate::ast::{BlockItem, ForInit, Program, Statement};

/// Validate labels and gotos in the program.
///
/// The chapter-6 subset has exactly one function (`main`); the walker
/// still scopes its label set per function so multi-function programs
/// from later chapters naturally get the right isolation.
pub fn label_loops(program: &mut Program) -> Result<()> {
    let body = program.function.body.clone();
    let mut labels = HashSet::new();
    collect_labels_block(&body, &mut labels)?;
    check_gotos_block(&body, &labels)?;
    Ok(())
}

/// Walk a block-item list, inserting every label name into `labels`.
/// Bails on the first duplicate label encountered.
fn collect_labels_block(items: &[BlockItem], labels: &mut HashSet<String>) -> Result<()> {
    for item in items {
        match item {
            BlockItem::Statement(stmt) => collect_labels_stmt(stmt, labels)?,
            // Declarations never carry labels; nothing to record.
            BlockItem::Declaration { .. } => {}
        }
    }
    Ok(())
}

fn collect_labels_stmt(stmt: &Statement, labels: &mut HashSet<String>) -> Result<()> {
    match stmt {
        Statement::Labeled { label, statement } => {
            if !labels.insert(label.clone()) {
                bail!(
                    "label_loops error: duplicate label '{label}' in function"
                );
            }
            collect_labels_stmt(statement, labels)?;
        }
        Statement::Block(items) => collect_labels_block(items, labels)?,
        Statement::If {
            then_branch,
            else_branch,
            ..
        } => {
            collect_labels_stmt(then_branch, labels)?;
            if let Some(else_branch) = else_branch {
                collect_labels_stmt(else_branch, labels)?;
            }
        }
        Statement::While { body, .. } => collect_labels_stmt(body, labels)?,
        Statement::DoWhile { body, .. } => collect_labels_stmt(body, labels)?,
        Statement::For { body, .. } => collect_labels_stmt(body, labels)?,
        Statement::Switch { body, .. } => collect_labels_stmt(body, labels)?,
        Statement::Case { statement, .. } => collect_labels_stmt(statement, labels)?,
        Statement::Default { statement } => collect_labels_stmt(statement, labels)?,
        Statement::Return(_)
        | Statement::Expr(_)
        | Statement::Break
        | Statement::Continue
        | Statement::Goto(_) => {}
    }
    Ok(())
}

/// Walk a block-item list and verify every `goto` references a known
/// label in the current function.
fn check_gotos_block(items: &[BlockItem], labels: &HashSet<String>) -> Result<()> {
    for item in items {
        match item {
            BlockItem::Statement(stmt) => check_gotos_stmt(stmt, labels)?,
            BlockItem::Declaration { init, .. } => {
                if let Some(expr) = init {
                    check_gotos_expr(expr, labels)?;
                }
            }
        }
    }
    Ok(())
}

fn check_gotos_stmt(stmt: &Statement, labels: &HashSet<String>) -> Result<()> {
    match stmt {
        Statement::Goto(target) => {
            if !labels.contains(target) {
                bail!(
                    "label_loops error: label '{target}' is not defined in the current function"
                );
            }
        }
        Statement::Return(expr) => check_gotos_expr(expr, labels)?,
        Statement::Expr(maybe_expr) => {
            if let Some(expr) = maybe_expr {
                check_gotos_expr(expr, labels)?;
            }
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            check_gotos_expr(condition, labels)?;
            check_gotos_stmt(then_branch, labels)?;
            if let Some(else_branch) = else_branch {
                check_gotos_stmt(else_branch, labels)?;
            }
        }
        Statement::Block(items) => check_gotos_block(items, labels)?,
        Statement::While { condition, body } => {
            check_gotos_expr(condition, labels)?;
            check_gotos_stmt(body, labels)?;
        }
        Statement::DoWhile { body, condition } => {
            check_gotos_stmt(body, labels)?;
            check_gotos_expr(condition, labels)?;
        }
        Statement::For {
            init,
            condition,
            post,
            body,
        } => {
            check_gotos_for_init(init, labels)?;
            if let Some(condition) = condition {
                check_gotos_expr(condition, labels)?;
            }
            if let Some(post) = post {
                check_gotos_expr(post, labels)?;
            }
            check_gotos_stmt(body, labels)?;
        }
        Statement::Switch { expr, body } => {
            check_gotos_expr(expr, labels)?;
            check_gotos_stmt(body, labels)?;
        }
        Statement::Case { value, statement } => {
            check_gotos_expr(value, labels)?;
            check_gotos_stmt(statement, labels)?;
        }
        Statement::Default { statement } => check_gotos_stmt(statement, labels)?,
        Statement::Labeled { statement, .. } => check_gotos_stmt(statement, labels)?,
        Statement::Break | Statement::Continue => {}
    }
    Ok(())
}

fn check_gotos_for_init(init: &Option<ForInit>, labels: &HashSet<String>) -> Result<()> {
    if let Some(init) = init {
        match init {
            ForInit::Declaration { init, .. } => {
                if let Some(expr) = init {
                    check_gotos_expr(expr, labels)?;
                }
            }
            ForInit::Expr(expr) => check_gotos_expr(expr, labels)?,
        }
    }
    Ok(())
}

/// Expressions don't carry goto, but the walker is recursive so any
/// future statement embedded inside an expression is still inspected.
/// The body is currently a no-op for gotos; it stays defensive so the
/// pass catches future regressions.
fn check_gotos_expr(_expr: &crate::ast::Expr, _labels: &HashSet<String>) -> Result<()> {
    Ok(())
}