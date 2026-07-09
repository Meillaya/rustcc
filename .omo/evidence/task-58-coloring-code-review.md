VERDICT: PASS

# Task 58 W21-T3 coloring/select independent code review

Reviewer role: code quality reviewer, read-only except this report artifact.
Workspace: `/home/mei/projects/rustcc`
HEAD inspected: `7a4ae7434feb02ae477a5efbaa380b0c495bd03a`
Report path: `.omo/evidence/task-58-coloring-code-review.md`
Evidence inspected:
- `.omo/evidence/task-58-coloring-implementation.txt`
- `.omo/evidence/task-58-coloring-probe.rs`
- OCaml reference `nqcc2/lib/backend/regalloc.ml` / `.mli`
- `src/codegen/regalloc/*.rs`
Notepad path: not supplied in task input; no notepad artifact was used.

## Final recommendation

APPROVE with WATCH-level notes only. The Task 58 implementation satisfies the W21-T3 slice requested by the user: select/color API only, no spill rewrite/reallocation/coalescing/full allocation wiring, reserved registers excluded, durable probe present and independently rerun, and required gates pass.

## Skill-perspective check

Required skill perspectives ran before judging test relevance/maintainability:

- `omo:remove-ai-slops`: loaded `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md` completely. Applied overfit/slop lens to production and probe code. Result: no deletion-only/tautological tests, no implementation-mirroring official test additions, no unnecessary production parsing/normalization/data extraction. One LOW documentation slop note remains in `src/codegen/regalloc/mod.rs` (stale W21-T1 wording).
- `omo:programming`: loaded `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md`, `references/rust/README.md`, and `references/code-smells.md`. Applied Rust lens: no new deps, no unwrap/expect/unsafe/debug leftovers in changed regalloc/probe scope, all regalloc/probe files under 250 pure LOC, no brittle prompt tests, no untyped escape hatch, no needless abstraction beyond the small API result type.

## Diff and scope inspected

`git status --short` showed the Task 58 product files plus existing unrelated `.omo` untracked evidence:

```text
 M src/codegen/regalloc/mod.rs
?? .omo/evidence/task-58-coloring-implementation.txt
?? .omo/evidence/task-58-coloring-probe.rs
?? src/codegen/regalloc/color.rs
```

Relevant diff:

```diff
diff --git a/src/codegen/regalloc/mod.rs b/src/codegen/regalloc/mod.rs
@@
+mod color;
 mod graph;
 mod liveness;
 mod operands;
 mod simplify;
 mod types;

+pub use color::{ColorMap, SelectResult, select};
 pub use graph::{
```

New `src/codegen/regalloc/color.rs` was inspected in full. It adds `ColorMap`, `SelectResult`, and `select`, plus private color helpers only.

## Outcome checks against expected behavior

PASS — reverse simplification stack:
- `src/codegen/regalloc/color.rs:26` iterates `simplification.stack.iter().rev()`.

PASS — hard-register/precolored neighbor conflicts:
- `src/codegen/regalloc/color.rs:19-21` builds the hard-register color table from the class allocatable registers.
- `src/codegen/regalloc/color.rs:73-88` removes colors for `Operand::Reg` neighbors and already-colored `Operand::Pseudo` neighbors.

PASS — lowest available hardreg per class:
- `src/codegen/regalloc/color.rs:61-70` builds an ordered `BTreeSet` of available color indices and returns `.next()` mapped through `hardregs`, so selection is deterministic and lowest-indexed in the class list.

PASS — uncolorable/spill marker:
- If every color is removed, `available.into_iter().next()` is `None`, so `assignments` records `None` at `src/codegen/regalloc/color.rs:36`; the probe verifies this for `pressure`.

PASS — reserved regs remain unallocatable:
- `src/codegen/regalloc/types.rs:51-69` allocatable GP registers are `[AX, BX, CX, DX, DI, SI, R8, R9, R12, R13, R14, R15]`; R10/R11/SP/BP are absent.
- `src/codegen/regalloc/types.rs:68` allocatable XMM registers are `XMM(0)..=XMM(13)`; XMM14/XMM15 are absent.
- `src/codegen/regalloc/types.rs:87-105` `contains` mirrors the same reserved-register exclusion.

PASS — W21-T3-only scope:
- `src/codegen/regalloc/mod.rs:72-75` `allocate` remains the pre-existing W21 placeholder; there is no full allocation wiring.
- No spill rewrite/reallocation loop or coalescing was added in `src/codegen/regalloc/*.rs`.
- `Cargo.toml`/`Cargo.lock` have no diff.

PASS — durable probe exists/runs and covers the requested cases:
- `.omo/evidence/task-58-coloring-probe.rs:194-209` checks two interfering pseudos get different hardregs.
- `.omo/evidence/task-58-coloring-probe.rs:211-223` checks a hard-register AX neighbor blocks AX.
- `.omo/evidence/task-58-coloring-probe.rs:225-239` checks all GP hardregs block `pressure`, yielding `None`.
- `.omo/evidence/task-58-coloring-probe.rs:246-253` prints GP/XMM allocatable lists, proving reserved exclusions in the probe output.

