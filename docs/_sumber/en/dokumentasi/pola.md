---
judul: Patterns and match
menu: Patterns
ringkas: Taking values apart and choosing a branch by their shape.
---
## match

`match` tries each arm from the top and runs the first one that matches. `_` matches anything.

```jrg match.jrg
fn jelaskan(x) {
    return match x {
        0 => "nol",
        n @ 1..=9 => "satu digit: " + str(n),
        n if n < 0 => "negatif",
        _ => "besar",
    }
}
wok(jelaskan(0), jelaskan(7), jelaskan(-3), jelaskan(42))
```
```keluaran
nol satu digit: 7 negatif besar
```

| Pattern | Matches |
|---|---|
| `0`, `"teks"`, `'a'`, `true` | exactly that value |
| `1..=9`, `'a'..='z'` | a value inside the range |
| `a \| b` | either of them |
| `n` | anything, binding it to `n` |
| `n @ pattern` | `pattern`, binding the whole value to `n` |
| `pattern if condition` | `pattern`, when `condition` is true |
| `(a, b)` | a two-element tuple |
| `[a, b]`, `[]`, `[head, ...rest]`, `[a, ..., z]` | an array of that shape |
| `Titik { x, y: 0 }` | a struct whose field `y` is 0 |
| `Bentuk::Lingkaran(r)`, `Some(v)`, `Err(e)` | an enum variant, unpacking its contents |

## Array patterns

```jrg pola-array.jrg
fn bentuk(a) {
    return match a {
        [] => "kosong",
        [x] => "satu: " + str(x),
        [pertama, ..., terakhir] => str(pertama) + " sampai " + str(terakhir),
    }
}
wok(bentuk([]))
wok(bentuk([5]))
wok(bentuk([1, 2, 3, 4]))
```
```keluaran
kosong
satu: 5
1 sampai 4
```

## Patterns in let and for

Patterns also work in `let`, in a `for` header, and in function parameters.

```jrg bongkar.jrg
let (a, b) = (1, 2)
let [p, q, ...sisa] = [10, 20, 30, 40]
wok(a, b, p, q, sisa)

let nilai = [("Ani", 90), ("Budi", 75)]
for _, (nama, skor) in nilai {
    wok(nama, skor >= 80 $ "A" : "B")
}

struct Titik { pub x: int, pub y: int }
let Titik { x, y } = Titik { x: 3, y: 4 }
wok(x * y)
```
```keluaran
1 2 10 20 [30, 40]
Ani A
Budi B
12
```

## let … else

When the pattern might not match, `let pattern = value else { … }` runs the `else` block. That block must leave the
flow (`return`, `break`, or `continue`), so after that line the names are guaranteed to be bound.

```jrg let-else.jrg
fn harga(menu, nama) {
    let Some(h) = dict::get(menu, nama) else {
        return "tidak ada " + nama
    }
    return nama + " Rp" + str(h)
}

let menu = {"bakso": 15000, "soto": 12000}
wok(harga(menu, "soto"))
wok(harga(menu, "rendang"))
```
```keluaran
soto Rp12000
tidak ada rendang
```

## ??

`a ?? b` opens an `Option`: its contents when `Some`, `b` when `None`. The right side is only evaluated when needed.

```jrg cadangan.jrg
let warna = {"langit": "biru"}
wok(dict::get(warna, "langit") ?? "?", dict::get(warna, "rumput") ?? "?")
wok([3, 9, 4].find(|x| x > 5) ?? 0, [1, 2].find(|x| x > 5) ?? 0)
```
```keluaran
biru ?
9 0
```

## Bare variants

In the file that declares an enum, its variants may be written without the type name, as long as no other enum in
that file has a variant with the same name.

```jrg telanjang.jrg
enum Lampu { Merah, Kuning, Hijau }

fn boleh_jalan(l) {
    return match l {
        Hijau => true,
        _ => false,
    }
}
wok(boleh_jalan(Hijau), boleh_jalan(Lampu::Merah))
```
```keluaran
true false
```
