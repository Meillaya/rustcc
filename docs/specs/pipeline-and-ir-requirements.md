# Pipeline and IR Requirements

## Pipeline stages

1. preprocess when required
2. lex
3. parse
4. semantic analysis
5. lower to TACKY / IR
6. optimize where enabled
7. lower to assembly-oriented form
8. allocate registers / spill stack slots
9. emit assembly
10. assemble and link when full build is requested

## IR requirements

- the IR SHALL make control flow explicit
- the IR SHALL make side effects explicit
- the IR SHALL preserve type/layout information needed later
- the IR SHALL be stable enough for optimization and backend passes

## Invariants

- no IR use before definition
- every basic block ends in a terminator or an equivalent structured exit
- optimizations SHALL not change observable behavior
