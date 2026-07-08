//! Type-checking pass.
//!
//! Mirrors the chapter-14 pointer rules from
//! `nqcc2/lib/semantic_analysis/typecheck.ml`: declarations retain their
//! parsed types, expressions are checked against a scoped symbol table, and
//! pointer lvalue/conversion rules are enforced before lowering.

use std::collections::HashMap;

use anyhow::{bail, Result};

use crate::ast::{
    AssignOp, BinaryOp, BlockItem, Expr, ForInit, GlobalDecl, GlobalVarDecl, Program, Statement,
    TopLevelItem, Type, UnaryOp, VarDecl,
};

use super::resolve::ResolvedProgram;

#[derive(Debug, Clone)]
pub struct TypedProgram {
    pub program: Program,
}

struct TypeCtx {
    objects: HashMap<String, Type>,
    funcs: HashMap<String, (Vec<Type>, Type)>,
    current_return: Type,
}

pub fn typecheck(ast: &ResolvedProgram) -> Result<TypedProgram> {
    let mut ctx = TypeCtx {
        objects: HashMap::new(),
        funcs: HashMap::new(),
        current_return: Type::Int,
    };
    for item in &ast.program.top_level_items {
        match item {
            TopLevelItem::Function(func) => {
                let param_tys = func.params.iter().map(|p| p.ty.clone()).collect();
                ctx.funcs
                    .insert(func.name.clone(), (param_tys, func.ret_ty.clone()));
            }
            TopLevelItem::Declaration(decl) => {
                let param_tys = decl.params.iter().map(|p| p.ty.clone()).collect();
                ctx.funcs
                    .insert(decl.name.clone(), (param_tys, decl.ret_ty.clone()));
            }
            TopLevelItem::Variable(var) => {
                validate_global_var(var, &mut ctx)?;
            }
        }
    }
    for item in &ast.program.top_level_items {
        match item {
            TopLevelItem::Function(func) => {
                ctx.current_return = func.ret_ty.clone();
                let saved = ctx.objects.clone();
                for param in &func.params {
                    ctx.objects.insert(param.name.clone(), param.ty.clone());
                }
                if let Some(body) = &func.body {
                    check_block(body, &mut ctx)?;
                }
                ctx.objects = saved;
            }
            TopLevelItem::Declaration(decl) => validate_function_decl(decl)?,
            TopLevelItem::Variable(_) => {}
        }
    }
    Ok(TypedProgram {
        program: ast.program.clone(),
    })
}

fn validate_global_var(var: &GlobalVarDecl, ctx: &mut TypeCtx) -> Result<()> {
    if let Some(init) = &var.init {
        let init_ty = type_of_expr(init, ctx)?;
        if !can_assign(init, &init_ty, &var.ty) {
            bail!("type error: invalid static initializer for '{}'", var.name);
        }
    }
    ctx.objects.insert(var.name.clone(), var.ty.clone());
    Ok(())
}

fn validate_function_decl(decl: &GlobalDecl) -> Result<()> {
    for param in &decl.params {
        if matches!(param.ty, Type::Void) {
            bail!("type error: parameter '{}' has void type", param.name);
        }
    }
    Ok(())
}

fn check_block(items: &[BlockItem], ctx: &mut TypeCtx) -> Result<()> {
    for item in items {
        check_block_item(item, ctx)?;
    }
    Ok(())
}

fn check_block_item(item: &BlockItem, ctx: &mut TypeCtx) -> Result<()> {
    match item {
        BlockItem::Declaration(decl) => check_var_decl(decl, ctx),
        BlockItem::FunctionDecl(decl) => {
            validate_function_decl(decl)?;
            let param_tys = decl.params.iter().map(|p| p.ty.clone()).collect();
            ctx.funcs
                .insert(decl.name.clone(), (param_tys, decl.ret_ty.clone()));
            Ok(())
        }
        BlockItem::Statement(stmt) => check_statement(stmt, ctx),
    }
}

fn check_var_decl(decl: &VarDecl, ctx: &mut TypeCtx) -> Result<()> {
    ctx.objects.insert(decl.name.clone(), decl.ty.clone());
    if let Some(init) = &decl.init {
        let init_ty = type_of_expr(init, ctx)?;
        if !can_assign(init, &init_ty, &decl.ty) {
            bail!("type error: invalid initializer for '{}'", decl.name);
        }
    }
    Ok(())
}

