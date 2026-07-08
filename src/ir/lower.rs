//! AST-to-TACKY lowering.
//!
//! Mirrors `nqcc2/lib/tacky_gen.ml`.  Through chapter 9:
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
//! - Chapter 9 widens the entry point from a single function to a
//!   translation unit: `lower_program` now iterates over every
//!   `TopLevelItem::Function` in the AST and emits one
//!   `TackyFunction` per definition (forward declarations without a
//!   body produce no TACKY).  Each function carries the resolved
//!   parameter names so the codegen pass can route incoming register
//!   arguments to the matching stack slots.

use anyhow::Result;
use std::collections::HashMap;

use crate::ast::{
    AssignOp, BinaryOp, BlockItem, Expr, ForInit, GlobalVarDecl, Program, Statement, StorageClass,
    TopLevelItem, Type, UnaryOp,
};
use crate::ir::tacky::{
    ConditionCode, Instruction, OperandType, TackyFunction, TackyProgram, TackyStaticInit,
    TackyStaticVariable, TypeEnv, Val,
};
use crate::ir::temp::TempIdGenerator;
use crate::util::labels::LabelGenerator;

pub type TypedProgram = Program;

/// Map the AST's `Type` to the TACKY `OperandType`.  Only the
/// chapter-11 surface is mapped; everything else falls back to `Int`
/// so the existing chapter-1..10 codegen keeps compiling.
fn type_to_operand_type(ty: Type) -> OperandType {
    match ty {
        Type::Long | Type::UnsignedLong => OperandType::Long,
        _ => OperandType::Int,
    }
}

pub fn lower_program(ast: &TypedProgram) -> Result<TackyProgram> {
    let mut ctx = LowerCtx::new();
    let mut functions: Vec<TackyFunction> = Vec::new();
    let mut globals: HashMap<
        String,
        (StorageClass, Option<Expr>, crate::ir::tacky::OperandType),
    > = HashMap::new();
    // Two-pass walk: gather file-scope variables first so each
    // function can seed its type env with their declared widths.
    for item in &ast.top_level_items {
        if let TopLevelItem::Variable(var) = item {
            merge_global_decl(var, &mut globals);
        }
    }
    // Chapter 11: also seed the function signature table so
    // `lower_call` can convert arguments to the parameter's
    // declared width (SignExtend for int -> long, Truncate for
    // long -> int).  Forward declarations and full definitions
    // both count.
    for item in &ast.top_level_items {
        let (name, params): (String, Vec<crate::ast::VarDecl>) = match item {
            TopLevelItem::Function(f) => (f.name.clone(), f.params.clone()),
            TopLevelItem::Declaration(d) => (d.name.clone(), d.params.clone()),
            _ => continue,
        };
        let param_tys: Vec<_> = params
            .iter()
            .map(|p| type_to_operand_type(p.ty))
            .collect();
        ctx.func_sigs.insert(name, param_tys);
    }
    for item in &ast.top_level_items {
        match item {
            TopLevelItem::Function(func) => {
                if let Some(body_items) = &func.body {
                    let params: Vec<String> = func.params.iter().map(|p| p.name.clone()).collect();
                    ctx.user_labels.clear();
                    ctx.user_label_counter = 0;
                    ctx.current_function = Some(func.name.clone());
                    ctx.type_env.clear();
                    ctx.const_counter = 0;
                    // Seed the env with the parameter types so the
                    // body can refer to each parameter by its declared
                    // width.
                    for param in &func.params {
                        ctx.type_env.insert(
                            param.name.clone(),
                            type_to_operand_type(param.ty),
                        );
                    }
                    // Also seed the env with file-scope variable
                    // types so `Copy` / `Add` / etc. of a `long`
                    // global pick the 64-bit instruction width.
                    for (gname, (_sc, _init, gty)) in &globals {
                        ctx.type_env.entry(gname.clone()).or_insert(*gty);
                    }
                    let body = lower_block_items(body_items, &mut ctx)?;
                    let body = ensure_trailing_return(body);
                    ctx.current_function = None;
                    let type_env = std::mem::take(&mut ctx.type_env);
                    functions.push(TackyFunction {
                        name: func.name.clone(),
                        global: true,
                        params,
                        body,
                        type_env,
                    });
                }
            }
            TopLevelItem::Declaration(_) => {
                // Forward declarations produce no TACKY; resolve already
                // recorded the name in the global function table.
            }
            TopLevelItem::Variable(_) => {
                // Already collected in the first pass.
            }
        }
    }
    let mut static_variables: Vec<TackyStaticVariable> = globals
        .into_iter()
        .filter_map(|(name, (storage, init, ty))| {
            // Chapter 9 / 10: an `extern` declaration at file scope
            // is a *reference* to a definition provided elsewhere; it
            // must NOT emit its own `.data` / `.bss` entry or the
            // linker will see a duplicate symbol against the real
            // definition.  Skip it here — the resolve pass already
            // recorded the name in the global table so subsequent
            // references still resolve.
            if storage == StorageClass::Extern {
                return None;
            }
            let global = matches!(storage, StorageClass::Auto);
            let init = match init {
                Some(Expr::Constant(n)) => TackyStaticInit::Int(n),
                Some(Expr::LongConstant(n)) => TackyStaticInit::Long(n),
                None => TackyStaticInit::Int(0),
                // Non-constant initializers rejected by resolve pass.
                Some(_) => TackyStaticInit::Int(0),
            };
            Some(TackyStaticVariable {
                name,
                init,
                global,
                ty,
            })
        })
        .collect();
    // Emit static variables in source order so tests read globals in
    // the order the human reader sees them.
    static_variables.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(TackyProgram {
        functions,
        static_variables,
    })
}

