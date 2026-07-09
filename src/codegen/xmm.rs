use crate::codegen::assembly::BinaryOpInstr;

pub(crate) const fn is_xmm_binary(op: BinaryOpInstr) -> bool {
    matches!(
        op,
        BinaryOpInstr::AddDouble
            | BinaryOpInstr::SubDouble
            | BinaryOpInstr::MultDouble
            | BinaryOpInstr::SseDivDouble
            | BinaryOpInstr::XorDouble
    )
}
