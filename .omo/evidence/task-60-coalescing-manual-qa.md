# Task60 Manual QA Matrix — W21-T5 Conservative Coalescing

Generated: 2026-07-09T05:53:50.432961 America/Toronto

Mode: manual QA executor; read-only product surface. Final artifact intentionally written at `.omo/evidence/task-60-coalescing-manual-qa.md`. No production code, tests, docs, or plan edits were made by this QA pass.

## Summary

- Overall verdict: PASS.
- Both compiler modes compile/link/run the required copy-heavy program from `/tmp` copies.
- Runtime behavior matches source semantics: expected exit code 1, actual exit code 1 for both modes.
- Allocator-relevant register-to-register mov-family moves, excluding frame setup/teardown: `--no-coalescing` = 1, default coalescing = 0; default is fewer.
- Chapter 20 latest-only harness passes in both modes.
- Scope checks pass: tests diff empty; no requested source path/chapter/test bridge hits in `src`; no R10/R11 allocatable hits in `src/codegen/regalloc/types.rs`.
- `/tmp` Task60 manual QA artifacts were cleaned and verified removed.

## manualQa.surfaceEvidence

| scenario id | criterion reference | surface | exact invocation | verdict | artifactRefs |
| --- | --- | --- | --- | --- | --- |
| S01 | git-diff-tests | shell | git diff -- tests | PASS | A01,A14 |
| S02 | fresh-release-build | shell | cargo build --release | PASS | A02,A17 |
| S03 | compile-no-coalescing | compiler CLI | ./target/release/rustcc --no-coalescing -S /tmp/task60-manual-qa/no/copy-heavy.c | PASS | A03,A21,A22 |
| S04 | compile-default-coalescing | compiler CLI | ./target/release/rustcc -S /tmp/task60-manual-qa/default/copy-heavy.c | PASS | A04,A21,A23 |
| S05 | link-no-coalescing | gcc linker | gcc /tmp/task60-manual-qa/no/copy-heavy.s -o /tmp/task60-manual-qa/no/copy-heavy | PASS | A05 |
| S06 | link-default | gcc linker | gcc /tmp/task60-manual-qa/default/copy-heavy.s -o /tmp/task60-manual-qa/default/copy-heavy | PASS | A06 |
| S07 | run-no-coalescing | native executable | /tmp/task60-manual-qa/no/copy-heavy | PASS expected=1 actual=1 | A07 |
| S08 | run-default | native executable | /tmp/task60-manual-qa/default/copy-heavy | PASS expected=1 actual=1 | A08 |
| S09 | move-count-no-coalescing | data-shaped shell parsing | python3 /tmp/task60-manual-qa/count_moves.py /tmp/task60-manual-qa/no/copy-heavy.s | PASS count=1 | A09,A22 |
| S10 | move-count-default | data-shaped shell parsing | python3 /tmp/task60-manual-qa/count_moves.py /tmp/task60-manual-qa/default/copy-heavy.s | PASS count=0 | A10,A23 |
| S11 | move-count-improvement | shell comparison | default count < --no-coalescing count | PASS 0 < 1 | A11 |
| S12 | chapter20-latest-no-coalescing | test harness | ./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing | PASS OK exit=0 | A12 |
| S13 | chapter20-latest-default | test harness | ./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only | PASS OK exit=0 | A13 |
| S14 | scope-no-bridge | rg source scan | rg -n "source_path_hint\|chapter_20\|chapter20\|test_compiler\|latest-only\|compile_with_system_cc_frontend\|evaluate_program\|SystemAssemblySanitizerOptions" src -S | PASS no hits exit=1 | A15 |
| S15 | scope-r10-r11 | rg/source inspection | sed -n "1,150p" src/codegen/regalloc/types.rs && rg -n "R10\|R11" src/codegen/regalloc/types.rs -S | PASS no R10/R11 hits | A16 |
| S16 | cleanup-receipt | shell cleanup | rm -rf /tmp/task60-manual-qa .omo/evidence/task60-manual-qa-artifacts && find /tmp -maxdepth 1 -name 'task60-manual-qa*' -print && test ! -e .omo/evidence/task60-manual-qa-artifacts | PASS exit=0 no stdout leftovers | A20 |

