VERDICT: REJECT

# Task 56 W21-T1 Assembly Liveness Code Review

Reviewer: independent code-quality reviewer
Date: 2026-07-09
Repo: `/home/mei/projects/rustcc`
HEAD reviewed: `f03a24f`
Report path: `.omo/evidence/task-56-liveness-code-review.md`

## Summary

The liveness fixed-point transfer shape is close to the OCaml reference for basic non-call instructions, and the implementation is scoped to W21-T1 (no graph coloring/spilling/coalescing beyond the existing stub and liveness-facing types). However, it is **not safe to proceed to adversarial gate** because the register-class definitions and call handling do not mirror `nqcc2/lib/backend/regalloc.ml` closely enough.

Most importantly, Rust includes scratch registers in allocator hard/caller-saved sets (`R10`, `R11`, `XMM14`, `XMM15`) that the OCaml allocator deliberately excludes. The current codebase uses those registers as fixup/codegen scratch registers, so making them allocatable/live-clobbered will distort call liveness and later interference/coloring.

## Skill-perspective check

- `omo:remove-ai-slops`: **ran/consulted** by reading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md`. Applied slop review for overfit tests, deletion-only tests, needless abstractions, oversized modules, hidden scope drift.
  - Result: no deletion-only/compiler-phase tests were added; no new dependencies; source files are under 250 pure LOC. The manual probe is narrow and not enough evidence for calls/div/address/XMM, but it is not a tautological production test because it is external evidence rather than committed tests.
- `omo:programming`: **ran/consulted** by reading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md` and Rust reference `references/rust/README.md`. Applied strict Rust review for no `unwrap`/`expect`, no `unsafe`, explicit scope, no needless abstraction, 250 pure LOC ceiling, and meaningful gates.
  - Result: no `unwrap`/`expect`/`unsafe` found in Task 56 files. A global `cargo clippy --release --all-targets -- -D warnings` gate fails on pre-existing project issues; see LOW findings.

## Changed/untracked files inspected

From `git status --short`:

```text
 M src/codegen/assembly.rs
 M src/codegen/regalloc/mod.rs
?? .omo/evidence/task-56-liveness-implementation.txt
?? src/codegen/regalloc/liveness.rs
?? src/codegen/regalloc/operands.rs
?? src/codegen/regalloc/types.rs
```

Also inspected required context:

- `.omo/plans/c-compiler-rust.md` Task 56 (`W21-T1` lines around 1894-1910)
- `.omo/evidence/task-56-liveness-implementation.txt`
- `nqcc2/lib/backend/regalloc.ml`
- `nqcc2/lib/cfg.ml`
- `nqcc2/lib/assembly.ml` (the requested `nqcc2/lib/backend/assembly.ml` path does not exist)
- `nqcc2/lib/optimizations/address_taken.ml`
- `nqcc2/lib/backward_dataflow.ml`
- `src/codegen/assembly.rs`
- `src/codegen/regalloc/{mod.rs,types.rs,operands.rs,liveness.rs}`
- `src/ir/cfg*.rs` and `src/ir/cfg/{build.rs,instr.rs,types.rs}`
- `src/pipeline.rs`

## Findings

### CRITICAL

None.

### HIGH

#### HIGH-1: Allocatable/caller-saved register sets do not match OCaml; scratch registers are incorrectly included

Files:

- `src/codegen/regalloc/types.rs:31-47`
- `src/codegen/regalloc/types.rs:51-64`
- Reference: `nqcc2/lib/backend/regalloc.ml:607-636`

OCaml GP allocator excludes `R10` and `R11` from `all_hardregs` and `caller_saved_regs`:

```text
OCaml GP all_hardregs = [ AX; BX; CX; DX; DI; SI; R8; R9; R12; R13; R14; R15 ]
OCaml GP caller_saved_regs = [ AX; CX; DX; DI; SI; R8; R9 ]
```

