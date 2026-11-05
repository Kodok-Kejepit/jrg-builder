---
judul: Modul dan import
menu: Modul
ringkas: Memecah program jadi beberapa berkas, dan memakai pustaka std/.
---
## Satu berkas, satu modul

Tiap berkas `.jrg` adalah sebuah modul. Berkas lain memakainya lewat `import`, dan isinya dipanggil dengan awalan
`nama::`.

```jrg-cuplikan geometri.jrg
package geometri

pub const PI = 3.14159

pub fn luas_lingkaran(r: float) -> float {
    return PI * r * r
}

fn _bantu() { return 1 }

pub struct Titik { pub x: float, pub y: float }

impl Titik {
    pub fn jarak(self, lain: Titik) -> float {
        return math::sqrt((self.x - lain.x) ** 2.0 + (self.y - lain.y) ** 2.0)
    }
}
```

```jrg-cuplikan main.jrg
package main

import "geometri"
import "geometri" as g

pub fn main() {
    wok(geometri::luas_lingkaran(2.0))
    let a = g::Titik { x: 0.0, y: 0.0 }
    let b = geometri::Titik { x: 3.0, y: 4.0 }
    wok(a.jarak(b), g::PI)
}
```

```sh
$ jrgc main.jrg
12.56636
5.0 3.14159
```

## Apa yang terlihat dari luar

- Yang bisa dipakai pengimpor: fungsi, `const`/`let` tingkat atas, struct, enum beserta variannya, dan method tipe
  milik modul itu.
- Nama yang diawali `_` privat. `geometri::_bantu()` ditolak dengan pesan yang menyebut alasannya.
- `var` tingkat atas tetap milik modulnya. Kalau nilainya perlu dibaca dari luar, sediakan fungsi pembacanya.
- Sebuah berkas cuma melihat modul yang ia import sendiri. Modul yang diimport oleh modul lain tidak ikut terlihat.

Satu berkas tetap satu modul berapa kali pun ia diimport, dengan alias apa pun.

## Di mana import mencari

`import "geometri"` mencari `geometri.jrg` di folder berkas yang mengimpor, lalu di folder dependensi proyek (lihat
[jrgm](jrgm.html)), lalu di folder yang ditambahkan dengan `jrgc -I folder`. Jalur bertingkat juga boleh:
`import "alat/teks"` memuat `alat/teks.jrg` sebagai modul `teks`.

## Pustaka std/

Empat belas modul pustaka standar ditulis dalam JRG dan tertanam di dalam `jrgc`, jadi bisa dipakai di mana saja tanpa
memasang apa pun.

```jrg std.jrg
import "std/set"
import "std/csv"

let s = set::from([3, 1, 3, 2])
wok(s.len(), s.has(3))

let baris = csv::parse("nama,kota\n\"Ani, S.Kom\",Bandung")
wok(baris[1][0])
```
```keluaran
3 true
Ani, S.Kom
```

Daftar modulnya ada di [Pustaka bawaan](pustaka.html#modul-std).

## Nama bawaan yang dibayangi

Kalau program punya fungsi bernama sama dengan fungsi bawaan, milik program yang menang. `::nama` selalu menunjuk
fungsi bawaan bahasa.

```jrg bayang.jrg
fn len(a, b, c) { return a + b + c }

wok(len(1, 2, 3), ::len("abc"))
```
```keluaran
6 3
```
