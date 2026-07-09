# Task 60 Code Quality Review: Conservative Coalescing + Both Modes

Verdict: REJECT
codeQualityStatus: BLOCK
recommendation: REQUEST_CHANGES
reviewed_at: 2026-07-09

## Scope Reviewed

Goal: W21-T5 Chapter 20 conservative coalescing with default coalescing enabled and `--no-coalescing` preserving the Task58/59 allocator path.

Inspected required evidence and sources:
- `.omo/evidence/task-60-coalescing-implementation.txt`
- `.omo/evidence/task-60-coalescing-move-count.txt`
- `.omo/evidence/task-60-copy-heavy.c`
- `src/codegen/regalloc/{allocate,coalesce,graph,spill,types,mod,color,rewrite,scratch,operands}.rs`
- `src/codegen/{codegen.rs,fixup.rs,replace_pseudos.rs}`
- `src/driver.rs`, `src/compiler.rs`, `src/pipeline.rs`
- `nqcc2/lib/backend/regalloc.ml`, `nqcc2/lib/util/disjoint_sets.ml`
- `.omo/plans/c-compiler-rust.md` Task 60

Skill-perspective check: ran. I loaded `omo:remove-ai-slops` and `omo:programming`, plus the Rust programming reference and code-smells reference. The diff violates both perspectives on scope/maintainability: an unrelated peephole cleanup was added to production regalloc rewrite code, and multiple touched files crossed or remain above the 250 pure-LOC ceiling.

## Findings by Severity

### CRITICAL

None.

### HIGH

1. `src/codegen/regalloc/rewrite.rs:19-74` adds an unrelated dividend-setup peephole in Task60.
   - Why this blocks: Task60 is supposed to implement Briggs/George-style conservative coalescing. This 56-line pattern rewrite is not part of the OCaml coalescing reference (`nqcc2/lib/backend/regalloc.ml:432-468`, `584-604`) and is not needed for the copy-heavy acceptance evidence. It is a separate backend optimization hidden inside `cleanup_redundant_moves`.
   - Safety concern: `dividend_setup_replacement` matches a six-instruction division shape while intentionally ignoring `instructions[index + 2]` (`src/codegen/regalloc/rewrite.rs:49-52`). It then moves the write to `saved_reg` later by replacing the first two instructions with `src -> %eax` (`src/codegen/regalloc/rewrite.rs:65-70`). That is subtle enough to require its own proof, because the ignored instruction may be part of divisor setup and may read the register whose write was moved. The official chapter gate passed, but this unrequested peephole is not directly specified, separately tested, or explained.
   - Remove-ai-slops/programming perspective: this is needless production complexity and scope drift in an allocator task. It should be removed from Task60, or split into a separately justified/tested change with targeted adversarial division cases.

### MEDIUM

1. Touched files exceed the programming skill's 250 pure-LOC ceiling.
   - Measured with `awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' <file> | wc -l`.
   - `src/codegen/regalloc/allocate.rs`: 239 -> 279 pure LOC.
   - `src/codegen/regalloc/graph.rs`: 237 -> 262 pure LOC.
   - `src/codegen/fixup.rs`: 235 -> 277 pure LOC.
   - `src/codegen/replace_pseudos.rs`: 474 -> 516 pure LOC.
   - `src/codegen/regalloc/coalesce.rs`: new file at 245 pure LOC, warning band.
   - This is not the primary rejection reason, but it is a real maintainability risk. Future work should split by responsibility instead of adding more logic to these files.

2. `src/codegen/regalloc/coalesce.rs:156-249` duplicates operand-rewrite coverage that already exists in `src/codegen/regalloc/rewrite.rs:95-190`.
   - This is currently functional, but it creates a drift risk: every new instruction variant or operand mapping rule must be updated in two places. The OCaml reference has a single `replace_ops` helper (`nqcc2/lib/backend/regalloc.ml:46-63`) used by both coloring replacement and coalescing rewrite.
   - Not a blocker by itself because the current instruction coverage appears class-aware and the chapter 20 gates pass.

### LOW

1. `.omo/boulder.json` records Task60 as `running`, while `.omo/plans/c-compiler-rust.md` still shows Task 60 unchecked. This is lifecycle/state hygiene, not compiler correctness.

2. `src/driver.rs:6` contains a generic documentation phrase, "official harness contract". Targeted bridge fingerprints were clean; this is not a test-name/chapter bridge.

## Positive Checks

- Official tests/harness diff is empty: `git diff -- tests` produced no output and exit 0.
- No targeted production bridge fingerprints found:
  - `source_path_hint`
  - `chapter_20` / `chapter20`
  - `test-name` / test-name bridge patterns
  - `SystemAssemblySanitizerOptions`
  - `compile_with_system_cc_frontend`
  - `evaluate_program`
