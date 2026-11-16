---
judul: jrgm
ringkas: Managing projects: creating, running, testing, formatting, checking, dependencies, and releases.
---
## A new project

```sh
$ jrgm new calc
Created `calc` (bin) package
$ cd calc
$ jrgm run
Halo dari JawaRustGo++!
```

What it contains:

```teks
calc/
├── jrgm.toml        project manifest
├── src/main.jrg     entry point
└── tests/dasar.jrg  tests
```

```toml jrgm.toml
[package]
name        = "calc"
version     = "0.1.0"
edition     = "jrg2025"
entry       = "src/main.jrg"

[dependencies]
helper = { path = "../helper" }
```

`jrgm new name --kind lib` makes a library, `--kind web` a web application, and `--android` adds an `[android]`
section. Projects in other languages work too: `jrgm new tool --bahasa python` (also `javascript`, `c`, `cpp`,
`rust`, `go`, `java`, `php`, `lua`, `ruby`, `sh`); `run`, `build`, `test`, and `fmt` then use that language's
tools.

## Commands

| Command | What it is for |
|---|---|
| `jrgm run [--vm \| --jit] [--watch]` | run the project; `--jit` through the VM that compiles the hot parts, `--watch` reruns on every change |
| `jrgm test [--filter word] [--vm \| --jit]` | run the tests in `tests/*.jrg` |
| `jrgm build` | check every file of the project and its dependencies |
| `jrgm check` | check the entry point only |
| `jrgm fmt [--check]` | tidy spacing and indentation in every file |
| `jrgm lint [--ketat]` | errors plus warnings |
| `jrgm exec file` | run one file without a project (`.jrg`, `.c`, `.py`, `.rs`, `.go`, …) |
| `jrgm clean` | delete build output |
| `jrgm lsp` | a language server for editors |
| `jrgm apk`, `jrgm kunci`, `jrgm siapkan android` | Android apps, see [Android apps](android.html) |
| `jrgm language [en \| id]` | the language of JRG's messages, for every later run (see [Diagnostics](diagnostik.html)) |

## Tests

A function named `test_…` or `uji_…` in `tests/*.jrg` is a test. Each file runs in its own process with a time
limit, so one stuck test does not hold up the others.

```sh
$ jrgm test
[jrgm] 1 test in 1 file

== tests/dasar.jrg (1 test)
  ok    test_contoh
--- 1 test: 1 passed, 0 failed ---

[jrgm] all 1 test passed
```

## Lint

```sh
$ jrgm lint
warning: `import "std/set"` is unused — there is no `set::` in this file
 --> src/main.jrg:2:1
  |
2 | import "std/set"
  | ^^^^^^^^^^^^^^^^
  |
  = help: remove the import line

warning: `x` is given a value but never read
 --> src/main.jrg:7:9
  |
7 |     let x = 5
  |         ^
  |
  = help: remove it, or prefix it with `_` (`_x`) if it is intentional

[jrgm] lint: 2 files, 0 errors, 2 warnings
```

The warnings: unused imports, functions never called, variables never read, integer literals in a float slot, and
files that are not tidy. `--ketat` (strict) makes warnings fail the command.

## Dependencies

A dependency can be a local folder or a git repository. Git serves as the registry: a version is a `v<version>` tag,
and the commit used is locked in `jrgm.lock`.

```sh
jrgm add helper --path ../helper
jrgm add colors --git https://github.com/example/colors-jrg --version 0.2.0
jrgm add tools --git https://github.com/example/tools --rev main --dev
jrgm update            # move every git dependency forward
jrgm update colors
jrgm remove helper
jrgm tree
```

```sh
$ jrgm tree
calc 0.1.0
└── helper 0.1.0
```

Once added, a dependency's module can be used right away with `import "helper"`. A dependency cycle, and two
dependencies exposing the same module name, are refused with the chain named.

## Publishing

```sh
jrgm publish --dry-run   # check first
jrgm publish             # make a v<version> tag from jrgm.toml and push it to the git remote
```

Another project can then use it with `jrgm add name --git url --version x.y.z`. `jrgm add name` without `--path` or
`--git` is refused, because there is no central registry yet to look a bare name up in.
