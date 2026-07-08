//! Identifier resolution pass.
//!
//! Mirrors `nqcc2/lib/semantic_analysis/resolve.ml`.  The pass:
//!
//! 1. Walks every function body in the translation unit, tracking a scope
//!    stack of declared variable names (including parameter names that
//!    are scoped to the function body).  Each declaration is inserted
//!    into the innermost scope and assigned a fresh unique name (e.g.
//!    `x` becomes `x.0`, `x.1`, ...).  A second declaration of the same
//!    name in the same scope is rejected.
//! 2. Visits every expression.  An `Expr::Var(name)` reference is resolved
//!    against the nearest enclosing scope (innermost first) and rewritten
//!    to use the unique name recorded for that declaration.  An undeclared
//!    reference is rejected.  A `Expr::Call(name, _)` reference is
//!    resolved against the global function table; an undeclared call is
//!    rejected and the argument count is checked against the callee's
//!    declared arity.  Any other expression form is recursively walked so
//!    nested variable references are picked up.
//!
//! The chapter-9 surface adds a **global function table** alongside the
//! per-function scope stack:
//!
//! - Each function definition adds an entry to the global table keyed
//!   by the function name, recording both the **arity** (parameter count)
//!   and that the function is *defined* (has a body).  A function
//!   declaration (`int foo(int x);`) adds an entry marked *declared only*.
//! - Top-level items are processed in source order so a function call
//!   inside an earlier function body must have a prior declaration or
//!   definition; later items are not yet visible (mirrors C's single-
//!   translation-unit visibility rule).
//! - A second definition of the same function name is rejected.  A
//!   second declaration with a matching arity is accepted; a second
//!   declaration with a different arity is rejected as a conflicting
//!   declaration.
//! - Function-call sites (`Expr::Call(name, ...)`) are resolved against
//!   the table; unknown names are rejected, and the argument count is
//!   compared against the recorded arity.
//! - Local function declarations inside a block (`int foo(int x);`)
//!   register the name and its arity in the per-block scope, where it
//!   conflicts with a same-scope variable of the same name (and vice
//!   versa).  Cross-scope shadowing is allowed (an inner block can
//!   re-declare a name that an outer block already declared).
//! - Parameter names are scoped to the function body in source order,
//!   and a parameter list with duplicate names is rejected.
//!
//! The chapter-7 extension over chapter-5 is the **per-block scope stack**
//! and the **shadowing / name-mangling** behaviour.  Each compound statement
//! (`{ ... }`) opens a fresh inner scope; declarations inside that block
//! get a fresh unique name so the outer binding is preserved.  A reference
//! to a name walks the scope stack from innermost to outermost, picking up
//! the closest declaration's unique name.  When the block exits, the inner
//! scope is popped and the outer binding is naturally visible again.
//!
//! The pass returns a cloned [`Program`] with all variable references
//! rewritten to use the unique names inside [`ResolvedProgram`]; the
//! reserved type alias keeps the downstream pipeline signature-stable.

use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};

use crate::ast::{
    BlockItem, Expr, ForInit, Function, GlobalDecl, GlobalVarDecl, Program, Statement, TopLevelItem,
    VarDecl,
};

/// Thin wrapper that carries a `Program` after resolution.
#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub program: Program,
}

/// A function entry in the global table: records the declared arity and
/// whether the function has been *defined* (has a body).  A function
/// declaration without a body sets `defined = false`; a subsequent
/// definition with the matching arity flips it to `true`.  A second
/// definition, or a declaration with a different arity, is rejected by
/// the resolve pass before the table is updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FunctionEntry {
    arity: usize,
    defined: bool,
}

type FunctionTable = HashMap<String, FunctionEntry>;

