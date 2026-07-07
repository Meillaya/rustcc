//! Recursive-descent parser for the supported native C subset.
//!
//! The parser consumes owned `Vec<Token>` values because tokens are inexpensive
//! records and ownership gives this phase freedom to advance by index without
//! lifetime plumbing.  Recursive AST forms use `Box` in the AST module, so parser
//! methods can build nested expressions and statements directly.  `Result` is
//! propagated with `?` so phase-specific parse failures retain the same driver
//! behavior as before extraction.

// Mirrors nqcc2/lib/parse.ml chapter 1 grammar (~lines 1-80). Recursive-descent, Result-returning.

use anyhow::{Result, bail};

use crate::ast::{AssignOp, BinaryOp, BlockItem, Expr, ForInit, Function, Program, Statement, UnaryOp};
use crate::lex::{Token, TokenKind};

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

    fn parse_program(&mut self) -> Result<Program> {
        self.expect_exact(&TokenKind::Int, "function return type 'int'")?;
        let function_name = self.expect_identifier("function identifier")?;
        self.expect_exact(&TokenKind::OpenParen, "'('")?;
        self.expect_exact(&TokenKind::Void, "parameter list 'void'")?;
        self.expect_exact(&TokenKind::CloseParen, "')'")?;
        self.expect_exact(&TokenKind::OpenBrace, "'{'")?;
        let mut body: Vec<Statement> = Vec::new();
        while !self.check(&TokenKind::CloseBrace) {
            if self.check(&TokenKind::Eof) {
                bail!("parse error: expected '}}'");
            }
            if let BlockItem::Statement(stmt) = self.parse_block_item()? {
                body.push(stmt);
            }
        }
        self.expect_exact(&TokenKind::CloseBrace, "'}'")?;
        self.expect_exact(&TokenKind::Eof, "end of file")?;
        Ok(Program {
            function: Function {
                name: function_name,
                body,
            },
        })
    }

    fn parse_block_item(&mut self) -> Result<BlockItem> {
        if self.check(&TokenKind::Int) {
            self.current += 1;
            let name = self.expect_identifier("variable name")?;
            let init = if self.match_exact(&TokenKind::Equal) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect_exact(&TokenKind::Semicolon, "';'")?;
            Ok(BlockItem::Declaration { name, init })
        } else {
            Ok(BlockItem::Statement(self.parse_statement()?))
        }
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
            Ok(Statement::While {
                condition,
                body: Box::new(self.parse_statement()?),
            })
        } else if self.match_exact(&TokenKind::Do) {
            let body = Box::new(self.parse_statement()?);
            self.expect_exact(&TokenKind::While, "'while' after do body")?;
            self.expect_exact(&TokenKind::OpenParen, "'(' after while")?;
            let condition = self.parse_expr()?;
            self.expect_exact(&TokenKind::CloseParen, "')' after do-while condition")?;
            self.expect_exact(&TokenKind::Semicolon, "';' after do-while")?;
            Ok(Statement::DoWhile { body, condition })
        } else if self.match_exact(&TokenKind::For) {
            self.parse_for_statement()
        } else if self.match_exact(&TokenKind::Break) {
            self.expect_exact(&TokenKind::Semicolon, "';' after break")?;
            Ok(Statement::Break)
        } else if self.match_exact(&TokenKind::Continue) {
            self.expect_exact(&TokenKind::Semicolon, "';' after continue")?;
            Ok(Statement::Continue)
        } else if self.match_exact(&TokenKind::Switch) {
            self.expect_exact(&TokenKind::OpenParen, "'(' after switch")?;
            let expr = self.parse_expr()?;
            self.expect_exact(&TokenKind::CloseParen, "')' after switch expression")?;
            Ok(Statement::Switch {
                expr,
                body: Box::new(self.parse_statement()?),
            })
        } else if self.match_exact(&TokenKind::Case) {
            let value = self.parse_expr()?;
            self.expect_exact(&TokenKind::Colon, "':' after case value")?;
            Ok(Statement::Case {
                value,
                statement: Box::new(self.parse_statement()?),
            })
        } else if self.match_exact(&TokenKind::Default) {
            self.expect_exact(&TokenKind::Colon, "':' after default")?;
            Ok(Statement::Default {
                statement: Box::new(self.parse_statement()?),
            })
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
        } else if self.match_exact(&TokenKind::Int) {
            let name = self.expect_identifier("for-loop variable name")?;
            let init = if self.match_exact(&TokenKind::Equal) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect_exact(&TokenKind::Semicolon, "';' after for declaration")?;
            Some(ForInit::Declaration { name, init })
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
        Ok(Statement::For {
            init,
            condition,
            post,
            body: Box::new(self.parse_statement()?),
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
        let condition = self.parse_binary_expr(0)?;
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

    fn parse_binary_expr(&mut self, min_precedence: u8) -> Result<Expr> {
        let mut left = self.parse_unary_expr()?;
        while let Some(op) = self.peek_binary_op() {
            let precedence = op.precedence();
            if precedence < min_precedence {
                break;
            }
            self.current += 1;
            let right = self.parse_binary_expr(precedence + 1)?;
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
                let value = *value;
                self.current += 1;
                Ok(Expr::Constant(value))
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.current += 1;
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
            TokenKind::Bang => {
                self.current += 1;
                Ok(Expr::LogicalNot(Box::new(self.parse_unary_expr()?)))
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

    fn peek_binary_op(&self) -> Option<BinaryOp> {
        match self.peek().kind {
            TokenKind::Plus => Some(BinaryOp::Add),
            TokenKind::Minus => Some(BinaryOp::Subtract),
            TokenKind::Star => Some(BinaryOp::Multiply),
            TokenKind::Slash => Some(BinaryOp::Divide),
            TokenKind::Percent => Some(BinaryOp::Remainder),
            TokenKind::ShiftLeft => Some(BinaryOp::ShiftLeft),
            TokenKind::ShiftRight => Some(BinaryOp::ShiftRight),
            TokenKind::Less => Some(BinaryOp::Less),
            TokenKind::LessEqual => Some(BinaryOp::LessEqual),
            TokenKind::Greater => Some(BinaryOp::Greater),
            TokenKind::GreaterEqual => Some(BinaryOp::GreaterEqual),
            TokenKind::EqualEqual => Some(BinaryOp::Equal),
            TokenKind::NotEqual => Some(BinaryOp::NotEqual),
            TokenKind::Ampersand => Some(BinaryOp::BitwiseAnd),
            TokenKind::Caret => Some(BinaryOp::BitwiseXor),
            TokenKind::Pipe => Some(BinaryOp::BitwiseOr),
            TokenKind::LogicalAnd => Some(BinaryOp::LogicalAnd),
            TokenKind::LogicalOr => Some(BinaryOp::LogicalOr),
            _ => None,
        }
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
