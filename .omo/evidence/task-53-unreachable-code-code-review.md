# Task 53 / W20-T3 Code Review — Chapter 19 unreachable code elimination

VERDICT: REJECT

Review date: 2026-07-09
Workspace: `/home/mei/projects/rustcc`
HEAD inspected: `57fe882` (`feat(compiler): chapter 19: constant folding`)
Review scope: current uncommitted Task 53 diff plus relevant untracked source/evidence.

## Required skill-perspective check

- Loaded and consulted `omo:remove-ai-slops` from `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md`.
- Loaded and consulted `omo:programming` from `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md`.
- Loaded and consulted Rust-specific programming reference `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md`.
- Skill-perspective result: production UCE implementation is small enough (<250 pure LOC), has no `unsafe`, no new dependencies, and uses existing CFG/TACKY seams. However, the new compiler-phase unit test violates both the project plan policy and the programming/remove-ai-slops test relevance perspective because it is an implementation-level compiler phase test in `src/ir`, not official harness coverage. This is a blocker.

## Plan/reference material inspected

- `.omo/plans/c-compiler-rust.md` lines 24-29: locked decisions, including strict OCaml mirror and per-chapter official harness gating.
- `.omo/plans/c-compiler-rust.md` lines 91-92: official `test_compiler` harness is the only compiler-correctness verification path; tests directory must remain unchanged.
- `.omo/plans/c-compiler-rust.md` line 104: guardrail forbids Rust unit tests for compiler phases.
- `.omo/plans/c-compiler-rust.md` line 121: explicit policy: “No Rust unit tests for compiler phases.” Existing tests are limited to CLI/orchestration glue.
- `.omo/plans/c-compiler-rust.md` lines 1850-1864: Task 53 acceptance is chapter 19 latest-only with `--eliminate-unreachable-code`.
- `nqcc2/lib/optimizations/unreachable_code_elim.ml`: removes unreachable blocks via CFG reachability from `Entry`, then eliminates useless jumps, useless labels, and empty blocks.
- `nqcc2/lib/optimizations/optimize.ml`: optimization order is constant folding, CFG construction, UCE, copy propagation, DSE, repeated to fixed point.
- `.omo/evidence/task-53-unreachable-code-implementation.txt`: executor evidence; treated as untrusted and cross-checked with commands below.

## Diff/source inspected

- `git status --short` showed:
  - `M .omo/boulder.json`
  - `M src/ir/mod.rs`
  - `M src/ir/opt.rs`
  - `M src/pipeline.rs`
  - `?? .omo/evidence/task-53-unreachable-code-implementation.txt`
  - `?? src/ir/unreachable_code_elim.rs`
  - plus older unrelated untracked `.omo/evidence/task-*` files and `.omo/start-work/ledger.jsonl`.
- `git diff -- src/ir/mod.rs src/ir/opt.rs src/pipeline.rs`
- `git diff --no-index -- /dev/null src/ir/unreachable_code_elim.rs`
- `nl -ba` on changed source files and CFG helpers:
  - `src/ir/unreachable_code_elim.rs`
  - `src/ir/opt.rs`
  - `src/pipeline.rs`
  - `src/ir/mod.rs`
  - `src/ir/cfg/types.rs`
  - `src/ir/cfg/build.rs`
  - `src/ir/cfg/instr.rs`
  - `src/driver.rs`
  - `src/compiler.rs`

## Commands run / inspected

```text
cargo fmt --all -- --check
# exit 0, no output

cargo check --release
# exit 0
# Finished `release` profile [optimized] target(s) in 0.03s

cargo build --release
# exit 0
# Finished `release` profile [optimized] target(s) in 0.01s

 git diff --check
# exit 0, no output

git diff --no-index --check /dev/null src/ir/unreachable_code_elim.rs
# no whitespace-error output; exit 1 is expected for --no-index when the compared files differ

./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-unreachable-code
# exit 0
# Ran 15 tests in 0.331s, OK

./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants
# exit 0
# Ran 16 tests in 0.419s, OK

rg -n 'evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_' src
# exit 1, no source matches

git diff -- Cargo.toml Cargo.lock tests
# no output

git status --short -- Cargo.toml Cargo.lock tests
# no output

grep -RIn '#\[cfg(test)\]\|#\[test\]' src/ir src/codegen src/semantics src/parse src/lex
# src/ir/unreachable_code_elim.rs:184:#[cfg(test)]
# src/ir/unreachable_code_elim.rs:191:    #[test]

cargo test --release -- --list
# showed 11 tests total, including:
# ir::unreachable_code_elim::tests::removes_dead_block_after_return_when_unreachable_code_elim_runs: test

./target/release/rustcc --tacky /tmp/rustcc-task53-ucc-probe.c
# without --eliminate-unreachable-code, TACKY still included "user_label.main.dead_label.0"

./target/release/rustcc --tacky --eliminate-unreachable-code /tmp/rustcc-task53-ucc-probe.c
# with the flag, optimized TACKY body contained only Return(Constant(7))
```

