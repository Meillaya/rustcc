VERDICT: PASS

# Task 55 DSE/copy-prop call-alias code review 5

Date: 2026-07-09
Repository: `/home/mei/projects/rustcc`
Role: independent code-quality reviewer; read-only except this report artifact.
Task: final review of latest Task 55 state after call-alias fix, specifically whether code-review-3 and code-review-4 blockers are resolved and whether the branch is safe for adversarial gate.

## Summary

`codeQualityStatus`: WATCH
`recommendation`: APPROVE
`reportPath`: `/home/mei/projects/rustcc/.omo/evidence/task-55-dse-code-review-5.md`
`blockers`: none

The latest state resolves both prior copy-prop alias blockers:

- code-review-3 direct store alias repro now exits `9` for baseline, `--propagate-copies`, and all requested optimizations.
- code-review-4 call pointer alias repro now exits `9` for baseline, `--propagate-copies`, and all requested optimizations.

The extern scalar and aggregate DSE/global probes also remain correct, and all requested official gates pass. I found no new `unwrap`/`expect`, `unsafe`, Rust compiler-phase tests, dependencies, bridge/system-C scope, or Chapter 20 scope in the scoped diff. `rewrite.rs`, `rewrite_support.rs`, and all DSE files are below the 250 pure-LOC ceiling.

I leave `WATCH` rather than `CLEAR` because Task 55 still touches very large pre-existing files (`src/codegen/codegen.rs`, `src/lex/scanner.rs`) and the alias repros remain temporary/manual rather than official harness regressions by task instruction. I do not consider that a blocker for this adversarial gate because the requested semantic repros and official gates pass.

## Required skill-perspective check

- `omo:remove-ai-slops`: **ran** by reading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md`. Perspective result: **no blocking violation found**. I found no deletion-only tests, removal-only tests, tautological tests, implementation-constant mirror tests, or unnecessary production parsing/normalization in the latest call-alias fix. The production support code is conservative alias protection for an observed wrong-code class, not speculative slop.
- `omo:programming`: **ran** by reading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md`, `references/rust/README.md`, and `references/code-smells.md`. Perspective result: **no blocking violation found** for the final fix. Diff-only scans found no new `unwrap`, `expect`, `unsafe`, compiler-phase tests, dependencies, bridge/system-C hooks, or Chapter 20 scope. The strict 250 pure-LOC target is satisfied for `rewrite.rs`, `rewrite_support.rs`, and the DSE files.

## Evidence inspected

Prior artifacts read:

```text
.omo/evidence/task-55-dse-code-review-4.md
.omo/evidence/task-55-copy-prop-call-alias-fix.txt
```

Source inspected directly/codegraph-assisted:

```text
src/ir/copy_propagation/rewrite.rs
src/ir/copy_propagation/rewrite_support.rs
src/ir/copy_propagation/dataflow.rs
src/ir/copy_propagation/facts.rs
src/ir/dead_store_elim/mod.rs
src/ir/dead_store_elim/analysis.rs
src/ir/dead_store_elim/liveness.rs
src/ir/dead_store_elim/rewrite.rs
src/ir/dead_store_elim/util.rs
src/ir/opt.rs
src/pipeline.rs
src/codegen/codegen.rs
src/lex/scanner.rs
```

Current worktree shape at review start:

```text
$ git status --short
 M .omo/boulder.json
 M src/codegen/codegen.rs
 M src/ir/copy_propagation.rs
 M src/ir/copy_propagation/facts.rs
 M src/ir/copy_propagation/rewrite.rs
 M src/ir/mod.rs
 M src/ir/opt.rs
 M src/lex/scanner.rs
 M src/pipeline.rs
?? .omo/evidence/task-55-copy-prop-call-alias-fix.txt
?? .omo/evidence/task-55-copy-prop-rewrite-split-fix.txt
?? .omo/evidence/task-55-copy-prop-write-alias-fix.txt
?? .omo/evidence/task-55-dse-code-review-*.md
?? .omo/evidence/task-55-dse-*.txt
?? src/ir/copy_propagation/rewrite_support.rs
?? src/ir/dead_store_elim/
```

