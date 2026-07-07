//! Identifier resolution pass.
//!
//! Mirrors `nqcc2/lib/semantic_analysis/resolve.ml`.  The pass:
//!
//! 1. Walks every function body, tracking a scope stack of declared variable
//!    names.  Each declaration is inserted into the innermost scope and
//!    assigned a fresh unique name (e.g. `x` becomes `x.0`, `x.1`, ...).
//!    A second declaration of the same name in the same scope is rejected.
//! 2. Visits every expression.  An `Expr::Var(name)` reference is resolved
//!    against the nearest enclosing scope (innermost first) and rewritten to
//!    use the unique name recorded for that declaration.  An undeclared
//!    reference is rejected.  Any other expression form is recursively
//!    walked so nested variable references are picked up.
//!
//! The chapter-7 extension over chapter-5 is the **per-block scope stack**
//! and the **shadowing / name-mangling** behaviour.  Each compound statement
//! (`{ ... }`) opens a fresh inner scope; declarations inside that block
//! get a fresh unique name so the outer binding is preserved.  A reference
//! to a name walks the scope stack from innermost to outermost, picking up
//! the closest declaration's unique name.  When the block exits, the inner
//! scope is popped and the outer binding is naturally visible again.
//!
//! Scope mechanics intentionally stay close to the OCaml reference:
//!
//! - The function body has one root scope (chapter 5/7 model).
//! - Each `Statement::Block` and each `Statement::For` opens a child scope.
//! - Declaration order: when resolving `int x = init`, the new `x` is
//!   inserted into the current scope **before** resolving `init`, matching
//!   the C rule that a declarator's scope begins at the end of the
//!   declarator (so `int a = a + 1` references the new `a`).
//! - Only block scope is tracked (no file-scope variables — chapter 10).
//! - No `static` / `extern` linkage tracking (chapter 10).
//!
//! The pass returns a cloned [`Program`] with all variable references
//! rewritten to use the unique names inside [`ResolvedProgram`]; the
//! reserved type alias keeps the downstream pipeline signature-stable.

use std::collections::HashMap;

use anyhow::{Result, bail};

use crate::ast::{BlockItem, Expr, ForInit, Function, Program, Statement};

/// Thin wrapper that carries a `Program` after resolution.
#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub program: Program,
}

/// Resolve identifiers, declarations, and goto/label consistency (chapter 5
/// covers identifier resolution; chapter 7 extends the scope model to nested
/// blocks and produces a renamed AST so the lowerer naturally maps each
/// scope to its own stack slot).
///
/// Walks the function body with a fresh [`ScopeStack`], validates every
/// declaration and reference, mangles each declaration to a unique name,
/// and returns the rewritten program inside a [`ResolvedProgram`] wrapper.
pub fn resolve_program(ast: &Program) -> Result<ResolvedProgram> {
    let mut scopes = ScopeStack::new();
    let body = resolve_block(&ast.function.body, &mut scopes)?;
    Ok(ResolvedProgram {
        program: Program {
            function: Function {
                name: ast.function.name.clone(),
                body,
            },
        },
    })
}

/// A stack of declaration scopes, innermost-first.
///
/// `push` opens a new scope (e.g. on entering a compound statement or
/// a `for` init).  `pop` closes it.  `declare` inserts into the
/// innermost scope and returns a freshly minted unique name; `lookup`
/// walks outward through enclosing scopes and returns the unique name
/// recorded for the nearest declaration of the requested source name.
///
/// Each scope is a `HashMap<source_name, unique_name>`.  The unique
/// name uses a globally monotonic counter so `x.0` / `x.1` / `x.2` /
/// ... never collide across blocks, mirroring the OCaml reference's
/// `Unique_ids.make_named_temporary` (which mints names like
/// `tmp.name.42`).
#[derive(Debug, Default)]
struct ScopeStack {
    scopes: Vec<HashMap<String, String>>,
    next_id: u32,
}