## manualQa.adversarialCases

| scenario id | criterion reference | adversarial class | expected behavior | verdict | artifactRefs |
| --- | --- | --- | --- | --- | --- |
| ADV01 | fresh-release-build | stale_state | Manual QA must use a release binary rebuilt in this run, not stale executor claims. | PASS cargo build --release exit=0; timestamp evidence captured. | A02,A17 |
| ADV02 | scope-no-tests-diff | dirty_worktree | Dirty implementation worktree is allowed, but tests must remain unmodified/untracked and this QA must not assume cleanliness. | PASS dirty state recorded; tests diff/untracked tests empty. | A01,A14,A18 |
| ADV03 | run-exit-semantics | misleading_success_output | copy-heavy program intentionally returns 1; nonzero executable exit is success only when expected=1 and actual=1. | PASS both modes actual exit=1. | A07,A08 |
| ADV04 | tmp-artifact-discipline | generated_artifacts | /tmp compiler/link outputs are real during QA but must be removed before completion. | PASS generated files listed before cleanup; cleanup command exit=0 and no /tmp task60 leftovers printed. | A19,A20 |
| ADV05 | command-fidelity | long_commands | Long commands must be recorded exactly, not summarized or dry-run. | PASS exact harness, rg, compiler, gcc, and cleanup invocations embedded with exit codes. | A01-A20 |
| ADV06 | input-fidelity | copy_heavy_input | Both modes must compile the required copy-heavy C input from isolated /tmp copies. | PASS source embedded; compile invocations target /tmp/no and /tmp/default copies. | A03,A04,A21 |
| ADV07 | scope-fidelity | scope_fidelity | Manual QA must not edit production code/tests/docs/plan and must verify no test diff plus no harness/path bridge or R10/R11 allocatable regression. | PASS only final artifact intentionally written; scope scans pass; no tests diff. | A14,A15,A16,A20 |

## manualQa.artifactRefs

| id | kind | description | path |
| --- | --- | --- | --- |
| A01 | embedded transcript | git diff -- tests scope check | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a01-git-diff-tests |
| A02 | embedded transcript | cargo build --release freshness build | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a02-cargo-build-release |
| A03 | embedded transcript | --no-coalescing compile from /tmp copy | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a03-compile-no-coalescing |
| A04 | embedded transcript | default coalescing compile from /tmp copy | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a04-compile-default |
| A05 | embedded transcript | gcc link for no-coalescing assembly | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a05-link-no-coalescing |
| A06 | embedded transcript | gcc link for default assembly | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a06-link-default |
| A07 | embedded transcript | run no-coalescing executable expected exit 1 | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a07-run-no-coalescing |
| A08 | embedded transcript | run default executable expected exit 1 | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a08-run-default |
| A09 | embedded transcript | no-coalescing allocator-relevant mov count | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a09-count-no-coalescing |
| A10 | embedded transcript | default allocator-relevant mov count | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a10-count-default |
| A11 | embedded transcript | mov-count comparison default fewer than no-coalescing | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a11-count-comparison |
| A12 | embedded transcript | chapter 20 latest-only no-coalescing harness | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a12-ch20-no-coalescing |
| A13 | embedded transcript | chapter 20 latest-only default harness | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a13-ch20-default |
| A14 | embedded transcript | scope check: tests diff empty | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a14-scope-tests-diff |
| A15 | embedded transcript | scope check: no source_path_hint/chapter_20/test bridge in src | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a15-scope-no-bridge |
| A16 | embedded transcript | scope check: R10/R11 not allocatable in regalloc/types.rs | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a16-scope-r10-r11 |
| A17 | embedded transcript | build artifact timestamps for stale-state check | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a17-build-freshness |
| A18 | embedded transcript | dirty worktree acknowledged while tests remain clean | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a18-dirty-worktree |
| A19 | embedded transcript | generated /tmp artifacts before cleanup | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a19-generated-artifacts-before-cleanup |
| A20 | embedded transcript | cleanup receipt proving /tmp task60 artifacts removed | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a20-cleanup-receipt |
| A21 | embedded input | copy-heavy input source used for both modes | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a21-copy-heavy-input |
| A22 | embedded assembly | no-coalescing assembly | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a22-no-coalescing-assembly |
| A23 | embedded assembly | default coalescing assembly | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a23-default-assembly |
| A24 | embedded inspection | prior move-count receipt and code review receipt inspected | /home/mei/projects/rustcc/.omo/evidence/task-60-coalescing-manual-qa.md#a24-existing-receipts-inspected |