PASS — no misleading success artifact issue:
- Implementation evidence path exists and was treated as untrusted; all key commands were independently rerun below.

## Findings by severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

None.

### LOW

1. `src/codegen/regalloc/mod.rs:3-5` contains stale wording: “Coloring, spilling, and coalescing remain intentionally out of scope for W21-T1.” After this diff, coloring is now in scope/implemented for W21-T3. This is comment slop under the remove-ai-slops lens, but it is non-functional and not a blocker.

2. `.omo/evidence/task-58-coloring-probe.rs` is 220 pure LOC. This is below the 250 LOC hard ceiling, but in the programming skill warning band. It remains acceptable for a standalone durable probe and is not production code.

3. Repo-wide strict clippy is not currently green due to pre-existing findings outside the Task 58 diff. This does not block this slice because required Task 58 gates and scoped hygiene pass, and `cargo clippy --all-targets --all-features -- -A warnings` succeeds.

## Verification command log

### Repository status / HEAD

Command:

```bash
pwd && git rev-parse HEAD && git status --short && printf '\n--- diff names ---\n' && git diff --name-only && printf '\n--- staged diff names ---\n' && git diff --cached --name-only
```

Result: exit 0

```text
/home/mei/projects/rustcc
7a4ae7434feb02ae477a5efbaa380b0c495bd03a
 M src/codegen/regalloc/mod.rs
?? .omo/evidence/task-18-adversarial-verify-2.txt
?? .omo/evidence/task-18-adversarial-verify-3.txt
?? .omo/evidence/task-18-adversarial-verify-4.txt
?? .omo/evidence/task-18-adversarial-verify.txt
?? .omo/evidence/task-18-gate-review.md
?? .omo/evidence/task-37-adversarial-verify-2.txt
?? .omo/evidence/task-37-adversarial-verify.txt
?? .omo/evidence/task-38-adversarial-verify.txt
?? .omo/evidence/task-39-adversarial-verify.txt
?? .omo/evidence/task-40-adversarial-verify-2.txt
?? .omo/evidence/task-40-adversarial-verify.txt
?? .omo/evidence/task-41-adversarial-verify.txt
?? .omo/evidence/task-58-coloring-implementation.txt
?? .omo/evidence/task-58-coloring-probe.rs
?? .omo/start-work/
?? src/codegen/regalloc/color.rs

--- diff names ---
src/codegen/regalloc/mod.rs

--- staged diff names ---
```

### Durable probe compile/run

Command:

```bash
rustc --edition=2024 -A dead_code .omo/evidence/task-58-coloring-probe.rs -o /tmp/task-58-coloring-probe
/tmp/task-58-coloring-probe
```

Result: exit 0

```text
{
    "gp_allocatable": "[AX, BX, CX, DX, DI, SI, R8, R9, R12, R13, R14, R15]",
    "hardreg_conflict": "{Pseudo(\"c\"): Some(BX)}",
    "small_graph": "{Pseudo(\"a\"): Some(BX), Pseudo(\"b\"): Some(AX)}",
    "spill_candidate": "{Pseudo(\"pressure\"): None}",
    "xmm_allocatable": "[XMM(0), XMM(1), XMM(2), XMM(3), XMM(4), XMM(5), XMM(6), XMM(7), XMM(8), XMM(9), XMM(10), XMM(11), XMM(12), XMM(13)]",
}
```

### Core Rust gates

Command:

```bash
cargo fmt --all -- --check && cargo check --release && cargo build --release && cargo test --release
```

Result: exit 0

```text
    Finished `release` profile [optimized] target(s) in 0.03s
    Finished `release` profile [optimized] target(s) in 0.01s
    Finished `release` profile [optimized] target(s) in 0.01s
     Running unittests src/lib.rs (target/release/deps/rustcc-41b78a55704c0e27)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/release/deps/rustcc-b48f2e14c29f3b0e)

running 10 tests
test compiler::tests::compiles_constant_return ... ok
test compiler::tests::compiles_expression_precedence ... ok
test compiler::tests::reaches_validate_through_pass_through_resolve ... ok
test compiler::tests::rejects_bad_lexeme ... ok
test driver::tests::derives_all_output_paths ... ok
test compiler::tests::handles_locals_and_assignment ... ok
test driver::tests::parses_artifact_and_feature_flags ... ok
test compiler::tests::parses_sizeof_expression_without_evaluating_it ... ok
test driver::tests::parses_default_run_stage ... ok
test driver::tests::parses_stage_flags_as_stdout_only ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests rustcc

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Official regression gates rerun from implementation evidence

Command:

```bash
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
```

Result: exit 0

```text
----------------------------------------------------------------------
Ran 120 tests in 2.933s

