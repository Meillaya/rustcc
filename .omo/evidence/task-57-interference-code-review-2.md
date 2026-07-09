VERDICT: REJECT

# Task 57 W21-T2 interference graph + simplification code review 2

Review date: 2026-07-09
Reviewer role: code-quality reviewer, read-only except this report.
Reviewed scope: current uncommitted Task 57 regalloc graph/simplify changes after gate-fix.

## Final recommendation

- `codeQualityStatus`: BLOCK
- `recommendation`: REQUEST_CHANGES
- `reportPath`: `.omo/evidence/task-57-interference-code-review-2.md`
- `blockers`: `cargo clippy --all-targets --all-features -- -D warnings` still exits 101. The failures I observed are outside the Task 57 regalloc files, but the requested success criteria included “official gates pass,” so I cannot approve while this required lint/static gate is red unless the project explicitly removes clippy from this task's official gate set.

Task-specific code review result: the prior Task 57 code blockers are otherwise resolved. The durable probe exists and runs, graph helper parameter bloat is fixed via named typed contexts, W21-T2 scope is respected, regalloc files are under 250 pure LOC, and no scoped unwrap/expect/unsafe/debug/test/dependency/bridge issue was found.

## Skill-perspective check

- `omo:remove-ai-slops`: loaded and applied. Result: no CRITICAL/HIGH slop found in scoped production code. No deletion-only, tautological, or implementation-constant-mirroring tests were added. The durable probe asserts observable graph/simplify behavior rather than only checking removals. No unnecessary production parsing/normalization/data extraction was introduced.
- `omo:programming`: loaded; Rust README and code-smells reference consulted. Result: graph helper functions now satisfy the >3-parameter rule through typed inputs (`InterferenceBuild`, `PseudoNodeContext`, `EdgeContext`). No scoped `unwrap`, `expect`, or `unsafe`. `src/codegen/regalloc/graph.rs` is in the 200-250 pure-LOC warning band but remains below the hard 250 ceiling.

## Changed files reviewed

Production/source:
- `src/codegen/regalloc/mod.rs`
- `src/codegen/regalloc/operands.rs`
- `src/codegen/regalloc/graph.rs` (untracked new file)
- `src/codegen/regalloc/simplify.rs` (untracked new file)
- Supporting current regalloc files: `src/codegen/regalloc/liveness.rs`, `src/codegen/regalloc/types.rs`

Evidence/probe:
- `.omo/evidence/task-57-interference-adversarial-verify.txt`
- `.omo/evidence/task-57-interference-fix.txt`
- `.omo/evidence/task-57-regalloc-probe.rs`
- Existing context checked: `.omo/evidence/task-57-interference-gate-review.md`, `.omo/evidence/task-57-interference-implementation.txt`, `.omx/notepad.md`, `.omo/start-work/ledger.jsonl`

Reference comparison:
- `nqcc2/lib/backend/regalloc.ml` lines 25-44, 89-149, 275-383, 470-516, 607-637
- `nqcc2/lib/optimizations/address_taken.ml` lines 1-17
- W21-T2 plan lines 1911-1924 in `.omo/plans/c-compiler-rust.md`

## Findings by severity

### CRITICAL

None.

### HIGH

1. Required lint/static gate is still red.
   - Evidence: `cargo clippy --all-targets --all-features -- -D warnings` exited 101.
   - Sample failures are outside Task 57 files: `src/ast/decl.rs:62`, `src/ast/expr.rs:35`, `src/codegen/assembly.rs:36`, `src/codegen/mod.rs:32`, `src/ir/lower.rs:441`, `src/semantics/typecheck.rs:108`.
   - Scoped check: `cargo clippy ... | grep -E 'src/codegen/regalloc/(graph|simplify|operands|mod)\.rs'` produced no matches, so I did not find a Task-57-local clippy error.
   - Why this blocks: the user explicitly asked to confirm “official gates pass.” This gate does not pass globally. If the project owner declares these clippy failures pre-existing/out-of-scope and not part of the Task 57 official gate, this finding can be downgraded; absent that, approval would overstate the evidence.

