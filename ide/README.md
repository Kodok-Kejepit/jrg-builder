# JRG IDE

IDE untuk JRG dan bahasa lain di Android dan laptop. Pelayannya ditulis dalam JRG, tampilannya halaman web lokal, dan
`jrgm apk` membungkus keduanya jadi aplikasi Android.

```sh
cd ide
jrgm run              # laptop: buka http://127.0.0.1:8080
sh bangun_apk.sh      # APK, lihat "Dua versi APK"
jrgm run --android    # pasang ke HP atau emulator lewat adb
```

## Susunan

| Bagian | Isi |
|---|---|
| `src/main.jrg` | pelayan: `dok::serve` di `127.0.0.1`, berkas pemakai, penjalan kode, daftar bahasa |
| `src/terminal.jrg` | terminal PTY lewat `extern "C"` ke libc |
| `src/linux.jrg` | Ubuntu di dalam proot (versi Terminal) |
| `web/` | halaman: penyunting, panel, terminal, lembar pengaturan |
| `linux/` | pemasang Ubuntu (C), biner proot beserta asal dan lisensinya |
| `varian/` | tambalan manifes dan layanan latar untuk versi Terminal |
| `contoh/` | berkas contoh yang disalin saat pertama dibuka |
| `uji/` | uji di chromium headless dan uji Linux |

Halaman tidak menjalankan kode sendiri. Tombol Jalankan meminta pelayan memulai program sebagai proses terpisah
(`proses::mulai`), lalu halaman membaca keluarannya sedikit demi sedikit dan bisa mengirim masukan atau
menghentikannya. Di Android prosesnya `jrgc` yang ikut di dalam APK, jadi seluruh bahasa JRG tersedia.

Berkas pemakai disimpan sebagai berkas biasa di `$HOME/jrg-ide/` (di Android: folder data aplikasi), jadi
memperbarui aplikasi tidak menghapusnya. Nama berkas dari halaman diperiksa; `..`, jalur mutlak, dan garis miring
terbalik ditolak.

## Penyunting

- Sorotan sintaks, tab, pohon berkas, daftar simbol, palet perintah (Ctrl+P, `>` perintah, `:` baris, `@` simbol,
  `#` simbol di semua berkas).
- Saran ketik dari `jrgc --daftar-builtin` di perangkat itu, kata kunci, nama di berkas, dan cuplikan.
- Panel Masalah dari `jrgc --periksa` dan `jrgm lint`; galat ditandai di talang dan bisa dilompati (F8).
- Ke definisi (F12), rujukan (Shift+F12), ganti nama (F2), dokumentasi (F1), rapikan (Shift+Alt+F; memakai
  perapi resmi bahasanya kalau ada).
- Riwayat berkas: salinan lama disimpan di `$HOME/jrg-ide-riwayat/` sebelum ditimpa atau dihapus, dan bisa
  dibandingkan atau dipulihkan.
- Pita simbol untuk HP, urung/ulang sendiri, ukuran huruf, latar foto atau video, tema, dan ikon yang bisa diganti.

Penyunting memakai `<pre>` berwarna di bawah `<textarea>` transparan dalam satu wadah gulir. Biaya mengetik di
textarea Chromium tumbuh kuadrat terhadap letak kursor, jadi textarea hanya memegang paling banyak 300 baris di
sekitar kursor dan sisanya disimpan di luar. Sorotan berkas besar dikerjakan Web Worker.

## Terminal

Panel Konsol adalah terminal sungguhan: tiap sesi satu shell (mksh di Android, bash di laptop) di atas PTY, jadi
`vi`, `top`, warna, Ctrl+C, dan `term::raw` bekerja seperti di Termux.

- Pelayan membuka PTY lewat libc dan menyangga keluaran tiap sesi, jadi sesi tetap hidup saat halaman ditutup.
- Halaman dan pelayan bertukar data lewat long-poll HTTP.
- Layarnya emulator VT/xterm sendiri (`web/js/vt.js`): layar alternatif, 256 warna dan truecolor, huruf lebar,
  tetikus, bracketed paste, OSC 7 dan 52.
- Baris tombol seperti Termux (ESC, CTRL, ALT, panah, Fn), dan tata letaknya bisa diganti.
- Perintah IDE di shell: `buka <berkas>`, `pasang`/`copot <bahasa>`, `penyimpanan`, `setup-storage`.

## Bahasa

`daftar_bahasa()` di `src/main.jrg` adalah satu-satunya daftar. Tiap bahasa menyebut penjalannya, dan kalau
penjalannya tidak ada, alasannya ditampilkan apa adanya.