## Embedded evidence transcripts


<a id="a01-git-diff-tests"></a>

### A01 git diff -- tests

```text
surface: shell
invocation: git diff -- tests
exit_code=0
```

<a id="a02-cargo-build-release"></a>

### A02 cargo build --release

```text
surface: shell
invocation: cargo build --release
    Finished `release` profile [optimized] target(s) in 0.05s
exit_code=0
```

<a id="a03-compile-no-coalescing"></a>

### A03 compile --no-coalescing

```text
surface: shell/compiler CLI
invocation: ./target/release/rustcc --no-coalescing -S /tmp/task60-manual-qa/no/copy-heavy.c
exit_code=0
produced:
total 8
-rw-r--r-- 1 mei mei 317 Jul  9 05:50 copy-heavy.c
-rw-r--r-- 1 mei mei 139 Jul  9 05:50 copy-heavy.s
```

<a id="a04-compile-default"></a>

### A04 compile default

```text
surface: shell/compiler CLI
invocation: ./target/release/rustcc -S /tmp/task60-manual-qa/default/copy-heavy.c
exit_code=0
produced:
total 8
-rw-r--r-- 1 mei mei 317 Jul  9 05:50 copy-heavy.c
-rw-r--r-- 1 mei mei 119 Jul  9 05:50 copy-heavy.s
```

<a id="a05-link-no-coalescing"></a>

### A05 link --no-coalescing

```text
surface: shell/linker
invocation: gcc /tmp/task60-manual-qa/no/copy-heavy.s -o /tmp/task60-manual-qa/no/copy-heavy
exit_code=0
```

<a id="a06-link-default"></a>

### A06 link default

```text
surface: shell/linker
invocation: gcc /tmp/task60-manual-qa/default/copy-heavy.s -o /tmp/task60-manual-qa/default/copy-heavy
exit_code=0
```

<a id="a07-run-no-coalescing"></a>

### A07 run --no-coalescing executable

```text
surface: shell/executable
invocation: /tmp/task60-manual-qa/no/copy-heavy
exit_code=1
expected_exit_code=1
```

<a id="a08-run-default"></a>

### A08 run default executable

```text
surface: shell/executable
invocation: /tmp/task60-manual-qa/default/copy-heavy
exit_code=1
expected_exit_code=1
```

<a id="a09-count-no-coalescing"></a>

### A09 count moves --no-coalescing

```text
surface: shell/data parsing
invocation: python3 /tmp/task60-manual-qa/count_moves.py /tmp/task60-manual-qa/no/copy-heavy.s
assembly=/tmp/task60-manual-qa/no/copy-heavy.s
allocator_relevant_register_to_register_mov_family_count=1
7: movl %r9d, %eax
exit_code=0
```

<a id="a10-count-default"></a>

### A10 count moves default

```text
surface: shell/data parsing
invocation: python3 /tmp/task60-manual-qa/count_moves.py /tmp/task60-manual-qa/default/copy-heavy.s
assembly=/tmp/task60-manual-qa/default/copy-heavy.s
allocator_relevant_register_to_register_mov_family_count=0
exit_code=0
```

<a id="a11-count-comparison"></a>

### A11 count comparison

```text
comparison=PASS default(0) < no-coalescing(1)
```

<a id="a12-ch20-no-coalescing"></a>

### A12 chapter 20 latest --no-coalescing

```text
surface: shell/test harness
invocation: ./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing
----------------------------------------------------------------------
Ran 66 tests in 3.121s

OK
exit_code=0
```

<a id="a13-ch20-default"></a>

### A13 chapter 20 latest default

```text
surface: shell/test harness
invocation: ./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only
----------------------------------------------------------------------
Ran 66 tests in 3.162s

OK
exit_code=0
```

<a id="a14-scope-tests-diff"></a>

### A14 scope tests diff

