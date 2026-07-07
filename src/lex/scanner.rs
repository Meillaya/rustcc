//! Character-by-character scanner for the book's C subset.
//!
//! Mirrors `nqcc2/lib/lex.ml` (1-211).  The reference implementation iterates
//! over an OCaml `Stream.t` of characters with regex helpers from the `re`
//! package.  This Rust port keeps the same overall algorithm -- skip
//! whitespace, try each token-recogniser, emit the longest match -- but
//! replaces the regex dispatch with explicit, hand-written state machines
//! that advance the `Chars<'_>` cursor one character at a time.  Per the task
//! brief, no regex engine is used.
//!
//! ## Token model
//!
//! The scanner classifies each lexeme into the book-faithful `Keyword` /
//! `Punct` enums defined in `token.rs`, then lowers the result to the flat
//! `TokenKind` variants consumed by the recursive-descent parser.  That keeps
//! the long-term token vocabulary declarative while preserving the parser's
//! discriminant-based matching.
//!
//! ## Errors
//!
//! Lex failures bail with `"lex error: ..."` so the driver contract is
//! unchanged: invalid lex tests return a clean non-zero compiler exit instead
//! of panicking.

use anyhow::{Result, bail};

use crate::lex::keyword::keyword_from_str;
use crate::lex::token::{Punct, Token, TokenKind};

/// Cursor over the source text.  `Peekable` lets the dispatch in `lex` look
/// one character ahead without committing, while `CharIndices` preserves the
/// byte offset for diagnostics when the parser layer adds span tracking.
type Chars<'a> = std::iter::Peekable<std::str::CharIndices<'a>>;

/// Public entry point mirroring the task spec.  `lex` is kept as an alias so
/// `compiler.rs` (which calls `crate::lex::{lex, pretty_tokens}`) continues to
/// compile unchanged.
pub fn scan(source: &str) -> Result<Vec<Token>> {
    lex(source)
}

/// Lex the entire source into a token stream terminated by an `Eof` sentinel.
///
/// The top-level dispatch mirrors `nqcc2/lib/lex.ml`: skip whitespace, then
/// for each non-whitespace character decide which recogniser owns it.  Unlike
/// the OCaml version (which collects all regex matches and picks the longest)
/// this port uses a direct char-to-recogniser switch -- Rust's deterministic
/// match is equivalent to "longest unambiguous match" for the book's token
/// grammar because each leading character identifies a single token shape.
pub fn lex(source: &str) -> Result<Vec<Token>> {
    let mut chars = source.char_indices().peekable();
    let mut tokens = Vec::new();

    while let Some((_, ch)) = chars.peek().copied() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        match ch {
            '/' => lex_slash(&mut chars, &mut tokens)?,
            '.' => lex_dot_or_leading_float(&mut chars, &mut tokens)?,
            '\'' => lex_char_literal(&mut chars, &mut tokens)?,
            '"' => lex_string_literal(&mut chars, &mut tokens)?,
            c if is_ident_start(c) => lex_identifier(&mut chars, &mut tokens),
            c if c.is_ascii_digit() => lex_number(&mut chars, &mut tokens)?,
            c => lex_punct(&mut chars, &mut tokens, c)?,
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        lexeme: String::new(),
    });
    Ok(tokens)
}

