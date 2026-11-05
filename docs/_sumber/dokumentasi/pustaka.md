---
judul: Pustaka bawaan
ringkas: {BUILTIN} fungsi bawaan dalam namespace, ditambah 14 modul std/ yang ditulis dalam JRG.
---
## Menemukan fungsi

Tidak perlu menghafal. Dari terminal:

```sh
$ jrgc --cari sort
arr::sort
arr::sort_by
arr::sort_by_key
arr::sort_desc
heap::to_sorted_array
wokwok::argsort
wokwok::sort
$ jrgc --doc arr::sort_by_key
arr::sort_by_key(a, f) -> array
  Salinan terurut menaik menurut `f(x)`; `f` dipanggil sekali per elemen.
```

Di REPL perintahnya `:cari` dan `:doc`. Salah ketik nama fungsi dijawab dengan saran ("maksudnya `upper`?").

Kebanyakan fungsi `arr::` dan `str::` juga bisa dipanggil sebagai method: `arr::sort(a)` sama dengan `a.sort()`.

## Contoh sehari-hari

```jrg sehari.jrg lokal
wok(math::sqrt(16.0), math::gcd(12, 18), math::round(2.567))
wok(arr::unique([1, 1, 2]), arr::chunks([1, 2, 3, 4, 5], 2))
wok(str::title("budi santoso"), str::words("satu dua  tiga"))

let j = json::parse("{\"nama\": \"Ani\", \"nilai\": [90, 85]}")
wok(j["nama"], j["nilai"][1], json::stringify({"ok": true}))

wok(re::test("[0-9]+", "kode 4521"), re::replace_all("[0-9]", "x9y8", "#"))

fs::write("catatan.txt", "baris satu\nbaris dua\n")
wok(fs::read_lines("catatan.txt"), fs::size("catatan.txt"))
fs::delete("catatan.txt")

wok(crypto::sha256("halo")[0:16], crypto::base64_encode("JRG"))
wok(time::format(0, "%d/%m/%Y %H:%M"))
```
```keluaran
4.0 6 3.0
[1, 2] [[1, 2], [3, 4], [5]]
Budi Santoso [satu, dua, tiga]
Ani 85 {"ok":true}
true x#y#
[baris satu, baris dua] 21
a4e63bcacf6c172a SlJH
01/01/1970 00:00
```

## Namespace

| Namespace | Isi |
|---|---|
| `str::` (52) | teks: potong, cari, ganti, pecah, format, ubah huruf, isi kiri/kanan |
| `arr::` (65) | array: urut, saring, petakan, lipat, kelompokkan, potong-potong, zip |
| `dict::` (14) | dict: ambil, set, gabung, kunci, nilai |
| `math::` (41) | trigonometri, akar, pangkat, pembulatan, FPB/KPK, faktorial, aritmetika membungkus |
| `fs::` (34), `file::` (7) | berkas dan folder; versi `try_` menjawab `Result` |
| `time::` (12) | waktu sekarang (milidetik), format tanggal, tidur |
| `rand::` (6) | angka acak, pilih acak, kocok |
| `json::` (9) | baca dan tulis JSON |
| `re::` (8) | ekspresi reguler |
| `crypto::` (20) | SHA, MD5, HMAC, AES, base64, hex |
| `env::` (5) | argumen program dan variabel lingkungan |
| `proses::` (11) | menjalankan program lain |
| `term::` (9) | terminal: mode mentah, membaca tombol, ukuran layar |
| `http::` (8) | klien HTTP dan HTTPS, unduh ke berkas |
| `dok::` (123) | server web, router, cookie, sesi, template, WebSocket, TCP/UDP, SQLite, log, statistik |
| `canvas::` (57) | menggambar: bentuk, teks, gambar PNG, transformasi, jendela |
| `suara::` (11) | membuat, menyimpan, dan memutar bunyi |
| `wokwok::` (86) | array numerik n-dimensi yang cepat |
| `chan::`, `future::`, `task::`, `atomic::`, `mutex::` | konkurensi |
| `ptr::` (11), `jawa::` (21), `jawir::` (2) | pegangan bersama, dan alokasi manual yang diperiksa |
| `py::` `js::` `php::` `lua::` `jvm::` `kotlin::` `dotnet::` `wasm::` | bahasa lain, lihat [FFI](ffi.html) |
| `fmt::` (5) | mencetak dan merangkai teks |

Fungsi tanpa namespace yang paling sering dipakai: `wok`, `wok_`, `eprint`, `tok`, `len`, `typeof`, `str`, `int`,
`float`, `range`, `enumerate`, `zip`, `assert`, `assert_eq`, `panik`, `exit`, `sleep`, `terap`.

## Modul std/ {#modul-std}

Modul ini ditulis dalam JRG dan tertanam di `jrgc`. Pakai dengan `import "std/nama"`.

| Modul | Gunanya |
|---|---|
| `set` | himpunan: cek keanggotaan cepat, gabungan, irisan |
| `queue` | antrean (masuk belakang, keluar depan) |
| `deque` | antrean dua ujung |
| `heap` | antrean prioritas, ambil yang terkecil |
| `csv` | membaca dan menulis CSV dengan aturan tanda kutip yang benar |
| `path` | jalur berkas: ekstensi, nama tanpa ekstensi, normalisasi `.` dan `..` |
| `functional` | `compose`, `pipe`, `partial`, `curry2`, `memoize`, `once` |
| `teks` | satu kosakata untuk array dan teks, ramah pemula |
| `uang` | hitungan uang tanpa pecahan float, format Rupiah |
| `tabel` | mencetak data berkolom yang lurus |
| `option`, `result` | fungsi bantu untuk `Option` dan `Result` |
| `iterator` | iterator malas |
| `testing` | kumpulan pemeriksaan yang dilaporkan sekaligus |
| `mesin` | mesin game 2D (12 modul), lihat [Mesin game](game.html) |

```jrg uang.jrg
import "std/uang"
import "std/tabel"

let harga = uang::rupiah(19999.5)
wok(harga.rupiah(), harga.kali(3).rupiah())

var t = tabel::baru(["barang", "jumlah"]).rata_kanan(1)
t = t.baris_baru(["kopi", 3]).baris_baru(["teh manis", 12])
wok(t)
```
```keluaran
Rp 19.999,50 Rp 59.998,50
barang     jumlah
---------  ------
kopi            3
teh manis      12
```

Berkas `.jrg` milikmu sendiri menang atas nama ini: kalau ada `set.jrg` di folder program, `import "set"` memuat
berkas itu, sedangkan `import "std/set"` tetap memuat pustaka standar.
