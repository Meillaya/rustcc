# Appendix B — Assembly and ABI Reference Tables

## System V AMD64 quick reference

### Integer/pointer argument registers

| Position | Register |
|---:|---|
| 1 | `rdi` |
| 2 | `rsi` |
| 3 | `rdx` |
| 4 | `rcx` |
| 5 | `r8` |
| 6 | `r9` |

### Floating-point argument registers

| Position | Register |
|---:|---|
| 1 | `xmm0` |
| 2 | `xmm1` |
| 3 | `xmm2` |
| 4 | `xmm3` |
| 5 | `xmm4` |
| 6 | `xmm5` |
| 7 | `xmm6` |
| 8 | `xmm7` |

### Return registers

| Value class | Register |
|---|---|
| integer / pointer | `rax` |
| floating point | `xmm0` |

### Common callee-saved registers

- `rbx`
- `rbp`
- `r12`
- `r13`
- `r14`
- `r15`

### Common caller-saved registers

- `rax`
- `rcx`
- `rdx`
- `rsi`
- `rdi`
- `r8`
- `r9`
- `r10`
- `r11`
- `xmm0`–`xmm15` in the usual SysV model

## Alignment rule of thumb

Before a `call`, keep the stack aligned so the callee sees the required 16-byte alignment convention.

If calls start crashing or helper-library tests fail unexpectedly, verify alignment first.

## Useful instruction families

| Purpose | Typical instructions |
|---|---|
| move/load/store | `mov`, `movsx`, `movzx`, `lea` |
| integer arithmetic | `add`, `sub`, `imul`, `idiv`, `neg` |
| bitwise | `and`, `or`, `xor`, `not`, `shl`, `shr`, `sar` |
| comparison / branching | `cmp`, `test`, `setcc`, `jcc` |
| floating point | `movsd`, `addsd`, `subsd`, `mulsd`, `divsd`, conversion instructions |
| prologue / epilogue | `push`, `pop`, stack-pointer arithmetic, `ret` |

## Memory-layout reminders

- array element address = base + index × element size
- structure field address = base + field offset
- spilled temporaries need stable stack slots
- string literals need stable, addressable data storage

## When to use this appendix

Use this page as a quick cross-check, not as the sole authority. For edge cases, defer to the ABI and ISA references listed in `docs/research/`.
