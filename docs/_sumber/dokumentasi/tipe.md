---
judul: Struct, enum, dan trait
menu: Struct, enum, trait
ringkas: Membuat tipe sendiri, memberinya method, dan berbagi perilaku lewat trait.
---
## Struct

Struct mengelompokkan data bernama. Field yang ditulis `pub` bisa dibaca dari luar; sisanya cuma terlihat dari method
tipe itu (dan turunannya). Field boleh punya nilai bawaan, dan field tanpa nilai bawaan wajib diisi.

```jrg struct.jrg
struct Akun {
    pub nama: string,
    pub saldo: int = 0,
    pin: string = "0000",
}

impl Akun {
    fn baru(nama) -> Akun {
        return Akun { nama: nama }
    }

    fn setor(self, n) -> Akun {
        var a = self
        a.saldo += n
        return a
    }

    fn cek_pin(self, p) -> bool {
        return self.pin == p
    }
}

var a = Akun::baru("Ani")
a = a.setor(50)
wok(a.nama, a.saldo, a.cek_pin("1234"))
```
```keluaran
Ani 50 false
```

Fungsi di dalam `impl` tanpa `self` dipanggil lewat nama tipe (`Akun::baru`). Yang memakai `self` dipanggil lewat
nilainya (`a.setor(50)`).

`self` di dalam method adalah **salinan**. Mengubah `self` tidak mengubah nilai milik pemanggil, jadi method yang
mengubah sesuatu mengembalikan nilai baru dan pemanggilnya menugaskan ulang: `a = a.setor(50)`. Bentuk ini tidak
menyalin data besar; lihat [Model memori](memori.html).

### Membuat dari nilai lain

`...` mengisi field yang tidak ditulis dari nilai lain. Ia harus ditulis paling akhir.

```jrg sebar.jrg
struct Titik { pub x: int, pub y: int, pub z: int }

let awal = Titik { x: 1, y: 2, z: 3 }
let geser = Titik { x: 10, ...awal }
wok(geser)
```
```keluaran
Titik {x: 10, y: 2, z: 3}
```

### Pewarisan

Sebuah struct boleh mewarisi satu struct lain. Field dan method induknya ikut.

```jrg waris.jrg
struct Hewan { pub nama: string }

impl Hewan {
    fn kenalan(self) -> string { return "aku " + self.nama }
}

struct Kucing: Hewan { pub warna: string }

let k = Kucing { nama: "Oyen", warna: "oranye" }
wok(k.kenalan(), k.warna)
```
```keluaran
aku Oyen oranye
```

## Enum

Enum adalah tipe yang nilainya satu dari beberapa varian. Varian boleh membawa data, berurutan atau bernama.

```jrg enum.jrg
enum Bentuk {
    Lingkaran(float),
    Persegi(float),
    Segitiga { alas: float, tinggi: float },
}

fn luas(b) {
    return match b {
        Bentuk::Lingkaran(r) => 3.14 * r * r,
        Bentuk::Persegi(s) => s * s,
        Bentuk::Segitiga { alas, tinggi } => alas * tinggi / 2.0,
    }
}

wok(luas(Bentuk::Lingkaran(1.0)))
wok(luas(Bentuk::Segitiga { alas: 4.0, tinggi: 3.0 }))

enum Arah { Utara, Timur, Selatan, Barat }
wok(Arah::Utara, Arah::Utara == Arah::Barat)
```
```keluaran
3.14
6.0
Arah::Utara false
```

`Option` dan `Result` adalah enum biasa yang sudah tersedia; keduanya dibahas di [Option dan Result](galat.html).

## Trait

Trait mendaftar method yang harus dimiliki sebuah tipe. Method tanpa badan wajib disediakan tiap `impl`; method dengan
badan menjadi bawaan yang boleh ditimpa.

```jrg trait.jrg
trait Bersuara {
    fn suara(self) -> string
    fn sapa(self) -> string = self.nama + " bilang " + self.suara()
}

struct Anjing { pub nama: string }
struct Bebek { pub nama: string }

impl Bersuara for Anjing {
    fn suara(self) -> string { return "guk" }
}

impl Bersuara for Bebek {
    fn suara(self) -> string { return "kwek" }
    fn sapa(self) -> string { return self.nama + " cuma diam" }
}

wok(Anjing { nama: "Doggo" }.sapa())
wok(Bebek { nama: "Donal" }.sapa())
```
```keluaran
Doggo bilang guk
Donal cuma diam
```

`impl` yang lupa menyediakan method wajib ditolak sebelum program jalan.

## Generik

Fungsi dan struct boleh punya parameter tipe. Karena JRG diketik dinamis, parameter tipe dan batas trait-nya
(`<T: Bersuara>`) adalah dokumentasi: namanya diperiksa, isinya tidak ditegakkan.

```jrg generik.jrg
fn pertama<T>(xs: []T) -> T {
    return xs[0]
}

struct Kotak<T> { pub isi: T }

wok(pertama([3, 4]), pertama(["a"]), Kotak { isi: 5.5 }.isi)
```
```keluaran
3 a 5.5
```

## Operator dan tampilan buatan sendiri

Sebuah tipe bisa memberi arti pada operator dengan method bernama khusus, dan menentukan cara ia dicetak lewat
`to_string`.

```jrg operator.jrg
struct Uang { pub rp: int }

impl Uang {
    fn add(self, o) -> Uang { return Uang { rp: self.rp + o.rp } }
    fn cmp(self, o) -> int { return self.rp - o.rp }
    fn to_string(self) -> string { return "Rp" + str(self.rp) }
}

let kopi = Uang { rp: 18000 }
let roti = Uang { rp: 12000 }
wok(kopi + roti, kopi > roti)
wok([kopi, roti], """total ${kopi + roti}""")
```
```keluaran
Rp30000 true
[Rp18000, Rp12000] total Rp30000
```

| Method | Operator |
|---|---|
| `add` `sub` `mul` `div` `rem` `pow` | `+` `-` `*` `/` `%` `**` |
| `bit_and` `bit_or` `bit_xor` `shl` `shr` | `&` `\|` `^` `<<` `>>` |
| `neg` `not` `bit_not` | `-x` `!x` `~x` |
| `eq` | `==` `!=` |
| `cmp` (mengembalikan bilangan bertanda) | `<` `<=` `>` `>=` |

`&&` dan `||` tidak bisa ditimpa. `to_string` dipakai di semua tempat sebuah nilai ditampilkan: `wok`, template,
penggabungan teks, dan nilai yang ada di dalam array, dict, atau struct lain.
