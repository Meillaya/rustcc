# Blogs and Papers

## Blog posts

- **Eli Bendersky — Parsing expressions by precedence climbing**
  - URL: https://eli.thegreenplace.net/2012/08/02/parsing-expressions-by-precedence-climbing
  - Authority tier: high-value secondary
  - Required or optional: recommended
  - Consult when: chapter 3 or 4 expression parsing starts to sprawl
  - Related chapters: 3, 4
  - Why it matters: it is one of the clearest practical explanations of precedence climbing for a hand-written parser

- **Eli Bendersky — A recursive descent parser with an infix expression evaluator**
  - URL: https://eli.thegreenplace.net/2009/03/20/a-recursive-descent-parser-with-an-infix-expression-evaluator
  - Authority tier: high-value secondary
  - Required or optional: optional
  - Consult when: you want to compare precedence climbing with shunting-yard-style hybrids
  - Related chapters: 3, 4
  - Why it matters: it gives a useful contrast to a pure recursive-descent expression strategy

## Papers

- **Register allocation via coloring (Chaitin et al., 1981)**
  - URL: https://research.ibm.com/publications/register-allocation-via-coloring
  - Authority tier: canonical paper
  - Required or optional: recommended
  - Consult when: chapter 20 needs a principled mental model for interference and coloring
  - Related chapters: 20
  - Why it matters: this is the classic graph-coloring framing of register allocation

- **Register allocation and spilling via graph coloring (Chaitin, 1982)**
  - URL: https://research.ibm.com/publications/register-allocation-andamp-spilling-via-graph-coloring
  - Authority tier: canonical paper
  - Required or optional: recommended
  - Consult when: you need more than the basic graph-coloring idea and want spill strategy intuition
  - Related chapters: 20
  - Why it matters: it extends the original coloring model to spill decisions

- **Linear Scan Register Allocation**
  - URL: https://www.ida.liu.se/~chrke55/courses/ACC/summ04/Linear_Scan_RA.html
  - Authority tier: companion summary
  - Required or optional: optional
  - Consult when: you want a simpler allocation model to contrast with graph coloring
  - Related chapters: 20
  - Why it matters: it helps you understand why the book uses a more expressive but heavier-weight allocator model

## How to use this file

Use these sources as concept clarifiers.

- If the question is “what must the compiler do?”, prefer the book, tests, ABI, ISA, and toolchain references.
- If the question is “how can I think about this algorithm more clearly?”, these blog posts and papers are often the fastest help.
