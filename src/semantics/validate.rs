//! Semantic validation and name resolution for the native frontend.
//!
//! This pass checks rules that require program context rather than token shape:
//! declaration before use, unique local names per scope, label/goto consistency,
//! loop control placement, and switch case validity.  The mutable `Validator`
//! struct owns phase state (`HashMap` scopes, `HashSet` labels/cases, and depth
//! counters) so recursive validation can borrow `&mut self` while walking the
//! AST without exposing those details to `compiler.rs`.

use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};

use crate::ast::{BinaryOp, BlockItem, Expr, ForInit, Program, Statement};

pub(crate) fn validate_program(program: &Program) -> Result<Program> {
    Validator::default().resolve_program(program)
}

#[derive(Default)]
struct Validator {
    scopes: Vec<HashMap<String, String>>,
    labels: HashSet<String>,
    gotos: Vec<String>,
    next_symbol: usize,
    loop_depth: usize,
    break_depth: usize,
    switch_stack: Vec<SwitchValidation>,
}

#[derive(Default)]
struct SwitchValidation {
    cases: HashSet<i32>,
    has_default: bool,
}

impl Validator {
    fn resolve_program(&mut self, program: &Program) -> Result<Program> {
        self.scopes.push(HashMap::new());
        let mut body = Vec::new();
        for item in &program.body {
            body.push(self.resolve_block_item(item)?);
        }
        self.scopes.pop();

        for label in &self.gotos {
            if !self.labels.contains(label) {
                bail!("semantic error: goto references missing label {label}");
            }
        }

        Ok(Program {
            function_name: program.function_name.clone(),
            body,
        })
    }

    fn resolve_block_item(&mut self, item: &BlockItem) -> Result<BlockItem> {
        match item {
            BlockItem::Declaration { name, init } => {
                let unique = self.declare_symbol(name)?;
                // A declaration is visible in its own initializer in this book's
                // dialect.  We therefore insert the freshly allocated symbol
                // before resolving `init`, so `int a = a = 4;` assigns the new
                // inner `a` rather than an outer shadowed `a`.
                let init = init
                    .as_ref()
                    .map(|expr| self.resolve_expr(expr))
                    .transpose()?;
                Ok(BlockItem::Declaration { name: unique, init })
            }
            BlockItem::Statement(statement) => {
                Ok(BlockItem::Statement(self.resolve_statement(statement)?))
            }
        }
    }

