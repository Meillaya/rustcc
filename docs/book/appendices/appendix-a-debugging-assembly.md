# Appendix A — Debugging Assembly and Runtime Failures

## When to use this appendix

Use this appendix when:

- the compiler builds, but a valid test case returns the wrong value
- function calls appear to corrupt state
- floating-point or structure-passing behavior looks wrong
- register allocation passes compile but runtime behavior fails

## First-level triage

1. rerun the failing test with `--keep-asm-on-failure`
2. inspect the emitted `.s` file
3. assemble and run manually if needed
4. compare the failing behavior against your mental model of:
   - calling convention
   - stack alignment
   - live-value preservation
   - control-flow shape

## Useful commands

Preserve assembly from the harness:

```bash
./tests/test_compiler ./target/release/rustcc --chapter N --latest-only --keep-asm-on-failure
```

Inspect the produced binary:

```bash
objdump -d ./failing_binary
readelf -a ./failing_binary
```

Debug with GDB:

```bash
gdb ./failing_binary
break main
run
layout asm
info registers
x/32gx $rsp
stepi
```

Debug with LLDB:

```bash
lldb ./failing_binary
breakpoint set --name main
run
register read
memory read --format x --size 8 --count 32 $rsp
stepi
```

## What to inspect first

### Wrong return value

Check:

- the expected return register
- whether the last assignment to the return register is correct
- whether a call clobbered the return value before `ret`

### Bad function call

Check:

- integer argument registers
- floating-point argument registers
- stack argument order
- stack alignment before `call`
- callee-saved register restoration

### Bad spill / regalloc result

Check:

- whether a spilled value is reloaded before use
- whether two interfering values received the same register
- whether stack slot offsets overlap incorrectly
- whether a caller-saved register was assumed live across a call

### Bad aggregate behavior

Check:

- field offsets and padding
- whether copies are by value instead of by address aliasing
- argument classification under the ABI

## A practical backend checklist

Before blaming a complex phase, verify simpler invariants:

1. control flow reaches the intended block
2. the right value is loaded or computed there
3. the value survives until use
4. the ABI boundary is respected
5. only then suspect instruction selection or optimization logic
