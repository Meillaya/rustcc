VERDICT: PASS

# Task 56 W21-T1 Liveness Re-review After Fix

Reviewer: independent read-only code-quality reviewer
Date: 2026-07-09
Repo: `/home/mei/projects/rustcc`
Report path: `/home/mei/projects/rustcc/.omo/evidence/task-56-liveness-code-review-2.md`

## Summary

The blockers from `.omo/evidence/task-56-liveness-code-review.md` are fixed in the current source. The register class definitions now match the OCaml allocator sets exactly, missing call metadata now returns an error, and the broad post-processing class retain pass was removed so ordinary memory/address operand liveness follows the OCaml `regs_used_and_written` semantics. I independently compiled and ran a temporary probe that covers branch liveness, calls, missing metadata, `idiv`/`div`, memory/indexed operands, and GP/XMM filtering.

Recommendation: **APPROVE**. Keep a watch item for the global strict clippy gate, which still fails on pre-existing project-wide diagnostics outside Task 56 and one pre-existing `Reg::XMM` acronym diagnostic in a touched file.

## Skill-perspective check

- `omo:remove-ai-slops`: **ran/consulted** by reading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md`. Applied its overfit/slop review to production code and evidence shape.
  - Result: no deletion-only tests, tautological tests, implementation-mirroring committed tests, new dependencies, or unnecessary production data extraction/parsing/normalization found in Task 56 scope.
- `omo:programming`: **ran/consulted** by reading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md`, `references/rust/README.md`, and `references/code-smells.md`. Applied Rust strictness, no `unwrap`/`expect`/`unsafe`, typed error, and 250 pure LOC review.
  - Result: no `unwrap`/`expect`/`unsafe` in Task 56 files; all Task 56 files are under 250 pure LOC. No violation of either skill perspective found that should block approval.

## Inputs inspected

Required evidence:

- `.omo/evidence/task-56-liveness-code-review.md` — prior verdict was `REJECT` with blockers on regclass sets, missing call metadata, broad class retain behavior, and insufficient manual probes.
- `.omo/evidence/task-56-liveness-fix.txt` — claimed fixes and gates; treated as untrusted until independently checked.

OCaml references:

- `nqcc2/lib/backend/regalloc.ml:89-149` (`regs_used_and_written`)
- `nqcc2/lib/backend/regalloc.ml:231-272` (liveness meet/transfer)
- `nqcc2/lib/backend/regalloc.ml:607-636` (GP/XMM register-class definitions)
- `nqcc2/lib/backend/assembly_symbols.ml:119-126` (call/return metadata lookup)
- `nqcc2/lib/assembly.ml:3-46,73-109` (register/operand/instruction surfaces)

Current Rust source:

- `src/codegen/assembly.rs`
- `src/codegen/regalloc/mod.rs`
- `src/codegen/regalloc/types.rs`
- `src/codegen/regalloc/operands.rs`
- `src/codegen/regalloc/liveness.rs`
- `src/ir/cfg.rs` and `src/ir/cfg/{build.rs,instr.rs,types.rs}`
- `.omo/plans/c-compiler-rust.md` Task 56 / W21-T1 scope

## Prior blocker re-check

### 1. RegisterClass sets mirror OCaml exactly — PASS

Rust source:

- `src/codegen/regalloc/types.rs:52-68`: GP hardregs are `AX, BX, CX, DX, DI, SI, R8, R9, R12, R13, R14, R15`; XMM hardregs are `XMM0..=XMM13`.
- `src/codegen/regalloc/types.rs:72-84`: GP caller-saved are `AX, CX, DX, DI, SI, R8, R9`; XMM caller-saved are `XMM0..=XMM13`.
- `src/codegen/regalloc/types.rs:87-105`: `contains` excludes `R10/R11` and `XMM14/XMM15`.

OCaml reference:

- `nqcc2/lib/backend/regalloc.ml:607-610`: GP hardregs and caller-saved exclude `R10/R11`.
- `nqcc2/lib/backend/regalloc.ml:614-635`: XMM hardregs/caller-saved are `XMM0..XMM13`, excluding `XMM14/XMM15`.

### 2. Missing call metadata errors rather than silently no-param — PASS

Rust source:

