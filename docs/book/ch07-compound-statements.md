# Chapter 7 — Compound Statements

## Core learning goal
Implement nested blocks, nested declarations, and shadowing rules.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Push a new scope for every `{ ... }` block and pop it on exit.
2. Resolve identifiers from inner to outer scope while preserving shadowing behavior.
3. Keep declaration lifetime and visibility aligned with block structure.
4. Retest assignment and branching code against nested-block cases.
5. Clarify how stack-slot reclamation should interact with nested scopes.

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
./tests/test_compiler ./target/release/rustcc --chapter 7 --latest-only
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_7/valid`
- `tests/tests/chapter_7/invalid_parse`
- `tests/tests/chapter_7/invalid_semantics`

## Key theory notes
- Lexical scope is hierarchical and should be modeled as such.
- Shadowing does not destroy outer bindings; it temporarily hides them.
- Block structure often determines both semantics and storage lifetime.

## Common failure modes
- Leaking inner declarations after a block exits.
- Mutating the wrong symbol-table layer.
- Reclaiming storage too early or not at all.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Good scope handling here pays off heavily when loops and functions introduce more nesting contexts.
