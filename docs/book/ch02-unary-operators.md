# Chapter 2 — Unary Operators

## Core learning goal
Support unary expression forms and preserve nesting and parentheses correctly.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Extend the token set for prefix operators like unary minus and bitwise complement.
2. Refactor the parser so unary operators bind tighter than binary operators you will add later.
3. Represent unary expressions explicitly in the AST rather than encoding them as special literals.
4. Lower unary negation and bitwise complement to backend instructions that preserve operand size.
5. Retest chapter 1 to ensure the new expression logic did not break the minimal path.

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
./tests/test_compiler ./target/release/rustcc --chapter 2 --latest-only
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_2/valid`
- `tests/tests/chapter_2/invalid_parse`

## Key theory notes
- Prefix operators need a precedence level higher than binary arithmetic.
- Unary minus is a semantic operation on a value, not part of the numeric token itself.
- Parentheses change parse structure even when they do not change final value.

## Common failure modes
- Treating unary `-` as if it were part of the integer token in every context.
- Collapsing multiple unary operators in the wrong order.
- Losing parentheses information during parsing.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Chapter 3 will force you to confront full precedence and associativity, so keep this chapter’s expression code tidy.
