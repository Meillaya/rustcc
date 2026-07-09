# Task59 Manual QA Matrix — W21-T4 spill/re-allocation loop

Manual QA executor: Codex manual QA role
Repository: `/home/mei/projects/rustcc`
Run date: 2026-07-09 America/Toronto
Artifact path: `.omo/evidence/task-59-spill-loop-manual-qa.md`

## Scope and inspected inputs

Read-only QA, except this artifact. I inspected the required existing evidence before running scenarios:

- `.omo/evidence/task-59-spill-loop-implementation.txt` — present, non-empty; records implementation gates and previous probe evidence.
- `.omo/evidence/task-59-spill-loop-code-review.md` — present, non-empty; records approved Task59 code review with watch risks.
- `.omo/evidence/task-59-spill-loop-adversarial-verify.txt` — present, non-empty; records prior final-gate rejection due to missing manual QA artifact.
- `.omo/evidence/task-59-spill-loop-probe.c` — present, non-empty; provided high-pressure integer probe.

Preflight command evidence: `ls -l` and `wc -l` showed line counts 168, 181, 589, and 38 respectively for those four files; `git diff -- tests` was empty.

## Summary verdict

PASS for manual QA surface behavior. The release compiler built, the provided spill probe compiled/linked/ran with expected exit `16`, generated assembly contained `40` negative `%rbp` stack-slot references, independent high-pressure integer and double programs compiled/linked/ran with expected exit `0`, their generated assemblies contained stack-slot evidence (`104` int references, `53` double references), chapter 20 latest-only no-coalescing harness returned exit `0` and `OK`, and `/tmp` artifacts were removed with absence checks.

## manualQa

### surfaceEvidence

