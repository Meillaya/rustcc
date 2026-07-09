VERDICT: APPROVE

# Task 58 harness-scope code review — W21-T3 Chapter 20 coloring/select

- **codeQualityStatus:** WATCH
- **recommendation:** APPROVE
- **reportPath:** `.omo/evidence/task-58-harness-scope-code-review.md`
- **Review mode:** read-only code review; wrote only this report artifact.
- **Scope reviewed:** current uncommitted Task58 repair in `/home/mei/projects/rustcc`, especially the restoration of `tests/test_framework/runner.py` and the `RegallocOptions::default()` no-coalescing default.

## Summary verdict

APPROVE. The harness-scope repair is acceptable under the W21-T3 plan scope:

- `git diff -- tests` is empty, and `git diff -- tests/test_framework/runner.py` is empty. The official Python harness is no longer patched to forward `--no-coalescing`.
- A `/tmp` compiler-wrapper run of the required acceptance command logged 66 compiler invocations and **0** invocations containing `--no-coalescing`; the command still passed. This proves the green Chapter 20 no-coalescing gate now comes from the compiler default, not from a test-runner bridge.
- Production source has no `source_path_hint`, `chapter_20`/`chapter20`, `latest-only`, `test_framework`, or test-name bridge.
- GP allocatable registers match the OCaml reference and exclude R10/R11; XMM allocatable registers exclude XMM14/XMM15.
- The durable probe's no-R11 OCaml color mapping remains valid and ran successfully.
- Setting `RegallocOptions::default().coalescing_enabled = false` is reasonable for W21-T3: Task58 only requires `--chapter 20 --latest-only --no-coalescing` (`.omo/plans/c-compiler-rust.md:1926-1938`), while W21-T5 explicitly owns coalescing/default-with-coalescing (`.omo/plans/c-compiler-rust.md:1955-1960`). Fresh prior-chapter/regression gates still pass.

## Findings by severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

#### MEDIUM-1: New allocation orchestration is in the programming warning band, so W21-T4/W21-T5 should split before adding more logic

- `src/codegen/regalloc/allocate.rs` is 247 pure LOC by the programming skill measurement. This is below the 250 pure-LOC defect threshold but already in the warning band.
- Related touched files also sit near or above the warning/legacy-defect bands:
  - `src/codegen/regalloc/graph.rs`: 237 pure LOC.
  - `src/codegen/fixup.rs`: 235 pure LOC.
  - `src/ir/copy_propagation/rewrite_support.rs`: 239 pure LOC.
  - `.omo/evidence/task-58-coloring-probe.rs`: 231 pure LOC.
  - Pre-existing oversized legacy files touched by the branch include `src/driver.rs` (278), `src/codegen/emit.rs` (504), and `src/codegen/codegen.rs` (2006).
- This does **not** block Task58 because the new source remains under 250 and the required gates pass. It is a clear next-edit constraint: W21-T4 spill/reallocation and W21-T5 coalescing should not keep appending to `allocate.rs` without splitting responsibilities.

#### MEDIUM-2: Chapter 20 helper `.s` files are ignored/untracked in the local test tree

- `git status --short --ignored -- tests/tests/chapter_20/helper_libs` shows three ignored helper assembly fixtures: `alignment_check_wrapper_linux.s`, `clobber_xmm_regs_linux.s`, and `wrapper_linux.s`.
- `git diff -- tests` and `git status --short -- tests` are still empty, so this is not the harness-modification blocker from the prior reviews.
- Treat this as a delivery/durability risk for a clean checkout, not as a Task58 repair blocker. The current review requirement was to reject if a tests/harness file remained modified; no tracked tests/harness diff remains.

### LOW

#### LOW-1: The durable probe remains an evidence artifact rather than integrated regression coverage

- `.omo/evidence/task-58-coloring-probe.rs:116-118` pins the expected GP color map without R11, and `.omo/evidence/task-58-coloring-probe.rs:222-228` asserts R10/R11/XMM14/XMM15 are not allocatable.
- The probe compiled and ran successfully, but it is still manual evidence outside Cargo's normal test suite. This is acceptable for Task58 evidence, but future register-allocation work should keep running it or migrate equivalent checks into a durable gate if project policy changes.

#### LOW-2: Several Task58 source files are untracked in the current uncommitted worktree

- `git ls-files --others --exclude-standard src/codegen/regalloc .omo/evidence/task-58-coloring-probe.rs` reports `src/codegen/regalloc/{allocate,color,rewrite,scratch}.rs` and the probe artifact.
- This is expected for a review of uncommitted work, but final delivery must stage/include the new source files.

## Required checklist results

