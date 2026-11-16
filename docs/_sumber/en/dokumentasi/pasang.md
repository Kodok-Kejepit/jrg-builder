---
judul: Install
ringkas: Use a ready-made binary, or build from source. Either way you get two programs, jrgc and jrgm.
---
Once installed you have two commands:

- `jrgc` runs, checks, and compiles `.jrg` files. Without arguments it opens the REPL.
- `jrgm` manages projects: creating, running, testing, handling dependencies, all the way to packaging an APK.

## Ready-made binaries

Releases for Linux (x86_64, aarch64), macOS (Intel, Apple Silicon), Windows, and Termux are on the
[Releases](https://github.com/Kodok-Kejepit/jrg/releases) page. On Linux, macOS, and Termux, the installer downloads
the matching file, checks its checksum, and puts `jrgc` and `jrgm` in `~/.local/bin` (on Termux: `$PREFIX/bin`):

```sh
curl -fsSL https://raw.githubusercontent.com/Kodok-Kejepit/jrg/main/rilis/pasang.sh | sh

# a specific version, or another folder
sh pasang.sh --versi 0.6.0 --ke /usr/local/bin
```

Make sure `~/.local/bin` is on your `PATH`, then check:

```sh
jrgc --version
```

## From source

All you need is stable [Rust](https://rustup.rs).

```sh
git clone https://github.com/Kodok-Kejepit/jrg
cd jrg
cargo build --release
./target/release/jrgc --version

# install into ~/.cargo/bin
cargo install --path .
```

The first build takes a few minutes and about 2 GB of memory. On a small machine, limit the number of jobs:
`cargo build --release -j 1`.

## Termux (Android)

The same installer command works on Termux. It recognises Termux and uses the Android binary, because an ordinary
Linux binary does not run there.

To build it yourself:

```sh
pkg install rust git
git clone https://github.com/Kodok-Kejepit/jrg
cd jrg
cargo build --release -j 1
```

`-j 1` matters on a phone: a parallel build can run out of memory during optimisation.

## Optional extras

JRG runs without any of the following. Each is only needed by a particular feature.

| Feature | Needs |
|---|---|
| native engine (`--emit-llvm`) | `clang`; if JRG was built from source, also a Rust toolchain to build its runtime once |
| `py::` | `python3` |
| `js::` | `node` |
| `php::` | `php` 8.0 or newer |
| `lua::` | `lua` 5.3 or newer |
| `jvm::`, `kotlin::` | a JDK (`libjvm.so`) |
| `dotnet::` | the .NET runtime, e.g. `dotnet-sdk-8.0` (`libhostfxr`) |
| `dok::db_*` (SQLite) | `libsqlite3` |
| `suara::` (playing sound) | PulseAudio or PipeWire |
| a `Canvas` window | a build with `--features canvas` |
| `jrgm apk` | the Android SDK and NDK, or `jrgm siapkan android` |

## Your first file

```sh
echo 'wok("hello")' > hello.jrg
jrgc hello.jrg
```

Continue with the [Quick tour](tur.html).
