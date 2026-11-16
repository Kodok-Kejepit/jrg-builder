---
judul: Quick tour
ringkas: Your first ten minutes with JRG, from one line to a program with tests.
---
## One line

The shortest `.jrg` file holds just the statement:

```jrg halo.jrg
wok("halo")
```
```keluaran
halo
```

```sh
jrgc halo.jrg
```

A file without `package` is a *script*: top-level lines run from top to bottom. Functions, structs, and other types
can be declared anywhere in the file.

```jrg sapa.jrg
wok(sapa("Ani"))

fn sapa(nama) {
    return "halo, " + nama
}

let angka = [5, 3, 8, 1]
wok("terurut:", arr::sort(angka))
```
```keluaran
halo, Ani
terurut: [1, 3, 5, 8]
```

## A program with main

For bigger programs, write `package` and a `pub fn main()`. That is where the program starts.

```jrg suhu.jrg
package main

fn ke_fahrenheit(c: float) -> float {
    return c * 9.0 / 5.0 + 32.0
}

pub fn main() {
    for _, c in [0.0, 25.0, 100.0] {
        wok(c, "°C =", ke_fahrenheit(c), "°F")
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
let nama = tok()
wok("halo,", nama)
wok(env::args())
```

```sh
$ jrgc tanya.jrg -- satu dua
Ani
halo, Ani
[tanya.jrg, satu, dua]
```

## The other engines

The same program can run in several ways:

```sh
jrgc suhu.jrg                 # interpreter
jrgc suhu.jrg --vm            # bytecode VM, faster
jrgc suhu.jrg --jit           # VM plus machine code for the hot parts
jrgc suhu.jrg --emit-llvm     # native compilation through clang, then run
jrgc suhu.jrg --emit-llvm -o suhu --no-run   # compile only; the result is ./suhu
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
jrg> fn kuadrat(n) { return n * n }
jrg> kuadrat(12)
144
jrg> :doc arr::sort
arr::sort(a) -> array
  Salinan terurut menaik; `a` sendiri tidak berubah.
jrg> :simpan coba.jrg
```

`:cari word` searches the built-in functions, `:doc name` explains one, and `:simpan` writes the session out as a
runnable `.jrg` file. Type `:bantuan` for the full list.

## Writing tests

A function named `test_…` or `uji_…` is a test. `--uji` runs all of them instead of `main`.

```jrg-cuplikan hitung.jrg
fn tambah(a, b) { return a + b }

fn test_tambah() {
    assert_eq(tambah(2, 3), 5)
}

fn test_salah() {
    assert_eq(tambah(2, 2), 5)
}
```

```sh
$ jrgc hitung.jrg --uji
  ok    test_tambah
  GAGAL test_salah (baris 8) — assert_eq gagal — kiri `4` (int), kanan `5` (int)
--- 2 test: 1 lulus, 1 gagal ---
```

For a project with many test files, use `jrgm test` (see [jrgm](jrgm.html)).

## When something goes wrong

Take this example:

```jrg harga.jrg
let harga = [12000, 8500, 4000]
wok(harga[3])
```
```keluaran
error: index [3] di luar batas — array ini isinya 3 elemen (index valid: 0..2, atau -1..-3 kalau mau hitung dari belakang)
 --> harga.jrg:2:5
  |
2 | wok(harga[3])
  |     ^^^^^^^^
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
