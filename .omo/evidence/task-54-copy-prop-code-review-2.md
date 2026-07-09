# Task 54 / W20-T4 Copy Propagation Code Review 2

VERDICT: PASS

codeQualityStatus: WATCH
recommendation: APPROVE
reportPath: `.omo/evidence/task-54-copy-prop-code-review-2.md`
blockers: None.

## Scope reviewed

Goal: Read-only re-review of `/home/mei/projects/rustcc` Task 54 / W20-T4 copy propagation after the file-size/slop fix. Decide whether previous rejection blockers are resolved and whether the task can proceed to adversarial gate.

Current HEAD inspected: `bd4aad2` (`feat(compiler): chapter 19: unreachable code elimination`).

Untrusted artifacts inspected directly:
- `.omo/evidence/task-54-copy-prop-code-review.md` — previous rejection.
- `.omo/evidence/task-54-copy-prop-fix.txt` — claimed file-size/slop fix evidence.
- `.omo/evidence/task-54-copy-prop-implementation.txt` — implementation evidence, used only for scope consistency.

Changed source files inspected, including untracked helper modules:
- `src/codegen/codegen.rs`
- `src/codegen/codegen/copy_prop_support.rs`
- `src/ir/constant_folding.rs`
- `src/ir/constant_folding/folds.rs`
- `src/ir/constant_folding/instr.rs`
- `src/ir/constant_folding/state.rs`
- `src/ir/copy_propagation.rs`
- `src/ir/copy_propagation/cleanup.rs`
- `src/ir/copy_propagation/dataflow.rs`
- `src/ir/copy_propagation/facts.rs`
- `src/ir/copy_propagation/rewrite.rs`
- `src/ir/mod.rs`
- `src/ir/opt.rs`
- `src/pipeline.rs`

## Skill-perspective check

Ran/consulted required skill perspectives before judging maintainability and tests:
- `omo:remove-ai-slops`: loaded `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md` completely. Applied its oversized-module, overfit-test, needless-abstraction, over-defensive, and behavior-coverage checks.
- `omo:programming`: loaded `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md` completely and loaded the Rust reference `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md`. Applied Rust-specific checks for the 250 pure-LOC ceiling, unsafe/unwrap/expect, type/phase scope, and test shape.

Skill-perspective result: no approval-blocking violation remains. The branch still touches legacy oversized `src/codegen/codegen.rs`, but Task 54 helper bodies were relocated out of it, the file's pure LOC is below the reviewed HEAD baseline, and the remaining hunks are narrow delegations rather than broad helper bodies. Low-risk watch notes remain below.

## Previous rejection blockers

### Blocker 1: `src/ir/constant_folding/instr.rs` over 250 pure LOC

Resolved.

Evidence command:

```bash
python3 - <<'PY'
from pathlib import Path
files = [
'src/ir/constant_folding/instr.rs',
'src/ir/constant_folding/folds.rs',
'src/ir/constant_folding/state.rs',
'src/codegen/codegen.rs',
'src/codegen/codegen/copy_prop_support.rs',
'src/ir/copy_propagation.rs',
'src/ir/copy_propagation/facts.rs',
'src/ir/copy_propagation/dataflow.rs',
'src/ir/copy_propagation/rewrite.rs',
'src/ir/copy_propagation/cleanup.rs',
'src/ir/constant_folding.rs',
'src/ir/mod.rs',
'src/ir/opt.rs',
'src/pipeline.rs',
]
for f in files:
    p=Path(f)
    pure=sum(1 for line in p.read_text().splitlines() if line.strip() and not line.strip().startswith('//'))
    print(f'{pure:4d} {f}')
PY
```

Output:

```text
 234 src/ir/constant_folding/instr.rs
 166 src/ir/constant_folding/folds.rs
  21 src/ir/constant_folding/state.rs
1908 src/codegen/codegen.rs
  98 src/codegen/codegen/copy_prop_support.rs
  73 src/ir/copy_propagation.rs
 179 src/ir/copy_propagation/facts.rs
 204 src/ir/copy_propagation/dataflow.rs
 244 src/ir/copy_propagation/rewrite.rs
 110 src/ir/copy_propagation/cleanup.rs
  78 src/ir/constant_folding.rs
  13 src/ir/mod.rs
  41 src/ir/opt.rs
  92 src/pipeline.rs
```

