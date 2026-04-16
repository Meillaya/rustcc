# Chapter 18 — Structures

## Core learning goal
Implement structure layout, member access, initialization, and ABI-sensitive passing/returning.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Parse structure tags, definitions, declarators, and member access forms.
2. Compute field offsets, alignment, padding, and total object size.
3. Support copying, passing, and returning structures by value according to the target ABI.
4. Handle structure member lvalues correctly and extend to unions if you opt into extra credit.
5. Retest pointer, array, and function-call paths because aggregates stress all of them.

## Recommended order of attack
1. confirm the grammar and type rules you are adding
2. add or extend tokens only if the chapter requires them
3. update the parser or semantic phase in the smallest reviewable slice
4. only then update IR/backend behavior
5. run latest-only tests, then rerun broader chapter coverage

## Commands to run
```bash
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_18/valid`
- `tests/tests/chapter_18/invalid_*`
- `tests/tests/chapter_18/valid/extra_credit`

## Key theory notes
- Aggregate layout is where semantic analysis and ABI rules meet directly.
- Tag namespaces and ordinary identifiers are related but distinct.
- Value semantics for structures are easy to state and hard to implement correctly without explicit copying logic.

## Common failure modes
- Computing wrong field offsets or padding.
- Treating struct copies as pointer aliasing.
- Resolving struct tags in the wrong namespace or scope.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
With full-language support in place, chapter 19 shifts focus from correctness-only to correctness-preserving optimization.
