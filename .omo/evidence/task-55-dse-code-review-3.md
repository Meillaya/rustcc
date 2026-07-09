VERDICT: REJECT

# Task 55 / W20-T5 post-fix code review 3

Date: 2026-07-09
Repository: `/home/mei/projects/rustcc`
Role: independent code-quality reviewer; read-only except this report artifact.
Task: review current uncommitted Task 55 work after the copy-prop rewrite split.

## Summary

The previous gate blocker B1 is fixed directly: `src/ir/copy_propagation/rewrite.rs` is now 106 pure LOC and `src/ir/copy_propagation/rewrite_support.rs` is 203 pure LOC. The split is one-way (`rewrite.rs` imports private helpers from `rewrite_support.rs`) and the file boundary is cohesive enough: `rewrite.rs` owns CFG rewrite orchestration/redundant-copy deletion; `rewrite_support.rs` owns instruction source replacement plus write-pointer guarding.

However, the current Task 55 work still cannot be approved. While reviewing the split behavior, I found a semantic correctness bug in the extracted copy-prop support logic: copy propagation can rewrite the address source for a pointer that is later used for a write when that pointer flows through a `Copy`. This changes observable behavior for a valid C program under `--propagate-copies` and under all Chapter 19 optimizations, even though the official gates pass.

`codeQualityStatus`: BLOCK
`recommendation`: REQUEST_CHANGES
`reportPath`: `/home/mei/projects/rustcc/.omo/evidence/task-55-dse-code-review-3.md`
`blockers`: B1 below (`copy_propagation/rewrite_support.rs` write-pointer alias miscompile)

## Required skill-perspective check

- `omo:remove-ai-slops`: **ran** by loading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md`. Perspective result: **violated**. The LOC blocker is fixed, but the current support code creates false confidence: official tests pass while a narrow valid C behavior probe fails. This is not a deletion-only/tautological-test issue (no tests were added), but it is a missing behavior-coverage / production-support correctness issue.
- `omo:programming`: **ran** by loading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md`, `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md`, and `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/code-smells.md`. Perspective result: **violated** by behavior-changing Rust support code not locked by coverage. No new `unwrap`/`expect`/`unsafe` was found in the scoped diff.

## Artifacts inspected

Commands/read paths:

```text
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/code-smells.md
sed -n '1870,1905p' .omo/plans/c-compiler-rust.md
sed -n '1,260p' .omo/evidence/task-55-dse-adversarial-verify.txt
sed -n '1,260p' .omo/evidence/task-55-copy-prop-rewrite-split-fix.txt
sed -n '1,260p' .omo/evidence/task-55-dse-code-review.md
sed -n '1,300p' .omo/evidence/task-55-dse-code-review-2.md
sed -n '1,260p' .omo/evidence/task-55-dse-adversarial-verify-gate-review.md
```

Key artifact conclusions verified against source/commands, not trusted blindly:

- Gate rejection B1 was `src/ir/copy_propagation/rewrite.rs` at 304 pure LOC.
- Executor split evidence claimed `rewrite.rs=106`, `rewrite_support.rs=203` and all gates green.
- Prior PASS (`task-55-dse-code-review-2.md`) was superseded by the adversarial gate rejection because of B1.

## Source and LOC inspection

### Direct B1 LOC check

Command:

```bash
for f in \
  src/ir/copy_propagation.rs \
  src/ir/copy_propagation/rewrite.rs \
  src/ir/copy_propagation/rewrite_support.rs \
  src/ir/dead_store_elim/mod.rs \
  src/ir/dead_store_elim/analysis.rs \
  src/ir/dead_store_elim/liveness.rs \
  src/ir/dead_store_elim/rewrite.rs \
  src/ir/dead_store_elim/util.rs \
  src/ir/opt.rs src/pipeline.rs src/codegen/codegen.rs src/lex/scanner.rs src/ir/copy_propagation/facts.rs src/ir/mod.rs; do
  awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' "$f" | wc -l
done
```

Result:

```text
src/ir/copy_propagation.rs                         74
src/ir/copy_propagation/rewrite.rs                 106
src/ir/copy_propagation/rewrite_support.rs         203
src/ir/dead_store_elim/mod.rs                      61
src/ir/dead_store_elim/analysis.rs                 30
src/ir/dead_store_elim/liveness.rs                 113
src/ir/dead_store_elim/rewrite.rs                  84
src/ir/dead_store_elim/util.rs                     148
src/ir/opt.rs                                      44
src/pipeline.rs                                    95
src/codegen/codegen.rs                             2028
src/lex/scanner.rs                                 557
src/ir/copy_propagation/facts.rs                   178
src/ir/mod.rs                                      13
```

Conclusion: B1's strict file-size condition is fixed for `rewrite.rs` and the new support module. DSE files remain below 250 pure LOC. Existing large files (`codegen.rs`, `scanner.rs`) are still large but were not the B1 target.

### Split quality

Source reviewed:

```text
nl -ba src/ir/copy_propagation.rs
nl -ba src/ir/copy_propagation/rewrite.rs
nl -ba src/ir/copy_propagation/rewrite_support.rs
nl -ba src/ir/dead_store_elim/*.rs
```

Findings:

- `src/ir/copy_propagation.rs:12-16` wires `rewrite_support` as a private sibling module.
- `src/ir/copy_propagation/rewrite.rs:33-75` now owns CFG/block rewrite orchestration and delegates source replacement to support helpers.
- `src/ir/copy_propagation/rewrite_support.rs:30-209` owns instruction source replacement/address-source rewriting.
- There is no circular abstraction: `rewrite.rs` imports `rewrite_support`; `rewrite_support` imports only TACKY/facts types and does not call back into `rewrite.rs`.

The boundary is acceptable mechanically. The blocker below is not a file-boundary issue; it is a correctness issue in the current support logic.

## Official gates

All required official gates were run; none were skipped.

Command/results:

```text
$ cargo fmt --all -- --check
exit 0

$ cargo check --release
Finished `release` profile [optimized] target(s) in 0.03s
exit 0

$ cargo build --release
Finished `release` profile [optimized] target(s) in 0.01s
exit 0

$ cargo test --release
10 main tests passed; lib/doc tests 0 passed/0 failed
exit 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
Ran 27 tests in 0.610s OK
exit 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
Ran 120 tests in 2.796s OK
exit 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --propagate-copies
Ran 42 tests in 0.989s OK
exit 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-unreachable-code
Ran 15 tests in 0.325s OK
exit 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants
Ran 16 tests in 0.436s OK
exit 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union
Ran 286 tests in 5.105s OK
exit 0
Note: same non-blocking assembler truncation warnings as prior reviews for two chapter_18 static initializer client `.s` files.
```

Additional hygiene command:

```text
$ git diff --check
git diff --check: PASS
```

## DSE extern/global fix check

The previous DSE extern/global fix remains intact.

Probe sources:

```c
/* /tmp/task55_extern_store.c */
extern int g;
int set_g(void) { g = 5; return 0; }

/* /tmp/task55_extern_client.c */
int g = 0;
int set_g(void);
int main(void) { set_g(); return g; }
```

Commands/results:

```text
$ ./target/release/rustcc --tacky --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_extern_store.c
TACKY body includes Copy { src: Constant(5), dst: "g" } and Return(Constant(0)).

$ ./target/release/rustcc -S /tmp/task55_extern_store.c && gcc /tmp/task55_extern_store.s /tmp/task55_extern_client.c -o /tmp/task55_extern_baseline && /tmp/task55_extern_baseline
baseline=5

$ ./target/release/rustcc -S --eliminate-dead-stores /tmp/task55_extern_store.c && gcc /tmp/task55_extern_store.s /tmp/task55_extern_client.c -o /tmp/task55_extern_dse && /tmp/task55_extern_dse
dse=5

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_extern_store.c && gcc /tmp/task55_extern_store.s /tmp/task55_extern_client.c -o /tmp/task55_extern_all && /tmp/task55_extern_all
all=5
```