- **`git diff -- tests` empty:** PASS (`exit 0`; no output). Post-test rerun also PASS.
- **`tests/test_framework/runner.py` restored:** PASS (`git diff -- tests/test_framework/runner.py` exit 0; no output). Current runner builds `cc_options` from `args.extra_cc_options` plus optimization flags only (`tests/test_framework/runner.py:468-508`) and does not append `--no-coalescing`.
- **No production test/path/name bridge:** PASS. `rg -n "source_path_hint|with_source_path_hint|chapter_20|chapter20|latest-only|test_framework|test-name|test_name" src` exited 1 with no matches.
- **No harness forwarding of `--no-coalescing`:** PASS. Wrapper run of `./tests/test_compiler /tmp/task58-rustcc-arglog --chapter 20 --latest-only --no-coalescing` exited 0, ran 66 tests OK, logged 66 compiler invocations, and logged 0 invocations containing `--no-coalescing`.
- **Compiler default no-coalescing is scoped and acceptable:** PASS. `RegallocOptions::default()` sets `coalescing_enabled: false` with a W21-T3/W21-T5 comment (`src/driver.rs:54-61`), and allocation is selected by `!options.regalloc_options.coalescing_enabled` (`src/compiler.rs:123-130`). W21-T5 owns flipping/implementing coalescing (`.omo/plans/c-compiler-rust.md:1955-1960`). Prior-chapter checks passed.
- **GP excludes R10/R11:** PASS. Rust GP hardregs/caller-saved/contains are `AX,BX,CX,DX,DI,SI,R8,R9,R12,R13,R14,R15` and exclude R10/R11 (`src/codegen/regalloc/types.rs:52-67`, `src/codegen/regalloc/types.rs:72-83`, `src/codegen/regalloc/types.rs:87-104`), matching OCaml (`nqcc2/lib/backend/regalloc.ml:607-610`).
- **XMM excludes XMM14/XMM15:** PASS. Rust uses `0..=13` for XMM hardregs/caller-saved/contains (`src/codegen/regalloc/types.rs:68`, `src/codegen/regalloc/types.rs:83`, `src/codegen/regalloc/types.rs:104`), matching OCaml XMM0-XMM13 (`nqcc2/lib/backend/regalloc.ml:614-635`).
- **Durable probe no-R11 mapping remains valid:** PASS. Probe output includes `{0: R9, 1: R8, 2: SI, 3: DI, 4: DX, 5: CX, 6: AX, 7: BX, 8: R12, 9: R13, 10: R14, 11: R15}` and no R11; `rustc ... && /tmp/task58-coloring-probe-review` exited 0.
- **Scratch/rewrite hygiene:** PASS with risk noted. `src/codegen/regalloc/rewrite.rs` is 143 pure LOC and uses the fixed let-chain at lines 36-41. `src/codegen/regalloc/scratch.rs` is 74 pure LOC; its R9->R11 reserved-address rewrite is specialized but small and tied to the observed regalloc scratch hazard (`src/codegen/regalloc/scratch.rs:21-77`). No blocker found.
- **Remove-ai-slops perspective:** PASS. I did not find deletion-only tests, tautological tests, tests that merely assert removal, or production parsing/normalization unrelated to the goal. The probe is implementation-adjacent but directly checks the required OCaml-derived color/reserved-register invariants.
- **Programming perspective:** PASS with WATCH. No new untyped escape hatches or unnecessary production validation layers were found in the Task58 regalloc files. The main programming concern is size pressure in `allocate.rs` (247 pure LOC warning band), not a current defect.

## Exact checks inspected/run

### Skill/perspective checks

```text
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md -> exit 0
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md -> exit 0
cat/sed /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md -> exit 0
cat/sed /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/python/README.md -> exit 0
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/code-smells.md -> exit 0
```

### Evidence/source/diff inspection

```text
git status --short -> exit 0
git diff --name-status -> exit 0
git diff --stat -> exit 0
git diff -- tests -> exit 0, no output
git diff -- tests/test_framework/runner.py -> exit 0, no output
git diff --check -> exit 0
rg -n "source_path_hint|with_source_path_hint|chapter_20|chapter20|latest-only|test_framework|test-name|test_name" src -> exit 1, no matches
rg -n "Reg::R10|Reg::R11|XMM\(14\)|XMM\(15\)" src/codegen/regalloc/types.rs -> exit 1, no matches
pure LOC awk measurement over reviewed files -> exit 0
```

Inspected artifacts/files:

```text
.omo/evidence/task-58-harness-scope-fix.txt
.omo/evidence/task-58-final-fix-code-review-2.md
.omo/evidence/task-58-final-fix-adversarial-verify.txt
.omo/evidence/task-58-coloring-probe.rs
.omo/plans/c-compiler-rust.md
nqcc2/lib/backend/regalloc.ml
src/driver.rs
src/compiler.rs
src/pipeline.rs
src/codegen/regalloc/{types,color,graph,allocate,rewrite,scratch,mod,operands}.rs
src/codegen/{codegen.rs,codegen/copy_prop_support.rs,emit.rs,fixup.rs}
src/ir/copy_propagation/rewrite_support.rs
tests/test_framework/runner.py
```

### Fresh build/test/probe commands

```text
cargo fmt --all -- --check -> exit 0
cargo check --release -> exit 0
cargo build --release -> exit 0
cargo test --release -> exit 0; 10 binary unit tests passed, lib/doc tests empty
./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing -> exit 0; Ran 66 tests, OK
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only -> exit 0; Ran 120 tests, OK
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores -> exit 0; Ran 27 tests, OK
./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union -> exit 0; Ran 286 tests, OK; assembler truncation warnings only
rustc .omo/evidence/task-58-coloring-probe.rs -o /tmp/task58-coloring-probe-review && /tmp/task58-coloring-probe-review -> exit 0; standalone dead-code warnings only
cargo clippy --release --bin rustcc -- -A warnings -W clippy::collapsible_if -> exit 0
/tmp wrapper harness probe: ./tests/test_compiler /tmp/task58-rustcc-arglog --chapter 20 --latest-only --no-coalescing -> exit 0; Ran 66 tests, OK; logged_invocations=66; invocations_with_no_coalescing=0
post-test git diff -- tests -> exit 0, no output
post-test git status --short -- tests -> exit 0, no output
```

## Blockers before approval

None.

Final verdict: APPROVE