impl ScopeStack {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            next_id: 0,
        }
    }

    fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.scopes
            .pop()
            .expect("scope stack underflow (popped past root scope)");
    }

    /// Insert `name` into the innermost scope and return a fresh unique
    /// name (`name.N`).  Rejects a duplicate declaration in the same
    /// scope — shadowing across nested scopes is allowed and is the
    /// reason the inner `HashMap` is keyed by the source name.
    fn declare(&mut self, name: &str) -> Result<String> {
        let current = self
            .scopes
            .last_mut()
            .expect("scope stack missing root scope");
        if current.contains_key(name) {
            bail!(
                "resolve error: duplicate declaration of '{name}' in the same scope"
            );
        }
        let unique = format!("{name}.{}", self.next_id);
        self.next_id += 1;
        current.insert(name.to_string(), unique.clone());
        Ok(unique)
    }

    /// Walk the scope stack innermost-first and return the unique name
    /// recorded for `name` in the nearest enclosing scope, or `None`
    /// if no scope declares it.
    fn lookup(&self, name: &str) -> Option<String> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }
}

fn resolve_block(items: &[BlockItem], scopes: &mut ScopeStack) -> Result<Vec<BlockItem>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(resolve_block_item(item, scopes)?);
    }
    Ok(out)
}

fn resolve_block_item(item: &BlockItem, scopes: &mut ScopeStack) -> Result<BlockItem> {
    match item {
        BlockItem::Statement(stmt) => Ok(BlockItem::Statement(resolve_statement(stmt, scopes)?)),
        BlockItem::Declaration { name, init } => {
            // C99 rule: the declarator's scope begins at the end of the
            // declarator, so `int a = a + 1` references the new `a` —
            // declare first, then resolve the init against the new
            // scope.  This matches the OCaml `resolve_local_var_helper`
            // ordering.
            let new_name = scopes.declare(name)?;
            let new_init = match init {
                Some(expr) => Some(resolve_expr(expr, scopes)?),
                None => None,
            };
            Ok(BlockItem::Declaration {
                name: new_name,
                init: new_init,
            })
        }
    }
}

fn resolve_for_init(init: &ForInit, scopes: &mut ScopeStack) -> Result<ForInit> {
    match init {
        ForInit::Declaration { name, init } => {
            let new_name = scopes.declare(name)?;
            let new_init = match init {
                Some(expr) => Some(resolve_expr(expr, scopes)?),
                None => None,
            };
            Ok(ForInit::Declaration {
                name: new_name,
                init: new_init,
            })
        }
        ForInit::Expr(expr) => Ok(ForInit::Expr(resolve_expr(expr, scopes)?)),
    }
}

