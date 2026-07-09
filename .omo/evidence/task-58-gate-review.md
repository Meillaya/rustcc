# Task 58 Final Gate Review

recommendation: REJECT
statusForUserRequestedPassFail: FAIL
reviewDate: 2026-07-09
workspace: /home/mei/projects/rustcc
mode: read-only verification of source/test/evidence; only this gate report artifact was written

## originalIntent

Verify the Task 58 final fix satisfies the user's explicit blockers:

- no compiler path bridge / no `source_path_hint` allocation decision
- exact local OCaml GP register parity excluding `R10`/`R11`
- durable probe mapping has no `R11`
- scoped `regalloc/rewrite.rs` `clippy::collapsible_if` finding is gone
- `./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing` is green
- no broad W21-T4/T5 implementation (no spill/reallocation loop, no conservative coalescing)

## desiredOutcome

The user can mark Task 58 complete only if the current source, current diff, required evidence logs, manual QA, code-review coverage, and direct anti-slop/programming review support completion from the user's perspective.

## userOutcomeReview

The explicit technical blockers are now satisfied by direct inspection and fresh reruns:

- `src/compiler.rs:40-58` has no `source_path_hint`; `src/compiler.rs:123-129` allocates only when `!options.regalloc_options.coalescing_enabled`; `src/driver.rs:148-155` no longer passes a source-path hint.
- `src/codegen/regalloc/types.rs:52-68`, `:72-84`, and `:87-105` match local OCaml GP/XMM register-class constraints: GP excludes `R10`/`R11`, caller-saved excludes `R10`/`R11`, XMM excludes `XMM14`/`XMM15`.
- `src/codegen/regalloc/color.rs:52-84` implements the OCaml min/max hard-register color policy; the durable probe output maps `{0:R9, 1:R8, 2:SI, 3:DI, 4:DX, 5:CX, 6:AX, 7:BX, 8:R12, 9:R13, 10:R14, 11:R15}` with no `R11`.
- `src/codegen/regalloc/rewrite.rs:16-23` has the collapsed `if let` chain; the scoped clippy rerun produced no `regalloc/rewrite.rs` hit.
- Fresh rerun of chapter 20 no-coalescing passed: `Ran 66 tests ... OK`.
- Source scan found no coalescing implementation and no iterative spill/reallocation loop in `src/codegen/regalloc`; default/coalescing-enabled compile path remains unallocated in `src/compiler.rs:124-129`.

However, final approval is blocked because the shipped artifact lacks a current, supported code-review report for the final diff. The available Task 58 code-review artifacts are stale relative to `.omo/evidence/task-58-final-fix.txt`: one earlier PASS reviews only the initial select/color slice before allocation wiring, and the later code review explicitly REJECTS the pre-final state for blockers that the final fix later changed. The final-gate contract requires a code-review report that explicitly covers the same `omo:remove-ai-slops` / `omo:programming` skill perspective and overfit/slop criteria for the shipped final diff. That artifact is absent.

## blockers

1. **Missing current final code-review coverage.** Existing reports do not approve the final fix:
   - `.omo/evidence/task-58-coloring-code-review.md` is an earlier PASS for the initial W21-T3 select/color slice; it explicitly scoped status to `src/codegen/regalloc/mod.rs` plus new `color.rs` and did not cover final files such as `src/compiler.rs`, `src/driver.rs`, `src/pipeline.rs`, `src/codegen/regalloc/allocate.rs`, `rewrite.rs`, `scratch.rs`, `tests/test_framework/runner.py`, or broader tracked diffs.
   - `.omo/evidence/task-58-coloring-code-review-2.md` is a REJECT report and predates the final fix that removed `source_path_hint` and restored no-`R11` parity.
   - No post-final-fix code-review artifact was found that explicitly re-applies the overfit/slop criteria to the final shipped diff.

2. **Scope/evidence gap from dirty worktree breadth.** Current `git status --short` includes modified tracked files outside the user's stated changed areas and outside the stale PASS review scope, including `src/codegen/codegen.rs`, `src/codegen/codegen/copy_prop_support.rs`, `src/codegen/emit.rs`, `src/codegen/fixup.rs`, `src/ir/copy_propagation/rewrite_support.rs`, `src/pipeline.rs`, and `tests/test_framework/runner.py`. Fresh tests are green, but final approval needs code-review/slop coverage over the actual shipped diff, not only the narrower regalloc blocker set.

3. **Notepad/current-original-brief artifact gap.** No current notepad path was supplied. `.omx/notepad.md` exists but contains stale May 2026 historical notes, not current Task 58 final-fix state. I did not use it as trusted completion evidence.