| scenario id | criterion reference | surface | exact invocation | expected outcome | actual outcome / exit code | verdict | artifactRefs |
|---|---|---|---|---|---|---|---|
| S0 | stale_state / compiler freshness | shell/cargo | `cargo build --release` | Release compiler exists and builds before manual QA. | `Finished release profile`; exit `0`. | PASS | A1, A6 |
| S1 | dirty_worktree / test-scope guard | shell/git | `git diff -- tests` | No production test changes; empty output; exit `0`. | Empty output; exit `0`. | PASS | A1, A6 |
| S2 | real compiler surface / provided probe compile | `rustcc` CLI | `./target/release/rustcc --no-coalescing -S .omo/evidence/task-59-spill-loop-probe.c` | Assembly generated without compiler failure. | No stderr/stdout; exit `0`; generated `.omo/evidence/task-59-spill-loop-probe.s`. | PASS | A1, A5, A6 |
| S3 | real link/run behavior / provided probe | `gcc` + executable | `gcc .omo/evidence/task-59-spill-loop-probe.s -o /tmp/task59_manual_probe && /tmp/task59_manual_probe; code=$?; echo program_exit=$code expected=16; test $code -eq 16` | Link succeeds; executable returns `16` (`528 mod 256`). | `program_exit=16 expected=16`; wrapper exit `0`. | PASS | A1, A5, A6 |
| S4 | stack spill inspection / provided probe | `rg` over generated assembly | `echo stack_ref_count=$(rg -o -- '-[0-9]+\(%rbp\)' .omo/evidence/task-59-spill-loop-probe.s <pipe> wc -l); rg -n -- '-[0-9]+\(%rbp\)' .omo/evidence/task-59-spill-loop-probe.s <pipe> head -40` | Negative `%rbp` stack slots are present. | `stack_ref_count=40`; sample includes `-4(%rbp)` through `-80(%rbp)` stores and loads. | PASS | A1, A5, A6 |
| S5 | independent high-pressure int input generation | shell generator | See raw transcript S5 for full long command creating `/tmp/task59_manual_int.c` with 64 live `int` locals. | Independent input is created under `/tmp`, not production tree. | `68 /tmp/task59_manual_int.c`; first/tail lines printed; exit `0`. | PASS | A1, A6 |
| S6 | independent high-pressure int compile/link/run | `rustcc` + `gcc` + executable | `./target/release/rustcc --no-coalescing -S /tmp/task59_manual_int.c && gcc /tmp/task59_manual_int.s -o /tmp/task59_manual_int && /tmp/task59_manual_int; code=$?; echo program_exit=$code expected=0; test $code -eq 0` | Program exits `0` when sum of 1..64 is 2080. | `program_exit=0 expected=0`; wrapper exit `0`. | PASS | A1, A6 |
| S7 | independent int stack spill inspection | `rg` over `/tmp` assembly | `echo stack_ref_count=$(rg -o -- '-[0-9]+\(%rbp\)' /tmp/task59_manual_int.s <pipe> wc -l); rg -n -- '-[0-9]+\(%rbp\)' /tmp/task59_manual_int.s <pipe> head -60` | High-pressure int program uses stack slots. | `stack_ref_count=104`; sample includes stores through at least `-208(%rbp)`. | PASS | A1, A6 |
| S8 | independent high-pressure double input generation | shell generator | See raw transcript S8 for full long command creating `/tmp/task59_manual_double.c` with 40 live `double` locals. | Independent floating-point input is created under `/tmp`, not production tree. | `44 /tmp/task59_manual_double.c`; first/tail lines printed; exit `0`. | PASS | A1, A6 |
| S9 | independent high-pressure double compile/link/run | `rustcc` + `gcc` + executable | `./target/release/rustcc --no-coalescing -S /tmp/task59_manual_double.c && gcc /tmp/task59_manual_double.s -o /tmp/task59_manual_double && /tmp/task59_manual_double; code=$?; echo program_exit=$code expected=0; test $code -eq 0` | Program exits `0` when sum of 1.0..40.0 is 820.0. | `program_exit=0 expected=0`; wrapper exit `0`. | PASS | A1, A6 |
| S10 | independent double stack spill inspection | `rg` over `/tmp` assembly | `echo stack_ref_count=$(rg -o -- '-[0-9]+\(%rbp\)' /tmp/task59_manual_double.s <pipe> wc -l); rg -n -- '-[0-9]+\(%rbp\)' /tmp/task59_manual_double.s <pipe> head -60; rg -n 'movsd\|addsd\|xmm' /tmp/task59_manual_double.s <pipe> head -60` | High-pressure double program uses stack slots and XMM operations. | `stack_ref_count=53`; sample includes `movsd %xmm15, -8(%rbp)` and `addsd -8(%rbp), %xmm14`. | PASS | A1, A6 |
| S11 | harness regression / misleading-success guard | test harness CLI | `./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing` | Harness exits `0` and prints `OK`. | `Ran 66 tests in 3.067s`; `OK`; exit `0`. | PASS | A1, A6 |
| S12 | generated `/tmp` artifact cleanup | shell cleanup | `rm -f /tmp/task59_manual_probe ...; for p in ...; do ...; done` | All manual `/tmp` artifacts absent after cleanup. | All seven paths printed `absent=...`; exit `0`. | PASS | A1, A6 |
| S13 | scope fidelity | shell/git | `git diff -- tests; git status --short .omo/evidence/task-59-spill-loop-manual-qa.md tests .omo/evidence/task-59-spill-loop-probe.s <pipe> cat` | Tests diff remains empty; no unexpected scoped status before writing this artifact. | Empty output; exit `0`. | PASS | A1, A6 |

Notes on table quoting: the `<pipe>` characters in S4/S7/S10 stand in for shell pipes in the summarized invocation only to avoid Markdown table ambiguity. The raw transcript below contains the exact invocations with literal `|` pipes as run.

### adversarialCases

| scenario id | criterion reference | adversarial class | expected behavior | verdict | artifactRefs |
|---|---|---|---|---|---|
| S0 | stale_state | Stale compiler binary could make QA pass old code. | `cargo build --release` must succeed immediately before real compiler scenarios. | PASS — build exit `0`. | A1, A6 |
| S1/S13 | dirty_worktree | Manual QA could hide test edits or drift. | `git diff -- tests` must be empty; no tests changes used as evidence. | PASS — empty diff both before and during scoped status check. | A1, A6 |
| S3/S6/S9/S11 | misleading_success_output | Shell output could claim success while process exits fail. | Commands must explicitly check executable exit codes and harness exit code. | PASS — wrappers used `test $code -eq ...`; harness command exit `0` and printed `OK`. | A1, A6 |
| S2/S12 | generated_artifacts | Compiler/linker outputs could pollute `/tmp` or evidence state. | `/tmp` binaries/sources/assemblies removed and absence verified. | PASS — all seven `/tmp/task59_manual_*` paths absent. Generated `.omo/evidence/task-59-spill-loop-probe.s` is an existing ignored assembly side effect of the required exact probe compile command. | A1, A5, A6 |
| S5/S8 | long_commands | Long shell generators for pressure cases could truncate or silently fail. | Full generator commands must run with exit `0` and print created source summaries. | PASS — int and double generators printed line counts and source heads/tails. | A1, A6 |
| S5-S10 | high_pressure_input | Allocation may pass small probe but fail many live GP/XMM values. | Independent int and double programs must compile, link, execute with expected exit, and show stack slots. | PASS — int exit `0`, stack refs `104`; double exit `0`, stack refs `53`. | A1, A6 |
| S13 | scope_fidelity | QA could edit production code/tests/docs/plan or stage/commit changes. | Only this manual QA artifact should be produced by this task; no staging/commit. | PASS with risk note — no production/test/doc/plan edits were made by QA. This artifact was written after S13; final status command below records the artifact as untracked. | A1, A6 |

