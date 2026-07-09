# Task 51 / W20-T1 CFG construction code review

VERDICT: PASS
codeQualityStatus: WATCH
recommendation: APPROVE
reportPath: `.omo/evidence/task-51-cfg-code-review.md`
blockers: []

## Review scope

- Repository: `/home/mei/projects/rustcc`
- HEAD inspected: `4c4d7b9 feat(compiler): chapter 18: aggregate ABI`
- Task plan inspected: `.omo/plans/c-compiler-rust.md`, Task 51 / W20-T1 requires a generic Chapter 19 CFG foundation with nodes/basic blocks, preds/succs, entry/exit, TACKY and assembly CFG flavors, and `cargo check --release` green so W20-T2..T5 can build atop it.
- Reference implementation inspected: `nqcc2/lib/cfg.ml:1-341`.
- Executor evidence inspected: `.omo/evidence/task-51-cfg-implementation.txt`.
- Source inspected:
  - `src/ir/cfg.rs`
  - `src/ir/cfg/build.rs`
  - `src/ir/cfg/instr.rs`
  - `src/ir/cfg/types.rs`
  - relevant seams in `src/ir/tacky.rs`, `src/codegen/assembly.rs`, `src/ir/opt.rs`, `src/codegen/regalloc/mod.rs`, `src/pipeline.rs`
- Current uncommitted state also includes `.omo/boulder.json` runtime-state edits and pre-existing untracked evidence files. Those are not part of the CFG source implementation, but are noted below under LOW findings.

## Skill-perspective check

- `omo:remove-ai-slops`: loaded from `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md` and applied as a read-only overfit/slop review pass.
  - Result: no deletion-only tests, tautological tests, implementation-mirroring constants, or requested-removal-only tests were added; no tests were changed at all.
  - Watch item: no direct CFG regression/unit tests exist yet, so edge correctness is currently manual-review plus upcoming adversarial-gate coverage.
- `omo:programming`: loaded from `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md`; Rust reference loaded from `references/rust/README.md`.
  - Result: no new dependencies, no `unsafe`, no `unwrap`/`expect`, no unsupported feature gates, no broad validation/parsing, no one-off abstraction that replaces an existing repo utility. CFG files are below the 250 pure-LOC ceiling.
  - `#![allow(dead_code)]` is present at `src/ir/cfg.rs:7`; assessment below.

## Exact commands inspected/run

```bash
pwd && git status --short
awk '/Task 51|W20-T1|Chapter 19 CFG/{flag=1} flag{print} /Task 52|W20-T2/{if(flag && NR>1){exit}}' .omo/plans/c-compiler-rust.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md
sed -n '1,317p' /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md
sed -n '1,380p' nqcc2/lib/cfg.ml
sed -n '1,260p' src/ir/cfg.rs
sed -n '1,260p' src/ir/cfg/build.rs
sed -n '1,200p' src/ir/cfg/instr.rs
sed -n '1,260p' src/ir/cfg/types.rs
cat .omo/evidence/task-51-cfg-implementation.txt
git diff -- src/ir/cfg.rs src/ir/cfg/build.rs src/ir/cfg/instr.rs src/ir/cfg/types.rs
git diff --no-index -- /dev/null src/ir/cfg/build.rs || true
git diff --no-index -- /dev/null src/ir/cfg/instr.rs || true
git diff --no-index -- /dev/null src/ir/cfg/types.rs || true
git diff -- .omo/boulder.json
rg -n "pub enum Instruction|pub enum Instr|JumpIfZero|JumpIfNotZero|JmpCC|Ret|Label\(" src/ir/tacky.rs src/codegen/assembly.rs src/ir/opt.rs src/codegen/regalloc/mod.rs src/compiler.rs src/pipeline.rs
rg -n "mod cfg|ir::cfg|cfg::|Cfg<|TackyCfg|AssemblyCfg|build_tacky_program|assembly_function_cfg|tacky_function_cfg|reachable_block_ids|initialize_annotation|strip_annotations" src Cargo.toml || true
cargo fmt --all -- --check
cargo check --release
git diff --check
rg -n "evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_" src tests Cargo.toml Cargo.lock .omo/evidence/task-51-cfg-implementation.txt
cargo test --release
for f in src/ir/cfg/build.rs src/ir/cfg/instr.rs src/ir/cfg/types.rs .omo/evidence/task-51-cfg-implementation.txt; do git diff --no-index --check -- /dev/null "$f"; done
rg -n "evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_" . --glob '!target/**' --glob '!.git/**'
```

