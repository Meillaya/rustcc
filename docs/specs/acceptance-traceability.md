# Acceptance Traceability

## Rule

Every requirement family in this package should map to one or more chapter test families.

## Requirement family map

| Family | Main evidence |
|---|---|
| Driver / CLI | chapter harness invocations, stage-stop behavior |
| Lexing | chapter 1 invalid lex tests and later lexical invalid cases |
| Parsing | invalid parse directories across chapters |
| Semantics | invalid semantics / invalid types / invalid declarations / invalid labels |
| Codegen | valid directories plus expected runtime outputs |
| Optimization | chapter 19 optimization directories |
| Regalloc | chapter 20 int-only / all-types / no-coalescing paths |

## Coverage rule

A chapter is not complete until:

- valid cases pass
- invalid cases reject correctly
- the corresponding requirements remain accurate
