---
judul: jrgc
ringkas: Menjalankan, memeriksa, menguji, dan mengompilasi berkas .jrg.
---
## Menjalankan

```sh
jrgc program.jrg              # interpreter
jrgc program.jrg --vm         # VM bytecode
jrgc program.jrg --emit-llvm  # native, lewat clang
jrgc program.jrg -- a b c     # argumen untuk programnya, dibaca lewat env::args()
jrgc program.jrg --watch      # jalankan ulang tiap berkasnya berubah
jrgc                          # REPL
```

## Memeriksa tanpa menjalankan

```sh
jrgc program.jrg --periksa    # urai dan periksa berkas beserta semua import-nya; diam kalau bersih
jrgc program.jrg --fmt        # cetak versi rapinya ke stdout (berkasnya tidak diubah)
jrgc program.jrg --emit-ast   # cetak pohon sintaks
jrgc program.jrg --emit-tokens
```

`--periksa` menemukan galat sintaks, nama yang belum dideklarasikan, dan penugasan ke sesuatu yang bukan variabel,
sebelum satu baris pun jalan.

## Tes dan profil

```sh
jrgc hitung.jrg --uji                   # jalankan tiap fn test_*/uji_*
jrgc hitung.jrg --uji --uji-filter tambah
jrgc program.jrg --profile              # waktu tiap fungsi sesudah program selesai
jrgc program.jrg --profile-json p.json  # versi JSON-nya
```

## Mencari fungsi

```sh
jrgc --cari kata        # nama bawaan yang memuat "kata"
jrgc --doc str::split   # tanda tangan dan penjelasan singkat
jrgc --daftar-builtin   # semua nama, satu per baris
```

## Kompilasi native

`--emit-llvm` menerjemahkan program ke LLVM IR lalu menyerahkannya ke `clang`. Runtime native-nya dibangun sekali
dengan cargo dan disimpan.

```sh
jrgc program.jrg --emit-llvm -o program --no-run   # cuma kompilasi
./program

jrgc program.jrg --emit-llvm -O3 --native-cpu --lto thin
jrgc program.jrg --emit-llvm --full-power          # preset tercepat, khusus CPU ini
jrgc program.jrg --emit-llvm --target aarch64-unknown-linux-gnu
jrgc program.jrg --apk app.apk                     # APK Android dari satu berkas
```

| Bendera | Gunanya |
|---|---|
| `-O 0..3` | tingkat optimisasi (bawaan 2) |
| `--lto auto\|off\|thin\|full`, `--lld` | optimisasi saat menautkan |
| `--native-cpu`, `--fast-math`, `--aggressive` | lebih cepat, dengan syaratnya masing-masing |
| `--pgo-generate`, `--pgo-use` | optimisasi berdasarkan profil pemakaian |
| `--strip`, `--no-gc-sections` | ukuran biner |

Backend native belum menerima seluruh bahasa; yang belum ada ditolak dengan alasannya. Lihat
[Tiga mesin](mesin.html).

## Pesan galat

```sh
jrgc program.jrg --nada hangat    # sopan, galak, judes, hangat, cool, lawak
jrgc program.jrg --sopan          # tanpa baris sapaan, cocok untuk CI
JRG_GALAT_KE=galat.txt jrgc program.jrg   # tulis tiap galat juga ke berkas: berkas, baris, kolom, pesan
```

Penjelasannya di [Diagnostik](diagnostik.html).

## Lain-lain

| Perintah | Gunanya |
|---|---|
| `jrgc --sunting berkas.jrg` | penyunting teks di terminal, lihat [Editor](editor.html) |
| `jrgc --lsp` | server bahasa untuk editor (VS Code, Neovim) |
| `jrgc -I folder` | folder tambahan untuk mencari `import` |
| `jrgc --jrgm …` | `jrgc` yang sama berperan sebagai `jrgm` |
| `-v` | cetak tiap tahap yang dikerjakan |
