# Task 54 / W20-T4 Copy Propagation Code Review

VERDICT: REJECT

codeQualityStatus: BLOCK
recommendation: REQUEST_CHANGES
reportPath: `.omo/evidence/task-54-copy-prop-code-review.md`

## Scope reviewed

Goal: Review current uncommitted diff for Task 54 / W20-T4 Chapter 19 copy propagation and decide whether it can proceed to adversarial gate.

Current HEAD inspected: `bd4aad2` (`feat(compiler): chapter 19: unreachable code elimination`).

Product files inspected:
- `src/ir/copy_propagation.rs`
- `src/ir/copy_propagation/{facts,dataflow,rewrite,cleanup}.rs`
- `src/ir/opt.rs`
- `src/pipeline.rs`
- `src/ir/mod.rs`
- `src/ir/constant_folding.rs`
- `src/ir/constant_folding/{folds,instr}.rs`
- `src/codegen/codegen.rs`
- `.omo/evidence/task-54-copy-prop-implementation.txt`

Reference/policy files inspected:
- `.omo/plans/c-compiler-rust.md` Task 54 and global test policy
- `docs/book/test-map.md`
- `nqcc2/lib/optimizations/copy_prop.ml`
- `nqcc2/lib/backward_dataflow.ml`
- `nqcc2/lib/optimizations/optimize_utils.ml`
- `nqcc2/lib/optimizations/address_taken.ml`
- `nqcc2/lib/optimizations/optimize.ml`
- `nqcc2/lib/optimizations/constant_folding.ml`
- `tests/tests/chapter_19/copy_propagation/README.md`
- `tests/test_framework/tacky/copy_prop.py`

## Skill-perspective check

Ran/consulted required skill perspectives before judgment:
- `omo:remove-ai-slops`: loaded `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md` completely. Applied its overfit/slop criteria: no test additions, no deletion-only/tautological/implementation-mirroring tests, but production complexity/file-size slop is present.
- `omo:programming`: loaded `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md` and Rust reference `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md`. Applied Rust criteria: strict types/no unsafe passed; 250 pure-LOC ceiling failed.

Diff violates the `programming` file-size perspective and the `remove-ai-slops` oversized-module/slop perspective.

## Exact commands inspected/run

Required gates run:

```bash
cargo fmt --all -- --check
cargo check --release
git diff --check
cargo test --release
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --propagate-copies
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-unreachable-code
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants
rg -n 'evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_' src || true
rg -n '\bunsafe\b' src/ir/copy_propagation.rs src/ir/copy_propagation src/ir/opt.rs src/pipeline.rs src/ir/mod.rs src/ir/constant_folding.rs src/ir/constant_folding src/codegen/codegen.rs || true
rg -n '#\[cfg\(test\)|#\[test\]|mod tests' src/ir/copy_propagation.rs src/ir/copy_propagation src/ir/opt.rs src/pipeline.rs src/ir/mod.rs src/ir/constant_folding.rs src/ir/constant_folding src/codegen/codegen.rs || true
git diff -- tests Cargo.toml Cargo.lock
```

Key inspection commands run:

```bash
git status --short
git diff --stat -- src .omo/evidence/task-54-copy-prop-implementation.txt tests Cargo.toml Cargo.lock
git diff --name-status -- . ':!target'
git diff -- src/codegen/codegen.rs src/ir/constant_folding.rs src/ir/constant_folding/folds.rs src/ir/constant_folding/instr.rs src/ir/mod.rs src/ir/opt.rs src/pipeline.rs
for f in src/ir/copy_propagation.rs src/ir/copy_propagation/facts.rs src/ir/copy_propagation/dataflow.rs src/ir/copy_propagation/rewrite.rs src/ir/copy_propagation/cleanup.rs; do nl -ba "$f"; done
for f in nqcc2/lib/optimizations/copy_prop.ml nqcc2/lib/backward_dataflow.ml nqcc2/lib/optimizations/optimize_utils.ml nqcc2/lib/optimizations/address_taken.ml nqcc2/lib/optimizations/optimize.ml; do nl -ba "$f"; done
for f in src/ir/constant_folding/instr.rs src/codegen/codegen.rs src/ir/copy_propagation/rewrite.rs; do awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#)/ {c++} END{print c+0}' "$f"; git show HEAD:"$f" | awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#)/ {c++} END{print c+0}'; done
./target/release/rustcc --codegen --fold-constants --eliminate-unreachable-code --propagate-copies tests/tests/chapter_19/copy_propagation/all_types/pointer_arithmetic.c
./target/release/rustcc --codegen --fold-constants --eliminate-unreachable-code --propagate-copies tests/tests/chapter_19/copy_propagation/int_only/propagate_static_var.c
```

