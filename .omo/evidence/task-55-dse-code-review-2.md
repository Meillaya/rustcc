VERDICT: PASS

# Task 55 / W20-T5 Dead Store Elimination Re-review 2

Date: 2026-07-08T23:22:43-04:00
Repository: `/home/mei/projects/rustcc`
Review role: independent code quality re-reviewer, read-only except this report artifact
Task: W20-T5 / Task 55, "Chapter 19 - dead store elimination + default all-optimizations gate"

## Summary

The first REJECT's blocking issues are addressed well enough to advance to the adversarial gate.

- The prior CRITICAL extern/global DSE bug is fixed. I reproduced the linked extern-global scenario: baseline, DSE-only, and all-optimization builds all exit `5`, and optimized TACKY preserves the `Copy { dst: "g" }` store.
- The new DSE implementation is split across `src/ir/dead_store_elim/{mod,analysis,liveness,rewrite,util}.rs`; each file is under the 250 pure-LOC ceiling.
- No new `expect(` or `.unwrap(` appears in the Task 55 scoped diff/new DSE files.
- No compiler-phase Rust unit tests were added; `tests/`, `Cargo.toml`, and `Cargo.lock` diffs are empty, and `src/ir src/codegen src/lex src/pipeline.rs` still have no `#[test]` or `#[cfg(test)]` markers.
- Required official gates pass locally. Optional copy-prop/UCE/fold and chapter 18 union regression gates also pass.

`codeQualityStatus`: WATCH
`recommendation`: APPROVE
`reportPath`: `/home/mei/projects/rustcc/.omo/evidence/task-55-dse-code-review-2.md`
`blockers`: none

WATCH is due to non-DSE support edits touching already large files, and `src/ir/copy_propagation/rewrite.rs` crossing 250 pure LOC. I do not classify this as a blocker for this re-review because the specific prior blockers were fixed, the DSE files themselves satisfy the split rule, and the support edits are bounded to official Chapter 19 whole-pipeline behavior.

## Skill-perspective check

- `omo:remove-ai-slops`: ran. I loaded `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md` and applied its overfit/slop checks to production and tests. No deletion-only tests, tautological tests, implementation-mirroring tests, or new compiler-phase tests were added. No CRITICAL/HIGH slop remains in the DSE implementation. WATCH: support edits add nontrivial code to existing large modules.
- `omo:programming`: ran. I loaded `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md`, `references/rust/README.md`, and `references/code-smells.md`. No new `unwrap`/`expect` outside tests and no `unsafe` were found in the scoped Task 55 diff/new DSE files. The strict 250 pure-LOC perspective is satisfied for DSE files, but still flags modified non-DSE files, especially `src/ir/copy_propagation/rewrite.rs` at 304 pure LOC after a +60 pure-LOC delta. I classify that as MEDIUM/WATCH, not a Task 55 blocker.

## Plan and prior evidence inspected

Commands/files inspected:

```text
$ git status --short
 M .omo/boulder.json
 M src/codegen/codegen.rs
 M src/ir/copy_propagation/facts.rs
 M src/ir/copy_propagation/rewrite.rs
 M src/ir/mod.rs
 M src/ir/opt.rs
 M src/lex/scanner.rs
 M src/pipeline.rs
?? .omo/evidence/task-55-dse-code-review.md
?? .omo/evidence/task-55-dse-fix.txt
?? .omo/evidence/task-55-dse-implementation.txt
?? src/ir/dead_store_elim/
...
```

- Plan guardrails read from `.omo/plans/c-compiler-rust.md`: official `test_compiler` harness is the only compiler-correctness verification path; `tests/` contents remain unchanged; Chapter 19 has exactly the four named optimization passes; Task 55 requires `--chapter 19 --latest-only --eliminate-dead-stores` and default `--chapter 19 --latest-only` passing.
- Prior REJECT read from `.omo/evidence/task-55-dse-code-review.md`: blockers were extern-global DSE removal, unsplit 400 pure-LOC DSE file, production `expect()`, and broad non-DSE edits.
- Executor fix evidence read from `.omo/evidence/task-55-dse-fix.txt`: claims were not trusted until source and commands below were inspected.