fn check_statement(stmt: &Statement, ctx: &mut TypeCtx) -> Result<()> {
    match stmt {
        Statement::Return(expr) => {
            let expr_ty = type_of_expr(expr, ctx)?;
            if !can_assign(expr, &expr_ty, &ctx.current_return) {
                bail!("type error: return expression has incompatible type");
            }
            Ok(())
        }
        Statement::Block(items) => check_block(items, ctx),
        Statement::While {
            condition, body, ..
        }
        | Statement::DoWhile {
            condition, body, ..
        } => {
            type_of_scalar(condition, ctx)?;
            check_statement(body, ctx)
        }
        Statement::For {
            init,
            condition,
            post,
            body,
            ..
        } => {
            if let Some(init) = init {
                match init {
                    ForInit::Declaration(decl) => check_var_decl(decl, ctx)?,
                    ForInit::Expr(expr) => {
                        type_of_expr(expr, ctx)?;
                    }
                }
            }
            if let Some(condition) = condition {
                type_of_scalar(condition, ctx)?;
            }
            if let Some(post) = post {
                type_of_expr(post, ctx)?;
            }
            check_statement(body, ctx)
        }
        Statement::Switch { expr, body, .. } => {
            let ty = type_of_expr(expr, ctx)?;
            if !ty.clone().is_integer() {
                bail!("type error: switch expression must be integer");
            }
            check_statement(body, ctx)
        }
        Statement::Case { value, statement } => {
            let ty = type_of_expr(value, ctx)?;
            if !ty.clone().is_integer() {
                bail!("type error: case value must be integer");
            }
            check_statement(statement, ctx)
        }
        Statement::Default { statement } | Statement::Labeled { statement, .. } => {
            check_statement(statement, ctx)
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            type_of_scalar(condition, ctx)?;
            check_statement(then_branch, ctx)?;
            if let Some(else_branch) = else_branch {
                check_statement(else_branch, ctx)?;
            }
            Ok(())
        }
        Statement::Expr(Some(expr)) => {
            type_of_expr(expr, ctx)?;
            Ok(())
        }
        Statement::Expr(None)
        | Statement::Break(_)
        | Statement::Continue(_)
        | Statement::Goto(_) => Ok(()),
    }
}

fn type_of_scalar(expr: &Expr, ctx: &TypeCtx) -> Result<Type> {
    let ty = type_of_expr(expr, ctx)?;
    if matches!(ty, Type::Void) {
        bail!("type error: scalar expression expected");
    }
    Ok(ty)
}

fn type_of_expr(expr: &Expr, ctx: &TypeCtx) -> Result<Type> {
    match expr {
        Expr::Constant(_) => Ok(Type::Int),
        Expr::LongConstant(_) => Ok(Type::Long),
        Expr::UIntConstant(_, is_long) => Ok(if *is_long {
            Type::UnsignedLong
        } else {
            Type::UnsignedInt
        }),
        Expr::DoubleConstant(_) => Ok(Type::Double),
        Expr::Var(name) => ctx
            .objects
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("type error: unknown object '{name}'")),
        Expr::Paren(inner) => type_of_expr(inner, ctx),
        Expr::Cast { target_type, expr } => {
            let source = type_of_expr(expr, ctx)?;
            validate_cast(&source, target_type)?;
            Ok(target_type.clone())
        }
        Expr::Unary { op, expr } => type_unary(*op, expr, ctx),
        Expr::PreInc(inner) | Expr::PreDec(inner) | Expr::PostInc(inner) | Expr::PostDec(inner) => {
            if !inner.is_lvalue() {
                bail!("type error: increment/decrement target is not an lvalue");
            }
            let ty = type_of_expr(inner, ctx)?;
            if !ty.clone().is_integer() && !matches!(ty, Type::Double) {
                bail!("type error: increment/decrement target must be arithmetic");
            }
            Ok(ty)
        }
        Expr::Assign { op, target, value } => type_assignment(*op, target, value, ctx),
        Expr::Conditional {
            condition,
            then_expr,
            else_expr,
        } => {
            type_of_scalar(condition, ctx)?;
            let then_ty = type_of_expr(then_expr, ctx)?;
            let else_ty = type_of_expr(else_expr, ctx)?;
            common_type(then_expr, &then_ty, else_expr, &else_ty)
        }
        Expr::Binary { op, left, right } => type_binary(*op, left, right, ctx),
        Expr::Call { name, args } => {
            let (params, ret) = ctx
                .funcs
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("type error: unknown function '{name}'"))?;
            if params.len() != args.len() {
                bail!("type error: wrong number of arguments to '{name}'");
            }
            for (arg, param_ty) in args.iter().zip(params.iter()) {
                let arg_ty = type_of_expr(arg, ctx)?;
                if !can_assign(arg, &arg_ty, param_ty) {
                    bail!("type error: argument to '{name}' has incompatible type");
                }
            }
            Ok(ret)
        }
        Expr::AddressOf(inner) => {
            if !inner.is_lvalue() {
                bail!("type error: cannot take address of non-lvalue");
            }
            Ok(Type::Pointer(Box::new(type_of_expr(inner, ctx)?)))
        }
        Expr::Dereference(inner) => match type_of_expr(inner, ctx)? {
            Type::Pointer(pointee) if !matches!(*pointee, Type::Void) => Ok(*pointee),
            Type::Pointer(_) => bail!("type error: cannot dereference void pointer"),
            _ => bail!("type error: cannot dereference non-pointer"),
        },
        Expr::Subscript { .. } => bail!("type error: arrays/subscript are out of chapter-14 scope"),
    }
}

