//! AST-to-linear-IR lowering.
//!
//! The lowerer owns label generation so structured statements (`if`, loops,
//! switch, labels/goto) become explicit jumps before evaluation.  It clones AST
//! expressions into instructions because this early IR is still interpreter-only
//! and the small official fixtures are better served by clear ownership than by
//! borrowing through multiple phase lifetimes.

use crate::ast::{BlockItem, Expr, ForInit, Program, Statement};
use crate::ir::tacky::Instr;

#[derive(Default)]
pub(crate) struct Lowerer {
    next_label: usize,
    break_targets: Vec<String>,
    continue_targets: Vec<String>,
    switch_case_labels: Vec<std::collections::VecDeque<String>>,
    switch_default_labels: Vec<Option<String>>,
}

impl Lowerer {
    pub(crate) fn lower_program(&mut self, program: &Program) -> Vec<Instr> {
        let mut instrs = Vec::new();
        for item in &program.body {
            self.lower_block_item(item, &mut instrs);
        }
        instrs
    }

    fn lower_block_item(&mut self, item: &BlockItem, instrs: &mut Vec<Instr>) {
        match item {
            BlockItem::Declaration { name, init } => instrs.push(Instr::Declare {
                name: name.clone(),
                init: init.clone(),
            }),
            BlockItem::Statement(statement) => self.lower_statement(statement, instrs),
        }
    }

    fn lower_statement(&mut self, statement: &Statement, instrs: &mut Vec<Instr>) {
        match statement {
            Statement::Return(expr) => instrs.push(Instr::Return(expr.clone())),
            Statement::Block(items) => {
                for item in items {
                    self.lower_block_item(item, instrs);
                }
            }
            Statement::While { condition, body } => {
                let condition_label = self.fresh_label("while_condition");
                let continue_label = self.fresh_label("while_continue");
                let break_label = self.fresh_label("while_break");
                instrs.push(Instr::Label(condition_label.clone()));
                instrs.push(Instr::JumpIfZero {
                    condition: condition.clone(),
                    target: break_label.clone(),
                });
                self.break_targets.push(break_label.clone());
                self.continue_targets.push(continue_label.clone());
                self.lower_statement(body, instrs);
                self.continue_targets.pop();
                self.break_targets.pop();
                instrs.push(Instr::Label(continue_label));
                instrs.push(Instr::Jump(condition_label));
                instrs.push(Instr::Label(break_label));
            }
            Statement::DoWhile { body, condition } => {
                let body_label = self.fresh_label("do_body");
                let continue_label = self.fresh_label("do_continue");
                let break_label = self.fresh_label("do_break");
                instrs.push(Instr::Label(body_label.clone()));
                self.break_targets.push(break_label.clone());
                self.continue_targets.push(continue_label.clone());
                self.lower_statement(body, instrs);
                self.continue_targets.pop();
                self.break_targets.pop();
                instrs.push(Instr::Label(continue_label));
                instrs.push(Instr::JumpIfZero {
                    condition: condition.clone(),
                    target: break_label.clone(),
                });
                instrs.push(Instr::Jump(body_label));
                instrs.push(Instr::Label(break_label));
            }
            Statement::For {
                init,
                condition,
                post,
                body,
            } => {
                if let Some(init) = init {
                    self.lower_for_init(init, instrs);
                }
                let condition_label = self.fresh_label("for_condition");
                let continue_label = self.fresh_label("for_continue");
                let break_label = self.fresh_label("for_break");
                instrs.push(Instr::Label(condition_label.clone()));
                if let Some(condition) = condition {
                    instrs.push(Instr::JumpIfZero {
                        condition: condition.clone(),
                        target: break_label.clone(),
                    });
                }
                self.break_targets.push(break_label.clone());
                self.continue_targets.push(continue_label.clone());
                self.lower_statement(body, instrs);
                self.continue_targets.pop();
                self.break_targets.pop();
                instrs.push(Instr::Label(continue_label));
                if let Some(post) = post {
                    instrs.push(Instr::Expr(Some(post.clone())));
                }
                instrs.push(Instr::Jump(condition_label));
                instrs.push(Instr::Label(break_label));
            }
            Statement::Break => {
                let target = self
                    .break_targets
                    .last()
                    .expect("semantic validation guarantees break target")
                    .clone();
                instrs.push(Instr::Jump(target));
            }
            Statement::Continue => {
                let target = self
                    .continue_targets
                    .last()
                    .expect("semantic validation guarantees continue target")
                    .clone();
                instrs.push(Instr::Jump(target));
            }
            Statement::Switch { expr, body } => self.lower_switch(expr, body, instrs),
            Statement::Case { statement, .. } => {
                let label = self
                    .switch_case_labels
                    .last_mut()
                    .and_then(|labels| labels.pop_front())
                    .expect("case label queue populated by enclosing switch");
                instrs.push(Instr::Label(label));
                self.lower_statement(statement, instrs);
            }
            Statement::Default { statement } => {
                let label = self
                    .switch_default_labels
                    .last_mut()
                    .and_then(Option::take)
                    .expect("default label populated by enclosing switch");
                instrs.push(Instr::Label(label));
                self.lower_statement(statement, instrs);
            }
            Statement::Expr(expr) => instrs.push(Instr::Expr(expr.clone())),
            Statement::Goto(label) => instrs.push(Instr::Jump(label.clone())),
            Statement::Labeled { label, statement } => {
                instrs.push(Instr::Label(label.clone()));
                self.lower_statement(statement, instrs);
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let false_label = self.fresh_label("if_false");
                let end_label = self.fresh_label("if_end");
                instrs.push(Instr::JumpIfZero {
                    condition: condition.clone(),
                    target: false_label.clone(),
                });
                self.lower_statement(then_branch, instrs);
                if let Some(else_branch) = else_branch {
                    instrs.push(Instr::Jump(end_label.clone()));
                    instrs.push(Instr::Label(false_label));
                    self.lower_statement(else_branch, instrs);
                    instrs.push(Instr::Label(end_label));
                } else {
                    instrs.push(Instr::Label(false_label));
                }
            }
        }
    }