### MEDIUM

None.

### LOW

1. `src/codegen/regalloc/graph.rs` is close to the file-size warning band ceiling.
   - Evidence: pure LOC = 235, under the hard 250 ceiling but within the programming skill's 200-250 warning band.
   - Risk: W21-T3 coloring/select work could push this file over the ceiling unless selection/coloring is split by responsibility.
   - Recommendation: keep future coloring/select work out of `graph.rs` or split before adding substantial logic.

2. `src/codegen/regalloc/mod.rs:3-5` still describes W21-T1-only scope.
   - Evidence: the comment says coloring/spilling/coalescing are out of scope for W21-T1, while W21-T2 graph/simplify code now exists.
   - Risk: minor reader confusion only; not a functional blocker.

## Prior gate blocker resolution check

| Prior blocker / requested confirmation | Result |
| --- | --- |
| Durable `.omo/evidence/task-57-regalloc-probe.rs` exists and runs | PASS. File exists and `rustc --edition=2024 .omo/evidence/task-57-regalloc-probe.rs -o /tmp/task57-regalloc-probe-review && /tmp/task57-regalloc-probe-review` exits 0. |
| Graph helper parameter bloat fixed via typed context | PASS for W21-T2 graph helpers. Scan shows `build_interference` 1 param, `add_pseudo_nodes` 3, `add_edges` 3; `InterferenceBuild`, `PseudoNodeContext`, and `EdgeContext` are typed contexts. Pre-existing W21-T1 liveness helpers still have 4 params but are outside graph helper scope. |
| Behavior mirrors OCaml enough for W21-T2 | PASS. Rust mirrors OCaml operand extraction, hardreg base graph, pseudo filtering, move-source suppression, spill-cost counting, GP/XMM class split, and low-degree/spill-candidate simplification for this task's slice. Selection/coloring remains W21-T3 scope. |
| Scope W21-T2 only | PASS. `allocate` remains staged/unimplemented; no coloring/spill/coalescing implementation was introduced. |
| Files <250 pure LOC | PASS. `graph.rs` 235, `operands.rs` 152, `liveness.rs` 106, `types.rs` 93, `simplify.rs` 87, `mod.rs` 54. |
| No scoped unwrap/expect/unsafe/debug/tests/deps/bridge | PASS. Scoped greps found no unwrap/expect/unsafe/debug macros, no added Rust tests, no Cargo manifest diff, and no runtime bridge/process use. Mirror comments referencing OCaml are documentation only. |
| Official gates pass | REJECT as written because clippy exits 101 globally. Project/task gates other than clippy pass. |

## Source review notes

- `src/codegen/regalloc/graph.rs:34-41` exposes `InterferenceBuild` as a typed input object, resolving the previous long-parameter public API smell.
- `src/codegen/regalloc/graph.rs:143-198` keeps helper signatures to 1-3 parameters and separates pseudo-node and edge context.
- `src/codegen/regalloc/graph.rs:162-178` mirrors OCaml pseudo filtering: include only current register class, exclude configured static symbols and aliased pseudos.
- `src/codegen/regalloc/graph.rs:194-213` mirrors OCaml interference edge insertion: for each live-after operand, connect to written regs, suppress move source/destination self-copy interference, and rely on graph membership to ignore wrong-class/static/aliased nodes.
- `src/codegen/regalloc/simplify.rs:27-103` implements W21-T2 simplification stack over pseudo nodes with low-degree first and spill-candidate fallback by spill-cost/degree metric. This is appropriate for W21-T2; W21-T3 still owns select/coloring.
- `src/codegen/regalloc/operands.rs:34-69` adds explicit operand extraction equivalent to OCaml `get_operands`; implicit operands are intentionally excluded because this path discovers pseudos, while liveness/use-def handles implicit hardregs.

## Exact command evidence

