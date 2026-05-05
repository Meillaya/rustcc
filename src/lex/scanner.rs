//! Character-by-character scanner for the book's C subset.
//!
//! The scanner owns the mutable character cursor because lexical analysis is a
//! single pass over source text.  Helpers take `&mut Chars` and `&mut Vec<Token>`
//! so ownership of the token stream stays with `lex`, while borrowing keeps each
//! helper small and allocation-free except for token lexemes.  Errors use
//! `anyhow::Result`/`bail!` to preserve the existing driver contract: invalid
//! lex tests return a clean non-zero compiler exit instead of panicking.

use anyhow::{Result, bail};

use crate::lex::keyword::keyword_kind;
use crate::lex::token::{Token, TokenKind};

type Chars<'a> = std::iter::Peekable<std::str::CharIndices<'a>>;

pub(crate) fn lex(source: &str) -> Result<Vec<Token>> {
    let mut chars = source.char_indices().peekable();
    let mut tokens = Vec::new();
    while let Some((_, ch)) = chars.peek().copied() {
        match ch {
            c if c.is_whitespace() => {
                chars.next();
            }
            '/' => lex_slash(&mut chars, &mut tokens)?,
            'a'..='z' | 'A'..='Z' | '_' => lex_identifier_or_keyword(&mut chars, &mut tokens),
            '0'..='9' => lex_integer(&mut chars, &mut tokens)?,
            '.' => lex_dot_or_leading_float(&mut chars, &mut tokens)?,
            '\'' => lex_char_literal(&mut chars, &mut tokens)?,
            '"' => lex_string_literal(&mut chars, &mut tokens)?,
            '(' => push_single(&mut tokens, &mut chars, TokenKind::OpenParen),
            ')' => push_single(&mut tokens, &mut chars, TokenKind::CloseParen),
            '{' => push_single(&mut tokens, &mut chars, TokenKind::OpenBrace),
            '}' => push_single(&mut tokens, &mut chars, TokenKind::CloseBrace),
            '[' => push_single(&mut tokens, &mut chars, TokenKind::OpenBracket),
            ']' => push_single(&mut tokens, &mut chars, TokenKind::CloseBracket),
            '?' => push_single(&mut tokens, &mut chars, TokenKind::Question),
            ':' => push_single(&mut tokens, &mut chars, TokenKind::Colon),
            ',' => push_single(&mut tokens, &mut chars, TokenKind::Comma),
            ';' => push_single(&mut tokens, &mut chars, TokenKind::Semicolon),
            '~' => push_single(&mut tokens, &mut chars, TokenKind::Tilde),
            '+' => lex_plus(&mut chars, &mut tokens),
            '-' => lex_minus(&mut chars, &mut tokens),
            '*' => lex_optional_equals(
                &mut chars,
                &mut tokens,
                TokenKind::StarEqual,
                TokenKind::Star,
            ),
            '%' => lex_optional_equals(
                &mut chars,
                &mut tokens,
                TokenKind::PercentEqual,
                TokenKind::Percent,
            ),
            '&' => lex_ampersand(&mut chars, &mut tokens),
            '|' => lex_pipe(&mut chars, &mut tokens),
            '^' => lex_optional_equals(
                &mut chars,
                &mut tokens,
                TokenKind::CaretEqual,
                TokenKind::Caret,
            ),
            '!' => lex_optional_equals(
                &mut chars,
                &mut tokens,
                TokenKind::NotEqual,
                TokenKind::Bang,
            ),
            '=' => lex_optional_equals(
                &mut chars,
                &mut tokens,
                TokenKind::EqualEqual,
                TokenKind::Equal,
            ),
            '<' => lex_angle(&mut chars, &mut tokens, '<')?,
            '>' => lex_angle(&mut chars, &mut tokens, '>')?,
            _ => bail!("lex error: invalid token {ch:?}"),
        }
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        lexeme: String::new(),
    });
    Ok(tokens)
}

/// Lex a C character constant.  Chapter 16 treats character constants as
/// integer values, so the token stores the decoded byte value.  The parser does
/// not yet consume this token natively for the advanced chapters; keeping it as
/// a first-class token still gives the lex stage an honest contract and lets the
/// system-frontend bridge distinguish lexical failures from later grammar or
/// semantic failures.
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

