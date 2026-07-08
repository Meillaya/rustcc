//! Recursive-descent parser for the supported native C subset.
//!
//! The parser consumes owned `Vec<Token>` values because tokens are inexpensive
//! records and ownership gives this phase freedom to advance by index without
//! lifetime plumbing.  Recursive AST forms use `Box` in the AST module, so parser
//! methods can build nested expressions and statements directly.  `Result` is
//! propagated with `?` so phase-specific parse failures retain the same driver
//! behavior as before extraction.
//!
//! Chapter 9 widens the surface from a single `int main(void) { ... }` shape
//! to a translation unit of multiple top-level function definitions and
//! declarations (forward declarations like `int foo(int x);`).  Mirrors
//! `nqcc2/lib/parse.ml` `parse_translation_unit` / `parse_program` for chapter 9.

// Mirrors nqcc2/lib/parse.ml chapter 9 grammar (~lines 1-50 of parse_program).
// Recursive-descent, Result-returning.

use anyhow::{Result, bail};

use crate::ast::{
    AssignOp, BinaryOp, BlockItem, Expr, ForInit, Function, GlobalDecl, GlobalVarDecl, Program,
    Statement, StorageClass, TopLevelItem, Type, UnaryOp, VarDecl,
};
use crate::lex::{Token, TokenKind};
use crate::parse::precedence::{Precedence, precedence_of};

