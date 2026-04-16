# Chapter 16 — Characters and Strings

## Core learning goal
Add character values and string literals with correct storage and initialization behavior.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Extend lexing for character and string literal syntax, including escapes required by the test suite.
2. Model `char` distinctly enough to preserve byte-sized semantics and promotions.
3. Represent string literals as stable data objects that can initialize arrays or decay to pointers when needed.
4. Audit null terminators, literal length, and storage class behavior.
5. Retest array and pointer logic because strings exercise both heavily.

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
./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_16/valid`
- `tests/tests/chapter_16/invalid_*`

## Key theory notes
- String literals are data objects, not just syntax sugar for arrays.
- Character values often promote in expressions, which can hide signedness bugs.
- Literal encoding and length handling quickly expose frontend/backend mismatches.

## Common failure modes
- Forgetting the terminating null byte.
- Treating string literals as mutable lvalues in the wrong contexts.
- Breaking signed-char or escape-sequence handling.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Chapter 17 adds `sizeof`, `void`, and heap-oriented support that depends on strong type and layout reasoning.
