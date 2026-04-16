# Stage Crosswalk

This repository historically describes compiler work in stage-oriented terms, while this package is organized by book chapters.

This file bridges those views.

## Why both views exist

- the **book view** is best for a learner working chapter by chapter
- the **stage view** is best for tracking compiler passes and architecture boundaries

## Crosswalk table

| Stage-oriented view | Main chapters | What changes most |
|---|---:|---|
| Driver / orchestration | 1, 2, 9, 10 | CLI, file handling, preprocess/assemble/link integration |
| Lexing | 1–18 | token recognition grows as syntax and types expand |
| Parsing | 1–18 | grammar, precedence, declarations, statements, types |
| Semantic analysis | 5–18 | scope, name resolution, type rules, layout validation |
| TACKY / IR lowering | 2–20 | normalization, control flow, lowering, optimization boundaries |
| Code generation | 1–20 | assembly lowering, ABI, layout, machine details |
| Optimization | 19 | constant folding, copy propagation, dead stores, unreachable code |
| Register allocation | 20 | liveness, interference, coloring, spills, coalescing |

## Practical interpretation

If you are tracking progress as passes:

- use the SRS in `docs/specs/`
- use the maps in this directory
- treat each chapter as the introduction or extension of one or more stages

If you are tracking progress as book milestones:

- follow the chapter pages in order
- use the stage view only to understand where a bug belongs