/// Resolve identifiers, declarations, and goto/label consistency (chapter 5
/// covers identifier resolution; chapter 7 extends the scope model to nested
/// blocks and produces a renamed AST so the lowerer naturally maps each
/// scope to its own stack slot; chapter 9 adds the global function table
/// with arity tracking and per-scope function declarations; chapter 10
/// adds a file-scope variable table that lets expressions inside a
/// function body read globals and rejects duplicate definitions).
///
/// Walks the translation unit in a single top-down pass: top-level items
/// are processed in source order, and the global function table grows as
/// each item is encountered.  A function body can therefore call only
/// functions that have been declared or defined earlier in the same
/// translation unit (matching C's single-pass visibility rule and the
/// behaviour of the canonical OCaml reference).
pub fn resolve_program(ast: &Program) -> Result<ResolvedProgram> {
    let mut table: FunctionTable = HashMap::new();
    let mut globals: HashMap<String, bool> = HashMap::new();
    let mut resolved_items: Vec<TopLevelItem> = Vec::with_capacity(ast.top_level_items.len());
    for item in &ast.top_level_items {
        match item {
            TopLevelItem::Function(func) => {
                let arity = func.params.len();
                check_function_conflict(&table, &func.name, arity)?;
                table.insert(
                    func.name.clone(),
                    FunctionEntry { arity, defined: true },
                );
                let resolved = resolve_function(func, &table, &globals)?;
                resolved_items.push(TopLevelItem::Function(resolved));
            }
            TopLevelItem::Declaration(decl) => {
                let arity = decl.params.len();
                check_function_conflict(&table, &decl.name, arity)?;
                table.insert(
                    decl.name.clone(),
                    FunctionEntry {
                        arity,
                        defined: false,
                    },
                );
                check_duplicate_params(&decl.params)?;
                resolved_items.push(TopLevelItem::Declaration(GlobalDecl {
                    name: decl.name.clone(),
                    params: decl.params.clone(),
                }));
            }
            TopLevelItem::Variable(var) => {
                resolve_global_variable(var, &mut globals)?;
                resolved_items.push(TopLevelItem::Variable(GlobalVarDecl {
                    name: var.name.clone(),
                    ty: var.ty.clone(),
                    init: var.init.clone(),
                    storage: var.storage,
                }));
            }
        }
    }
    Ok(ResolvedProgram {
        program: Program {
            top_level_items: resolved_items,
        },
    })
}

/// Reject adding `name` with `arity` to a table that already contains a
/// conflicting entry.  A previous *declaration* with the same arity is
/// allowed (multiple declarations are legal C); a previous *definition*
/// is rejected as a duplicate definition.  Any entry with a different
/// arity is rejected as a conflicting declaration.
fn check_function_conflict(
    table: &FunctionTable,
    name: &str,
    arity: usize,
) -> Result<()> {
    match table.get(name) {
        None => Ok(()),
        Some(entry) if entry.arity == arity && !entry.defined => Ok(()),
        Some(entry) if entry.arity == arity && entry.defined => bail!(
            "resolve error: duplicate definition of function '{name}'"
        ),
        Some(entry) => bail!(
            "resolve error: conflicting declaration of '{name}' (previous arity {}, new arity {arity})",
            entry.arity
        ),
    }
}

/// Reject a parameter list with duplicate names.  Mirrors the
/// `resolve_function` parameter check: parameters occupy the function's
/// root scope, and C requires all parameter names to be unique within
/// that scope (this is what `decl_params_with_same_name.c` exercises
/// against a *declaration*, not a definition).
fn check_duplicate_params(params: &[VarDecl]) -> Result<()> {
    let mut seen: HashSet<&str> = HashSet::with_capacity(params.len());
    for param in params {
        if !seen.insert(&param.name) {
            bail!(
                "resolve error: duplicate parameter name '{}' in function declaration",
                param.name
            );
        }
    }
    Ok(())
}

/// Resolve a file-scope variable declaration.  Mirrors
/// `resolve_file_scope_variable_declaration` in OCaml resolve.ml.
/// Multiple tentative declarations of the same name are merged by
/// the lowerer; only an *initialized* second declaration is
/// rejected.
fn resolve_global_variable(
    var: &GlobalVarDecl,
    globals: &mut HashMap<String, bool>,
) -> Result<()> {
    let has_init_now = var.init.is_some();
    if let Some(&previously_initialized) = globals.get(&var.name) {
        if previously_initialized && has_init_now {
            bail!(
                "resolve error: duplicate definition of file-scope variable '{}' (already initialized)",
                var.name
            );
        }
    }
    let resolved_init = match &var.init {
        Some(expr) => Some(resolve_global_init(expr)?),
        None => None,
    };
    if has_init_now {
        globals.insert(var.name.clone(), true);
    } else {
        globals.entry(var.name.clone()).or_insert(false);
    }
    let _ = resolved_init;
    Ok(())
}

/// File-scope variable initializers must be constant expressions
/// for chapter 10.
fn resolve_global_init(expr: &Expr) -> Result<Expr> {
    match expr {
        Expr::Constant(_) => Ok(expr.clone()),
        other => bail!(
            "resolve error: file-scope variable initializer must be a constant expression (got {other:?})"
        ),
    }
}

