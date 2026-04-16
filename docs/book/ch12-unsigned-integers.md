# Chapter 12 — Unsigned Integers

## Core learning goal
Add unsigned integer semantics, conversions, and operations.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Represent unsignedness explicitly in the type system.
2. Choose correct zero-extension and comparison behavior in the backend.
3. Audit promotions and mixed signed/unsigned expressions.
4. Keep shifts and modulo/division semantics aligned with unsigned rules.
5. Retest all integer chapters because promotions now affect older expressions too.

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
./tests/test_compiler ./target/release/rustcc --chapter 12 --latest-only
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_12/valid`
- `tests/tests/chapter_12/invalid_*`

## Key theory notes
- Unsigned arithmetic is modular arithmetic.
- Signed and unsigned comparison operators may use different machine instructions or interpretation rules.
- The most common unsigned bugs are actually conversion bugs.

## Common failure modes
- Sign-extending values that should be zero-extended.
- Treating unsigned compare as signed compare.
- Ignoring mixed-type promotion rules.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Floating point in chapter 13 will add a second major numeric domain with its own register class.
