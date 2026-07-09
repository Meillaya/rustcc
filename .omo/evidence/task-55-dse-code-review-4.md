VERDICT: REJECT

# Task 55 / W20-T5 copy-prop write-alias fix code review 4

Date: 2026-07-09
Repository: `/home/mei/projects/rustcc`
Role: independent code-quality reviewer; read-only except this report artifact.
Task: review whether the latest fix resolves the CRITICAL from `.omo/evidence/task-55-dse-code-review-3.md` while preserving previous Task 55 acceptance.

## Summary

The exact CRITICAL repro from review 3 is fixed: `/tmp/task55_copyprop_cross_block_write.c` now exits `9` for baseline, `--propagate-copies`, and all enabled optimizations. Official gates also pass, `rewrite.rs` and `rewrite_support.rs` are below 250 pure LOC, all DSE files are below 250 pure LOC, and the scalar/aggregate extern/global DSE probes still pass.

However, I cannot approve the latest fix. The new write-pointer protection is still not conservative enough: it protects addresses that flow into direct `Store`/`CopyBytes` write destinations, but it does not protect addresses that flow into pointer arguments of a `Call`. The existing copy-prop dataflow already treats calls as alias/memory clobbers after the call, so rewriting a pointer argument before a call can change the object the callee writes. I reproduced a valid C miscompile: baseline exits `9`, but `--propagate-copies` and all opts exit `1`.

`codeQualityStatus`: BLOCK
`recommendation`: REQUEST_CHANGES
`reportPath`: `/home/mei/projects/rustcc/.omo/evidence/task-55-dse-code-review-4.md`
`blockers`: B1 below (`copy_propagation/rewrite_support.rs` does not protect call pointer arguments that may write through aliases)

## Required skill-perspective check

- `omo:remove-ai-slops`: **ran** by loading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md`. Perspective result: **violated**. No deletion-only, tautological, implementation-mirroring, or removal-only tests were added; no new tests were added at all. The violation is production-support complexity that gives false confidence: the new conservative set is specifically for write alias protection, but omits a write-capable boundary (`Call` pointer args) that this compiler's own copy-prop transfer treats as aliasing.
- `omo:programming`: **ran** by loading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md`, `references/rust/README.md`, and `references/code-smells.md`. Perspective result: **violated** by behavior-changing Rust compiler code not locked by a regression and not conservative at a boundary. Diff-only scans found no new `unwrap`, `expect`, `unsafe`, tests, dependencies, bridge/system-C fingerprints, or Chapter 20 scope.

## Evidence inspected

Read/inspected:

```text
.omo/evidence/task-55-dse-code-review-3.md
.omo/evidence/task-55-copy-prop-write-alias-fix.txt
src/ir/copy_propagation.rs
src/ir/copy_propagation/facts.rs
src/ir/copy_propagation/rewrite.rs
src/ir/copy_propagation/rewrite_support.rs
src/ir/copy_propagation/dataflow.rs
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

Worktree shape:

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
?? src/ir/copy_propagation/rewrite_support.rs
?? src/ir/dead_store_elim/
?? .omo/evidence/task-55-*.txt/.md and older evidence files
```

I did not edit source, tests, plans, Boulder, or git history.

## Source review notes

- `src/ir/copy_propagation/rewrite.rs:17-23` computes `collect_write_pointers` once per function and passes the set into every block rewrite.
- `src/ir/copy_propagation/rewrite_support.rs:16-30` seeds protected write pointers only from `Store`/`CopyBytes` destination pointer operands, then records `Copy` and `AddPtr` source edges.
- `src/ir/copy_propagation/rewrite_support.rs:35-44` propagates protection backward through those edges to a fixed point.
- `src/ir/copy_propagation/rewrite_support.rs:179-184` blocks aggregate `GetAddress` source replacement only when the `GetAddress` destination is in that set.
- `src/ir/copy_propagation/rewrite_support.rs:174-178` still rewrites `Call` arguments with `replace_val`, but `collect_write_pointers` does not seed any call argument as write-capable.
- `src/ir/copy_propagation/dataflow.rs:187-203` treats calls as alias-sensitive after the call by filtering aliased/static copy facts, which is consistent with calls being possible memory writes.

## Required acceptance checks

### Official gates

All requested official gates were run and passed:

```text
$ cargo fmt --all -- --check
fmt exit=0

$ cargo check --release
Finished `release` profile [optimized] target(s) in 0.05s
check exit=0

$ cargo build --release
Finished `release` profile [optimized] target(s) in 0.01s
build exit=0

$ cargo test --release
10 passed; 0 failed; doc-tests 0 passed/0 failed
test exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
Ran 27 tests in 0.601s OK; exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
Ran 120 tests in 2.816s OK; exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --propagate-copies
Ran 42 tests in 1.004s OK; exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-unreachable-code
Ran 15 tests in 0.340s OK; exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants
Ran 16 tests in 0.419s OK; exit=0

$ ./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union
Ran 286 tests in 5.125s OK; exit=0
```

