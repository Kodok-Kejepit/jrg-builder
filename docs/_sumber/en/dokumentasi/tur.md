---
judul: Quick tour
ringkas: Your first ten minutes with JRG, from one line to a program with tests.
---
## One line

The shortest `.jrg` file holds just the statement:

```jrg hello.jrg
wok("hello")
```
```keluaran
hello
```

```sh
jrgc hello.jrg
```

A file without `package` is a *script*: top-level lines run from top to bottom. Functions, structs, and other types
can be declared anywhere in the file.

```jrg greet.jrg
wok(greet("Ani"))

fn greet(name) {
    return "hello, " + name
}

let numbers = [5, 3, 8, 1]
wok("sorted:", arr::sort(numbers))
```
```keluaran
hello, Ani
sorted: [1, 3, 5, 8]
```

## A program with main

For bigger programs, write `package` and a `pub fn main()`. That is where the program starts.

```jrg temperature.jrg
package main

fn to_fahrenheit(c: float) -> float {
    return c * 9.0 / 5.0 + 32.0
}

pub fn main() {
    for _, c in [0.0, 25.0, 100.0] {
        wok(c, "°C =", to_fahrenheit(c), "°F")
    }
}
```
```keluaran
0.0 °C = 32.0 °F
25.0 °C = 77.0 °F
100.0 °C = 212.0 °F
```

`wok` takes any number of arguments and separates them with spaces. Its siblings: `wok_` (no newline), `eprint` (to
stderr), and `str::format` for building text.

## Reading input and arguments

`tok()` reads one line from the keyboard. `env::args()` holds the script name, then the arguments written after `--`.

```jrg-cuplikan
let name = tok()
wok("hello,", name)
wok(env::args())
```

```sh
$ jrgc ask.jrg -- one two
Ani
hello, Ani
[ask.jrg, one, two]
```

## The other engines

The same program can run in several ways:

```sh
jrgc temperature.jrg                 # interpreter
jrgc temperature.jrg --vm            # bytecode VM, faster
jrgc temperature.jrg --jit           # VM plus machine code for the hot parts
jrgc temperature.jrg --emit-llvm     # native compilation through clang, then run
jrgc temperature.jrg --emit-llvm -o temperature --no-run   # compile only; the result is ./temperature
```

The differences are explained in [The engines](mesin.html). For everyday work the interpreter is enough.

## Trying things in the REPL

`jrgc` with no file opens the REPL. An expression prints its value right away, and a block that is not closed yet
continues on the next line.

```sh
$ jrgc
jrg> let x = 10
jrg> x * 3
30
jrg> fn square(n) { return n * n }
jrg> square(12)
144
jrg> :doc arr::sort
arr::sort(a) -> array
  Salinan terurut menaik; `a` sendiri tidak berubah.
jrg> :simpan try.jrg
```

`:cari word` searches the built-in functions, `:doc name` explains one, and `:simpan` writes the session out as a
runnable `.jrg` file. Type `:bantuan` for the full list.

## Writing tests

A function named `test_…` or `uji_…` is a test. `--uji` runs all of them instead of `main`.

```jrg-cuplikan calc.jrg
fn add(a, b) { return a + b }

fn test_add() {
    assert_eq(add(2, 3), 5)
}

fn test_wrong() {
    assert_eq(add(2, 2), 5)
}
```

```sh
$ jrgc calc.jrg --uji
  ok    test_add
  GAGAL test_wrong (baris 8) — assert_eq gagal — kiri `4` (int), kanan `5` (int)
--- 2 test: 1 lulus, 1 gagal ---
```

For a project with many test files, use `jrgm test` (see [jrgm](jrgm.html)).

## When something goes wrong

Take this example:

```jrg prices.jrg
let prices = [12000, 8500, 4000]
wok(prices[3])
```
```keluaran
error: index [3] is out of bounds — this array holds 3 elements (valid indices: 0..2, or -1..-3 counting from the end)
 --> prices.jrg:2:5
  |
2 | wok(prices[3])
  |     ^^^^^^^^^
  |
```

An error message says what went wrong, where, and shows the code with a `^` under the problem (here: "index [3] is
out of bounds — this array holds 3 elements; valid indices are 0..2, or -1..-3 counting from the end"). Above it there
is usually a one-line greeting (dropped here with `--sopan`); how to set its tone is in [Diagnostics](diagnostik.html).
Messages are in Indonesian today; English messages are being written.

## Next

- [Values and types](nilai.html): numbers, text, arrays, dicts, and conversions.
- [Built-in library](pustaka.html): what is already available.
- [jrgm](jrgm.html): projects with many files.