    fn resolve_statement(&mut self, statement: &Statement) -> Result<Statement> {
        match statement {
            Statement::Return(expr) => Ok(Statement::Return(self.resolve_expr(expr)?)),
            Statement::Block(items) => {
                // Compound statements create a lexical scope.  A `Vec` of scope
                // maps is enough here because Chapter 7 has one integer type and
                // no declarations that escape their block; lookup walks the Vec
                // from innermost to outermost, which mirrors lexical shadowing.
                self.scopes.push(HashMap::new());
                let mut resolved = Vec::new();
                for item in items {
                    resolved.push(self.resolve_block_item(item)?);
                }
                self.scopes.pop();
                Ok(Statement::Block(resolved))
            }
            Statement::While { condition, body } => {
                let condition = self.resolve_expr(condition)?;
                self.loop_depth += 1;
                self.break_depth += 1;
                let body = self.resolve_statement(body)?;
                self.break_depth -= 1;
                self.loop_depth -= 1;
                Ok(Statement::While {
                    condition,
                    body: Box::new(body),
                })
            }
            Statement::DoWhile { body, condition } => {
                self.loop_depth += 1;
                self.break_depth += 1;
                let body = self.resolve_statement(body)?;
                self.break_depth -= 1;
                self.loop_depth -= 1;
                let condition = self.resolve_expr(condition)?;
                Ok(Statement::DoWhile {
                    body: Box::new(body),
                    condition,
                })
            }
            Statement::For {
                init,
                condition,
                post,
                body,
            } => {
                self.scopes.push(HashMap::new());
                let init = init
                    .as_ref()
                    .map(|init| self.resolve_for_init(init))
                    .transpose()?;
                let condition = condition
                    .as_ref()
                    .map(|expr| self.resolve_expr(expr))
                    .transpose()?;
                let post = post
                    .as_ref()
                    .map(|expr| self.resolve_expr(expr))
                    .transpose()?;
                self.loop_depth += 1;
                self.break_depth += 1;
                let body = self.resolve_statement(body)?;
                self.break_depth -= 1;
                self.loop_depth -= 1;
                self.scopes.pop();
                Ok(Statement::For {
                    init,
                    condition,
                    post,
                    body: Box::new(body),
                })
            }
            Statement::Break => {
                if self.break_depth == 0 {
                    bail!("semantic error: break outside loop or switch");
                }
                Ok(Statement::Break)
            }
            Statement::Continue => {
                if self.loop_depth == 0 {
                    bail!("semantic error: continue outside loop");
                }
                Ok(Statement::Continue)
            }
            Statement::Switch { expr, body } => {
                let expr = self.resolve_expr(expr)?;
                self.break_depth += 1;
                self.switch_stack.push(SwitchValidation::default());
                let body = self.resolve_statement(body)?;
                self.switch_stack.pop();
                self.break_depth -= 1;
                Ok(Statement::Switch {
                    expr,
                    body: Box::new(body),
                })
            }
            Statement::Case { value, statement } => {
                let case_value = self.const_eval(value)?;
                let switch = self
                    .switch_stack
                    .last_mut()
                    .ok_or_else(|| anyhow::anyhow!("semantic error: case outside switch"))?;
                if !switch.cases.insert(case_value) {
                    bail!("semantic error: duplicate case {case_value}");
                }
                Ok(Statement::Case {
                    value: Expr::Constant(case_value),
                    statement: Box::new(self.resolve_statement(statement)?),
                })
            }
            Statement::Default { statement } => {
                let switch = self
                    .switch_stack
                    .last_mut()
                    .ok_or_else(|| anyhow::anyhow!("semantic error: default outside switch"))?;
                if switch.has_default {
                    bail!("semantic error: duplicate default");
                }
                switch.has_default = true;
                Ok(Statement::Default {
                    statement: Box::new(self.resolve_statement(statement)?),
                })
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(Statement::If {
                condition: self.resolve_expr(condition)?,
                then_branch: Box::new(self.resolve_statement(then_branch)?),
                else_branch: else_branch
                    .as_ref()
                    .map(|branch| self.resolve_statement(branch))
                    .transpose()?
                    .map(Box::new),
            }),
            Statement::Goto(label) => {
                self.gotos.push(label.clone());
                Ok(Statement::Goto(label.clone()))
            }
            Statement::Labeled { label, statement } => {
                if !self.labels.insert(label.clone()) {
                    bail!("semantic error: duplicate label {label}");
                }
                Ok(Statement::Labeled {
                    label: label.clone(),
                    statement: Box::new(self.resolve_statement(statement)?),
                })
            }
            Statement::Expr(Some(expr)) => Ok(Statement::Expr(Some(self.resolve_expr(expr)?))),
            Statement::Expr(None) => Ok(Statement::Expr(None)),
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) -> Result<Expr> {
        match expr {
            Expr::Constant(value) => Ok(Expr::Constant(*value)),
            Expr::Var(name) => Ok(Expr::Var(self.lookup_symbol(name)?)),
            Expr::Paren(inner) => Ok(Expr::Paren(Box::new(self.resolve_expr(inner)?))),
            Expr::Negate(inner) => Ok(Expr::Negate(Box::new(self.resolve_expr(inner)?))),
            Expr::Complement(inner) => Ok(Expr::Complement(Box::new(self.resolve_expr(inner)?))),
            Expr::LogicalNot(inner) => Ok(Expr::LogicalNot(Box::new(self.resolve_expr(inner)?))),
            Expr::PreInc(inner) => {
                self.resolve_lvalue_expr(expr, inner, |inner| Expr::PreInc(Box::new(inner)))
            }
            Expr::PreDec(inner) => {
                self.resolve_lvalue_expr(expr, inner, |inner| Expr::PreDec(Box::new(inner)))
            }
            Expr::PostInc(inner) => {
                self.resolve_lvalue_expr(expr, inner, |inner| Expr::PostInc(Box::new(inner)))
            }
            Expr::PostDec(inner) => {
                self.resolve_lvalue_expr(expr, inner, |inner| Expr::PostDec(Box::new(inner)))
            }
            Expr::Assign { op, target, value } => {
                self.ensure_lvalue(target)?;
                Ok(Expr::Assign {
                    op: *op,
                    target: Box::new(self.resolve_expr(target)?),
                    value: Box::new(self.resolve_expr(value)?),
                })
            }
            Expr::Conditional {
                condition,
                then_expr,
                else_expr,
            } => Ok(Expr::Conditional {
                condition: Box::new(self.resolve_expr(condition)?),
                then_expr: Box::new(self.resolve_expr(then_expr)?),
                else_expr: Box::new(self.resolve_expr(else_expr)?),
            }),
            Expr::Binary { op, left, right } => Ok(Expr::Binary {
                op: *op,
                left: Box::new(self.resolve_expr(left)?),
                right: Box::new(self.resolve_expr(right)?),
            }),
        }
    }

    fn resolve_for_init(&mut self, init: &ForInit) -> Result<ForInit> {
        match init {
            ForInit::Declaration { name, init } => {
                let unique = self.declare_symbol(name)?;
                let init = init
                    .as_ref()
                    .map(|expr| self.resolve_expr(expr))
                    .transpose()?;
                Ok(ForInit::Declaration { name: unique, init })
            }
            ForInit::Expr(expr) => Ok(ForInit::Expr(self.resolve_expr(expr)?)),
        }
    }

    fn const_eval(&self, expr: &Expr) -> Result<i32> {
        match expr {
            Expr::Constant(value) => Ok(*value),
            Expr::Paren(inner) => self.const_eval(inner),
            Expr::Negate(inner) => Ok(self.const_eval(inner)?.wrapping_neg()),
            Expr::Complement(inner) => Ok(!self.const_eval(inner)?),
            Expr::LogicalNot(inner) => Ok(i32::from(self.const_eval(inner)? == 0)),
            Expr::Binary { op, left, right } => {
                let left = self.const_eval(left)?;
                match op {
                    BinaryOp::LogicalAnd => {
                        if left == 0 {
                            Ok(0)
                        } else {
                            Ok(i32::from(self.const_eval(right)? != 0))
                        }
                    }
                    BinaryOp::LogicalOr => {
                        if left != 0 {
                            Ok(1)
                        } else {
                            Ok(i32::from(self.const_eval(right)? != 0))
                        }
                    }
                    _ => Ok(op.eval_values(left, self.const_eval(right)?)),
                }
            }
            Expr::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                if self.const_eval(condition)? != 0 {
                    self.const_eval(then_expr)
                } else {
                    self.const_eval(else_expr)
                }
            }
            Expr::Var(_)
            | Expr::PreInc(_)
            | Expr::PreDec(_)
            | Expr::PostInc(_)
            | Expr::PostDec(_)
            | Expr::Assign { .. } => bail!("semantic error: case value is not constant"),
        }
    }

    fn resolve_lvalue_expr(
        &mut self,
        _original: &Expr,
        inner: &Expr,
        rebuild: impl FnOnce(Expr) -> Expr,
    ) -> Result<Expr> {
        self.ensure_lvalue(inner)?;
        Ok(rebuild(self.resolve_expr(inner)?))
    }

    fn ensure_lvalue(&mut self, expr: &Expr) -> Result<()> {
        let name = expr
            .lvalue_name()
            .ok_or_else(|| anyhow::anyhow!("semantic error: invalid lvalue"))?;
        self.lookup_symbol(name)?;
        Ok(())
    }

    fn declare_symbol(&mut self, name: &str) -> Result<String> {
        let scope = self
            .scopes
            .last_mut()
            .expect("resolver always has at least one active scope");
        if scope.contains_key(name) {
            bail!("semantic error: redefinition of {name}");
        }
        let unique = format!("{name}.{}", self.next_symbol);
        self.next_symbol += 1;
        scope.insert(name.to_string(), unique.clone());
        Ok(unique)
    }

    fn lookup_symbol(&self, name: &str) -> Result<String> {
        for scope in self.scopes.iter().rev() {
            if let Some(unique) = scope.get(name) {
                return Ok(unique.clone());
            }
        }
        bail!("semantic error: undeclared variable {name}")
    }
}
