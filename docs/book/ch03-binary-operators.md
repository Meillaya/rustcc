# Chapter 3 — Binary Operators

## Core learning goal
Add arithmetic binary operators with correct precedence, associativity, and backend lowering.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Extend the lexer for arithmetic and optional bitwise operators.
2. Choose and document a precedence strategy: layered recursive descent or precedence climbing.
3. Ensure left-associative operators parse left-associatively and bind according to the chapter rules.
4. Lower multiplication, division, and remainder carefully, especially around signed behavior.
5. Optionally enable the book’s bitwise extra-credit cases through the harness flags.

## Recommended order of attack
1. confirm the grammar and type rules you are adding
2. add or extend tokens only if the chapter requires them
3. update the parser or semantic phase in the smallest reviewable slice
4. only then update IR/backend behavior
5. run latest-only tests, then rerun broader chapter coverage

## Commands to run
```bash
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 3 --latest-only --bitwise
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_3/valid`
- `tests/tests/chapter_3/invalid_parse`

## Key theory notes
- Binary-expression parsing is where grammar shape starts to matter a lot.
- Division and remainder on x86-64 have register conventions that differ from simple two-operand arithmetic.
- Precedence bugs often look like backend bugs, so confirm parse shape before debugging assembly.

## Common failure modes
- Using the wrong register setup for division/remainder.
- Making all operators the same precedence.
- Breaking unary parsing when introducing binary forms.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Chapter 4 builds on the precedence machinery you choose here, so prefer a strategy that scales cleanly.
