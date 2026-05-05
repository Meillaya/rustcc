# Chapter 20 — Register Allocation

## Purpose

Replace pseudoregisters with physical registers or stack spills while preserving ABI constraints.

## Dataflow pseudocode

```text
liveness:
  initialize live_in/live_out empty
  repeat until fixed point:
    live_out(block) = union live_in(successors)
    live_in(block) = uses(block) union (live_out(block) - defs(block))
```

## Allocation pseudocode

```text
build_interference:
  for each instruction in reverse order:
    for each defined pseudo d:
      add edge between d and every pseudo live after instruction

color_graph:
  simplify nodes with degree < available_register_count
  choose spill candidate when no simplifiable node exists
  assign physical registers during select phase
  mark uncolored pseudos for spilling

rewrite_spills:
  assign stack slots
  insert reloads before uses
  insert stores after defs
  rerun allocation until legal

coalescing:
  when enabled, merge move-related pseudos only if interference constraints remain valid
  when --no-coalescing, skip coalescing but still allocate registers
```

## Final fixup pseudocode

```text
replace pseudos with registers/stack operands
repair illegal memory-to-memory instructions using scratch register
save/restore callee-saved registers used by allocated code
preserve stack alignment at calls
```

## Verification target

Pseudocode guidance only. After real code is filled in, run official chapter 20 int-only/all-types/helper-library and `--no-coalescing` gates; use direct `-S` smoke checks for assembly inspection.