```text
surface: shell/scope check
invocation: git diff -- tests
exit_code=0
result=PASS empty diff
```

<a id="a15-scope-no-bridge"></a>

### A15 scope no source/chapter/test bridge

```text
surface: shell/scope check
invocation: rg -n "source_path_hint|chapter_20|chapter20|test_compiler|latest-only|compile_with_system_cc_frontend|evaluate_program|SystemAssemblySanitizerOptions" src -S
exit_code=1
result=PASS no bridge hits
```

<a id="a16-scope-r10-r11"></a>

### A16 scope R10/R11 allocatable

```text
surface: shell/scope check
invocation: sed -n "1,150p" src/codegen/regalloc/types.rs && rg -n "R10|R11" src/codegen/regalloc/types.rs -S
// Mirrors nqcc2/lib/backend/regalloc.ml:1-22 and :87-123.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::codegen::assembly::{Operand, Reg};

pub type LiveSet = BTreeSet<Operand>;
pub type LiveMap = BTreeMap<crate::ir::cfg::BlockId, BlockLiveness>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegisterClass {
    Gp,
    Xmm,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockLiveness {
    pub live_in: LiveSet,
    pub live_out: LiveSet,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LivenessConfig {
    pub return_regs: Vec<Reg>,
    pub call_param_regs: BTreeMap<String, Vec<Reg>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LivenessError {
    MissingCallMetadata { callee: String },
    PopInLiveness { reg: Reg },
}

impl fmt::Display for LivenessError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCallMetadata { callee } => {
                write!(out, "missing call metadata for callee '{callee}'")
            }
            Self::PopInLiveness { reg } => {
                write!(out, "pop reached liveness analysis for register {reg:?}")
            }
        }
    }
}

impl Error for LivenessError {}

impl RegisterClass {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gp => "general-purpose",
            Self::Xmm => "xmm",
        }
    }
    pub fn all_hardregs(self) -> Vec<Reg> {
        match self {
            Self::Gp => vec![
                Reg::AX,
                Reg::BX,
                Reg::CX,
                Reg::DX,
                Reg::DI,
                Reg::SI,
                Reg::R8,
                Reg::R9,
                Reg::R12,
                Reg::R13,
                Reg::R14,
                Reg::R15,
            ],
            Self::Xmm => (0..=13).map(Reg::XMM).collect(),
        }
    }

    pub fn caller_saved_regs(self) -> Vec<Reg> {
        match self {
            Self::Gp => vec![
                Reg::AX,
                Reg::CX,
                Reg::DX,
                Reg::DI,
                Reg::SI,
                Reg::R8,
                Reg::R9,
            ],
            Self::Xmm => (0..=13).map(Reg::XMM).collect(),
        }
    }

    pub fn contains(self, reg: &Reg) -> bool {
        match self {
            Self::Gp => matches!(
                reg,
                Reg::AX
                    | Reg::BX
                    | Reg::CX
                    | Reg::DX
                    | Reg::DI
                    | Reg::SI
                    | Reg::R8
                    | Reg::R9
                    | Reg::R12
                    | Reg::R13
                    | Reg::R14
                    | Reg::R15
            ),
            Self::Xmm => matches!(reg, Reg::XMM(0..=13)),
        }
    }
}

pub(crate) fn regs_to_operands(regs: &[Reg]) -> LiveSet {
    regs.iter().cloned().map(Operand::Reg).collect()
}
--- R10/R11 hits ---
rg_exit_code=1
result=PASS no R10/R11 allocatable hits
```

<a id="a17-build-freshness"></a>

### A17 build freshness timestamps

```text
surface: shell/freshness check
invocation: stat -c "%y %n" target/release/rustcc Cargo.toml Cargo.lock src/codegen/regalloc/coalesce.rs src/codegen/regalloc/types.rs
2026-07-09 05:38:49.299390531 -0400 target/release/rustcc
2026-03-18 03:53:44.817599740 -0400 Cargo.toml
2026-03-18 05:05:43.548489528 -0400 Cargo.lock
2026-07-09 05:34:17.735776441 -0400 src/codegen/regalloc/coalesce.rs
2026-07-09 04:28:31.523434958 -0400 src/codegen/regalloc/types.rs
exit_code=0
```

