#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

mod codegen {
    pub mod assembly {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Reg { AX, CX, DX, DI, SI, R8, R9, R10, R11, SP, BP, BX, R12, R13, R14, R15, XMM(u8) }
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Operand { Imm(i64), Reg(Reg), Memory(Reg, i32), MemoryIndexed(Reg, Reg, i32), Pseudo(String), PseudoMem(String, i32), Stack(i32), Data(String), DataOffset(String, i32) }
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum BinaryOpInstr { Add }
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum UnaryOpInstr { Neg }
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum ConditionCode { E }
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub enum Instr {
            Mov { src: Operand, dst: Operand }, Movq { src: Operand, dst: Operand }, MovByte { src: Operand, dst: Operand }, Movabsq { src: i64, dst: Operand }, Movsx { src: Operand, dst: Operand }, MovZeroExtend { src: Operand, dst: Operand }, MovSignExtendByte { src: Operand, dst: Operand }, Movsd { src: Operand, dst: Operand }, MovsdLoad { src: String, dst: Operand }, Lea { src: Operand, dst: Operand }, Cmp { left: Operand, right: Operand }, Cmpq { left: Operand, right: Operand }, CmpDouble { left: Operand, right: Operand }, BinaryOp { op: BinaryOpInstr, src: Operand, dst: Operand }, Idiv(Operand), Div(Operand), Idivq(Operand), Divq(Operand), Cdq, Cqo, Cltq, Cvtsi2sd { src: Operand, dst: Operand }, Cvttsd2si { src: Operand, dst: Operand }, Unary { op: UnaryOpInstr, operand: Operand }, UnaryQ { op: UnaryOpInstr, operand: Operand }, Call(String), Ret, Push(Operand), Pop(Reg), Jmp(String), JmpCC { cc: ConditionCode, label: String }, SetCC { cc: ConditionCode, dst: Operand }, Label(String), AllocateStack(i32), DeallocateStack(i32), Comment(String),
        }
    }
}

mod ir {
    pub mod cfg {
        use crate::codegen::assembly::Instr;
        pub type BlockId = usize;
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum NodeId { Entry, Exit, Block(BlockId) }
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct BasicBlock<A, I> { pub id: BlockId, pub instructions: Vec<(A, I)>, pub value: A, pub preds: Vec<NodeId>, pub succs: Vec<NodeId> }
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Cfg<A, I> { blocks: Vec<BasicBlock<A, I>> }
        pub type AssemblyCfg = Cfg<(), Instr>;
        impl<A: Clone, I: Clone> Cfg<A, I> {
            pub fn new(blocks: Vec<BasicBlock<A, I>>) -> Self { Self { blocks } }
            pub fn initialize_annotation<B: Clone>(&self, value: B) -> Cfg<B, I> {
                Cfg { blocks: self.blocks.iter().map(|block| BasicBlock { id: block.id, instructions: block.instructions.iter().map(|(_, instr)| (value.clone(), instr.clone())).collect(), value: value.clone(), preds: block.preds.clone(), succs: block.succs.clone() }).collect() }
            }
            pub fn block(&self, id: BlockId) -> Option<&BasicBlock<A, I>> { self.blocks.iter().find(|block| block.id == id) }
            pub fn block_ids(&self) -> impl Iterator<Item = BlockId> + '_ { self.blocks.iter().map(|block| block.id) }
            pub fn blocks(&self) -> &[BasicBlock<A, I>] { &self.blocks }
            pub fn get_block_value(&self, id: BlockId) -> Option<&A> { self.block(id).map(|block| &block.value) }
            pub fn update_basic_block(&mut self, block: BasicBlock<A, I>) { if let Some(slot) = self.blocks.iter_mut().find(|item| item.id == block.id) { *slot = block; } }
        }
    }
    pub mod tacky {
        use std::collections::HashMap;
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum OperandType { Int, UInt, Byte, UByte, Long, ULong, Double, ByteArray { size: i64 } }
        pub type TypeEnv = HashMap<String, OperandType>;
    }
}

mod regalloc {
    #[path = "/home/mei/projects/rustcc/src/codegen/regalloc/types.rs"] pub mod types;
    #[path = "/home/mei/projects/rustcc/src/codegen/regalloc/operands.rs"] pub mod operands;
    #[path = "/home/mei/projects/rustcc/src/codegen/regalloc/liveness.rs"] pub mod liveness;
    #[path = "/home/mei/projects/rustcc/src/codegen/regalloc/graph.rs"] pub mod graph;
    #[path = "/home/mei/projects/rustcc/src/codegen/regalloc/simplify.rs"] pub mod simplify;
}

use codegen::assembly::{Instr, Operand, Reg};
use ir::cfg::{BasicBlock, Cfg, NodeId};
use ir::tacky::{OperandType, TypeEnv};
use regalloc::graph::{InterferenceBuild, InterferenceConfig, InterferenceGraph};
use regalloc::liveness::LiveCfg;
use regalloc::simplify::{simplify, SimplifyChoice};
use regalloc::types::{LiveSet, LivenessConfig, RegisterClass};

fn pseudo(name: &str) -> Operand { Operand::Pseudo(name.to_owned()) }
fn singleton(op: Operand) -> LiveSet { BTreeSet::from([op]) }
fn assert_true(condition: bool, message: &str) -> Result<(), String> { if condition { Ok(()) } else { Err(message.to_owned()) } }
fn annotated_block(instructions: Vec<(LiveSet, Instr)>) -> LiveCfg {
    Cfg::new(vec![BasicBlock { id: 0, instructions, value: LiveSet::new(), preds: vec![NodeId::Entry], succs: vec![NodeId::Exit] }])
}

