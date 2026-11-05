---
judul: Editor, REPL, dan IDE
menu: Editor dan IDE
ringkas: Penyunting di terminal, server bahasa untuk editor favoritmu, dan JRG IDE yang jalan di peramban dan HP.
---
## jrgedit

`jrgc --sunting` membuka penyunting layar penuh bergaya nano. Penyunting ini ditulis dalam JRG dan ikut di dalam
`jrgc`, jadi selalu tersedia.

```sh
jrgc --sunting program.jrg
jrgc --sunting a.jrg b.jrg:120     # beberapa berkas; buka b.jrg di baris 120
jrgc --sunting proyek/             # jelajahi isi folder
```

| Tombol | Gunanya |
|---|---|
| <kbd>Ctrl</kbd>+<kbd>S</kbd> / <kbd>Ctrl</kbd>+<kbd>Q</kbd> | simpan / keluar |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd> / <kbd>Ctrl</kbd>+<kbd>Y</kbd> | urung / ulang |
| <kbd>Ctrl</kbd>+<kbd>F</kbd> / <kbd>Ctrl</kbd>+<kbd>P</kbd> | cari / ganti |
| <kbd>Ctrl</kbd>+<kbd>R</kbd> | simpan lalu jalankan; sesudahnya kursor pindah ke letak galat |
| <kbd>Alt</kbd>+<kbd>P</kbd> | periksa tanpa menjalankan |
| <kbd>Ctrl</kbd>+<kbd>O</kbd> | buka berkas lain; <kbd>Alt</kbd>+panah pindah berkas |
| <kbd>Shift</kbd>+panah, <kbd>Ctrl</kbd>+<kbd>C</kbd>/<kbd>X</kbd>/<kbd>V</kbd> | pilih, salin, potong, tempel |
| <kbd>F1</kbd> atau <kbd>Esc</kbd> | daftar tombol lengkap |

Ada penyorotan sintaks, kurung yang menutup sendiri, indentasi yang mengikuti baris sebelumnya, dan tetikus.

## REPL

`jrgc` tanpa argumen membuka REPL. Panah atas/bawah menelusuri riwayat, blok yang belum tertutup dilanjutkan
otomatis, dan galat tidak mengakhiri sesi.

| Perintah | Gunanya |
|---|---|
| `:cari kata`, `:doc nama`, `:daftar ns` | menemukan fungsi bawaan |
| `:tipe ekspresi` | tipe tanpa mencetak nilainya |
| `:simpan berkas.jrg` | tulis sesi jadi program yang bisa dijalankan |
| `:riwayat`, `:hapus`, `:keluar` | riwayat, mulai ulang, keluar |

## Server bahasa (LSP)

`jrgc --lsp` (atau `jrgm lsp`) adalah server Language Server Protocol. Editor yang mendukung LSP mendapat: galat dan
peringatan sambil mengetik, pelengkapan nama (termasuk seluruh fungsi bawaan), keterangan saat kursor di atas nama,
lompat ke definisi, dan daftar simbol berkas.

Contoh untuk Neovim:

```teks
vim.lsp.start({ name = "jrg", cmd = { "jrgc", "--lsp" }, root_dir = vim.fn.getcwd() })
```

Di VS Code, pakai ekstensi LSP generik dengan perintah `jrgc --lsp` untuk berkas `*.jrg`.

## JRG IDE

JRG IDE adalah lingkungan pengembangan di peramban, dan dibuat dengan JRG sendiri. Isinya penyunting dengan
penyorotan dan pelengkapan, panel Masalah (galat dan peringatan lint), terminal, pohon berkas, proyek `jrgm`, dan
tombol Jalankan untuk JRG serta bahasa lain.

```sh
cd ide && jrgm run       # buka http://127.0.0.1:8080
```

Di HP ada dua versi APK:

| | JRG IDE | JRG IDE Terminal |
|---|---|---|
| untuk | Play Store | dipasang langsung |
| bahasa yang jalan | JRG, Lua, JavaScript, SQL, ditambah Python dan Ruby di halaman | semuanya, plus apa pun yang dipasang di Ubuntu |
| Linux | tidak ada | Ubuntu di dalam aplikasi (`pkg install gcc`, `rustc`, `python3`, …) |
| mesin Native (LLVM) | tidak ada | sesudah `pkg install clang` di Ubuntu |
| jalan di latar | tidak | ya, dengan notifikasi |
