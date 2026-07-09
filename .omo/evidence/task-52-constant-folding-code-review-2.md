# Task 52 / W20-T2 Constant Folding Code Review 2

VERDICT: PASS

codeQualityStatus: CLEAR
recommendation: APPROVE
reportPath: `.omo/evidence/task-52-constant-folding-code-review-2.md`

## Scope reviewed

Read-only re-review of Task 52 constant folding after executor fixes. The prior rejection and fix evidence were treated as untrusted until inspected:

- Previous rejection: `.omo/evidence/task-52-constant-folding-code-review.md`
- Fix evidence: `.omo/evidence/task-52-constant-folding-fix.txt`
- Original implementation evidence: `.omo/evidence/task-52-constant-folding-implementation.txt`

Changed source files inspected:

- `src/ir/const_eval.rs`
- `src/ir/constant_folding.rs`
- `src/ir/constant_folding/folds.rs`
- `src/ir/constant_folding/instr.rs`
- `src/ir/constant_folding/util.rs`
- `src/ir/opt.rs`
- `src/pipeline.rs`
- `src/ir/mod.rs`

## Skill-perspective check

Required review perspectives were loaded/consulted before judging test relevance and maintainability:

- `omo:remove-ai-slops`: loaded `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md`. Applied the slop/overfit pass to production code and tests. No tests were changed, weakened, deleted, or added as tautological implementation mirrors. The previous production slop blockers are resolved: the oversized module was split and the dead `shift_u32` helper is gone.
- `omo:programming`: loaded `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md` and Rust reference `references/rust/README.md`. The Rust 250 pure-LOC ceiling now passes for every touched Rust file; no `unsafe`, new dependency, `unwrap()`, or `expect()` was found in the changed files. Remaining raw `as` casts in `src/ir/const_eval.rs` are localized C constant-conversion semantics and were covered by targeted cast/comparison probes in this review.

Verdict on skill perspectives: no blocking violation of either skill perspective remains.

## Commands inspected/run

Exact commands run from `/home/mei/projects/rustcc` unless otherwise noted:

```bash
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md

git status --short
sed -n '1,240p' .omo/evidence/task-52-constant-folding-code-review.md
sed -n '1,240p' .omo/evidence/task-52-constant-folding-fix.txt
git diff --stat
git diff --name-status
git diff -- src/ir/mod.rs src/ir/opt.rs src/pipeline.rs
for f in src/ir/const_eval.rs src/ir/constant_folding.rs src/ir/constant_folding/folds.rs src/ir/constant_folding/instr.rs src/ir/constant_folding/util.rs; do git diff --no-index -- /dev/null "$f" || true; done
sed -n '1,260p' .omo/evidence/task-52-constant-folding-implementation.txt
sed -n '1,220p' nqcc2/lib/optimizations/constant_folding.ml
sed -n '1,120p' nqcc2/lib/optimizations/optimize.ml

for f in src/ir/const_eval.rs src/ir/constant_folding.rs src/ir/constant_folding/folds.rs src/ir/constant_folding/instr.rs src/ir/constant_folding/util.rs src/ir/opt.rs src/pipeline.rs src/ir/mod.rs; do printf '%s ' "$f"; awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(#|\/\/)/' "$f" | wc -l; done
rg -n '\bshift_u32\b' src/ir/const_eval.rs src/ir/constant_folding.rs src/ir/constant_folding || true
git status --short Cargo.toml Cargo.lock tests || true
git diff -- Cargo.toml Cargo.lock tests || true
rg -n 'evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_' src tests Cargo.toml Cargo.lock || true
rg -n 'UnreachableCodeElim|CopyPropagation|DeadStoreElim|regalloc|register allocation|RegisterAlloc|uce|dse' src/ir src/pipeline.rs src/driver.rs || true
rg -n '\bunsafe\b|\.unwrap\(|\.expect\(|#\[allow|\sas\s' src/ir/const_eval.rs src/ir/constant_folding.rs src/ir/constant_folding/folds.rs src/ir/constant_folding/instr.rs src/ir/constant_folding/util.rs src/ir/opt.rs src/pipeline.rs src/ir/mod.rs || true

cargo fmt --all -- --check
git diff --check
cargo check --release
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants

# NaN repro with timeout, using /tmp/task52_nan_review2.c
timeout 5s ./target/release/rustcc --fold-constants -S /tmp/task52_nan_review2.c

# Targeted semantic probes under /tmp/task52_*_review2.c, each compiled/run with rustcc no-fold, rustcc --fold-constants, and gcc:
# branch_zero, branch_nonzero, nan_condition, dead_divzero, dead_remzero,
# uchar_cast, double_to_int, unsigned_cmp, signed_unsigned_cmp, double_cmp,
# store_alias, call_alias, block_reassign_true, block_reassign_false,
# block_merge_runtime, loop_basic
```

I also made one invalid exploratory NaN command with `--output`; `rustcc` rejected it as `unknown flag: --output`. I reran the NaN repro with the compiler's supported default `.s` output behavior and used the passing supported-command result for this verdict.

## Required gate results