Chapter 18 union emitted the same non-blocking assembler truncation warnings noted by prior reviews for static initializer client `.s` files.

### Prior CRITICAL direct repro

Source:

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

Commands/results:

```text
$ ./target/release/rustcc -S /tmp/task55_copyprop_cross_block_write.c && gcc /tmp/task55_copyprop_cross_block_write.s -o /tmp/task55_copyprop_baseline && /tmp/task55_copyprop_baseline; echo "baseline exit=$?"
baseline exit=9

$ ./target/release/rustcc -S --propagate-copies /tmp/task55_copyprop_cross_block_write.c && gcc /tmp/task55_copyprop_cross_block_write.s -o /tmp/task55_copyprop_propagate && /tmp/task55_copyprop_propagate; echo "propagate exit=$?"
propagate exit=9

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_copyprop_cross_block_write.c && gcc /tmp/task55_copyprop_cross_block_write.s -o /tmp/task55_copyprop_all && /tmp/task55_copyprop_all; echo "all exit=$?"
all exit=9
```

TACKY under `--propagate-copies` keeps the address source as `main.dst.1` and rewrites only the store pointer to the protected address temp:

```text
137 GetAddress { src: "main.dst.1", dst: "tmp.10" }
141 Copy { src: Var("tmp.10"), dst: "main.p.2" }
153 Store { src: Constant(9), dst_pointer: Var("tmp.10") }
164 GetAddress { src: "main.dst.1", dst: "tmp.11" }
```

### DSE extern/global regressions

Scalar extern/global store remains intact:

```text
$ ./target/release/rustcc -S /tmp/task55_extern_store.c && gcc /tmp/task55_extern_store.s /tmp/task55_extern_client.c -o /tmp/task55_extern_baseline && /tmp/task55_extern_baseline; echo "baseline exit=$?"
baseline exit=5

$ ./target/release/rustcc -S --eliminate-dead-stores /tmp/task55_extern_store.c && gcc /tmp/task55_extern_store.s /tmp/task55_extern_client.c -o /tmp/task55_extern_dse && /tmp/task55_extern_dse; echo "dse exit=$?"
dse exit=5

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_extern_store.c && gcc /tmp/task55_extern_store.s /tmp/task55_extern_client.c -o /tmp/task55_extern_all && /tmp/task55_extern_all; echo "all exit=$?"
all exit=5
```

Aggregate extern/global store remains intact:

```text
$ ./target/release/rustcc -S /tmp/task55_extern_agg_store.c && gcc /tmp/task55_extern_agg_store.s /tmp/task55_extern_agg_client.c -o /tmp/task55_extern_agg_baseline && /tmp/task55_extern_agg_baseline; echo "baseline exit=$?"
baseline exit=7

$ ./target/release/rustcc -S --eliminate-dead-stores /tmp/task55_extern_agg_store.c && gcc /tmp/task55_extern_agg_store.s /tmp/task55_extern_agg_client.c -o /tmp/task55_extern_agg_dse && /tmp/task55_extern_agg_dse; echo "dse exit=$?"
dse exit=7

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_extern_agg_store.c && gcc /tmp/task55_extern_agg_store.s /tmp/task55_extern_agg_client.c -o /tmp/task55_extern_agg_all && /tmp/task55_extern_agg_all; echo "all exit=$?"
all exit=7
```

### LOC checks

```text
$ awk pure LOC on scoped files
src/ir/copy_propagation.rs                           74
src/ir/copy_propagation/rewrite.rs                   107
src/ir/copy_propagation/rewrite_support.rs           217
src/ir/copy_propagation/facts.rs                     178
src/ir/dead_store_elim/mod.rs                        61
src/ir/dead_store_elim/analysis.rs                   30
src/ir/dead_store_elim/liveness.rs                   113
src/ir/dead_store_elim/rewrite.rs                    84
src/ir/dead_store_elim/util.rs                       148
src/ir/opt.rs                                        44
src/pipeline.rs                                      95
src/codegen/codegen.rs                               2028
src/lex/scanner.rs                                   557
src/ir/mod.rs                                        13
```

Conclusion: the strict Task 55 split targets pass (`rewrite.rs`, `rewrite_support.rs`, and all DSE files are under 250 pure LOC). `codegen.rs` and `scanner.rs` remain oversized pre-existing files touched by this task's broader support work; prior reviews accepted them as outside the B1 split target, but they remain regression-risk surfaces.

### Diff hygiene and forbidden-scope scans

```text
$ git diff --check
git diff --check exit=0

$ rg -n '^\+.*\.(expect|unwrap)\s*\(' /tmp/task55_review_scoped_diff_current.txt
(no output)

$ rg -n '^\+.*unsafe' /tmp/task55_review_scoped_diff_current.txt
(no output)

$ rg -n '^\+.*#\[cfg\(test\)\]|^\+.*#\[test\]' /tmp/task55_review_scoped_diff_current.txt
(no output)

$ git diff -- tests Cargo.toml Cargo.lock
(no output)

$ rg -n 'bridge|system_c|SystemAssembly|compile_with_system|source_has|interpreter|evaluate_program' /tmp/task55_review_scoped_diff_current.txt
(no output)

$ rg -n 'regalloc|coalesc|interference|spill|register allocation|register-allocation|Chapter 20|chapter 20' /tmp/task55_review_scoped_diff_current.txt
(no output)
```

