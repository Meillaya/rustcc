# Chapter 6 — if Statements and Conditional Expressions

## Core learning goal
Add statement-level branching and ternary expressions while preserving existing expression semantics.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Parse `if`, `else`, and the conditional operator `?:` without ambiguity.
2. Generate labels for branch entry, else paths, and merge points.
3. Lower statement branches separately from expression branches.
4. Keep semantic checking aware of condition expression types.
5. If you enable the extra-credit `goto` path later, keep labels conceptually separate from local names.

## Recommended order of attack
1. confirm the grammar and type rules you are adding
2. add or extend tokens only if the chapter requires them
3. update the parser or semantic phase in the smallest reviewable slice
4. only then update IR/backend behavior
5. run latest-only tests, then rerun broader chapter coverage

## Commands to run
```bash
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 6 --latest-only --bitwise --compound --increment --goto
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_6/valid`
- `tests/tests/chapter_6/invalid_lex`
- `tests/tests/chapter_6/invalid_parse`
- `tests/tests/chapter_6/invalid_semantics`

## Key theory notes
- The ternary operator is an expression-level control-flow construct.
- Statement branching and expression branching are related but not identical lowering problems.
- This chapter is a good entry point for CFG thinking even before explicit basic blocks are introduced.

## Common failure modes
- Parsing `?:` with the wrong precedence.
- Executing both branches of a conditional expression.
- Incorrect label placement that falls through into the wrong block.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Chapter 7 deepens scope management; keep branching code clean so nested blocks do not become unmanageable.
