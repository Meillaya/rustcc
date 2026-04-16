# Chapter 19 — Optimizing TACKY Programs

## Core learning goal
Implement semantics-preserving optimization passes over the IR.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Make TACKY/IR explicit enough that optimization passes can reason about values and control flow.
2. Implement constant folding, unreachable-code elimination, copy propagation, and dead-store elimination in a deliberate order.
3. Preserve side effects, labels, and control-flow semantics while simplifying code.
4. Test both pass-local behavior and whole-pipeline behavior.
5. Track whether optimizations should run for all types or only int-only subsets under the harness.

## Recommended order of attack
1. confirm the grammar and type rules you are adding
2. add or extend tokens only if the chapter requires them
3. update the parser or semantic phase in the smallest reviewable slice
4. only then update IR/backend behavior
5. run latest-only tests, then rerun broader chapter coverage

## Commands to run
```bash
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_19/constant_folding`
- `tests/tests/chapter_19/unreachable_code_elimination`
- `tests/tests/chapter_19/copy_propagation`
- `tests/tests/chapter_19/dead_store_elimination`
- `tests/tests/chapter_19/whole_pipeline`

## Key theory notes
- Optimization is a proof obligation: every transformation must preserve observable behavior.
- Data-flow reasoning becomes central here, especially for liveness and dead-store questions.
- Pass ordering matters because one optimization often enables the next.

## Common failure modes
- Removing code that still has side effects.
- Folding expressions across type-sensitive boundaries incorrectly.
- Running passes in an order that prevents later simplifications.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Chapter 20 turns IR-level value reasoning into physical register decisions and spill strategy.