fn type_unary(op: UnaryOp, expr: &Expr, ctx: &TypeCtx) -> Result<Type> {
    let ty = type_of_expr(expr, ctx)?;
    match op {
        UnaryOp::Not => Ok(Type::Int),
        UnaryOp::Negate => {
            if ty.clone().is_integer() || matches!(ty, Type::Double) {
                Ok(ty)
            } else {
                bail!("type error: cannot negate pointer")
            }
        }
        UnaryOp::Complement => {
            if ty.clone().is_integer() {
                Ok(ty)
            } else {
                bail!("type error: bitwise complement requires integer")
            }
        }
    }
}

fn type_assignment(op: AssignOp, target: &Expr, value: &Expr, ctx: &TypeCtx) -> Result<Type> {
    if !target.is_lvalue() {
        bail!("type error: assignment target is not an lvalue");
    }
    let target_ty = type_of_expr(target, ctx)?;
    let value_ty = type_of_expr(value, ctx)?;
    if op == AssignOp::Assign {
        if !can_assign(value, &value_ty, &target_ty) {
            bail!("type error: assignment has incompatible types");
        }
        return Ok(target_ty);
    }
    let result = type_binary(compound_to_binary(op)?, target, value, ctx)?;
    if !can_assign(value, &result, &target_ty) {
        bail!("type error: compound assignment has incompatible types");
    }
    Ok(target_ty)
}

fn type_binary(op: BinaryOp, left: &Expr, right: &Expr, ctx: &TypeCtx) -> Result<Type> {
    let left_ty = type_of_expr(left, ctx)?;
    let right_ty = type_of_expr(right, ctx)?;
    match op {
        BinaryOp::Equal | BinaryOp::NotEqual => {
            comparable(left, &left_ty, right, &right_ty)?;
            Ok(Type::Int)
        }
        BinaryOp::LessThan
        | BinaryOp::LessOrEqual
        | BinaryOp::GreaterThan
        | BinaryOp::GreaterOrEqual => {
            if matches!(left_ty, Type::Pointer(_)) || matches!(right_ty, Type::Pointer(_)) {
                if left_ty == right_ty && matches!(left_ty, Type::Pointer(_)) {
                    return Ok(Type::Int);
                }
                bail!("type error: ordered pointer comparison requires matching pointer types");
            }
            common_arithmetic(&left_ty, &right_ty)?;
            Ok(Type::Int)
        }
        BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
            ensure_scalar(&left_ty)?;
            ensure_scalar(&right_ty)?;
            Ok(Type::Int)
        }
        BinaryOp::Add | BinaryOp::Subtract => common_arithmetic(&left_ty, &right_ty),
        BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => {
            if matches!(left_ty, Type::Pointer(_)) || matches!(right_ty, Type::Pointer(_)) {
                bail!("type error: arithmetic operator cannot be applied to pointer");
            }
            common_arithmetic(&left_ty, &right_ty)
        }
        BinaryOp::ShiftLeft
        | BinaryOp::ShiftRight
        | BinaryOp::BitwiseAnd
        | BinaryOp::BitwiseXor
        | BinaryOp::BitwiseOr => {
            if left_ty.clone().is_integer() && right_ty.clone().is_integer() {
                common_arithmetic(&left_ty, &right_ty)
            } else {
                bail!("type error: bitwise operator requires integer operands")
            }
        }
    }
}

