# Theory Foundations

## Automata and lexing

- lexical analysis is grounded in regular languages and finite automata
- maximal munch and reserved-word classification matter for practical lexers

## Parsing

- parsing is grounded in context-free grammars
- recursive descent and precedence climbing are the most relevant methods for this project

## Semantic analysis

- environments / symbol tables model lexical scope
- many type checks can be viewed as static judgments over expressions and declarations

## IR and optimization

- control-flow graphs and basic blocks are central for later passes
- liveness is a backward data-flow analysis problem
- dead-store elimination and copy propagation depend on valid flow reasoning

## Register allocation

- interference graphs model simultaneous liveness
- graph coloring is the canonical mental model for allocation
- spills and coalescing are constrained transformations, not arbitrary rewrites
