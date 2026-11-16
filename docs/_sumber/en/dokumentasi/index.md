---
judul: JawaRustGo++
menu: Introduction
ringkas: A programming language with four execution engines, diagnostics that explain the way out, and a library big enough for real applications.
---
JawaRustGo++, JRG for short, is a programming language written in Rust. Its code looks like Rust and Go, it is
dynamically typed, and its messages explain what went wrong and where. These docs are for version **0.6.0**.

```jrg halo.jrg
pub fn main() {
    let nama = "Nusantara"
    wok("""sugeng rawuh, ${nama}!""")
}
```
```keluaran
sugeng rawuh, Nusantara!
```

`wok` prints to the screen. The **Run** button above a code block runs the example right in the browser, and **Edit**
makes it editable.

## What is inside

- **Four engines.** The interpreter (the default), a bytecode VM (`--vm`), a JIT that compiles hot code while it runs
  (`--jit`), and native compilation through LLVM (`--emit-llvm`). The same program prints the same output on all four.
  A fifth engine, WebAssembly, is being built so JRG programs run on the web.
- **Values never share.** Copying, passing, and returning a value always yields a copy, so there is no garbage
  collector and no surprising change from a distance.
- **Errors you can read.** Every error names the file, line, column, the piece of code, and usually the way out.
  Messages are in Indonesian today; English messages are being written.
- **A complete library.** {BUILTIN} built-in functions and the `std/` modules: text, collections, files, JSON, HTTP, a
  web server, SQLite, canvas, sound, numerics, and a 2D game engine.
- **Other languages.** Call C, C++, Rust, Go, Python, JavaScript, PHP, Lua, Java, Kotlin, .NET, and WebAssembly.
- **Tools.** `jrgm` for projects, dependencies, tests, formatting, and linting; a terminal editor; a REPL; a language
  server (LSP); and an Android APK packager.

## Where to start

| If you… | Read |
|---|---|
| just want to try it | [Install](pasang.html), then the [Quick tour](tur.html) |
| already know another language | [Values and types](nilai.html) through [Concurrency](konkuren.html) |
| have never programmed | the [Matematika × JRG](../matematika_x_jrg/index.html) course (in Indonesian) |
| are looking for a specific function | the [Built-in library](pustaka.html), or press <kbd>/</kbd> to search |
| want to start a project | [jrgm](jrgm.html) |

## Status

JRG is young. The interpreter and the VM run the whole language; the LLVM backend runs every conformance file but
not yet the whole library. The full list of what is not finished is in [Not finished yet](batasan.html), and the
changes in this version are in the [Release notes](rilis.html).