- `src/codegen/regalloc/types.rs:31-34`: defines `LivenessError::MissingCallMetadata`.
- `src/codegen/regalloc/operands.rs:68-84`: `Instr::Call(name)` now does `config.call_param_regs.get(name).ok_or_else(...) ?` before filtering call parameter registers.

This matches the OCaml stance that `Assembly_symbols.param_regs_used` queries the symbol table and fails on invalid/missing function metadata (`nqcc2/lib/backend/assembly_symbols.ml:119-126`) rather than silently treating a call as using no registers.

### 3. Retain/filter behavior matches OCaml class-specific semantics — PASS

The broad final `retain_class` pass from the rejected review is gone. Current `regs_used_and_written` flattens operands directly:

- `src/codegen/regalloc/operands.rs:20-31`: reads and writes are flattened from raw operands without a final class retain.
- `src/codegen/regalloc/operands.rs:97-125`: memory and indexed operands preserve address registers as reads, including hard registers used only for addressing.
- `src/codegen/regalloc/operands.rs:69-81`: class filtering is limited to call parameter registers against `all_hardregs`, matching `regalloc.ml:106-114`.
- `src/codegen/regalloc/liveness.rs:78-82`: return-register filtering is limited to `return_regs ∩ all_hardregs`, matching `regalloc.ml:231-239`.

### 4. Manual probes cover required cases — PASS

I independently ran a temporary standalone probe compiled against the current source files. It checked:

- GP/XMM class sets and scratch exclusions.
- Missing call metadata returns `LivenessError::MissingCallMetadata`.
- GP and XMM call parameter filtering plus caller-saved write sets.
- `idiv` and `div` implicit `%ax/%dx` use/write behavior.
- Memory/indexed address operands preserve base/index register reads.
- XMM-class memory operand liveness keeps GP address registers while writing the XMM destination.
- A small branch CFG whose block live-in/live-out sets match the hand reference.

Probe result: `PASS task56 review probe: regclasses, call metadata/filtering, idiv/div/address operands, XMM/GP, branch liveness`.

### 5. Scope control / hygiene — PASS

- W21-T1 scope is respected: liveness/use-def support only; `allocate()` remains a W21 placeholder (`src/codegen/regalloc/mod.rs:64-65`); no interference graph, simplification, coloring, spilling, or coalescing implementation was added.
- No changes to `Cargo.toml`, `Cargo.lock`, `tests`, `docs`, `.omo/plans`, or `src/pipeline.rs`.
- No new committed tests, dependencies, bridge/system-C code, or production probe binaries.
- `src/bin/task56_liveness_probe.rs` is absent.
- Task 56 files are below 250 pure LOC.

## Findings by severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

None.

### LOW

#### LOW-1: Global strict clippy gate still fails on pre-existing project-wide diagnostics

Command:

```bash
cargo clippy --release --all-targets -- -D warnings
```

Result: exit `101`; 31 clippy errors. Most are outside Task 56 (`src/ast/*`, `src/ir/lower.rs`, `src/semantics/*`, etc.). One diagnostic is in a touched file, `src/codegen/assembly.rs:36` (`Reg::XMM` upper-case acronym), but the enum variant predates this task and the Task 56 diff only adds ordering derives. I am not treating this as a blocker for the liveness fix, but the repository's strict lint gate is not globally green.

#### LOW-2: First chapter 19 latest-only gate run showed harness intermediate-file flake; immediate rerun passed

First command:

```bash
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
```

Initial result: exit `1`; `Ran 120 tests in 2.680s`, `FAILED (failures=4, errors=16)`, with failures all shaped as missing generated `.i`/`.s`/executable intermediate files under `tests/tests/chapter_19/...`.

Immediate rerun of the exact same command: exit `0`; `Ran 120 tests in 2.820s`, `OK`. Because the rerun passed and the errors were missing temporary harness artifacts rather than liveness output mismatches, this is a watch item, not a Task 56 blocker.

## Commands and results

### Skill/reference loading

```bash
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/code-smells.md
```

Result: all files read successfully; applied anti-slop and Rust/code-smell criteria above.

### Evidence and source inspection

```bash
cat .omo/evidence/task-56-liveness-code-review.md
cat .omo/evidence/task-56-liveness-fix.txt
```