I did not edit source, tests, plans, Boulder, or git history.

## Source review notes

- `src/ir/copy_propagation/rewrite.rs:17-23` computes `write_pointers` once from the annotated CFG and passes the set into every copy-prop rewrite block.
- `src/ir/copy_propagation/rewrite_support.rs:16-28` now seeds the protected pointer set from direct `Store` / `CopyBytes` destination pointers **and** from `Call` arguments.
- `src/ir/copy_propagation/rewrite_support.rs:29-50` propagates that protection backward through `Copy` and `AddPtr` pointer-source edges to a fixed point.
- `src/ir/copy_propagation/rewrite_support.rs:181-184` still rewrites call argument values, but `src/ir/copy_propagation/rewrite_support.rs:186-191` blocks aggregate `GetAddress` source replacement when the address temp is write-protected. The reproduced call-alias TACKY confirms the call receives the protected pointer to `main.dst.1`, not a pointer to `main.src.0`.
- `src/ir/copy_propagation/dataflow.rs:187-203` continues to treat calls as alias-sensitive copy-fact clobbers after the call.
- `src/ir/dead_store_elim/analysis.rs:16-27` and `src/ir/dead_store_elim/liveness.rs:53-68` still keep visible static-storage variables live for the extern/global DSE cases.

## Independent repros

All commands were run in `/home/mei/projects/rustcc` after `cargo build --release` completed successfully.

### Direct store alias repro

Source used:

```c
struct S { int a; int b; };
int main(void) {
    struct S src = {1, 2};
    struct S dst = {3, 4};
    dst = src;
    int *p = &dst.a;
    if (1) { *p = 9; }
    return dst.a;
}
```

Results:

```text
$ ./target/release/rustcc -S /tmp/task55_review5_direct_store.c && gcc /tmp/task55_review5_direct_store.s -o /tmp/task55_review5_direct_store_baseline && /tmp/task55_review5_direct_store_baseline
rustcc exit=0
gcc exit=0
program exit=9

$ ./target/release/rustcc -S --propagate-copies /tmp/task55_review5_direct_store.c && gcc /tmp/task55_review5_direct_store.s -o /tmp/task55_review5_direct_store_propagate && /tmp/task55_review5_direct_store_propagate
rustcc exit=0
gcc exit=0
program exit=9

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_review5_direct_store.c && gcc /tmp/task55_review5_direct_store.s -o /tmp/task55_review5_direct_store_all && /tmp/task55_review5_direct_store_all
rustcc exit=0
gcc exit=0
program exit=9
```

TACKY under `--propagate-copies` preserves the address source as `main.dst.1` and stores through that protected pointer:

```text
GetAddress { src: "main.dst.1", dst: "tmp.10" }
Copy { src: Var("tmp.10"), dst: "main.p.2" }
Store { src: Constant(9), dst_pointer: Var("tmp.10") }
GetAddress { src: "main.dst.1", dst: "tmp.11" }
```

### Call pointer alias repro

Source used:

```c
struct S { int a; int b; };
int write_int(int *p) { *p = 9; return 0; }
int main(void) {
    struct S src = {1, 2};
    struct S dst = {3, 4};
    dst = src;
    int *p = &dst.a;
    write_int(p);
    return dst.a;
}
```

Results:

```text
$ ./target/release/rustcc -S /tmp/task55_review5_call_alias.c && gcc /tmp/task55_review5_call_alias.s -o /tmp/task55_review5_call_alias_baseline && /tmp/task55_review5_call_alias_baseline
rustcc exit=0
gcc exit=0
program exit=9

$ ./target/release/rustcc -S --propagate-copies /tmp/task55_review5_call_alias.c && gcc /tmp/task55_review5_call_alias.s -o /tmp/task55_review5_call_alias_propagate && /tmp/task55_review5_call_alias_propagate
rustcc exit=0
gcc exit=0
program exit=9

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_review5_call_alias.c && gcc /tmp/task55_review5_call_alias.s -o /tmp/task55_review5_call_alias_all && /tmp/task55_review5_call_alias_all
rustcc exit=0
gcc exit=0
program exit=9
```

