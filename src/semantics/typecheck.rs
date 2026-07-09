//! Type-checking pass.
//!
//! Mirrors the chapter-14 pointer rules from
//! `nqcc2/lib/semantic_analysis/typecheck.ml`: declarations retain their
//! parsed types, expressions are checked against a scoped symbol table, and
//! pointer lvalue/conversion rules are enforced before lowering.

use std::collections::HashMap;

use anyhow::{Result, bail};

use crate::ast::{
    AggregateKind, AssignOp, BinaryOp, BlockItem, Expr, ForInit, GlobalDecl, GlobalVarDecl,
    Program, Statement, StorageClass, StructDecl, TopLevelItem, Type, UnaryOp, VarDecl,
};
use crate::codegen::type_table::{self, MemberEntry, StructEntry};

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
    type_table::reset();
    let mut ctx = TypeCtx {
        objects: HashMap::new(),
        funcs: HashMap::new(),
        current_return: Type::Int,
    };
    for item in &ast.program.top_level_items {
        match item {
            TopLevelItem::Function(func) => {
                let param_tys: Vec<Type> = func.params.iter().map(|p| p.ty.clone()).collect();
                validate_function_signature(&func.name, &param_tys, &func.ret_ty, &ctx)?;
                if func.body.is_some()
                    && !matches!(func.ret_ty, Type::Void)
                    && !func.ret_ty.clone().is_complete()
                {
                    bail!("type error: function definition has incomplete return type");
                }
                if func.body.is_some() {
                    for param_ty in &param_tys {
                        if !param_ty.clone().is_complete() {
                            bail!("type error: function definition has incomplete parameter type");
                        }
                    }
                }
                ctx.funcs
                    .insert(func.name.clone(), (param_tys, func.ret_ty.clone()));
            }
            TopLevelItem::Declaration(decl) => {
                let param_tys: Vec<Type> = decl.params.iter().map(|p| p.ty.clone()).collect();
                validate_function_signature(&decl.name, &param_tys, &decl.ret_ty, &ctx)?;
                ctx.funcs
                    .insert(decl.name.clone(), (param_tys, decl.ret_ty.clone()));
            }
            TopLevelItem::Variable(var) => {
                validate_global_var(var, &mut ctx)?;
            }
            TopLevelItem::StructDecl(sd) => {
                typecheck_struct_decl(sd)?;
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
            TopLevelItem::Variable(_) | TopLevelItem::StructDecl(_) => {}
        }
    }
    Ok(TypedProgram {
        program: ast.program.clone(),
    })
}

fn validate_function_signature(
    name: &str,
    param_tys: &[Type],
    ret_ty: &Type,
    ctx: &TypeCtx,
) -> Result<()> {
    for param_ty in param_tys {
        if matches!(param_ty, Type::Void) {
            bail!("type error: function parameter has void type");
        }
        validate_type(param_ty)?;
    }
    validate_type(ret_ty)?;
    if let Some((existing_params, existing_ret)) = ctx.funcs.get(name)
        && (existing_params.as_slice() != param_tys || existing_ret != ret_ty)
    {
        bail!("type error: conflicting declarations for function '{name}'");
    }
    Ok(())
}

fn validate_global_var(var: &GlobalVarDecl, ctx: &mut TypeCtx) -> Result<()> {
    if var.storage == StorageClass::Extern {
        validate_type(&var.ty)?;
    } else {
        validate_object_type(&var.ty)?;
    }
    if let Some(existing) = ctx.objects.get(&var.name)
        && existing != &var.ty
    {
        bail!("type error: conflicting declarations for '{}'", var.name);
    }
    if let Some(init) = &var.init {
        if matches!(
            var.ty,
            Type::Array { .. } | Type::Struct(_) | Type::Union(_)
        ) {
            if var.storage == StorageClass::Static && !initializer_is_constant(init) {
                bail!("type error: static array initializer must be constant");
            }
            validate_initializer(init, &var.ty, ctx)?;
            ctx.objects.insert(var.name.clone(), var.ty.clone());
            return Ok(());
        }
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
        BlockItem::StructDecl(sd) => typecheck_struct_decl(sd),
        BlockItem::FunctionDecl(decl) => {
            let param_tys: Vec<Type> = decl.params.iter().map(|p| p.ty.clone()).collect();
            validate_function_signature(&decl.name, &param_tys, &decl.ret_ty, ctx)?;
            ctx.funcs
                .insert(decl.name.clone(), (param_tys, decl.ret_ty.clone()));
            Ok(())
        }
        BlockItem::Statement(stmt) => check_statement(stmt, ctx),
    }
}