<a id="a18-dirty-worktree"></a>

### A18 dirty worktree acknowledgement

```text
surface: shell/adversarial dirty worktree
invocation: git status --short && git diff --name-only -- tests && git ls-files --others --exclude-standard tests
 M .omo/boulder.json
 M src/codegen/codegen.rs
 M src/codegen/fixup.rs
 M src/codegen/mod.rs
 M src/codegen/regalloc/allocate.rs
 M src/codegen/regalloc/graph.rs
 M src/codegen/regalloc/mod.rs
 M src/codegen/regalloc/rewrite.rs
 M src/codegen/replace_pseudos.rs
 M src/compiler.rs
 M src/driver.rs
 M src/pipeline.rs
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
?? .omo/evidence/task-60-coalescing-adversarial-verify-2.txt
?? .omo/evidence/task-60-coalescing-adversarial-verify.txt
?? .omo/evidence/task-60-coalescing-code-review-2.md
?? .omo/evidence/task-60-coalescing-code-review.md
?? .omo/evidence/task-60-coalescing-fix.txt
?? .omo/evidence/task-60-coalescing-implementation.txt
?? .omo/evidence/task-60-coalescing-move-count.txt
?? .omo/evidence/task-60-copy-heavy.c
?? .omo/evidence/task60-manual-qa-artifacts/
?? .omo/notepad.md
?? .omo/start-work/
?? src/codegen/fixup/
?? src/codegen/regalloc/abi_liveness.rs
?? src/codegen/regalloc/coalesce.rs
?? src/codegen/regalloc/division_copy.rs
?? src/codegen/regalloc/graph_pseudos.rs
?? src/codegen/replace_pseudos/
?? src/codegen/xmm.rs
--- git diff --name-only -- tests ---
--- untracked tests ---
exit_code=0
result=PASS dirty worktree acknowledged; tests scope clean
```

<a id="a19-generated-artifacts-before-cleanup"></a>

### A19 generated artifacts before cleanup

```text
surface: shell/adversarial generated artifacts before cleanup
invocation: find /tmp/task60-manual-qa -maxdepth 3 -type f -printf "%p %s bytes\\n" | sort
/tmp/task60-manual-qa/count_moves.py 655 bytes
/tmp/task60-manual-qa/default/copy-heavy 15752 bytes
/tmp/task60-manual-qa/default/copy-heavy.c 317 bytes
/tmp/task60-manual-qa/default/copy-heavy.s 119 bytes
/tmp/task60-manual-qa/logs/01-git-diff-tests.txt 57 bytes
/tmp/task60-manual-qa/logs/02-cargo-build-release.txt 123 bytes
/tmp/task60-manual-qa/logs/03-compile-no-coalescing.txt 253 bytes
/tmp/task60-manual-qa/logs/04-compile-default.txt 242 bytes
/tmp/task60-manual-qa/logs/05-link-no-coalescing.txt 127 bytes
/tmp/task60-manual-qa/logs/06-link-default.txt 137 bytes
/tmp/task60-manual-qa/logs/07-run-no-coalescing.txt 107 bytes
/tmp/task60-manual-qa/logs/08-run-default.txt 112 bytes
/tmp/task60-manual-qa/logs/09-count-no-coalescing.txt 260 bytes
/tmp/task60-manual-qa/logs/10-count-default.txt 251 bytes
/tmp/task60-manual-qa/logs/11-count-comparison.txt 46 bytes
/tmp/task60-manual-qa/logs/12-test-ch20-latest-no-coalescing.txt 239 bytes
/tmp/task60-manual-qa/logs/13-test-ch20-latest-default.txt 223 bytes
/tmp/task60-manual-qa/logs/14-scope-tests-diff.txt 92 bytes
/tmp/task60-manual-qa/logs/15-scope-no-source-path-bridge.txt 237 bytes
/tmp/task60-manual-qa/logs/16-scope-regalloc-types-r10-r11.txt 3218 bytes
/tmp/task60-manual-qa/logs/17-build-freshness.txt 465 bytes
/tmp/task60-manual-qa/logs/18-adversarial-dirty-worktree.txt 1914 bytes
/tmp/task60-manual-qa/logs/19-generated-artifacts-before-cleanup.txt 153 bytes
/tmp/task60-manual-qa/no/copy-heavy 15752 bytes
/tmp/task60-manual-qa/no/copy-heavy.c 317 bytes
/tmp/task60-manual-qa/no/copy-heavy.s 139 bytes
exit_code=0
```

