# Chapter 17 — Supporting Dynamic Memory Allocation

## Core learning goal
Add `sizeof`, `void`, and generic-pointer support so dynamic memory APIs fit cleanly.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Implement compile-time `sizeof` over the supported object and type forms.
2. Model `void` and `void*` without treating `void` as a normal object type.
3. Support heap-allocation-style library calls and conversions required by the tests.
4. Ensure incomplete-type and size reasoning stay in semantic analysis, not runtime lowering.
5. Audit pointer and array code for size-calculation reuse.

## Recommended order of attack
1. confirm the grammar and type rules you are adding
2. add or extend tokens only if the chapter requires them
3. update the parser or semantic phase in the smallest reviewable slice
4. only then update IR/backend behavior
5. run latest-only tests, then rerun broader chapter coverage

## Commands to run
```bash
# Build the current compiler
cargo build --release

# Run the latest chapter only
./tests/test_compiler ./target/release/rustcc --chapter 17 --latest-only
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_17/valid`
- `tests/tests/chapter_17/invalid_*`

## Key theory notes
- `sizeof` is a semantic/layout query, not an ordinary runtime expression in the common cases used here.
- `void*` is the language’s generic object-pointer bridge.
- This chapter intensifies the link between semantic completeness checks and backend correctness.

## Common failure modes
- Evaluating `sizeof` at runtime unnecessarily.
- Permitting invalid uses of `void` as a concrete object type.
- Using wrong byte counts for pointer or array sizes.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Chapter 18 extends layout reasoning from arrays to user-defined aggregates.