/// Merge a file-scope variable declaration into the chapter-10
/// global table.  Multiple tentative declarations of the same name
/// are folded into a single entry; the merge keeps the *first*
/// storage class and only adopts an initializer if the existing
/// entry has none.  Mirrors the OCaml `Hashtbl.replace`-style
/// dedup in `nqcc2/lib/symbols.ml`.
fn merge_global_decl(
    var: &GlobalVarDecl,
    globals: &mut HashMap<String, (StorageClass, Option<Expr>, crate::ir::tacky::OperandType)>,
) {
    let entry = globals
        .entry(var.name.clone())
        .or_insert((var.storage, var.init.clone(), type_to_operand_type(var.ty)));
    if entry.1.is_none() && var.init.is_some() {
        entry.0 = var.storage;
        entry.1 = var.init.clone();
    }
}

/// Namespace-prefix user-defined labels so they cannot collide with
/// function names (`main:`) or with the auto-generated labels
/// (`if_end.0`, `while_cond.3`, ...).  The chapter-6 `--goto` extra
/// makes the conflict observable: the assembly emitter writes a
/// top-level `<name>:` for every TACKY `Label(name)`, so leaving a
/// user `main:` label unmangled would shadow the function entry
/// symbol.  Chapter 9 also disambiguates across functions: the same
/// source label `foo:` in two different functions compiles to two
/// different assembly labels (the function name is part of the
/// prefix) so the linker sees distinct symbols even when the
/// counter would otherwise collide across functions.
///
/// Repeated occurrences of the same source label within a single
/// function return the same assembly name so the corresponding
/// `goto` and `label:` instructions pair up correctly.
fn mangle_user_label(
    name: &str,
    counter: &mut u32,
    cache: &mut HashMap<String, String>,
    function: Option<&str>,
) -> String {
    if let Some(existing) = cache.get(name) {
        return existing.clone();
    }
    let id = *counter;
    *counter += 1;
    // The function-name component is required so the same source
    // label in two different functions never collapses to the same
    // assembly symbol.  Without it, the chapter-9
    // `label_naming_scheme` test (which deliberately picks names
    // that would collide under naive prefixing) hits a duplicate
    // symbol error at link time.
    let mangled = match function {
        Some(func) => format!("user_label.{func}.{name}.{id}"),
        None => format!("user_label.{name}.{id}"),
    };
    cache.insert(name.to_string(), mangled.clone());
    mangled
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
    /// Map from `case value -> dispatch label` for the switch
    /// currently being lowered.  Set by `lower_switch` before
    /// lowering the switch body and cleared afterwards so that
    /// any `Case` nodes encountered inside the body emit the
    /// matching `Label` instruction.  Supports Duff's device
    /// where `case` labels are nested inside loops / other
    /// constructs — every `Case` value gets exactly one entry
    /// here, and `lower_statement` looks it up by value.
    case_labels: Option<HashMap<i64, String>>,
    /// Dispatch label for `default:` of the switch currently
    /// being lowered, if any.  Same nesting semantics as
    /// `case_labels`.
    default_label: Option<String>,
    /// Per-function counter that namespaces user labels so the
    /// same source label name in two different functions
    /// compiles to two distinct assembly labels.  Reset to zero
    /// at the start of every function in `lower_program`.
    user_label_counter: u32,
    /// Per-function cache mapping source label names to their
    /// mangled assembly names so repeated `goto <name>` and the
    /// matching `<name>:` use the same assembly symbol.  Cleared
    /// at the start of every function.
    user_labels: HashMap<String, String>,
    /// Name of the function currently being lowered; used as a
    /// prefix on user-label assembly symbols so cross-function
    /// label name collisions never reach the linker.
    current_function: Option<String>,
    /// Per-function type env tracking the operand width of every
    /// TACKY variable the lowerer creates.  Populated alongside
    /// each `Copy` to a fresh tmp and consulted by the binary /
    /// unary / return codegen paths to decide between 32-bit and
    /// 64-bit instructions.  Emitted with the function so the
    /// codegen pass can look up types without re-walking the AST.
    type_env: TypeEnv,
    /// Monotonic counter for the synthetic names used to materialise
    /// long-typed constant values into the IR (each long constant
    /// gets a fresh `const.<n>` name and a matching env entry).
    const_counter: u32,
    /// Chapter 11: program-wide function signature table mapping
    /// `f -> [param_type, ...]`.  Populated from the
    /// `TopLevelItem::Function` / `Declaration` items before any
    /// function body is lowered; consulted by `lower_call` to
    /// widen / narrow arguments to the parameter's declared
    /// type.  Mirrors the OCaml `Symbols` table of parameter
    /// types used by `typecheck.ml::typecheck_fun_call`.
    func_sigs: HashMap<String, Vec<crate::ir::tacky::OperandType>>,
}