<a id="a20-cleanup-receipt"></a>

### A20 cleanup receipt

```text
surface: shell/cleanup
invocation: rm -rf /tmp/task60-manual-qa .omo/evidence/task60-manual-qa-artifacts && find /tmp -maxdepth 1 -name 'task60-manual-qa*' -print && test ! -e .omo/evidence/task60-manual-qa-artifacts
stdout:
stderr:
exit_code=0
```

<a id="a21-copy-heavy-input"></a>

### A21 copy-heavy input source

```text
int main(void) {
    int a0 = 1;
    int a1 = a0;
    int a2 = a1;
    int a3 = a2;
    int a4 = a3;
    int a5 = a4;
    int a6 = a5;
    int a7 = a6;
    int a8 = a7;
    int a9 = a8;
    int a10 = a9;
    int a11 = a10;
    int a12 = a11;
    int a13 = a12;
    int a14 = a13;
    int a15 = a14;
    return a15;
}
```

<a id="a22-no-coalescing-assembly"></a>

### A22 no-coalescing assembly

```text
.text
.globl main
main:
    pushq %rbp
    movq %rsp, %rbp
    movl $1, %r9d
    movl %r9d, %eax
    movq %rbp, %rsp
    popq %rbp
    ret
```

<a id="a23-default-assembly"></a>

### A23 default assembly

```text
.text
.globl main
main:
    pushq %rbp
    movq %rsp, %rbp
    movl $1, %eax
    movq %rbp, %rsp
    popq %rbp
    ret
```

<a id="a24-existing-receipts-inspected"></a>

### A24 existing receipts inspected

```text
--- task-60-coalescing-move-count.txt ---
manual copy-heavy move-count QA
source: .omo/evidence/task-60-copy-heavy.c
no-coalescing assembly: .omo/evidence/task-60-copy-heavy.no-coalescing.s
coalescing assembly: .omo/evidence/task-60-copy-heavy.coalescing.s
no-coalescing register-to-register moves: 1
coalescing register-to-register moves: 0
result: PASS coalescing produced fewer moves

--- task-60-coalescing-code-review-2.md head ---
# Task 60 Coalescing Code Review 2

Verdict: APPROVE
codeQualityStatus: WATCH
recommendation: APPROVE
reviewed_at: 2026-07-09
workspace: `/home/mei/projects/rustcc`
mode: read-only review; wrote only this artifact

## Scope Reviewed

Re-reviewed current uncommitted Task60 after the cleanup fix, including the required evidence:

- `.omo/evidence/task-60-coalescing-fix.txt`
- `.omo/evidence/task-60-coalescing-code-review.md`
- `.omo/evidence/task-60-coalescing-adversarial-verify.txt`

Inspected current source/diff for:

- `src/codegen/regalloc/{allocate,coalesce,division_copy,abi_liveness,graph,graph_pseudos,rewrite,mod}.rs`
- `src/codegen/{fixup,fixup/split,replace_pseudos,replace_pseudos/split,replace_pseudos/move_split,xmm}.rs`
- `src/codegen/codegen.rs`, `src/driver.rs`, `src/compiler.rs`, `src/pipeline.rs`
- `nqcc2/lib/backend/regalloc.ml`

Skill-perspective check: ran. I loaded `omo:remove-ai-slops` and `omo:programming`, plus the Rust programming reference and code-smells reference. Result: the cleanup resolves the prior high-maintenance issues from those perspectives. No tests were changed, so there are no deletion-only, tautological, path-bridge, or implementation-mirroring tests to reject. The remaining concerns are recorded as risks, not blockers.

## Findings by Severity

### CRITICAL

None.

### HIGH

None.
```

## Cleanup and write-scope note

This QA pass used `/tmp/task60-manual-qa` for real compiler/link/run artifacts, then removed it. The only intended persistent file written by this pass is this manual QA matrix artifact.