Scoped file scan finds existing `expect(...)` calls in `src/lex/scanner.rs`, but the diff-only scan confirms no new `unwrap`/`expect` was added.

## Findings by severity

### CRITICAL

#### B1. Copy-prop write-pointer protection still miscompiles writes through call pointer arguments

Files/lines:

- `src/ir/copy_propagation/rewrite_support.rs:16-30` only seeds write-protected pointer values from `Store` and `CopyBytes` destination pointer operands.
- `src/ir/copy_propagation/rewrite_support.rs:35-44` propagates protection backward through `Copy` and `AddPtr` to a fixed point, but only from those seeds.
- `src/ir/copy_propagation/rewrite_support.rs:174-178` rewrites `Call` arguments with copy propagation.
- `src/ir/copy_propagation/rewrite_support.rs:179-184` allows aggregate `GetAddress` source replacement unless the address temp is in the protected set.
- `src/ir/copy_propagation/dataflow.rs:187-203` already treats calls as alias-sensitive after the call, so a pointer argument to a call must be considered potentially write-capable before the call too.

Failing repro:

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

Commands/results:

```text
$ ./target/release/rustcc -S /tmp/task55_copyprop_call_write.c && gcc /tmp/task55_copyprop_call_write.s -o /tmp/task55_copyprop_call_baseline && /tmp/task55_copyprop_call_baseline; echo "baseline exit=$?"
baseline exit=9

$ ./target/release/rustcc -S --propagate-copies /tmp/task55_copyprop_call_write.c && gcc /tmp/task55_copyprop_call_write.s -o /tmp/task55_copyprop_call_propagate && /tmp/task55_copyprop_call_propagate; echo "propagate exit=$?"
propagate exit=1

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_copyprop_call_write.c && gcc /tmp/task55_copyprop_call_write.s -o /tmp/task55_copyprop_call_all && /tmp/task55_copyprop_call_all; echo "all exit=$?"
all exit=1
```

TACKY evidence under `--propagate-copies`:

```text
168 GetAddress { src: "main.src.0", dst: "tmp.10" }
172 Copy { src: Var("tmp.10"), dst: "main.p.2" }
178 Call { name: "write_int", args: [Var("tmp.10")], dst: Some("tmp.11") }
189 GetAddress { src: "main.dst.1", dst: "tmp.12" }
193 Load { src_pointer: Var("tmp.12"), dst: "tmp.13" }
199 Return(Var("tmp.13"))
```

Baseline TACKY for the same region uses `GetAddress { src: "main.dst.1", dst: "tmp.10" }` and passes `main.p.2` to `write_int`. The optimized TACKY instead passes a pointer into `main.src.0`, so the callee writes `src.a` and `dst.a` remains `1`.

Impact: observable wrong-code in valid C under `--propagate-copies` and all Chapter 19 optimizations. The latest fix resolves the reported direct `Store` repro but not the broader write-alias class it intended to conservatively protect.

Required before approval: either seed call pointer arguments (or otherwise all escaping pointer values) into the write-protection closure, or use a more conservative rule that blocks aggregate `GetAddress` replacement whenever the address may escape to a call. Re-run official gates plus direct store and call-write repros or add an official harness regression.

### HIGH

None separate from the CRITICAL blocker.

### MEDIUM

1. **Broader touched support surface remains risky without adversarial coverage.** `src/codegen/codegen.rs` and `src/lex/scanner.rs` are pre-existing oversized files touched by Task 55 support fixes. I did not find a new blocker in those hunks during this pass, and the requested gates pass, but the new call-write repro shows official Chapter 19 gates are still not covering all optimizer aliasing edges.

### LOW

1. The prior direct copy-prop write-alias repro now passes for baseline, `--propagate-copies`, and all opts.
2. `rewrite.rs` is 107 pure LOC and `rewrite_support.rs` is 217 pure LOC; both are below the 250 pure LOC ceiling.
3. DSE files remain below 250 pure LOC: `mod=61`, `analysis=30`, `liveness=113`, `rewrite=84`, `util=148`.
4. DSE extern/global scalar and aggregate fixes remain intact in manual repros.
5. Official fmt/check/build/test and Chapter 18/19 gates pass.
6. Diff-only hygiene scans found no new `unwrap`/`expect`, `unsafe`, Rust compiler-phase tests, dependencies, bridge/system-C fingerprints, or Chapter 20 scope.

## Final recommendation

REQUEST_CHANGES. The originally reported direct write-alias CRITICAL is fixed, but the new write-pointer protection is not conservative enough for pointer arguments passed to calls and still miscompiles valid C. Do not approve Task 55 until this aliasing class is protected and re-verified.