Exploratory large-offset probe was started and interrupted with status 130 because the lowerer expands huge static aggregate initialization before codegen; it was cleaned from `/tmp` and is not used as a blocker.

## Verification results

- `cargo fmt --all -- --check`: PASS, exit 0.
- `cargo check --release`: PASS, exit 0 (`Finished release profile ... 0.02s`).
- `git diff --check`: PASS, exit 0.
- `cargo test --release`: PASS, exit 0; `src/main.rs` ran 10 tests, all passed; `src/lib.rs` and doc-tests ran 0 tests.
- `cargo build --release`: PASS, exit 0.
- `--chapter 19 --latest-only --propagate-copies`: PASS, 42 tests, `OK`.
- `--chapter 19 --latest-only --eliminate-unreachable-code`: PASS, 15 tests, `OK`.
- `--chapter 19 --latest-only --fold-constants`: PASS, 16 tests, `OK`.
- Forbidden bridge/interpreter fingerprint scan: PASS, no matches in `src`.
- Unsafe scan over changed Rust files: PASS, no matches.
- Compiler-phase Rust unit-test scan over changed files: PASS, no matches.
- Test harness/dependency diff: PASS, `git diff -- tests Cargo.toml Cargo.lock` empty.

## Findings by severity

### CRITICAL

None.

### HIGH

1. **Task 54 pushes a touched Rust module over the 250 pure-LOC ceiling.**
   - File: `src/ir/constant_folding/instr.rs` (current lines 1-319; main growth around `optimize_instruction` lines 43-267 and duplicated helper lines 304-319).
   - Evidence: measurement command reports `src/ir/constant_folding/instr.rs current_pure_loc=303`; `HEAD` was `225` pure LOC.
   - Why this blocks: the user explicitly required rejection for file-size/slop violations. The loaded `omo:programming` and `omo:remove-ai-slops` perspectives treat `>250` pure LOC as a defect, and this branch crosses the threshold as part of Task 54. The added static-variable plumbing also duplicates `remember_constant`/`forget_constant` in both `folds.rs` and `instr.rs`, increasing maintenance burden rather than splitting or simplifying.
   - Required before approval: reduce/split `src/ir/constant_folding/instr.rs` below the 250 pure-LOC ceiling or provide a narrow, reviewable structural split that keeps behavior unchanged and preserves official harness results.

2. **Task 54 adds code to an already oversized codegen module without reducing the oversized hotspot.**
   - File: `src/codegen/codegen.rs` current pure LOC `1949`; `HEAD` was `1910`. New code is at `src/codegen/codegen.rs:393-422`, `src/codegen/codegen.rs:476-503`, and `src/codegen/codegen.rs:1881-1890`.
   - Evidence: measurement command reports `src/codegen/codegen.rs current_pure_loc=1949`, `head_pure_loc=1910`.
   - Nuance: the call-argument register copy and constant-index `AddPtr` lowering are plausibly scoped to official copy-prop assembly validators (`same_arg_test` and `pointer_arithmetic.c`), and spot-checks showed expected assembly shape. However, under the loaded skill rules and the user's explicit file-size/slop gate, adding more production logic to a 1,900+ pure-LOC file is still a file-size/slop violation unless refactored or explicitly accepted by policy.
   - Required before approval: either move these narrowly related codegen helpers into an existing/smaller responsibility boundary or record an accepted project-level exception for this book-mirror codegen file before proceeding.

### MEDIUM

