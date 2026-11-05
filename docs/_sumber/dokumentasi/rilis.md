---
judul: Catatan rilis 0.2.0
menu: Catatan rilis
ringkas: Rilis kedua JawaRustGo++.
---
0.2.0 adalah rilis kedua JRG. Rincian tiap perubahan sejak 0.1.0 ada di
[CHANGELOG.md](https://github.com/Kodok-Kejepit/jrg/blob/main/CHANGELOG.md); ringkasannya di bawah.

## Yang ada di rilis ini

**Bahasa**
- Struct dengan pewarisan, enum dengan data, trait dengan method bawaan, generik, operator buatan sendiri, dan
  `to_string` untuk tampilan.
- Pola lengkap: tuple, array, struct, enum, rentang, `@`, penjaga `if`, `let … else`, dan `??`.
- Fungsi dengan nilai bawaan, argumen bernama, pengumpul, sebar, closure, dan `move`.
- Goroutine, channel, `select`, Future, Task, Atomic, dan Mutex.
- Modul dengan ruang nama per berkas, dan `std/` yang tertanam.

**Mesin**
- Interpreter, VM bytecode, dan backend LLVM, dijaga dengan 851 tes dan tabel paritas antar-mesin.
- Backend LLVM setara C untuk kerja angka dan pemanggilan fungsi.

**Pustaka**
- {BUILTIN} fungsi bawaan, 14 modul `std/`, dan mesin game 2D 12 modul.
- Server web dengan router, middleware, cookie, sesi, template, dan WebSocket; klien HTTP/HTTPS; SQLite.
- Kanvas dengan ekspor dan impor PNG, suara, dan array numerik `wokwok::`.
- FFI ke C, C++, Rust, Go, Python, JavaScript, PHP, Lua, Java, Kotlin, .NET, dan WebAssembly.

**Perkakas**
- `jrgc`: REPL, editor terminal, server bahasa, pemeriksa, profiler, dan kompilasi native.
- `jrgm`: proyek, tes, format, lint, dependensi lewat git, penerbitan, dan pembungkus APK/AAB Android.
- Pesan galat berbahasa Indonesia dengan enam pilihan nada.
- JRG IDE untuk peramban dan Android.

## Memasang

Lihat [Pasang](pasang.html). Biner tersedia untuk Linux (x86_64, aarch64), macOS (Intel, Apple Silicon), Windows
(x86_64), dan Termux, masing-masing dengan checksum SHA-256. Arsipnya sudah membawa runtime untuk `--emit-llvm`, jadi
yang perlu dipasang sendiri cuma `clang`.

## Yang belum

Lihat [Yang belum jadi](batasan.html).