fn resolve_function(
    func: &Function,
    table: &FunctionTable,
    globals: &HashMap<String, bool>,
) -> Result<Function> {
    let mut scopes = ScopeStack::new();
    check_duplicate_params(&func.params)?;
    let mut resolved_params: Vec<VarDecl> = Vec::with_capacity(func.params.len());
    for param in &func.params {
        let unique = scopes.declare(&param.name)?;
        resolved_params.push(VarDecl {
            name: unique,
            init: None,
        });
    }
    let body = match &func.body {
        Some(items) => Some(resolve_block(items, &mut scopes, table, globals)?),
        None => None,
    };
    Ok(Function {
        name: func.name.clone(),
        params: resolved_params,
        body,
    })
}

/// A stack of declaration scopes, innermost-first.
///
/// `push` opens a new scope (e.g. on entering a compound statement or
/// a `for` init).  `pop` closes it.  `declare` inserts into the
/// innermost scope and returns a freshly minted unique name; `lookup`
/// walks outward through enclosing scopes and returns the unique name
/// recorded for the nearest declaration of the requested source name.
/// `declare_fun` records a function-prototype name and arity in the
/// innermost scope so subsequent variable declarations (and vice
/// versa) can detect a same-scope collision, and so a call site can
/// resolve the prototype's arity without consulting the global table.
///
/// Each scope carries two maps side by side: a `HashMap<source_name,
/// unique_name>` for variables (so a variable reference can be
/// resolved at a use site), and a `HashMap<source_name, arity>` for
/// function prototypes (their arity is needed to validate call-site
/// argument counts).  Cross-namespace collisions are detected by
/// checking both maps within the same scope.
///
/// The unique name uses a globally monotonic counter so `x.0` / `x.1`
/// / `x.2` / ... never collide across blocks, mirroring the OCaml
/// reference's `Unique_ids.make_named_temporary`.
#[derive(Debug, Default)]
struct ScopeStack {
    scopes: Vec<HashMap<String, String>>,
    fun_decls: Vec<HashMap<String, usize>>,
    next_id: u32,
}

impl ScopeStack {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            fun_decls: vec![HashMap::new()],
            next_id: 0,
        }
    }

    fn push(&mut self) {
        self.scopes.push(HashMap::new());
        self.fun_decls.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.scopes
            .pop()
            .expect("scope stack underflow (popped past root scope)");
        self.fun_decls
            .pop()
            .expect("scope stack underflow (popped past root scope)");
    }

    /// Insert `name` into the innermost scope and return a fresh unique
    /// name (`name.N`).  Rejects a duplicate declaration in the same
    /// scope and a same-scope collision with a function prototype —
    /// shadowing across nested scopes is allowed and is the reason the
    /// inner `HashMap` is keyed by the source name.
    fn declare(&mut self, name: &str) -> Result<String> {
        let fun_set = self
            .fun_decls
            .last()
            .expect("scope stack missing root scope");
        if fun_set.contains_key(name) {
            bail!(
                "resolve error: variable '{name}' conflicts with function declaration in the same scope"
            );
        }
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

    /// Record `name` as a function prototype (with the given arity) in
    /// the innermost scope.  Rejects a same-scope collision with a
    /// variable declaration.  A second prototype in the same scope is
    /// accepted when its arity matches the existing entry (multiple
    /// declarations of the same function are legal C, mirroring the
    /// OCaml reference's `has_linkage = true` re-declaration path);
    /// a second prototype with a different arity is rejected as a
    /// conflicting declaration.
    fn declare_fun(&mut self, name: &str, arity: usize) -> Result<()> {
        let var_scope = self
            .scopes
            .last()
            .expect("scope stack missing root scope");
        if var_scope.contains_key(name) {
            bail!(
                "resolve error: function '{name}' conflicts with variable declaration in the same scope"
            );
        }
        let fun_set = self
            .fun_decls
            .last_mut()
            .expect("scope stack missing root scope");
        match fun_set.get(name).copied() {
            Some(existing) if existing != arity => bail!(
                "resolve error: conflicting declaration of '{name}' (previous arity {existing}, new arity {arity})"
            ),
            Some(_) => {}
            None => {
                fun_set.insert(name.to_string(), arity);
            }
        }
        Ok(())
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

    /// Walk the per-scope function-prototype map innermost-first and
    /// return the arity of the nearest enclosing-scope prototype of
    /// `name`, or `None` if no scope declares it as a function.
    fn lookup_fun_arity(&self, name: &str) -> Option<usize> {
        self.fun_decls
            .iter()
            .rev()
            .find_map(|set| set.get(name).copied())
    }
}

fn resolve_block(
    items: &[BlockItem],
    scopes: &mut ScopeStack,
    table: &FunctionTable,
    globals: &HashMap<String, bool>,
) -> Result<Vec<BlockItem>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(resolve_block_item(item, scopes, table, globals)?);
    }
    Ok(out)
}

