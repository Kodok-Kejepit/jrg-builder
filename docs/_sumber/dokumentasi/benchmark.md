---
judul: Benchmark
ringkas: Seberapa cepat backend native JRG dibanding C dan Go, dan cara mengukurnya sendiri.
---
Angka di bawah dari backend LLVM (`--emit-llvm`), diukur di Intel i5-1235U dengan clang 21, rustc 1.97, dan Go 1.26.
Tiap angka adalah waktu terbaik dari enam putaran, dijalankan di satu inti CPU.

| Kernel | JRG (ms) | C (ms) | JRG : C |
|---|---|---|---|
| `numeric_kernel` | 732 | 748 | 0,98 |
| `call_recursion` | 595 | 589 | 1,01 |
| `struct_particles` | 1044 | 981 | 1,06 |
| `array_sort` | 1060 | 880 | 1,20 |
| `match_dispatch` | 788 | 585 | 1,35 |
| `mixed_report` | 523 | 243 | 2,15 |
| `string_ops` | 516 | 147 | 3,51 |

Ringkasnya: setara C untuk angka dan pemanggilan fungsi, sedikit di belakang untuk struct, array, dan `match`, dan
jelas lebih lambat untuk pengolahan teks. Dibanding Go, JRG menang di tiga kernel (`struct_particles` 5,8 kali,
`call_recursion` 1,5 kali, `match_dispatch` 1,45 kali), seri di dua, dan kalah di dua.

Selisih di `string_ops` datang dari teks sementara yang belum dibebaskan selama program berjalan; itu pekerjaan
berikutnya untuk backend ini.

## Mengukur sendiri

```sh
ROUNDS=5 CPU=0 benchmarks/llvm/run.sh
```

Untuk interpreter dan VM ada sembilan kernel tersendiri:

```sh
sh benchmarks/acuan/jalankan.sh "$PWD/target/release/jrgc" 3
```

Perbedaan di bawah sekitar 6–10 persen tidak bisa dibedakan dari derau pengukuran di mesin biasa. Untuk membuktikan
sebuah perubahan lebih cepat, hitung instruksinya dengan `valgrind --tool=callgrind`, jangan cuma membandingkan jam.
