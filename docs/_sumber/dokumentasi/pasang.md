---
judul: Pasang
ringkas: Pakai biner siap jadi, atau bangun sendiri dari sumber. Keduanya menghasilkan dua program, jrgc dan jrgm.
---
Sesudah terpasang kamu punya dua perintah:

- `jrgc` menjalankan, memeriksa, dan mengompilasi berkas `.jrg`. Tanpa argumen ia membuka REPL.
- `jrgm` mengurus proyek: membuat, menjalankan, menguji, mengatur dependensi, sampai membungkus APK.

## Biner siap pakai

Rilis untuk Linux (x86_64, aarch64), macOS (Intel, Apple Silicon), Windows, dan Termux ada di halaman
[Releases](https://github.com/Kodok-Kejepit/jrg/releases). Di Linux, macOS, dan Termux, pemasangnya mengunduh berkas
yang cocok, memeriksa checksum-nya, lalu menaruh `jrgc` dan `jrgm` di `~/.local/bin` (di Termux: `$PREFIX/bin`):

```sh
curl -fsSL https://raw.githubusercontent.com/Kodok-Kejepit/jrg/main/rilis/pasang.sh | sh

# versi tertentu, atau folder lain
sh pasang.sh --versi 0.2.0 --ke /usr/local/bin
```

Pastikan `~/.local/bin` ada di `PATH`, lalu periksa:

```sh
jrgc --version
```

## Dari sumber

Yang dibutuhkan cuma [Rust](https://rustup.rs) versi stabil.

```sh
git clone https://github.com/Kodok-Kejepit/jrg
cd jrg
cargo build --release
./target/release/jrgc --version

# pasang ke ~/.cargo/bin
cargo install --path .
```

Build pertama memakan beberapa menit dan sekitar 2 GB memori. Kalau mesinmu kecil, batasi jumlah pekerjanya:
`cargo build --release -j 1`.

## Termux (Android)

Perintah pemasang yang sama jalan di Termux. Pemasangnya mengenali Termux dan memakai biner Android, karena biner
Linux biasa tidak jalan di sana.

Kalau mau membangun sendiri:

```sh
pkg install rust git
git clone https://github.com/Kodok-Kejepit/jrg
cd jrg
cargo build --release -j 1
```

`-j 1` penting di HP: build paralel bisa kehabisan memori di tahap optimisasi.

## Yang opsional

JRG jalan tanpa apa pun di bawah ini. Masing-masing cuma dibutuhkan fitur tertentu.

| Fitur | Butuh |
|---|---|
| mesin native (`--emit-llvm`) | `clang`; kalau JRG dibangun dari sumber, juga toolchain Rust untuk membangun runtime-nya sekali |
| `py::` | `python3` |
| `js::` | `node` |
| `php::` | `php` 8.0 ke atas |
| `lua::` | `lua` 5.3 ke atas |
| `jvm::`, `kotlin::` | JDK (`libjvm.so`) |
| `dotnet::` | runtime .NET, misalnya `dotnet-sdk-8.0` (`libhostfxr`) |
| `dok::db_*` (SQLite) | `libsqlite3` |
| `suara::` (memutar bunyi) | PulseAudio atau PipeWire |
| jendela `Canvas` | build dengan `--features canvas` |
| `jrgm apk` | Android SDK dan NDK, atau `jrgm siapkan android` |

## Berkas pertama

```sh
echo 'wok("halo")' > halo.jrg
jrgc halo.jrg
```

Lanjut ke [Tur singkat](tur.html).