- No dependency changes: `git diff -- Cargo.toml Cargo.lock` is empty.
- No new `unwrap`, `expect`, `unsafe`, `dbg!`, `todo!`, `unimplemented!`, or production `println!` in changed production hunks or the new `coalesce.rs`.
- R10/R11 remain non-allocatable for GP allocation: `RegisterClass::Gp::all_hardregs` is `AX,BX,CX,DX,DI,SI,R8,R9,R12,R13,R14,R15` in `src/codegen/regalloc/types.rs:58-75`; `contains` also excludes R10/R11 at `src/codegen/regalloc/types.rs:93-110`.
- XMM14/XMM15 remain non-allocatable: `RegisterClass::Xmm::all_hardregs` and `caller_saved_regs` use `0..=13` in `src/codegen/regalloc/types.rs:74,89`; `contains` is `XMM(0..=13)` at `src/codegen/regalloc/types.rs:110`.
- Default and `--no-coalescing` paths are real and distinct:
  - Default enables coalescing at `src/driver.rs:54-59`.
  - `--no-coalescing` disables it at `src/driver.rs:100`.
  - Allocator branches on `input.options.coalescing_enabled` at `src/codegen/regalloc/allocate.rs:126-142`.
  - Stdout assembly for `.omo/evidence/task-60-copy-heavy.c` differs: no-coalescing emits `movl $1, %r9d; movl %r9d, %eax`, coalescing emits `movl $1, %eax`.
- Coalescing core is directionally aligned with the OCaml reference:
  - move collection over copies, root finding, graph containment/interference checks: `src/codegen/regalloc/coalesce.rs:25-50`
  - Briggs/George conservative tests: `src/codegen/regalloc/coalesce.rs:52-91`
  - graph merge: `src/codegen/regalloc/graph.rs:142-166`
  - iterative coalescing loop before select: `src/codegen/regalloc/allocate.rs:126-136`
  - separate GP then XMM allocation passes remain at `src/codegen/regalloc/allocate.rs:66-82`
- Codegen/fixup/replace-pseudos XMM changes appear scope-related to allowing regalloc to own XMM pseudos while preserving illegal-asm fixups for memory/data operands.

## Commands Run

```text
git status --short
git diff --stat
git diff -- tests
git diff -- src/codegen/regalloc/allocate.rs src/codegen/regalloc/graph.rs src/codegen/regalloc/rewrite.rs src/codegen/regalloc/mod.rs src/codegen/codegen.rs src/codegen/fixup.rs src/codegen/replace_pseudos.rs src/compiler.rs src/driver.rs src/pipeline.rs
rg -n "source_path_hint|chapter_20|chapter20|test[-_ ]?name|SystemAssemblySanitizerOptions|compile_with_system_cc_frontend|evaluate_program" src || true
rg -n "harness" src || true
rg -n "\.unwrap\(|\.expect\(|unsafe\b|dbg!\(|println!\(|todo!\(|unimplemented!\(" src/codegen/regalloc/coalesce.rs || true
git diff -U0 -- <changed production files> | rg -n '^\+.*(\.unwrap\(|\.expect\(|unsafe\b|dbg!\(|println!\(|todo!\(|unimplemented!\()' || true
git diff -- Cargo.toml Cargo.lock
git diff --check
cargo fmt --all -- --check
cargo test --release
./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing
./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only
./target/release/rustcc --codegen --no-coalescing .omo/evidence/task-60-copy-heavy.c > /tmp/task60-copy-heavy.no-coalescing.s
./target/release/rustcc --codegen .omo/evidence/task-60-copy-heavy.c > /tmp/task60-copy-heavy.coalescing.s
diff -u /tmp/task60-copy-heavy.no-coalescing.s /tmp/task60-copy-heavy.coalescing.s
cargo clippy --release -- -W clippy::all
```

## Check Results

- `git diff -- tests`: PASS, empty.
- `git diff --check`: PASS.
- `cargo fmt --all -- --check`: PASS.
- `cargo test --release`: PASS, 10 tests passed.
- Chapter 20 no-coalescing gate: PASS, 66 tests OK.
- Chapter 20 default coalescing gate: PASS, 66 tests OK.
- Copy-heavy stdout comparison: PASS, default coalescing emits fewer register-to-register moves than no-coalescing under the review regex (2 vs 3), and the direct diff shows the expected copy removal.
- `cargo clippy --release -- -W clippy::all`: exit 0 with existing warnings across unrelated/pre-existing files; not used as a Task60 blocker.

## Blockers

1. Remove or separately justify/test the unrelated dividend setup peephole at `src/codegen/regalloc/rewrite.rs:19-74`. Task60 should not hide a non-reference division optimization inside the coalescing change.
2. Address or explicitly accept the file-size regression caused by Task60 additions, especially `src/codegen/regalloc/allocate.rs` crossing 250 pure LOC and the new `src/codegen/regalloc/coalesce.rs` landing at 245 pure LOC.

## Final Recommendation

REQUEST_CHANGES. The coalescing path itself passes the required gates and appears broadly correct, but the unrequested dividend peephole is scope drift with subtle semantic risk, and the diff violates the loaded remove-ai-slops/programming maintainability criteria.
