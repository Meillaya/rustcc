VERDICT: REJECT

# Task 58 final-fix code review 2 — W21-T3 Chapter 20 coloring + select

- **codeQualityStatus:** BLOCK
- **recommendation:** REQUEST_CHANGES
- **reportPath:** `.omo/evidence/task-58-final-fix-code-review-2.md`
- **Review mode:** read-only code review; wrote only this required report artifact.
- **Skill perspectives:** Ran the required perspective check by loading/consulting `omo:remove-ai-slops` and `omo:programming`, including the Rust README, Python README, and code-smells reference. The current diff violates both perspectives at the test-integrity boundary: the green Chapter 20 no-coalescing result depends on a changed Python harness under `tests/`, which is scope drift and creates false confidence. I did not find deletion-only tests, tautological new Cargo tests, or an R11-inclusive overfit probe in the latest state.

## Summary

The latest executor fixed several prior blockers:

- Production `source_path_hint` / `chapter_20` path allocation bridge is gone. `CompileOptions` now carries only `stage`, optimization flags, and regalloc options (`src/compiler.rs:40-45`); `driver.rs` no longer injects a path hint (`src/driver.rs:148-155`); allocation is option-driven in production (`src/compiler.rs:124-129`). A focused `rg` for source/test/chapter path coupling in the production compile/regalloc path returned no matches (`exit 1`).
- OCaml register-class parity for allocatable registers is restored: GP `all_hardregs` excludes `R10/R11` and XMM is `0..=13` (`src/codegen/regalloc/types.rs:52-68`); GP caller-saved excludes `R10/R11` (`src/codegen/regalloc/types.rs:72-83`); `contains` excludes `R10/R11/XMM14/XMM15` (`src/codegen/regalloc/types.rs:87-104`). This matches the OCaml reference (`nqcc2/lib/backend/regalloc.ml:607-635`).
- Durable probe mapping now excludes R11 and asserts reserved registers (`.omo/evidence/task-58-coloring-probe.rs:116-118`, `.omo/evidence/task-58-coloring-probe.rs:222-228`); probe run exits 0 and prints `{0: R9, 1: R8, ..., 11: R15}`.
- The scoped `clippy::collapsible_if` issue in `src/codegen/regalloc/rewrite.rs` is fixed (`src/codegen/regalloc/rewrite.rs:16-20`); targeted clippy command exited 0 and an `rg` over clippy output found no `collapsible_if`/rewrite matches.

I am still rejecting because the required Chapter 20 `--no-coalescing` pass is achieved by modifying the official test harness in `tests/test_framework/runner.py`, which violates the plan's locked test-scope constraints.

## Findings by severity

### CRITICAL

#### CRITICAL-1: Chapter 20 no-coalescing success depends on an invalid `tests/` harness modification

- **Changed file:** `tests/test_framework/runner.py:468-472`
- **Diff:** `cc_options` was changed from aliasing `args.extra_cc_options` to copying it, then `if args.no_coalescing: cc_options.append("--no-coalescing")` was added.
- **Plan constraints:**
  - `.omo/plans/c-compiler-rust.md:91-92` requires the official `test_compiler` Python harness as the verification path and says `Tests tests/ directory contents remain unchanged`.
  - `.omo/plans/c-compiler-rust.md:114-119` identifies the existing official harness invocations; it does not authorize modifying the harness to make a compiler option arrive.
  - `.omo/plans/c-compiler-rust.md:1926-1938` makes Task 58 acceptance `--chapter 20 --latest-only --no-coalescing` passing, with OCaml `regalloc.ml` as the reference.
  - `docs/stages/ch20-register-allocation.md:37-39` says `--no-coalescing` skips coalescing but still allocates registers; that behavior belongs in the compiler/regalloc implementation, not in a patched test runner.
