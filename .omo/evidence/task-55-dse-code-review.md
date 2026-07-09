# Task 55 / W20-T5 Dead Store Elimination Code Review

VERDICT: REJECT

Date: 2026-07-08
Repository: `/home/mei/projects/rustcc`
Reviewed HEAD: `6bc6feb feat(compiler): chapter 19: copy propagation`
Review role: code quality reviewer (read-only except this report artifact)

## Skill-perspective check

- `omo:remove-ai-slops` perspective: **ran** by loading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md` completely. Result: **violated** by oversized new production module, added broad helper complexity, and out-of-scope non-DSE production edits.
- `omo:programming` perspective: **ran** by loading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md` and Rust reference `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md`; consulted code-smells file-size criteria. Result: **violated** by >250 pure LOC files touched/new and `expect()` in production code (`src/codegen/codegen.rs:2145-2148`).

## Required commands inspected/run

### Context and diff inspection

- `git status --short`
- `git diff --name-status`
- `git diff --stat`
- `sed -n '1830,1905p' .omo/plans/c-compiler-rust.md`
- `sed -n '72,136p' .omo/plans/c-compiler-rust.md`
- `sed -n '1,260p' .omo/evidence/task-55-dse-implementation.txt` and `sed -n '220,460p' .omo/evidence/task-55-dse-implementation.txt`
- `git diff -- src/codegen/codegen.rs src/ir/copy_propagation/facts.rs src/ir/copy_propagation/rewrite.rs src/ir/mod.rs src/ir/opt.rs src/lex/scanner.rs src/pipeline.rs`
- `git diff --no-index -- /dev/null src/ir/dead_store_elim.rs`
- `nl -ba` on changed source regions and OCaml references.

### OCaml references compared

- `nqcc2/lib/optimizations/dead_store_elim.ml`
- `nqcc2/lib/backward_dataflow.ml`
- `nqcc2/lib/optimizations/optimize_utils.ml`
- `nqcc2/lib/optimizations/address_taken.ml`
- `nqcc2/lib/optimizations/optimize.ml`

### Verification commands run

```text
$ cargo fmt --all -- --check
EXIT cargo fmt: 0

$ cargo check --release
Finished `release` profile [optimized] target(s) in 0.02s
EXIT cargo check: 0

$ git diff --check
EXIT git diff --check: 0

$ cargo test --release
running 10 tests
...
test result: ok. 10 passed; 0 failed
EXIT cargo test: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
Ran 27 tests in 0.622s
OK
EXIT dse gate: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
Ran 120 tests in 2.824s
OK
EXIT default ch19 gate: 0

$ rg -n "evaluate_program|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|source_has_" src
no-matches

$ rg -n "unsafe" src Cargo.toml
no matches

$ git diff -- tests Cargo.toml Cargo.lock
(no output)
```

### Targeted semantic repro run

```text
$ cat > /tmp/task55_extern_store.c
extern int g;
int set_g(void) {
    g = 5;
    return 0;
}

$ cat > /tmp/task55_extern_client.c
int g = 0;
int set_g(void);
int main(void) {
    set_g();
    return g;
}

$ ./target/release/rustcc --tacky --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_extern_store.c
TackyFunction {
    name: "set_g",
    body: [
        Return(Constant(0)),
    ],
    type_env: { "g": Int },
    ...
}

$ ./target/release/rustcc -S --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores /tmp/task55_extern_store.c \
  && gcc /tmp/task55_extern_store.s /tmp/task55_extern_client.c -o /tmp/task55_extern_repro \
  && /tmp/task55_extern_repro; echo $?
0

$ ./target/release/rustcc -S /tmp/task55_extern_store.c \
  && gcc /tmp/task55_extern_store.s /tmp/task55_extern_client.c -o /tmp/task55_extern_baseline \
  && /tmp/task55_extern_baseline; echo $?
5
```

## Findings by severity

### CRITICAL

1. **DSE changes observable behavior for valid extern-global programs.**

   - Rust code builds the DSE `static_vars` set only from emitted `program.static_variables` (`src/ir/dead_store_elim.rs:13-18`).
   - `lower_program` intentionally excludes file-scope `extern` declarations from `TackyProgram.static_variables` (`src/ir/lower.rs:319-331`) while still seeding function type environments with them.
   - DSE then treats `Copy { dst: "g", ... }` to an extern global as an ordinary dead local write and removes it when `g` is not locally live after the instruction (`src/ir/dead_store_elim.rs:145-150`).
   - The OCaml reference uses the global symbols table for static-storage variables (`nqcc2/lib/optimizations/dead_store_elim.ml:106-113`), not only variables emitted in the current translation unit.
   - Artifact-backed repro above: optimized linked program exits `0`; non-optimized baseline exits `5`.

   This violates the explicit review rule: do not approve if DSE can change observable behavior for a valid accepted program.

### HIGH

