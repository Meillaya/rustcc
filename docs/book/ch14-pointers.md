# Chapter 14 — Pointers

## Core learning goal
Support pointer types, address-of, dereference, and pointer-aware semantics.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Add pointer type constructors and pointee information to the type system.
2. Implement unary address-of and dereference with correct lvalue rules.
3. Handle pointer/int casts and pointer comparisons according to the chapter subset.
4. Preserve pointee size info for later array and arithmetic work.
5. Audit backend loads/stores for width and indirection correctness.

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
./tests/test_compiler ./target/release/rustcc --chapter 14 --latest-only
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_14/valid`
- `tests/tests/chapter_14/invalid_*`

## Key theory notes
- Pointers represent addresses, not just opaque integers.
- Lvalues become more nuanced once dereference appears.
- Pointer semantics are the foundation for arrays, strings, heap objects, and aggregates.

## Common failure modes
- Forgetting to scale pointer arithmetic later on.
- Allowing invalid dereference targets.
- Confusing address values with loaded object values.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Chapter 15 turns pointers into layout-sensitive arithmetic and indexing.
