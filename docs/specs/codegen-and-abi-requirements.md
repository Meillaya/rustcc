# Code Generation and ABI Requirements

## Target assumptions

- target backend is x86-64 in the System V AMD64 family
- generated assembly SHALL be accepted by the host assembler

## ABI requirements

- integer/pointer arguments SHALL follow the platform calling convention
- floating arguments SHALL use the proper floating register class
- stack alignment SHALL be preserved at calls
- caller-saved and callee-saved obligations SHALL be respected

## Backend requirements

- loads, stores, arithmetic, comparisons, branches, and returns SHALL preserve type semantics
- aggregate layout and passing rules SHALL match the ABI when aggregate chapters are enabled
- labels SHALL be unique and branch targets valid