Result: prior blockers and claimed fix evidence loaded; independently verified rather than trusted.

```bash
grep -n -A45 -B15 'Task 56\|W21-T1\|Wave 21' .omo/plans/c-compiler-rust.md
```

Result: Task 56 is W21-T1 liveness only; W21-T2+ graph/color/spill/coalesce work starts at Tasks 57+.

```bash
nl -ba nqcc2/lib/backend/regalloc.ml | sed -n '80,160p;220,280p;607,636p'
nl -ba nqcc2/lib/backend/assembly_symbols.ml | sed -n '100,135p'
nl -ba nqcc2/lib/assembly.ml | sed -n '1,220p'
```

Key results:

```text
OCaml GP all_hardregs = AX BX CX DX DI SI R8 R9 R12 R13 R14 R15
OCaml GP caller_saved_regs = AX CX DX DI SI R8 R9
OCaml XMM all_hardregs/caller_saved_regs = XMM0..XMM13
Call uses Assembly_symbols.param_regs_used f and filters against R.all_hardregs
Return uses Assembly_symbols.return_regs_used fn_name and intersects all_hardregs
Memory/Indexed operands read address base/index registers
```

```bash
git diff -- src/codegen/assembly.rs src/codegen/regalloc/mod.rs
for f in src/codegen/regalloc/types.rs src/codegen/regalloc/operands.rs src/codegen/regalloc/liveness.rs; do nl -ba "$f" | sed -n '1,260p'; done
nl -ba src/codegen/regalloc/mod.rs | sed -n '1,220p'
nl -ba src/codegen/assembly.rs | sed -n '1,260p'
```

Result: tracked diff is limited to ordering derives in `assembly.rs` and liveness module exports/API/error wrapper in `regalloc/mod.rs`; untracked new regalloc files contain liveness/use-def/types only.

```bash
rg -n 'retain_class|contains\(|all_hardregs|caller_saved_regs|MissingCallMetadata|PopInLiveness' src/codegen/regalloc/*.rs
```

Result: no `retain_class`; filtering appears only in call and return-register paths plus transfer kill-set membership.

### Repo state / scope

```bash
git status --short
git diff --name-status
git ls-files --others --exclude-standard | sort | sed -n '1,120p'
git diff --stat
```

Result summary:

```text
 M src/codegen/assembly.rs
 M src/codegen/regalloc/mod.rs
?? src/codegen/regalloc/liveness.rs
?? src/codegen/regalloc/operands.rs
?? src/codegen/regalloc/types.rs
?? .omo/evidence/task-56-liveness-code-review.md
?? .omo/evidence/task-56-liveness-fix.txt
?? .omo/evidence/task-56-liveness-implementation.txt
?? .omo/evidence/task-56-liveness-adversarial-verify*.txt/md
Tracked diff stat: 2 files changed, 56 insertions(+), 11 deletions(-)
```

Other unrelated untracked `.omo/evidence/task-*` artifacts pre-existed in the working tree and were not part of this review decision.

```bash
git diff --name-only -- Cargo.toml Cargo.lock tests docs .omo/plans src/pipeline.rs
rg -n 'system[_-]?c|evaluate_with_system|compile_with_system|system_c|bridge|gcc -S|gcc -E' src Cargo.toml || true
test ! -e src/bin/task56_liveness_probe.rs; echo "src/bin/task56_liveness_probe.rs absent exit=$?"
```

Result:

```text
(no changed Cargo/tests/docs/plan/pipeline paths)
(no bridge/system-C matches in src or Cargo.toml)
src/bin/task56_liveness_probe.rs absent exit=0
```

### LOC / hygiene

```bash
for f in src/codegen/assembly.rs src/codegen/regalloc/mod.rs src/codegen/regalloc/types.rs src/codegen/regalloc/operands.rs src/codegen/regalloc/liveness.rs; do
  printf '%s ' "$f"
  awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' "$f" | wc -l
done
```

Result:

```text
src/codegen/assembly.rs 204
src/codegen/regalloc/mod.rs 46
src/codegen/regalloc/types.rs 93
src/codegen/regalloc/operands.rs 116
src/codegen/regalloc/liveness.rs 106
```

