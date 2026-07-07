//! Operators used by expression AST nodes.
//!
//! Operators are small `Copy` enums because they are closed sets of symbolic
//! choices and have no owned data. This keeps parser/evaluator code simple and
//! avoids unnecessary cloning of expression trees.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssignOp {
    Assign,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    ShiftLeft,
    ShiftRight,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnaryOp {
    /// `-<expr>` — additive inverse.
    Negate,
    /// `~<expr>` — bitwise NOT.
    Complement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    ShiftLeft,
    ShiftRight,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    LogicalAnd,
    LogicalOr,
}

impl BinaryOp {
    pub(crate) fn precedence(self) -> u8 {
        match self {
            Self::Multiply | Self::Divide | Self::Remainder => 60,
            Self::Add | Self::Subtract => 50,
            Self::ShiftLeft | Self::ShiftRight => 40,
            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual => 35,
            Self::Equal | Self::NotEqual => 34,
            Self::BitwiseAnd => 30,
            Self::BitwiseXor => 25,
            Self::BitwiseOr => 20,
            Self::LogicalAnd => 15,
            Self::LogicalOr => 10,
        }
    }

    pub(crate) fn eval_values(self, left: i32, right: i32) -> i32 {
        match self {
            Self::Add => left.wrapping_add(right),
            Self::Subtract => left.wrapping_sub(right),
            Self::Multiply => left.wrapping_mul(right),
            Self::Divide => left / right,
            Self::Remainder => left % right,
            Self::ShiftLeft => left.wrapping_shl((right as u32) & 31),
            Self::ShiftRight => left >> ((right as u32) & 31),
            Self::Less => i32::from(left < right),
            Self::LessEqual => i32::from(left <= right),
            Self::Greater => i32::from(left > right),
            Self::GreaterEqual => i32::from(left >= right),
            Self::Equal => i32::from(left == right),
            Self::NotEqual => i32::from(left != right),
            Self::BitwiseAnd => left & right,
            Self::BitwiseXor => left ^ right,
            Self::BitwiseOr => left | right,
            Self::LogicalAnd | Self::LogicalOr => unreachable!("short-circuit separately"),
        }
    }
}