Key results:

- `cargo fmt --all -- --check`: PASS, exit 0.
- `cargo check --release`: PASS, exit 0.
- `git diff --check`: PASS, exit 0.
- Explicit `git diff --no-index --check` over untracked new CFG/evidence files: no whitespace-error output; command exits 1 because files differ from `/dev/null`, not because whitespace errors were reported.
- Exact stale-fingerprint scan over `src`, `tests`, manifests, and Task 51 evidence: PASS; no matches (`rg` exit 1).
- Broad exact stale-fingerprint scan excluding `target`/`.git`: matches only historical `docs/COACHING_LOG.md` references, not source/harness/runtime implementation.
- `cargo test --release`: PASS; 10 unit tests passed, doc tests passed.

## CFG adequacy assessment

PASS. The implementation provides the required generic CFG foundation for W20-T2..T5:

- Generic instruction seam: `CfgInstruction::simplify` and `SimpleInstr` classify labels, conditional jumps, unconditional jumps, returns, and other instructions (`src/ir/cfg/instr.rs:9-20`).
- TACKY seam: labels, jumps, conditional jumps, and returns are recognized explicitly (`src/ir/cfg/instr.rs:22-31`), with all other TACKY variants exhaustively classified as `Other` (`src/ir/cfg/instr.rs:32-59`).
- Assembly seam: `Label`, `Jmp`, `JmpCC`, and `Ret` are recognized explicitly (`src/ir/cfg/instr.rs:64-70`), with all other assembly variants exhaustively classified as `Other` (`src/ir/cfg/instr.rs:71-102`).
- Basic blocks and graph shape: `NodeId::{Entry, Block, Exit}`, `BlockId`, `BasicBlock`, `Cfg`, `FunctionCfg`, `preds`, `succs`, `entry_succs`, and `exit_preds` are present (`src/ir/cfg/types.rs:11-44`).
- Edge APIs and bidirectional maintenance: `add_edge` / `remove_edge` update successor and predecessor sides (`src/ir/cfg/types.rs:117-124`).
- Reachability and dataflow seams: `get_succs`, `block_ids`, `blocks`, `blocks_mut`, `reachable_block_ids`, `initialize_annotation`, `strip_annotations`, and `cfg_to_instructions` are available for forward/backward dataflow and reassembly (`src/ir/cfg/types.rs:47-79`, `src/ir/cfg/types.rs:127-178`).
- Block splitting: starts a new block at labels and terminates blocks after conditional jumps, unconditional jumps, and returns (`src/ir/cfg/build.rs:91-117`), matching the inspected OCaml `partition_into_basic_blocks` shape.
- Edge construction:
  - Empty instruction slice yields Entry -> Exit (`src/ir/cfg/build.rs:139-142`), a safe extension over the OCaml assumption that functions have blocks.
  - Non-empty CFG yields Entry -> Block(0) (`src/ir/cfg/build.rs:144`).
  - Returns flow to Exit, unconditional jumps flow to the target label block, conditional jumps flow to both fallthrough and target, and ordinary/label-ended blocks fall through to next block or Exit (`src/ir/cfg/build.rs:166-190`).
  - Missing jump labels return a typed `CfgBuildError` rather than panicking (`src/ir/cfg/build.rs:195-203`).
- Public seams: `build`, `build_tacky_program`, `tacky_function_cfg`, `assembly_function_cfg`, type aliases `TackyCfg` / `AssemblyCfg`, and core types are re-exported from `src/ir/cfg.rs:13-17`.