fn resolve_block_item(
    item: &BlockItem,
    scopes: &mut ScopeStack,
    table: &FunctionTable,
    globals: &HashMap<String, bool>,
) -> Result<BlockItem> {
    match item {
        BlockItem::Statement(stmt) => Ok(BlockItem::Statement(resolve_statement(
            stmt, scopes, table, globals,
        )?)),
        BlockItem::Declaration(var_decl) => {
            // C99 rule: the declarator's scope begins at the end of the
            // declarator, so `int a = a + 1` references the new `a` —
            // declare first, then resolve the init against the new
            // scope.  This matches the OCaml `resolve_local_var_helper`
            // ordering.
            let new_name = scopes.declare(&var_decl.name)?;
            let new_init = match &var_decl.init {
                Some(expr) => Some(resolve_expr(expr, scopes, table, globals)?),
                None => None,
            };
            Ok(BlockItem::Declaration(VarDecl {
                name: new_name,
                init: new_init,
            }))
        }
        BlockItem::FunctionDecl(fd) => {
            check_duplicate_params(&fd.params)?;
            scopes.declare_fun(&fd.name, fd.params.len())?;
            Ok(BlockItem::FunctionDecl(GlobalDecl {
                name: fd.name.clone(),
                params: fd.params.clone(),
            }))
        }
    }
}

fn resolve_for_init(
    init: &ForInit,
    scopes: &mut ScopeStack,
    table: &FunctionTable,
    globals: &HashMap<String, bool>,
) -> Result<ForInit> {
    match init {
        ForInit::Declaration(var_decl) => {
            let new_name = scopes.declare(&var_decl.name)?;
            let new_init = match &var_decl.init {
                Some(expr) => Some(resolve_expr(expr, scopes, table, globals)?),
                None => None,
            };
            Ok(ForInit::Declaration(VarDecl {
                name: new_name,
                init: new_init,
            }))
        }
        ForInit::Expr(expr) => Ok(ForInit::Expr(resolve_expr(expr, scopes, table, globals)?)),
    }
}

