use crate::codegen::assembly::{Instr, Operand, Reg};

pub(crate) fn use_reserved_address_scratch(instructions: &[Instr]) -> Vec<Instr> {
    let mut rewritten = Vec::with_capacity(instructions.len());
    let mut index = 0;
    while index < instructions.len() {
        if let Some((load_pointer, memory_access)) =
            rewrite_address_scratch_pair(instructions, index)
        {
            rewritten.push(load_pointer);
            rewritten.push(memory_access);
            index += 2;
        } else {
            rewritten.push(instructions[index].clone());
            index += 1;
        }
    }
    rewritten
}

fn rewrite_address_scratch_pair(instructions: &[Instr], index: usize) -> Option<(Instr, Instr)> {
    let Instr::Movq {
        src,
        dst: Operand::Reg(Reg::R9),
    } = instructions.get(index)?
    else {
        return None;
    };
    let memory_access = rewrite_r9_zero_memory_to_r11(instructions.get(index + 1)?)?;
    Some((
        Instr::Movq {
            src: src.clone(),
            dst: Operand::Reg(Reg::R11),
        },
        memory_access,
    ))
}

fn rewrite_r9_zero_memory_to_r11(instr: &Instr) -> Option<Instr> {
    let memory = Operand::Memory(Reg::R9, 0);
    let reserved = Operand::Memory(Reg::R11, 0);
    match instr {
        Instr::Mov { src, dst } if src == &memory => Some(Instr::Mov {
            src: reserved,
            dst: dst.clone(),
        }),
        Instr::Mov { src, dst } if dst == &memory => Some(Instr::Mov {
            src: src.clone(),
            dst: reserved,
        }),
        Instr::Movq { src, dst } if src == &memory => Some(Instr::Movq {
            src: reserved,
            dst: dst.clone(),
        }),
        Instr::Movq { src, dst } if dst == &memory => Some(Instr::Movq {
            src: src.clone(),
            dst: reserved,
        }),
        Instr::MovByte { src, dst } if src == &memory => Some(Instr::MovByte {
            src: reserved,
            dst: dst.clone(),
        }),
        Instr::MovByte { src, dst } if dst == &memory => Some(Instr::MovByte {
            src: src.clone(),
            dst: reserved,
        }),
        Instr::Movsd { src, dst } if src == &memory => Some(Instr::Movsd {
            src: reserved,
            dst: dst.clone(),
        }),
        Instr::Movsd { src, dst } if dst == &memory => Some(Instr::Movsd {
            src: src.clone(),
            dst: reserved,
        }),
        _ => None,
    }
}
