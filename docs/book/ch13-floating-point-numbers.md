# Chapter 13 — Floating-Point Numbers

## Core learning goal
Add floating-point literals, arithmetic, conversions, and calling behavior.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Extend the lexer and parser for floating literals and type spellings required by the book subset.
2. Propagate floating-point types through semantic analysis and mixed-type expressions.
3. Use the correct XMM-based calling and return conventions.
4. Implement int/float and float/int conversions carefully.
5. Handle NaN-sensitive comparisons and helper-library-linked tests explicitly.

## Recommended order of attack
1. confirm the grammar and type rules you are adding
2. add or extend tokens only if the chapter requires them
3. update the parser or semantic phase in the smallest reviewable slice
4. only then update IR/backend behavior
5. run latest-only tests, then rerun broader chapter coverage

## Commands to run
```bash
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 13 --latest-only --nan
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_13/valid`
- `tests/tests/chapter_13/helper_libs`
- `tests/tests/chapter_13/invalid_*`

## Key theory notes
- Floating-point semantics are not integer semantics with decimal points.
- IEEE-754 special values affect comparisons and optimization safety.
- Floating arguments and returns often use different register classes from integers.

## Common failure modes
- Doing float work in integer registers.
- Comparing NaN as if it were an ordinary number.
- Forgetting helper-library or math-library integration.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Pointers in chapter 14 will connect types to addresses and memory layout more directly.