fn probe_instructions() -> Vec<Instr> {
    vec![
        Instr::Mov { src: Operand::Imm(0), dst: pseudo("a") }, Instr::Mov { src: Operand::Imm(0), dst: pseudo("b") }, Instr::Mov { src: Operand::Imm(0), dst: pseudo("c") }, Instr::Mov { src: Operand::Imm(0), dst: pseudo("d") }, Instr::Mov { src: Operand::Imm(0), dst: pseudo("e") }, Instr::Mov { src: Operand::Imm(0), dst: pseudo("x") }, Instr::Mov { src: pseudo("x"), dst: pseudo("y") }, Instr::Mov { src: Operand::Imm(0), dst: pseudo("h") }, Instr::Mov { src: Operand::Imm(0), dst: pseudo("static_g") }, Instr::Mov { src: Operand::Imm(0), dst: pseudo("addr_taken") }, Instr::Movsd { src: Operand::Reg(Reg::XMM(0)), dst: pseudo("dbl") }, Instr::Cdq,
    ]
}

fn probe_cfg(instructions: &[Instr]) -> LiveCfg {
    annotated_block(vec![
        (BTreeSet::from([pseudo("b"), pseudo("c")]), instructions[0].clone()), (singleton(pseudo("c")), instructions[1].clone()), (LiveSet::new(), instructions[2].clone()), (singleton(pseudo("e")), instructions[3].clone()), (LiveSet::new(), instructions[4].clone()), (LiveSet::new(), instructions[5].clone()), (singleton(pseudo("x")), instructions[6].clone()), (LiveSet::new(), instructions[7].clone()), (LiveSet::new(), instructions[8].clone()), (LiveSet::new(), instructions[9].clone()), (LiveSet::new(), instructions[10].clone()), (singleton(pseudo("h")), instructions[11].clone()),
    ])
}

fn build_graph(class: RegisterClass) -> Result<InterferenceGraph, String> {
    let mut type_env = TypeEnv::new();
    type_env.insert("dbl".to_owned(), OperandType::Double);
    let config = InterferenceConfig { aliased_pseudos: BTreeSet::from(["addr_taken".to_owned()]), static_symbols: BTreeSet::from(["static_g".to_owned()]) };
    let liveness = LivenessConfig::default();
    let instructions = probe_instructions();
    let cfg = probe_cfg(&instructions);
    regalloc::graph::build_interference(InterferenceBuild { instructions: &instructions, liveness_cfg: &cfg, class, type_env: &type_env, interference: &config, liveness: &liveness }).map_err(|err| err.to_string())
}

fn pseudo_edges(graph: &InterferenceGraph) -> BTreeSet<String> {
    [("a", "b"), ("a", "c"), ("b", "c"), ("d", "e"), ("x", "y")]
        .into_iter()
        .filter(|(left, right)| graph.are_neighbors(&pseudo(left), &pseudo(right)))
        .map(|(left, right)| format!("{left}-{right}"))
        .collect()
}

fn pressure_summary() -> Result<String, String> {
    let mut graph = InterferenceGraph::new(RegisterClass::Gp);
    let pressure = pseudo("pressure");
    graph.add_node(pressure.clone(), 1.0);
    for reg in RegisterClass::Gp.all_hardregs() { graph.add_edge(&pressure, &Operand::Reg(reg)); }
    let simplification = simplify(&graph);
    let step = simplification.stack.iter().find(|item| item.node == pressure).ok_or_else(|| "missing pressure simplification step".to_owned())?;
    assert_true(step.choice == SimplifyChoice::SpillCandidate, "pressure was not spill fallback")?;
    Ok(format!("{:?}:{}", step.choice, step.degree))
}

fn main() -> Result<(), String> {
    let gp = build_graph(RegisterClass::Gp)?;
    let xmm = build_graph(RegisterClass::Xmm)?;
    let edges = pseudo_edges(&gp);
    let expected = BTreeSet::from(["a-b".to_owned(), "a-c".to_owned(), "b-c".to_owned(), "d-e".to_owned()]);
    assert_true(edges == expected, "manual pseudo edges did not match")?;
    assert_true(!gp.are_neighbors(&pseudo("x"), &pseudo("y")), "move x-y edge was not suppressed")?;
    assert_true(gp.are_neighbors(&pseudo("h"), &Operand::Reg(Reg::DX)), "h-DX hardreg edge is missing")?;
    assert_true(!gp.contains(&pseudo("dbl")), "GP graph included double pseudo")?;
    assert_true(xmm.contains(&pseudo("dbl")), "XMM graph excluded double pseudo")?;
    assert_true(!xmm.contains(&pseudo("a")), "XMM graph included GP pseudo")?;
    assert_true(!gp.contains(&pseudo("static_g")), "static pseudo was not excluded")?;
    assert_true(!gp.contains(&pseudo("addr_taken")), "aliased pseudo was not excluded")?;
    let low_simplify = simplify(&gp).stack.iter().filter(|step| matches!(step.node, Operand::Pseudo(_))).map(|step| format!("{:?}:{:?}:{}", step.node, step.choice, step.degree)).collect::<Vec<_>>();
    let mut report = BTreeMap::new();
    report.insert("gp_hardregs", RegisterClass::Gp.all_hardregs().len().to_string());
    report.insert("pseudo_edges", format!("{edges:?}"));
    report.insert("low_simplify", format!("{low_simplify:?}"));
    report.insert("pressure_simplify", pressure_summary()?);
    report.insert("xmm_hardregs", RegisterClass::Xmm.all_hardregs().len().to_string());
    let output = format!("{report:#?}\n");
    io::stdout()
        .write_all(output.as_bytes())
        .map_err(|err| err.to_string())
}