## direct remove-ai-slops / programming pass

Skills consulted before judgment:

- `omo:programming` SKILL.md and `references/rust/README.md` plus `references/code-smells.md`.
- `omo:remove-ai-slops` SKILL.md.

Direct findings:

- No deletion-only official test, tautological deletion test, or test that merely checks a requested removal was added for Task 58.
- The durable probe is implementation-level, but it is appropriate for the exact low-level OCaml color-map/no-`R11` parity requirement and is backed by the chapter 20 e2e gate.
- No new dependency, `unsafe`, `dbg!`, or `println!/eprintln!` debug output was found in the Task58 regalloc production files inspected.
- No conservative coalescing or iterative spill/reallocation loop was found.
- Warning-band/legacy size risks remain in several dirty files, but the approval blocker is the absent current code-review/slop report for the final diff.

## checked artifact paths

- `.omo/evidence/task-58-verify-run.log`
- `.omo/evidence/task-58-manual-qa.log`
- `.omo/evidence/task-58-hands-on-qa/manualQa.json`
- `.omo/evidence/task-58-hands-on-qa/manualqa-rerun.log`
- `.omo/evidence/task-58-final-fix.txt`
- `.omo/evidence/task-58-coloring-probe.rs`
- `.omo/evidence/task-58-coloring-code-review.md`
- `.omo/evidence/task-58-coloring-code-review-2.md`
- `.omo/evidence/task-58-coloring-gate-review.md`
- `.omo/evidence/task-58-coloring-adversarial-verify-2-gate-review.md`
- `.omo/plans/c-compiler-rust.md`
- `.omx/notepad.md` (stale; not trusted)
- `src/compiler.rs`
- `src/driver.rs`
- `src/pipeline.rs`
- `src/codegen/regalloc/types.rs`
- `src/codegen/regalloc/color.rs`
- `src/codegen/regalloc/allocate.rs`
- `src/codegen/regalloc/rewrite.rs`
- `src/codegen/regalloc/scratch.rs`
- `src/codegen/regalloc/graph.rs`
- `src/codegen/regalloc/mod.rs`
- `src/codegen/regalloc/operands.rs`
- `src/codegen/codegen.rs`
- `src/codegen/codegen/copy_prop_support.rs`
- `src/codegen/emit.rs`
- `src/codegen/fixup.rs`
- `src/ir/copy_propagation/rewrite_support.rs`
- `tests/test_framework/runner.py`
- `nqcc2/lib/backend/regalloc.ml`
- `nqcc2/lib/assembly.ml`

## fresh verification evidence

Fresh reruns from this gate:

- `cargo fmt --all -- --check` -> exit 0
- `cargo check --release` -> exit 0
- `cargo build --release` -> exit 0
- `cargo test --release` -> exit 0; 10 `src/main.rs` tests passed
- `./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing` -> exit 0; `Ran 66 tests ... OK`
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only` -> exit 0; `Ran 120 tests ... OK`
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores` -> exit 0; `Ran 27 tests ... OK`
- `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union` -> exit 0; `Ran 286 tests ... OK` with the same non-fatal assembler truncation warnings recorded in prior evidence
- `rustc .omo/evidence/task-58-coloring-probe.rs -o /tmp/task58-gate-probe && /tmp/task58-gate-probe` -> exit 0; output contains no `R11` in `ocaml_color_mapping` or reserved GP list
- Scoped clippy command `cargo clippy --release --bin rustcc -- -A warnings -W clippy::collapsible_if` plus `rg` check for `regalloc/rewrite.rs` -> exit 0
- `python3 -m py_compile tests/test_framework/runner.py` -> exit 0
- `git diff --check` -> exit 0
- LSP diagnostics on `src/compiler.rs`, `src/driver.rs`, `src/pipeline.rs`, `src/codegen/codegen.rs`, `src/codegen/fixup.rs`, and the Task58 regalloc files -> no diagnostics found

## exact evidence gaps

- Missing current post-final-fix code-review report with explicit `omo:remove-ai-slops` and `omo:programming` coverage for the final diff.
- Existing Task 58 code-review reports are stale or rejecting; neither supports final approval of `.omo/evidence/task-58-final-fix.txt`.
- No current trusted notepad/current-original-brief artifact was supplied.
- Current dirty worktree includes broad files not covered by the stale PASS review; final approval needs review coverage for the actual diff being shipped.

Final verdict: FAIL / REJECT
