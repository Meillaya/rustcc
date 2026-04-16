# Chapter Lock

## Canonical baseline

This package locks chapter numbering and chapter titles to the following sources:

1. `docs/Writing a C Compiler - Sandler, Nora.pdf`
2. `tests/README.md`
3. `tests/tests/chapter_*`

## Locked chapter sequence

1. A Minimal Compiler
2. Unary Operators
3. Binary Operators
4. Logical and Relational Operators
5. Local Variables
6. if Statements and Conditional Expressions
7. Compound Statements
8. Loops
9. Functions
10. File Scope Variable Declarations and Storage-Class Specifiers
11. Long Integers
12. Unsigned Integers
13. Floating-Point Numbers
14. Pointers
15. Arrays and Pointer Arithmetic
16. Characters and Strings
17. Supporting Dynamic Memory Allocation
18. Structures
19. Optimizing TACKY Programs
20. Register Allocation

## Early-access note

`tests/README.md` explicitly warns that early-access chapter numbers differed from the final book.

This means:

- do not use old blog-post numbering without checking the PDF
- do not rename guide pages based on older early-access material
- do not assume an older implementation or blog series uses the same numbering

## Mismatch policy

If a later discovery appears to contradict the current sequence:

1. verify the PDF TOC again
2. verify `tests/README.md`
3. verify the on-disk `tests/tests/chapter_*` structure
4. record the discrepancy here before changing any public-facing chapter page
