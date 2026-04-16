# Chapter 5 — Local Variables

## Core learning goal
Add mutable local storage, declarations, assignment, and the first real semantic checks.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Represent local declarations distinctly from expression statements.
2. Create an environment that maps identifiers to stack locations and type info.
3. Implement assignment semantics and keep lvalue/rvalue distinctions explicit.
4. Reject invalid references and bad assignments in a semantic-analysis phase.
5. Document how stack slots are allocated and reused.

## Recommended order of attack
1. confirm the grammar and type rules you are adding
2. add or extend tokens only if the chapter requires them
3. update the parser or semantic phase in the smallest reviewable slice
4. only then update IR/backend behavior
5. run latest-only tests, then rerun broader chapter coverage

## Commands to run
```bash
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 5 --latest-only --bitwise --compound --increment
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_5/valid`
- `tests/tests/chapter_5/invalid_parse`
- `tests/tests/chapter_5/invalid_semantics`

## Key theory notes
- This is where syntax stops being enough; you now need name resolution and static checking.
- Stack-frame layout starts to matter because names become addresses.
- Assignments are expressions in C, which affects parse shape and lowering.

## Common failure modes
- Allowing use-before-definition or self-referential initializers.
- Treating any expression as assignable.
- Reusing stack storage unsafely when scopes are still live.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
From here on, semantic analysis becomes a permanent part of the compiler rather than an optional extra.