- `cargo fmt --all -- --check`: PASS (`cargo_fmt_status=0`).
- `cargo check --release`: PASS (`Finished release profile [optimized] target(s) in 0.03s`).
- `cargo build --release`: PASS (`Finished release profile [optimized] target(s) in 0.01s`), run to ensure `./target/release/rustcc` matched current sources before harness/repro verification.
- `git diff --check`: PASS (`git_diff_check_status=0`). Note: this checks tracked diffs; I also ran `git diff --no-index --check -- /dev/null <new-file>` on every new Task 52 Rust file and saw no whitespace-error output.
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants`: PASS, `Ran 16 tests ... OK`.

## Previous rejection blockers

### 1. NaN fixed-point hang

Resolved.

Repro source:

```c
int main(void) {
    double x = 0.0 / 0.0;
    return x != x;
}
```

Observed supported-command results:

```text
nofold_exit=1
fold_compile_status=0
fold_exit=1
gcc_exit=1
nan_repro_status=PASS
```

Implementation evidence:

- `src/ir/opt.rs:34-50` now terminates on pass-level `changed` flags, not whole-program `PartialEq`, so NaN `PartialEq` no longer controls fixed-point convergence.
- `src/ir/constant_folding.rs:24-45` returns `PassResult { program, changed }`, and `src/ir/constant_folding.rs:61-80` ORs instruction-level changes while preserving function metadata and replacing only the reassembled body.
- `src/ir/constant_folding/util.rs:70-76` compares double constants with `to_bits()` for copy-change detection, so a stable NaN constant copy is not reported changed forever.

### 2. Constant-folding module size

Resolved.

Pure LOC counts measured by `awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(#|\/\/)/' <file> | wc -l`:

```text
src/ir/const_eval.rs 221
src/ir/constant_folding.rs 63
src/ir/constant_folding/folds.rs 159
src/ir/constant_folding/instr.rs 225
src/ir/constant_folding/util.rs 70
src/ir/opt.rs 29
src/pipeline.rs 86
src/ir/mod.rs 10
```

All touched Rust files, including `src/ir/constant_folding.rs` and the split submodules, are below the 250 pure-LOC ceiling.

### 3. Dead `shift_u32` helper

Resolved.

`rg -n '\bshift_u32\b' src/ir/const_eval.rs src/ir/constant_folding.rs src/ir/constant_folding || true` produced no matches.

## Semantic risk re-check

Targeted probes all matched rustcc no-fold, rustcc `--fold-constants`, and GCC exits:

```text
branch_zero:          nofold=2  fold=2  gcc=2  PASS
branch_nonzero:       nofold=1  fold=1  gcc=1  PASS
nan_condition:        nofold=11 fold=11 gcc=11 PASS
dead_divzero:         nofold=7  fold=7  gcc=7  PASS
dead_remzero:         nofold=8  fold=8  gcc=8  PASS
uchar_cast:           nofold=44 fold=44 gcc=44 PASS
double_to_int:        nofold=3  fold=3  gcc=3  PASS
unsigned_cmp:         nofold=1  fold=1  gcc=1  PASS
signed_unsigned_cmp:  nofold=1  fold=1  gcc=1  PASS
double_cmp:           nofold=1  fold=1  gcc=1  PASS
store_alias:          nofold=7  fold=7  gcc=7  PASS
call_alias:           nofold=7  fold=7  gcc=7  PASS
block_reassign_true:  nofold=9  fold=9  gcc=9  PASS
block_reassign_false: nofold=2  fold=2  gcc=2  PASS
block_merge_runtime:  nofold=9  fold=9  gcc=9  PASS
loop_basic:           nofold=3  fold=3  gcc=3  PASS
```

Code audit notes:

- Fixed-point convergence is now pass-level changed-flag based (`src/ir/opt.rs:34-50`), removing the NaN equality hang from Review 1.
- Division/remainder by zero and overflow use `checked_*().unwrap_or(0)` in `src/ir/const_eval.rs:128-185`, matching the reference intent to avoid optimizer crashes for UB/dead code; dead div/rem probes passed.
- Double comparisons explicitly model unordered/NaN behavior in `src/ir/const_eval.rs:215-225`; NaN comparison and NaN condition probes passed.
- Branch folding only rewrites constant conditional jumps in `src/ir/constant_folding/instr.rs:141-160`; branch and loop probes passed.
- Constants are local per CFG block (`src/ir/constant_folding.rs:61-75`) and are cleared/invalidated for memory and call effects in `src/ir/constant_folding/instr.rs:161-202`; store/call/block probes passed.

## Scope-control checks

- No UCE/copy-prop/DSE implementation added. `src/ir/opt.rs:43-45` leaves `UnreachableCodeElim`, `CopyPropagation`, and `DeadStoreElim` as no-ops.
- No regalloc implementation added by this task. `regalloc` scan hits were pre-existing driver/module comments and fields, not Task 52 code.
- No test harness weakening: `git diff -- Cargo.toml Cargo.lock tests` was empty; `git status --short Cargo.toml Cargo.lock tests` was empty.
- No new dependencies: `Cargo.toml` and `Cargo.lock` unchanged.
- Exact forbidden bridge/interpreter fingerprint scan over `src tests Cargo.toml Cargo.lock` found no matches for `evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_`. A broader earlier scan saw only the pre-existing explanatory comment `src/ir/mod.rs:15` saying there is no runtime interpreter, which is not a forbidden bridge implementation.

## Findings by severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

None.

### LOW

1. No durable in-repo regression test was added for the NaN fixed-point repro. This is not a blocker for this re-review/adversarial-gate handoff because the repro now passes with a timeout and the user-requested official gate passes, but the NaN case should be retained as an adversarial-gate case or promoted into the compiler tests later.

2. `.omo/boulder.json` remains modified in the working tree alongside Task 52 source changes. This is workflow state, not a code correctness issue, but should be handled intentionally before any commit/handoff that excludes state files.

## Blockers

None. The previous rejection blockers are resolved, and I found no new semantic blocker in the required risk areas.

Final status: PASS — proceed to adversarial gate.
