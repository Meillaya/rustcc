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
    BlockItem, Expr, ForInit, Function, GlobalDecl, GlobalVarDecl, Program, Statement,
    StorageClass, TopLevelItem, VarDecl,
};

/// Thin wrapper that carries a `Program` after resolution.
#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub program: Program,
}

/// Linkage of a global symbol, per C17 6.2.2.  Mirrors the OCaml
/// `has_linkage` + `storage_class` discrimination in
/// `nqcc2/lib/semantic_analysis/resolve.ml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Linkage {
    External,
    Internal,
}

/// Kind of a global declaration — function or variable.  Mirrors the
/// two arms of `resolve_global_declaration` in OCaml.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlobalKind {
    Function { arity: usize, defined: bool },
    Variable,
}

/// One row in the file-scope symbol table.  The row remembers the kind
/// (function/variable) and the linkage (external/internal) so a second
/// declaration can detect a kind or linkage conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GlobalEntry {
    kind: GlobalKind,
    linkage: Linkage,
}

type GlobalTable = HashMap<String, GlobalEntry>;

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
    let mut globals: GlobalTable = HashMap::new();
    let mut resolved_items: Vec<TopLevelItem> = Vec::with_capacity(ast.top_level_items.len());
    for item in &ast.top_level_items {
        match item {
            TopLevelItem::Function(func) => {
                check_function_conflict(
                    &globals,
                    &func.name,
                    func.params.len(),
                    true,
                    linkage_of(func.storage),
                )?;
                globals.insert(
                    func.name.clone(),
                    GlobalEntry {
                        kind: GlobalKind::Function {
                            arity: func.params.len(),
                            defined: true,
                        },
                        linkage: linkage_of(func.storage),
                    },
                );
                let resolved = resolve_function(func, &globals)?;
                resolved_items.push(TopLevelItem::Function(resolved));
            }
            TopLevelItem::Declaration(decl) => {
                let arity = decl.params.len();
                check_function_conflict(
                    &globals,
                    &decl.name,
                    arity,
                    false,
                    linkage_of(decl.storage),
                )?;
                check_duplicate_params(&decl.params)?;
                globals.insert(
                    decl.name.clone(),
                    GlobalEntry {
                        kind: GlobalKind::Function {
                            arity,
                            defined: false,
                        },
                        linkage: linkage_of(decl.storage),
                    },
                );
                resolved_items.push(TopLevelItem::Declaration(GlobalDecl {
                    name: decl.name.clone(),
                    ret_ty: decl.ret_ty.clone(),
                    params: decl.params.clone(),
                    storage: decl.storage,
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

fn linkage_of(storage: StorageClass) -> Linkage {
    match storage {
        StorageClass::Static => Linkage::Internal,
        StorageClass::Extern | StorageClass::Auto => Linkage::External,
    }
}

/// Reject adding `name` (with `arity`, `defined`, `linkage`) to a table
/// that already contains a conflicting entry.  Mirrors the OCaml
/// `resolve_function_declaration` rules plus the chapter-10 extension
/// that detects kind (function vs variable) and linkage (external vs
/// internal) conflicts.
fn check_function_conflict(
    table: &GlobalTable,
    name: &str,
    arity: usize,
    defined: bool,
    linkage: Linkage,
) -> Result<()> {
    match table.get(name) {
        None => Ok(()),
        Some(entry) => {
            // Mismatched kind (function vs variable) is always an error.
            if !matches!(entry.kind, GlobalKind::Function { .. }) {
                bail!(
                    "resolve error: conflicting declarations of '{name}' (function and variable)"
                );
            }
            // Mismatched linkage (external vs internal) is always an error.
            if entry.linkage != linkage {
                bail!(
                    "resolve error: conflicting linkage for '{name}' (function declared with both external and internal linkage)"
                );
            }
            let entry_arity = match entry.kind {
                GlobalKind::Function { arity, .. } => arity,
                _ => unreachable!(),
            };
            let entry_defined = match entry.kind {
                GlobalKind::Function { defined, .. } => defined,
                _ => unreachable!(),
            };
            if entry_arity == arity && !entry_defined {
                Ok(())
            } else if entry_arity == arity && entry_defined && defined {
                bail!("resolve error: duplicate definition of function '{name}'")
            } else {
                bail!(
                    "resolve error: conflicting declaration of '{name}' (previous arity {entry_arity}, new arity {arity})"
                )
            }
        }
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
/// Rejects kind / linkage conflicts with a function of the same
/// name and linkage conflicts with a variable of the same name.
/// The chapter-10 multi-declaration (tentative-definition) merging
/// is handled in the lowerer; the resolve pass only blocks
/// pathological cases.
fn resolve_global_variable(
    var: &GlobalVarDecl,
    globals: &mut GlobalTable,
) -> Result<()> {
    let new_linkage = linkage_of(var.storage);
    if let Some(entry) = globals.get(&var.name).copied() {
        match entry.kind {
            GlobalKind::Function { .. } => bail!(
                "resolve error: conflicting declarations of '{name}' (function and variable)",
                name = var.name
            ),
            GlobalKind::Variable => {
                if entry.linkage != new_linkage {
                    bail!(
                        "resolve error: conflicting linkage for '{name}' (variable declared with both external and internal linkage)",
                        name = var.name
                    );
                }
            }
        }
    }
    let resolved_init = match &var.init {
        Some(expr) => Some(resolve_global_init(expr)?),
        None => None,
    };
    globals.insert(
        var.name.clone(),
        GlobalEntry {
            kind: GlobalKind::Variable,
            linkage: new_linkage,
        },
    );
    let _ = resolved_init;
    Ok(())
}

/// File-scope variable initializers must be constant expressions
/// for chapter 10.
fn resolve_global_init(expr: &Expr) -> Result<Expr> {
    match expr {
        Expr::Constant(_) | Expr::LongConstant(_) => Ok(expr.clone()),
        other => bail!(
            "resolve error: file-scope variable initializer must be a constant expression (got {other:?})"
        ),
    }
}

fn resolve_function(
    func: &Function,
    globals: &GlobalTable,
) -> Result<Function> {
    let mut scopes = ScopeStack::new();
    check_duplicate_params(&func.params)?;
    let mut resolved_params: Vec<VarDecl> = Vec::with_capacity(func.params.len());
    for param in &func.params {
        let unique = scopes.declare(&param.name)?;
        resolved_params.push(VarDecl {
            name: unique,
            ty: param.ty.clone(),
            init: None,
            storage: StorageClass::Auto,
        });
    }
    let body = match &func.body {
        Some(items) => Some(resolve_block(items, &mut scopes, globals)?),
        None => None,
    };
    Ok(Function {
        name: func.name.clone(),
        ret_ty: func.ret_ty.clone(),
        params: resolved_params,
        body,
        storage: func.storage,
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

    /// Insert a block-scope `extern` reference.  The unique name is
    /// the source name (no mangling) so the variable resolves to the
    /// file-scope symbol of the same name.  Mirrors the OCaml
    /// `has_linkage = true` path in `resolve_local_var_helper`.
    fn declare_extern(&mut self, name: &str) -> Result<String> {
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
        // Multiple `extern int x;` in the same block are allowed; a
        // second declaration just re-asserts the same linkage.
        if current.contains_key(name) {
            return Ok(name.to_string());
        }
        current.insert(name.to_string(), name.to_string());
        Ok(name.to_string())
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
    globals: &GlobalTable,
) -> Result<Vec<BlockItem>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(resolve_block_item(item, scopes, globals)?);
    }
    Ok(out)
}

fn resolve_block_item(
    item: &BlockItem,
    scopes: &mut ScopeStack,
    globals: &GlobalTable,
) -> Result<BlockItem> {
    match item {
        BlockItem::Statement(stmt) => Ok(BlockItem::Statement(resolve_statement(
            stmt, scopes, globals,
        )?)),
        BlockItem::Declaration(var_decl) => match var_decl.storage {
            StorageClass::Extern => {
                if var_decl.init.is_some() {
                    bail!(
                        "resolve error: extern variable '{name}' may not have an initializer",
                        name = var_decl.name
                    );
                }
                if !matches!(globals.get(&var_decl.name), Some(GlobalEntry { kind: GlobalKind::Variable, .. })) {
                    bail!(
                        "resolve error: extern declaration of '{name}' has no prior file-scope variable",
                        name = var_decl.name
                    );
                }
                let name = scopes.declare_extern(&var_decl.name)?;
                Ok(BlockItem::Declaration(VarDecl {
                    name,
                    ty: var_decl.ty.clone(),
                    init: None,
                    storage: StorageClass::Extern,
                }))
            }
            StorageClass::Static | StorageClass::Auto => {
                let new_name = scopes.declare(&var_decl.name)?;
                let new_init = match &var_decl.init {
                    Some(expr) => Some(resolve_expr(expr, scopes, globals)?),
                    None => None,
                };
                Ok(BlockItem::Declaration(VarDecl {
                    name: new_name,
                    ty: var_decl.ty.clone(),
                    init: new_init,
                    storage: var_decl.storage,
                }))
            }
        },
        BlockItem::FunctionDecl(fd) => {
            check_duplicate_params(&fd.params)?;
            if fd.storage == StorageClass::Static {
                bail!(
                    "resolve error: static keyword not allowed on local function declaration"
                );
            }
            if let Some(entry) = globals.get(&fd.name) {
                if !matches!(entry.kind, GlobalKind::Function { .. }) {
                    bail!(
                        "resolve error: conflicting declarations of '{name}' (function and variable)",
                        name = fd.name
                    );
                }
            }
            scopes.declare_fun(&fd.name, fd.params.len())?;
            Ok(BlockItem::FunctionDecl(GlobalDecl {
                name: fd.name.clone(),
                ret_ty: fd.ret_ty.clone(),
                params: fd.params.clone(),
                storage: fd.storage,
            }))
        }
    }
}

fn resolve_for_init(
    init: &ForInit,
    scopes: &mut ScopeStack,
    globals: &GlobalTable,
) -> Result<ForInit> {
    match init {
        ForInit::Declaration(var_decl) => {
            let new_name = scopes.declare(&var_decl.name)?;
            let new_init = match &var_decl.init {
                Some(expr) => Some(resolve_expr(expr, scopes, globals)?),
                None => None,
            };
            Ok(ForInit::Declaration(VarDecl {
                name: new_name,
                ty: var_decl.ty.clone(),
                init: new_init,
                storage: var_decl.storage,
            }))
        }
        ForInit::Expr(expr) => Ok(ForInit::Expr(resolve_expr(expr, scopes, globals)?)),
    }
}

fn resolve_statement(
    stmt: &Statement,
    scopes: &mut ScopeStack,
    globals: &GlobalTable,
) -> Result<Statement> {
    match stmt {
        Statement::Return(expr) => {
            Ok(Statement::Return(resolve_expr(expr, scopes, globals)?))
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => Ok(Statement::If {
            condition: resolve_expr(condition, scopes, globals)?,
            then_branch: Box::new(resolve_statement(then_branch, scopes, globals)?),
            else_branch: match else_branch {
                Some(else_branch) => {
                    Some(Box::new(resolve_statement(else_branch, scopes, globals)?))
                }
                None => None,
            },
        }),
        Statement::Block(items) => {
            scopes.push();
            let result = resolve_block(items, scopes, globals);
            scopes.pop();
            Ok(Statement::Block(result?))
        }
        Statement::While {
            condition,
            body,
            label,
        } => Ok(Statement::While {
            condition: resolve_expr(condition, scopes, globals)?,
            body: Box::new(resolve_statement(body, scopes, globals)?),
            label: label.clone(),
        }),
        Statement::DoWhile {
            body,
            condition,
            label,
        } => Ok(Statement::DoWhile {
            body: Box::new(resolve_statement(body, scopes, globals)?),
            condition: resolve_expr(condition, scopes, globals)?,
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
                Some(init) => Some(resolve_for_init(init, scopes, globals)?),
                None => None,
            };
            let resolved_condition = match condition {
                Some(expr) => Some(resolve_expr(expr, scopes, globals)?),
                None => None,
            };
            let resolved_post = match post {
                Some(expr) => Some(resolve_expr(expr, scopes, globals)?),
                None => None,
            };
            let body_result = resolve_statement(body, scopes, globals);
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
            expr: resolve_expr(expr, scopes, globals)?,
            body: Box::new(resolve_statement(body, scopes, globals)?),
            label: label.clone(),
        }),
        Statement::Case { value, statement } => Ok(Statement::Case {
            value: resolve_expr(value, scopes, globals)?,
            statement: Box::new(resolve_statement(statement, scopes, globals)?),
        }),
        Statement::Default { statement } => Ok(Statement::Default {
            statement: Box::new(resolve_statement(statement, scopes, globals)?),
        }),
        Statement::Goto(target) => Ok(Statement::Goto(target.clone())),
        Statement::Labeled { label, statement } => Ok(Statement::Labeled {
            label: label.clone(),
            statement: Box::new(resolve_statement(statement, scopes, globals)?),
        }),
        Statement::Expr(maybe_expr) => {
            let resolved = match maybe_expr {
                Some(expr) => Some(resolve_expr(expr, scopes, globals)?),
                None => None,
            };
            Ok(Statement::Expr(resolved))
        }
    }
}

fn resolve_expr(
    expr: &Expr,
    scopes: &mut ScopeStack,
    globals: &GlobalTable,
) -> Result<Expr> {
    match expr {
        Expr::Constant(n) => Ok(Expr::Constant(*n)),
        Expr::LongConstant(n) => Ok(Expr::LongConstant(*n)),
        Expr::UIntConstant(n, _) => Ok(Expr::UIntConstant(*n, false)),
        Expr::Cast { target_type, expr: inner } => Ok(Expr::Cast {
            target_type: target_type.clone(),
            expr: Box::new(resolve_expr(inner, scopes, globals)?),
        }),
        Expr::Paren(inner) => {
            Ok(Expr::Paren(Box::new(resolve_expr(inner, scopes, globals)?)))
        }
        Expr::Var(name) => {
            if let Some(unique) = scopes.lookup(name) {
                return Ok(Expr::Var(unique));
            }
            if matches!(
                globals.get(name),
                Some(GlobalEntry { kind: GlobalKind::Variable, .. })
            ) {
                return Ok(Expr::Var(name.clone()));
            }
            bail!("resolve error: undeclared variable '{name}'")
        }
        Expr::Call { name, args } => {
            let arity = if let Some(arity) = scopes.lookup_fun_arity(name) {
                arity
            } else if let Some(GlobalEntry {
                kind: GlobalKind::Function { arity, .. },
                ..
            }) = globals.get(name)
            {
                *arity
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
                .map(|a| resolve_expr(a, scopes, globals))
                .collect::<Result<Vec<_>>>()?;
            Ok(Expr::Call {
                name: name.clone(),
                args: resolved_args,
            })
        }
        Expr::Unary { op, expr: inner } => Ok(Expr::Unary {
            op: *op,
            expr: Box::new(resolve_expr(inner, scopes, globals)?),
        }),
        Expr::PreInc(inner) => Ok(Expr::PreInc(Box::new(resolve_expr(inner, scopes, globals)?))),
        Expr::PreDec(inner) => Ok(Expr::PreDec(Box::new(resolve_expr(inner, scopes, globals)?))),
        Expr::PostInc(inner) => Ok(Expr::PostInc(Box::new(resolve_expr(inner, scopes, globals)?))),
        Expr::PostDec(inner) => Ok(Expr::PostDec(Box::new(resolve_expr(inner, scopes, globals)?))),
        Expr::Assign { op, target, value } => Ok(Expr::Assign {
            op: *op,
            target: Box::new(resolve_expr(target, scopes, globals)?),
            value: Box::new(resolve_expr(value, scopes, globals)?),
        }),
        Expr::Conditional {
            condition,
            then_expr,
            else_expr,
        } => Ok(Expr::Conditional {
            condition: Box::new(resolve_expr(condition, scopes, globals)?),
            then_expr: Box::new(resolve_expr(then_expr, scopes, globals)?),
            else_expr: Box::new(resolve_expr(else_expr, scopes, globals)?),
        }),
        Expr::Binary { op, left, right } => Ok(Expr::Binary {
            op: *op,
            left: Box::new(resolve_expr(left, scopes, globals)?),
            right: Box::new(resolve_expr(right, scopes, globals)?),
        }),
    }
}