Rust includes `Reg::R10` and `Reg::R11` in both:

```text
src/codegen/regalloc/types.rs:40-41 Reg::R10, Reg::R11 in all_hardregs
src/codegen/regalloc/types.rs:61-62 Reg::R10, Reg::R11 in caller_saved_regs
```

OCaml XMM allocator excludes `XMM14` and `XMM15` from `all_hardregs` (`XMM0` through `XMM13` only), while Rust uses `(0..=15)` for both hardregs and caller-saved sets.

This matters now, not only in later coloring: `regs_used_and_written` uses `class.caller_saved_regs()` for `Call`, so calls currently write/clobber `R10/R11` and `XMM14/XMM15` in the liveness sets even though OCaml does not. The repo also uses `R10`, `R11`, `XMM14`, and `XMM15` as scratch registers in codegen/fixup/replace-pseudos paths (`rg` finds many uses), matching the OCaml reason for excluding them from allocation.

Impact: call liveness, live-after annotations, and later interference graph construction will diverge from the OCaml reference. This directly violates the expected GP/XMM class filtering parity.

#### HIGH-2: Call operand analysis silently treats missing callee metadata as “uses no parameter registers”

Files:

- `src/codegen/regalloc/operands.rs:70-80`
- Reference: `nqcc2/lib/backend/regalloc.ml:106-115`
- Reference: `nqcc2/lib/backend/assembly_symbols.ml:119-126`

OCaml `Call f` always queries `Assembly_symbols.param_regs_used f` and `return_regs_used fn_name`; absent/malformed symbol metadata is an internal error, not a valid “no register args” state.

Rust does this:

```rust
config.call_param_regs.get(name).map_or_else(Vec::new, |regs| { ... })
```

That means any unlisted callee is analyzed as using no parameter-passing registers. A call like `f(x)` can therefore lose the liveness edge from the argument register into `Call("f")` unless the caller constructs a perfect `LivenessConfig`. The public `analyze_function_liveness` signature accepts `fn_name` but does not use it to derive return/parameter registers from the project’s function metadata; it relies entirely on external config.

Impact: ordinary call liveness can be wrong while all current cargo/chapter gates still pass, because Task 56 is not wired into compiler behavior and the manual probe does not test calls.

### MEDIUM

#### MEDIUM-1: Rust applies a broad class-retain pass that is not a 1:1 mirror of OCaml `regs_used_and_written`

Files:

- `src/codegen/regalloc/operands.rs:31-33`
- `src/codegen/regalloc/operands.rs:123-142`
- Reference: `nqcc2/lib/backend/regalloc.ml:89-149`

OCaml flattens operands into read/written `OperandSet`s and only explicitly class-filters call parameter registers plus return registers through `R.all_hardregs`; it does not run a final `retain_class` over every read/write. Rust does:

```rust
let mut use_def = UseDef { used, written };
use_def.retain_class(class);
```

This drops non-class hard registers from XMM liveness, including GP address-calculation registers produced from `Memory`/`Indexed` operands. Later graph construction may ignore non-XMM nodes anyway, but the W21-T1 acceptance says small-program/function liveness should match the OCaml/hand reference. This is a semantic mismatch in the liveness output itself.

#### MEDIUM-2: Manual liveness evidence is credible only for a simple branch transfer; it does not cover required adversarial operand classes

Evidence file: `.omo/evidence/task-56-liveness-implementation.txt`
Probe source: `/tmp/rustcc_liveness_probe.rs`

The manual probe confirms a simple `mov/cmp/jcc/add/mov/ret` branch example:

```text
block 0 live_in {Pseudo("a"), Pseudo("c")} live_out {Pseudo("b"), Pseudo("c")}
block 1 live_in {Pseudo("b"), Pseudo("c")} live_out {Pseudo("b")}
block 2 live_in {Pseudo("b")} live_out {Reg(AX)}
```