/// Lex a string literal while validating the escape sequences accepted by the
/// book tests.  The decoded bytes are retained in the token for readable debug
/// output, but ownership stays local to the token (`String`) because string
/// literals may outlive this scanning function and should not borrow from the
/// temporary character iterator.
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
            kind: TokenKind::Dot,
            lexeme,
        });
        return Ok(());
    }
    if matches!(chars.peek().copied(), Some((_, 'e' | 'E'))) {
        let (_, c) = chars.next().expect("peeked leading-dot exponent");
        lexeme.push(c);
        if let Some((_, sign @ ('+' | '-'))) = chars.peek().copied() {
            lexeme.push(sign);
            chars.next();
        }
        let mut saw_exponent_digit = false;
        while let Some((_, c)) = chars.peek().copied() {
            if c.is_ascii_digit() {
                saw_exponent_digit = true;
                lexeme.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if !saw_exponent_digit {
            bail!("lex error: malformed floating constant {lexeme}");
        }
    }
    if matches!(chars.peek().copied(), Some((_, c)) if c.is_ascii_alphabetic() || c == '_' || c == '.')
    {
        bail!("lex error: invalid floating constant suffix in {lexeme}");
    }
    tokens.push(Token {
        kind: TokenKind::Constant(0),
        lexeme,
    });
    Ok(())
}

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
        }
        Some((_, '=')) => {
            chars.next();
            tokens.push(Token {
                kind: TokenKind::SlashEqual,
                lexeme: "/=".into(),
            });
        }
        _ => tokens.push(Token {
            kind: TokenKind::Slash,
            lexeme: "/".into(),
        }),
    }
    Ok(())
}

fn lex_identifier_or_keyword(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) {
    let mut lexeme = String::new();
    while let Some((_, c)) = chars.peek().copied() {
        if c.is_ascii_alphanumeric() || c == '_' {
            lexeme.push(c);
            chars.next();
        } else {
            break;
        }
    }
    let kind = keyword_kind(&lexeme).unwrap_or_else(|| TokenKind::Identifier(lexeme.clone()));
    tokens.push(Token { kind, lexeme });
}

