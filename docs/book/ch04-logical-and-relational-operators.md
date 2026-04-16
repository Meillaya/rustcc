# Chapter 4 — Logical and Relational Operators

## Core learning goal
Compile comparisons and short-circuit logic without evaluating operands eagerly.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Add tokens and parse support for relational, equality, and logical operators.
2. Normalize boolean results to integer truth values expected by C tests.
3. Lower logical `&&` and `||` with labels and jumps instead of always evaluating both sides.
4. Keep relational and equality precedence distinct from arithmetic precedence.
5. Re-run earlier arithmetic chapters after adding branching-oriented lowering.

## Recommended order of attack
1. confirm the grammar and type rules you are adding
2. add or extend tokens only if the chapter requires them
3. update the parser or semantic phase in the smallest reviewable slice
4. only then update IR/backend behavior
5. run latest-only tests, then rerun broader chapter coverage

## Commands to run
```bash
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 4 --latest-only --bitwise
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_4/valid`
- `tests/tests/chapter_4/invalid_parse`

## Key theory notes
- Short-circuiting is a control-flow problem, not just an arithmetic one.
- Comparisons in C produce integer values rather than a dedicated boolean type.
- This is a natural point to start thinking in control-flow-graph terms.

## Common failure modes
- Evaluating both sides of `&&` or `||`.
- Returning non-normalized truth values.
- Getting comparison precedence wrong relative to arithmetic.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Once control flow appears, later chapters on `if`, loops, and TACKY become much easier to reason about.
