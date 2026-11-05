---
judul: Diagnostik
ringkas: Bentuk pesan galat JRG, nadanya, dan cara memakainya dari editor atau CI.
---
## Bentuk pesan

```teks
berhenti di baris 3. gw tunjukin apa yang salah, di bawah.
error: variabel `jumlas` belum dikenal — maksudnya `jumlah`? kalau bukan, cek lagi penulisannya atau jangan-jangan lupa `let jumlas = ...` dulu
 --> hitung.jrg:3:9
  |
3 |     wok(jumlas)
  |         ^^^^^^
  |
```

1. **Sapaan**, satu baris di atas. Kalimatnya berganti-ganti; `--sopan` membuangnya.
2. **Apa yang salah**, dengan nama yang kamu tulis dan, kalau ada, saran ejaannya.
3. **Letaknya**: berkas, baris, dan kolom.
4. **Potongan kode** dengan `^` di bagian yang bermasalah.
5. **`bantuan:`** berisi jalan keluar, dan **`catatan:`** konteks tambahan, kalau ada.
6. **`jejak:`** untuk galat saat jalan: dari mana fungsi yang gagal itu dipanggil.

Pesan menyebut **tipe** nilai, bukan isinya. Salah memakai array berisi tiga angka atau tiga ribu angka menghasilkan
pesan yang sama pendeknya.

## Nada

Galat pertama kali selalu disampaikan dengan nada serius. Kalau galat yang sama muncul lagi, sapaannya berganti
sesuai nada yang dipilih, dan pada pengulangan ketujuh ditambah satu baris bantuan, karena pesan yang sama berulang
tujuh kali berarti penjelasannya belum sampai.

```sh
jrgc f.jrg --nada hangat     # sopan | galak (bawaan) | judes | hangat | cool | lawak
JRG_NADA=cool jrgc f.jrg
jrgc f.jrg --sopan           # tanpa sapaan sama sekali
```

Nada cuma mengubah baris sapaan. Badan pesannya selalu sama, jadi tidak ada informasi yang hilang di nada mana pun.
Variasi kalimatnya ditentukan dari jenis galat dan jumlah pengulangannya, bukan diacak, jadi keluaran untuk masukan
yang sama selalu sama.

## Untuk editor dan CI

- Di luar terminal (dialirkan ke berkas atau program lain) tidak ada warna dan garis pemisah, dan penghitung
  pengulangan tidak disimpan di antara jalannya program.
- `--sopan` atau `JRG_SOPAN=1` membuang sapaan, cocok untuk membandingkan keluaran.
- `JRG_GALAT_KE=galat.txt` menulis tiap galat juga ke berkas, satu baris per galat: `berkas<TAB>baris<TAB>kolom<TAB>pesan`.
- `jrgc --lsp` mengirim galat dan peringatan ke editor sambil kamu mengetik.

## Tidak ada hasil yang salah diam-diam

Aturan dasarnya: sesuatu yang tidak bisa dihitung dengan benar menghasilkan galat, bukan `0`, `nil`, atau nilai
kosong yang tampak masuk akal. Beberapa contohnya:

| Kasus | Di JRG |
|---|---|
| indeks di luar batas | galat, tidak pernah dijepit |
| `int` meluap | galat, tidak membungkus |
| `300 as u8`, `int("abc")` | galat |
| mencari yang tidak ada | `Option::None`, tidak pernah `-1` |
| `x = 5` atas nama yang belum ada | galat, tidak membuat variabel baru |
| fungsi bawaan dengan argumen kurang atau lebih | galat, dengan tanda tangannya |
| `Err` yang keluar dari `main` | dilaporkan, kode keluar 1 |