impl LowerCtx {
    fn new() -> Self {
        Self {
            temps: TempIdGenerator::new(),
            labels: LabelGenerator::new(),
            case_labels: None,
            default_label: None,
            user_label_counter: 0,
            user_labels: HashMap::new(),
            current_function: None,
            type_env: HashMap::new(),
            const_counter: 0,
            func_sigs: HashMap::new(),
        }
    }

    fn fresh_tmp(&mut self) -> String {
        format!("tmp.{}", self.temps.next().0)
    }

    fn fresh_typed_tmp(&mut self, ty: OperandType) -> String {
        let name = self.fresh_tmp();
        self.type_env.insert(name.clone(), ty);
        name
    }

    /// Materialise a long-typed constant into the IR.  Emits a
    /// `Copy` from the inline `Val::Constant` to a fresh synthetic
    /// name and records the name's type in `type_env`.  The caller
    /// receives the synthetic name as a `Val::Var` so downstream
    /// uses can look up the type from the env.
    fn materialize_long_constant(
        &mut self,
        instrs: &mut Vec<Instruction>,
        value: i64,
    ) -> Val {
        let name = format!("const.{}", self.const_counter);
        self.const_counter += 1;
        self.type_env.insert(name.clone(), OperandType::Long);
        instrs.push(Instruction::Copy {
            src: Val::Constant(value),
            dst: name.clone(),
        });
        Val::Var(name)
    }
}