1. **`copy_propagation::cleanup` is a narrowly scoped dead-store-like cleanup under the copy-prop flag.**
   - File: `src/ir/copy_propagation/cleanup.rs:7-24`, especially `copybytes_writes_unused_temp` at `src/ir/copy_propagation/cleanup.rs:45-54`.
   - Evidence: it removes `GetAddress` instructions whose destination is unused and removes `CopyBytes` when the destination base is an unused `tmp.*` aggregate temp.
   - Assessment: this appears intended to remove aggregate assignment-result scaffolding created by the Rust lowering representation after redundant aggregate copies are eliminated. It does not wire `OptPass::DeadStoreElim` (`src/ir/opt.rs:56` remains a no-op) and does not add liveness/regalloc/coalescing. Still, it is broader than the OCaml `copy_prop.ml` scalar-copy deletion and should be kept minimal/justified because W20-T5 owns full DSE.

2. **Unconditional constant-index `AddPtr` displacement lowering has an unchecked narrowing cast.**
   - File: `src/codegen/codegen.rs:1881-1888` computes `(n * *scale) as i32`.
   - Evidence: diff changes the previous register-index path into a direct displacement path for every constant index. This is needed for the official `pointer_arithmetic.c` copy-prop assembly validator (no `imul`/`movsx`), but it should ideally fall back to the old register path if the scaled displacement does not fit in signed 32 bits.
   - Assessment: no accepted official test failed and no minimal observable repro was confirmed in this review, so this is not the blocking reason. It is a regression risk to address when touching this hunk again.

### LOW

1. **`src/ir/copy_propagation/rewrite.rs` is close to the warning band limit.**
   - Evidence: current pure LOC `244`.
   - Assessment: below the 250 hard ceiling, but near enough that future edits should split by source-rewrite responsibility before adding logic.

2. **State/evidence dirtiness exists outside product code.**
   - Evidence: `git status --short` shows `.omo/boulder.json` modified and several historical untracked `.omo/evidence/*` files. `.omo/boulder.json` diff is task/session state, not source/harness behavior.
   - Assessment: not a Task 54 product blocker, but final handoff should distinguish product diffs from workflow-state diffs.

## Semantic/scope audit notes

- Copy propagation is wired only when `OptimizationFlags.propagate_copies` is set: `src/driver.rs:41-47`, `src/driver.rs:94-99`, `src/pipeline.rs:81-91`, `src/ir/opt.rs:51-55`.
- `OptPass::DeadStoreElim` remains no-op at `src/ir/opt.rs:56`; no regalloc/liveness/coalescing implementation was added in this diff.
- The pass uses CFG/dataflow: `src/ir/copy_propagation.rs:60-72`, `src/ir/copy_propagation/dataflow.rs:15-40`, `src/ir/copy_propagation/rewrite.rs:10-29`.
- `TackyFunction` metadata is preserved by replacing only `function.body`: `src/ir/copy_propagation.rs:54-77`.
- Address-taken/static/call/store handling follows the OCaml intent: `src/ir/copy_propagation.rs:80-89`, `src/ir/copy_propagation/facts.rs:117-127`, `src/ir/copy_propagation/dataflow.rs:143-155`, `src/ir/copy_propagation/dataflow.rs:187-204`.
- Constant folding changes are plausibly scoped to keep static-variable copies visible for copy-prop assembly validators such as `propagate_static_var.c`; `--fold-constants` regression still passes 16 tests.
- Codegen changes are plausibly scoped to official assembly-sensitive copy-prop validators: `same_arg_test` expects `%edi -> %esi`-style same-arg evidence, and `pointer_arithmetic.c` expects no computation instructions beyond allowed moves/leas. Spot-checks showed `movl %edi, %esi` before `call callee` for `propagate_static_var.c` and no `imul`/`movsx` in the pointer arithmetic target body apart from prologue stack adjustment.

## Blockers

- `src/ir/constant_folding/instr.rs` now exceeds the 250 pure-LOC ceiling due to Task 54 changes (`303` current vs `225` at `HEAD`). This is a HIGH file-size/slop violation under the required skill perspectives and the user's explicit rejection rule.
- `src/codegen/codegen.rs` remains a 1,949 pure-LOC touched module with new Task 54 logic added. The codegen hunks are likely necessary for official assembly validators, but the size/slop policy requires refactor/exception before approval.