That supports the basic backward transfer/meet path, but it does not exercise calls, `idiv`/`div`, `cdq`/`cqo`, memory/indexed address operands, GP-vs-XMM filtering, or missing callee metadata. It also uses a `/tmp` mini-CFG scaffold rather than the repo’s `src/ir/cfg` builder. The production `assembly_function_cfg` source looks aligned with OCaml CFG construction, but the provided probe is too narrow to prove the full expected outcome.

### LOW

#### LOW-1: Strict clippy gate is not green globally

Command run:

```bash
cargo clippy --release --all-targets -- -D warnings
```

Result: exit `101`; 31 clippy errors across existing project files. Most appear pre-existing and outside Task 56, but one diagnostic is in a touched file (`src/codegen/assembly.rs:36`, `Reg::XMM` upper-case acronym). Because this review is read-only and the task did not claim clippy, I am not treating this as the primary blocker; however, it means a full lint gate is not green.

#### LOW-2: `Pop` handling diverges from OCaml’s explicit internal-error stance

Files:

- `src/codegen/regalloc/operands.rs:81-88`
- Reference: `nqcc2/lib/backend/regalloc.ml:121-122`

OCaml treats `Pop` in `regs_used_and_written` as an internal error. Rust treats `Instr::Pop(_)` as no use/no write. If `Pop` reaches liveness, Rust will silently compute an incorrect set rather than surfacing a malformed instruction stream. This is low risk for W21-T1 if `Pop` is not expected before allocation, but it is another non-1:1 semantic difference.

## Positive observations

- Scope is mostly controlled: no interference graph, coloring, spilling, or coalescing implementation was added beyond liveness-facing types and the pre-existing `allocate()` placeholder.
- `transfer` implements `(live_after - written) union used` walking instructions backward, matching the OCaml shape in `regalloc.ml:251-270`.
- `meet` unions successor block live-ins and adds return registers at `Exit`, matching the OCaml shape in `regalloc.ml:231-249` when `LivenessConfig` is correctly populated.
- Memory and indexed operands are flattened as address-register reads for ordinary use/def before Rust’s class filter.
- Files measured below the 250 pure LOC ceiling.
- No new dependencies, no official test edits, and no compiler-phase Rust tests were added.
- No `unwrap`, `expect`, or `unsafe` was found in the Task 56 files.

## Commands and results

### Repo state

```bash
pwd
git rev-parse --short HEAD
git status --short
git diff --name-status
git ls-files --others --exclude-standard
git diff --stat
```

Result summary:

```text
PWD: /home/mei/projects/rustcc
HEAD: f03a24f
 M src/codegen/assembly.rs
 M src/codegen/regalloc/mod.rs
?? .omo/evidence/task-56-liveness-implementation.txt
?? src/codegen/regalloc/liveness.rs
?? src/codegen/regalloc/operands.rs
?? src/codegen/regalloc/types.rs
Tracked diff stat: 2 files changed, 25 insertions(+), 11 deletions(-)
```

### Required plan/evidence/reference inspection

```bash
grep -n -A80 -B20 'Task 56\|W21-T1\|liveness' .omo/plans/c-compiler-rust.md
cat .omo/evidence/task-56-liveness-implementation.txt
nl -ba nqcc2/lib/backend/regalloc.ml | sed -n '1,700p'
nl -ba nqcc2/lib/cfg.ml | sed -n '1,380p'
find nqcc2/lib -maxdepth 3 -type f -name 'assembly.ml' -o -name '*assembly*' | sort
nl -ba nqcc2/lib/assembly.ml | sed -n '1,260p'
nl -ba nqcc2/lib/optimizations/address_taken.ml | sed -n '1,120p'
nl -ba nqcc2/lib/backward_dataflow.ml | sed -n '1,140p'
```

Result summary: Task 56 requires W21-T1 assembly liveness; requested `nqcc2/lib/backend/assembly.ml` does not exist, actual path is `nqcc2/lib/assembly.ml`; OCaml register-class source confirms GP excludes `R10/R11` and XMM excludes `XMM14/XMM15`.

