---
judul: Tiga mesin
ringkas: Interpreter, VM bytecode, dan backend LLVM, serta cara ketiganya dijaga tetap sama.
---
Ketiga mesin memakai bagian depan yang sama (pemindai dan pengurai), jadi sintaks dan pesan galat sintaksnya
identik. Bedanya ada di cara program dijalankan.

| | Interpreter | VM | LLVM native |
|---|---|---|---|
| perintah | `jrgc f.jrg` | `jrgc f.jrg --vm` | `jrgc f.jrg --emit-llvm` |
| cara kerja | menjalankan pohon sintaks langsung | menerjemahkan ke bytecode lalu menjalankannya | menerjemahkan ke LLVM IR, dikompilasi `clang` jadi kode mesin |
| bahasa | seluruhnya | seluruhnya | semua berkas konformans, belum seluruh pustaka |
| goroutine | thread sistem | thread sistem | korutin di satu thread |
| cocok untuk | coba-coba, skrip | program yang perlu lebih cepat | program jadi, biner mandiri, APK kanvas |

## Satu program, satu arti

Program yang jalan di dua mesin harus mencetak keluaran yang sama persis, termasuk pesan galatnya, letak kolomnya,
dan saat galat itu terjadi. Kalau dua mesin menjawab berbeda, salah satunya rusak dan diperbaiki, tidak dibiarkan
sebagai "perbedaan implementasi".

Ini dijaga dengan tabel program yang dijalankan di beberapa mesin sekaligus lalu keluarannya dibandingkan:

| Tabel | Isi | Jumlah |
|---|---|---|
| interpreter = VM | program yang keluarannya harus identik | 617 |
| ditolak keduanya | program yang harus ditolak dua mesin dengan pesan yang benar | 365 |
| interpreter = LLVM | program yang keluarannya harus identik | 311 |
| belum di LLVM | program yang ditolak LLVM dengan alasan yang jelas | 28 |
| konformans | berkas uji semantik lengkap | 11 dari 11 di ketiga mesin |

Angka ini dari versi 0.2.0; seluruh rangkaian ujinya berisi 851 tes. Tidak ada satu pun program yang diketahui
jalan di dua mesin dan menjawab berbeda.

## Interpreter

Mesin bawaan. Paling cepat mulai, dan pesan galatnya membawa jejak pemanggilan. Goroutine di sini adalah thread
sistem operasi sungguhan.

## VM

Program diterjemahkan dulu jadi instruksi bytecode yang ringkas, lalu dijalankan. Untuk perulangan dan pemanggilan
fungsi yang berat, VM jauh lebih cepat daripada interpreter. Galatnya menunjuk baris dan kolom yang sama persis.

## LLVM native

`--emit-llvm` menghasilkan program native yang berdiri sendiri. Untuk kerja angka dan pemanggilan fungsi,
kecepatannya setara C (lihat [Benchmark](benchmark.html)).

Yang perlu diketahui:

- Bilangan bulat di sini 64 bit, bukan 128. Luapan tetap diperiksa, dan angka yang tidak muat ditolak dengan
  menyebut `i128`.
- Konkurensi berjalan sebagai korutin di satu thread: `spawn`, channel, dan `select` bekerja, tapi tidak paralel.
  Kebuntuan dilaporkan, tidak menggantung.
- Fitur yang belum didukung ditolak saat kompilasi dengan alasannya, tidak pernah menghasilkan program yang salah.
  Daftarnya di [Yang belum jadi](batasan.html#backend-llvm).