### artifactRefs

| id | kind | description | path |
|---|---|---|---|
| A1 | manual QA artifact | This non-empty manual QA matrix and embedded transcript. | `/home/mei/projects/rustcc/.omo/evidence/task-59-spill-loop-manual-qa.md` |
| A2 | existing implementation evidence | Required inspected implementation evidence. | `/home/mei/projects/rustcc/.omo/evidence/task-59-spill-loop-implementation.txt` |
| A3 | existing code-review evidence | Required inspected code-review evidence. | `/home/mei/projects/rustcc/.omo/evidence/task-59-spill-loop-code-review.md` |
| A4 | existing adversarial verify evidence | Required inspected previous adversarial/final-gate evidence. | `/home/mei/projects/rustcc/.omo/evidence/task-59-spill-loop-adversarial-verify.txt` |
| A5 | generated assembly artifact | Assembly regenerated by the required exact probe compile command; ignored by git and already present before this QA run. | `/home/mei/projects/rustcc/.omo/evidence/task-59-spill-loop-probe.s` |
| A6 | embedded raw transcript | Raw command transcript embedded in this artifact under “Raw transcript”. | `/home/mei/projects/rustcc/.omo/evidence/task-59-spill-loop-manual-qa.md#raw-transcript` |

## Cleanup receipts

- `/tmp/task59_manual_probe` — absent.
- `/tmp/task59_manual_int.c` — absent.
- `/tmp/task59_manual_int.s` — absent.
- `/tmp/task59_manual_int` — absent.
- `/tmp/task59_manual_double.c` — absent.
- `/tmp/task59_manual_double.s` — absent.
- `/tmp/task59_manual_double` — absent.

The temporary runner script and transcript used to assemble this artifact were also removed after writing (see final verification note appended below).

## Risks and limitations

- The required exact command `./target/release/rustcc --no-coalescing -S .omo/evidence/task-59-spill-loop-probe.c` writes/regenerates `.omo/evidence/task-59-spill-loop-probe.s` next to the input. That file was already present and ignored before this QA run; it is referenced as generated evidence, not as a production/test/doc/plan edit.
- This manual QA exercises real compiler, assembler/linker, executable, and harness surfaces, but it is not a committed regression test.
- The independent double probe shows stack slots and XMM operations, including scratch-like `%xmm14/%xmm15` usage in generated assembly; the behavior passed execution and stack-slot checks.
- Existing repo state contains unrelated Task59 implementation/code-review evidence and ignored/generated files from previous work. This QA did not stage or commit anything.

## Raw transcript

