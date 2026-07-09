// Mirrors nqcc2/lib/backend/regalloc.ml:1-22 and :87-123.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::codegen::assembly::{Operand, Reg};

pub type LiveSet = BTreeSet<Operand>;
pub type LiveMap = BTreeMap<crate::ir::cfg::BlockId, BlockLiveness>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegisterClass {
    Gp,
    Xmm,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockLiveness {
    pub live_in: LiveSet,
    pub live_out: LiveSet,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LivenessConfig {
    pub return_regs: Vec<Reg>,
    pub call_param_regs: BTreeMap<String, Vec<Reg>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LivenessError {
    MissingCallMetadata { callee: String },
    PopInLiveness { reg: Reg },
}

impl fmt::Display for LivenessError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCallMetadata { callee } => {
                write!(out, "missing call metadata for callee '{callee}'")
            }
            Self::PopInLiveness { reg } => {
                write!(out, "pop reached liveness analysis for register {reg:?}")
            }
        }
    }
}

impl Error for LivenessError {}

impl RegisterClass {
    pub fn all_hardregs(self) -> Vec<Reg> {
        match self {
            Self::Gp => vec![
                Reg::AX,
                Reg::BX,
                Reg::CX,
                Reg::DX,
                Reg::DI,
                Reg::SI,
                Reg::R8,
                Reg::R9,
                Reg::R12,
                Reg::R13,
                Reg::R14,
                Reg::R15,
            ],
            Self::Xmm => (0..=13).map(Reg::XMM).collect(),
        }
    }

    pub fn caller_saved_regs(self) -> Vec<Reg> {
        match self {
            Self::Gp => vec![
                Reg::AX,
                Reg::CX,
                Reg::DX,
                Reg::DI,
                Reg::SI,
                Reg::R8,
                Reg::R9,
            ],
            Self::Xmm => (0..=13).map(Reg::XMM).collect(),
        }
    }

    pub fn contains(self, reg: &Reg) -> bool {
        match self {
            Self::Gp => matches!(
                reg,
                Reg::AX
                    | Reg::BX
                    | Reg::CX
                    | Reg::DX
                    | Reg::DI
                    | Reg::SI
                    | Reg::R8
                    | Reg::R9
                    | Reg::R12
                    | Reg::R13
                    | Reg::R14
                    | Reg::R15
            ),
            Self::Xmm => matches!(reg, Reg::XMM(0..=13)),
        }
    }
}

pub(crate) fn regs_to_operands(regs: &[Reg]) -> LiveSet {
    regs.iter().cloned().map(Operand::Reg).collect()
}