fn lex_integer(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) -> Result<()> {
    let mut lexeme = String::new();
    while let Some((_, c)) = chars.peek().copied() {
        if c.is_ascii_digit() {
            lexeme.push(c);
            chars.next();
        } else {
            break;
        }
    }
    let mut is_float = false;
    if matches!(chars.peek().copied(), Some((_, '.'))) {
        is_float = true;
        lexeme.push('.');
        chars.next();
        while let Some((_, c)) = chars.peek().copied() {
            if c.is_ascii_digit() {
                lexeme.push(c);
                chars.next();
            } else {
                break;
            }
        }
    }
    if matches!(chars.peek().copied(), Some((_, 'e' | 'E'))) {
        is_float = true;
        let (_, c) = chars.next().expect("peeked exponent");
        lexeme.push(c);
        if let Some((_, sign @ ('+' | '-'))) = chars.peek().copied() {
            lexeme.push(sign);
            chars.next();
        }
        let mut saw_exponent_digit = false;
        while let Some((_, c)) = chars.peek().copied() {
            if c.is_ascii_digit() {
                saw_exponent_digit = true;
                lexeme.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if !saw_exponent_digit {
            bail!("lex error: malformed floating constant {lexeme}");
        }
    }
    if is_float {
        if matches!(chars.peek().copied(), Some((_, c)) if c.is_ascii_alphabetic() || c == '_' || c == '.')
        {
            bail!("lex error: invalid floating constant suffix in {lexeme}");
        }
        tokens.push(Token {
            kind: TokenKind::Constant(0),
            lexeme,
        });
        return Ok(());
    }
    let mut suffix = String::new();
    while let Some((_, c)) = chars.peek().copied() {
        if matches!(c, 'l' | 'L' | 'u' | 'U') {
            suffix.push(c.to_ascii_lowercase());
            lexeme.push(c);
            chars.next();
        } else {
            break;
        }
    }
    let long_suffix_count = suffix.chars().filter(|c| *c == 'l').count();
    let unsigned_suffix_count = suffix.chars().filter(|c| *c == 'u').count();
    if long_suffix_count > 1
        || unsigned_suffix_count > 1
        || (suffix.len() == 2 && suffix != "ul" && suffix != "lu")
        || suffix.len() > 2
    {
        bail!("lex error: unsupported integer suffix in {lexeme}");
    }
    if matches!(chars.peek().copied(), Some((_, c)) if c.is_ascii_alphabetic() || c == '_') {
        bail!("lex error: invalid identifier starts with digits: {lexeme}");
    }
    let digits = lexeme.trim_end_matches(['l', 'L', 'u', 'U']);
    let value = digits
        .parse::<u128>()
        .map_err(|err| anyhow::anyhow!("lex error: invalid integer {lexeme}: {err}"))?;
    tokens.push(Token {
        kind: TokenKind::Constant(value as i32),
        lexeme,
    });
    Ok(())
}

fn push_single(tokens: &mut Vec<Token>, chars: &mut Chars<'_>, kind: TokenKind) {
    let (_, ch) = chars.next().expect("caller peeked before push_single");
    tokens.push(Token {
        kind,
        lexeme: ch.to_string(),
    });
}

fn lex_plus(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) {
    chars.next();
    let (kind, lexeme) = match chars.peek().copied() {
        Some((_, '+')) => {
            chars.next();
            (TokenKind::PlusPlus, "++")
        }
        Some((_, '=')) => {
            chars.next();
            (TokenKind::PlusEqual, "+=")
        }
        _ => (TokenKind::Plus, "+"),
    };
    tokens.push(Token {
        kind,
        lexeme: lexeme.into(),
    });
}

fn lex_minus(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) {
    chars.next();
    let (kind, lexeme) = match chars.peek().copied() {
        Some((_, '-')) => {
            chars.next();
            (TokenKind::MinusMinus, "--")
        }
        Some((_, '=')) => {
            chars.next();
            (TokenKind::MinusEqual, "-=")
        }
        Some((_, '>')) => {
            chars.next();
            (TokenKind::Arrow, "->")
        }
        _ => (TokenKind::Minus, "-"),
    };
    tokens.push(Token {
        kind,
        lexeme: lexeme.into(),
    });
}

fn lex_optional_equals(
    chars: &mut Chars<'_>,
    tokens: &mut Vec<Token>,
    equals_kind: TokenKind,
    bare_kind: TokenKind,
) {
    let (_, ch) = chars
        .next()
        .expect("caller peeked before lex_optional_equals");
    if matches!(chars.peek().copied(), Some((_, '='))) {
        chars.next();
        tokens.push(Token {
            kind: equals_kind,
            lexeme: format!("{ch}="),
        });
    } else {
        tokens.push(Token {
            kind: bare_kind,
            lexeme: ch.to_string(),
        });
    }
}

fn lex_ampersand(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) {
    chars.next();
    let (kind, lexeme) = match chars.peek().copied() {
        Some((_, '&')) => {
            chars.next();
            (TokenKind::LogicalAnd, "&&")
        }
        Some((_, '=')) => {
            chars.next();
            (TokenKind::AmpersandEqual, "&=")
        }
        _ => (TokenKind::Ampersand, "&"),
    };
    tokens.push(Token {
        kind,
        lexeme: lexeme.into(),
    });
}

fn lex_pipe(chars: &mut Chars<'_>, tokens: &mut Vec<Token>) {
    chars.next();
    let (kind, lexeme) = match chars.peek().copied() {
        Some((_, '|')) => {
            chars.next();
            (TokenKind::LogicalOr, "||")
        }
        Some((_, '=')) => {
            chars.next();
            (TokenKind::PipeEqual, "|=")
        }
        _ => (TokenKind::Pipe, "|"),
    };
    tokens.push(Token {
        kind,
        lexeme: lexeme.into(),
    });
}

fn lex_angle(chars: &mut Chars<'_>, tokens: &mut Vec<Token>, ch: char) -> Result<()> {
    chars.next();
    let (single, equal, shift, shift_equal) = if ch == '<' {
        (
            TokenKind::Less,
            TokenKind::LessEqual,
            TokenKind::ShiftLeft,
            TokenKind::ShiftLeftEqual,
        )
    } else {
        (
            TokenKind::Greater,
            TokenKind::GreaterEqual,
            TokenKind::ShiftRight,
            TokenKind::ShiftRightEqual,
        )
    };
    if matches!(chars.peek().copied(), Some((_, c)) if c == ch) {
        chars.next();
        if matches!(chars.peek().copied(), Some((_, '='))) {
            chars.next();
            tokens.push(Token {
                kind: shift_equal,
                lexeme: format!("{ch}{ch}="),
            });
        } else {
            tokens.push(Token {
                kind: shift,
                lexeme: format!("{ch}{ch}"),
            });
        }
    } else if matches!(chars.peek().copied(), Some((_, '='))) {
        chars.next();
        tokens.push(Token {
            kind: equal,
            lexeme: format!("{ch}="),
        });
    } else {
        tokens.push(Token {
            kind: single,
            lexeme: ch.to_string(),
        });
    }
    Ok(())
}

pub(crate) fn pretty_tokens(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| format!("{:?} {}", token.kind, token.lexeme))
        .collect::<Vec<_>>()
        .join("\n")
}
