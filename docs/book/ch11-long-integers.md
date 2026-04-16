# Chapter 11 — Long Integers

## Core learning goal
Extend the compiler from 32-bit signed integers to 64-bit signed integers.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Teach the type system, constant parser, and literal handling about `long`.
2. Update arithmetic, comparisons, casts, and return handling to preserve 64-bit behavior.
3. Audit all integer-sensitive code paths for accidental 32-bit truncation.
4. Retest function-calling paths because argument and return width now matter more.
5. Keep traceability for signed width conversions explicit.

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
./tests/test_compiler ./target/release/rustcc --chapter 11 --latest-only
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_11/valid`
- `tests/tests/chapter_11/invalid_*`

## Key theory notes
- Width-aware semantics are a type-system problem first and a codegen problem second.
- Sign extension and truncation must be deliberate, not accidental side effects.
- Backend instructions often have width-sensitive variants that must match the IR type.

## Common failure modes
- Truncating to 32 bits too early.
- Using 32-bit comparisons for 64-bit values.
- Forgetting to update cast and call boundaries.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Chapter 12 adds unsigned semantics, which are often more subtle than just adding another keyword.