TACKY under `--propagate-copies` keeps the protected address pointing at `main.dst.1`; the call argument is rewritten to the protected temp, not to a source-object temp:

```text
GetAddress { src: "main.dst.1", dst: "tmp.10" }
Copy { src: Var("tmp.10"), dst: "main.p.2" }
Call { name: "write_int", args: [Var("tmp.10")], dst: Some("tmp.11") }
GetAddress { src: "main.dst.1", dst: "tmp.12" }
Return(Var("tmp.13"))
```

### Extern/global DSE probes

Scalar extern/global store:

```text
$ ./target/release/rustcc -S /tmp/task55_review5_extern_store.c && gcc /tmp/task55_review5_extern_store.s /tmp/task55_review5_extern_client.c -o /tmp/task55_review5_extern_scalar_baseline && /tmp/task55_review5_extern_scalar_baseline
rustcc exit=0
gcc exit=0
program exit=5

$ ./target/release/rustcc -S --eliminate-dead-stores /tmp/task55_review5_extern_store.c && gcc /tmp/task55_review5_extern_store.s /tmp/task55_review5_extern_client.c -o /tmp/task55_review5_extern_scalar_dse && /tmp/task55_review5_extern_scalar_dse
rustcc exit=0
gcc exit=0
program exit=5

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_review5_extern_store.c && gcc /tmp/task55_review5_extern_store.s /tmp/task55_review5_extern_client.c -o /tmp/task55_review5_extern_scalar_all && /tmp/task55_review5_extern_scalar_all
rustcc exit=0
gcc exit=0
program exit=5
```

All-opts TACKY preserves the scalar global write:

```text
Copy { src: Constant(5), dst: "g" }
Return(Constant(0))
```

Aggregate extern/global store:

```text
$ ./target/release/rustcc -S /tmp/task55_review5_extern_agg_store.c && gcc /tmp/task55_review5_extern_agg_store.s /tmp/task55_review5_extern_agg_client.c -o /tmp/task55_review5_extern_agg_baseline && /tmp/task55_review5_extern_agg_baseline
rustcc exit=0
gcc exit=0
program exit=7

$ ./target/release/rustcc -S --eliminate-dead-stores /tmp/task55_review5_extern_agg_store.c && gcc /tmp/task55_review5_extern_agg_store.s /tmp/task55_review5_extern_agg_client.c -o /tmp/task55_review5_extern_agg_dse && /tmp/task55_review5_extern_agg_dse
rustcc exit=0
gcc exit=0
program exit=7

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_review5_extern_agg_store.c && gcc /tmp/task55_review5_extern_agg_store.s /tmp/task55_review5_extern_agg_client.c -o /tmp/task55_review5_extern_agg_all && /tmp/task55_review5_extern_agg_all
rustcc exit=0
gcc exit=0
program exit=7
```

All-opts TACKY preserves the aggregate global copy:

```text
GetAddress { src: "g", dst: "tmp.3" }
GetAddress { src: "set_g.x.0", dst: "tmp.4" }
CopyBytes { src_pointer: Var("tmp.4"), dst_pointer: Var("tmp.3"), size: 8 }
Return(Constant(0))
```

## Official gates

```text
$ cargo fmt --all -- --check
exit=0

$ cargo check --release
Finished `release` profile [optimized] target(s) in 0.04s
exit=0

$ cargo build --release
Finished `release` profile [optimized] target(s) in 0.01s
exit=0

$ cargo test --release
10 passed; 0 failed; doc-tests 0 passed/0 failed
exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
Ran 27 tests in 0.600s
OK
exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
Ran 120 tests in 2.818s
OK
exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --propagate-copies
Ran 42 tests in 0.969s
OK
exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-unreachable-code
Ran 15 tests in 0.325s
OK
exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants
Ran 16 tests in 0.423s
OK
exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union
Ran 286 tests in 5.223s
OK
exit=0
```