## Source review evidence

### DSE extern/global fix

Relevant source:

- `src/ir/dead_store_elim/analysis.rs:16-27` builds `static_vars` from emitted statics plus function `type_env` names that are not local/tmp/const/string names, covering file-scope `extern` names that are absent from `TackyProgram.static_variables`.
- `src/ir/dead_store_elim/liveness.rs:53-68` makes `static_vars` live at CFG exit.
- `src/ir/dead_store_elim/liveness.rs:103-120` keeps static/aliased vars live across calls and loads.
- `src/ir/dead_store_elim/rewrite.rs:51-76` does not delete calls, opaque stores, or copy bytes by default, and only deletes known-memory stores when the resolved base is not live.
- `src/ir/lower.rs:319-331` still intentionally omits file-scope `extern` declarations from emitted `static_variables`, so the new function `type_env` inclusion is necessary for the prior bug class.

Manual repro commands and exact results:

```text
$ ./target/release/rustcc --tacky --eliminate-dead-stores /tmp/task55_extern_store.c
TackyProgram {
    functions: [
        TackyFunction {
            name: "set_g",
            body: [
                Copy { src: Constant(5), dst: "g" },
                Return(Constant(0)),
            ],
            type_env: { "g": Int },
            ...
        },
    ],
    static_variables: [],
    ...
}
EXIT: 0

$ ./target/release/rustcc --tacky --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_extern_store.c
Tacky body for set_g still contains `Copy { src: Constant(5), dst: "g" }`.
EXIT: 0

$ ./target/release/rustcc -S /tmp/task55_extern_store.c
EXIT: 0
$ gcc /tmp/task55_extern_store.s /tmp/task55_extern_client.c -o /tmp/task55_extern_baseline
EXIT: 0
$ /tmp/task55_extern_baseline
EXIT: 5

$ ./target/release/rustcc -S --eliminate-dead-stores /tmp/task55_extern_store.c
EXIT: 0
$ gcc /tmp/task55_extern_store.s /tmp/task55_extern_client.c -o /tmp/task55_extern_dse_only
EXIT: 0
$ /tmp/task55_extern_dse_only
EXIT: 5

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_extern_store.c
EXIT: 0
$ gcc /tmp/task55_extern_store.s /tmp/task55_extern_client.c -o /tmp/task55_extern_all_opts
EXIT: 0
$ /tmp/task55_extern_all_opts
EXIT: 5
```

I also ran an extern aggregate write smoke check; all-optimization linked output exited `3` for a two-int struct assignment, matching the expected `g.a + g.b` value.

### DSE split and LOC

Pure LOC measured with `awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#)/' <file> | wc -l`:

```text
src/ir/dead_store_elim/mod.rs        61
src/ir/dead_store_elim/analysis.rs   30
src/ir/dead_store_elim/liveness.rs  113
src/ir/dead_store_elim/rewrite.rs    84
src/ir/dead_store_elim/util.rs      148
```

The prior 400 pure-LOC monolith blocker is fixed.

### Opt/default wiring

- `src/ir/mod.rs:24-28` registers `mod dead_store_elim`.
- `src/ir/opt.rs:24-28` includes `OptPass::DeadStoreElim`.
- `src/ir/opt.rs:36-66` runs selected passes in book order to fixed point and now calls `eliminate_dead_stores_program` for the DSE pass.
- `src/pipeline.rs:80-95` adds `OptPass::DeadStoreElim` when `optimization_flags.eliminate_dead_stores` is true.
- Official default all-optimization behavior is exercised by `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only`, which passed 120 tests.

### No new unwrap/expect or compiler-phase tests