OK
```

Command:

```bash
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
```

Result: exit 0

```text
----------------------------------------------------------------------
Ran 27 tests in 0.640s

OK
```

Command:

```bash
./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union
```

Result: exit 0

```text
----------------------------------------------------------------------
Ran 286 tests in 5.037s

OK
/home/mei/projects/rustcc/tests/tests/chapter_18/valid/no_structure_parameters/libraries/initializers/nested_static_struct_initializers_client.s: Assembler messages:
/home/mei/projects/rustcc/tests/tests/chapter_18/valid/no_structure_parameters/libraries/initializers/nested_static_struct_initializers_client.s:17: Warning: value 0x1000000080000000 truncated to 0x80000000

/home/mei/projects/rustcc/tests/tests/chapter_18/valid/no_structure_parameters/libraries/initializers/static_struct_initializers_client.s: Assembler messages:
/home/mei/projects/rustcc/tests/tests/chapter_18/valid/no_structure_parameters/libraries/initializers/static_struct_initializers_client.s:9: Warning: value 0x400000005 truncated to 0x5
```

### Probe formatting, LOC, hygiene, deps, diff check

Command:

```bash
rustfmt --edition 2024 --check .omo/evidence/task-58-coloring-probe.rs
printf '%s\n' '--- pure LOC ---'
for f in src/codegen/regalloc/*.rs .omo/evidence/task-58-coloring-probe.rs; do printf '%s ' "$f"; awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(#|\/\/|--)/' "$f" | wc -l; done
printf '%s\n' '--- hygiene scan changed/probe ---'
rg -n "unwrap\(|expect\(|unsafe|dbg!|println!|eprintln!|TODO|FIXME|bridge|debug" src/codegen/regalloc/color.rs src/codegen/regalloc/mod.rs .omo/evidence/task-58-coloring-probe.rs || true
printf '%s\n' '--- dependency diff ---'
git diff -- Cargo.toml Cargo.lock
printf '%s\n' '--- git diff check ---'
git diff --check
```

Result: exit 0

```text
--- pure LOC ---
src/codegen/regalloc/color.rs 85
src/codegen/regalloc/graph.rs 235
src/codegen/regalloc/liveness.rs 106
src/codegen/regalloc/mod.rs 56
src/codegen/regalloc/operands.rs 152
src/codegen/regalloc/simplify.rs 87
src/codegen/regalloc/types.rs 93
.omo/evidence/task-58-coloring-probe.rs 220
--- hygiene scan changed/probe ---
--- dependency diff ---
--- git diff check ---
```

Command:

```bash
rg -n "unwrap\(|expect\(|unsafe|dbg!|println!|eprintln!|TODO|FIXME|bridge|debug|#\[test\]|\btests\b" src/codegen/regalloc/*.rs || true
```

Result: exit 0

```text
(no matches)
```

### Clippy context

Command:

```bash
cargo clippy --all-targets --all-features -- -A warnings
```

Result: exit 0

```text
    Checking rustcc v0.0.1 (/home/mei/projects/rustcc)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.08s
```

Command:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Result: exit 101 (non-blocking WATCH: failures are outside `src/codegen/regalloc/*` / Task 58 diff)

Representative output:

```text
error: doc list item overindented
  --> src/ast/decl.rs:62:5
error: variant name ends with the enum's name
  --> src/ast/expr.rs:35:5
error: methods called `is_*` usually take `self` by mutable reference or `self` by reference or no `self`
  --> src/ast/ty.rs:45:23
error: name `XMM` contains a capitalized acronym
  --> src/codegen/assembly.rs:36:5
error: module has the same name as its containing module
  --> src/codegen/mod.rs:32:1
error: could not compile `rustcc` (bin "rustcc") due to 31 previous errors
```

Command:

```bash
cargo clippy --all-targets --all-features -- -A warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::dbg_macro -D clippy::todo -D unsafe_code
```

Result: exit 101 (non-blocking WATCH: failures are pre-existing/outside `src/codegen/regalloc/*`; scoped regex scan of regalloc had no matches)

Representative output:

```text
error: used `unwrap()` on a `Result` value
   --> src/compiler.rs:172:25
error: used `expect()` on an `Option` value
   --> src/lex/scanner.rs:128:21
error: used `expect()` on an `Option` value
   --> src/semantics/resolve.rs:414:9
error: could not compile `rustcc` (bin "rustcc") due to 15 previous errors
error: could not compile `rustcc` (bin "rustcc" test) due to 29 previous errors
```

## Scope notes

- I did not run `--chapter 20 --latest-only --no-coalescing` as a pass/fail gate because the user explicitly scoped W21-T3 to select/color only and explicitly excluded spill rewrite/reallocation/coalescing/full allocation wiring. `allocate` intentionally remains a placeholder in this slice.
- No code, plan, docs, Boulder state, or git metadata was edited; only this report artifact was written.

## Blockers

None.