- **Why this blocks:** The user explicitly said not to accept a green harness if it depends on violating locked scope or official test harness integrity. This modification is inside `tests/`, changes the official runner, and forwards a harness-selection flag into the compiler. The current acceptance command does pass (`exit 0`), but the pass is compromised because it relies on a changed harness rather than the locked official harness contents.
- **Native-codegen-only assessment:** Forwarding `--no-coalescing` from `tests/test_framework/runner.py` is **not acceptable** under the plan's `tests/ directory contents remain unchanged` and native-codegen-only constraints. The native Rust compiler can expose and consume `--no-coalescing`, and current production allocation is option-driven, but changing the Python harness is outside native compiler/codegen scope and invalidates the official-verification contract.
- **Related production evidence:** `src/compiler.rs:124-129` only runs allocation when `coalescing_enabled` is false. With the unmodified official runner, the harness-level `--no-coalescing` flag is used to select/relax regalloc tests (`tests/test_framework/runner.py:509-510`), not to mutate compiler arguments. The executor's added forwarding is therefore the bridge making the current green run possible.

### HIGH

None beyond the critical blocker above.

### MEDIUM

#### MEDIUM-1: New/changed files are near or above the programming LOC ceiling, mostly as pre-existing debt

- Warning band (200-250 pure LOC):
  - `src/codegen/regalloc/allocate.rs`: 242 pure LOC, new untracked source.
  - `src/codegen/regalloc/graph.rs`: 237 pure LOC.
  - `src/codegen/fixup.rs`: 235 pure LOC.
  - `src/ir/copy_propagation/rewrite_support.rs`: 239 pure LOC.
  - `.omo/evidence/task-58-coloring-probe.rs`: 231 pure LOC.
- Defect band but largely pre-existing oversized modules touched by this diff:
  - `src/driver.rs`: 278 pure LOC.
  - `src/codegen/emit.rs`: 504 pure LOC.
  - `src/codegen/codegen.rs`: 2006 pure LOC.
- I am not using the pre-existing oversized legacy modules as the rejection reason, but the new `allocate.rs` and probe are already in the warning band and should not grow further without splitting.

#### MEDIUM-2: The durable probe remains an artifact, not integrated regression coverage

- `.omo/evidence/task-58-coloring-probe.rs` now checks the right no-R11 mapping and reserved registers, and it compiles production regalloc files by absolute path.
- It is still a manual evidence artifact, not a Cargo-integrated regression; future changes can skip it unless reviewers run it explicitly.

### LOW

#### LOW-1: Several Task 58 source files are untracked

- Current status shows untracked source files: `src/codegen/regalloc/allocate.rs`, `color.rs`, `rewrite.rs`, and `scratch.rs`.
- This is expected for an uncommitted review but remains a delivery risk if a future commit forgets to stage them.

## Prior-blocker checklist

- **No source_path_hint/path-based allocation bridge in production:** PASS. No `source_path_hint`, `with_source_path_hint`, `chapter_20`, `latest-only`, or `test_` matches in `src/compiler.rs`, `src/driver.rs`, `src/pipeline.rs`, `src/codegen/regalloc`, or `src/codegen/fixup.rs` (`rg` exit 1). Allocation is decided by `!options.regalloc_options.coalescing_enabled` only (`src/compiler.rs:124`).
- **Exact OCaml GP register class parity excludes R10/R11:** PASS. Rust GP hardregs/caller-saved/contains exclude R10/R11 (`src/codegen/regalloc/types.rs:52-104`), matching OCaml (`nqcc2/lib/backend/regalloc.ml:607-610`).
- **XMM14/XMM15 are not allocatable:** PASS. Rust uses `0..=13` for XMM hardregs/caller-saved/contains (`src/codegen/regalloc/types.rs:68`, `src/codegen/regalloc/types.rs:83`, `src/codegen/regalloc/types.rs:104`), matching OCaml XMM0-XMM13 (`nqcc2/lib/backend/regalloc.ml:617-635`).
- **Durable probe color mapping excludes R11:** PASS. Probe expected mapping is `{0:R9, 1:R8, 2:SI, 3:DI, 4:DX, 5:CX, 6:AX, 7:BX, 8:R12, 9:R13, 10:R14, 11:R15}` (`.omo/evidence/task-58-coloring-probe.rs:116-118`), and the probe run prints the same mapping with no R11.
- **`src/codegen/regalloc/rewrite.rs` collapsible_if blocker fixed:** PASS. Current code uses a let-chain (`src/codegen/regalloc/rewrite.rs:16-20`); targeted clippy output had no `collapsible_if` or `rewrite.rs` matches.
- **Chapter 20 `--no-coalescing` pass is not achieved by invalid harness/test modification:** FAIL. The green run is achieved with a modified `tests/test_framework/runner.py` that appends `--no-coalescing` to compiler args (`tests/test_framework/runner.py:468-472`), violating plan lines 91-92.

