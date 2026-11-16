---
judul: Release notes 0.6.0
menu: Release notes
ringkas: The sixth release of JawaRustGo++.
---
0.6.0 is JRG's sixth release. It makes `--jit` much faster for code that lives on structs and
`map`/`filter`/`reduce`, brings `--jit` to `jrgm`, and tightens some resource limits. Three changes can make old
programs behave differently; they are listed below.

## Faster

`--jit` now computes small formula methods right where they are called. `x = x.m(dt)` that builds a new `Self` from
a float formula over its fields no longer calls the method and builds a new struct: the fields are read, computed in
registers, and written back into the same struct. A method that only returns a float formula over its fields
(`rec.score()`) is computed in place too. Callbacks of `map`/`filter`/`reduce` and friends are called without setting
up a new frame for every element. Wall-clock time against 0.5.0:

| program | 0.5.0 | 0.6.0 |
|---|---|---|
| `struct_particles` | 12.9 s | 3.3 s |
| `mixed_report` | 2.2 s | 1.4 s |
| `hof_closure` | 0.66 s | 0.44 s |
| `string_scan` | 0.44 s | 0.32 s |

The answers stay exactly the same as the interpreter's and the VM's, including when the struct is shared with another
variable, a field changes type, or a callback fails halfway through an array.

## New

- `jrgm run --jit`, `jrgm exec --jit`, and `jrgm test --jit`: a project can be run and tested through the fourth
  engine, the same as `jrgc --jit`. `--watch` passes it on too.

## May break old programs

- `json::stringify`, `json::pretty`, and `dok::json_encode` refuse library handles (`Ptr`, `Future`, `DokDb`, …),
  like `Mutex`. They used to produce `{"id":1}`.
- `dok::serve` WebSockets release a client that stays silent and does not answer a `ping` (60 + 30 seconds), close a
  message that is not complete within 30 seconds plus 1 second per 16 KiB, and count half-received messages against
  the same 256 MiB budget as HTTP bodies.
- `wokwok::` refuses broadcast and `matmul` results above 256 MiB, like its array builders. Two small arrays could
  once ask for tens of GB and kill the process.

## Fixed

- `jrgm siapkan android` skips catalogue entries whose file or folder name is not a plain name (`..`, `/`).

The full details are in [CHANGELOG.md](https://github.com/Kodok-Kejepit/jrg/blob/main/CHANGELOG.md).

## Installing

See [Install](pasang.html). Binaries are available for Linux (x86_64, aarch64), macOS (Intel, Apple Silicon), Windows
(x86_64), and Termux, each with a SHA-256 checksum. The archive already carries the runtime for `--emit-llvm`, so the
only thing to install yourself is `clang`.

## Not yet

See [Not done yet](batasan.html).
