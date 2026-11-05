---
judul: jrgm
ringkas: Mengurus proyek: membuat, menjalankan, menguji, merapikan, memeriksa, dependensi, dan rilis.
---
## Proyek baru

```sh
$ jrgm new kalku
Created `kalku` (bin) package
$ cd kalku
$ jrgm run
Halo dari JawaRustGo++!
```

Isinya:

```teks
kalku/
├── jrgm.toml        manifes proyek
├── src/main.jrg     titik masuk
└── tests/dasar.jrg  tes
```

```toml jrgm.toml
[package]
name        = "kalku"
version     = "0.1.0"
edition     = "jrg2025"
entry       = "src/main.jrg"

[dependencies]
bantu = { path = "../bantu" }
```

`jrgm new nama --kind lib` membuat pustaka, `--kind web` membuat aplikasi web, dan `--android` menambahkan bagian
`[android]`. Proyek bahasa lain juga bisa: `jrgm new alat --bahasa python` (juga `javascript`, `c`, `cpp`, `rust`,
`go`, `java`, `php`, `lua`, `ruby`, `sh`); `run`, `build`, `test`, dan `fmt` lalu memakai perkakas bahasa itu.

## Perintah

| Perintah | Gunanya |
|---|---|
| `jrgm run [--vm] [--watch]` | jalankan proyek; `--watch` mengulang tiap ada yang berubah |
| `jrgm test [--filter kata] [--vm]` | jalankan tes di `tests/*.jrg` |
| `jrgm build` | periksa semua berkas proyek dan dependensinya |
| `jrgm check` | periksa titik masuk saja |
| `jrgm fmt [--check]` | rapikan spasi dan indentasi tiap berkas |
| `jrgm lint [--ketat]` | galat ditambah peringatan |
| `jrgm exec berkas` | jalankan satu berkas tanpa proyek (`.jrg`, `.c`, `.py`, `.rs`, `.go`, …) |
| `jrgm clean` | hapus hasil bangun |
| `jrgm lsp` | server bahasa untuk editor |
| `jrgm apk`, `jrgm kunci`, `jrgm siapkan android` | aplikasi Android, lihat [Aplikasi Android](android.html) |

## Tes

Fungsi bernama `test_…` atau `uji_…` di `tests/*.jrg` adalah tes. Tiap berkas jalan di prosesnya sendiri dengan batas
waktu, jadi satu tes yang macet tidak menahan yang lain.

```sh
$ jrgm test
[jrgm] 1 test di 1 berkas

== tests/dasar.jrg (1 test)
  ok    test_contoh
--- 1 test: 1 lulus, 0 gagal ---

[jrgm] semua 1 test lulus
```

## Lint

```sh
$ jrgm lint
peringatan: `import "std/set"` tidak dipakai — tidak ada `set::` di berkas ini
 --> src/main.jrg:2:1
  |
2 | import "std/set"
  | ^^^^^^^^^^^^^^^^
  |
  = bantuan: hapus baris import-nya

peringatan: `x` diberi nilai tapi tidak pernah dibaca
 --> src/main.jrg:7:9
  |
7 |     let x = 5
  |         ^
  |
  = bantuan: hapus, atau beri awalan `_` (`_x`) kalau memang sengaja

[jrgm] lint: 2 berkas, 0 galat, 2 peringatan
```

Peringatannya: import yang tidak dipakai, fungsi yang tidak pernah dipanggil, variabel yang tidak pernah dibaca,
literal bulat di tempat pecahan, dan berkas yang belum rapi. `--ketat` membuat peringatan menggagalkan perintahnya.

## Dependensi

Dependensi bisa berupa folder lokal atau repositori git. Git berperan sebagai registri: versinya adalah tag
`v<versi>`, dan commit yang dipakai dikunci di `jrgm.lock`.

```sh
jrgm add bantu --path ../bantu
jrgm add warna --git https://github.com/contoh/warna-jrg --version 0.2.0
jrgm add alat --git https://github.com/contoh/alat --rev main --dev
jrgm update            # majukan semua dependensi git
jrgm update warna
jrgm remove bantu
jrgm tree
```

```sh
$ jrgm tree
kalku 0.1.0
└── bantu 0.1.0
```

Sesudah ditambahkan, modul dependensi bisa langsung di-`import "bantu"`. Siklus dependensi dan dua dependensi yang
memberi nama modul sama ditolak dengan rantainya disebut.

## Menerbitkan

```sh
jrgm publish --dry-run   # periksa dulu
jrgm publish             # buat tag v<versi> dari jrgm.toml lalu dorong ke remote git
```

Proyek lain lalu bisa memakainya dengan `jrgm add nama --git url --version x.y.z`. `jrgm add nama` tanpa `--path`
atau `--git` ditolak, karena belum ada registri pusat untuk mencari nama saja.