- **Ikut di APK:** JRG, Lua, JavaScript (QuickJS), SQL (SQLite), lewat `bahasa = [...]` di `jrgm.toml`.
- **Terkompilasi:** C, C++, Assembly, Rust, Zig, Fortran, Java, aktif kalau kompilatornya ada di PATH (di HP: lewat
  Ubuntu pada versi Terminal).
- **Dipasang dari dalam IDE, jalan di halaman:** Python (Pyodide), TypeScript, CoffeeScript, Ruby (Opal), Scheme,
  Prolog. Berkasnya diunduh sekali, sidiknya diperiksa, lalu disimpan untuk dipakai tanpa internet. Pustaka Python
  dari daftar resmi Pyodide ikut dipasang saat diimpor.

## Proyek

Folder menjadi proyek kalau punya berkas utama: `entry` di `jrgm.toml`, atau penanda `.utama` dari IDE. Tombol
Jalankan dari berkas mana pun di dalamnya menjalankan berkas utama dengan akar proyek sebagai folder kerja. Proyek
baru bisa dibuat dari rangka `jrgm new` untuk JRG dan sepuluh bahasa lain.

## Dua versi APK

`sh ide/bangun_apk.sh [play|terminal|semua]`

| | Play Store | Terminal |
|---|---|---|
| paket | `id.jrg.ide` | `id.jrg.ide.terminal` (bisa dipasang berdampingan) |
| targetSdk | 34 | 28, seperti Termux |
| port | 8080 | 8081 |
| Linux (Ubuntu di proot) | tidak ada | ada |
| mesin Native | tidak ada | lewat clang di Ubuntu |
| layanan latar dan notifikasi | tidak ada | ada |

Keduanya ditandatangani `~/.android/debug.keystore` mesin yang membangunnya. `sh ide/tanda_tangan.sh` memasang kunci
yang sama di mesin lain, atau menandatangani ulang APK yang sudah jadi.

## Linux (versi Terminal)

Ubuntu 24.04 berjalan tanpa root lewat proot (termux/proot dengan tiga tambalan; asal dan lisensinya di
`linux/proot/SUMBER.md`).

- **Pasang** (Lainnya > Linux): arsip `ubuntu-base` diunduh, sidiknya dicocokkan dengan yang dipatok di
  `src/linux.jrg`, lalu dibongkar oleh `linux/jrglinux.c`. Arsip yang ditaruh sendiri di folder berkas dipakai tanpa
  internet.
- **Di dalam:** `pkg install ...` (pembungkus apt), `sudo`, `~/berkas` = folder berkas IDE, `buka <berkas>`, dan
  `jrgc`. Folder berkas diikat di jalur yang sama di dua sisi.
- **Bahasa:** penjalan yang tidak ada di Android dicari di Ubuntu, termasuk kompilator.
- **Mesin Native:** sesudah `pkg install clang` di Ubuntu, Pengaturan menawarkan Native. `jrgc` milik APK jalan di
  Ubuntu dan clang menautkan biner glibc dengan runtime yang ikut di APK.
- **Sumber JRG:** APK membawa sumber repo; `jrg-sumber` membongkarnya ke `~/berkas/jrg-lang`, lalu
  `cargo install --path .` membangun `jrgc` dan `jrgm` di sana.
- **Bertukar berkas dengan Termux:** lewat pelayan IDE di `127.0.0.1:8081` (`linux/termux.sh`), atau folder
  `JRG-bersama` di memori HP.

## Pelayan di HP

Android menghentikan proses yang dimulai aktivitas setiap kali aktivitas itu dibuat ulang. Karena itu aktivitas hanya
menjalankan penjaga (`jaga()` di `src/main.jrg`), dan pelayan sungguhan berjalan terlepas, jadi sesi terminal dan
program yang sedang jalan tetap hidup. Permintaan dari situs lain (Origin bukan `127.0.0.1:<port>`) ditolak.

## Menguji

```sh
sh perkakas/ide/uji_ide.sh   # dari akar repo: skenario dasar di chromium headless
sh ide/uji/uji.sh            # penyunting, terminal, bahasa, proyek, Linux, penjaga, Native
```

## Belum ada

- Mesin Native tanpa Ubuntu, dan `dok::`/`crypto::`/`re::`/`ptr::` di Native.
- Masukan interaktif untuk bahasa yang berjalan di halaman (Python, Ruby): masukannya diisi sebelum jalan.
- Lipat kode, banyak kursor, penyunting terbelah.
- Debugger dengan titik henti.

## Asal berkas

Penyunting, penyorot, dan huruf di `web/aset/huruf/` (lisensi OFL, berkas lisensinya ikut) berasal dari aplikasi
Mawar karya penulis yang sama. proot dan talloc mengikuti lisensinya masing-masing (`linux/proot/SUMBER.md`).