Aggregate extern probe:

```c
/* /tmp/task55_extern_agg_store.c */
struct Pair { int a; int b; };
extern struct Pair g;
int set_g(void) { struct Pair x = {3, 4}; g = x; return 0; }

/* /tmp/task55_extern_agg_client.c */
struct Pair { int a; int b; };
struct Pair g = {0, 0};
int set_g(void);
int main(void) { set_g(); return g.a + g.b; }
```

Commands/results:

```text
$ ./target/release/rustcc -S /tmp/task55_extern_agg_store.c && gcc /tmp/task55_extern_agg_store.s /tmp/task55_extern_agg_client.c -o /tmp/task55_extern_agg_baseline && /tmp/task55_extern_agg_baseline
baseline=7

$ ./target/release/rustcc -S --eliminate-dead-stores /tmp/task55_extern_agg_store.c && gcc /tmp/task55_extern_agg_store.s /tmp/task55_extern_agg_client.c -o /tmp/task55_extern_agg_dse && /tmp/task55_extern_agg_dse
dse=7

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_extern_agg_store.c && gcc /tmp/task55_extern_agg_store.s /tmp/task55_extern_agg_client.c -o /tmp/task55_extern_agg_all && /tmp/task55_extern_agg_all
all=7
```

Source support:

- `src/ir/dead_store_elim/analysis.rs:16-27` includes visible static-storage names from the function type environment.
- `src/ir/dead_store_elim/liveness.rs:53-68` keeps static vars live at CFG exit.

## Forbidden-scope scans

Scoped diff was built from tracked Task 55 files plus untracked `rewrite_support.rs` and `src/ir/dead_store_elim/*.rs`.

Commands/results:

```text
$ rg -n "\.(expect|unwrap)\s*\(" /tmp/task55_review_scoped_diff.txt
(no output)

$ rg -n "#\[cfg\(test\)\]|#\[test\]" /tmp/task55_review_scoped_diff.txt
(no output)

$ git diff -- tests Cargo.toml Cargo.lock
(no output)

$ rg -n "bridge|system_c|SystemAssembly|compile_with_system|source_has|interpreter|evaluate_program" /tmp/task55_review_scoped_diff.txt
(no output)

$ rg -n "regalloc|coalesc|interference|spill|register allocation|register-allocation|liveness" /tmp/task55_review_scoped_diff.txt
774:diff --git a/src/ir/dead_store_elim/liveness.rs b/src/ir/dead_store_elim/liveness.rs
778:+++ b/src/ir/dead_store_elim/liveness.rs
924:+mod liveness;
1001:+use super::liveness::{LiveBlock, find_live_variables};
1098:+use super::liveness::add_val;

$ rg -n "unsafe" /tmp/task55_review_scoped_diff.txt
(no output)
```

Conclusion: no new `unwrap`/`expect`, compiler-phase Rust tests, dependencies, bridge/system-C fingerprints, or Chapter 20 regalloc/coalescing scope were found. The only `liveness` hits are the Task 55 DSE liveness module.

## Findings by severity

### CRITICAL

#### B1. Copy-prop write-pointer guard is incomplete and miscompiles valid C when an address flows through `Copy`

Files/lines:

- `src/ir/copy_propagation/rewrite_support.rs:8-27` collects write pointers only from direct `Store`/`CopyBytes` destination pointer operands and `AddPtr` parents.
- `src/ir/copy_propagation/rewrite_support.rs:162-167` rewrites `GetAddress { src, dst }` unless `dst` itself is in that direct write-pointer set.
- `src/ir/copy_propagation/rewrite_support.rs:133-143` then rewrites `Store` / `CopyBytes` pointer operands through reaching copies.

The guard misses this common shape:

1. aggregate copy establishes a copy fact (`dst = src`),
2. `GetAddress dst -> tmp_ptr`,
3. `Copy tmp_ptr -> p`,
4. `Store ... -> p`.

