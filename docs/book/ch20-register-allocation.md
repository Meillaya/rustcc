# Chapter 20 — Register Allocation

## Core learning goal
Map virtual values to physical registers while preserving correctness under pressure, calls, and spills.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Compute liveness information or an equivalent live-range model for temporaries.
2. Build interference information and choose a coloring or equivalent allocation strategy.
3. Spill values to stable stack slots when the register set is insufficient.
4. Preserve caller/callee-save constraints, stack alignment, and floating/int register-class distinctions.
5. Test no-coalescing first, then enable coalescing once the baseline is stable.

## Recommended order of attack
1. confirm the grammar and type rules you are adding
2. add or extend tokens only if the chapter requires them
3. update the parser or semantic phase in the smallest reviewable slice
4. only then update IR/backend behavior
5. run latest-only tests, then rerun broader chapter coverage

## Commands to run
```bash
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_20/int_only`
- `tests/tests/chapter_20/all_types`
- `tests/tests/chapter_20/helper_libs`

## Key theory notes
- Register allocation is a graph problem because simultaneous liveness induces interference.
- Spilling is a correctness tool first and a performance penalty second.
- The allocator must cooperate with, not replace, calling-convention rules.

## Common failure modes
- Assigning the same physical register to interfering values.
- Failing to reload spilled values before use.
- Breaking stack alignment or clobber rules across calls.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
At this point, the project becomes an end-to-end compiler engineering artifact rather than a frontend/backend learning exercise.
