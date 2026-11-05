---
judul: Yang belum jadi
ringkas: Kekurangan JRG 0.2.0 yang sudah diketahui.
---
## Bahasa

- **Diskriminan enum** (`enum E { A = 5 }`) belum ada. Pemetaan varian ke angka ditulis dengan `match`.
- **Tidak ada `async`/`await`.** Konkurensi memakai goroutine dan channel; tiap goroutine adalah thread sistem, jadi
  cocok untuk puluhan sampai ratusan tugas, bukan jutaan.
- **Anotasi tipe tidak ditegakkan.** Nama tipenya diperiksa, isinya tidak.
- **`for` dengan satu nama memberi indeks**, juga untuk rentang. Ini disengaja, tapi sering mengejutkan; lihat
  [Kendali alur](kendali.html#for).
- **Closure yang memanggil dirinya sendiri** tidak pernah dibebaskan dari memori. Pakai `fn` bernama.

## Backend LLVM {#backend-llvm}

Interpreter dan VM menjalankan seluruh bahasa. Backend LLVM menjalankan semua berkas konformans, dan menolak (dengan
alasannya) beberapa bentuk:

- sebar di pemanggilan (`f(...v)`) dan argumen bernama (`f(a: 1)`);
- konstruktor varian sebagai nilai (`let f = Some`);
- mewarisi struct generik;
- sebagian modul `std/` yang ditulis sangat dinamis;
- bilangan bulat di atas 64 bit.

Program kanvas yang dibungkus jadi APK memakai backend ini, jadi batasan yang sama berlaku di sana.

## Pustaka

- **Waktu selalu UTC.** `time::format` belum mengenal zona waktu.
- **Jendela kanvas** butuh build dengan `--features canvas`; bawaannya menggambar ke berkas atau ke peramban.
- **Belum ada registri paket pusat.** Dependensi diambil dari folder lokal atau repositori git.
- **FFI Java/Kotlin belum tersedia di dalam APK.**

## Perkakas dan platform

- **Termux**: biner `android-aarch64` ikut rilis sejak 0.2.0 dan dipasang lewat pemasang yang sama, tapi belum
  pernah dicoba di HP sungguhan.
- **Windows**: biner dibangun dan diuji di CI, tapi editor terminal dan penyuntingan baris REPL di Windows belum
  pernah dicoba di mesin sungguhan.
- **JRG IDE di HP**: mesin Native cuma ada di versi Terminal (lewat clang di Ubuntu), dan belum mendukung
  `dok::`, `crypto::`, `re::`, dan `ptr::` di mesin itu.

Kalau kamu menemukan perilaku yang salah tanpa pesan galat, itu dianggap bug serius. Laporkan di
[GitHub](https://github.com/Kodok-Kejepit/jrg/issues) beserta programnya.
