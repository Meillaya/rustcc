# Task 59 Spill/Reallocation Loop Code Review

Verdict: **APPROVE**
Recommendation: **APPROVE**
Code quality status: **WATCH** (non-blocking size/test-harden risks only)
Review date: 2026-07-09
Reviewer role: code quality reviewer (read-only except this artifact)

## Scope inspected

Task: W21-T4 Chapter 20 spilling + re-allocation loop.

Uncommitted production diff inspected:
- `src/codegen/regalloc/allocate.rs`
- `src/codegen/regalloc/mod.rs`
- `src/codegen/regalloc/types.rs`
- `src/codegen/regalloc/spill.rs` (new, untracked)

Evidence / reference inputs inspected:
- `.omo/evidence/task-59-spill-loop-implementation.txt`
- `.omo/evidence/task-59-spill-loop-probe.c`
- `.omo/evidence/task-59-spill-loop-probe.s`
- `.omo/plans/c-compiler-rust.md` task 59
- `docs/book/ch20-register-allocation.md`
- `docs/stages/ch20-register-allocation.md`
- `nqcc2/lib/backend/regalloc.ml`
- `nqcc2/lib/backend/replace_pseudos.ml`
- Required regalloc support files: `graph.rs`, `color.rs`, `rewrite.rs`, `scratch.rs`, `operands.rs`, plus current Rust `replace_pseudos.rs` / `fixup.rs` for stack-spill handoff.

## Skill-perspective check

Required skill-perspective check **ran**:
- Loaded and applied `omo:remove-ai-slops` criteria: overfit/slop review, deletion-only/tautological tests, needless production extraction/parsing/normalization, oversized modules, dead code, excessive complexity.
- Loaded and applied `omo:programming` criteria plus `references/rust/README.md`: strict Rust hygiene, no `unwrap`/`expect`/`unsafe`, type/variant discipline, no needless abstraction, 250 pure-LOC ceiling.

Result: no blocking violation of either skill perspective. One non-blocking WATCH item: `allocate.rs` is in the 200-250 pure-LOC warning band.

## Findings by severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

None.

### LOW

1. `src/codegen/regalloc/allocate.rs` is in the Rust hygiene warning band at 239 pure LOC.
   - Evidence: `awk` pure LOC command reported `src/codegen/regalloc/allocate.rs 239`.
   - Impact: not a Task59 blocker because it remains below the hard 250 pure-LOC ceiling and the file still owns one coherent concept: per-function register allocation orchestration. Future additions should split before crossing 250.

2. The high-pressure spill probe is evidence-only, not an official harness test.
   - Evidence: `.omo/evidence/task-59-spill-loop-probe.c` and regenerated `/tmp` probe passed with exit `16` and 40 stack references.
   - Impact: non-blocking because the task explicitly required `git diff -- tests` to remain empty, and official chapter gates were run. Keep this in mind if Task60 changes allocator behavior.

## Required checks

### Official tests / harness unchanged

PASS.

Commands inspected/run:
- `git diff -- tests` → empty
- `git diff --cached -- tests` → empty
- `git diff HEAD -- tests` → empty
- `git status --short -- tests` → empty

No official tests or test harness changes were present.

### No production test-name/source-path bridge

PASS.

Command:
- `rg -n 'source_path_hint|chapter_20|test(_|-)?name|test_compiler' src || true` → no matches

No `source_path_hint`, `chapter_20`, test-name, or test harness bridge is present in production source.

### Reserved registers remain non-allocatable

PASS.

Evidence:
- `src/codegen/regalloc/types.rs:60-74` GP allocatable set is `AX, BX, CX, DX, DI, SI, R8, R9, R12, R13, R14, R15`; it excludes `R10` and `R11`.
- `src/codegen/regalloc/types.rs:78-90` GP caller-saved set excludes `R10` and `R11`; XMM caller-saved uses `0..=13`.
- `src/codegen/regalloc/types.rs:93-110` `contains` excludes `R10`, `R11`, `XMM14`, and `XMM15` by construction.
- `src/codegen/regalloc/scratch.rs:33,41` reserves `R11` only for address scratch rewriting.
- `src/codegen/replace_pseudos.rs:141-145,320-327,355-366` uses `XMM15` as a fixup scratch, outside the allocator range.

### Spill/reallocation loop correctness

PASS.

Key source evidence:
- `src/codegen/regalloc/allocate.rs:74-100` creates a `SpillState`, computes `max_reallocation_passes`, reruns select/build/color while newly uncolored pseudos are discovered, and only returns once a pass adds zero new spills.
- `src/codegen/regalloc/allocate.rs:102-125` rebuilds liveness/interference each pass with `aliased_pseudos` set to the current spill set, so known spilled/stack-only pseudos are forced out of the graph instead of being recolored.
- `src/codegen/regalloc/spill.rs:30-40` makes progress monotonic: `add_coloring_spills` only counts newly inserted pseudo names, and `max_reallocation_passes` is `all_pseudos + 1`. Since every continuing pass must add at least one pseudo from the finite instruction operand set, the loop is bounded and not prematurely small.
- `src/codegen/regalloc/spill.rs:42-68` carries forward the existing Task58 stack-only treatment for `Lea` sources and `PseudoMem` operands.
- `src/codegen/regalloc/spill.rs:70-89` only turns uncolored `Operand::Pseudo` assignments into spill decisions and bounds the pass count by all pseudo/PseudoMem names in the input.

