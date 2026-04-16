# Optimization and Register Allocation Requirements

## Optimization

- constant folding SHALL be semantics-preserving
- unreachable-code elimination SHALL not erase side effects
- copy propagation SHALL not cross invalid boundaries
- dead-store elimination SHALL rely on valid liveness/observability reasoning

## Register allocation

- the compiler SHALL compute liveness or equivalent live-range information
- interference or equivalent conflict data SHALL be used to avoid assigning the same register to conflicting values
- spills SHALL preserve correctness even when they reduce performance
- coalescing, if enabled, SHALL not violate conflicts or ABI rules