`src/ir/constant_folding/instr.rs` is now 234 pure LOC, below the 250 hard ceiling. Static-variable constant-state handling is split into `src/ir/constant_folding/state.rs` lines 5-25 and wired through `src/ir/constant_folding.rs` lines 13-15 and 25-82.

### Blocker 2: Task 54 helper logic added to oversized `src/codegen/codegen.rs`

Resolved for this task-specific gate.

Evidence:
- New helper module: `src/codegen/codegen/copy_prop_support.rs` is 98 pure LOC and contains the call-argument reuse and constant-index `AddPtr` support (`move_reused_int_arg` lines 21-49, `move_call_arg` lines 51-80, `lower_const_index_addptr` lines 82-108).
- Parent module path is normal Rust module layout for a file module: `src/codegen/codegen.rs` declares `mod copy_prop_support;` at line 50, and `cargo check --release` passed, so there is no hidden path hack.
- Remaining Task 54 codegen hunks in `src/codegen/codegen.rs` are narrow delegations:
  - line 50: `mod copy_prop_support;`
  - lines 448-465: delegate integer call-arg move selection to `copy_prop_support`.
  - lines 1844-1847: delegate constant-index `AddPtr` lowering to `copy_prop_support`.
- `src/codegen/codegen.rs` remains a legacy oversized file at 1908 pure LOC, but the Task 54 fix reduced it below the reviewed HEAD baseline (`1911` in my measurement, `1910` in the previous review's measurement) and no longer leaves the new helper bodies there.

Assessment: the remaining codegen hunks are not broad/sloppy. They are small call sites necessary to route existing codegen flow to the relocated helper module. This does not block adversarial gate.

## Exact commands inspected/run

Read/inspection commands:

```bash
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md
cat .omo/evidence/task-54-copy-prop-code-review.md
cat .omo/evidence/task-54-copy-prop-fix.txt
cat .omo/evidence/task-54-copy-prop-implementation.txt
git status --short
git rev-parse --short HEAD
git log -1 --oneline
(git diff --name-only -- src tests Cargo.toml Cargo.lock; git ls-files --others --exclude-standard src tests Cargo.toml Cargo.lock) | sort -u
git diff --unified=35 -- src/codegen/codegen.rs src/ir/constant_folding.rs src/ir/constant_folding/folds.rs src/ir/constant_folding/instr.rs src/ir/mod.rs src/ir/opt.rs src/pipeline.rs
nl -ba src/codegen/codegen/copy_prop_support.rs
nl -ba src/ir/constant_folding/state.rs
nl -ba src/ir/copy_propagation.rs
nl -ba src/ir/copy_propagation/facts.rs
nl -ba src/ir/copy_propagation/dataflow.rs
nl -ba src/ir/copy_propagation/rewrite.rs
nl -ba src/ir/copy_propagation/cleanup.rs
nl -ba src/codegen/codegen.rs | sed -n '45,55p;395,465p;1830,1895p'
nl -ba src/ir/constant_folding.rs | sed -n '1,95p'
nl -ba src/ir/constant_folding/instr.rs | sed -n '35,65p;120,180p'
nl -ba src/ir/opt.rs | sed -n '20,60p'
nl -ba src/pipeline.rs | sed -n '65,95p'
```

Required verification commands rerun:

```bash
cargo fmt --all -- --check
cargo check --release
git diff --check
cargo test --release
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --propagate-copies
```

Additional policy scans:

```bash
git diff -U0 -- src | rg -n '\b(DeadStoreElim|dead[-_ ]?store|DSE|liveness|live[-_ ]?|regalloc|register allocation|coalesc|coalesce)\b' || true
git diff -U0 -- src | rg -n '#\[cfg\(test\)|#\[test\]|mod tests' || true
git diff -U0 -- src | rg -n '\bunsafe\b|\bunwrap\(|\bexpect\(' || true
rg -n 'evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_|bridge|fingerprint' src || true
git diff -- tests Cargo.toml Cargo.lock
```

## Verification results

- `cargo fmt --all -- --check`: PASS, exit 0.
- `cargo check --release`: PASS, exit 0 (`Finished release profile ... 0.02s`).
- `git diff --check`: PASS, exit 0.
- `cargo test --release`: PASS, exit 0. `src/main.rs` ran the expected 10 tests, all passed; `src/lib.rs` and doc-tests ran 0 tests.
- `cargo build --release`: PASS, exit 0. Run to ensure `./target/release/rustcc` was current before the harness gate.
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --propagate-copies`: PASS, exit 0; 42 tests ran, `OK`.

Policy scan results:
- No compiler-phase Rust unit tests added: diff-only scan for `#[cfg(test)]`, `#[test]`, and `mod tests` returned no matches.
- No unsafe/unwrap/expect introduced in changed Rust diff: diff-only scan returned no matches.
- No harness weakening or dependency changes: `git diff -- tests Cargo.toml Cargo.lock` returned empty output.
- No bridge/interpreter fingerprints in `src`: fingerprint scan returned no matches.
- No regalloc/liveness/coalescing implementation added. Diff-only forbidden-term scan only found the existing `DeadStoreElim` enum/no-op wiring and comment in `src/ir/opt.rs`; `OptPass::DeadStoreElim => current` remains at `src/ir/opt.rs:56`.

## Findings by severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

None.

### LOW

1. **Legacy `src/codegen/codegen.rs` is still oversized, but Task 54 no longer worsens it.**
   - Evidence: `src/codegen/codegen.rs` is 1908 pure LOC. Remaining Task 54 lines are `src/codegen/codegen.rs:50`, `src/codegen/codegen.rs:448-465`, and `src/codegen/codegen.rs:1844-1847`.
   - Assessment: not a blocker for this re-review because the helper bodies moved to `src/codegen/codegen/copy_prop_support.rs` (98 pure LOC) and `codegen.rs` net pure LOC is below HEAD. Future codegen work should still decompose the legacy file rather than add more logic.

2. **`src/ir/copy_propagation/rewrite.rs` is close to the warning band ceiling.**
   - Evidence: measured at 244 pure LOC.
   - Assessment: below the 250 hard ceiling, but future edits should split source-rewrite responsibilities before adding logic.

3. **Minor over-defensive fallback in `move_reused_int_arg`.**
   - File: `src/codegen/codegen/copy_prop_support.rs:30-35`.
   - Evidence: the source register position is derived from a slot already selected out of `ctx.classified.int_slots[..idx]`, so `.position(...)` over `ctx.classified.int_slots` should always succeed; `.unwrap_or(0)` would silently select the first integer register if the invariant were broken.
   - Assessment: LOW maintainability note, not a gate blocker. Minimal remediation when this helper is next touched: derive the previous slot index with `enumerate()` during the initial search and remove the fallback.

## Semantic/scope audit notes

- Copy propagation is gated by `OptimizationFlags.propagate_copies`: `src/pipeline.rs:81-90` pushes `OptPass::CopyPropagation` only under that flag, and `src/ir/opt.rs:51-55` dispatches only that pass implementation.
- DSE remains unwired: `src/ir/opt.rs:56` keeps `OptPass::DeadStoreElim => current`.
- The narrow cleanup in `src/ir/copy_propagation/cleanup.rs:7-24` removes unused aggregate scaffolding exposed by copy propagation, and `src/ir/copy_propagation/cleanup.rs:45-54` limits `CopyBytes` removal to writes into unused `tmp.*` aggregate temps. This is still broader than scalar-only copy replacement, but it is under the copy-prop module and does not wire DSE/regalloc/liveness/coalescing.
- No test harness, `Cargo.toml`, or `Cargo.lock` changes were present.
- No compiler-phase Rust unit tests were added. This matches the Task 54 plan constraint; behavior is covered here by official harness gates rather than implementation-mirroring unit tests.
- No deletion-only, tautological, implementation-mirroring, or prompt-style brittle tests were added because no tests were added or weakened.

## Final judgment

The previous rejection blockers are resolved:
- `src/ir/constant_folding/instr.rs` is below 250 pure LOC.
- Task 54 codegen helper bodies are relocated into `src/codegen/codegen/copy_prop_support.rs`; remaining `codegen.rs` hunks are narrow delegations and are not broad/sloppy.
- Required formatting, check, diff whitespace, Rust tests, current release build, and official Chapter 19 copy-prop harness gate all pass.
- Scope-control checks show no DSE/regalloc/liveness/coalescing implementation, no compiler-phase Rust unit tests, no harness weakening, no dependency changes, and no bridge fingerprints.

Final Status: PASS. Task 54 can proceed to adversarial gate.