// --- Identifier & keyword recognition -------------------------------------

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_cont(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Consume `[A-Za-z_][A-Za-z0-9_]*` and lower to either a `Keyword` token
/// or an `Identifier` token.  Mirrors the `convert_identifier` branch of
/// `nqcc2/lib/lex.ml`.
fn lex_identifier(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) {
    let mut lexeme = String::new();
    while let Some((_, c)) = chars.peek().copied() {
        if is_ident_cont(c) {
            lexeme.push(c);
            chars.next();
        } else {
            break;
        }
    }
    let kind = match keyword_from_str(&lexeme) {
        Some(kw) => TokenKind::from_keyword(kw),
        None => TokenKind::Identifier(lexeme.clone()),
    };
    tokens.push(Token { kind, lexeme });
}

// --- Number recognition ----------------------------------------------------
//
// Three numeric shapes are recognised:
//   - Integer constants: decimal / octal / hex, with optional `L`, `l`,
//     `U`, `u`, `LU`, `lU`, `Lu`, `lu`, `UL`, `uL`, `Ul`, `ul` suffix forms.
//   - Floating-point constants: decimal with `.`, optional `e/E` exponent
//     and optional sign.
// The numeric value is folded into the existing `Constant(i32)` slot so the
// chapter-1 parser keeps compiling; chapter 11+ will widen the integer types
// and chapter 13+ will add a real `DoubleConstant` payload.

fn lex_number(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) -> Result<()> {
    let mut lexeme = String::new();

    // Detect the `0x` / `0X` hex prefix before consuming the leading zero so
    // we can route the rest of the digits through `is_ascii_hexdigit`.
    let hex_prefix = matches!(chars.peek().copied(), Some((_, '0')))
        && matches!(
            chars.clone().nth(1).map(|(_, c)| c),
            Some('x') | Some('X')
        );

    if hex_prefix {
        // Consume '0' and 'x'.
        lexeme.push(chars.next().expect("peeked '0'").1);
        lexeme.push(chars.next().expect("peeked 'x'").1);
        let hex_start = lexeme.len();
        while let Some((_, c)) = chars.peek().copied() {
            if c.is_ascii_hexdigit() {
                lexeme.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if lexeme.len() == hex_start {
            bail!("lex error: empty hex integer constant");
        }
        let suffix = collect_int_suffix(chars, &mut lexeme);
        let digits = &lexeme[hex_start..lexeme.len() - suffix.len()];
        let value = u128::from_str_radix(digits, 16)
            .map_err(|err| anyhow::anyhow!("lex error: invalid hex {lexeme}: {err}"))?;
        if matches!(chars.peek().copied(), Some((_, c)) if is_ident_cont(c)) {
            bail!("lex error: invalid identifier starts with digits: {lexeme}");
        }
        tokens.push(Token {
            kind: TokenKind::Constant(value as i32),
            lexeme,
        });
        return Ok(());
    }

    // Plain decimal (or octal-looking) digits.
    while let Some((_, c)) = chars.peek().copied() {
        if c.is_ascii_digit() {
            lexeme.push(c);
            chars.next();
        } else {
            break;
        }
    }

    // Floating-point: a `.` followed by a digit attaches to the number; a
    // `.` followed by a non-digit (or EOF) is a struct-member `Dot` and the
    // integer path resumes.  This matches the `Dot` / float disambiguation
    // the OCaml reference expresses as `[^\d]` after the literal `.`.
    let mut is_float = false;
    if let Some((_, '.')) = chars.peek().copied() {
        let mut probe = chars.clone();
        probe.next();
        if matches!(probe.peek().copied(), Some((_, c)) if c.is_ascii_digit()) {
            is_float = true;
            lexeme.push(chars.next().expect("peeked '.'").1);
            while let Some((_, c)) = chars.peek().copied() {
                if c.is_ascii_digit() {
                    lexeme.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
        }
    }

    if is_float {
        // Optional exponent.
        if matches!(chars.peek().copied(), Some((_, 'e' | 'E'))) {
            lexeme.push(chars.next().expect("peeked exponent").1);
            if let Some((_, sign @ ('+' | '-'))) = chars.peek().copied() {
                lexeme.push(sign);
                chars.next();
            }
            let exp_start = lexeme.len();
            while let Some((_, c)) = chars.peek().copied() {
                if c.is_ascii_digit() {
                    lexeme.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if lexeme.len() == exp_start {
                bail!("lex error: malformed floating constant {lexeme}");
            }
        }
        // Refuse alphabetic / underscore / trailing-dot suffixes; the book
        // does not introduce `f` / `F` / `l` / `L` float suffixes until
        // chapter 13, so anything that looks like one is a lexical error.
        if matches!(
            chars.peek().copied(),
            Some((_, c)) if c.is_ascii_alphabetic() || c == '_' || c == '.'
        ) {
            bail!("lex error: invalid floating constant suffix in {lexeme}");
        }
        tokens.push(Token {
            kind: TokenKind::Constant(0),
            lexeme,
        });
        return Ok(());
    }

    // Integer suffix.  We accept every shape the OCaml reference lists:
    // L / l / U / u / LU / lU / Lu / lu / UL / uL / Ul / ul.  Anything else
    // (e.g. `123abc`) is a lexical error so we don't silently bleed into
    // the next identifier.
    let suffix = collect_int_suffix(chars, &mut lexeme);
    if !is_valid_int_suffix(&suffix) {
        bail!("lex error: invalid integer suffix in {lexeme}");
    }
    if matches!(chars.peek().copied(), Some((_, c)) if is_ident_cont(c)) {
        bail!("lex error: invalid identifier starts with digits: {lexeme}");
    }

    let digits = &lexeme[..lexeme.len() - suffix.len()];
    let value = digits
        .parse::<u128>()
        .map_err(|err| anyhow::anyhow!("lex error: invalid integer {lexeme}: {err}"))?;
    tokens.push(Token {
        kind: TokenKind::Constant(value as i32),
        lexeme,
    });
    Ok(())
}

/// Consume an optional `L` / `l` / `U` / `u` run and append it to `lexeme`,
/// returning the suffix so the caller can validate the shape.
fn collect_int_suffix(chars: &mut Chars<'_>, lexeme: &mut String) -> String {
    let suffix_start = lexeme.len();
    while let Some((_, c)) = chars.peek().copied() {
        if matches!(c, 'l' | 'L' | 'u' | 'U') {
            lexeme.push(c);
            chars.next();
        } else {
            break;
        }
    }
    lexeme[suffix_start..].to_string()
}

fn is_valid_int_suffix(suffix: &str) -> bool {
    matches!(
        suffix,
        "" | "l"
            | "L"
            | "u"
            | "U"
            | "lu"
            | "lU"
            | "Lu"
            | "LU"
            | "ul"
            | "uL"
            | "Ul"
            | "UL"
    )
}

// --- Character & string literals ------------------------------------------

fn lex_char_literal(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) -> Result<()> {
    let mut lexeme = String::new();
    let (_, opening) = chars.next().expect("caller peeked character literal");
    lexeme.push(opening);

    let value = match chars.next() {
        Some((_, '\\')) => {
            lexeme.push('\\');
            let Some((_, escaped)) = chars.next() else {
                bail!("lex error: unterminated character constant");
            };
            lexeme.push(escaped);
            decode_c_escape(escaped)? as i32
        }
        Some((_, '\n')) => bail!("lex error: newline in character constant"),
        Some((_, '\'')) => bail!("lex error: empty character constant"),
        Some((_, ch)) => {
            lexeme.push(ch);
            ch as i32
        }
        None => bail!("lex error: unterminated character constant"),
    };

    match chars.next() {
        Some((_, '\'')) => lexeme.push('\''),
        Some((_, '\n')) => bail!("lex error: newline in character constant"),
        Some((_, ch)) => bail!("lex error: malformed character constant before {ch:?}"),
        None => bail!("lex error: unterminated character constant"),
    }

    tokens.push(Token {
        kind: TokenKind::CharLiteral(value),
        lexeme,
    });
    Ok(())
}

fn lex_string_literal(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) -> Result<()> {
    let mut lexeme = String::new();
    let mut decoded = String::new();
    let (_, opening) = chars.next().expect("caller peeked string literal");
    lexeme.push(opening);

    loop {
        let Some((_, ch)) = chars.next() else {
            bail!("lex error: unterminated string literal");
        };
        match ch {
            '"' => {
                lexeme.push('"');
                break;
            }
            '\n' => bail!("lex error: newline in string literal"),
            '\\' => {
                lexeme.push('\\');
                let Some((_, escaped)) = chars.next() else {
                    bail!("lex error: unterminated string literal");
                };
                lexeme.push(escaped);
                decoded.push(decode_c_escape(escaped)?);
            }
            _ => {
                lexeme.push(ch);
                decoded.push(ch);
            }
        }
    }

    tokens.push(Token {
        kind: TokenKind::StringLiteral(decoded),
        lexeme,
    });
    Ok(())
}

fn decode_c_escape(escaped: char) -> Result<char> {
    match escaped {
        '\'' => Ok('\''),
        '"' => Ok('"'),
        '?' => Ok('?'),
        '\\' => Ok('\\'),
        'a' => Ok('\x07'),
        'b' => Ok('\x08'),
        'f' => Ok('\x0c'),
        'n' => Ok('\n'),
        'r' => Ok('\r'),
        't' => Ok('\t'),
        'v' => Ok('\x0b'),
        _ => bail!("lex error: unsupported escape sequence \\{escaped}"),
    }
}

// --- Punctuation & operators ----------------------------------------------

/// Emit a standalone `.` or start lexing a leading-dot floating constant
/// (e.g. `.5`).  When the dot is followed by a digit we continue reading the
/// fractional part; otherwise we emit `Punct::Dot` directly.
fn lex_dot_or_leading_float(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) -> Result<()> {
    let mut lexeme = String::from(".");
    chars.next();
    let mut saw_digit = false;
    while let Some((_, c)) = chars.peek().copied() {
        if c.is_ascii_digit() {
            saw_digit = true;
            lexeme.push(c);
            chars.next();
        } else {
            break;
        }
    }
    if !saw_digit {
        tokens.push(Token {
            kind: TokenKind::from_punct(Punct::Dot),
            lexeme,
        });
        return Ok(());
    }
    if matches!(chars.peek().copied(), Some((_, 'e' | 'E'))) {
        let (_, c) = chars.next().expect("peeked exponent");
        lexeme.push(c);
        if let Some((_, sign @ ('+' | '-'))) = chars.peek().copied() {
            lexeme.push(sign);
            chars.next();
        }
        let exp_start = lexeme.len();
        while let Some((_, c)) = chars.peek().copied() {
            if c.is_ascii_digit() {
                lexeme.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if lexeme.len() == exp_start {
            bail!("lex error: malformed floating constant {lexeme}");
        }
    }
    if matches!(
        chars.peek().copied(),
        Some((_, c)) if c.is_ascii_alphabetic() || c == '_' || c == '.'
    ) {
        bail!("lex error: invalid floating constant suffix in {lexeme}");
    }
    tokens.push(Token {
        kind: TokenKind::Constant(0),
        lexeme,
    });
    Ok(())
}

/// Dispatch every punctuation / operator character to its single- or
/// multi-character form.  Mirrors the literal-token table in
/// `nqcc2/lib/lex.ml` (lines 116-150) but without the regex compile step.
fn lex_punct(chars: &mut Chars<'_>, tokens: &mut Vec<Token>, ch: char) -> Result<()> {
    chars.next();
    let (lexeme, punct) = match ch {
        '(' => ("(", Punct::OpenParen),
        ')' => (")", Punct::CloseParen),
        '{' => ("{", Punct::OpenBrace),
        '}' => ("}", Punct::CloseBrace),
        '[' => ("[", Punct::OpenBracket),
        ']' => ("]", Punct::CloseBracket),
        ',' => (",", Punct::Comma),
        ';' => (";", Punct::Semicolon),
        '~' => ("~", Punct::Tilde),
        '?' => ("?", Punct::Question),
        ':' => (":", Punct::Colon),
        '+' => match chars.peek().copied() {
            Some((_, '+')) => {
                chars.next();
                ("++", Punct::PlusPlus)
            }
            Some((_, '=')) => {
                chars.next();
                ("+=", Punct::PlusEq)
            }
            _ => ("+", Punct::Plus),
        },
        '-' => match chars.peek().copied() {
            Some((_, '-')) => {
                chars.next();
                ("--", Punct::MinusMinus)
            }
            Some((_, '=')) => {
                chars.next();
                ("-=", Punct::MinusEq)
            }
            Some((_, '>')) => {
                chars.next();
                ("->", Punct::Arrow)
            }
            _ => ("-", Punct::Minus),
        },
        '*' => match chars.peek().copied() {
            Some((_, '=')) => {
                chars.next();
                ("*=", Punct::StarEq)
            }
            _ => ("*", Punct::Star),
        },
        '/' => unreachable!("'/' is handled by lex_slash before punct dispatch"),
        '%' => match chars.peek().copied() {
            Some((_, '=')) => {
                chars.next();
                ("%=", Punct::PercentEq)
            }
            _ => ("%", Punct::Percent),
        },
        '&' => match chars.peek().copied() {
            Some((_, '&')) => {
                chars.next();
                ("&&", Punct::AmpAmp)
            }
            Some((_, '=')) => {
                chars.next();
                ("&=", Punct::AmpEq)
            }
            _ => ("&", Punct::Amp),
        },
        '|' => match chars.peek().copied() {
            Some((_, '|')) => {
                chars.next();
                ("||", Punct::PipePipe)
            }
            Some((_, '=')) => {
                chars.next();
                ("|=", Punct::PipeEq)
            }
            _ => ("|", Punct::Pipe),
        },
        '^' => match chars.peek().copied() {
            Some((_, '=')) => {
                chars.next();
                ("^=", Punct::CaretEq)
            }
            _ => ("^", Punct::Caret),
        },
        '=' => match chars.peek().copied() {
            Some((_, '=')) => {
                chars.next();
                ("==", Punct::EqEq)
            }
            _ => ("=", Punct::Eq),
        },
        '!' => match chars.peek().copied() {
            Some((_, '=')) => {
                chars.next();
                ("!=", Punct::NotEq)
            }
            _ => ("!", Punct::Bang),
        },
        '<' => match chars.peek().copied() {
            Some((_, '<')) => {
                chars.next();
                match chars.peek().copied() {
                    Some((_, '=')) => {
                        chars.next();
                        ("<<=", Punct::LtLtEq)
                    }
                    _ => ("<<", Punct::LtLt),
                }
            }
            Some((_, '=')) => {
                chars.next();
                ("<=", Punct::LtEq)
            }
            _ => ("<", Punct::Lt),
        },
        '>' => match chars.peek().copied() {
            Some((_, '>')) => {
                chars.next();
                match chars.peek().copied() {
                    Some((_, '=')) => {
                        chars.next();
                        (">>=", Punct::GtGtEq)
                    }
                    _ => (">>", Punct::GtGt),
                }
            }
            Some((_, '=')) => {
                chars.next();
                (">=", Punct::GtEq)
            }
            _ => (">", Punct::Gt),
        },
        _ => bail!("lex error: invalid punctuation character {ch:?}"),
    };
    tokens.push(Token {
        kind: TokenKind::from_punct(punct),
        lexeme: lexeme.into(),
    });
    Ok(())
}

// --- Comments --------------------------------------------------------------

/// `/` is the only character whose meaning depends on what follows: comment
/// opener (`//` / `/*`), compound assignment (`/=`), or plain division.
fn lex_slash(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) -> Result<()> {
    chars.next();
    match chars.peek().copied() {
        Some((_, '/')) => {
            chars.next();
            for (_, c) in chars.by_ref() {
                if c == '\n' {
                    break;
                }
            }
            Ok(())
        }
        Some((_, '*')) => {
            chars.next();
            let mut previous = '\0';
            let mut closed = false;
            for (_, c) in chars.by_ref() {
                if previous == '*' && c == '/' {
                    closed = true;
                    break;
                }
                previous = c;
            }
            if !closed {
                bail!("lex error: unterminated block comment");
            }
            Ok(())
        }
        Some((_, '=')) => {
            chars.next();
            tokens.push(Token {
                kind: TokenKind::from_punct(Punct::SlashEq),
                lexeme: "/=".into(),
            });
            Ok(())
        }
        _ => {
            tokens.push(Token {
                kind: TokenKind::from_punct(Punct::Slash),
                lexeme: "/".into(),
            });
            Ok(())
        }
    }
}

// --- Pretty-printing -------------------------------------------------------

/// Render a token stream for the `--stage lex` driver output.  Each line is
/// `<TokenKind-label> <lexeme>` so callers can sanity-check what the scanner
/// produced without needing to instantiate the AST.
pub fn pretty_tokens(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| {
            let label = token.kind.label();
            if token.lexeme.is_empty() {
                label.to_string()
            } else {
                format!("{label} {}", token.lexeme)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}