```bash
rg -n '\b(unwrap|expect)\s*\(|unsafe\b|dbg!\s*\(|todo!\s*\(' src/codegen/assembly.rs src/codegen/regalloc/mod.rs src/codegen/regalloc/types.rs src/codegen/regalloc/operands.rs src/codegen/regalloc/liveness.rs || true
rg -n 'unimplemented!\s*\(' src/codegen/assembly.rs src/codegen/regalloc/mod.rs src/codegen/regalloc/types.rs src/codegen/regalloc/operands.rs src/codegen/regalloc/liveness.rs || true
git diff --check
```

Result:

```text
unwrap/expect/unsafe/dbg/todo scan: no matches
unimplemented scan: src/codegen/regalloc/mod.rs:65:    unimplemented!("ch.20 regalloc wired in wave 21")
git diff --check: exit 0
```

The `unimplemented!` is the pre-existing W21 allocator placeholder and is in scope for later tasks, not W21-T1 liveness.

### Required gates

```bash
cargo fmt --all -- --check
```

Result: exit `0`; no output.

```bash
cargo check --release
```

Result:

```text
Finished `release` profile [optimized] target(s) in 0.04s
```

```bash
cargo build --release
```

Result:

```text
Finished `release` profile [optimized] target(s) in 0.01s
```

```bash
cargo test --release
```

Result:

```text
running 0 tests ... ok
running 10 tests ... ok
Doc-tests rustcc ... ok
```

```bash
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
```

Result on immediate rerun after one transient harness failure:

```text
Ran 120 tests in 2.820s
OK
```

```bash
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
```

Result:

```text
Ran 27 tests in 0.615s
OK
```

```bash
./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union
```

Result:

```text
Ran 286 tests in 5.097s
OK
```

The chapter 18 union gate also printed two assembler truncation warnings from existing chapter 18 initializer fixtures; the harness still returned exit `0`.

### Independent liveness probe

Command:

```bash
ANYHOW=$(ls target/release/deps/libanyhow-*.rlib | head -1)
rustc --edition=2024 /tmp/task56_liveness_review_probe.rs \
  -L dependency=target/release/deps \
  --extern anyhow="$ANYHOW" \
  -o /tmp/task56_liveness_review_probe
/tmp/task56_liveness_review_probe
```

Result:

```text
warning: 16 warnings emitted
PASS task56 review probe: regclasses, call metadata/filtering, idiv/div/address operands, XMM/GP, branch liveness
```

Warnings are from the temporary standalone harness stubs/re-exports and are not production compile diagnostics. The probe compiled against current repo source via absolute `#[path = ...]` module inclusions.

### Strict clippy watch gate

```bash
cargo clippy --release --all-targets -- -D warnings
```

Result:

```text
exit 101
could not compile `rustcc` (bin "rustcc") due to 31 previous errors
```

Representative diagnostics include `src/ast/decl.rs:62`, `src/ast/expr.rs:35`, `src/ast/ty.rs:45`, `src/codegen/assembly.rs:36`, `src/codegen/mod.rs:32`, `src/ir/lower.rs:*`, `src/parse/parser.rs:*`, and `src/semantics/*`. This was already present in the prior review and is not introduced by the liveness fix.

## Acceptance assessment

- RegisterClass mirrors OCaml exactly: **PASS**.
- GP excludes `R10/R11`; XMM excludes `XMM14/XMM15`: **PASS**.
- Missing call metadata errors instead of no-param default: **PASS**.
- Retain/filter behavior matches OCaml class-specific semantics: **PASS**.
- Manual probes cover branch, calls, missing metadata, `div`/`idiv`, memory/address operands, GP/XMM classes: **PASS**.
- No W21-T2+ scope creep: **PASS**.
- Files under 250 pure LOC: **PASS**.
- No `unwrap`/`expect`/`unsafe` in Task 56 files: **PASS**.
- No committed tests/deps/bridge/system-C/probe binary: **PASS**.
- Required task gates: **PASS**, with one transient chapter 19 harness flake that passed on immediate rerun.
- Strict clippy: **WATCH**, pre-existing global failure not attributed to this fix.

## Final recommendation

**APPROVE**. The prior HIGH blockers are resolved. No CRITICAL/HIGH/MEDIUM findings remain for Task 56 W21-T1.