fn lower_block_items(items: &[BlockItem], ctx: &mut LowerCtx) -> Result<Vec<Instruction>> {
    let mut out = Vec::new();
    for item in items {
        match item {
            BlockItem::Statement(stmt) => out.extend(lower_statement(stmt, ctx)?),
            BlockItem::Declaration(decl) => {
                let decl_ty = type_to_operand_type(decl.ty);
                ctx.type_env
                    .entry(decl.name.clone())
                    .or_insert(decl_ty);
                if let Some(expr) = &decl.init {
                    let (instrs, val) = lower_expr(expr, ctx)?;
                    out.extend(instrs);
                    out.push(Instruction::Copy {
                        src: val,
                        dst: decl.name.clone(),
                    });
                }
            }
            BlockItem::FunctionDecl(_) => {}
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
        Statement::While {
            condition,
            body,
            label,
        } => {
            // Mirrors `emit_tacky_for_while_loop` in
            // `nqcc2/lib/tacky_gen.ml`:
            //   Label continue.<id>
            //   <eval condition>
            //   JumpIfZero c, break.<id>
            //   <body>
            //   Jump continue.<id>
            //   Label break.<id>
            let cont = continue_label(label);
            let br = break_label(label);
            let (cond_instrs, cond_val) = lower_expr(condition, ctx)?;
            let mut out = Vec::new();
            out.push(Instruction::Label(cont.clone()));
            out.extend(cond_instrs);
            out.push(Instruction::JumpIfZero {
                condition: cond_val,
                target: br.clone(),
            });
            out.extend(lower_statement(body, ctx)?);
            out.push(Instruction::Jump { target: cont });
            out.push(Instruction::Label(br));
            Ok(out)
        }
        Statement::DoWhile {
            body,
            condition,
            label,
        } => {
            // Mirrors `emit_tacky_for_do_loop`:
            //   Label start_label
            //   <body>
            //   Label continue.<id>
            //   <eval condition>
            //   JumpIfNotZero c, start_label
            //   Label break.<id>
            let start = format!("do_start.{label}");
            let cont = continue_label(label);
            let br = break_label(label);
            let (cond_instrs, cond_val) = lower_expr(condition, ctx)?;
            let mut out = Vec::new();
            out.push(Instruction::Label(start.clone()));
            out.extend(lower_statement(body, ctx)?);
            out.push(Instruction::Label(cont));
            out.extend(cond_instrs);
            out.push(Instruction::JumpIfNotZero {
                condition: cond_val,
                target: start,
            });
            out.push(Instruction::Label(br));
            Ok(out)
        }
        Statement::For {
            init,
            condition,
            post,
            body,
            label,
        } => {
            // Mirrors `emit_tacky_for_for_loop`:
            //   <init>
            //   Label start_label
            //   <eval condition>; JumpIfZero c, break.<id>
            //   <body>
            //   Label continue.<id>; <post>
            //   Jump start_label
            //   Label break.<id>
            let start = format!("for_start.{label}");
            let cont = continue_label(label);
            let br = break_label(label);
            let mut out = Vec::new();
            if let Some(init) = init {
                match init {
                    ForInit::Declaration(decl) => {
                        ctx.type_env
                            .entry(decl.name.clone())
                            .or_insert(type_to_operand_type(decl.ty));
                        if let Some(expr) = &decl.init {
                            let (instrs, val) = lower_expr(expr, ctx)?;
                            out.extend(instrs);
                            out.push(Instruction::Copy {
                                src: val,
                                dst: decl.name.clone(),
                            });
                        }
                    }
                    ForInit::Expr(expr) => {
                        let (instrs, _val) = lower_expr(expr, ctx)?;
                        out.extend(instrs);
                    }
                }
            }
            out.push(Instruction::Label(start.clone()));
            if let Some(condition) = condition {
                let (cond_instrs, cond_val) = lower_expr(condition, ctx)?;
                out.extend(cond_instrs);
                out.push(Instruction::JumpIfZero {
                    condition: cond_val,
                    target: br.clone(),
                });
            }
            out.extend(lower_statement(body, ctx)?);
            out.push(Instruction::Label(cont));
            if let Some(post) = post {
                let (instrs, _val) = lower_expr(post, ctx)?;
                out.extend(instrs);
            }
            out.push(Instruction::Jump { target: start });
            out.push(Instruction::Label(br));
            Ok(out)
        }
        Statement::Goto(target) => Ok(vec![Instruction::Jump {
            target: mangle_user_label(
                target,
                &mut ctx.user_label_counter,
                &mut ctx.user_labels,
                ctx.current_function.as_deref(),
            ),
        }]),
        Statement::Labeled { label, statement } => {
            let mut out = Vec::new();
            out.push(Instruction::Label(mangle_user_label(
                label,
                &mut ctx.user_label_counter,
                &mut ctx.user_labels,
                ctx.current_function.as_deref(),
            )));
            out.extend(lower_statement(statement, ctx)?);
            Ok(out)
        }
        Statement::Break(target) => Ok(vec![Instruction::Jump {
            target: break_label(target),
        }]),
        Statement::Continue(target) => Ok(vec![Instruction::Jump {
            target: continue_label(target),
        }]),
        Statement::Switch { expr, body, label } => lower_switch(expr, body, label, ctx),
        Statement::Case { value, statement } => {
            // `Case` nodes are only ever encountered while
            // lowering a switch body — the outer switch has
            // populated `ctx.case_labels` with the dispatch
            // labels.  Emit the matching label, then lower the
            // case's own statement.
            let label = match value {
                Expr::Constant(n) => ctx
                    .case_labels
                    .as_ref()
                    .and_then(|m| m.get(n).cloned())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "lower: case {} outside of any switch dispatch",
                            n
                        )
                    })?,
                _ => {
                    return Err(anyhow::anyhow!(
                        "lower: switch case value must be a constant integer"
                    ))
                }
            };
            let mut out = vec![Instruction::Label(label)];
            out.extend(lower_statement(statement, ctx)?);
            Ok(out)
        }
        Statement::Default { statement } => {
            // Same story as `Case`: emit the default label
            // (populated by the enclosing switch), then lower
            // the default's own statement.
            let label = ctx.default_label.as_ref().ok_or_else(|| {
                anyhow::anyhow!("lower: `default` outside of any switch")
            })?;
            let mut out = vec![Instruction::Label(label.clone())];
            out.extend(lower_statement(statement, ctx)?);
            Ok(out)
        }
    }
}

