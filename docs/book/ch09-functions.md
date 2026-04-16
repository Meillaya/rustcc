# Chapter 9 — Functions

## Core learning goal
Support function definitions, parameters, calls, and ABI-compliant returns.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Extend the parser for parameter lists, call expressions, and multiple function definitions.
2. Represent function signatures and enforce argument-count/type checks.
3. Lower calls according to the SysV AMD64 calling convention for the currently supported types.
4. Preserve stack alignment and caller/callee-saved register obligations.
5. Test no-argument, register-argument, stack-argument, and helper-library cases separately.

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
./tests/test_compiler ./target/release/rustcc --chapter 9 --latest-only
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_9/valid`
- `tests/tests/chapter_9/invalid_parse`
- `tests/tests/chapter_9/invalid_types`
- `tests/tests/chapter_9/invalid_labels`
- `tests/tests/chapter_9/invalid_declarations`

## Key theory notes
- Calling conventions are part of compiler correctness, not optional backend polish.
- Functions introduce a second scale of scope: local block scope inside a function plus file-level symbol visibility.
- This chapter is a natural dividing line between toy backends and real machine-level calling semantics.

## Common failure modes
- Passing arguments in the wrong order or register class.
- Breaking stack alignment before a call.
- Losing live values across nested calls.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Chapter 10 expands the symbol story from function-local to translation-unit scope.
