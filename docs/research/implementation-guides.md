# Implementation Guides

## GCC family docs

- **Using the GNU Compiler Collection (GCC)**
  - URL: https://gcc.gnu.org/onlinedocs/gcc/
  - Authority tier: primary toolchain
  - Required or optional: required
  - Consult when: designing the compile driver, preprocessing flow, and compilation-stage behavior
  - Related chapters: 1, 9, 10, 19, 20
  - Why it matters: the driver/toolchain model in this project depends heavily on GCC-compatible flows

- **The C Preprocessor**
  - URL: https://gcc.gnu.org/onlinedocs/cpp/
  - Authority tier: primary toolchain
  - Required or optional: required
  - Consult when: wiring preprocessing and understanding line markers and macro behavior
  - Related chapters: 1, 10, 16, 17
  - Why it matters: preprocessing is external, but driver correctness depends on understanding it

- **GCC Internals**
  - URL: https://gcc.gnu.org/onlinedocs/gccint/
  - Authority tier: primary companion
  - Required or optional: recommended
  - Consult when: comparing your pass boundaries and backend structure to a mature compiler
  - Related chapters: 5–20
  - Why it matters: it gives perspective on pass factoring, RTL/IR boundaries, and backend concerns

## Practical parsing / frontend references

- **Eli Bendersky — expression parsing articles**
  - URL: https://eli.thegreenplace.net/
  - Authority tier: high-value secondary
  - Required or optional: recommended
  - Consult when: precedence climbing or recursive descent expression parsing feels awkward
  - Related chapters: 2–4
  - Why it matters: these posts explain the engineering intuition behind expression parsing strategies very clearly

- **LLVM Kaleidoscope tutorial**
  - URL: https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/
  - Authority tier: high-value secondary
  - Required or optional: optional
  - Consult when: you want another example of frontend decomposition and IR-oriented thinking
  - Related chapters: 1–4, 19
  - Why it matters: it is not a C compiler, but it is a useful contrast in staged compiler construction