```text
$ (scoped diff plus new src/ir/dead_store_elim/*.rs) | rg -n "\.(expect|unwrap)\s*\("
(no output)

$ (scoped diff plus new src/ir/dead_store_elim/*.rs) | rg -n "#\[cfg\(test\)\]|#\[test\]"
(no output)

$ git diff -- tests Cargo.toml Cargo.lock
(no output)

$ rg -n "#\[cfg\(test\)\]|#\[test\]" src/ir src/codegen src/lex src/pipeline.rs
(no output)
```

### Non-DSE support edits

Scoped diff summary:

```text
src/codegen/codegen.rs             | 139 +++++++++++++++++++++++++++++++++++--
src/ir/copy_propagation/facts.rs   |   1 +
src/ir/copy_propagation/rewrite.rs |  76 ++++++++++++++++++--
src/ir/mod.rs                      |   1 +
src/ir/opt.rs                      |  11 ++-
src/lex/scanner.rs                 |  10 ++-
src/pipeline.rs                    |   3 +
7 files changed, 222 insertions(+), 19 deletions(-)
```

Source inspection:

- `src/lex/scanner.rs:151-163` and `src/lex/scanner.rs:272-284` only adjust unsigned literal kind selection to widen unsuffixed unsigned values above `u32::MAX`, matching the comment and official Chapter 19 unsigned-folding tests.
- `src/ir/copy_propagation/facts.rs:78-88` allows integer constants representable in `i32` to propagate into integer scalar destinations. This is a narrow type-compatibility change.
- `src/ir/copy_propagation/rewrite.rs:59-77` tracks write pointers, and `src/ir/copy_propagation/rewrite.rs:270-317` constrains aggregate address-source propagation to byte arrays and avoids rewriting addresses used as write pointers. This is nontrivial but targeted at official aggregate copy-prop/whole-pipeline tests such as `chapter_19/whole_pipeline/all_types/propagate_into_copyfromoffset.c`.
- `src/codegen/codegen.rs:84-95` and `src/codegen/codegen.rs:661-667` type scalar constant returns from the current function return type, avoiding wrong-width returns exposed by DSE's return-copy collapse.
- `src/codegen/codegen.rs:1915-1995` and `src/codegen/codegen.rs:2125-2148` add a constrained direct lowering for `CopyBytes` into file-scope globals when the destination is provably the address of a global and used once.

Risk assessment: these are still broader than pure DSE wiring, and they touch large files. However, they are tied to official Chapter 19 whole-pipeline behavior, no forbidden tests/dependencies were added, and targeted scalar and aggregate extern/global smoke checks passed. I flag this as MEDIUM/WATCH, not a blocker.

Pure LOC for modified existing files compared to HEAD:

```text
src/ir/copy_propagation/rewrite.rs  HEAD=244 WT=304 delta=+60
src/codegen/codegen.rs              HEAD=1907 WT=2028 delta=+121
src/lex/scanner.rs                  HEAD=557 WT=557 delta=0
src/ir/copy_propagation/facts.rs    HEAD=177 WT=178 delta=+1
src/ir/opt.rs                       HEAD=39  WT=44  delta=+5
src/pipeline.rs                     HEAD=92  WT=95  delta=+3
src/ir/mod.rs                       HEAD=12  WT=13  delta=+1
```

## Verification commands and exact results