    fn lower_for_init(&mut self, init: &ForInit, instrs: &mut Vec<Instr>) {
        match init {
            ForInit::Declaration { name, init } => instrs.push(Instr::Declare {
                name: name.clone(),
                init: init.clone(),
            }),
            ForInit::Expr(expr) => instrs.push(Instr::Expr(Some(expr.clone()))),
        }
    }

    fn lower_switch(&mut self, expr: &Expr, body: &Statement, instrs: &mut Vec<Instr>) {
        let break_label = self.fresh_label("switch_break");
        let mut cases = Vec::new();
        let mut case_labels = std::collections::VecDeque::new();
        let mut default_label = None;
        self.collect_switch_targets(body, &mut cases, &mut case_labels, &mut default_label);
        instrs.push(Instr::Switch {
            expr: expr.clone(),
            cases,
            default: default_label.clone(),
            end: break_label.clone(),
        });
        self.break_targets.push(break_label.clone());
        self.switch_case_labels.push(case_labels);
        self.switch_default_labels.push(default_label);
        self.lower_statement(body, instrs);
        self.switch_default_labels.pop();
        self.switch_case_labels.pop();
        self.break_targets.pop();
        instrs.push(Instr::Label(break_label));
    }

    fn collect_switch_targets(
        &mut self,
        statement: &Statement,
        cases: &mut Vec<(i32, String)>,
        case_labels: &mut std::collections::VecDeque<String>,
        default_label: &mut Option<String>,
    ) {
        match statement {
            Statement::Case { value, statement } => {
                let Expr::Constant(value) = value else {
                    unreachable!("semantic validation resolves case values to constants")
                };
                let label = self.fresh_label("case");
                cases.push((*value, label.clone()));
                case_labels.push_back(label);
                self.collect_switch_targets(statement, cases, case_labels, default_label);
            }
            Statement::Default { statement } => {
                let label = self.fresh_label("default");
                *default_label = Some(label);
                self.collect_switch_targets(statement, cases, case_labels, default_label);
            }
            Statement::Block(items) => {
                for item in items {
                    if let BlockItem::Statement(statement) = item {
                        self.collect_switch_targets(statement, cases, case_labels, default_label);
                    }
                }
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.collect_switch_targets(then_branch, cases, case_labels, default_label);
                if let Some(else_branch) = else_branch {
                    self.collect_switch_targets(else_branch, cases, case_labels, default_label);
                }
            }
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                self.collect_switch_targets(body, cases, case_labels, default_label);
            }
            Statement::For { body, .. }
            | Statement::Labeled {
                statement: body, ..
            } => {
                self.collect_switch_targets(body, cases, case_labels, default_label);
            }
            // A nested switch owns its own case/default labels; do not let the
            // enclosing switch jump into it.
            Statement::Switch { .. }
            | Statement::Return(_)
            | Statement::Break
            | Statement::Continue
            | Statement::Goto(_)
            | Statement::Expr(_) => {}
        }
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let label = format!(".L_{prefix}_{}", self.next_label);
        self.next_label += 1;
        label
    }
}