1. **New DSE module violates the 250 pure-LOC file-size/slop gate.**

   Measured with `awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' <file> | wc -l`:

   ```text
   src/ir/dead_store_elim.rs 400
   src/ir/copy_propagation/rewrite.rs 304
   src/codegen/codegen.rs 2029
   src/lex/scanner.rs 557
   ```

   The new `src/ir/dead_store_elim.rs` combines pass orchestration, liveness dataflow, transfer functions, rewrite logic, destination utilities, address-fact tracking, extra memory-store analysis, and return-copy collapsing in one 400 pure-LOC file. The executor's own evidence calls this out as a scoped exception, but the review policy says to be strict and that the new DSE file should preferably be split if >250 LOC. This is not a tight exception.

2. **Broad non-DSE edits are out-of-scope and behavior-risky despite green gates.**

   Task 55 asks for `dead_store_elim.ml` and default all-optimization wiring. The diff also changes:

   - `src/codegen/codegen.rs`: adds `collect_global_copy_dests`, `lower_copybytes_to_global`, a global CopyBytes lowering bypass, and return-constant typing (`src/codegen/codegen.rs:84-95`, `661-667`, `1915-1995`, `2125-2150`). This is backend behavior, not DSE, and it includes production `expect()` at `src/codegen/codegen.rs:2145-2148`.
   - `src/ir/copy_propagation/rewrite.rs`: adds write-pointer collection and address-source propagation for aggregate copies (`src/ir/copy_propagation/rewrite.rs:34-65`, `270-317`).
   - `src/lex/scanner.rs`: changes unsigned literal width classification (`src/lex/scanner.rs:151-163`, `272-284`).

   These may be latent bug fixes required to pass the current harness, but they are not justified by Task 55's DSE acceptance criteria and expand the blast radius substantially.

### MEDIUM

1. **DSE is not a direct OCaml mirror in several important areas.**

   The Rust pass adds `collapse_return_copies` (`src/ir/dead_store_elim.rs:81-95`) and known-memory `Store`/`CopyBytes` elimination (`src/ir/dead_store_elim.rs:123-169`, `275-305`) that are not present in `nqcc2/lib/optimizations/dead_store_elim.ml:97-130`, where `Store` is explicitly never eliminated. Some aggregate/local-memory cleanup may be semantically valid, but it increases the need for adversarial alias/global tests and contributes to the file-size/scope issue.

2. **Initial executor evidence's changed-file list is incomplete unless the later note is read.**

   `.omo/evidence/task-55-dse-implementation.txt` initially lists changed files but omits untracked `src/ir/dead_store_elim.rs`; a later note explains `git diff --name-only` omits untracked files. Reviewers must use `git status --short`, not the first changed-file section, to see the main new file.

### LOW

1. The required official gates do pass locally: format, check, whitespace, cargo tests, DSE harness, and default Chapter 19 harness are green.
2. DSE is wired through `src/ir/mod.rs:27`, `src/ir/opt.rs:17` and `src/ir/opt.rs:57-60`, and `src/pipeline.rs:91-93` for `--eliminate-dead-stores`.
3. Chapter 19 default-all behavior is supplied by the official harness: `tests/test_framework/runner.py:32-42` adds all four optimization flags when chapter >= 19 and no specific optimization is selected; `tests/test_framework/tacky/suite.py:103-151` builds all four optimization suites plus whole-pipeline tests.
4. `TackyFunction` metadata is preserved in DSE by replacing only `function.body` (`src/ir/dead_store_elim.rs:47-67`) and preserving `TackyProgram` static constants and function type metadata (`src/ir/dead_store_elim.rs:30-37`).
5. No compiler-phase Rust tests were added; `tests/`, `Cargo.toml`, and `Cargo.lock` diffs are empty. This complies with the plan's official-harness-only policy.
6. No forbidden bridge/interpreter fingerprints were found in `src/`; no `unsafe` was found; no regalloc/liveness/coalescing implementation diff was present.

## Scope checklist

- DSE wired for `--eliminate-dead-stores`: **yes**.
- Default Chapter 19 all-passes runs all four passes under harness: **yes**.
- TackyFunction metadata preserved: **yes**.
- No regalloc/liveness/coalescing/Wave21 implementation in this diff: **yes**.
- No test harness weakening or new dependencies: **yes**.
- No forbidden bridge/interpreter fingerprints in `src`: **yes**.
- No `unsafe`: **yes**.
- File-size/slop policy: **fail**.
- Semantic preservation for valid accepted programs: **fail** (extern-global store repro).

## Recommendation

REQUEST_CHANGES. Do not proceed to adversarial gate until:

1. DSE treats all static-storage variables visible in the translation unit, including `extern` file-scope globals, as live at function exit / across calls and loads as the OCaml symbols-table approach does.
2. Add an official-harness-style or temporary adversarial multi-translation-unit check for the extern-global repro before re-review.
3. Split `src/ir/dead_store_elim.rs` by responsibility to satisfy the >250 pure-LOC policy, or provide a genuinely tight exception that does not combine dataflow, rewrite, utility, and extra memory-store optimization in one new module.
4. Isolate or explicitly justify the non-DSE codegen/scanner/copy-prop changes with minimal repros; remove production `expect()` from `src/codegen/codegen.rs:2145-2148`.