Because `collect_write_pointers` records `p` but not the copied source `tmp_ptr`, `replace_address_source` is allowed to rewrite `GetAddress dst` into `GetAddress src`. Then the store pointer is also copy-propagated to `tmp_ptr`, so the write goes to `src` rather than `dst`.

Repro:

```c
/* /tmp/task55_copyprop_cross_block_write.c */
struct S { int a; int b; };
int main(void) {
    struct S src = {1, 2};
    struct S dst = {3, 4};
    dst = src;
    int *p = &dst.a;
    if (1) {
        *p = 9;
    }
    return dst.a;
}
```

Commands/results:

```text
$ ./target/release/rustcc -S /tmp/task55_copyprop_cross_block_write.c && gcc /tmp/task55_copyprop_cross_block_write.s -o /tmp/task55_copyprop_baseline && /tmp/task55_copyprop_baseline
baseline exit=9

$ ./target/release/rustcc -S --propagate-copies /tmp/task55_copyprop_cross_block_write.c && gcc /tmp/task55_copyprop_cross_block_write.s -o /tmp/task55_copyprop_prop && /tmp/task55_copyprop_prop
--propagate-copies exit=1

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_copyprop_cross_block_write.c && gcc /tmp/task55_copyprop_cross_block_write.s -o /tmp/task55_copyprop_all && /tmp/task55_copyprop_all
all-opts exit=1
```

TACKY evidence:

```text
Baseline TACKY:
150 GetAddress { src: "main.dst.1", dst: "tmp.10" }
154 Copy { src: Var("tmp.10"), dst: "main.p.2" }
166 Store { src: Constant(9), dst_pointer: Var("main.p.2") }
177 GetAddress { src: "main.dst.1", dst: "tmp.11" }

After --propagate-copies:
137 GetAddress { src: "main.src.0", dst: "tmp.10" }
141 Copy { src: Var("tmp.10"), dst: "main.p.2" }
153 Store { src: Constant(9), dst_pointer: Var("tmp.10") }
164 GetAddress { src: "main.dst.1", dst: "tmp.11" }
```

Impact: this is an observable wrong-code bug in the current uncommitted Task 55 work. It also invalidates the statement that the prior PASS remains valid after the rewrite split, because the current official gates do not cover this write-through-copied-pointer case.

Required fix before approval: make the write-pointer protection account for pointer aliases/copies (and CFG flow, if applicable), or avoid rewriting aggregate `GetAddress` sources when the address value may feed a write. Re-run all official gates plus this repro or an equivalent official-harness C test.

### HIGH

None separate from the CRITICAL blocker.

### MEDIUM

1. **Support edits remain broader than pure DSE/default wiring.**

   The B1 split is acceptable mechanically, and the non-DSE edits appear tied to Chapter 19 whole-pipeline behavior. Still, `src/codegen/codegen.rs` and `src/lex/scanner.rs` are large pre-existing files, and Task 55 now depends on copy-prop/codegen/scanner support changes. The CRITICAL copy-prop probe demonstrates why this broader support surface needs more adversarial coverage than official gates currently provide.

### LOW

1. B1 LOC blocker is fixed: `rewrite.rs=106`, `rewrite_support.rs=203` pure LOC.
2. DSE files remain below 250 pure LOC: `mod=61`, `analysis=30`, `liveness=113`, `rewrite=84`, `util=148`.
3. Official gates pass locally, including fmt/check/build/test, Chapter 19 DSE/default/copy-prop/UCE/fold, and Chapter 18 union.
4. DSE extern/global scalar and aggregate probes pass.
5. No new dependencies, compiler-phase Rust tests, `unwrap`/`expect`, `unsafe`, bridge/system-C fingerprints, or Chapter 20 regalloc/coalescing work were found.

## Final recommendation

REQUEST_CHANGES. Do not approve Task 55 until the copy-prop write-pointer alias miscompile is fixed and re-verified. The file-size split itself resolves the previous B1 size blocker, but the current work fails semantic preservation.
