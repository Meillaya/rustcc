// Mirrors nqcc2/lib/backend/codegen.ml ABI section.

use crate::ast::{AggregateKind, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EightbyteClass {
    Integer,
    Sse,
    Memory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamSlot {
    pub param_index: usize,
    pub offset: i64,
    pub size: i64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ClassifiedParams {
    pub int_slots: Vec<ParamSlot>,
    pub sse_slots: Vec<ParamSlot>,
    pub stack_slots: Vec<ParamSlot>,
}

pub const INT_PARAM_REGS: [Reg; 6] = [Reg::DI, Reg::SI, Reg::DX, Reg::CX, Reg::R8, Reg::R9];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    DI,
    SI,
    DX,
    CX,
    R8,
    R9,
}

pub fn int_param_reg(idx: usize) -> Reg {
    INT_PARAM_REGS[idx]
}

pub fn eightbyte_size(total_size: i64, index: usize) -> i64 {
    let used = index as i64 * 8;
    let remaining = total_size - used;
    remaining.clamp(1, 8)
}

pub fn classify_aggregate(ty: &Type) -> Vec<EightbyteClass> {
    let size = ty.clone().size();
    if size <= 0 {
        return vec![EightbyteClass::Memory];
    }
    let count = ((size + 7) / 8) as usize;
    if size > 16 {
        return vec![EightbyteClass::Memory; count];
    }
    let mut classes = vec![EightbyteClass::Sse; count];
    classify_type_at(ty, 0, &mut classes);
    classes
}

fn merge(classes: &mut [EightbyteClass], offset: i64, class: EightbyteClass) {
    let idx = (offset / 8) as usize;
    if let Some(slot) = classes.get_mut(idx) {
        *slot = match (*slot, class) {
            (EightbyteClass::Memory, _) | (_, EightbyteClass::Memory) => EightbyteClass::Memory,
            (EightbyteClass::Integer, _) | (_, EightbyteClass::Integer) => EightbyteClass::Integer,
            (EightbyteClass::Sse, EightbyteClass::Sse) => EightbyteClass::Sse,
        };
    }
}

fn classify_type_at(ty: &Type, base: i64, classes: &mut [EightbyteClass]) {
    match ty {
        Type::Double => merge(classes, base, EightbyteClass::Sse),
        Type::Struct(tag) | Type::Union(tag) => {
            if let Some(entry) = crate::codegen::type_table::get(tag) {
                match entry.kind {
                    AggregateKind::Struct => {
                        for member in crate::codegen::type_table::members_in_order(tag) {
                            classify_type_at(&member.member_type, base + member.offset, classes);
                        }
                    }
                    AggregateKind::Union => {
                        for member in crate::codegen::type_table::members_in_order(tag) {
                            classify_type_at(&member.member_type, base, classes);
                        }
                    }
                }
            } else {
                for idx in 0..classes.len() {
                    merge(classes, (idx as i64) * 8, EightbyteClass::Memory);
                }
            }
        }
        Type::Array {
            element,
            size: Some(n),
        } => {
            let elem_size = element.clone().size();
            for idx in 0..*n {
                classify_type_at(element, base + elem_size * idx as i64, classes);
            }
        }
        Type::Array { size: None, .. } | Type::Void => {
            merge(classes, base, EightbyteClass::Memory);
        }
        Type::Int
        | Type::Long
        | Type::UnsignedInt
        | Type::UnsignedLong
        | Type::Char
        | Type::SignedChar
        | Type::UnsignedChar
        | Type::Pointer(_) => merge(classes, base, EightbyteClass::Integer),
    }
}

pub fn returns_on_stack(ret_type: &Type) -> bool {
    matches!(
        classify_aggregate(ret_type).first(),
        Some(EightbyteClass::Memory)
    )
}

pub fn classify_typed_parameters(types: &[Type], return_on_stack: bool) -> ClassifiedParams {
    let mut result = ClassifiedParams::default();
    let int_limit = if return_on_stack { 5 } else { 6 };
    let mut int_count = 0usize;
    let mut sse_count = 0usize;

    for (param_index, ty) in types.iter().enumerate() {
        match ty {
            Type::Double => {
                if sse_count < 8 {
                    result.sse_slots.push(ParamSlot {
                        param_index,
                        offset: 0,
                        size: 8,
                    });
                    sse_count += 1;
                } else {
                    result.stack_slots.push(ParamSlot {
                        param_index,
                        offset: 0,
                        size: 8,
                    });
                }
            }
            Type::Struct(_) | Type::Union(_) => {
                let classes = classify_aggregate(ty);
                let size = ty.clone().size();
                let needs_stack = matches!(classes.first(), Some(EightbyteClass::Memory));
                let needed_int = classes
                    .iter()
                    .filter(|c| matches!(c, EightbyteClass::Integer))
                    .count();
                let needed_sse = classes
                    .iter()
                    .filter(|c| matches!(c, EightbyteClass::Sse))
                    .count();
                if needs_stack || int_count + needed_int > int_limit || sse_count + needed_sse > 8 {
                    for idx in 0..classes.len() {
                        result.stack_slots.push(ParamSlot {
                            param_index,
                            offset: (idx as i64) * 8,
                            size: eightbyte_size(size, idx),
                        });
                    }
                } else {
                    for (idx, class) in classes.iter().enumerate() {
                        let slot = ParamSlot {
                            param_index,
                            offset: (idx as i64) * 8,
                            size: eightbyte_size(size, idx),
                        };
                        match class {
                            EightbyteClass::Integer => {
                                result.int_slots.push(slot);
                                int_count += 1;
                            }
                            EightbyteClass::Sse => {
                                result.sse_slots.push(slot);
                                sse_count += 1;
                            }
                            EightbyteClass::Memory => result.stack_slots.push(slot),
                        }
                    }
                }
            }
            _ => {
                if int_count < int_limit {
                    result.int_slots.push(ParamSlot {
                        param_index,
                        offset: 0,
                        size: ty.clone().size().clamp(1, 8),
                    });
                    int_count += 1;
                } else {
                    result.stack_slots.push(ParamSlot {
                        param_index,
                        offset: 0,
                        size: ty.clone().size().clamp(1, 8),
                    });
                }
            }
        }
    }
    result
}
