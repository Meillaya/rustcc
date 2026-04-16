# Chapter 15 — Arrays and Pointer Arithmetic

## Core learning goal
Add arrays, indexing, decay, and offset computation.

## Book scope
Implement exactly the feature set this chapter adds, then retest all earlier behavior before moving on.

## Prerequisites
- the previous chapter is green
- the matching SRS sections are understood
- you know which test directories matter for this chapter

## Key implementation steps
1. Parse array declarators and array initialization forms the book subset requires.
2. Model array size and element type distinctly from pointer type.
3. Lower `a[i]` through base address plus scaled offset semantics.
4. Keep decay rules explicit rather than letting arrays silently become pointers everywhere.
5. Audit stack/data layout for contiguous storage and correct initializer placement.

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
./tests/test_compiler ./target/release/rustcc --chapter 15 --latest-only
```

## Likely tests / chapter mapping hints
- `tests/tests/chapter_15/valid`
- `tests/tests/chapter_15/invalid_*`

## Key theory notes
- Array-to-pointer decay is contextual, not universal.
- Pointer arithmetic scales by element size, not by bytes typed directly in source.
- Offsets, strides, and sizes now drive both semantics and code generation.

## Common failure modes
- Treating arrays as pointers in every context.
- Computing offsets in units of one byte regardless of element size.
- Mishandling nested or partially initialized arrays.

## Definition of done
- latest-only tests for this chapter pass
- earlier dependent behavior still passes
- invalid cases reject at the right stage
- docs, maps, and notes for the chapter are internally consistent

## Handoff to the next chapter
Characters and strings in chapter 16 build directly on the array-and-layout model you choose here.