Additional full-repo fingerprint scan excluding `target/` and `.git/` found only documentation/coaching-log references, not source reintroductions.

## Functional observations

- UCE is flag-gated in source:
  - `src/driver.rs:94-97` sets `OptimizationFlags::eliminate_unreachable_code` only for `--eliminate-unreachable-code`.
  - `src/pipeline.rs:81-88` pushes `OptPass::UnreachableCodeElim` only when that flag is true.
  - Manual `/tmp` TACKY probe confirmed dead labeled code remains without the flag and is removed with the flag.
- UCE uses CFG reachability:
  - `src/ir/unreachable_code_elim.rs:70-92` calls `cfg.reachable_block_ids()` and retains only reachable basic blocks.
  - `src/ir/cfg/types.rs:127-141` computes reachable block ids by traversing successors from `NodeId::Entry`.
- TackyFunction metadata is preserved:
  - `src/ir/unreachable_code_elim.rs:42-58` mutates only `function.body`; the program-level wrapper carries `static_variables`, `static_constants`, `function_param_types`, and `function_return_types` through at `src/ir/unreachable_code_elim.rs:25-32`.
- Copy propagation, DSE, and regalloc are not implemented by this diff:
  - `src/ir/opt.rs:50` keeps `OptPass::CopyPropagation | OptPass::DeadStoreElim` as no-ops.
  - `src/pipeline.rs:81-88` does not wire `propagate_copies` or `eliminate_dead_stores`.
- No new dependencies or test harness changes were found:
  - `git diff -- Cargo.toml Cargo.lock tests` had no output.
- No `unsafe` was found in changed source files.
- Changed/new Rust pure LOC counts:
  - `src/ir/unreachable_code_elim.rs`: 182
  - `src/ir/opt.rs`: 33
  - `src/pipeline.rs`: 89
  - `src/ir/mod.rs`: 11

## Findings by severity

### CRITICAL

1. **Plan-policy violation: new compiler-phase Rust unit test was added.**
   - Evidence: `src/ir/unreachable_code_elim.rs:184-221` defines a `#[cfg(test)]` module and `#[test] fn removes_dead_block_after_return_when_unreachable_code_elim_runs` directly against the IR optimizer internals.
   - Evidence: `cargo test --release -- --list` reports `ir::unreachable_code_elim::tests::removes_dead_block_after_return_when_unreachable_code_elim_runs: test`, bringing the suite to 11 tests.
   - Policy conflict:
     - `.omo/plans/c-compiler-rust.md:91` says the official `test_compiler` Python harness is the only verification path for compiler correctness.
     - `.omo/plans/c-compiler-rust.md:104` forbids Rust unit tests for compiler phases.
     - `.omo/plans/c-compiler-rust.md:121` repeats “No Rust unit tests for compiler phases”; existing unit tests are only CLI/orchestration glue.
   - Why this blocks: the user explicitly required rejecting plan-policy violations, and this unit test is compiler-phase coverage in `src/ir`. It also violates the remove-ai-slops/programming test-shape perspective for this project because it adds an implementation-mirroring Rust test where the plan requires official harness/e2e evidence.
   - Required fix before approval: remove the `#[cfg(test)]` test module from `src/ir/unreachable_code_elim.rs` and rely on official harness evidence/manual CLI probes saved as evidence, not compiler-phase unit tests.

### HIGH

- None found beyond the CRITICAL blocker above.

### MEDIUM

- None found.

### LOW

1. **Scope hygiene: unrelated workflow/evidence noise is present in the worktree.**
   - Evidence: `git status --short` shows modified `.omo/boulder.json` and many older untracked `.omo/evidence/task-*` files unrelated to Task 53.
   - Impact: not a code correctness blocker for UCE, but it makes review/commit scope noisier. Keep the Task 53 code commit scoped to the four source files plus intentional evidence only.

2. **UCE silently treats CFG construction failure as “no change.”**
   - Evidence: `src/ir/unreachable_code_elim.rs:44-49` returns the original function with `changed: false` if `cfg::tacky_function_cfg(&function)` fails.
   - Impact: not shown to affect valid accepted programs or the current gate, but it is less fail-fast than the OCaml reference and may hide an internal compiler invariant violation. Consider tightening when optimizer error plumbing exists.

## Blockers

- Remove the compiler-phase Rust unit test at `src/ir/unreachable_code_elim.rs:184-221` to comply with the plan’s “no Rust unit tests for compiler phases” policy.

## Final recommendation

Do **not** proceed to adversarial gate yet. The functional UCE path and required harness checks are green, but the diff violates an explicit project guardrail. Re-run the same checks after removing the compiler-phase unit test.

codeQualityStatus: BLOCK
recommendation: REQUEST_CHANGES
reportPath: `.omo/evidence/task-53-unreachable-code-code-review.md`