The Chapter 18 union run emitted the same non-blocking assembler truncation warnings previously observed in client `.s` fixtures.

## LOC checks

```text
$ awk pure LOC on scoped files
src/ir/copy_propagation/rewrite.rs                      107
src/ir/copy_propagation/rewrite_support.rs              224
src/ir/dead_store_elim/mod.rs                           61
src/ir/dead_store_elim/analysis.rs                      30
src/ir/dead_store_elim/liveness.rs                      113
src/ir/dead_store_elim/rewrite.rs                       84
src/ir/dead_store_elim/util.rs                          148
src/codegen/codegen.rs                                  2029
src/lex/scanner.rs                                      557
```

Task-specific LOC requirements pass: `rewrite.rs`, `rewrite_support.rs`, and all DSE files are under 250 pure LOC. `codegen.rs` and `scanner.rs` remain oversized pre-existing files touched by Task 55 support work; I did not find a new blocker there in this final pass, and the requested semantic gates cover the touched behavior.

## Diff hygiene / forbidden-scope scans

Scoped diff included tracked Task 55 source changes plus untracked `src/ir/copy_propagation/rewrite_support.rs` and `src/ir/dead_store_elim/*.rs`.

```text
$ git diff --check
exit=0

$ rg -n '^\+.*\.(expect|unwrap)\s*\(' /tmp/task55_review5_scoped_diff.txt
(no output)

$ rg -n '^\+.*unsafe' /tmp/task55_review5_scoped_diff.txt
(no output)

$ rg -n '^\+.*#\[cfg\(test\)\]|^\+.*#\[test\]' /tmp/task55_review5_scoped_diff.txt
(no output)

$ git diff -- tests Cargo.toml Cargo.lock
(no output)

$ rg -n 'bridge|system_c|SystemAssembly|compile_with_system|source_has|interpreter|evaluate_program' /tmp/task55_review5_scoped_diff.txt
(no output)

$ rg -n 'regalloc|coalesc|interference|spill|register allocation|register-allocation|Chapter 20|chapter 20' /tmp/task55_review5_scoped_diff.txt
(no output)
```

## Findings by severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

1. **Manual alias repros are not official harness regressions.** The direct store and call pointer alias wrong-code cases are now fixed in direct evidence, but the task explicitly expected no new Rust compiler-phase tests, so future regressions depend on adversarial/manual gate coverage rather than an in-repo test. This is a release-process risk, not a current correctness blocker.
2. **Broad support surface remains in oversized pre-existing files.** `src/codegen/codegen.rs` and `src/lex/scanner.rs` remain far above 250 pure LOC and were touched by Task 55 support fixes. The requested gates and scalar/aggregate global probes pass, so I do not block this final Task 55 gate on that pre-existing architecture debt.

### LOW

1. The code-review-3 direct store alias blocker is resolved: baseline/propagate/all all exit `9`.
2. The code-review-4 call pointer alias blocker is resolved: baseline/propagate/all all exit `9`.
3. `rewrite.rs` is 107 pure LOC and `rewrite_support.rs` is 224 pure LOC.
4. DSE files are all below 250 pure LOC: `mod=61`, `analysis=30`, `liveness=113`, `rewrite=84`, `util=148`.
5. Extern scalar and aggregate DSE/global probes remain correct.
6. Official fmt/check/build/test and Chapter 18/19 gates pass.
7. No forbidden-scope additions were found in the scoped diff scans.

## Final recommendation

APPROVE for adversarial gate. The prior CRITICAL/HIGH blockers from code-review-3 and code-review-4 are resolved with independent compile/link/run evidence, official gates pass, and no new blocking code-quality issue was found.
