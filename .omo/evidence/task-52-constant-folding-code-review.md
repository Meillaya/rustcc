# Task 52 / W20-T2 Constant Folding Code Review

VERDICT: REJECT

codeQualityStatus: BLOCK
recommendation: REQUEST_CHANGES
reportPath: `.omo/evidence/task-52-constant-folding-code-review.md`

## Scope reviewed

Goal from `.omo/plans/c-compiler-rust.md:1830-1849`: implement Chapter 19 constant folding, with acceptance requiring:

- `int main(void) { int x = 2; x = x + 3; return x; }` emits folded `movl $5` in the destination slot.
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants` passes.

Changed implementation/evidence inspected:

- `src/ir/const_eval.rs` (new)
- `src/ir/constant_folding.rs` (new)
- `src/ir/opt.rs`
- `src/pipeline.rs`
- `src/ir/mod.rs`
- `.omo/evidence/task-52-constant-folding-implementation.txt`
- Current uncommitted state also includes `.omo/boulder.json` changes and unrelated untracked prior evidence files; no test harness or dependency file was changed by this task diff.

Reference implementation inspected:

- `nqcc2/lib/optimizations/constant_folding.ml`
- `nqcc2/lib/optimizations/optimize.ml`

## Skill-perspective check

Required review perspectives were loaded/consulted before judgment:

- `omo:remove-ai-slops`: ran the slop/overfit review perspective over production code and tests. No tests were added or weakened, so no deletion-only/tautological/implementation-mirroring test issue was found. Production slop findings remain: a dead helper in `src/ir/const_eval.rs:228` and an oversized new module in `src/ir/constant_folding.rs`.
- `omo:programming`: loaded `SKILL.md` and Rust reference `references/rust/README.md`. The diff violates the Rust size/maintainability perspective because `src/ir/constant_folding.rs` is 369 pure LOC (>250 limit). No `unsafe` was found. Numeric `as` casts in `const_eval.rs` are localized to C constant-conversion semantics, but need targeted tests around edge conversions.

## Commands inspected/run

Exact commands run from `/home/mei/projects/rustcc`:

```bash
git status --short
git rev-parse --short HEAD && git log -1 --oneline
git diff --name-status && git diff --stat
awk '/Task 52|W20-T2|constant folding|fold-constants/{print NR ":" $0}' .omo/plans/c-compiler-rust.md | sed -n '1,120p'
sed -n '1828,1855p' .omo/plans/c-compiler-rust.md
sed -n '1,260p' .omo/evidence/task-52-constant-folding-implementation.txt
git diff -- src/ir/mod.rs src/ir/opt.rs src/pipeline.rs .omo/boulder.json
git diff --no-index -- /dev/null src/ir/const_eval.rs
git diff --no-index -- /dev/null src/ir/constant_folding.rs
sed -n '1,240p' nqcc2/lib/optimizations/constant_folding.ml
sed -n '1,120p' nqcc2/lib/optimizations/optimize.ml
cargo fmt --all -- --check
cargo check --release
cargo build --release
git diff --check
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants
rg -n "evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_" . --glob '!target/**' --glob '!nqcc2/**' --glob '!writing-a-c-compiler-tests/**' --glob '!*.log'
rg -n "evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_" src tests Cargo.toml Cargo.lock || true
rg -n "\bunsafe\b" src/ir/const_eval.rs src/ir/constant_folding.rs src/ir/opt.rs src/pipeline.rs src/ir/mod.rs || true
git diff -- Cargo.toml Cargo.lock tests || true
for f in src/ir/const_eval.rs src/ir/constant_folding.rs src/ir/opt.rs src/pipeline.rs src/ir/mod.rs; do printf '%s ' "$f"; awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#)/' "$f" | wc -l; done
rg -n "\.unwrap\(|\.expect\(|\bas\b|#\[allow" src/ir/const_eval.rs src/ir/constant_folding.rs src/ir/opt.rs src/pipeline.rs src/ir/mod.rs || true
cargo clippy --release -- -D warnings
```

Targeted semantic probes run from `/home/mei/projects/rustcc` with temporary sources under `/tmp`:

```bash
# Critical repro
cat > /tmp/task52_nan.c <<'C'
int main(void) {
    double x = 0.0 / 0.0;
    return x != x;
}
C
./target/release/rustcc -S /tmp/task52_nan.c
cc /tmp/task52_nan.s -o /tmp/task52_nan
/tmp/task52_nan
timeout 5s ./target/release/rustcc --fold-constants -S /tmp/task52_nan.c