fn check_var_decl(decl: &VarDecl, ctx: &mut TypeCtx) -> Result<()> {
    if decl.storage == StorageClass::Extern {
        validate_type(&decl.ty)?;
    } else {
        validate_object_type(&decl.ty)?;
    }
    if decl.storage == StorageClass::Extern
        && let Some(existing) = ctx.objects.get(&decl.name)
        && existing != &decl.ty
    {
        bail!("type error: conflicting declarations for '{}'", decl.name);
    }
    ctx.objects.insert(decl.name.clone(), decl.ty.clone());
    if let Some(init) = &decl.init {
        if matches!(
            decl.ty,
            Type::Array { .. } | Type::Struct(_) | Type::Union(_)
        ) {
            if decl.storage == StorageClass::Static && !initializer_is_constant(init) {
                bail!("type error: static array initializer must be constant");
            }
            validate_initializer(init, &decl.ty, ctx)?;
            return Ok(());
        }
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
            match expr {
                Some(expr) => {
                    if matches!(ctx.current_return, Type::Void) {
                        bail!("type error: function with void return type cannot return a value");
                    }
                    let expr_ty = type_of_expr(expr, ctx)?;
                    if !can_assign(expr, &expr_ty, &ctx.current_return) {
                        bail!("type error: return expression has incompatible type");
                    }
                }
                None => {
                    if !matches!(ctx.current_return, Type::Void) {
                        bail!("type error: function with non-void return type must return a value");
                    }
                }
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
                        let ty = type_of_expr(expr, ctx)?;
                        if !matches!(ty, Type::Void) && !ty.is_complete() {
                            bail!("type error: full expression has incomplete type");
                        }
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
            let ty = type_of_expr(expr, ctx)?;
            if !matches!(ty, Type::Void) && !ty.is_complete() {
                bail!("type error: full expression has incomplete type");
            }
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
    if !is_scalar_type(&ty) {
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
        Expr::StringLiteral(value) => Ok(Type::Array {
            element: Box::new(Type::Char),
            size: Some(value.len() + 1),
        }),
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
        Expr::SizeOfExpr(inner) => {
            let ty = type_of_expr(inner, ctx)?;
            if !ty.is_complete() {
                bail!("type error: cannot apply sizeof to incomplete type");
            }
            Ok(Type::UnsignedLong)
        }
        Expr::SizeOfType(ty) => {
            validate_type(ty)?;
            if !ty.clone().is_complete() {
                bail!("type error: cannot apply sizeof to incomplete type");
            }
            Ok(Type::UnsignedLong)
        }
        Expr::Unary { op, expr } => type_unary(*op, expr, ctx),
        Expr::PreInc(inner) | Expr::PreDec(inner) | Expr::PostInc(inner) | Expr::PostDec(inner) => {
            if !inner.is_lvalue() {
                bail!("type error: increment/decrement target is not an lvalue");
            }
            let ty = type_of_expr(inner, ctx)?;
            match ty.clone().decay() {
                Type::Pointer(pointee) if matches!(*pointee, Type::Void) => {
                    bail!("type error: cannot increment/decrement void pointer");
                }
                Type::Pointer(_)
                | Type::Int
                | Type::Long
                | Type::UnsignedInt
                | Type::UnsignedLong
                | Type::Char
                | Type::SignedChar
                | Type::UnsignedChar
                | Type::Double => {}
                _ => bail!("type error: increment/decrement target must be scalar"),
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
                if !arg_ty.clone().is_complete() {
                    bail!("type error: argument to '{name}' has incomplete type");
                }
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
        Expr::Dereference(inner) => match type_of_expr(inner, ctx)?.decay() {
            Type::Pointer(pointee) if !matches!(*pointee, Type::Void) => Ok(*pointee),
            Type::Pointer(_) => bail!("type error: cannot dereference void pointer"),
            _ => bail!("type error: cannot dereference non-pointer"),
        },
        Expr::Subscript { base, index } => {
            let base_ty = type_of_expr(base, ctx)?.decay();
            let index_ty = type_of_expr(index, ctx)?.decay();
            match (base_ty, index_ty) {
                (Type::Pointer(pointee), index_ty) if index_ty.clone().is_integer() => {
                    if matches!(*pointee, Type::Void) {
                        bail!("type error: cannot subscript void pointer");
                    }
                    Ok(*pointee)
                }
                (base_ty, Type::Pointer(pointee)) if base_ty.clone().is_integer() => {
                    if matches!(*pointee, Type::Void) {
                        bail!("type error: cannot subscript void pointer");
                    }
                    Ok(*pointee)
                }
                (Type::Pointer(_), _) | (_, Type::Pointer(_)) => {
                    bail!("type error: subscript index must be integer")
                }
                _ => bail!("type error: subscript base must be pointer or array"),
            }
        }
        Expr::Dot { structure, member } => {
            let structure_ty = type_of_expr(structure, ctx)?;
            let tag = match structure_ty {
                Type::Struct(tag) | Type::Union(tag) => tag,
                _ => bail!("type error: dot operator requires aggregate type"),
            };
            type_table::member(&tag, member)
                .map(|entry| entry.member_type)
                .ok_or_else(|| anyhow::anyhow!("type error: aggregate has no member '{member}'"))
        }
        Expr::Arrow { structure, member } => {
            let structure_ty = type_of_expr(structure, ctx)?.decay();
            let Type::Pointer(pointee) = structure_ty else {
                bail!("type error: arrow operator requires pointer to aggregate");
            };
            let tag = match *pointee {
                Type::Struct(tag) | Type::Union(tag) => tag,
                _ => bail!("type error: arrow operator requires pointer to aggregate"),
            };
            type_table::member(&tag, member)
                .map(|entry| entry.member_type)
                .ok_or_else(|| anyhow::anyhow!("type error: aggregate has no member '{member}'"))
        }
        Expr::InitializerList(_) => {
            bail!("type error: initializer list is only valid in an initializer")
        }
    }
}

fn type_unary(op: UnaryOp, expr: &Expr, ctx: &TypeCtx) -> Result<Type> {
    let ty = type_of_expr(expr, ctx)?;
    match op {
        UnaryOp::Not if !is_scalar_type(&ty) => {
            bail!("type error: logical not requires scalar operand")
        }
        UnaryOp::Not => Ok(Type::Int),
        UnaryOp::Negate => {
            if ty.clone().is_integer() || matches!(ty, Type::Double) {
                Ok(promote_char_type(ty))
            } else {
                bail!("type error: cannot negate pointer")
            }
        }
        UnaryOp::Complement => {
            if ty.clone().is_integer() {
                Ok(promote_char_type(ty))
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
    if matches!(target_ty, Type::Array { .. }) {
        bail!("type error: cannot assign to array object");
    }
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
    let left_ty = type_of_expr(left, ctx)?.decay();
    let right_ty = type_of_expr(right, ctx)?.decay();
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
        BinaryOp::Add => match (&left_ty, &right_ty) {
            (Type::Pointer(pointee), _) if right_ty.clone().is_integer() => {
                validate_complete_pointee(pointee)?;
                Ok(left_ty)
            }
            (_, Type::Pointer(pointee)) if left_ty.clone().is_integer() => {
                validate_complete_pointee(pointee)?;
                Ok(right_ty)
            }
            (Type::Pointer(_), _) | (_, Type::Pointer(_)) => {
                bail!("type error: pointer addition requires integer offset")
            }
            _ => common_arithmetic(&left_ty, &right_ty),
        },
        BinaryOp::Subtract => match (&left_ty, &right_ty) {
            (Type::Pointer(pointee), _) if right_ty.clone().is_integer() => {
                validate_complete_pointee(pointee)?;
                Ok(left_ty)
            }
            (Type::Pointer(left_pointee), Type::Pointer(right_pointee))
                if left_pointee == right_pointee =>
            {
                validate_complete_pointee(left_pointee)?;
                Ok(Type::Long)
            }
            (Type::Pointer(_), _) | (_, Type::Pointer(_)) => {
                bail!("type error: invalid pointer subtraction")
            }
            _ => common_arithmetic(&left_ty, &right_ty),
        },
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
    let source = source.clone().decay();
    validate_type(target)?;
    if matches!(source, Type::Double) && matches!(target, Type::Pointer(_)) {
        bail!("type error: cannot cast double to pointer");
    }
    if matches!(source, Type::Pointer(_)) && matches!(target, Type::Double) {
        bail!("type error: cannot cast pointer to double");
    }
    if matches!(target, Type::Void) {
        if matches!(source, Type::Void) || source.clone().is_complete() {
            return Ok(());
        }
        bail!("type error: cannot cast incomplete type");
    }
    if !source.clone().is_complete() {
        bail!("type error: cannot cast incomplete type");
    }
    if !is_scalar_type(&source) {
        bail!("type error: can only cast scalar expressions to non-void type");
    }
    if !is_scalar_type(target) {
        bail!("type error: can only cast to scalar types or void");
    }
    Ok(())
}

fn can_assign(expr: &Expr, source: &Type, target: &Type) -> bool {
    let source = source.clone().decay();
    let target = target.clone();
    source == target
        || (is_null_pointer_constant(expr) && matches!(target, Type::Pointer(_)))
        || (matches!((&source, &target), (Type::Pointer(a), Type::Pointer(b)) if a == b))
        || (matches!((&source, &target), (Type::Pointer(a), Type::Pointer(_)) if matches!(**a, Type::Void)))
        || (matches!((&source, &target), (Type::Pointer(_), Type::Pointer(b)) if matches!(**b, Type::Void)))
        || matches!((&source, &target), (Type::Struct(a), Type::Struct(b)) if a == b)
        || matches!((&source, &target), (Type::Union(a), Type::Union(b)) if a == b)
        || (source.clone().is_integer() && target.clone().is_integer())
        || ((source.clone().is_integer() || matches!(source, Type::Double))
            && (target.clone().is_integer() || matches!(target, Type::Double))
            && !matches!(target, Type::Pointer(_)))
}

fn validate_initializer(init: &Expr, target: &Type, ctx: &TypeCtx) -> Result<()> {
    match (target, init) {
        (
            Type::Array {
                element,
                size: Some(size),
            },
            Expr::StringLiteral(value),
        ) if is_char_type(element) => {
            if value.len() > *size {
                bail!("type error: too many characters in string literal");
            }
            Ok(())
        }
        (
            Type::Array {
                element,
                size: Some(size),
            },
            Expr::InitializerList(items),
        ) => {
            if items.len() > *size {
                bail!("type error: too many array initializers");
            }
            for item in items {
                validate_initializer(item, element, ctx)?;
            }
            Ok(())
        }
        (Type::Array { .. }, _) => bail!("type error: scalar initializer for array"),
        (Type::Struct(tag), Expr::InitializerList(items)) => {
            let members = type_table::members_in_order(tag);
            if items.len() > members.len() {
                bail!("type error: too many struct initializers");
            }
            for (item, member) in items.iter().zip(members.iter()) {
                validate_initializer(item, &member.member_type, ctx)?;
            }
            Ok(())
        }
        (Type::Union(tag), Expr::InitializerList(items)) => {
            if items.len() != 1 {
                bail!("type error: union initializer must have exactly one element");
            }
            let first = type_table::members_in_order(tag)
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("type error: union has no members"))?;
            validate_initializer(&items[0], &first.member_type, ctx)
        }
        (_, Expr::InitializerList(_)) => bail!("type error: initializer list for scalar"),
        (target, expr) => {
            let source = type_of_expr(expr, ctx)?;
            if can_assign(expr, &source, target) {
                Ok(())
            } else {
                bail!("type error: initializer has incompatible type")
            }
        }
    }
}

fn initializer_is_constant(init: &Expr) -> bool {
    match init {
        Expr::Constant(_)
        | Expr::LongConstant(_)
        | Expr::UIntConstant(_, _)
        | Expr::DoubleConstant(_)
        | Expr::StringLiteral(_) => true,
        Expr::InitializerList(items) => items.iter().all(initializer_is_constant),
        _ => false,
    }
}

fn is_null_pointer_constant(expr: &Expr) -> bool {
    match expr {
        Expr::Constant(0) | Expr::LongConstant(0) | Expr::UIntConstant(0, _) => true,
        Expr::Paren(inner) => is_null_pointer_constant(inner),
        _ => false,
    }
}

fn comparable(left: &Expr, left_ty: &Type, right: &Expr, right_ty: &Type) -> Result<()> {
    let left_ty = left_ty.clone().decay();
    let right_ty = right_ty.clone().decay();
    if left_ty == right_ty {
        if !is_scalar_type(&left_ty) {
            bail!("type error: cannot compare void expressions");
        }
        return Ok(());
    }
    if matches!(left_ty, Type::Pointer(_)) && is_null_pointer_constant(right) {
        return Ok(());
    }
    if matches!(right_ty, Type::Pointer(_)) && is_null_pointer_constant(left) {
        return Ok(());
    }
    if matches!((&left_ty, &right_ty), (Type::Pointer(a), Type::Pointer(_)) if matches!(**a, Type::Void))
        || matches!((&left_ty, &right_ty), (Type::Pointer(_), Type::Pointer(b)) if matches!(**b, Type::Void))
    {
        return Ok(());
    }
    if left_ty.clone().is_integer() && right_ty.clone().is_integer() {
        return Ok(());
    }
    if (matches!(left_ty, Type::Double) || matches!(right_ty, Type::Double))
        && !matches!(left_ty, Type::Pointer(_))
        && !matches!(right_ty, Type::Pointer(_))
    {
        return Ok(());
    }
    bail!("type error: incompatible comparison operands")
}

fn common_type(left: &Expr, left_ty: &Type, right: &Expr, right_ty: &Type) -> Result<Type> {
    let left_ty = left_ty.clone().decay();
    let right_ty = right_ty.clone().decay();
    if left_ty == right_ty {
        return Ok(left_ty);
    }
    if matches!(left_ty, Type::Void) || matches!(right_ty, Type::Void) {
        bail!("type error: conditional operands have incompatible void type");
    }
    if matches!(left_ty, Type::Pointer(_)) && is_null_pointer_constant(right) {
        return Ok(left_ty.clone());
    }
    if matches!(right_ty, Type::Pointer(_)) && is_null_pointer_constant(left) {
        return Ok(right_ty.clone());
    }
    if matches!((&left_ty, &right_ty), (Type::Pointer(a), Type::Pointer(_)) if matches!(**a, Type::Void))
        || matches!((&left_ty, &right_ty), (Type::Pointer(_), Type::Pointer(b)) if matches!(**b, Type::Void))
    {
        return Ok(Type::Pointer(Box::new(Type::Void)));
    }
    common_arithmetic(&left_ty, &right_ty)
}

fn common_arithmetic(left: &Type, right: &Type) -> Result<Type> {
    let left = promote_char_type(left.clone());
    let right = promote_char_type(right.clone());
    if !is_arithmetic_type(&left) || !is_arithmetic_type(&right) {
        bail!("type error: arithmetic operands expected");
    }
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

fn promote_char_type(ty: Type) -> Type {
    match ty {
        Type::Char | Type::SignedChar | Type::UnsignedChar => Type::Int,
        other => other,
    }
}

fn is_char_type(ty: &Type) -> bool {
    matches!(ty, Type::Char | Type::SignedChar | Type::UnsignedChar)
}

fn ensure_scalar(ty: &Type) -> Result<()> {
    if !is_scalar_type(ty) {
        bail!("type error: scalar expression expected");
    }
    Ok(())
}

fn is_scalar_type(ty: &Type) -> bool {
    let ty = ty.clone().decay();
    matches!(ty, Type::Pointer(_)) || ty.clone().is_integer() || matches!(ty, Type::Double)
}

fn is_arithmetic_type(ty: &Type) -> bool {
    ty.clone().is_integer() || matches!(ty, Type::Double)
}

fn validate_complete_pointee(pointee: &Type) -> Result<()> {
    if !pointee.clone().is_complete() {
        bail!("type error: pointer arithmetic requires complete pointed-to type");
    }
    Ok(())
}

fn validate_type(ty: &Type) -> Result<()> {
    match ty {
        Type::Array { element, .. } => validate_object_type(element),
        Type::Pointer(pointee) => validate_type(pointee),
        Type::Struct(tag) if type_table::is_declared(tag) => Ok(()),
        Type::Struct(tag) => bail!("type error: undeclared struct tag '{tag}'"),
        Type::Union(tag) if type_table::is_declared(tag) => Ok(()),
        Type::Union(tag) => bail!("type error: undeclared union tag '{tag}'"),
        _ => Ok(()),
    }
}

fn validate_object_type(ty: &Type) -> Result<()> {
    match ty {
        Type::Array { element, size } => {
            if size.is_none() {
                bail!("type error: array object has incomplete type");
            }
            validate_object_type(element)
        }
        Type::Pointer(pointee) => validate_type(pointee),
        Type::Void => bail!("type error: object has void type"),
        Type::Struct(_) if !ty.clone().is_complete() => {
            bail!("type error: object has incomplete struct type")
        }
        Type::Union(_) if !ty.clone().is_complete() => {
            bail!("type error: object has incomplete union type")
        }
        _ => Ok(()),
    }
}

fn typecheck_struct_decl(sd: &StructDecl) -> Result<()> {
    if sd.members.is_empty() {
        type_table::declare(&sd.tag);
        return Ok(());
    }
    if type_table::contains(&sd.tag) {
        bail!(
            "type error: duplicate {} definition '{}'",
            sd.kind.keyword(),
            sd.tag
        );
    }
    type_table::declare(&sd.tag);
    let mut offset = 0;
    let mut alignment = 1;
    let mut members = std::collections::HashMap::new();
    let mut order = Vec::new();
    for member in &sd.members {
        if members.contains_key(&member.name) {
            bail!("type error: duplicate aggregate member '{}'", member.name);
        }
        validate_object_type(&member.ty)?;
        let member_alignment = member.ty.clone().alignment();
        let member_offset = match sd.kind {
            AggregateKind::Struct => {
                offset = round_up(offset, member_alignment);
                offset
            }
            AggregateKind::Union => 0,
        };
        members.insert(
            member.name.clone(),
            MemberEntry {
                member_type: member.ty.clone(),
                offset: member_offset,
            },
        );
        order.push(member.name.clone());
        alignment = alignment.max(member_alignment);
        match sd.kind {
            AggregateKind::Struct => offset += member.ty.clone().size(),
            AggregateKind::Union => offset = offset.max(member.ty.clone().size()),
        }
    }
    let size = round_up(offset, alignment);
    type_table::add(
        sd.tag.clone(),
        StructEntry {
            kind: sd.kind,
            size,
            alignment,
            members,
            order,
        },
    );
    Ok(())
}

fn round_up(value: i64, alignment: i64) -> i64 {
    ((value + alignment - 1) / alignment) * alignment
}
