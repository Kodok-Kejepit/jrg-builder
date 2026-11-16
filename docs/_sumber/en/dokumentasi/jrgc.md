---
judul: jrgc
ringkas: Running, checking, testing, and compiling .jrg files.
---
## Running

```sh
jrgc program.jrg              # interpreter
jrgc program.jrg --vm         # VM bytecode
jrgc program.jrg --jit        # VM plus machine code for the hot parts
jrgc program.jrg --emit-llvm  # native, through clang
jrgc program.jrg -- a b c     # arguments for the program, read through env::args()
jrgc program.jrg --watch      # rerun every time the file changes
jrgc                          # REPL
```

## Checking without running

```sh
jrgc program.jrg --periksa    # parse and check the file and all its imports; silent when clean
jrgc program.jrg --fmt        # print the tidied version to stdout (the file is not changed)
jrgc program.jrg --emit-ast   # print the syntax tree
jrgc program.jrg --emit-tokens
```

`--periksa` (check) finds syntax errors, undeclared names, and assignments to things that are not variables, before a
single line runs.

## Tests and profiling

```sh
jrgc calc.jrg --uji                     # run every fn test_*/uji_*
jrgc calc.jrg --uji --uji-filter add
jrgc program.jrg --profile              # time per function after the program ends
jrgc program.jrg --profile-json p.json  # the JSON version
```

## Finding functions

```sh
jrgc --cari word        # built-in names containing "word"
jrgc --doc str::split   # signature and a short explanation
jrgc --daftar-builtin   # every name, one per line
```

## Native compilation

`--emit-llvm` translates the program to LLVM IR and hands it to `clang`. The native runtime is built once with cargo
and cached.

```sh
jrgc program.jrg --emit-llvm -o program --no-run   # compile only
./program

jrgc program.jrg --emit-llvm -O3 --native-cpu --lto thin
jrgc program.jrg --emit-llvm --full-power          # fastest preset, for this CPU only
jrgc program.jrg --emit-llvm --target aarch64-unknown-linux-gnu
jrgc program.jrg --apk app.apk                     # an Android APK from one file
```

| Flag | What it is for |
|---|---|
| `-O 0..3` | optimisation level (default 2) |
| `--lto auto\|off\|thin\|full`, `--lld` | link-time optimisation |
| `--native-cpu`, `--fast-math`, `--aggressive` | faster, each with its own conditions |
| `--pgo-generate`, `--pgo-use` | profile-guided optimisation |
| `--strip`, `--no-gc-sections` | binary size |

The native backend does not accept the whole language yet; what is missing is refused with its reason. See
[The engines](mesin.html).

## WebAssembly (in progress)

A fifth engine is being built: it compiles a JRG program to a WebAssembly module, through the same LLVM IR as
`--emit-llvm`, so the program can run in a browser or under Node. It is not released yet; the
[engines page](mesin.html) is where it will be announced.

## Error messages

```sh
jrgc program.jrg --nada hangat    # sopan, galak, judes, hangat, cool, lawak
jrgc program.jrg --sopan          # no greeting line, good for CI
JRG_GALAT_KE=errors.txt jrgc program.jrg  # also write each error to a file: file, line, column, message
```

The details are in [Diagnostics](diagnostik.html).

## Other

| Command | What it is for |
|---|---|
| `jrgc --sunting file.jrg` | a text editor in the terminal, see [Editor](editor.html) |
| `jrgc --lsp` | a language server for editors (VS Code, Neovim) |
| `jrgc -I folder` | an extra folder to search for `import` |
| `jrgc --jrgm …` | the same `jrgc` acting as `jrgm` |
| `-v` | print every stage it works through |