```text
## SCENARIO S0
criterion: stale_state
surface: shell/cargo
expected: release compiler build succeeds
invocation: cargo build --release
--- output start ---
    Finished `release` profile [optimized] target(s) in 0.06s
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:00-04:00
ended_at: 2026-07-09T04:43:00-04:00

## SCENARIO S1
criterion: dirty_worktree
surface: shell/git
expected: no tests diff output; exit 0
invocation: git diff -- tests
--- output start ---
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:00-04:00
ended_at: 2026-07-09T04:43:00-04:00

## SCENARIO S2
criterion: probe_compile
surface: rustcc CLI
expected: compile provided probe to assembly; exit 0
invocation: ./target/release/rustcc --no-coalescing -S .omo/evidence/task-59-spill-loop-probe.c
--- output start ---
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:00-04:00
ended_at: 2026-07-09T04:43:00-04:00

## SCENARIO S3
criterion: probe_link_run
surface: gcc + executable
expected: link succeeds; executable exits 16
invocation: gcc .omo/evidence/task-59-spill-loop-probe.s -o /tmp/task59_manual_probe && /tmp/task59_manual_probe; code=$?; echo program_exit=$code expected=16; test $code -eq 16
--- output start ---
program_exit=16 expected=16
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:00-04:00
ended_at: 2026-07-09T04:43:00-04:00

## SCENARIO S4
criterion: probe_stack_slots
surface: rg over generated assembly
expected: negative rbp stack slots are present; record count/sample
invocation: echo stack_ref_count=$(rg -o -- '-[0-9]+\(%rbp\)' .omo/evidence/task-59-spill-loop-probe.s | wc -l); rg -n -- '-[0-9]+\(%rbp\)' .omo/evidence/task-59-spill-loop-probe.s | head -40
--- output start ---
stack_ref_count=40
14:    movl $2, -4(%rbp)
15:    movl $3, -8(%rbp)
16:    movl $4, -12(%rbp)
17:    movl $5, -16(%rbp)
18:    movl $6, -20(%rbp)
19:    movl $7, -24(%rbp)
20:    movl $8, -28(%rbp)
21:    movl $9, -32(%rbp)
22:    movl $10, -36(%rbp)
23:    movl $11, -40(%rbp)
24:    movl $12, -44(%rbp)
25:    movl $13, -48(%rbp)
26:    movl $14, -52(%rbp)
27:    movl $15, -56(%rbp)
28:    movl $16, -60(%rbp)
29:    movl $17, -64(%rbp)
30:    movl $18, -68(%rbp)
31:    movl $19, -72(%rbp)
32:    movl $20, -76(%rbp)
33:    movl $21, -80(%rbp)
45:    addl -4(%rbp), %r9d
46:    addl -8(%rbp), %r9d
47:    addl -12(%rbp), %r9d
48:    addl -16(%rbp), %r9d
49:    addl -20(%rbp), %r9d
50:    addl -24(%rbp), %r9d
51:    addl -28(%rbp), %r9d
52:    addl -32(%rbp), %r9d
53:    addl -36(%rbp), %r9d
54:    addl -40(%rbp), %r9d
55:    addl -44(%rbp), %r9d
56:    addl -48(%rbp), %r9d
57:    addl -52(%rbp), %r9d
58:    addl -56(%rbp), %r9d
59:    addl -60(%rbp), %r9d
60:    addl -64(%rbp), %r9d
61:    addl -68(%rbp), %r9d
62:    addl -72(%rbp), %r9d
63:    addl -76(%rbp), %r9d
64:    addl -80(%rbp), %r9d
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:00-04:00
ended_at: 2026-07-09T04:43:00-04:00

## SCENARIO S5
criterion: high_pressure_input
surface: shell generator
expected: create independent int high-pressure C program
invocation: rm -f /tmp/task59_manual_int.c /tmp/task59_manual_int.s /tmp/task59_manual_int; { echo 'int main(void) {'; for raw in $(seq -w 0 63); do n=$((10#$raw + 1)); echo "    int a$raw = $n;"; done; printf '    int sum = '; sep=''; for raw in $(seq -w 0 63); do printf '%sa%s' "$sep" "$raw"; sep=' + '; done; echo ';'; echo '    return sum == 2080 ? 0 : 1;'; echo '}'; } > /tmp/task59_manual_int.c; wc -l /tmp/task59_manual_int.c; sed -n '1,10p' /tmp/task59_manual_int.c; tail -5 /tmp/task59_manual_int.c
--- output start ---
68 /tmp/task59_manual_int.c
int main(void) {
    int a00 = 1;
    int a01 = 2;
    int a02 = 3;
    int a03 = 4;
    int a04 = 5;
    int a05 = 6;
    int a06 = 7;
    int a07 = 8;
    int a08 = 9;
    int a62 = 63;
    int a63 = 64;
    int sum = a00 + a01 + a02 + a03 + a04 + a05 + a06 + a07 + a08 + a09 + a10 + a11 + a12 + a13 + a14 + a15 + a16 + a17 + a18 + a19 + a20 + a21 + a22 + a23 + a24 + a25 + a26 + a27 + a28 + a29 + a30 + a31 + a32 + a33 + a34 + a35 + a36 + a37 + a38 + a39 + a40 + a41 + a42 + a43 + a44 + a45 + a46 + a47 + a48 + a49 + a50 + a51 + a52 + a53 + a54 + a55 + a56 + a57 + a58 + a59 + a60 + a61 + a62 + a63;
    return sum == 2080 ? 0 : 1;
}
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:00-04:00
ended_at: 2026-07-09T04:43:00-04:00

## SCENARIO S6
criterion: high_pressure_input
surface: rustcc/gcc/executable
expected: independent int pressure program compiles, links, exits 0
invocation: ./target/release/rustcc --no-coalescing -S /tmp/task59_manual_int.c && gcc /tmp/task59_manual_int.s -o /tmp/task59_manual_int && /tmp/task59_manual_int; code=$?; echo program_exit=$code expected=0; test $code -eq 0
--- output start ---
program_exit=0 expected=0
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:00-04:00
ended_at: 2026-07-09T04:43:00-04:00

## SCENARIO S7
criterion: high_pressure_input
surface: rg over independent int assembly
expected: negative rbp stack slots are present; record count/sample
invocation: echo stack_ref_count=$(rg -o -- '-[0-9]+\(%rbp\)' /tmp/task59_manual_int.s | wc -l); rg -n -- '-[0-9]+\(%rbp\)' /tmp/task59_manual_int.s | head -60
--- output start ---
stack_ref_count=104
14:    movl $2, -4(%rbp)
15:    movl $3, -8(%rbp)
16:    movl $4, -12(%rbp)
17:    movl $5, -16(%rbp)
18:    movl $6, -20(%rbp)
19:    movl $7, -24(%rbp)
20:    movl $8, -28(%rbp)
21:    movl $9, -32(%rbp)
22:    movl $10, -36(%rbp)
23:    movl $11, -40(%rbp)
24:    movl $12, -44(%rbp)
25:    movl $13, -48(%rbp)
26:    movl $14, -52(%rbp)
27:    movl $15, -56(%rbp)
28:    movl $16, -60(%rbp)
29:    movl $17, -64(%rbp)
30:    movl $18, -68(%rbp)
31:    movl $19, -72(%rbp)
32:    movl $20, -76(%rbp)
33:    movl $21, -80(%rbp)
34:    movl $22, -84(%rbp)
35:    movl $23, -88(%rbp)
36:    movl $24, -92(%rbp)
37:    movl $25, -96(%rbp)
38:    movl $26, -100(%rbp)
39:    movl $27, -104(%rbp)
40:    movl $28, -108(%rbp)
41:    movl $29, -112(%rbp)
42:    movl $30, -116(%rbp)
43:    movl $31, -120(%rbp)
44:    movl $32, -124(%rbp)
45:    movl $33, -128(%rbp)
46:    movl $34, -132(%rbp)
47:    movl $35, -136(%rbp)
48:    movl $36, -140(%rbp)
49:    movl $37, -144(%rbp)
50:    movl $38, -148(%rbp)
51:    movl $39, -152(%rbp)
52:    movl $40, -156(%rbp)
53:    movl $41, -160(%rbp)
54:    movl $42, -164(%rbp)
55:    movl $43, -168(%rbp)
56:    movl $44, -172(%rbp)
57:    movl $45, -176(%rbp)
58:    movl $46, -180(%rbp)
59:    movl $47, -184(%rbp)
60:    movl $48, -188(%rbp)
61:    movl $49, -192(%rbp)
62:    movl $50, -196(%rbp)
63:    movl $51, -200(%rbp)
64:    movl $52, -204(%rbp)
65:    movl $53, -208(%rbp)
77:    addl -4(%rbp), %r9d
78:    addl -8(%rbp), %r9d
79:    addl -12(%rbp), %r9d
80:    addl -16(%rbp), %r9d
81:    addl -20(%rbp), %r9d
82:    addl -24(%rbp), %r9d
83:    addl -28(%rbp), %r9d
84:    addl -32(%rbp), %r9d
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:00-04:00
ended_at: 2026-07-09T04:43:00-04:00

## SCENARIO S8
criterion: high_pressure_input
surface: shell generator
expected: create independent double high-pressure C program
invocation: rm -f /tmp/task59_manual_double.c /tmp/task59_manual_double.s /tmp/task59_manual_double; { echo 'int main(void) {'; for raw in $(seq -w 0 39); do n=$((10#$raw + 1)); echo "    double d$raw = $n.0;"; done; printf '    double sum = '; sep=''; for raw in $(seq -w 0 39); do printf '%sd%s' "$sep" "$raw"; sep=' + '; done; echo ';'; echo '    return sum == 820.0 ? 0 : 1;'; echo '}'; } > /tmp/task59_manual_double.c; wc -l /tmp/task59_manual_double.c; sed -n '1,10p' /tmp/task59_manual_double.c; tail -5 /tmp/task59_manual_double.c
--- output start ---
44 /tmp/task59_manual_double.c
int main(void) {
    double d00 = 1.0;
    double d01 = 2.0;
    double d02 = 3.0;
    double d03 = 4.0;
    double d04 = 5.0;
    double d05 = 6.0;
    double d06 = 7.0;
    double d07 = 8.0;
    double d08 = 9.0;
    double d38 = 39.0;
    double d39 = 40.0;
    double sum = d00 + d01 + d02 + d03 + d04 + d05 + d06 + d07 + d08 + d09 + d10 + d11 + d12 + d13 + d14 + d15 + d16 + d17 + d18 + d19 + d20 + d21 + d22 + d23 + d24 + d25 + d26 + d27 + d28 + d29 + d30 + d31 + d32 + d33 + d34 + d35 + d36 + d37 + d38 + d39;
    return sum == 820.0 ? 0 : 1;
}
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:00-04:00
ended_at: 2026-07-09T04:43:01-04:00

## SCENARIO S9
criterion: high_pressure_input
surface: rustcc/gcc/executable
expected: independent double pressure program compiles, links, exits 0
invocation: ./target/release/rustcc --no-coalescing -S /tmp/task59_manual_double.c && gcc /tmp/task59_manual_double.s -o /tmp/task59_manual_double && /tmp/task59_manual_double; code=$?; echo program_exit=$code expected=0; test $code -eq 0
--- output start ---
program_exit=0 expected=0
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:01-04:00
ended_at: 2026-07-09T04:43:01-04:00

## SCENARIO S10
criterion: high_pressure_input
surface: rg over independent double assembly
expected: negative rbp stack slots and XMM ops are present; record count/sample
invocation: echo stack_ref_count=$(rg -o -- '-[0-9]+\(%rbp\)' /tmp/task59_manual_double.s | wc -l); rg -n -- '-[0-9]+\(%rbp\)' /tmp/task59_manual_double.s | head -60; echo '-- xmm sample --'; rg -n 'movsd|addsd|xmm' /tmp/task59_manual_double.s | head -60
--- output start ---
stack_ref_count=53
9:    movsd %xmm15, -8(%rbp)
11:    movsd %xmm15, -16(%rbp)
13:    movsd %xmm15, -24(%rbp)
15:    movsd %xmm15, -32(%rbp)
17:    movsd %xmm15, -40(%rbp)
19:    movsd %xmm15, -48(%rbp)
21:    movsd %xmm15, -56(%rbp)
23:    movsd %xmm15, -64(%rbp)
25:    movsd %xmm15, -72(%rbp)
27:    movsd %xmm15, -80(%rbp)
29:    movsd %xmm15, -88(%rbp)
31:    movsd %xmm15, -96(%rbp)
33:    movsd %xmm15, -104(%rbp)
35:    movsd %xmm15, -112(%rbp)
37:    movsd %xmm15, -120(%rbp)
39:    movsd %xmm15, -128(%rbp)
41:    movsd %xmm15, -136(%rbp)
43:    movsd %xmm15, -144(%rbp)
45:    movsd %xmm15, -152(%rbp)
47:    movsd %xmm15, -160(%rbp)
49:    movsd %xmm15, -168(%rbp)
51:    movsd %xmm15, -176(%rbp)
53:    movsd %xmm15, -184(%rbp)
55:    movsd %xmm15, -192(%rbp)
57:    movsd %xmm15, -200(%rbp)
59:    movsd %xmm15, -208(%rbp)
74:    addsd -8(%rbp), %xmm14
77:    addsd -16(%rbp), %xmm14
80:    addsd -24(%rbp), %xmm14
83:    addsd -32(%rbp), %xmm14
86:    addsd -40(%rbp), %xmm14
89:    addsd -48(%rbp), %xmm14
92:    addsd -56(%rbp), %xmm14
95:    addsd -64(%rbp), %xmm14
98:    addsd -72(%rbp), %xmm14
101:    addsd -80(%rbp), %xmm14
104:    addsd -88(%rbp), %xmm14
107:    addsd -96(%rbp), %xmm14
110:    addsd -104(%rbp), %xmm14
113:    addsd -112(%rbp), %xmm14
116:    addsd -120(%rbp), %xmm14
119:    addsd -128(%rbp), %xmm14
122:    addsd -136(%rbp), %xmm14
125:    addsd -144(%rbp), %xmm14
128:    addsd -152(%rbp), %xmm14
131:    addsd -160(%rbp), %xmm14
134:    addsd -168(%rbp), %xmm14
137:    addsd -176(%rbp), %xmm14
140:    addsd -184(%rbp), %xmm14
143:    addsd -192(%rbp), %xmm14
146:    addsd -200(%rbp), %xmm14
149:    addsd -208(%rbp), %xmm14
190:    movl -216(%rbp), %r9d
-- xmm sample --
7:    movsd dbl.0(%rip), %xmm13
8:    movsd dbl.1(%rip), %xmm15
9:    movsd %xmm15, -8(%rbp)
10:    movsd dbl.2(%rip), %xmm15
11:    movsd %xmm15, -16(%rbp)
12:    movsd dbl.3(%rip), %xmm15
13:    movsd %xmm15, -24(%rbp)
14:    movsd dbl.4(%rip), %xmm15
15:    movsd %xmm15, -32(%rbp)
16:    movsd dbl.5(%rip), %xmm15
17:    movsd %xmm15, -40(%rbp)
18:    movsd dbl.6(%rip), %xmm15
19:    movsd %xmm15, -48(%rbp)
20:    movsd dbl.7(%rip), %xmm15
21:    movsd %xmm15, -56(%rbp)
22:    movsd dbl.8(%rip), %xmm15
23:    movsd %xmm15, -64(%rbp)
24:    movsd dbl.9(%rip), %xmm15
25:    movsd %xmm15, -72(%rbp)
26:    movsd dbl.10(%rip), %xmm15
27:    movsd %xmm15, -80(%rbp)
28:    movsd dbl.11(%rip), %xmm15
29:    movsd %xmm15, -88(%rbp)
30:    movsd dbl.12(%rip), %xmm15
31:    movsd %xmm15, -96(%rbp)
32:    movsd dbl.13(%rip), %xmm15
33:    movsd %xmm15, -104(%rbp)
34:    movsd dbl.14(%rip), %xmm15
35:    movsd %xmm15, -112(%rbp)
36:    movsd dbl.15(%rip), %xmm15
37:    movsd %xmm15, -120(%rbp)
38:    movsd dbl.16(%rip), %xmm15
39:    movsd %xmm15, -128(%rbp)
40:    movsd dbl.17(%rip), %xmm15
41:    movsd %xmm15, -136(%rbp)
42:    movsd dbl.18(%rip), %xmm15
43:    movsd %xmm15, -144(%rbp)
44:    movsd dbl.19(%rip), %xmm15
45:    movsd %xmm15, -152(%rbp)
46:    movsd dbl.20(%rip), %xmm15
47:    movsd %xmm15, -160(%rbp)
48:    movsd dbl.21(%rip), %xmm15
49:    movsd %xmm15, -168(%rbp)
50:    movsd dbl.22(%rip), %xmm15
51:    movsd %xmm15, -176(%rbp)
52:    movsd dbl.23(%rip), %xmm15
53:    movsd %xmm15, -184(%rbp)
54:    movsd dbl.24(%rip), %xmm15
55:    movsd %xmm15, -192(%rbp)
56:    movsd dbl.25(%rip), %xmm15
57:    movsd %xmm15, -200(%rbp)
58:    movsd dbl.26(%rip), %xmm15
59:    movsd %xmm15, -208(%rbp)
60:    movsd dbl.27(%rip), %xmm0
61:    movsd dbl.28(%rip), %xmm1
62:    movsd dbl.29(%rip), %xmm2
63:    movsd dbl.30(%rip), %xmm3
64:    movsd dbl.31(%rip), %xmm4
65:    movsd dbl.32(%rip), %xmm5
66:    movsd dbl.33(%rip), %xmm6
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:01-04:00
ended_at: 2026-07-09T04:43:01-04:00

## SCENARIO S11
criterion: misleading_success_output
surface: test harness CLI
expected: chapter 20 latest-only no-coalescing exits 0 and prints OK
invocation: ./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing
--- output start ---
----------------------------------------------------------------------
Ran 66 tests in 3.067s

OK
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:01-04:00
ended_at: 2026-07-09T04:43:04-04:00

## SCENARIO S12
criterion: generated_artifacts
surface: shell cleanup
expected: all /tmp Task59 manual artifacts removed and absent
invocation: rm -f /tmp/task59_manual_probe /tmp/task59_manual_int.c /tmp/task59_manual_int.s /tmp/task59_manual_int /tmp/task59_manual_double.c /tmp/task59_manual_double.s /tmp/task59_manual_double; for p in /tmp/task59_manual_probe /tmp/task59_manual_int.c /tmp/task59_manual_int.s /tmp/task59_manual_int /tmp/task59_manual_double.c /tmp/task59_manual_double.s /tmp/task59_manual_double; do if [ -e $p ]; then echo still_exists=$p; exit 1; else echo absent=$p; fi; done
--- output start ---
absent=/tmp/task59_manual_probe
absent=/tmp/task59_manual_int.c
absent=/tmp/task59_manual_int.s
absent=/tmp/task59_manual_int
absent=/tmp/task59_manual_double.c
absent=/tmp/task59_manual_double.s
absent=/tmp/task59_manual_double
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:04-04:00
ended_at: 2026-07-09T04:43:04-04:00

## SCENARIO S13
criterion: scope_fidelity
surface: git status scoped
expected: only allowed manual QA artifact should be newly written by this task; no tests diff
invocation: git diff -- tests; git status --short .omo/evidence/task-59-spill-loop-manual-qa.md tests .omo/evidence/task-59-spill-loop-probe.s | cat
--- output start ---
--- output end ---
exit_code: 0
started_at: 2026-07-09T04:43:04-04:00
ended_at: 2026-07-09T04:43:04-04:00

```