## Non-goals / scope checks

- Optimization passes are not implemented by this diff. `src/ir/opt.rs:34-35` still contains the pre-existing `unimplemented!()` stub and is unchanged by Task 51.
- Register allocation is not implemented by this diff. `src/codegen/regalloc/mod.rs:19-20` still contains the pre-existing `unimplemented!("ch.20 regalloc wired in wave 21")` stub and is unchanged by Task 51.
- No test harness weakening: no files under `tests/` were changed or newly added for this task.
- No forbidden bridge/interpreter fingerprints in source/test/manifests/task evidence: exact scan for `evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_` returned no matches in those paths.
- No new dependencies: `Cargo.toml` and `Cargo.lock` have no diff.
- No unsupported Rust features in changed CFG files: no `unsafe`, `#![feature]`, inline asm, process bridge, or system-frontend bridge usage found.

## `#![allow(dead_code)]` assessment

`src/ir/cfg.rs:7` uses `#![allow(dead_code)]`. This is task-appropriate temporary scaffolding rather than a blocker:

- Task 51 introduces a CFG seam before W20-T2..T5 consume it, so many public/private helpers are intentionally unused until subsequent optimization tasks wire them in.
- The repository already uses module-level `dead_code` allowances for staged scaffolding in comparable modules (`src/ir/opt.rs:15`, `src/codegen/regalloc/mod.rs:11`, `src/ir/tacky.rs:10`, `src/codegen/assembly.rs:9`).
- The allowance should be revisited once W20 passes consume the CFG; leaving it indefinitely would reduce dead-code signal.

## Findings by severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

1. **No direct CFG edge-construction regression tests yet.**
   - Evidence: Codegraph reported no covering tests for the new CFG symbols; `cargo test --release` runs only existing compiler/driver tests and no CFG-specific tests; `git diff --name-only -- tests src` shows no test changes under `tests/`.
   - Impact: The CFG edge rules were verified by source inspection against `nqcc2/lib/cfg.ml`, but adversarial gate should exercise labels, unconditional jumps, conditional jumps, returns, fallthrough, missing-label errors, and empty/label-only blocks before W20-T2..T5 rely on the API.
   - Approval impact: Not a blocker for proceeding to adversarial gate because Task 51 acceptance is `cargo check --release` plus a usable seam, and this review confirms the seam shape manually. It is a required WATCH item for the gate.

### LOW

1. **Runtime-state noise outside the listed CFG source files.**
   - Evidence: `.omo/boulder.json` has tracked updates adding/resuming Task 50/51 session state and timestamps.
   - Impact: Not source/test harness behavior, but it is outside the user-listed CFG implementation files and should be handled intentionally if/when committing.
   - Approval impact: Not a blocker.

2. **`build_tacky_program` returns only function name plus CFG, not full `TackyFunction` metadata.**
   - Evidence: `src/ir/cfg/build.rs:69-81` returns `Vec<FunctionCfg<tacky::Instruction>>`, and `FunctionCfg` contains only `name` and `cfg` (`src/ir/cfg/types.rs:40-44`), while `TackyFunction` also carries `global`, `params`, `type_env`, `ast_type_env`, and `return_type` (`src/ir/tacky.rs:293-300`).
   - Impact: W20 consumers must reattach optimized instruction bodies to the original `TackyFunction` rather than reconstructing functions solely from `FunctionCfg`. This is consistent with a CFG builder seam and not a correctness defect in Task 51.
   - Approval impact: Not a blocker; document for W20-T2..T5 implementers.

## Final decision

VERDICT: PASS

The current CFG implementation is acceptable to proceed to the adversarial gate. No CRITICAL or HIGH findings remain. The adversarial gate should focus on behavior tests for edge construction and downstream W20 pass integration, especially conditional jump fallthrough/target ordering, missing-label diagnostics, unreachable block filtering, and preserving original `TackyFunction` metadata during reassembly.
