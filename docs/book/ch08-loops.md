# Chapter 8 — Loops

## Core learning goal
Add iterative control flow with correct `break` and `continue` behavior.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Parse and lower `while`, `do...while`, and `for` forms.
2. Track loop entry, continue, and break labels explicitly, especially for nested loops.
3. Support omitted `for` clauses and declaration forms in loop headers if the chapter requires them.
4. Extend semantic checks so `break` and `continue` are only valid inside loops.
5. Validate loop lowering first at the control-flow level, then at runtime.

## Recommended order of attack
1. confirm the grammar and type rules you are adding
2. add or extend tokens only if the chapter requires them
3. update the parser or semantic phase in the smallest reviewable slice
4. only then update IR/backend behavior
5. run latest-only tests, then rerun broader chapter coverage

## Commands to run
```bash
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 8 --latest-only --compound --increment --goto --switch
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_8/valid`
- `tests/tests/chapter_8/invalid_parse`
- `tests/tests/chapter_8/invalid_semantics`

## Key theory notes
- Loops introduce backedges in the control-flow graph.
- In `for` loops, `continue` usually jumps to the post-expression, not directly to the condition.
- Control-flow correctness is easier to debug at the label level before debugging instructions.

## Common failure modes
- Incorrect `continue` targets in `for` loops.
- Allowing `break` or `continue` outside loop context.
- Mis-parsing empty loop clauses.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Chapter 9 introduces function calls, which combine control-flow and ABI reasoning.