/// Lower a `switch (expr) body`.
///
/// The lowering has two phases:
///
/// 1. **Dispatch** — walk `body` (recursively, so nested
///    `Case`/`Default` inside loops / `if`s / other switches are
///    picked up; this matches Duff's device where case labels are
///    scattered through nested constructs) and collect every
///    case value in source order, plus optionally one `default`.
///    Then emit the canonical chain:
///
///    ```text
///      eval(expr) -> v
///      for each case value (in source order):
///          copy v to tmp
///          sub tmp, case_value
///          JumpIfZero tmp, case.<i>.<switch_label>
///      jump to default_label if any, else jump to switch_end
///    ```
///
/// 2. **Body** — lower `body` via the normal `lower_statement`
///    path.  `Case` and `Default` nodes emit a `Label` using the
///    case-label map stored on `ctx`; everything else falls
///    through normally.  This means case labels appear inside the
///    body wherever the C source put them — including inside
///    nested loops / `if`s — and the dispatch's `case.<i>` jumps
///    land on the same labels that the body emits at those
///    positions.
///
/// `break;` jumps to `switch_end` (= `break.<label>`); the
/// `label_loops` pass already filled in that target.
fn lower_switch(
    expr: &Expr,
    body: &Statement,
    label: &str,
    ctx: &mut LowerCtx,
) -> Result<Vec<Instruction>> {
    let switch_end = break_label(label);
    let default_label = format!("default.{label}");

    // Phase 1: collect all case values (recursively) and the
    // presence of a default.  Assign sequential indices so the
    // dispatch's `case.<i>` matches the label that the body
    // emits for that occurrence.
    let mut case_values: Vec<i64> = Vec::new();
    let mut has_default = false;
    collect_switch_dispatch(body, &mut case_values, &mut has_default)?;

    // Save any outer switch's state so a nested switch doesn't
    // clobber it.
    let saved_case_labels = ctx.case_labels.take();
    let saved_default_label = ctx.default_label.take();

    let mut case_label_map: HashMap<i64, String> = HashMap::new();
    for (i, v) in case_values.iter().enumerate() {
        case_label_map.insert(*v, format!("case.{i}.{label}"));
    }
    ctx.case_labels = Some(case_label_map);
    ctx.default_label = if has_default {
        Some(default_label.clone())
    } else {
        None
    };

    let (eval_instrs, switch_val) = lower_expr(expr, ctx)?;
    let mut out = eval_instrs;

    for case_val in &case_values {
        let case_label = format!("case.{}.{label}", case_label_index(case_val, &case_values));
        let tmp = ctx.fresh_tmp();
        out.push(Instruction::Copy {
            src: switch_val.clone(),
            dst: tmp.clone(),
        });
        out.push(Instruction::Sub {
            src: Val::Constant(*case_val),
            dst: tmp.clone(),
        });
        out.push(Instruction::JumpIfZero {
            condition: Val::Var(tmp),
            target: case_label,
        });
    }

    let default_target = if has_default {
        default_label.clone()
    } else {
        switch_end.clone()
    };
    out.push(Instruction::Jump {
        target: default_target,
    });

    // Phase 2: lower the body normally.  `Case` and `Default`
    // nodes encountered here emit `Label` instructions via
    // `ctx.case_labels` / `ctx.default_label`; everything else
    // lowers as it would outside a switch.
    out.extend(lower_statement(body, ctx)?);

    out.push(Instruction::Label(switch_end));

    // Restore outer switch state (or clear if this was the
    // outermost switch).
    ctx.case_labels = saved_case_labels;
    ctx.default_label = saved_default_label;

    Ok(out)
}

/// Find the dispatch index of a given case value in the
/// collected list.  O(n) but the case lists are tiny.
fn case_label_index(value: &i64, values: &[i64]) -> usize {
    values.iter().position(|v| v == value).unwrap_or(0)
}

/// Walk `stmt` collecting every `Case` value (recursively into
/// nested constructs) and recording whether a `default:` exists.
/// The order of `case_values` matches source order, which is
/// what determines the dispatch's `case.<i>` numbering.
fn collect_switch_dispatch(
    stmt: &Statement,
    case_values: &mut Vec<i64>,
    has_default: &mut bool,
) -> Result<()> {
    match stmt {
        Statement::Block(items) => {
            for item in items {
                if let BlockItem::Statement(s) = item {
                    collect_switch_dispatch(s, case_values, has_default)?;
                }
            }
            Ok(())
        }
        Statement::Case { value, statement } => {
            let v = match value {
                Expr::Constant(n) => *n,
                _ => {
                    return Err(anyhow::anyhow!(
                        "lower: switch case value must be a constant integer"
                    ))
                }
            };
            case_values.push(v);
            // Recurse into the case body — Duff's device has
            // case labels nested inside loops / ifs that are
            // themselves inside a case's body.
            collect_switch_dispatch(statement, case_values, has_default)
        }
        Statement::Default { statement } => {
            *has_default = true;
            collect_switch_dispatch(statement, case_values, has_default)
        }
        Statement::If {
            then_branch,
            else_branch,
            ..
        } => {
            collect_switch_dispatch(then_branch, case_values, has_default)?;
            if let Some(else_branch) = else_branch {
                collect_switch_dispatch(else_branch, case_values, has_default)?;
            }
            Ok(())
        }
        Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
            collect_switch_dispatch(body, case_values, has_default)
        }
        Statement::For { body, .. } => collect_switch_dispatch(body, case_values, has_default),
        Statement::Switch { .. } => {
            // A nested switch has its own dispatch; its
            // cases belong to that inner switch, not to us.
            // `lower_switch` saves and restores the outer
            // switch's case-label map, so the inner switch
            // gets a fresh map of its own.
            Ok(())
        }
        Statement::Labeled { statement, .. } => {
            collect_switch_dispatch(statement, case_values, has_default)
        }
        // Everything else (expressions, returns, declarations,
        // gotos, etc.) carries no case/default.
        _ => Ok(()),
    }
}

fn break_label(id: &str) -> String {
    format!("break.{id}")
}

