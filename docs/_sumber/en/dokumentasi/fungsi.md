---
judul: Functions and closures
ringkas: Declaring functions, flexible parameters, functions as values, and closures.
---
## Declaring functions

```jrg fungsi.jrg
fn luas(p: int, l: int) -> int {
    return p * l
}

fn keliling(p, l) {
    return 2 * (p + l)
}

fn kali_dua(x) = x * 2

wok(luas(3, 4), keliling(3, 4), kali_dua(5))
```
```keluaran
12 14 10
```

Type annotations on parameters and the return value may be written or left out. `fn f(x) = expression` is the short
form for a function that only returns one expression. A function may be called before its line, and a function may be
declared inside another function (but it does not see the outer function's locals; use a closure for that).

## Parameters

```jrg parameter.jrg
fn sapa(nama, salam = "halo") {
    return salam + ", " + nama
}

fn total(angka...) {
    return arr::sum(angka)
}

fn bagi(pembilang, penyebut) {
    return pembilang / penyebut
}

wok(sapa("Ani"), sapa("Budi", "selamat pagi"))
wok(total(), total(1, 2, 3))
wok(total(...[4, 5, 6]))
wok(bagi(penyebut: 2, pembilang: 10))
```
```keluaran
halo, Ani selamat pagi, Budi
0 6
15
5
```

- **Defaults** (`salam = "halo"`) are evaluated at call time, and may use earlier parameters.
- **A collector** (`angka...`) takes the remaining arguments as an array; with none left it is `[]`.
- **Spread** (`...arr`) is the reverse: the array's elements become separate arguments.
- **Named arguments** (`penyebut: 2`) may come in any order and mix with positional ones.

A parameter may be a pattern: `fn jarak((x1, y1), (x2, y2))` unpacks two tuples right in the function head.

## Returning several values

Return a tuple, then unpack it at the call site:

```jrg kembali.jrg
fn statistik(xs) {
    return (arr::min(xs), arr::max(xs), arr::sum(xs) / len(xs))
}

kecil, besar, rata := statistik([4, 8, 15, 16, 23, 42])
wok(kecil, besar, rata)
```
```keluaran
4 42 18
```

## Closure

A closure is a nameless function written in place: `|x| x * x`, or `|a, b| { … }` when the body is more than one
expression.

```jrg closure.jrg
let kuadrat = |x| x * x
let nilai = [1, 2, 3, 4, 5]
wok(nilai.map(kuadrat))
wok(nilai.filter(|x| x % 2 == 1))
wok(nilai.reduce(|a, b| a + b, 0))
wok(nilai.map(|x| x * 10).filter(|x| x > 20))
```
```keluaran
[1, 4, 9, 16, 25]
[1, 3, 5]
15
[30, 40, 50]
```

A closure **remembers the variable**, not a copy of its value. A change inside is visible outside, and the other
way round:

```jrg tangkap.jrg
var hitung = 0
let naik = || { hitung += 1 }
naik()
naik()
wok(hitung)

fn pembuat_tambah(n) {
    return |x| x + n
}
let tambah10 = pembuat_tambah(10)
wok(tambah10(5))
```
```keluaran
2
15
```

If you want the closure to take a snapshot of the value when it is created, write `move`:

```jrg move.jrg
var n = 1
let lihat = || n
let foto = move || n
n = 9
wok(lihat(), foto())
```
```keluaran
9 1
```

## Functions as values

Named functions, closures, built-in functions, and enum variant constructors are all values that can be stored and
passed around.

```jrg nilai-fungsi.jrg
fn ganda(x) = x * 2

let langkah = [ganda, |x| x + 1, math::abs]
var v = -5
for _, f in langkah {
    v = f(v)
}
wok(v)
wok([3, -1, 2].map(math::abs), ["a", "b"].map(str::upper))
wok(4 |> ganda |> ganda)
wok(terap(math::max, [3, 9]))
```
```keluaran
9
[3, 1, 2] [A, B]
16
9
```

`x |> f` is the same as `f(x)`, useful for chaining steps from left to right. `terap(f, args)` calls `f` with the
array's elements as its arguments.

Two functions cannot be compared with `==`; JRG refuses, because there is no correct answer to "are these two
functions the same".

## Recursion

```jrg rekursi.jrg
fn fib(n) {
    return n < 2 $ n : fib(n - 1) + fib(n - 2)
}
wok(fib(20))
```
```keluaran
6765
```

Recursion depth is limited by the stack size: about 20 thousand levels on the interpreter, up to a million on `--vm`,
and hundreds of thousands in an `--emit-llvm` binary. Going past it gives an error that names the function, not a
crash. For heavy recursion, use a named `fn`, not a closure that calls itself.
