//! Lexical-analysis phase.
//!
//! This module exposes only the crate-internal lexer contract used by the
//! compiler facade: source text in, tokens plus readable `--stage lex` output
//! out.  Submodules separate token shapes, reserved words, and scanning
//! helpers so later chapters can extend one concern without reopening the
//! facade.
//!
//! The token vocabulary mirrors `nqcc2/lib/tokens.ml` via the `Keyword` and
//! `Punct` enums (book-faithful shape); the parser-facing `TokenKind` is a
//! flat projection so the recursive-descent parser can pattern-match by
//! discriminant without paying for sub-payload inspection.
//!
//! `scan`, `Keyword`, and `Punct` are reserved for waves 2-20; the
//! `unused_imports` allow preserves the long-term module surface.

#![allow(unused_imports)]

pub mod keyword;
pub mod scanner;
pub mod token;

pub(crate) use scanner::{lex, pretty_tokens, scan};
pub(crate) use token::{Keyword, Punct, Token, TokenKind};