fn continue_label(id: &str) -> String {
    format!("continue.{id}")
}

fn lower_expr(expr: &Expr, ctx: &mut LowerCtx) -> Result<(Vec<Instruction>, Val)> {
    match expr {
        Expr::Constant(n) => Ok((Vec::new(), Val::Constant(*n))),
        Expr::LongConstant(n) => {
            // The lowerer can't keep `Val::Constant` typed, so it
            // materialises the long constant into a fresh synthetic
            // name.  Downstream uses see a `Val::Var` and look the
            // type up from the env.
            let mut instrs = Vec::new();
            let v = ctx.materialize_long_constant(&mut instrs, *n);
            Ok((instrs, v))
        }
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
        Expr::Call { name, args } => lower_call(name, args, ctx),
        Expr::Cast { target_type, expr: inner } => {
            // Chapter 11 explicit cast: `SignExtend` (int -> long)
            // or `Truncate` (long -> int).  Other combinations
            // (int -> int, long -> long) lower to a plain Copy.
            let (mut instrs, src) = lower_expr(inner, ctx)?;
            let src_ty = type_of_val(&src, ctx);
            let dst_ty = type_to_operand_type(*target_type);
            if src_ty == dst_ty {
                return Ok((instrs, src));
            }
            let dst_name = if dst_ty == OperandType::Long {
                ctx.fresh_typed_tmp(OperandType::Long)
            } else {
                ctx.fresh_typed_tmp(OperandType::Int)
            };
            match (src_ty, dst_ty) {
                (OperandType::Int, OperandType::Long) => {
                    instrs.push(Instruction::SignExtend {
                        src: src.clone(),
                        dst: dst_name.clone(),
                    });
                }
                (OperandType::Long, OperandType::Int) => {
                    instrs.push(Instruction::Truncate {
                        src: src.clone(),
                        dst: dst_name.clone(),
                    });
                }
                _ => {
                    instrs.push(Instruction::Copy {
                        src: src.clone(),
                        dst: dst_name.clone(),
                    });
                }
            }
            Ok((instrs, Val::Var(dst_name)))
        }
    }
}

/// Lower a function call `f(args)`.
///
/// Mirrors `emit_fun_call` in `nqcc2/lib/tacky_gen.ml:311-325`:
/// 1. Lower each argument expression in source order.
/// 2. Concatenate the per-argument instruction lists (each
///    `lower_expr` for an argument may itself allocate a
///    temporary).
/// 3. Append `Instruction::Call { name, args, dst }` where
///    `dst` is `None` for the chapter-9 surface — chapter 12
///    widens this to `Some(...)` when a non-void call result is
///    actually consumed (we already use `dst` for chapter-9's
///    `int`-returning functions because the call site may
///    immediately use the value).
fn lower_call(
    name: &str,
    args: &[Expr],
    ctx: &mut LowerCtx,
) -> Result<(Vec<Instruction>, Val)> {
    let mut out: Vec<Instruction> = Vec::new();
    let mut arg_vals: Vec<Val> = Vec::with_capacity(args.len());
    // Chapter 11: when the called function's signature is known,
    // convert each argument to the corresponding parameter's
    // declared width.  An int passed to a long parameter is
    // sign-extended; a long passed to an int parameter is
    // truncated.  Mirrors the `convert_by_assignment` calls in
    // OCaml `typecheck_fun_call`.
    let param_tys: Option<Vec<crate::ir::tacky::OperandType>> =
        ctx.func_sigs.get(name).cloned();
    for (idx, arg) in args.iter().enumerate() {
        let (instrs, val) = lower_expr(arg, ctx)?;
        out.extend(instrs);
        let val = if let Some(ref pts) = param_tys {
            if let Some(&param_ty) = pts.get(idx) {
                let arg_ty = type_of_val(&val, ctx);
                if arg_ty == OperandType::Int && param_ty == OperandType::Long {
                    let tmp = ctx.fresh_typed_tmp(OperandType::Long);
                    out.push(Instruction::SignExtend {
                        src: val,
                        dst: tmp.clone(),
                    });
                    Val::Var(tmp)
                } else if arg_ty == OperandType::Long && param_ty == OperandType::Int {
                    let tmp = ctx.fresh_typed_tmp(OperandType::Int);
                    out.push(Instruction::Truncate {
                        src: val,
                        dst: tmp.clone(),
                    });
                    Val::Var(tmp)
                } else {
                    val
                }
            } else {
                val
            }
        } else {
            val
        };
        arg_vals.push(val);
    }
    let dst_name = ctx.fresh_tmp();
    out.push(Instruction::Call {
        name: name.to_string(),
        args: arg_vals,
        dst: Some(dst_name.clone()),
    });
    Ok((out, Val::Var(dst_name)))
}