pub(crate) fn parse_program(tokens: Vec<Token>) -> Result<Program> {
    Parser::new(tokens).parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    /// Chapter 9: parse a translation unit as a sequence of top-level
    /// function definitions and declarations.  Mirrors
    /// `nqcc2/lib/parse.ml` `parse_program` ~lines 952-961, which loops
    /// `parse_declaration` until EOF and wraps the result in
    /// `Ast.Program fun_decls`.
    fn parse_program(&mut self) -> Result<Program> {
        let mut top_level_items: Vec<TopLevelItem> = Vec::new();
        while !self.check(&TokenKind::Eof) {
            top_level_items.push(self.parse_top_level_item()?);
        }
        // Single trailing newline equivalent: consume EOF (already there).
        Ok(Program { top_level_items })
    }

    /// Parse a single top-level item.
    ///
    /// Chapter 9 only has function definitions and forward declarations of
    /// functions; chapter 10 widens this with file-scope variable
    /// declarations (`int g = 5;`, `static int h;`, `extern int k;`).  The
    /// sequence is:
    ///
    ///   `[static | extern] int NAME`
    ///
    /// followed by either `(` (function declaration/definition) or `=`/`;`
    /// (file-scope variable declaration).  Mirrors
    /// `nqcc2/lib/parse.ml` `parse_function_or_variable_declaration`
    /// (~lines 798-823) for chapter-10 surface.
    fn parse_top_level_item(&mut self) -> Result<TopLevelItem> {
        // Chapter 11: storage-class specifiers may appear in any
        // order relative to the type specifiers (`static int long`,
        // `int static long`, `long int static`).  Loop until
        // we've consumed both classes of tokens, accumulating the
        // storage class.
        let (ty, storage) = self.parse_specifiers_interleaved()?;
        let name = self.expect_identifier("function or variable name")?;
        if self.check(&TokenKind::OpenParen) {
            self.current += 1;
            let params = self.parse_param_list()?;
            self.expect_exact(&TokenKind::CloseParen, "')' after parameter list")?;
            if self.match_exact(&TokenKind::Semicolon) {
                Ok(TopLevelItem::Declaration(GlobalDecl {
                    name,
                    ret_ty: ty,
                    params,
                    storage,
                }))
            } else {
                self.expect_exact(&TokenKind::OpenBrace, "'{' to start function body")?;
                let mut body: Vec<BlockItem> = Vec::new();
                while !self.check(&TokenKind::CloseBrace) {
                    if self.check(&TokenKind::Eof) {
                        bail!("parse error: expected '}}' to close function body");
                    }
                    body.push(self.parse_block_item()?);
                }
                self.expect_exact(&TokenKind::CloseBrace, "'}' to close function body")?;
                Ok(TopLevelItem::Function(Function {
                    name,
                    ret_ty: ty,
                    params,
                    body: Some(body),
                    storage,
                }))
            }
        } else {
            let init = if self.match_exact(&TokenKind::Equal) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect_exact(&TokenKind::Semicolon, "';' after file-scope variable declaration")?;
            Ok(TopLevelItem::Variable(GlobalVarDecl {
                name,
                ty,
                init,
                storage,
            }))
        }
    }

    /// Chapter 11 helper: consume a sequence of type specifiers
    /// (`int` / `long`) and storage-class specifiers (`static` /
    /// `extern`) in any order, returning the resolved `Type` and
    /// `StorageClass`.  Rejects a duplicate storage class.
    fn parse_specifiers_interleaved(&mut self) -> Result<(Type, StorageClass)> {
        let mut saw_int = false;
        let mut is_long = false;
        let mut storage = StorageClass::Auto;
        let mut had_storage = false;
        loop {
            match self.peek().kind {
                TokenKind::Int => {
                    if saw_int {
                        bail!("parse error: duplicate 'int' in type specifier");
                    }
                    saw_int = true;
                    self.current += 1;
                }
                TokenKind::Long => {
                    is_long = true;
                    self.current += 1;
                }
                TokenKind::Static => {
                    if had_storage {
                        bail!("parse error: multiple storage-class specifiers in declaration");
                    }
                    storage = StorageClass::Static;
                    had_storage = true;
                    self.current += 1;
                }
                TokenKind::Extern => {
                    if had_storage {
                        bail!("parse error: multiple storage-class specifiers in declaration");
                    }
                    storage = StorageClass::Extern;
                    had_storage = true;
                    self.current += 1;
                }
                _ => break,
            }
        }
        if !saw_int && !is_long {
            bail!(
                "parse error: expected a type specifier ('int' or 'long'), found {:?}",
                self.peek().kind
            );
        }
        Ok((if is_long { Type::Long } else { Type::Int }, storage))
    }

    /// Parse an optional storage-class specifier at the start of a
    /// top-level item or a block-level declaration.  Returns `Auto` if
    /// no specifier is present.
    fn parse_optional_storage_class(&mut self) -> Result<StorageClass> {
        if self.match_exact(&TokenKind::Static) {
            Ok(StorageClass::Static)
        } else if self.match_exact(&TokenKind::Extern) {
            Ok(StorageClass::Extern)
        } else {
            Ok(StorageClass::Auto)
        }
    }

    /// Same as [`parse_optional_storage_class`] but used at top level
    /// where the specifier is the very first token (so we don't have to
    /// distinguish "no specifier" from "saw `int` and then no specifier").
    fn parse_optional_storage_class_top_level(&mut self) -> Result<StorageClass> {
        self.parse_optional_storage_class()
    }

    /// Parse a chapter-11 type specifier: a permutation of `int` and
    /// `long` (e.g. `long`, `int`, `long int`, `int long`).  At most
    /// one `int` and at most one `long` may appear; the resulting
    /// type is `long` if any `long` token was seen and `int`
    /// otherwise.  Mirrors the OCaml `parse_type_specifier_list` +
    /// `parse_type` combination for the chapter-11 surface.
    fn parse_type_specifier(&mut self) -> Result<Type> {
        let mut is_long = false;
        let mut saw_int = false;
        let mut saw_long = false;
        loop {
            match self.peek().kind {
                TokenKind::Int => {
                    if saw_int {
                        bail!("parse error: duplicate 'int' in type specifier");
                    }
                    saw_int = true;
                    self.current += 1;
                }
                TokenKind::Long => {
                    if saw_long {
                        bail!("parse error: duplicate 'long' in type specifier");
                    }
                    saw_long = true;
                    is_long = true;
                    self.current += 1;
                }
                _ => break,
            }
        }
        if !saw_int && !saw_long {
            bail!(
                "parse error: expected a type specifier ('int' or 'long'), found {:?}",
                self.peek().kind
            );
        }
        Ok(if is_long { Type::Long } else { Type::Int })
    }

    /// If a storage-class specifier follows `int`, combine it with
    /// `previous`.  Rejects `static int extern foo;` and friends that
    /// carry two specifiers at the same time.  Mirrors the OCaml
    /// partition of specifiers into type-specifiers and storage-class
    /// specifiers: if both buckets end up with more than one token the
    /// grammar is malformed.
    fn combine_storage_class(&mut self, previous: StorageClass) -> Result<StorageClass> {
        if self.match_exact(&TokenKind::Static) {
            if previous != StorageClass::Auto {
                bail!("parse error: multiple storage-class specifiers in declaration");
            }
            Ok(StorageClass::Static)
        } else if self.match_exact(&TokenKind::Extern) {
            if previous != StorageClass::Auto {
                bail!("parse error: multiple storage-class specifiers in declaration");
            }
            Ok(StorageClass::Extern)
        } else {
            Ok(previous)
        }
    }

    /// Parse the parameter list inside `(...)` after the function name.
    ///
    /// The grammar allows either `(void)` (no parameters) or a comma-
    /// separated list of `int` parameters (`int x, int y, ...`).  Chapter 9
    /// supports up to 6 register-passed args (the rest go on the stack).
    fn parse_param_list(&mut self) -> Result<Vec<VarDecl>> {
        if self.match_exact(&TokenKind::Void) {
            // `(void)` means no parameters; empty list.
            return Ok(Vec::new());
        }
        let mut params = Vec::new();
        loop {
            let ty = self.parse_type_specifier()?;
            let name = self.expect_identifier("parameter name")?;
            params.push(VarDecl {
                name,
                ty,
                init: None,
                storage: StorageClass::Auto,
            });
            if !self.match_exact(&TokenKind::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_block_item(&mut self) -> Result<BlockItem> {
        // Chapter 9 + 10: a block-level declaration can be
        //   [static|extern] int NAME ...
        //   int [static|extern] NAME ...   (type-before-storage-class)
        // Chapter 11 widens the type to `int` or `long` (any order).
        if self.peek().kind == TokenKind::Int
            || self.peek().kind == TokenKind::Long
            || self.peek().kind == TokenKind::Static
            || self.peek().kind == TokenKind::Extern
        {
            let (ty, storage) = self.parse_specifiers_interleaved()?;
            if let TokenKind::Identifier(_) = self.peek().kind {
                let name = self.expect_identifier("function or variable name")?;
                if self.check(&TokenKind::OpenParen) {
                    self.current += 1; // consume `(`
                    let params = self.parse_param_list()?;
                    self.expect_exact(&TokenKind::CloseParen, "')' after parameter list")?;
                    if self.match_exact(&TokenKind::Semicolon) {
                        return Ok(BlockItem::FunctionDecl(GlobalDecl {
                            name,
                            ret_ty: ty,
                            params,
                            storage,
                        }));
                    }
                    // Nested function definitions are illegal in C.
                    bail!("parse error: nested function definitions are not allowed");
                }
                let init = if self.match_exact(&TokenKind::Equal) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect_exact(&TokenKind::Semicolon, "';'")?;
                return Ok(BlockItem::Declaration(VarDecl {
                    name,
                    ty,
                    init,
                    storage,
                }));
            }
            bail!(
                "parse error: expected identifier after type specifier, found {:?}",
                self.peek().kind
            );
        }
        Ok(BlockItem::Statement(self.parse_statement()?))
    }

    fn parse_statement(&mut self) -> Result<Statement> {
        if let Some(label) = self.match_label_prefix() {
            // Labels are statement prefixes in C, not expressions.  We parse
            // the following statement recursively so labels can attach to
            // returns, null statements, `if`s, gotos, or another label.  Because
            // declarations are block items rather than statements in C17, a
            // label directly before `int x;` naturally fails in the recursive
            // call, which matches the chapter's invalid_parse suite.
            let statement = self.parse_statement()?;
            Ok(Statement::Labeled {
                label,
                statement: Box::new(statement),
            })
        } else if self.match_exact(&TokenKind::Return) {
            let expr = self.parse_expr()?;
            self.expect_exact(&TokenKind::Semicolon, "';'")?;
            Ok(Statement::Return(expr))
        } else if self.match_exact(&TokenKind::OpenBrace) {
            let mut items = Vec::new();
            while !self.check(&TokenKind::CloseBrace) {
                if self.check(&TokenKind::Eof) {
                    bail!("parse error: expected '}}' to close compound statement");
                }
                items.push(self.parse_block_item()?);
            }
            self.expect_exact(&TokenKind::CloseBrace, "'}'")?;
            Ok(Statement::Block(items))
        } else if self.match_exact(&TokenKind::If) {
            self.expect_exact(&TokenKind::OpenParen, "'(' after if")?;
            let condition = self.parse_expr()?;
            self.expect_exact(&TokenKind::CloseParen, "')' after if condition")?;
            let then_branch = Box::new(self.parse_statement()?);
            let else_branch = if self.match_exact(&TokenKind::Else) {
                Some(Box::new(self.parse_statement()?))
            } else {
                None
            };
            Ok(Statement::If {
                condition,
                then_branch,
                else_branch,
            })
        } else if self.match_exact(&TokenKind::Goto) {
            let label = self.expect_identifier("label after goto")?;
            self.expect_exact(&TokenKind::Semicolon, "';'")?;
            Ok(Statement::Goto(label))
        } else if self.match_exact(&TokenKind::While) {
            self.expect_exact(&TokenKind::OpenParen, "'(' after while")?;
            let condition = self.parse_expr()?;
            self.expect_exact(&TokenKind::CloseParen, "')' after while condition")?;
            let body = Box::new(self.parse_statement()?);
            let label = String::new();
            Ok(Statement::While {
                condition,
                body,
                label,
            })
        } else if self.match_exact(&TokenKind::Do) {
            let body = Box::new(self.parse_statement()?);
            self.expect_exact(&TokenKind::While, "'while' after do body")?;
            self.expect_exact(&TokenKind::OpenParen, "'(' after while")?;
            let condition = self.parse_expr()?;
            self.expect_exact(&TokenKind::CloseParen, "')' after do-while condition")?;
            self.expect_exact(&TokenKind::Semicolon, "';' after do-while")?;
            let label = String::new();
            Ok(Statement::DoWhile {
                body,
                condition,
                label,
            })
        } else if self.match_exact(&TokenKind::For) {
            self.parse_for_statement()
        } else if self.match_exact(&TokenKind::Break) {
            let target = self.match_break_continue_target();
            self.expect_exact(&TokenKind::Semicolon, "';' after break")?;
            Ok(Statement::Break(target))
        } else if self.match_exact(&TokenKind::Continue) {
            let target = self.match_break_continue_target();
            self.expect_exact(&TokenKind::Semicolon, "';' after continue")?;
            Ok(Statement::Continue(target))
        } else if self.match_exact(&TokenKind::Switch) {
            self.expect_exact(&TokenKind::OpenParen, "'(' after switch")?;
            let expr = self.parse_expr()?;
            self.expect_exact(&TokenKind::CloseParen, "')' after switch expression")?;
            let body = Box::new(self.parse_statement()?);
            let label = String::new();
            Ok(Statement::Switch {
                expr,
                body,
                label,
            })
        } else if self.match_exact(&TokenKind::Case) {
            let value = self.parse_expr()?;
            self.expect_exact(&TokenKind::Colon, "':' after case value")?;
            let statement = Box::new(self.parse_statement()?);
            Ok(Statement::Case { value, statement })
        } else if self.match_exact(&TokenKind::Default) {
            self.expect_exact(&TokenKind::Colon, "':' after default")?;
            let statement = Box::new(self.parse_statement()?);
            Ok(Statement::Default { statement })
        } else if self.match_exact(&TokenKind::Semicolon) {
            Ok(Statement::Expr(None))
        } else {
            let expr = self.parse_expr()?;
            self.expect_exact(&TokenKind::Semicolon, "';'")?;
            Ok(Statement::Expr(Some(expr)))
        }
    }

    fn parse_for_statement(&mut self) -> Result<Statement> {
        self.expect_exact(&TokenKind::OpenParen, "'(' after for")?;
        let init = if self.match_exact(&TokenKind::Semicolon) {
            None
        } else if self.peek().kind == TokenKind::Int || self.peek().kind == TokenKind::Long {
            let ty = self.parse_type_specifier()?;
            let name = self.expect_identifier("for-loop variable name")?;
            let init = if self.match_exact(&TokenKind::Equal) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect_exact(&TokenKind::Semicolon, "';' after for declaration")?;
            Some(ForInit::Declaration(VarDecl {
                name,
                ty,
                init,
                storage: StorageClass::Auto,
            }))
        } else {
            let expr = self.parse_expr()?;
            self.expect_exact(&TokenKind::Semicolon, "';' after for init")?;
            Some(ForInit::Expr(expr))
        };

        let condition = if self.match_exact(&TokenKind::Semicolon) {
            None
        } else {
            let condition = self.parse_expr()?;
            self.expect_exact(&TokenKind::Semicolon, "';' after for condition")?;
            Some(condition)
        };

        let post = if self.check(&TokenKind::CloseParen) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect_exact(&TokenKind::CloseParen, "')' after for header")?;
        let body = Box::new(self.parse_statement()?);
        let label = String::new();
        Ok(Statement::For {
            init,
            condition,
            post,
            body,
            label,
        })
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr> {
        let left = self.parse_conditional_expr()?;
        if let Some(op) = self.match_assignment_op() {
            let value = self.parse_assignment()?;
            Ok(Expr::Assign {
                op,
                target: Box::new(left),
                value: Box::new(value),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_conditional_expr(&mut self) -> Result<Expr> {
        let condition = self.parse_binary_expr(Precedence::Lowest)?;
        if self.match_exact(&TokenKind::Question) {
            // The middle operand is a full expression in C, so assignment is
            // legal here (`flag ? a = 1 : ...`).  The right operand is another
            // conditional expression, making `?:` right-associative while still
            // allowing the outer assignment parser to reject unparenthesized
            // assignment on the far right when it targets the whole ternary.
            let then_expr = self.parse_expr()?;
            self.expect_exact(&TokenKind::Colon, "':' in conditional expression")?;
            let else_expr = self.parse_conditional_expr()?;
            Ok(Expr::Conditional {
                condition: Box::new(condition),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            })
        } else {
            Ok(condition)
        }
    }

    fn parse_binary_expr(&mut self, min_precedence: Precedence) -> Result<Expr> {
        let mut left = self.parse_unary_expr()?;
        while let Some((op, op_prec)) = self.peek_binary_op() {
            if op_prec < min_precedence {
                break;
            }
            self.current += 1;
            let next_min = op_prec.next_higher().unwrap_or(Precedence::Highest);
            let right = self.parse_binary_expr(next_min)?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<Expr> {
        match &self.peek().kind {
            TokenKind::Constant(value) => {
                let value = i64::from(*value);
                self.current += 1;
                Ok(Expr::Constant(value))
            }
            TokenKind::LongConstant(value) => {
                let value = *value;
                self.current += 1;
                Ok(Expr::LongConstant(value))
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.current += 1;
                // Chapter 9: identifier followed by `(` is a function call.
                // The grammar permits `f()` for zero-arg functions or
                // `f(a, b, c)` for multi-arg; the result type for chapter 9
                // is always `int` because we do not yet implement return
                // types other than int.  Function name is `name`; the
                // argument list is parsed as a comma-separated sequence of
                // full expressions.
                if self.check(&TokenKind::OpenParen) {
                    self.current += 1;
                    let args = self.parse_arg_list()?;
                    self.expect_exact(&TokenKind::CloseParen, "')' after arguments")?;
                    Ok(Expr::Call {
                        name,
                        args,
                    })
                } else {
                    let mut expr = Expr::Var(name);
                    loop {
                        if self.match_exact(&TokenKind::PlusPlus) {
                            expr = Expr::PostInc(Box::new(expr));
                        } else if self.match_exact(&TokenKind::MinusMinus) {
                            expr = Expr::PostDec(Box::new(expr));
                        } else {
                            break;
                        }
                    }
                    Ok(expr)
                }
            }
            TokenKind::Minus => {
                self.current += 1;
                Ok(Expr::Unary {
                    op: UnaryOp::Negate,
                    expr: Box::new(self.parse_unary_expr()?),
                })
            }
            TokenKind::Tilde => {
                self.current += 1;
                Ok(Expr::Unary {
                    op: UnaryOp::Complement,
                    expr: Box::new(self.parse_unary_expr()?),
                })
            }
            // Chapter-4 logical not (`!e`).  Distinct from `~e` (bitwise
            // complement, handled by the `Tilde` arm above): `!0 == 1` while
            // `~0 == -1`.  The parser folds both into the same `Expr::Unary`
            // shape and lets the lowerer dispatch on `UnaryOp`.
            TokenKind::Bang => {
                self.current += 1;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(self.parse_unary_expr()?),
                })
            }
            TokenKind::PlusPlus => {
                self.current += 1;
                Ok(Expr::PreInc(Box::new(self.parse_unary_expr()?)))
            }
            TokenKind::MinusMinus => {
                self.current += 1;
                Ok(Expr::PreDec(Box::new(self.parse_unary_expr()?)))
            }
            TokenKind::OpenParen => {
                self.current += 1;
                // Chapter 11: `(T) expr` cast.  If the token after
                // `(` is a type specifier, parse it as a cast; the
                // closing `)` and the casted expression follow.
                if self.peek().kind == TokenKind::Int || self.peek().kind == TokenKind::Long {
                    let target_type = self.parse_type_specifier()?;
                    self.expect_exact(&TokenKind::CloseParen, "')' after cast type")?;
                    let inner = self.parse_unary_expr()?;
                    let mut expr = Expr::Cast {
                        target_type,
                        expr: Box::new(inner),
                    };
                    loop {
                        if self.match_exact(&TokenKind::PlusPlus) {
                            expr = Expr::PostInc(Box::new(expr));
                        } else if self.match_exact(&TokenKind::MinusMinus) {
                            expr = Expr::PostDec(Box::new(expr));
                        } else {
                            break;
                        }
                    }
                    return Ok(expr);
                }
                let inner = self.parse_expr()?;
                self.expect_exact(&TokenKind::CloseParen, "')'")?;
                let mut expr = Expr::Paren(Box::new(inner));
                loop {
                    if self.match_exact(&TokenKind::PlusPlus) {
                        expr = Expr::PostInc(Box::new(expr));
                    } else if self.match_exact(&TokenKind::MinusMinus) {
                        expr = Expr::PostDec(Box::new(expr));
                    } else {
                        break;
                    }
                }
                Ok(expr)
            }
            _ => bail!(
                "parse error: expected expression, found {:?} ({:?})",
                self.peek().kind,
                self.peek().lexeme
            ),
        }
    }

    /// Parse the argument list inside a function-call's `(...)`.  Returns
    /// an empty vector for `f()`, otherwise a comma-separated list of
    /// full expressions.
    fn parse_arg_list(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        if self.check(&TokenKind::CloseParen) {
            return Ok(args);
        }
        loop {
            args.push(self.parse_assignment()?);
            if !self.match_exact(&TokenKind::Comma) {
                break;
            }
        }
        Ok(args)
    }

    fn match_assignment_op(&mut self) -> Option<AssignOp> {
        let op = match self.peek().kind {
            TokenKind::Equal => AssignOp::Assign,
            TokenKind::PlusEqual => AssignOp::Add,
            TokenKind::MinusEqual => AssignOp::Subtract,
            TokenKind::StarEqual => AssignOp::Multiply,
            TokenKind::SlashEqual => AssignOp::Divide,
            TokenKind::PercentEqual => AssignOp::Remainder,
            TokenKind::ShiftLeftEqual => AssignOp::ShiftLeft,
            TokenKind::ShiftRightEqual => AssignOp::ShiftRight,
            TokenKind::AmpersandEqual => AssignOp::BitwiseAnd,
            TokenKind::CaretEqual => AssignOp::BitwiseXor,
            TokenKind::PipeEqual => AssignOp::BitwiseOr,
            _ => return None,
        };
        self.current += 1;
        Some(op)
    }

    fn match_label_prefix(&mut self) -> Option<String> {
        match (
            &self.peek().kind,
            self.tokens.get(self.current + 1).map(|t| &t.kind),
        ) {
            (TokenKind::Identifier(label), Some(TokenKind::Colon)) => {
                let label = label.clone();
                self.current += 2;
                Some(label)
            }
            _ => None,
        }
    }

    /// Optional identifier after `break` / `continue`.  Empty string
    /// means a bare `break;` / `continue;`; a non-empty value means
    /// the chapter-8 extra `break <id>;` / `continue <id>;`.  Either
    /// form is later rewritten by the `label_loops` pass.
    fn match_break_continue_target(&mut self) -> String {
        if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.current += 1;
            name
        } else {
            String::new()
        }
    }

    fn peek_binary_op(&self) -> Option<(BinaryOp, Precedence)> {
        let kind = &self.peek().kind;
        let precedence = precedence_of(kind)?;
        let op = match kind {
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Subtract,
            TokenKind::Star => BinaryOp::Multiply,
            TokenKind::Slash => BinaryOp::Divide,
            TokenKind::Percent => BinaryOp::Remainder,
            TokenKind::ShiftLeft => BinaryOp::ShiftLeft,
            TokenKind::ShiftRight => BinaryOp::ShiftRight,
            TokenKind::Ampersand => BinaryOp::BitwiseAnd,
            TokenKind::Caret => BinaryOp::BitwiseXor,
            TokenKind::Pipe => BinaryOp::BitwiseOr,
            // Chapter 4 — equality, relational, logical operators.
            TokenKind::EqualEqual => BinaryOp::Equal,
            TokenKind::NotEqual => BinaryOp::NotEqual,
            TokenKind::Less => BinaryOp::LessThan,
            TokenKind::LessEqual => BinaryOp::LessOrEqual,
            TokenKind::Greater => BinaryOp::GreaterThan,
            TokenKind::GreaterEqual => BinaryOp::GreaterOrEqual,
            TokenKind::LogicalAnd => BinaryOp::LogicalAnd,
            TokenKind::LogicalOr => BinaryOp::LogicalOr,
            // `precedence_of` returned `Some` so the token must be one of
            // the variants above; any other variant here is a bug in the
            // precedence table.
            _ => unreachable!("precedence_of returned Some for an unhandled TokenKind"),
        };
        Some((op, precedence))
    }

    fn expect_exact(&mut self, expected: &TokenKind, label: &str) -> Result<()> {
        if self.check(expected) {
            self.current += 1;
            Ok(())
        } else {
            bail!(
                "parse error: expected {label}, found {:?} ({:?})",
                self.peek().kind,
                self.peek().lexeme
            )
        }
    }

    fn expect_identifier(&mut self, label: &str) -> Result<String> {
        match &self.peek().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.current += 1;
                Ok(name)
            }
            _ => bail!(
                "parse error: expected {label}, found {:?} ({:?})",
                self.peek().kind,
                self.peek().lexeme
            ),
        }
    }

    fn match_exact(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.current += 1;
            true
        } else {
            false
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
}