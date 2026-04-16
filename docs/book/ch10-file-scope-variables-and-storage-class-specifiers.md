# Chapter 10 — File Scope Variable Declarations and Storage-Class Specifiers

## Core learning goal
Add globals, linkage, tentative definitions, and storage-class behavior.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Distinguish file-scope declarations from definitions.
2. Track linkage, visibility, and storage duration for globals.
3. Emit data objects and initializers into the right sections.
4. Support cross-file symbol references and helper-library integration.
5. Retest functions and local scopes to avoid confusing file scope with block scope.

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
./tests/test_compiler ./target/release/rustcc --chapter 10 --latest-only
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_10/valid`
- `tests/tests/chapter_10/invalid_parse`
- `tests/tests/chapter_10/invalid_types`
- `tests/tests/chapter_10/invalid_labels`
- `tests/tests/chapter_10/invalid_declarations`

## Key theory notes
- Storage duration and linkage are semantic properties that affect both diagnostics and code emission.
- Tentative definitions behave differently from ordinary declarations.
- A compiler now has to reason about a whole translation unit, not only one function body.

## Common failure modes
- Emitting duplicate globals.
- Resolving `static` and `extern` incorrectly.
- Treating file-scope initializers as if they were runtime assignments.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Once globals exist, widening the type system becomes the next natural pressure point.
