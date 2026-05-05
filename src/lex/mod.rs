//! Lexical-analysis phase.
//!
//! This module exposes only the crate-internal lexer contract used by the
//! compiler facade: source text in, tokens plus readable `--stage lex` output
//! out.  Submodules separate token shapes, reserved words, and scanning helpers
//! so later chapters can extend one concern without reopening the facade.

pub(crate) mod cursor;
pub(crate) mod keyword;
pub(crate) mod scanner;
pub(crate) mod token;

pub(crate) use scanner::{lex, pretty_tokens};
pub(crate) use token::{Token, TokenKind};
