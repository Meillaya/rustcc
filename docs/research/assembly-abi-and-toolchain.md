# Assembly, ABI, and Toolchain References

- **x86-64 psABI / System V AMD64 ABI**
  - URL: https://gitlab.com/x86-psABIs/x86-64-ABI
  - Authority tier: primary
  - Required or optional: required
  - Consult when: implementing calls, returns, stack alignment, aggregate passing, and helper-library interactions
  - Related chapters: 9, 10, 13, 18, 20
  - Why it matters: this is the calling-convention contract your generated code must obey

- **GNU assembler manual (`as`)**
  - URL: https://sourceware.org/binutils/docs/as.html
  - Authority tier: primary
  - Required or optional: required
  - Consult when: emitting assembly syntax, directives, sections, and symbols
  - Related chapters: 1, 9, 10, 13, 18, 20
  - Why it matters: emitted assembly is only correct if the assembler accepts the exact syntax and directives you generate

- **GNU linker manual (`ld`)**
  - URL: https://sourceware.org/binutils/docs/ld.html
  - Authority tier: primary
  - Required or optional: recommended
  - Consult when: object/link failures appear or custom artifact behavior matters
  - Related chapters: 9, 10, 18
  - Why it matters: linking errors are easier to debug when you understand the linker’s model

- **readelf / objdump docs**
  - URLs: https://sourceware.org/binutils/docs/binutils/readelf.html and https://sourceware.org/binutils/docs/binutils/objdump.html
  - Authority tier: primary tool docs
  - Required or optional: recommended
  - Consult when: inspecting artifacts, symbols, sections, relocations, and disassembly
  - Related chapters: 9–20
  - Why it matters: these tools turn invisible backend mistakes into visible evidence

- **Intel SDM**
  - URL: https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html
  - Authority tier: primary
  - Required or optional: required
  - Consult when: choosing instructions or verifying flag behavior and operand semantics
  - Related chapters: 1–20, especially backend-heavy chapters
  - Why it matters: this is the ISA authority

- **AMD64 manuals**
  - URLs: https://docs.amd.com/v/u/en-US/24592_3.24 and https://docs.amd.com/v/u/en-US/24594_3.37
  - Authority tier: primary
  - Required or optional: recommended
  - Consult when: cross-checking instruction behavior and programming-model wording
  - Related chapters: 9–20
  - Why it matters: AMD’s wording can clarify behavior when Intel-focused material feels too encoding-centric
