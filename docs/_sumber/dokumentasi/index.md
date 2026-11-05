---
judul: JawaRustGo++
menu: Pengantar
ringkas: Bahasa pemrograman dengan pesan galat berbahasa Indonesia, tiga mesin eksekusi, dan pustaka yang cukup untuk membuat aplikasi sungguhan.
---
JawaRustGo++, disingkat JRG, adalah bahasa pemrograman yang ditulis dengan Rust. Bentuk kodenya mirip Rust dan Go,
diketik dinamis, dan semua pesannya berbahasa Indonesia. Dokumentasi ini untuk versi **0.2.0**.

```jrg halo.jrg
pub fn main() {
    let nama = "Nusantara"
    wok("""sugeng rawuh, ${nama}!""")
}
```
```keluaran
sugeng rawuh, Nusantara!
```

`wok` mencetak ke layar. Tombol **Jalankan** di atas blok kode menjalankan contohnya langsung di peramban, dan
**Ubah** membuatnya bisa disunting.

## Yang ada di dalamnya

- **Tiga mesin.** Interpreter (bawaan), VM bytecode (`--vm`), dan kompilasi native lewat LLVM (`--emit-llvm`). Program
  yang sama memberi keluaran yang sama di ketiganya.
- **Nilai tidak pernah berbagi.** Menyalin, mengoper, dan mengembalikan nilai selalu menghasilkan salinan, jadi tidak
  ada pengumpul sampah dan tidak ada perubahan dari jauh yang mengejutkan.
- **Galat yang bisa dibaca.** Tiap galat menyebut berkas, baris, kolom, potongan kodenya, dan biasanya jalan keluarnya.
- **Pustaka yang lengkap.** {BUILTIN} fungsi bawaan dan 14 modul `std/`: teks, koleksi, berkas, JSON, HTTP, server web,
  SQLite, kanvas, suara, numerik, dan mesin game 2D.
- **Bahasa lain.** Panggil C, C++, Rust, Go, Python, JavaScript, PHP, Lua, Java, Kotlin, .NET, dan WebAssembly.
- **Perkakas.** `jrgm` untuk proyek, dependensi, tes, format, dan lint; editor terminal; REPL; server bahasa (LSP);
  dan pembungkus APK Android.

## Mulai dari mana

| Kalau kamu… | Baca |
|---|---|
| baru mau mencoba | [Pasang](pasang.html), lalu [Tur singkat](tur.html) |
| sudah bisa bahasa lain | [Nilai dan tipe](nilai.html) sampai [Konkurensi](konkuren.html) |
| belum pernah memprogram | kelas [Matematika × JRG](../matematika_x_jrg/index.html) |
| mencari fungsi tertentu | [Pustaka bawaan](pustaka.html), atau tekan <kbd>/</kbd> untuk mencari |
| mau membuat proyek | [jrgm](jrgm.html) |

## Status

JRG masih muda. Interpreter dan VM menjalankan seluruh bahasa; backend LLVM sudah menjalankan semua berkas konformans
tapi belum seluruh pustaka. Daftar lengkap yang belum jadi ada di [Yang belum jadi](batasan.html), dan perubahan
versi ini di [Catatan rilis](rilis.html).
