//! Identifier resolution pass for the chapter-5 subset.
//!
//! Mirrors `nqcc2/lib/semantic_analysis/resolve.ml`.  The pass:
//!
//! 1. Walks every function body, tracking a scope stack of declared variable
//!    names.  Each declaration is inserted into the innermost scope.  A second
//!    declaration of the same name in the same scope is rejected.
//! 2. Visits every expression.  An `Expr::Var(name)` reference is resolved
//!    against the nearest enclosing scope; an undeclared reference is rejected.
//!    Any other expression form is recursively walked so nested variable
//!    references are picked up.
//!
//! Scope mechanics intentionally stay close to the OCaml reference but are
//! flattened for the chapter-5 subset:
//!
//! - Only block scope is tracked (no file-scope variables — chapter 10).
//! - No `static` / `extern` linkage tracking (chapter 10).
//! - No goto/label validation (chapter 6).
//!
//! The pass returns the cloned [`Program`] in [`ResolvedProgram`]; the
//! reserved type alias keeps the downstream pipeline signature-stable.

use std::collections::HashSet;

use anyhow::{Result, bail};

use crate::ast::{BlockItem, Expr, ForInit, Program, Statement};

/// Thin wrapper that carries a `Program` after resolution.
#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub program: Program,
}

/// Resolve identifiers, declarations, and goto/label consistency (chapter 5
/// covers identifier resolution only).
///
/// Walks the function body with a fresh `ScopeStack`, validates every
/// declaration and reference, and returns the cloned program inside a
/// [`ResolvedProgram`] wrapper.
pub fn resolve_program(ast: &Program) -> Result<ResolvedProgram> {
    let mut scopes = ScopeStack::new();
    resolve_block(&ast.function.body, &mut scopes)?;
    Ok(ResolvedProgram {
        program: ast.clone(),
    })
}

/// A stack of declaration scopes, innermost-first.
///
/// `push` opens a new scope (e.g. on entering a `for` init or a block
/// statement).  `pop` closes it.  `declare` inserts into the innermost
/// scope; `lookup` walks outward through enclosing scopes.
#[derive(Debug, Default)]
struct ScopeStack {
    scopes: Vec<HashSet<String>>,
}

impl ScopeStack {
    fn new() -> Self {
        Self {
            scopes: vec![HashSet::new()],
        }
    }

    fn push(&mut self) {
        self.scopes.push(HashSet::new());
    }

    fn pop(&mut self) {
        self.scopes
            .pop()
            .expect("scope stack underflow (popped past root scope)");
    }

    fn declare(&mut self, name: &str) -> Result<()> {
        let current = self
            .scopes
            .last_mut()
            .expect("scope stack missing root scope");
        if !current.insert(name.to_string()) {
            bail!(
                "resolve error: duplicate declaration of '{name}' in the same scope"
            );
        }
        Ok(())
    }

    fn lookup(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }
}

fn resolve_block(items: &[BlockItem], scopes: &mut ScopeStack) -> Result<()> {
    for item in items {
        resolve_block_item(item, scopes)?;
    }
    Ok(())
}

fn resolve_block_item(item: &BlockItem, scopes: &mut ScopeStack) -> Result<()> {
    match item {
        BlockItem::Statement(stmt) => resolve_statement(stmt, scopes),
        BlockItem::Declaration { name, init } => {
            scopes.declare(name)?;
            if let Some(expr) = init {
                resolve_expr(expr, scopes)?;
            }
            Ok(())
        }
    }
}

fn resolve_for_init(init: &ForInit, scopes: &mut ScopeStack) -> Result<()> {
    match init {
        ForInit::Declaration { name, init } => {
            scopes.declare(name)?;
            if let Some(expr) = init {
                resolve_expr(expr, scopes)?;
            }
            Ok(())
        }
        ForInit::Expr(expr) => resolve_expr(expr, scopes),
    }
}

fn resolve_statement(stmt: &Statement, scopes: &mut ScopeStack) -> Result<()> {
    match stmt {
        Statement::Return(expr) => resolve_expr(expr, scopes),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            resolve_expr(condition, scopes)?;
            resolve_statement(then_branch, scopes)?;
            if let Some(else_branch) = else_branch {
                resolve_statement(else_branch, scopes)?;
            }
            Ok(())
        }
        Statement::Block(items) => {
            // Blocks open a fresh inner scope, just like the OCaml reference
            // shadows outer declarations on re-declaration but otherwise
            // inherits them through the parent stack.
            scopes.push();
            let result = resolve_block(items, scopes);
            scopes.pop();
            result
        }
        Statement::While { condition, body } => {
            resolve_expr(condition, scopes)?;
            resolve_statement(body, scopes)
        }
        Statement::DoWhile { body, condition } => {
            resolve_statement(body, scopes)?;
            resolve_expr(condition, scopes)
        }
        Statement::For {
            init,
            condition,
            post,
            body,
        } => {
            // `for` declares its own scope for the init declaration; this
            // matches the OCaml `For { ... }` arm that copies the identifier
            // map before resolving the init.
            scopes.push();
            if let Some(init) = init {
                resolve_for_init(init, scopes)?;
            }
            if let Some(condition) = condition {
                resolve_expr(condition, scopes)?;
            }
            if let Some(post) = post {
                resolve_expr(post, scopes)?;
            }
            let body_result = resolve_statement(body, scopes);
            scopes.pop();
            body_result
        }
        Statement::Break | Statement::Continue => Ok(()),
        Statement::Switch { expr, body } => {
            resolve_expr(expr, scopes)?;
            resolve_statement(body, scopes)
        }
        Statement::Case { value, statement } => {
            resolve_expr(value, scopes)?;
            resolve_statement(statement, scopes)
        }
        Statement::Default { statement } => resolve_statement(statement, scopes),
        Statement::Goto(_) => Ok(()),
        Statement::Labeled { statement, .. } => resolve_statement(statement, scopes),
        Statement::Expr(maybe_expr) => {
            if let Some(expr) = maybe_expr {
                resolve_expr(expr, scopes)?;
            }
            Ok(())
        }
    }
}

fn resolve_expr(expr: &Expr, scopes: &mut ScopeStack) -> Result<()> {
    match expr {
        Expr::Constant(_) | Expr::Paren(_) => Ok(()),
        Expr::Var(name) => {
            if scopes.lookup(name) {
                Ok(())
            } else {
                bail!("resolve error: undeclared variable '{name}'")
            }
        }
        Expr::Unary { expr: inner, .. } => resolve_expr(inner, scopes),
        Expr::PreInc(inner) | Expr::PreDec(inner) | Expr::PostInc(inner) | Expr::PostDec(inner) => {
            resolve_expr(inner, scopes)
        }
        Expr::Assign {
            target, value, ..
        } => {
            resolve_expr(target, scopes)?;
            resolve_expr(value, scopes)
        }
        Expr::Conditional {
            condition,
            then_expr,
            else_expr,
        } => {
            resolve_expr(condition, scopes)?;
            resolve_expr(then_expr, scopes)?;
            resolve_expr(else_expr, scopes)
        }
        Expr::Binary { left, right, .. } => {
            resolve_expr(left, scopes)?;
            resolve_expr(right, scopes)
        }
    }
}
