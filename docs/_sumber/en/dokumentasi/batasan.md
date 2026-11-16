---
judul: Not done yet
ringkas: The known shortcomings of JRG 0.6.0.
---
## Language

- **Enum discriminants** (`enum E { A = 5 }`) do not exist yet. Map variants to numbers with a `match`.
- **No `async`/`await`.** Concurrency uses goroutines and channels; each goroutine is an OS thread, so it suits tens to
  hundreds of tasks, not millions.
- **Type annotations are not enforced.** The type name is checked, the type itself is not.
- **`for` with one name gives the index**, ranges included. This is deliberate, but often surprising; see
  [Control flow](kendali.html#for).
- **A closure that calls itself** is never freed from memory. Use a named `fn`.
- **Error messages are in Indonesian.** English messages are on the way (`jrgm language en`).

## Backend LLVM {#backend-llvm}

The interpreter and the VM run the whole language. The LLVM backend runs every conformance file, and refuses (with
its reason) a few shapes:

- spread (`f(...v)`) and named arguments (`f(a: 1)`) to a closure held in a variable or parameter (to a named
  function they work);
- calling a function stored as a tagged value (for example inside a mixed array);
- some `std/` modules written very dynamically;
- integers above 64 bits.

Canvas programs packaged as an APK use this backend, so the same limits apply there.

## Library

- **Time is always UTC.** `time::format` does not know time zones yet.
- **Canvas windows** need a build with `--features canvas`; by default it draws to a file or to the browser.
- **No central package registry yet.** Dependencies come from local folders or git repositories.
- **Java/Kotlin FFI is not available inside an APK yet.**

## Tools and platforms

- **Termux**: an `android-aarch64` binary has shipped since 0.2.0 and installs through the same installer, but it has
  not been tried on a real phone.
- **Windows**: binaries are built and tested in CI, but the terminal editor and REPL line editing on Windows have not
  been tried on a real machine.
- **JRG IDE on phones**: the Native engine only exists in the Terminal version (through clang in Ubuntu), and does not
  support `dok::`, `crypto::`, `re::`, and `ptr::` there yet.
- **WebAssembly output** (the fifth engine) is still being built and is not in a release.

If you find a wrong result with no error message, that counts as a serious bug. Report it on
[GitHub](https://github.com/Kodok-Kejepit/jrg/issues) with the program.