/// Lower a cast expression `(T) expr`.  Mirrors
/// `nqcc2/lib/tacky_gen.ml` `emit_cast_expression`: when the inner
/// type and target type differ in width, emit `SignExtend` (int ->
/// long) or `Truncate` (long -> int); same-width casts degrade to a
/// plain `Copy`.
fn lower_cast(target_type: Type, inner: &Expr, ctx: &mut LowerCtx) -> Result<(Vec<Instruction>, Val)> {
    let (mut instrs, inner_val) = lower_expr(inner, ctx)?;
    let inner_ty = type_of_val(&inner_val, ctx);
    let target_ty = type_to_operand_type(target_type);
    let dst = ctx.fresh_typed_tmp(target_ty);
    match (inner_ty, target_ty) {
        (OperandType::Int, OperandType::Long) => {
            instrs.push(Instruction::SignExtend {
                src: inner_val,
                dst: dst.clone(),
            });
        }
        (OperandType::Long, OperandType::Int) => {
            instrs.push(Instruction::Truncate {
                src: inner_val,
                dst: dst.clone(),
            });
        }
        _ => {
            instrs.push(Instruction::Copy {
                src: inner_val,
                dst: dst.clone(),
            });
        }
    }
    Ok((instrs, Val::Var(dst)))
}

fn lower_unary(
    op: UnaryOp,
    inner: &Expr,
    ctx: &mut LowerCtx,
) -> Result<(Vec<Instruction>, Val)> {
    let (mut instrs, inner_val) = lower_expr(inner, ctx)?;
    let inner_ty = type_of_val(&inner_val, ctx);
    let tmp = ctx.fresh_typed_tmp(inner_ty);
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
    let target_ty = ctx
        .type_env
        .get(&target_name)
        .copied()
        .unwrap_or(OperandType::Int);
    if op == AssignOp::Assign {
        let (mut instrs, rhs_val) = lower_expr(value, ctx)?;
        // Truncate an over-wide RHS into the LHS's type so a
        // `long` expression on the right of `int x = ...` is
        // narrowed.  The book's chapter-11 `convert_by_assignment`
        // covers this; we emit the explicit Truncate here because
        // the lowerer keeps TACKY untagged.
        let rhs_ty = type_of_val(&rhs_val, ctx);
        let rhs_val = narrow_to_target(rhs_val, rhs_ty, target_ty, &mut instrs, ctx);
        instrs.push(Instruction::Copy {
            src: rhs_val.clone(),
            dst: target_name,
        });
        return Ok((instrs, rhs_val));
    }
    let bin_op = compound_binop(op)
        .ok_or_else(|| anyhow::anyhow!("lower: invalid compound assignment operator"))?;
    let tmp = ctx.fresh_typed_tmp(target_ty);
    let (mut instrs, rhs_val) = lower_expr(value, ctx)?;
    let rhs_ty = type_of_val(&rhs_val, ctx);
    let (lhs_for_copy, rhs_for_op, _) = promote_for_binary(
        Val::Var(target_name.clone()),
        rhs_val,
        target_ty,
        rhs_ty,
        &mut instrs,
        ctx,
    );
    instrs.push(Instruction::Copy {
        src: lhs_for_copy,
        dst: tmp.clone(),
    });
    instrs.push(binary_to_tacky(bin_op, rhs_for_op, tmp.clone()));
    instrs.push(Instruction::Copy {
        src: Val::Var(tmp.clone()),
        dst: target_name,
    });
    Ok((instrs, Val::Var(tmp)))
}

/// Narrow an over-wide value into `target_ty` via a `Truncate`
/// instruction.  A same-width or wider-to-narrower transition emits
/// the truncate; an int value being widened to long is left as-is
/// (the binary op handles the promotion).
fn narrow_to_target(
    val: Val,
    val_ty: OperandType,
    target_ty: OperandType,
    instrs: &mut Vec<Instruction>,
    ctx: &mut LowerCtx,
) -> Val {
    if val_ty == OperandType::Long && target_ty == OperandType::Int {
        let tmp = ctx.fresh_typed_tmp(OperandType::Int);
        instrs.push(Instruction::Truncate {
            src: val,
            dst: tmp.clone(),
        });
        Val::Var(tmp)
    } else {
        val
    }
}