### Source/diff inspection

```bash
git diff -- src/codegen/assembly.rs src/codegen/regalloc/mod.rs
nl -ba src/codegen/assembly.rs | sed -n '1,420p'
nl -ba src/codegen/regalloc/mod.rs | sed -n '1,220p'
nl -ba src/codegen/regalloc/types.rs | sed -n '1,220p'
nl -ba src/codegen/regalloc/operands.rs | sed -n '1,260p'
nl -ba src/codegen/regalloc/liveness.rs | sed -n '1,260p'
find src/ir/cfg -type f -maxdepth 2 -print -exec sh -c 'echo --- $1 ---; nl -ba "$1" | sed -n "1,320p"' sh {} \;
nl -ba src/pipeline.rs | sed -n '1,260p'
```

Result summary: liveness is implemented in new `types.rs`, `operands.rs`, and `liveness.rs`; `mod.rs` exports liveness/use-def APIs; `assembly.rs` only gains ordering derives for `Reg`/`Operand`; CFG source is the existing assembly CFG builder.

### Targeted semantic comparison

```bash
nl -ba nqcc2/lib/backend/regalloc.ml | sed -n '607,636p'
nl -ba src/codegen/regalloc/types.rs | sed -n '28,66p'
nl -ba src/codegen/regalloc/operands.rs | sed -n '70,80p'
nl -ba nqcc2/lib/backend/regalloc.ml | sed -n '106,115p'
```

Key result:

```text
OCaml GP all_hardregs: AX BX CX DX DI SI R8 R9 R12 R13 R14 R15
OCaml GP caller_saved: AX CX DX DI SI R8 R9
Rust GP all_hardregs: AX BX CX DX DI SI R8 R9 R10 R11 R12 R13 R14 R15
Rust GP caller_saved: AX CX DX DI SI R8 R9 R10 R11
OCaml XMM hardregs: XMM0..XMM13
Rust XMM hardregs/caller-saved: XMM0..XMM15
```

### Hygiene / LOC

```bash
for f in src/codegen/assembly.rs src/codegen/regalloc/mod.rs src/codegen/regalloc/types.rs src/codegen/regalloc/operands.rs src/codegen/regalloc/liveness.rs src/ir/cfg.rs src/ir/cfg/build.rs src/ir/cfg/instr.rs src/ir/cfg/types.rs src/pipeline.rs; do
  awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' "$f" | wc -l
done
rg -n '\b(unwrap|expect)\s*\(|unsafe\b|dbg!\s*\(|todo!\s*\(|unimplemented!\s*\(' src/codegen/assembly.rs src/codegen/regalloc/mod.rs src/codegen/regalloc/types.rs src/codegen/regalloc/operands.rs src/codegen/regalloc/liveness.rs || true
git diff -- Cargo.toml Cargo.lock tests .omo/plans docs src/pipeline.rs
rg -n 'system[_-]?c|evaluate_with_system|compile_with_system|system_c|bridge|gcc -S|gcc -E' src || true
git diff --check
```

Results:

```text
src/codegen/assembly.rs: 204
src/codegen/regalloc/mod.rs: 21
src/codegen/regalloc/types.rs: 66
src/codegen/regalloc/operands.rs: 130
src/codegen/regalloc/liveness.rs: 102
src/ir/cfg.rs: 8
src/ir/cfg/build.rs: 182
src/ir/cfg/instr.rs: 95
src/ir/cfg/types.rs: 155
src/pipeline.rs: 95
unwrap/expect/unsafe scan: only src/codegen/regalloc/mod.rs:34 unimplemented!(...) existing W21 placeholder
Cargo/tests/docs/pipeline diff: empty
system-C/bridge scan: no matches
git diff --check: exit 0
```

### Verification gates run independently

```bash
cargo fmt --all -- --check
```

