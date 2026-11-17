---
judul: Option, Result, dan galat
menu: Option dan Result
ringkas: Cara JRG menyatakan "mungkin tidak ada" dan "mungkin gagal", dan cara menghentikan program dengan sengaja.
---
## Option

`Option` adalah jawaban yang mungkin kosong: `Some(nilai)` atau `None`. Fungsi bawaan yang bisa tidak menemukan
sesuatu (`first`, `get`, `find`, `pop`, `dict::get`, `env::get`, `str::find`) mengembalikan `Option`, tidak pernah
`nil` atau `-1`.

```jrg option.jrg
let ada = [4, 8, 15].find(|x| x > 5)
let kosong = [4, 8, 15].find(|x| x > 50)
wok(ada, kosong)
wok(ada.is_some(), kosong.is_none())
wok(ada.unwrap(), kosong.unwrap_or(0), kosong ?? -1)
wok(ada.map(|x| x * 2), kosong.map(|x| x * 2))
```
```keluaran
Option::Some(8) Option::None
true true
8 0 -1
Option::Some(16) Option::None
```

`unwrap()` atas `None` adalah galat. Pakai `unwrap_or`, `??`, atau `match` kalau kekosongannya wajar.

## Result

`Result` adalah jawaban yang mungkin gagal: `Ok(nilai)` atau `Err(alasan)`. Fungsi yang menyentuh dunia luar
(`fs::try_read`, `http::get`, FFI, basis data) mengembalikan `Result`, dan fungsimu sendiri juga boleh.

```jrg result.jrg
fn bagi(a, b) -> Result<int, string> {
    if b == 0 {
        return Err("pembagi nol")
    }
    return Ok(a / b)
}

wok(bagi(10, 2), bagi(1, 0))

match bagi(9, 0) {
    Ok(v) => wok("hasil", v),
    Err(e) => wok("gagal:", e),
}

wok(bagi(8, 2).unwrap_or(-1), bagi(8, 0).unwrap_or(-1))
```
```keluaran
Result::Ok(5) Result::Err(pembagi nol)
gagal: pembagi nol
4 -1
```

## Meneruskan kegagalan dengan ?

`ekspresi?` membuka `Ok`, atau langsung mengembalikan `Err`-nya dari fungsi yang sedang berjalan. Berlaku juga untuk
`Option`: `None` diteruskan.

```jrg tanya.jrg
fn bagi(a, b) -> Result<int, string> {
    if b == 0 { return Err("pembagi nol") }
    return Ok(a / b)
}

fn hitung(a, b, c) -> Result<int, string> {
    let x = bagi(a, b)?
    let y = bagi(x, c)?
    return Ok(y)
}

wok(hitung(100, 5, 2))
wok(hitung(100, 0, 2))
```
```keluaran
Result::Ok(10)
Result::Err(pembagi nol)
```

## Kalau main gagal

`main` boleh mengembalikan `Result`. `Err` yang keluar dari `main` dilaporkan sebagai galat dan program berhenti dengan
kode 1.

```jrg main-gagal.jrg
pub fn main() -> Result<int, string> {
    return Err("berkas konfigurasi tidak ada")
}
```
```keluaran
error: program berhenti karena `Result::Err` merambat keluar dari `main`: berkas konfigurasi tidak ada — tangani dengan `match` di dalam `main`, atau pakai `.unwrap_or(...)`, kalau kegagalan ini memang boleh terjadi
```

## Menghentikan program

`panik(pesan)` menghentikan program dengan galat. Pakai untuk keadaan yang seharusnya mustahil; untuk kegagalan yang
wajar, kembalikan `Result`.

```jrg panik.jrg
fn umur_valid(n) {
    if n < 0 {
        panik("umur tidak boleh negatif")
    }
    return n
}

wok(umur_valid(5))
wok(umur_valid(-1))
```
```keluaran
5
error: umur tidak boleh negatif
 --> panik.jrg:3:9
  |
3 |         panik("umur tidak boleh negatif")
  |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = jejak: `umur_valid` dipanggil di panik.jrg:9:5
```

Baris `jejak:` menunjukkan dari mana fungsi itu dipanggil. `exit(kode)` mengakhiri program dengan kode keluar 0–255
tanpa pesan.

## Menangkap galat

`tangkap(f)` menjalankan fungsi tanpa argumen dan menjawab `Result`: `Ok` berisi nilainya, atau `Err` berisi `Galat`
kalau ada galat saat ia jalan. Field-nya `pesan` (kalimatnya, dalam bahasa pesan), `berkas`, `baris` dan `kolom`
(tempatnya). `defer` di dalam `f` tetap jalan lebih dulu.

```jrg tangkap.jrg
fn bagi(a, b) {
    return a / b
}

match tangkap(|| bagi(10, 0)) {
    Ok(v) => wok("hasil:", v),
    Err(g) => wok("gagal di baris", g.baris, "-", g.pesan),
}
wok("masih jalan")
```
```keluaran
gagal di baris 2 - `10 / 0` — nggak bisa bagi dengan nol, hasilnya nggak pernah terdefinisi. Cek lagi angka pembaginya sebelum dipakai bagi
masih jalan
```

Semua galat ditangkap, termasuk stack yang habis, kecuali `exit()`, yang memang sengaja menghentikan program, dan
memori yang habis. Tidak ada yang dikembalikan: yang sudah diubah `f` sebelum galat tetap berubah. `panik(g)`
melempar `Galat` yang tertangkap lagi, dengan catatan tempat asalnya.

## Memeriksa asumsi

`assert(syarat)` dan `assert_eq(a, b)` menghentikan program kalau asumsinya salah. Di dalam tes (lihat
[Tur singkat](tur.html#menulis-tes)), kegagalan assert menandai tes itu gagal.

```jrg assert.jrg
assert(1 + 1 == 2)
assert_eq([1, 2], [1, 2])
assert_eq(2 + 2, "4")
```
```keluaran
error: assert_eq gagal — kiri `4` (int), kanan `4` (string)
 --> assert.jrg:3:1
  |
3 | assert_eq(2 + 2, "4")
  | ^^^^^^^^^^^^^^^^^^^^^
  |
```

Pesannya menyebut tipe kedua sisi, karena `4` dan `"4"` tercetak sama.
