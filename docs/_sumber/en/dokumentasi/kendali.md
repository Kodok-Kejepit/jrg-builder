---
judul: Control flow
ringkas: Branches, loops, and how to leave both.
---
## if and else

The condition has no parentheses, and the body always has braces. `if` can also be used as a value.

```jrg if.jrg
let n = 7
if n % 2 == 0 {
    wok("genap")
} else if n > 5 {
    wok("ganjil dan besar")
} else {
    wok("ganjil")
}

let jenis = if n > 5 { "besar" } else { "kecil" }
wok(jenis)
```
```keluaran
ganjil dan besar
besar
```

## Ternary

The short form is `condition $ if_true : if_false`. It opens with `$`, not `?`, because `?` is used to propagate
errors (see [Option and Result](galat.html)).

```jrg ternary.jrg
let nilai = 72
wok(nilai >= 75 $ "lulus" : "remedial")
```
```keluaran
remedial
```

## for

`for` has two forms, and the difference matters:

- `for i, x in collection`: `i` is the index, `x` the value. Use `_` for the one you do not need.
- `for i in collection`: **a single name always means the index**, ranges included.

```jrg for.jrg
let buah = ["apel", "jeruk"]
for i, b in buah {
    wok(i, b)
}

var nilai = []
for _, angka in 5..8 {
    nilai = nilai.push(angka)
}
wok(nilai)

var indeks = []
for i in 5..8 {
    indeks = indeks.push(i)
}
wok(indeks)
```
```keluaran
0 apel
1 jeruk
[5, 6, 7]
[0, 1, 2]
```

The last line holds `[0, 1, 2]`, not `[5, 6, 7]`: with one name, `i` is the 0th, 1st, and 2nd index of that
range. If you want the values, write `for _, i in 5..8`.

`a..b` excludes `b`, `a..=b` includes it. The two-name form also works on text (index and character) and dicts (key
and value). A C-style form exists too:

```jrg for-c.jrg
for i := 0; i < 10; i += 3 {
    wok(i)
}
```
```keluaran
0
3
6
9
```

A range in a `for` header is never turned into an array, so `for _, i in 0..1000000000` uses no memory.

## while and loop

```jrg while.jrg
var n = 27
var langkah = 0
while n != 1 {
    n = n % 2 == 0 $ n / 2 : 3 * n + 1
    langkah += 1
}
wok("Collatz 27:", langkah, "langkah")

var tebakan = 0
loop {
    tebakan += 1
    if tebakan * tebakan > 50 { break }
}
wok(tebakan)
```
```keluaran
Collatz 27: 111 langkah
8
```

## Labeled break and continue

`break` and `continue` apply to the nearest loop. To leave an outer loop, give it a label that starts with a single
quote.

```jrg label.jrg
'cari: for _, i in 1..10 {
    for _, j in 1..10 {
        if i * j == 42 {
            wok("ketemu:", i, j)
            break 'cari
        }
    }
}
```
```keluaran
ketemu: 6 7
```

## match

`match` matches a value against patterns and runs the first arm that matches. It is also an expression.

```jrg match.jrg
fn hari(n) {
    return match n {
        0 | 6 => "akhir pekan",
        1..=5 => "hari kerja",
        _ => "bukan hari",
    }
}
wok(hari(0), hari(3), hari(9))
```
```keluaran
akhir pekan hari kerja bukan hari
```

The full set of patterns (tuples, arrays, structs, enums, `if` guards, `@` bindings) is in [Patterns](pola.html).

## defer

`defer` delays a call until the function finishes, including when the function stops because of an error. The
last one registered runs first, and its arguments are frozen when the `defer` line is read.

```jrg defer.jrg
fn proses() {
    var berkas = "laporan.txt"
    defer wok("tutup", berkas)
    defer wok("simpan")
    berkas = "lain.txt"
    wok("kerja dengan", berkas)
}
proses()
```
```keluaran
kerja dengan lain.txt
simpan
tutup laporan.txt
```