Result: exit `0`.

```bash
cargo check --release
```

Result:

```text
Finished `release` profile [optimized] target(s) in 0.01s
[exit 0]
```

```bash
cargo build --release
```

Result:

```text
Finished `release` profile [optimized] target(s) in 0.01s
[exit 0]
```

```bash
cargo test --release
```

Result:

```text
running 0 tests ... ok
running 10 tests ... ok
Doc-tests rustcc ... ok
[exit 0]
```

```bash
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
```

Result:

```text
Ran 120 tests in 2.810s
OK
[exit 0]
```

```bash
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
```

Result:

```text
Ran 27 tests in 0.589s
OK
[exit 0]
```

```bash
./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union
```

Result:

```text
Ran 286 tests in 5.129s
OK
Assembler warnings about truncated initializer values were printed by the harness artifacts.
[exit 0]
```

```bash
rustc --edition=2024 /tmp/rustcc_liveness_probe.rs -o /tmp/rustcc_liveness_probe && /tmp/rustcc_liveness_probe
```

Result:

```text
warning: variant `Xmm` is never constructed
block 0 live_in {Pseudo("a"), Pseudo("c")} live_out {Pseudo("b"), Pseudo("c")}
block 1 live_in {Pseudo("b"), Pseudo("c")} live_out {Pseudo("b")}
block 2 live_in {Pseudo("b")} live_out {Reg(AX)}
[exit 0]
```

```bash
cargo clippy --release --all-targets -- -D warnings
```

Result: exit `101`; 31 clippy errors, mostly outside Task 56. See LOW-1.

## Acceptance assessment

- Limited to W21-T1 liveness foundation: **PASS**.
- No interference graph/coloring/spilling/coalescing beyond placeholders: **PASS**.
- Ordinary mov/binary/unary/cmp/setcc/push/idiv/div/cdq operand use-def shape: **MOSTLY PASS**.
- Calls mirror OCaml: **REJECT** due silent missing metadata default and incorrect caller-saved sets.
- GP/XMM hard register filtering mirrors OCaml: **REJECT** due `R10/R11` and `XMM14/XMM15` inclusion.
- Address operand flattening: **PARTIAL**; flattening exists, but final class filter changes OCaml liveness output for non-class hard registers.
- Backward dataflow live-after/live-in/live-out algorithm: **PASS for core transfer/meet shape**.
- Files below 250 pure LOC: **PASS**.
- No unwrap/expect/unsafe/new deps/compiler-phase Rust tests/official test edits/system-C fingerprints: **PASS**.
- Verification gates: **PARTIAL**; fmt/check/build/test/chapter gates pass, strict clippy does not.
- Manual small-function evidence: **PARTIAL**; credible for one branch example, insufficient for the required adversarial cases.

## Blockers before approval

1. Make `RegisterClass::{all_hardregs, caller_saved_regs}` match OCaml exactly:
   - GP hardregs: `AX, BX, CX, DX, DI, SI, R8, R9, R12, R13, R14, R15`
   - GP caller-saved: `AX, CX, DX, DI, SI, R8, R9`
   - XMM hardregs/caller-saved: `XMM0..XMM13`
   - Keep `R10/R11/XMM14/XMM15` reserved for scratch/fixup use.
2. Do not silently default missing call metadata to no parameter registers. Mirror `Assembly_symbols.param_regs_used` behavior by deriving from function metadata or returning an error/explicit missing-config condition.
3. Re-check the final `retain_class` behavior against OCaml. If the intent is to deviate, document why and prove liveness/interference remains equivalent; otherwise remove/narrow it to match OCaml’s filtering points.
4. Add/record adversarial manual evidence for calls, `idiv`/`div`, address operands, and GP/XMM classes before claiming readiness for adversarial gate.

## Final recommendation

REQUEST_CHANGES. The branch should not proceed to adversarial gate until the HIGH findings are fixed and independently re-verified.
