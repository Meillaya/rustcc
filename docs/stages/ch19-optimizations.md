# Chapter 19 — TACKY Optimizations

## Purpose

Add semantics-preserving TACKY optimization passes and the CFG/dataflow machinery they require.

## CFG pseudocode

```text
split function into basic blocks:
  start new block at labels and after terminators
  each block records instructions, predecessors, successors
  verify every block has explicit terminator or structured exit
```

## Optimization pseudocode

```text
constant_folding:
  replace pure operations on constants with constant result when defined

unreachable_code_elimination:
  mark blocks reachable from entry
  remove unmarked blocks

copy_propagation:
  compute reaching copies
  substitute temp uses only when source still valid and no side-effect/clobber intervenes

dead_store_elimination:
  compute liveness backwards
  remove assignments to dead temps when rhs has no side effects

pipeline:
  run enabled passes in harness order
  repeat to bounded fixed point when required
  validate IR invariants after each pass
```

## Verification target

Pseudocode guidance only. After real code is filled in, use official Part III optimization harness commands; do not use `tests/test_compiler --stage` for chapter 19.