# Additional fold/no-fold probes executed:
# branch_zero, branch_nonzero, store_alias, call_alias, uchar_cast, unsigned_cmp, dead_divzero
```

## Gate results

- `cargo fmt --all -- --check`: PASS.
- `cargo check --release`: PASS (`Finished release profile ...`).
- `cargo build --release`: PASS; run to ensure `./target/release/rustcc` matched current sources before harness/probes.
- `git diff --check`: PASS for tracked diff.
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants`: PASS, `Ran 16 tests ... OK`.
- Fingerprint scan:
  - Broad scan found only historical documentation mentions in `docs/COACHING_LOG.md`.
  - Narrow `src tests Cargo.toml Cargo.lock` scan found no matches for `evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_`.
- Unsafe scan over changed Rust files: PASS, no `unsafe` found.
- Dependency/test harness diff: PASS, no `Cargo.toml`, `Cargo.lock`, or `tests/` diff.
- Optional `cargo clippy --release -- -D warnings`: FAILS on pre-existing project-wide warnings outside the Task 52 diff; not used as a Task 52 blocker.

## Reference comparison and scope-control observations

- `nqcc2/lib/optimizations/constant_folding.ml` folds literal TACKY constants, casts constants, folds constant conditional jumps, and handles integer division/modulo by zero by returning zero so dead UB does not crash the optimizer.
- `nqcc2/lib/optimizations/optimize.ml` applies constant folding only when the constant-folding option is enabled, then builds CFGs for the later optimization passes and repeats to a fixed point.
- Rust wiring is correctly restricted to `--fold-constants`: `src/driver.rs:94` sets only `fold_constants`; `src/pipeline.rs:81-85` pushes only `OptPass::ConstantFolding`; later flags remain parsed but unwired in `pipeline.rs`.
- The implementation preserves `TackyProgram` metadata in `src/ir/constant_folding.rs:15-26` and `TackyFunction` metadata in `src/ir/constant_folding.rs:29-49`, replacing only `function.body` after CFG reassembly.
- It does not implement UCE, DSE, regalloc, or general copy propagation. `src/ir/opt.rs:39-41` leaves `UnreachableCodeElim`, `CopyPropagation`, and `DeadStoreElim` as no-ops. It does perform local constant tracking/copying within a basic block, which is a constant-folding support mechanism for this repo's two-address TACKY form.
- Local tracking is conservative across calls/stores/blocks: constants are per block (`src/ir/constant_folding.rs:34-35`), and `Call`/`Store` clear the map (`src/ir/constant_folding.rs:180-187`). Probes for store and call aliasing returned expected values under `--fold-constants`.

## Findings by severity

### CRITICAL