fn resolve_statement(
    stmt: &Statement,
    scopes: &mut ScopeStack,
    table: &FunctionTable,
    globals: &HashMap<String, bool>,
) -> Result<Statement> {
    match stmt {
        Statement::Return(expr) => {
            Ok(Statement::Return(resolve_expr(expr, scopes, table, globals)?))
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => Ok(Statement::If {
            condition: resolve_expr(condition, scopes, table, globals)?,
            then_branch: Box::new(resolve_statement(then_branch, scopes, table, globals)?),
            else_branch: match else_branch {
                Some(else_branch) => {
                    Some(Box::new(resolve_statement(else_branch, scopes, table, globals)?))
                }
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
            let result = resolve_block(items, scopes, table, globals);
            scopes.pop();
            Ok(Statement::Block(result?))
        }
        Statement::While {
            condition,
            body,
            label,
        } => Ok(Statement::While {
            condition: resolve_expr(condition, scopes, table, globals)?,
            body: Box::new(resolve_statement(body, scopes, table, globals)?),
            label: label.clone(),
        }),
        Statement::DoWhile {
            body,
            condition,
            label,
        } => Ok(Statement::DoWhile {
            body: Box::new(resolve_statement(body, scopes, table, globals)?),
            condition: resolve_expr(condition, scopes, table, globals)?,
            label: label.clone(),
        }),
        Statement::For {
            init,
            condition,
            post,
            body,
            label,
        } => {
            scopes.push();
            let resolved_init = match init {
                Some(init) => Some(resolve_for_init(init, scopes, table, globals)?),
                None => None,
            };
            let resolved_condition = match condition {
                Some(expr) => Some(resolve_expr(expr, scopes, table, globals)?),
                None => None,
            };
            let resolved_post = match post {
                Some(expr) => Some(resolve_expr(expr, scopes, table, globals)?),
                None => None,
            };
            let body_result = resolve_statement(body, scopes, table, globals);
            scopes.pop();
            Ok(Statement::For {
                init: resolved_init,
                condition: resolved_condition,
                post: resolved_post,
                body: Box::new(body_result?),
                label: label.clone(),
            })
        }
        Statement::Break(target) => Ok(Statement::Break(target.clone())),
        Statement::Continue(target) => Ok(Statement::Continue(target.clone())),
        Statement::Switch { expr, body, label } => Ok(Statement::Switch {
            expr: resolve_expr(expr, scopes, table, globals)?,
            body: Box::new(resolve_statement(body, scopes, table, globals)?),
            label: label.clone(),
        }),
        Statement::Case { value, statement } => Ok(Statement::Case {
            value: resolve_expr(value, scopes, table, globals)?,
            statement: Box::new(resolve_statement(statement, scopes, table, globals)?),
        }),
        Statement::Default { statement } => Ok(Statement::Default {
            statement: Box::new(resolve_statement(statement, scopes, table, globals)?),
        }),
        Statement::Goto(target) => Ok(Statement::Goto(target.clone())),
        Statement::Labeled { label, statement } => Ok(Statement::Labeled {
            label: label.clone(),
            statement: Box::new(resolve_statement(statement, scopes, table, globals)?),
        }),
        Statement::Expr(maybe_expr) => {
            let resolved = match maybe_expr {
                Some(expr) => Some(resolve_expr(expr, scopes, table, globals)?),
                None => None,
            };
            Ok(Statement::Expr(resolved))
        }
    }
}

fn resolve_expr(
    expr: &Expr,
    scopes: &mut ScopeStack,
    table: &FunctionTable,
    globals: &HashMap<String, bool>,
) -> Result<Expr> {
    match expr {
        Expr::Constant(n) => Ok(Expr::Constant(*n)),
        Expr::Paren(inner) => {
            Ok(Expr::Paren(Box::new(resolve_expr(inner, scopes, table, globals)?)))
        }
        Expr::Var(name) => {
            // Locals shadow globals; an undeclared local reference
            // then falls back to the file-scope variable set so
            // `return g;` inside a function can read `int g = 5;`
            // declared earlier in the translation unit.
            if let Some(unique) = scopes.lookup(name) {
                return Ok(Expr::Var(unique));
            }
            if globals.contains_key(name) {
                return Ok(Expr::Var(name.clone()));
            }
            bail!("resolve error: undeclared variable '{name}'")
        }
        Expr::Call { name, args } => {
            let arity = if let Some(arity) = scopes.lookup_fun_arity(name) {
                arity
            } else if let Some(entry) = table.get(name) {
                entry.arity
            } else {
                bail!("resolve error: call to undeclared function '{name}'")
            };
            if args.len() != arity {
                bail!(
                    "resolve error: function '{name}' called with {} argument(s) but declared with {arity}",
                    args.len()
                );
            }
            let resolved_args = args
                .iter()
                .map(|a| resolve_expr(a, scopes, table, globals))
                .collect::<Result<Vec<_>>>()?;
            Ok(Expr::Call {
                name: name.clone(),
                args: resolved_args,
            })
        }
        Expr::Unary { op, expr: inner } => Ok(Expr::Unary {
            op: *op,
            expr: Box::new(resolve_expr(inner, scopes, table, globals)?),
        }),
        Expr::PreInc(inner) => Ok(Expr::PreInc(Box::new(resolve_expr(inner, scopes, table, globals)?))),
        Expr::PreDec(inner) => Ok(Expr::PreDec(Box::new(resolve_expr(inner, scopes, table, globals)?))),
        Expr::PostInc(inner) => Ok(Expr::PostInc(Box::new(resolve_expr(inner, scopes, table, globals)?))),
        Expr::PostDec(inner) => Ok(Expr::PostDec(Box::new(resolve_expr(inner, scopes, table, globals)?))),
        Expr::Assign { op, target, value } => Ok(Expr::Assign {
            op: *op,
            target: Box::new(resolve_expr(target, scopes, table, globals)?),
            value: Box::new(resolve_expr(value, scopes, table, globals)?),
        }),
        Expr::Conditional {
            condition,
            then_expr,
            else_expr,
        } => Ok(Expr::Conditional {
            condition: Box::new(resolve_expr(condition, scopes, table, globals)?),
            then_expr: Box::new(resolve_expr(then_expr, scopes, table, globals)?),
            else_expr: Box::new(resolve_expr(else_expr, scopes, table, globals)?),
        }),
        Expr::Binary { op, left, right } => Ok(Expr::Binary {
            op: *op,
            left: Box::new(resolve_expr(left, scopes, table, globals)?),
            right: Box::new(resolve_expr(right, scopes, table, globals)?),
        }),
    }
}