use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

mod codegen {
    #[path = "/home/mei/projects/rustcc/src/codegen/assembly.rs"]
    pub mod assembly;
}

mod ir {
    pub mod cfg {
        pub type BlockId = usize;
    }

    pub mod tacky {
        pub use super::super::OperandType;

        pub type TypeEnv = std::collections::HashMap<String, OperandType>;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperandType {
    Int,
    UInt,
    Byte,
    UByte,
    Long,
    ULong,
    Double,
    ByteArray { size: i64 },
}

mod regalloc {
    pub mod liveness {
        use std::collections::BTreeSet;

        use crate::codegen::assembly::{Instr, Operand};

        pub struct LiveBlock {
            pub instructions: Vec<(BTreeSet<Operand>, Instr)>,
        }

        pub struct LiveCfg {
            blocks: Vec<LiveBlock>,
        }

        impl LiveCfg {
            pub fn blocks(&self) -> &[LiveBlock] {
                &self.blocks
            }
        }
    }

    #[path = "/home/mei/projects/rustcc/src/codegen/regalloc/color.rs"]
    pub mod color;
    #[path = "/home/mei/projects/rustcc/src/codegen/regalloc/graph.rs"]
    pub mod graph;
    #[path = "/home/mei/projects/rustcc/src/codegen/regalloc/operands.rs"]
    pub mod operands;
    #[path = "/home/mei/projects/rustcc/src/codegen/regalloc/simplify.rs"]
    pub mod simplify;
    #[path = "/home/mei/projects/rustcc/src/codegen/regalloc/types.rs"]
    pub mod types;
}

use codegen::assembly::{Operand, Reg};
use regalloc::color::select;
use regalloc::graph::InterferenceGraph;
use regalloc::simplify::{Simplification, SimplifyChoice, SimplifyStep};
use regalloc::types::RegisterClass;

type ProbeResult<T> = Result<T, String>;

fn pseudo(name: &str) -> Operand {
    Operand::Pseudo(name.to_owned())
}

fn step(name: &str, choice: SimplifyChoice) -> SimplifyStep {
    SimplifyStep {
        node: pseudo(name),
        degree: 0,
        choice,
    }
}

fn assignment(
    assignments: &BTreeMap<Operand, Option<Reg>>,
    name: &str,
) -> ProbeResult<Option<Reg>> {
    assignments
        .get(&pseudo(name))
        .cloned()
        .ok_or_else(|| format!("missing assignment for {name}"))
}

fn assert_true(condition: bool, message: &str) -> ProbeResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn assert_eq_value<T: PartialEq + std::fmt::Debug>(
    left: T,
    right: T,
    message: &str,
) -> ProbeResult<()> {
    if left == right {
        Ok(())
    } else {
        Err(format!("{message}: left={left:?} right={right:?}"))
    }
}

fn ocaml_color_mapping() -> ProbeResult<BTreeMap<usize, Reg>> {
    #[rustfmt::skip]
    let expected = BTreeMap::from([(0, Reg::R9), (1, Reg::R8), (2, Reg::SI), (3, Reg::DI), (4, Reg::DX), (5, Reg::CX), (6, Reg::AX), (7, Reg::BX), (8, Reg::R12), (9, Reg::R13), (10, Reg::R14), (11, Reg::R15)]);
    for (color, reg) in &expected {
        let mut graph = InterferenceGraph::new(RegisterClass::Gp);
        let name = format!("color_{color}");
        let node = pseudo(&name);
        graph.add_node(node.clone(), 1.0);
        for hardreg in RegisterClass::Gp.all_hardregs() {
            if &hardreg != reg {
                graph.add_edge(&node, &Operand::Reg(hardreg));
            }
        }
        let result = select(
            &graph,
            &Simplification {
                stack: vec![step(&name, SimplifyChoice::LowDegree)],
            },
        );
        assert_eq_value(
            assignment(&result.assignments, &name)?,
            Some(reg.clone()),
            "bad color map",
        )?;
    }
    Ok(expected)
}

fn small_conflict() -> ProbeResult<BTreeMap<Operand, Option<Reg>>> {
    let mut graph = InterferenceGraph::new(RegisterClass::Gp);
    graph.add_node(pseudo("a"), 1.0);
    graph.add_node(pseudo("b"), 1.0);
    graph.add_edge(&pseudo("a"), &pseudo("b"));
    let result = select(
        &graph,
        &Simplification {
            stack: vec![
                step("a", SimplifyChoice::LowDegree),
                step("b", SimplifyChoice::LowDegree),
            ],
        },
    );
    assert_eq_value(
        assignment(&result.assignments, "b")?,
        Some(Reg::R9),
        "first color",
    )?;
    assert_eq_value(
        assignment(&result.assignments, "a")?,
        Some(Reg::R8),
        "second color",
    )?;
    Ok(result.assignments)
}

fn callee_saved_usage() -> ProbeResult<BTreeSet<Reg>> {
    let mut graph = InterferenceGraph::new(RegisterClass::Gp);
    let node = pseudo("callee");
    graph.add_node(node.clone(), 1.0);
    for reg in [
        Reg::R9,
        Reg::R8,
        Reg::SI,
        Reg::DI,
        Reg::DX,
        Reg::CX,
        Reg::AX,
    ] {
        graph.add_edge(&node, &Operand::Reg(reg));
    }
    let result = select(
        &graph,
        &Simplification {
            stack: vec![step("callee", SimplifyChoice::LowDegree)],
        },
    );
    assert_eq_value(
        assignment(&result.assignments, "callee")?,
        Some(Reg::BX),
        "callee color",
    )?;
    assert_true(
        result.used_callee_saved_regs.contains(&Reg::BX),
        "BX not reported",
    )?;
    Ok(result.used_callee_saved_regs)
}

fn spill_marker() -> ProbeResult<Option<Reg>> {
    let mut graph = InterferenceGraph::new(RegisterClass::Gp);
    let node = pseudo("pressure");
    graph.add_node(node.clone(), 1.0);
    for reg in RegisterClass::Gp.all_hardregs() {
        graph.add_edge(&node, &Operand::Reg(reg));
    }
    let result = select(
        &graph,
        &Simplification {
            stack: vec![step("pressure", SimplifyChoice::SpillCandidate)],
        },
    );
    let assignment = assignment(&result.assignments, "pressure")?;
    assert_true(assignment.is_none(), "spill marker was colored")?;
    Ok(assignment)
}

fn reserved_registers() -> ProbeResult<BTreeMap<&'static str, String>> {
    let gp = RegisterClass::Gp.all_hardregs();
    let xmm = RegisterClass::Xmm.all_hardregs();
    assert_true(!gp.contains(&Reg::R10), "R10 is allocatable")?;
    assert_true(!gp.contains(&Reg::R11), "R11 is allocatable")?;
    assert_true(!xmm.contains(&Reg::XMM(14)), "XMM14 is allocatable")?;
    assert_true(!xmm.contains(&Reg::XMM(15)), "XMM15 is allocatable")?;
    Ok(BTreeMap::from([
        ("gp", format!("{gp:?}")),
        ("xmm", format!("{xmm:?}")),
    ]))
}

fn hardreg_conflict() -> ProbeResult<Option<Reg>> {
    let mut graph = InterferenceGraph::new(RegisterClass::Gp);
    graph.add_node(pseudo("hard"), 1.0);
    graph.add_edge(&pseudo("hard"), &Operand::Reg(Reg::R9));
    let result = select(
        &graph,
        &Simplification {
            stack: vec![step("hard", SimplifyChoice::LowDegree)],
        },
    );
    let assignment = assignment(&result.assignments, "hard")?;
    assert_eq_value(assignment.clone(), Some(Reg::R8), "hardreg conflict")?;
    Ok(assignment)
}

fn main() -> ProbeResult<()> {
    let mut report = BTreeMap::new();
    report.insert(
        "ocaml_color_mapping",
        format!("{:?}", ocaml_color_mapping()?),
    );
    report.insert("small_conflict", format!("{:?}", small_conflict()?));
    report.insert("callee_saved", format!("{:?}", callee_saved_usage()?));
    report.insert("reserved", format!("{:?}", reserved_registers()?));
    report.insert("hardreg_conflict", format!("{:?}", hardreg_conflict()?));
    report.insert("spill_marker", format!("{:?}", spill_marker()?));
    io::stdout()
        .write_all(format!("{report:#?}\n").as_bytes())
        .map_err(|err| err.to_string())
}