1. `--fold-constants` can hang forever after folding a valid/accepted double expression to NaN.

   Evidence:

   - `src/ir/const_eval.rs:188-194` folds double division with native `left / right`, so `0.0 / 0.0` becomes `ConstVal::Double(NaN)`.
   - `src/ir/opt.rs:32-47` uses `current == previous` as the fixed-point termination condition.
   - `src/ir/tacky.rs:23-28` and `src/ir/tacky.rs:292-310` derive `PartialEq` over `Val::ConstantDouble(f64)` and `TackyProgram`; Rust `NaN != NaN`, so an unchanged optimized program containing NaN never reaches the fixed point.

   Minimal repro:

   ```c
   int main(void) {
       double x = 0.0 / 0.0;
       return x != x;
   }
   ```

   Observed commands/results:

   ```text
   ./target/release/rustcc -S /tmp/task52_nan.c
   cc /tmp/task52_nan.s -o /tmp/task52_nan
   /tmp/task52_nan
   exit=1

   timeout 5s ./target/release/rustcc --fold-constants -S /tmp/task52_nan.c
   status=124
   stdout: <empty>
   stderr: <empty>
   fold asm missing
   ```

   This is not caught by the official 16-test Chapter 19 gate but is a hard optimizer correctness/termination failure on an accepted Chapter 13+ program when Chapter 19 optimization is enabled. It blocks adversarial gate.

### HIGH

1. New production module exceeds the required programming maintainability ceiling.

   Evidence:

   ```text
   src/ir/constant_folding.rs 369 pure LOC
   ```

   The loaded `programming` Rust perspective treats files over 250 pure LOC as a defect unless explicitly justified/split. This new file contains several separable responsibilities (CFG pass driver, instruction fold dispatcher, cast/unary/binary helpers, type resolution, condition helpers). Even aside from the critical semantic bug, this should be split or substantially simplified before approval.

### MEDIUM

1. Dead private helper left in new evaluator.

   Evidence: `src/ir/const_eval.rs:228-230` defines `shift_u32`, and `rg -n "shift_u32"` found only that definition. This is remove-ai-slops category 6 dead code.

2. `src/ir/const_eval.rs` uses several raw `as` casts for C conversion semantics (`src/ir/const_eval.rs:65-68`, `238`, `244`, `246`, `248`, `256-257`). Some are likely intentional for C wrapping/truncation, but they need targeted tests for signed/unsigned byte casts, long/ulong boundary casts, and double-to-integer edge behavior. I verified one `unsigned char` and one unsigned compare probe, but coverage is not broad enough for the new evaluator surface.

### LOW

1. `.omo/boulder.json` was modified as session state alongside code. This is not a code correctness issue, but it is outside the Task 52 changed-file list provided for review and should be handled intentionally before commit/review handoff.

2. Broad forbidden-fingerprint scan finds historical mentions in `docs/COACHING_LOG.md`. Narrow source/test scan is clean, so this is informational only.

## Additional semantic audit results

Targeted fold/no-fold probes that passed:

- Constant branch folding:
  - `int main(void) { if (0) return 1; else return 2; }` -> no-fold exit 2, fold exit 2.
  - `int main(void) { if (3) return 1; else return 2; }` -> no-fold exit 1, fold exit 1.
- Local tracking invalidation:
  - `int main(void) { int x = 2; int *p = &x; *p = 7; return x; }` -> no-fold exit 7, fold exit 7.
  - `int mutate(int *p) { *p = 7; return 0; } int main(void) { int x = 2; mutate(&x); return x; }` -> no-fold exit 7, fold exit 7.
- Byte/unsigned sanity:
  - `unsigned char c = (unsigned char)255; return c;` -> no-fold/fold exit 255.
  - `unsigned int x = 4294967295U; return x > 1U;` -> no-fold/fold exit 1.
- Dead integer divide-by-zero branch:
  - `if (0) return 1 / 0; return 7;` -> no-fold/fold exit 7.

These reduce risk in the requested areas but do not offset the NaN fixed-point hang.

## Blockers

1. Fix the NaN/floating fixed-point termination bug and add a regression that fails on the current implementation. A minimal test/repro is shown above.
2. Address the new oversized `src/ir/constant_folding.rs` module or provide an explicit, accepted exception; under the loaded programming criteria this remains a high-severity maintainability defect.
3. Remove the dead `shift_u32` helper while touching the evaluator.

Final status: REQUIRES ATTENTION
