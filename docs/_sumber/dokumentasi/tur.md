---
judul: Tur singkat
ringkas: Sepuluh menit pertama dengan JRG, dari satu baris sampai program dengan tes.
---
## Satu baris

Berkas `.jrg` paling pendek cukup berisi perintahnya saja:

```jrg halo.jrg
wok("halo")
```
```keluaran
halo
```

```sh
jrgc halo.jrg
```

Berkas tanpa `package` disebut *skrip*: baris-baris di tingkat atas dijalankan dari atas ke bawah. Fungsi, struct, dan
tipe lain boleh dideklarasikan di mana saja di berkas itu.

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

## Program dengan main

Untuk program yang lebih besar, tulis `package` dan sebuah `pub fn main()`. Di sinilah program mulai berjalan.

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

`wok` menerima berapa pun argumen dan memisahkannya dengan spasi. Saudaranya: `wok_` (tanpa pindah baris), `eprint`
(ke stderr), dan `str::format` untuk merangkai teks.

## Membaca masukan dan argumen

`tok()` membaca satu baris dari keyboard. `env::args()` berisi nama skrip lalu argumen yang ditulis sesudah `--`.

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

## Mesin yang lain

Program yang sama bisa dijalankan dengan tiga cara:

```sh
jrgc suhu.jrg                 # interpreter
jrgc suhu.jrg --vm            # VM bytecode, lebih cepat
jrgc suhu.jrg --emit-llvm     # kompilasi native lewat clang, lalu dijalankan
jrgc suhu.jrg --emit-llvm -o suhu --no-run   # cuma kompilasi, hasilnya ./suhu
```

Bedanya dijelaskan di [Tiga mesin](mesin.html). Untuk sehari-hari, interpreter sudah cukup.

## Mencoba di REPL

`jrgc` tanpa berkas membuka REPL. Ekspresi langsung dicetak nilainya, dan blok yang belum tertutup dilanjutkan di
baris berikutnya.

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

`:cari kata` mencari fungsi bawaan, `:doc nama` menjelaskannya, dan `:simpan` menulis sesi jadi berkas `.jrg` yang
bisa dijalankan. Ketik `:bantuan` untuk daftar lengkapnya.

## Menulis tes

Fungsi bernama `test_…` atau `uji_…` adalah tes. `--uji` menjalankan semuanya, bukan `main`.

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

Untuk proyek dengan banyak berkas tes, pakai `jrgm test` (lihat [jrgm](jrgm.html)).

## Kalau ada yang salah

Ambil contoh ini:

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

Pesan galat menyebut apa yang salah, letaknya, dan potongan kode dengan tanda `^` di bagian yang bermasalah. Di
atasnya biasanya ada satu baris sapaan (di contoh ini dibuang dengan `--sopan`); cara mengatur nadanya ada di
[Diagnostik](diagnostik.html).

## Sesudah ini

- [Nilai dan tipe](nilai.html): angka, teks, array, dict, dan konversinya.
- [Pustaka bawaan](pustaka.html): apa saja yang sudah tersedia.
- [jrgm](jrgm.html): membuat proyek dengan banyak berkas.
