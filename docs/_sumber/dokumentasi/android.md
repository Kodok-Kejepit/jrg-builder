---
judul: Aplikasi Android
ringkas: Membungkus program atau proyek JRG jadi APK, dari game kanvas sampai aplikasi berantarmuka web.
---
## Dari satu berkas

```sh
jrgc game.jrg --apk game.apk                      # untuk HP (arm64-v8a)
jrgc game.jrg --apk game.apk --apk-abi arm64-v8a,x86_64
adb install -r game.apk
```

## Dari proyek

```sh
jrgm siapkan android      # sekali: unduh platform, build-tools, dan NDK
jrgm new gameku --android
cd gameku
jrgm apk                  # target/gameku.apk
jrgm run --android        # pasang ke HP lewat adb, buka, ikuti keluarannya
```

Identitas aplikasinya ditulis di `jrgm.toml`:

```toml
[android]
nama       = "Game Ku"
paket      = "id.jrg.gameku"
versi      = "1.2.0"
orientasi  = "portrait"          # bebas | portrait | landscape
abi        = ["arm64-v8a"]       # tambah "x86_64" untuk emulator
ikon_depan = "aset/ikon.png"     # ikon adaptif Android 8+
ikon_latar = "#0B1230"           # warna atau PNG
izin       = ["internet", "getar"]
```

## Dua jenis tampilan

| `tampilan` | Isinya | Cocok untuk |
|---|---|---|
| `"kanvas"` (bawaan) | program dikompilasi native lewat LLVM, menggambar ke layar penuh | game, gambar, alat sederhana |
| `"web"` | `jrgc` ikut di dalam APK dan menjalankan proyekmu sebagai server lokal; halamannya tampil di WebView | aplikasi dengan antarmuka HTML |

Proyek dari `jrgm new x --kind web` sudah memakai tampilan web. Di sana seluruh bahasa dan pustaka tersedia, karena
yang menjalankannya interpreter yang sama dengan di komputer. Halamannya juga mendapat objek `JRGAndroid` untuk
meminta izin (`mintaIzin("kamera")`), menggetarkan HP, dan bertukar berkas dengan penyimpanan HP.

Tampilan kanvas memakai backend LLVM, jadi berlaku batasan backend itu (lihat [Yang belum jadi](batasan.html)).

## Izin

`izin = [...]` menerima `internet`, `jaringan`, `wifi`, `getar`, `tetap_menyala`, `latar` (jalan di latar dengan
notifikasi), serta izin yang perlu dialog di tampilan web: `kamera`, `mikrofon`, `lokasi`, `kontak`, `penyimpanan`,
`telepon`, `sms`, `kalender`. Nama izin Android lengkap (`android.permission.X`) juga diterima.

## Bahasa dan pustaka native di dalam APK

```toml
[android]
bahasa = ["lua", "js", "sql"]     # penjalan Lua, QuickJS, dan SQLite ikut di APK

[android.pustaka_c]
libhitung = ["native/hitung.c", "native/teks.cpp"]

[android.pustaka_rust]
librumus = "rust/rumus"
```

`jrgm` mengunduh sumber resminya sekali (sidiknya diperiksa), lalu mengompilasinya untuk tiap ABI. Pustaka native
dipanggil dari JRG lewat `extern "C" "libhitung.so" { … }`.

## Rilis ke Play Store

```sh
jrgm kunci                         # sekali: buat kunci rilis
JRG_KUNCI_SANDI=… jrgm apk --rilis
JRG_KUNCI_SANDI=… jrgm apk --aab   # Android App Bundle untuk Play Store
```

APK biasa ditandatangani dengan kunci debug. Simpan berkas kunci rilis baik-baik: tanpa kunci yang sama, pembaruan
aplikasi tidak bisa dipasang di atas versi lama.
