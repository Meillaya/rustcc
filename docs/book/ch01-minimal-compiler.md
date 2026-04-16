# Chapter 1 — A Minimal Compiler

## Core learning goal
Build the smallest end-to-end compiler that accepts a single function returning an integer constant.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Implement the compile driver surface first: input path, stage selection, output naming, and error handling.
2. Teach the lexer to recognize the minimum token set: `int`, `main`, `void`, `return`, integer literals, parentheses, braces, and semicolons.
3. Define a tiny AST that can represent one function containing one return statement.
4. Emit a minimal function prologue/epilogue and place the return value in the correct return register.
5. Verify that invalid lex/parse inputs are rejected cleanly and do not produce stale output artifacts.

## Recommended order of attack
1. confirm the grammar and type rules you are adding
2. add or extend tokens only if the chapter requires them
3. update the parser or semantic phase in the smallest reviewable slice
4. only then update IR/backend behavior
5. run latest-only tests, then rerun broader chapter coverage

## Commands to run
```bash
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 1 --latest-only --expected-error-codes 1 2
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_1/valid`
- `tests/tests/chapter_1/invalid_lex`
- `tests/tests/chapter_1/invalid_parse`

## Key theory notes
- A compiler is a pipeline; even the minimal compiler still has multiple conceptual stages.
- Lexing and parsing are different problems; keep token recognition separate from grammar recognition.
- On x86-64 SysV, integer returns come back in `rax`.

## Common failure modes
- Forgetting that the book now uses `int main(void)` instead of `int main()`.
- Accepting trailing garbage after a valid program.
- Returning the constant incorrectly because the wrong register was used.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Move to chapter 2 only after your driver, lexer, parser, and minimal codegen all agree on the same tiny language.
