---
judul: Pola dan match
menu: Pola
ringkas: Membongkar nilai dan memilih cabang berdasarkan bentuknya.
---
## match

`match` mencoba tiap cabang dari atas dan menjalankan yang pertama cocok. `_` cocok dengan apa saja.

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

| Pola | Cocok dengan |
|---|---|
| `0`, `"teks"`, `'a'`, `true` | nilai itu persis |
| `1..=9`, `'a'..='z'` | nilai di dalam rentang |
| `a \| b` | salah satunya |
| `n` | apa saja, dan mengikatnya ke `n` |
| `n @ pola` | `pola`, dan mengikat seluruh nilainya ke `n` |
| `pola if syarat` | `pola`, kalau `syarat` benar |
| `(a, b)` | tuple dua elemen |
| `[a, b]`, `[]`, `[kepala, ...sisa]`, `[a, ..., z]` | array dengan bentuk itu |
| `Titik { x, y: 0 }` | struct dengan field `y` bernilai 0 |
| `Bentuk::Lingkaran(r)`, `Some(v)`, `Err(e)` | varian enum, sekaligus membongkar isinya |

## Pola array

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

## Pola di let dan for

Pola juga dipakai di `let`, di kepala `for`, dan di parameter fungsi.

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

Kalau polanya mungkin tidak cocok, `let pola = nilai else { … }` menjalankan blok `else`. Blok itu harus keluar dari
alurnya (`return`, `break`, atau `continue`), jadi sesudah baris itu nama-namanya pasti terisi.

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

`a ?? b` membuka sebuah `Option`: isinya kalau `Some`, `b` kalau `None`. Sisi kanan cuma dihitung kalau dibutuhkan.

```jrg cadangan.jrg
let warna = {"langit": "biru"}
wok(dict::get(warna, "langit") ?? "?", dict::get(warna, "rumput") ?? "?")
wok([3, 9, 4].find(|x| x > 5) ?? 0, [1, 2].find(|x| x > 5) ?? 0)
```
```keluaran
biru ?
9 0
```

## Varian telanjang

Di dalam berkas yang mendeklarasikan sebuah enum, variannya boleh ditulis tanpa nama tipenya, asal tidak ada enum lain
di berkas itu yang punya varian bernama sama.

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