fn lower_prefix_incdec(
    inner: &Expr,
    increment: bool,
    ctx: &mut LowerCtx,
) -> Result<(Vec<Instruction>, Val)> {
    let target_name = inner
        .lvalue_name()
        .ok_or_else(|| anyhow::anyhow!("lower: invalid lvalue in ++/--"))?
        .to_string();
    let target_ty = ctx
        .type_env
        .get(&target_name)
        .copied()
        .unwrap_or(OperandType::Int);
    let mut instrs = Vec::new();
    let one = if target_ty == OperandType::Long {
        // Materialise `1` as a long-typed const so the codegen
        // emits `addq $1, slot` rather than `addl $1, slot`.
        let v = ctx.materialize_long_constant(&mut instrs, 1);
        v
    } else {
        Val::Constant(1)
    };
    let instr = if increment {
        Instruction::Add {
            src: one,
            dst: target_name.clone(),
        }
    } else {
        Instruction::Sub {
            src: one,
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
    let target_ty = ctx
        .type_env
        .get(&target_name)
        .copied()
        .unwrap_or(OperandType::Int);
    let old = ctx.fresh_typed_tmp(target_ty);
    let mut instrs = Vec::new();
    instrs.push(Instruction::Copy {
        src: Val::Var(target_name.clone()),
        dst: old.clone(),
    });
    let one = if target_ty == OperandType::Long {
        ctx.materialize_long_constant(&mut instrs, 1)
    } else {
        Val::Constant(1)
    };
    let instr = if increment {
        Instruction::Add {
            src: one,
            dst: target_name,
        }
    } else {
        Instruction::Sub {
            src: one,
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
    let else_label = ctx.labels.next_with_prefix("cond_else");
    let end_label = ctx.labels.next_with_prefix("cond_end");
    let (cond_instrs, cond_val) = lower_expr(condition, ctx)?;
    let (mut then_instrs, then_val) = lower_expr(then_expr, ctx)?;
    let (mut else_instrs, else_val) = lower_expr(else_expr, ctx)?;
    // The result's type follows the usual arithmetic conversion of
    // the two branch values: long if either branch is long, int
    // otherwise.  Mirrors `get_common_type` for the chapter-11
    // surface.
    let then_ty = type_of_val(&then_val, ctx);
    let else_ty = type_of_val(&else_val, ctx);
    let result_ty = if then_ty == OperandType::Long || else_ty == OperandType::Long {
        OperandType::Long
    } else {
        OperandType::Int
    };
    let result = ctx.fresh_typed_tmp(result_ty);

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
            // Promotion: when one operand is long, both are
            // materialised as long.  An int operand is widened with
            // `SignExtend` into a fresh tmp before the binary op;
            // an int temporary that is the destination of a
            // comparison or arithmetic op is upgraded to long so
            // the assembler emits the quadword form.
            let left_ty = type_of_val(&left_val, ctx);
            let right_ty = type_of_val(&right_val, ctx);
            let (left_val, right_val, dst_ty) = promote_for_binary(left_val, right_val, left_ty, right_ty, &mut instrs, ctx);
            // Re-resolve types after promotion so the destination
            // tmp is correctly tagged.
            let _ = (left_ty, right_ty);
            let _ = dst_ty;
            if is_cmp_op(op) {
                let tmp = ctx.fresh_typed_tmp(OperandType::Int);
                instrs.push(Instruction::Copy {
                    src: left_val.clone(),
                    dst: tmp.clone(),
                });
                instrs.push(cmp_to_tacky(op, left_val, right_val, tmp.clone()));
                Ok((instrs, Val::Var(tmp)))
            } else {
                let tmp_ty = type_of_val(&left_val, ctx);
                let tmp = ctx.fresh_typed_tmp(tmp_ty);
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

/// Return the operand type of a `Val` for the purposes of TACKY
/// instruction selection.  Constants default to `Int`; named
/// variables look their type up from the lowerer's `type_env` (the
/// env is populated for every parameter, every local, every
/// materialised long constant, and every synthetic tmp the lowerer
/// has already created).
fn type_of_val(val: &Val, ctx: &LowerCtx) -> OperandType {
    match val {
        Val::Constant(_) => OperandType::Int,
        Val::Var(name) => ctx
            .type_env
            .get(name)
            .copied()
            .unwrap_or(OperandType::Int),
    }
}

/// Usual arithmetic conversion for int / long.  When one operand is
/// long, the other is sign-extended into a fresh tmp and the result
/// type is long.  When both are int, the operands are left as-is.
/// Mirrors `convert_to` + the chapter-11 `get_common_type` path in
/// `nqcc2/lib/semantic_analysis/typecheck.ml`.
fn promote_for_binary(
    left_val: Val,
    right_val: Val,
    left_ty: OperandType,
    right_ty: OperandType,
    instrs: &mut Vec<Instruction>,
    ctx: &mut LowerCtx,
) -> (Val, Val, OperandType) {
    match (left_ty, right_ty) {
        (OperandType::Int, OperandType::Long) => {
            let tmp = ctx.fresh_typed_tmp(OperandType::Long);
            instrs.push(Instruction::SignExtend {
                src: left_val,
                dst: tmp.clone(),
            });
            (Val::Var(tmp), right_val, OperandType::Long)
        }
        (OperandType::Long, OperandType::Int) => {
            let tmp = ctx.fresh_typed_tmp(OperandType::Long);
            instrs.push(Instruction::SignExtend {
                src: right_val,
                dst: tmp.clone(),
            });
            (left_val, Val::Var(tmp), OperandType::Long)
        }
        (a, b) => (left_val, right_val, if a == OperandType::Long || b == OperandType::Long {
            OperandType::Long
        } else {
            OperandType::Int
        }),
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