fn resolve_statement(stmt: &Statement, scopes: &mut ScopeStack) -> Result<Statement> {
    match stmt {
        Statement::Return(expr) => Ok(Statement::Return(resolve_expr(expr, scopes)?)),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => Ok(Statement::If {
            condition: resolve_expr(condition, scopes)?,
            then_branch: Box::new(resolve_statement(then_branch, scopes)?),
            else_branch: match else_branch {
                Some(else_branch) => Some(Box::new(resolve_statement(else_branch, scopes)?)),
                None => None,
            },
        }),
        Statement::Block(items) => {
            // Blocks open a fresh inner scope, just like the OCaml
            // reference.  The inner `declare` calls get fresh unique
            // names, so an inner `int x` shadows the outer `x` for
            // references inside the block; once the block exits, the
            // outer scope's binding is visible again.
            scopes.push();
            let result = resolve_block(items, scopes);
            scopes.pop();
            Ok(Statement::Block(result?))
        }
        Statement::While { condition, body } => Ok(Statement::While {
            condition: resolve_expr(condition, scopes)?,
            body: Box::new(resolve_statement(body, scopes)?),
        }),
        Statement::DoWhile { body, condition } => Ok(Statement::DoWhile {
            body: Box::new(resolve_statement(body, scopes)?),
            condition: resolve_expr(condition, scopes)?,
        }),
        Statement::For {
            init,
            condition,
            post,
            body,
        } => {
            // `for` opens its own scope for the init declaration so a
            // `for (int i = ...)` doesn't leak the loop variable.  The
            // body, condition, and post all see the loop variable.
            scopes.push();
            let resolved_init = match init {
                Some(init) => Some(resolve_for_init(init, scopes)?),
                None => None,
            };
            let resolved_condition = match condition {
                Some(expr) => Some(resolve_expr(expr, scopes)?),
                None => None,
            };
            let resolved_post = match post {
                Some(expr) => Some(resolve_expr(expr, scopes)?),
                None => None,
            };
            let body_result = resolve_statement(body, scopes);
            scopes.pop();
            Ok(Statement::For {
                init: resolved_init,
                condition: resolved_condition,
                post: resolved_post,
                body: Box::new(body_result?),
            })
        }
        Statement::Break | Statement::Continue => Ok(stmt.clone()),
        Statement::Switch { expr, body } => Ok(Statement::Switch {
            expr: resolve_expr(expr, scopes)?,
            body: Box::new(resolve_statement(body, scopes)?),
        }),
        Statement::Case { value, statement } => Ok(Statement::Case {
            value: resolve_expr(value, scopes)?,
            statement: Box::new(resolve_statement(statement, scopes)?),
        }),
        Statement::Default { statement } => Ok(Statement::Default {
            statement: Box::new(resolve_statement(statement, scopes)?),
        }),
        Statement::Goto(target) => Ok(Statement::Goto(target.clone())),
        Statement::Labeled { label, statement } => Ok(Statement::Labeled {
            label: label.clone(),
            statement: Box::new(resolve_statement(statement, scopes)?),
        }),
        Statement::Expr(maybe_expr) => {
            let resolved = match maybe_expr {
                Some(expr) => Some(resolve_expr(expr, scopes)?),
                None => None,
            };
            Ok(Statement::Expr(resolved))
        }
    }
}

fn resolve_expr(expr: &Expr, scopes: &mut ScopeStack) -> Result<Expr> {
    match expr {
        Expr::Constant(n) => Ok(Expr::Constant(*n)),
        Expr::Paren(inner) => Ok(Expr::Paren(Box::new(resolve_expr(inner, scopes)?))),
        Expr::Var(name) => {
            // Resolve through the scope stack to find the nearest
            // enclosing declaration's unique name.  An undeclared
            // reference is rejected with a precise message.
            let unique = scopes
                .lookup(name)
                .ok_or_else(|| anyhow::anyhow!("resolve error: undeclared variable '{name}'"))?;
            Ok(Expr::Var(unique))
        }
        Expr::Unary { op, expr: inner } => Ok(Expr::Unary {
            op: *op,
            expr: Box::new(resolve_expr(inner, scopes)?),
        }),
        Expr::PreInc(inner) => Ok(Expr::PreInc(Box::new(resolve_expr(inner, scopes)?))),
        Expr::PreDec(inner) => Ok(Expr::PreDec(Box::new(resolve_expr(inner, scopes)?))),
        Expr::PostInc(inner) => Ok(Expr::PostInc(Box::new(resolve_expr(inner, scopes)?))),
        Expr::PostDec(inner) => Ok(Expr::PostDec(Box::new(resolve_expr(inner, scopes)?))),
        Expr::Assign { op, target, value } => Ok(Expr::Assign {
            op: *op,
            target: Box::new(resolve_expr(target, scopes)?),
            value: Box::new(resolve_expr(value, scopes)?),
        }),
        Expr::Conditional {
            condition,
            then_expr,
            else_expr,
        } => Ok(Expr::Conditional {
            condition: Box::new(resolve_expr(condition, scopes)?),
            then_expr: Box::new(resolve_expr(then_expr, scopes)?),
            else_expr: Box::new(resolve_expr(else_expr, scopes)?),
        }),
        Expr::Binary { op, left, right } => Ok(Expr::Binary {
            op: *op,
            left: Box::new(resolve_expr(left, scopes)?),
            right: Box::new(resolve_expr(right, scopes)?),
        }),
    }
}