```text
$ cargo fmt --all -- --check
EXIT: 0

$ cargo check --release
    Finished `release` profile [optimized] target(s) in 0.03s
EXIT: 0

$ cargo build --release
    Finished `release` profile [optimized] target(s) in 0.01s
EXIT: 0

$ cargo test --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running unittests src/lib.rs (target/release/deps/rustcc-41b78a55704c0e27)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/release/deps/rustcc-b48f2e14c29f3b0e)

running 10 tests
test compiler::tests::compiles_constant_return ... ok
test compiler::tests::compiles_expression_precedence ... ok
test compiler::tests::rejects_bad_lexeme ... ok
test driver::tests::derives_all_output_paths ... ok
test compiler::tests::reaches_validate_through_pass_through_resolve ... ok
test driver::tests::parses_artifact_and_feature_flags ... ok
test compiler::tests::handles_locals_and_assignment ... ok
test driver::tests::parses_default_run_stage ... ok
test driver::tests::parses_stage_flags_as_stdout_only ... ok
test compiler::tests::parses_sizeof_expression_without_evaluating_it ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests rustcc

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
EXIT: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
----------------------------------------------------------------------
Ran 27 tests in 0.615s

OK
EXIT: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
----------------------------------------------------------------------
Ran 120 tests in 2.799s

OK
EXIT: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --propagate-copies
----------------------------------------------------------------------
Ran 42 tests in 0.970s

OK
EXIT: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-unreachable-code
----------------------------------------------------------------------
Ran 15 tests in 0.334s

OK
EXIT: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants
----------------------------------------------------------------------
Ran 16 tests in 0.426s

OK
EXIT: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union
----------------------------------------------------------------------
Ran 286 tests in 5.146s

OK
/home/mei/projects/rustcc/tests/tests/chapter_18/valid/no_structure_parameters/libraries/initializers/nested_static_struct_initializers_client.s: Assembler messages:
/home/mei/projects/rustcc/tests/tests/chapter_18/valid/no_structure_parameters/libraries/initializers/nested_static_struct_initializers_client.s:17: Warning: value 0x1000000080000000 truncated to 0x80000000

/home/mei/projects/rustcc/tests/tests/chapter_18/valid/no_structure_parameters/libraries/initializers/static_struct_initializers_client.s: Assembler messages:
/home/mei/projects/rustcc/tests/tests/chapter_18/valid/no_structure_parameters/libraries/initializers/static_struct_initializers_client.s:9: Warning: value 0x400000005 truncated to 0x5
EXIT: 0

$ git diff --check
EXIT: 0
```

The chapter 18 union gate emitted assembler truncation warnings but exited 0; I record them as non-blocking warnings from the optional regression command.

## Findings by severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

1. **Non-DSE support edits are still sizeable and include a modified file above 250 pure LOC.**

   - `src/ir/copy_propagation/rewrite.rs` grew from 244 to 304 pure LOC, crossing the strict programming/code-smells ceiling.
   - `src/codegen/codegen.rs` remains a very large pre-existing file and grew by 121 pure LOC.
   - The changes appear targeted and official gates pass, but this should be treated as cleanup debt before further copy-prop/codegen expansion.

2. **DSE includes safe-looking but non-OCaml-extra known-memory store deletion.**

   - `src/ir/dead_store_elim/rewrite.rs:61-76` deletes known-base `Store`/`CopyBytes` when the base is not live, while OCaml `dead_store_elim.ml:97-104` never deletes `Store`.
   - The implementation keeps calls and opaque stores conservative, preserves globals/externs, and passes official gates plus targeted extern scalar/aggregate smoke checks. This remains a reasonable adversarial-gate focus area rather than a release blocker.

### LOW

1. `src/ir/dead_store_elim/util.rs` is near the warning band at 148 pure LOC but still under 250 and cohesive as DSE instruction utility logic.
2. `.omo/boulder.json` is modified in the worktree, but I did not inspect or edit it because the task is a code re-review and explicitly forbids Boulder edits.
3. The report artifact itself is the only file I intentionally wrote.

## Scope checklist

- Prior extern/global DSE bug fixed: PASS.
- DSE split under 250 pure LOC: PASS.
- No new `expect(` / `unwrap(` in Task 55 diff outside tests: PASS.
- No compiler-phase Rust unit tests added: PASS.
- Non-DSE support edits minimal enough for Task 55: PASS with WATCH.
- Official DSE gate: PASS.
- Official Chapter 19 default all-optimizations gate: PASS.
- Optional copy-prop/UCE/fold gates: PASS.
- Optional Chapter 18 union regression: PASS with assembler warnings.

## Recommendation

APPROVE for adversarial gate. No CRITICAL or HIGH findings remain from this re-review. The adversarial gate should focus on the WATCH areas: aggregate/global `CopyBytes`, copy-prop address-source rewriting, and known-memory DSE store deletion.