## Final artifact and scope verification

```text
$ test -s .omo/evidence/task-59-spill-loop-manual-qa.md && git diff -- tests && git status --short .omo/evidence/task-59-spill-loop-manual-qa.md tests .omo/evidence/task-59-spill-loop-probe.s && for p in /tmp/task59_manual_qa_run.sh /tmp/task59_manual_qa_run.log /tmp/task59_manual_probe /tmp/task59_manual_int.c /tmp/task59_manual_int.s /tmp/task59_manual_int /tmp/task59_manual_double.c /tmp/task59_manual_double.s /tmp/task59_manual_double; do if [ -e "$p" ]; then echo still_exists=$p; else echo absent=$p; fi; done
?? .omo/evidence/task-59-spill-loop-manual-qa.md
absent=/tmp/task59_manual_qa_run.sh
absent=/tmp/task59_manual_qa_run.log
absent=/tmp/task59_manual_probe
absent=/tmp/task59_manual_int.c
absent=/tmp/task59_manual_int.s
absent=/tmp/task59_manual_int
absent=/tmp/task59_manual_double.c
absent=/tmp/task59_manual_double.s
absent=/tmp/task59_manual_double
```

## Post-sanitization verification

After replacing Markdown-table pipe placeholders, I reran artifact/scope/tmp checks:

```text
$ test -s .omo/evidence/task-59-spill-loop-manual-qa.md; echo artifact_nonempty_exit=$?; git diff -- tests; git status --short .omo/evidence/task-59-spill-loop-manual-qa.md tests .omo/evidence/task-59-spill-loop-probe.s; for p in /tmp/task59_manual_qa_run.sh /tmp/task59_manual_qa_run.log /tmp/task59_manual_probe /tmp/task59_manual_int.c /tmp/task59_manual_int.s /tmp/task59_manual_int /tmp/task59_manual_double.c /tmp/task59_manual_double.s /tmp/task59_manual_double; do if [ -e "$p" ]; then echo still_exists=$p; else echo absent=$p; fi; done
artifact_nonempty_exit=0
?? .omo/evidence/task-59-spill-loop-manual-qa.md
absent=/tmp/task59_manual_qa_run.sh
absent=/tmp/task59_manual_qa_run.log
absent=/tmp/task59_manual_probe
absent=/tmp/task59_manual_int.c
absent=/tmp/task59_manual_int.s
absent=/tmp/task59_manual_int
absent=/tmp/task59_manual_double.c
absent=/tmp/task59_manual_double.s
absent=/tmp/task59_manual_double
```
