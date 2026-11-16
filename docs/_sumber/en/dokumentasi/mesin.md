---
judul: Four engines
ringkas: The interpreter, the bytecode VM, the JIT, and the LLVM backend, how all four are kept in agreement, and the fifth engine on the way.
---
All four engines share the same front end (lexer and parser), so their syntax and syntax-error messages are identical.
They differ in how a program is run.

| | Interpreter | VM | JIT | LLVM native |
|---|---|---|---|---|
| command | `jrgc f.jrg` | `jrgc f.jrg --vm` | `jrgc f.jrg --jit` | `jrgc f.jrg --emit-llvm` |
| how it works | walks the syntax tree directly | translates to bytecode, then runs it | a VM that compiles the hot parts to machine code while running | translates to LLVM IR, which `clang` compiles to machine code |
| language | all of it | all of it | all of it | every conformance file, not the whole library yet |
| goroutines | OS threads | OS threads | OS threads | coroutines on one thread |
| good for | trying things, scripts | programs that need more speed | heavy loops with no compile step | finished programs, standalone binaries, canvas APKs |

## One program, one meaning

A program that runs on two engines must print exactly the same output, including its error messages, their column,
and the moment the error happens. When two engines answer differently, one of them is broken and gets fixed; it is
never left as "an implementation difference".

This is guarded by tables of programs that are run on several engines at once, with their output compared:

| Table | Contents | Count |
|---|---|---|
| interpreter = VM | programs whose output must be identical | 617 |
| rejected by both | programs both engines must reject with the right message | 365 |
| interpreter = LLVM | programs whose output must be identical | 311 |
| not yet on LLVM | programs LLVM refuses with a clear reason | 28 |
| conformance | complete semantic test files | 11 of 11 on all three engines |

These figures are from version 0.2.0; the whole test suite held 851 tests then. No program is known that runs on two
engines and answers differently.

## Interpreter

The default engine. It starts fastest, and its error messages carry a call trace. Goroutines here are real
operating-system threads.

## VM

The program is first translated into compact bytecode instructions, then run. For heavy loops and function calls, the
VM is much faster than the interpreter. Its errors point at exactly the same line and column.

## JIT

`--jit` runs the program on the VM while recording which parts run often. Hot functions and loops are compiled to
machine code mid-run, including ones that hold structs, enums, arrays, and text. When machine code meets a case
outside its guess (a 64-bit overflow, a type that changes), it hands the rest to the VM at exactly that instruction,
so its answers and error messages are exactly those of `--vm`.

`JRG_JIT_LOG=ringkas` reports the functions compiled, the ones refused and why, and how many times machine code
handed back to the VM.

## LLVM native

`--emit-llvm` produces a standalone native program. For number crunching and function calls, its speed is on a par
with C (see [Benchmarks](benchmark.html)).

Worth knowing:

- Integers here are 64-bit, not 128. Overflow is still checked, and a number that does not fit is refused, naming
  `i128`.
- Concurrency runs as coroutines on one thread: `spawn`, channels, and `select` work, but not in parallel. A deadlock
  is reported, not left hanging.
- Unsupported features are refused at compile time with their reason, never producing a wrong program. The list is in
  [Not done yet](batasan.html#backend-llvm).

## WebAssembly (the fifth engine, in progress)

A fifth engine is being built: it compiles a JRG program into a WebAssembly module, so the same program runs in a
browser or under Node without an interpreter shipped alongside it. It goes through the same LLVM IR as the native
backend, aimed at the `wasm32` (WASI) target, so it inherits that backend's meaning, optimisations, and test tables: every
program in the interpreter = LLVM table becomes a parity check for WebAssembly too. It is not part of a release yet.

This is different from the [Practice](../index.html) engine on this site, which runs the JRG *interpreter* compiled to
WebAssembly, and from `wasm::`, which runs *other people's* `.wasm` modules from JRG.