fn compound_to_binary(op: AssignOp) -> Result<BinaryOp> {
    match op {
        AssignOp::Add => Ok(BinaryOp::Add),
        AssignOp::Subtract => Ok(BinaryOp::Subtract),
        AssignOp::Multiply => Ok(BinaryOp::Multiply),
        AssignOp::Divide => Ok(BinaryOp::Divide),
        AssignOp::Remainder => Ok(BinaryOp::Remainder),
        AssignOp::ShiftLeft => Ok(BinaryOp::ShiftLeft),
        AssignOp::ShiftRight => Ok(BinaryOp::ShiftRight),
        AssignOp::BitwiseAnd => Ok(BinaryOp::BitwiseAnd),
        AssignOp::BitwiseXor => Ok(BinaryOp::BitwiseXor),
        AssignOp::BitwiseOr => Ok(BinaryOp::BitwiseOr),
        AssignOp::Assign => bail!("type error: plain assignment is not compound"),
    }
}

fn validate_cast(source: &Type, target: &Type) -> Result<()> {
    if matches!(source, Type::Double) && matches!(target, Type::Pointer(_)) {
        bail!("type error: cannot cast double to pointer");
    }
    if matches!(source, Type::Pointer(_)) && matches!(target, Type::Double) {
        bail!("type error: cannot cast pointer to double");
    }
    Ok(())
}

fn can_assign(expr: &Expr, source: &Type, target: &Type) -> bool {
    source == target
        || (is_null_pointer_constant(expr) && matches!(target, Type::Pointer(_)))
        || (source.clone().is_integer() && target.clone().is_integer())
        || ((source.clone().is_integer() || matches!(source, Type::Double))
            && (target.clone().is_integer() || matches!(target, Type::Double))
            && !matches!(target, Type::Pointer(_)))
}

fn is_null_pointer_constant(expr: &Expr) -> bool {
    match expr {
        Expr::Constant(0) | Expr::LongConstant(0) | Expr::UIntConstant(0, _) => true,
        Expr::Paren(inner) | Expr::Cast { expr: inner, .. } => is_null_pointer_constant(inner),
        _ => false,
    }
}

fn comparable(left: &Expr, left_ty: &Type, right: &Expr, right_ty: &Type) -> Result<()> {
    if left_ty == right_ty {
        return Ok(());
    }
    if matches!(left_ty, Type::Pointer(_)) && is_null_pointer_constant(right) {
        return Ok(());
    }
    if matches!(right_ty, Type::Pointer(_)) && is_null_pointer_constant(left) {
        return Ok(());
    }
    if left_ty.clone().is_integer() && right_ty.clone().is_integer() {
        return Ok(());
    }
    if matches!(left_ty, Type::Double) || matches!(right_ty, Type::Double) {
        if !matches!(left_ty, Type::Pointer(_)) && !matches!(right_ty, Type::Pointer(_)) {
            return Ok(());
        }
    }
    bail!("type error: incompatible comparison operands")
}

fn common_type(left: &Expr, left_ty: &Type, right: &Expr, right_ty: &Type) -> Result<Type> {
    if left_ty == right_ty {
        return Ok(left_ty.clone());
    }
    if matches!(left_ty, Type::Pointer(_)) && is_null_pointer_constant(right) {
        return Ok(left_ty.clone());
    }
    if matches!(right_ty, Type::Pointer(_)) && is_null_pointer_constant(left) {
        return Ok(right_ty.clone());
    }
    common_arithmetic(left_ty, right_ty)
}

fn common_arithmetic(left: &Type, right: &Type) -> Result<Type> {
    if matches!(left, Type::Pointer(_)) || matches!(right, Type::Pointer(_)) {
        bail!("type error: pointer is not an arithmetic operand");
    }
    if matches!(left, Type::Double) || matches!(right, Type::Double) {
        Ok(Type::Double)
    } else if matches!(left, Type::UnsignedLong) || matches!(right, Type::UnsignedLong) {
        Ok(Type::UnsignedLong)
    } else if matches!(left, Type::Long) || matches!(right, Type::Long) {
        Ok(Type::Long)
    } else if matches!(left, Type::UnsignedInt) || matches!(right, Type::UnsignedInt) {
        Ok(Type::UnsignedInt)
    } else {
        Ok(Type::Int)
    }
}

fn ensure_scalar(ty: &Type) -> Result<()> {
    if matches!(ty, Type::Void | Type::Array { .. }) {
        bail!("type error: scalar expression expected");
    }
    Ok(())
}