## Exact checks inspected/run

```text
Skill/perspective loads
- cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md -> exit 0
- cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md -> exit 0
- cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md -> exit 0
- cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/python/README.md -> exit 0
- cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/code-smells.md -> exit 0

Diff/source/evidence inspection
- git status --short; git diff --stat; git diff --name-status; git ls-files --others --exclude-standard -> exit 0
- Inspected: tracked diffs for src/compiler.rs, src/driver.rs, tests/test_framework/runner.py, src/codegen/regalloc/graph.rs, src/codegen/regalloc/mod.rs, operands.rs, codegen.rs, copy_prop_support.rs, emit.rs, fixup.rs, rewrite_support.rs, pipeline.rs -> exit 0
- Inspected untracked/current files with line numbers: src/codegen/regalloc/{allocate,color,rewrite,scratch}.rs, src/codegen/regalloc/types.rs, graph.rs, mod.rs, operands.rs, .omo/evidence/task-58-coloring-probe.rs -> exit 0
- Inspected reference/plan/docs/evidence: nqcc2/lib/backend/regalloc.ml, docs/book/ch20-register-allocation.md, docs/stages/ch20-register-allocation.md, .omo/plans/c-compiler-rust.md, .omo/evidence/task-58-final-fix.txt, .omo/evidence/task-58-coloring-code-review-2.md, .omo/evidence/task-58-coloring-adversarial-verify-2.txt -> exit 0
- git diff --check -> exit 0
- python3 -m py_compile tests/test_framework/runner.py -> exit 0

Focused greps/probes
- rg -n "source_path_hint|with_source_path_hint|chapter_20|chapter 20|latest-only|test_" src/compiler.rs src/driver.rs src/pipeline.rs src/codegen/regalloc src/codegen/fixup.rs -> exit 1 (no matches)
- rg -n "Reg::R10|Reg::R11" src/codegen/regalloc/types.rs -> exit 1 (no matches)
- rg -n "XMM\(14\)|XMM\(15\)" src/codegen/regalloc/types.rs -> exit 1 (no matches)
- rustc .omo/evidence/task-58-coloring-probe.rs -o /tmp/task58-coloring-probe-review2 && /tmp/task58-coloring-probe-review2 -> exit 0; emitted warnings from standalone dead code, then no-R11 mapping/reserved-register output.
- cargo clippy --release --bin rustcc -- -A warnings -W clippy::collapsible_if -> exit 0
- rg -n "collapsible_if|src/codegen/regalloc/rewrite.rs" /tmp/task58_clippy_review2.log -> exit 1 (no matches)

Build/test commands
- cargo fmt --all -- --check -> exit 0
- cargo check --release -> exit 0
- cargo build --release -> exit 0
- cargo test --release -> exit 0 (10 binary unit tests passed; lib/doc tests empty)
- ./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing -> exit 0 (Ran 66 tests; OK, but invalid as acceptance evidence because runner.py is modified)
- /tmp pointer-multiply smoke: ./target/release/rustcc --no-coalescing -S /tmp/task58_mul_ptr.c -> exit 0; gcc /tmp/task58_mul_ptr.s -o /tmp/task58_mul_ptr -> exit 0; /tmp/task58_mul_ptr -> exit 42 (expected semantic return)
```

## Blockers before approval

1. Revert the `tests/test_framework/runner.py` change or otherwise restore the official `tests/` harness contents unchanged. Do not rely on a patched test runner to forward `--no-coalescing` into the compiler.
2. Make the required `./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing` acceptance pass under the locked official harness contract, with the behavior implemented in the native Rust compiler/regalloc path rather than the Python test harness.

Final verdict: REJECT