```text
$ git status --short
 M src/codegen/regalloc/mod.rs
 M src/codegen/regalloc/operands.rs
?? .omo/evidence/task-57-regalloc-probe.rs
?? src/codegen/regalloc/graph.rs
?? src/codegen/regalloc/simplify.rs
```

```text
$ cargo fmt --all -- --check
exit status: 0
```

```text
$ cargo check --release
Finished `release` profile [optimized] target(s) in 0.03s
exit status: 0
```

```text
$ cargo build --release
Finished `release` profile [optimized] target(s) in 0.01s
exit status: 0
```

```text
$ cargo test --release
10 passed; 0 failed; doc-tests 0; exit status: 0
```

```text
$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
Ran 120 tests ... OK
exit status: 0
```

```text
$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
Ran 27 tests ... OK
exit status: 0
```

```text
$ ./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union
Ran 286 tests ... OK
exit status: 0
Note: assembler truncation warnings repeated from prior evidence; command still exited 0.
```

```text
$ git diff --check
exit status: 0
```

```text
$ rustc --edition=2024 .omo/evidence/task-57-regalloc-probe.rs -o /tmp/task57-regalloc-probe-review
exit status: 0

$ /tmp/task57-regalloc-probe-review
{
    "gp_hardregs": "12",
    "low_simplify": "[\"Pseudo(\\\"a\\\"):LowDegree:2\", \"Pseudo(\\\"b\\\"):LowDegree:1\", \"Pseudo(\\\"c\\\"):LowDegree:0\", \"Pseudo(\\\"d\\\"):LowDegree:1\", \"Pseudo(\\\"e\\\"):LowDegree:0\", \"Pseudo(\\\"h\\\"):LowDegree:1\", \"Pseudo(\\\"x\\\"):LowDegree:0\", \"Pseudo(\\\"y\\\"):LowDegree:0\"]",
    "pressure_simplify": "SpillCandidate:12",
    "pseudo_edges": "{\"a-b\", \"a-c\", \"b-c\", \"d-e\"}",
    "xmm_hardregs": "14",
}
exit status: 0

$ rm -f /tmp/task57-regalloc-probe-review && test ! -e /tmp/task57-regalloc-probe-review
exit status: 0
```

```text
$ cargo clippy --all-targets --all-features -- -D warnings
exit status: 101
error: could not compile `rustcc` (bin "rustcc") due to 31 previous errors
warning: build failed, waiting for other jobs to finish...
error: could not compile `rustcc` (bin "rustcc" test) due to 31 previous errors
```

```text
$ cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -E 'src/codegen/regalloc/(graph|simplify|operands|mod)\.rs' || true
(no output)
```

```text
$ for f in src/codegen/regalloc/*.rs; do awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' "$f" | wc -l; done
src/codegen/regalloc/graph.rs 235
src/codegen/regalloc/liveness.rs 106
src/codegen/regalloc/mod.rs 54
src/codegen/regalloc/operands.rs 152
src/codegen/regalloc/simplify.rs 87
src/codegen/regalloc/types.rs 93
```

```text
$ grep -RInE '\b(unwrap|expect)\s*\(|unsafe\b|dbg!\s*\(|println!\s*\(|panic!\s*\(' src/codegen/regalloc .omo/evidence/task-57-regalloc-probe.rs || true
(no output)

$ grep -RInE '#\[test\]|mod tests|cargo test|proptest|quickcheck' src/codegen/regalloc .omo/evidence/task-57-regalloc-probe.rs || true
(no output)

$ git diff --name-only -- Cargo.toml Cargo.lock
(no output)

$ grep -RInE 'std::process|Command::new|ocaml|nqcc2|\.ml|bridge|ffi' src/codegen/regalloc .omo/evidence/task-57-regalloc-probe.rs || true
Only mirror comments in regalloc source; no runtime bridge/process use.
```

## Conclusion

I would approve the Task 57 code shape and behavior under a “no new failures in scoped files” interpretation. I am rejecting this review under the stated criteria because a required official/static lint gate remains globally red. Fix or explicitly waive/re-scope that gate, then this should be re-reviewable as PASS unless new evidence changes.
