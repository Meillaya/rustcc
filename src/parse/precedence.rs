//! Expression-precedence helpers.
//!
//! The parser uses precedence climbing (Listing 3-9 of the book): each
//! binary operator has a precedence level, and `parse_binary_expr`
//! walks operators whose precedence is at least the floor it was
//! called with.  The variants are ordered from lowest to highest so
//! the derived `Ord` matches the comparison the algorithm needs:
//! `MulDiv > AddSub > BitShift > Relational > Equality > BitAnd >
//! BitXor > BitOr > LogicalAnd > LogicalOr > Lowest`.

use crate::lex::TokenKind;

/// Precedence levels for the binary operators covered through chapter 4.
///
/// Levels are ordered low-to-high.  `Lowest` is a sentinel below every
/// real operator; the parser starts `parse_binary_expr(Lowest)` so the
/// first real operator is always accepted.  `Highest` is the
/// complementary sentinel above every real operator used as the
/// recursive floor after consuming a top-level `MulDiv` operator; no
/// real operator matches it so the recursion terminates.
///
/// C precedence (high to low):
/// ```text
///   * / %       MulDiv
///   + -         AddSub
///   << >>       BitShift
///   < > <= >=   Relational   (chapter 4)
///   == !=       Equality      (chapter 4)
///   &           BitAnd
///   ^           BitXor
///   |           BitOr
///   &&          LogicalAnd    (chapter 4)
///   ||          LogicalOr     (chapter 4)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest,
    LogicalOr,
    LogicalAnd,
    BitOr,
    BitXor,
    BitAnd,
    Equality,
    Relational,
    BitShift,
    AddSub,
    MulDiv,
    Highest,
}

impl Precedence {
    /// The next-higher precedence level used as the recursive floor
    /// after consuming an operator.
    ///
    /// Precedence climbing requires the right-hand recursive call to
    /// accept only operators with strictly higher precedence than the
    /// one just consumed (this enforces left-associativity).  At the
    /// top of the table (`MulDiv`) the next step is `Highest`, which
    /// no real operator matches — so the recursive call rejects every
    /// follow-up operator and the parse terminates.
    pub fn next_higher(self) -> Option<Self> {
        use Precedence::*;
        match self {
            Highest => None,
            MulDiv => Some(Highest),
            AddSub => Some(MulDiv),
            BitShift => Some(AddSub),
            Relational => Some(BitShift),
            Equality => Some(Relational),
            BitAnd => Some(Equality),
            BitXor => Some(BitAnd),
            BitOr => Some(BitXor),
            LogicalAnd => Some(BitOr),
            LogicalOr => Some(LogicalAnd),
            Lowest => Some(LogicalOr),
        }
    }
}

/// Map a `TokenKind` to its operator precedence when it represents a
/// binary operator, or `None` for everything else (punctuation, names,
/// EOF, …).
///
/// The full set covers chapter 3 (`+ - * / %`) and the chapter 3
/// bitwise extras (`& | ^ << >>`).  Relational (`< <= > >=`),
/// equality (`== !=`), and logical (`&& ||`) operators keep their
/// precedence slot even though the chapter 3 grammar rejects them at
/// parse time; their slots remain so chapter 4 only needs to flip a
/// bit in `peek_binary_op` to enable them.
pub fn precedence_of(kind: &TokenKind) -> Option<Precedence> {
    let prec = match kind {
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::MulDiv,
        TokenKind::Plus | TokenKind::Minus => Precedence::AddSub,
        TokenKind::ShiftLeft | TokenKind::ShiftRight => Precedence::BitShift,
        TokenKind::Less | TokenKind::LessEqual | TokenKind::Greater | TokenKind::GreaterEqual => {
            Precedence::Relational
        }
        TokenKind::EqualEqual | TokenKind::NotEqual => Precedence::Equality,
        TokenKind::Ampersand => Precedence::BitAnd,
        TokenKind::Caret => Precedence::BitXor,
        TokenKind::Pipe => Precedence::BitOr,
        TokenKind::LogicalAnd => Precedence::LogicalAnd,
        TokenKind::LogicalOr => Precedence::LogicalOr,
        _ => return None,
    };
    Some(prec)
}