Assessment:
- No hidden infinite loop: continuing requires at least one new spill, and the number of possible names is finite.
- Not prematurely failing: `N + 1` passes is sufficient for at most `N` newly discovered spills plus one final zero-new-spill fixed-point pass.
- Stack-only/spilled pseudos are handled consistently with the current Rust pipeline: uncolored pseudos are left as `Pseudo`, current `replace_pseudos` assigns stable stack slots (`src/codegen/replace_pseudos.rs:34-70`, `471-516`), and current fixup splits illegal memory forms after pseudo replacement.
- GP/XMM assignments are not lost: `src/codegen/regalloc/allocate.rs:50-64` runs GP first, feeds GP-rewritten instructions into XMM, then preserves GP callee-saved registers. `rewrite.rs` maps only class-relevant operands, so the XMM pass does not erase GP register assignments.

### OCaml comparison

PASS.

Relevant OCaml evidence:
- `nqcc2/lib/backend/regalloc.ml:597-604` colors once, builds a register map for colored pseudos, and leaves uncolored pseudos for pseudo replacement.
- `nqcc2/lib/backend/regalloc.ml:607-635` uses the same allocatable hardreg ranges: GP excludes `R10/R11`, XMM uses `XMM0..XMM13`.
- `nqcc2/lib/backend/regalloc.ml:639-643` runs GP then XMM.
- `nqcc2/lib/backend/replace_pseudos.ml:24-49` assigns stack slots for remaining `Pseudo`/`PseudoMem` operands.

Rust divergence is justified and safe for this pipeline:
- Rust adds a bounded fixed-point loop that treats newly uncolored pseudos as aliased/stack-only for the next interference build. This is stricter than OCaml's single color pass but preserves the same final handoff: colored pseudos become registers, uncolored pseudos remain for stack replacement.
- Rust does not insert explicit reload/store instructions during allocation. That is acceptable here because the existing Rust `replace_pseudos` + `fixup` pipeline already supports stack operands and scratch-register repair for illegal memory forms.

### File-size / hygiene

PASS with WATCH.

Commands:
- `rg -n 'unwrap\(|expect\(|unsafe' src/codegen/regalloc/allocate.rs src/codegen/regalloc/mod.rs src/codegen/regalloc/types.rs src/codegen/regalloc/spill.rs || true` → no matches
- `git diff -- Cargo.toml Cargo.lock` → empty
- Pure LOC:
  - `src/codegen/regalloc/allocate.rs` → 239 (WATCH band, below 250)
  - `src/codegen/regalloc/mod.rs` → 61
  - `src/codegen/regalloc/types.rs` → 99
  - `src/codegen/regalloc/spill.rs` → 71

No new dependency, `unsafe`, `unwrap`, or `expect` was introduced in the Task59 source diff.

## Commands run / evidence

- `git status --short`
- `git diff --stat`
- `git diff -- src/codegen/regalloc/allocate.rs src/codegen/regalloc/mod.rs src/codegen/regalloc/types.rs src/codegen/regalloc/spill.rs`
- `git diff -- tests` → empty
- `git diff --cached -- tests` → empty
- `git diff HEAD -- tests` → empty
- `git status --short -- tests` → empty
- `rg -n 'source_path_hint|chapter_20|test(_|-)?name|test_compiler' src || true` → no matches
- `rg -n 'unwrap\(|expect\(|unsafe' ...changed regalloc files... || true` → no matches
- `git diff -- Cargo.toml Cargo.lock` → empty
- `git diff --check` → pass
- `cargo fmt --all -- --check` → pass
- `cargo check --release` → pass
- `cargo test --release` → pass; 10 binary tests, 0 lib/doc tests
- `./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing` → 66 tests OK
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only` → 120 tests OK
- `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union` → 286 tests OK; assembler truncation warnings match existing evidence context
- `/tmp` manual high-pressure probe from `.omo/evidence/task-59-spill-loop-probe.c`:
  - `./target/release/rustcc --no-coalescing -S /tmp/.../probe.c` → pass
  - `gcc /tmp/.../probe.s -o /tmp/.../probe` → pass
  - executable exit code → `16` (expected `528 mod 256`)
  - stack references → `40`
  - reserved-register scan in generated probe output for `%r10|%r11|%xmm14|%xmm15` → no matches
- `cargo clippy --all-targets --all-features` → exit 0 with existing warnings outside Task59-changed files; no clippy warning in the changed regalloc files.

## Risks / notes

- `src/codegen/regalloc/allocate.rs` is close to the 250 pure-LOC ceiling; future Task60 coalescing work should avoid adding to this file without splitting responsibilities.
- The Task59 source leaves official tests unchanged by requirement. The manual probe provides spill-specific evidence, but a future harness-approved regression would be useful if test policy changes.
- Several unrelated `.omo/evidence/task-18..task-41` files and `.omo/start-work/ledger.jsonl` are untracked; not production-code blockers for Task59.

## Blockers

None.

## Final verdict

**APPROVE** — the spill/reallocation loop is bounded, scope-faithful, compatible with the existing `replace_pseudos`/fixup stack-slot pipeline, preserves Task58 reserved-register and GP/XMM invariants, and passes the required gates without official test/harness changes.
