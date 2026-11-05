---
judul: Nilai dan tipe
ringkas: Angka, teks, array, tuple, dict, dan cara mengikatnya ke nama.
---
## Mengikat nama

| Bentuk | Bisa ditugasi ulang? |
|---|---|
| `let x = 1` | tidak |
| `const X = 1` | tidak |
| `var x = 1` | ya |
| `x := 1` | ya, sekaligus mendeklarasikan |
| `let mut x = 1` | ya, sama dengan `var` |

```jrg ikat.jrg
var skor = 10
skor = skor + 5
x := 3
x = x * 2
const BATAS = 100
wok(skor, x, BATAS)
```
```keluaran
15 6 100
```

Nama harus dideklarasikan sebelum ditugasi. `totl = 5` atas nama yang belum ada ditolak, jadi salah ketik tidak diam-diam
membuat variabel baru. Nama yang diikat `let` juga tidak bisa diubah isinya: `let a = [1, 2]` lalu `a[0] = 9` ditolak.

Anotasi tipe boleh ditulis (`let n: int = 5`), dan JRG memeriksa bahwa nama tipenya ada. Isinya tidak ditegakkan: JRG
diketik dinamis, jadi anotasi berfungsi sebagai dokumentasi.

## Angka

`int` adalah bilangan bulat 128 bit, `float` pecahan 64 bit. Pembagian dua `int` menghasilkan `int` (dipotong ke arah
nol), dan `%` adalah sisa bagi.

```jrg angka.jrg
wok(7 / 2, 7 % 2, 7.0 / 2.0, -7 / 2)
wok(2 ** 10, 2.0 ** 0.5)
wok(1_000_000, 0xff, 0b1010)
wok(0.1 + 0.2)
```
```keluaran
3 1 3.5 -3
1024 1.4142135623730951
1000000 255 10
0.30000000000000004
```

Luapan tidak membungkus diam-diam. `170141183460469231731687303715884105727 + 1` adalah galat, begitu juga pembagian
dengan nol. Untuk aritmetika yang memang ingin membungkus (hash, pembangkit acak), pakai `math::wrap_add`,
`math::wrap_sub`, dan `math::wrap_mul`.

Tipe bulat berlebar tetap juga ada: `i8` sampai `i128`, `u8` sampai `u128`, `isize`, `usize`, dan `byte` (bilangan
0–255). Pecahan: `f32`, `f64`.

## Teks

`string` adalah teks UTF-8. Indeks menghitung karakter, bukan byte, dan satu karakter bertipe `rune` (ditulis
`'a'`).

```jrg teks.jrg
let s = "Halo"
wok(s.len(), s.upper(), s[0], s[-1], s[1:3])
wok(s + " dunia", str::repeat("ab", 3))
wok(str::split("a,b,c", ","), "  rapi  ".trim())
```
```keluaran
4 HALO H o al
Halo dunia ababab
[a, b, c] rapi
```

Ada tiga cara menulis teks literal:

```jrg literal.jrg
let nama = "Ani"
let umur = 20
wok("biasa, dengan \"escape\"\tdan tab")
wok("""halo ${nama}, tahun depan umurmu ${umur + 1}""")
wok(«sama, versi pendek: ${nama}»)
wok(`mentah: ${nama} \n tidak diproses`)
```
```keluaran
biasa, dengan "escape"	dan tab
halo Ani, tahun depan umurmu 21
sama, versi pendek: Ani
mentah: ${nama} \n tidak diproses
```

`"""…"""` dan `«…»` adalah *template*: isi `${…}` dihitung lalu disisipkan, dan boleh lebih dari satu baris. Backtick
menghasilkan teks mentah tanpa escape dan tanpa penyisipan.

`str::format` merangkai teks dengan `{}`, termasuk format angka:

```jrg format.jrg
wok(str::format("{} dapat {:.2}", "Ani", 91.456))
wok(str::pad_left("7", 3, "0"))
```
```keluaran
Ani dapat 91.46
007
```

## Array

Array berisi nilai berurutan. Indeks negatif menghitung dari belakang, dan indeks di luar batas selalu galat.

```jrg array.jrg
var a = [10, 20, 30]
a[0] = 11
a = a.push(40)
wok(a, a[-1], a[1:3], len(a))
wok(a.map(|x| x * 2), a.filter(|x| x > 20), arr::sum(a))
wok(a.contains(20), a.first(), [].first())
```
```keluaran
[11, 20, 30, 40] 40 [20, 30] 4
[22, 40, 60, 80] [30, 40] 101
true Option::Some(11) Option::None
```

Method yang "mengubah" seperti `push` mengembalikan array baru; tugaskan hasilnya kembali (`a = a.push(x)`). Bentuk
ini tidak menyalin seluruh array, jadi menambah elemen satu per satu tetap cepat.

Yang mungkin tidak menemukan apa pun (`first`, `last`, `get`, `find`, `pop`) mengembalikan `Option`, tidak pernah
`nil` atau `-1`. Lihat [Option dan Result](galat.html).

## Tuple

Tuple mengelompokkan beberapa nilai dengan tipe berbeda. Elemennya dibaca dengan `.0`, `.1`, atau `[i]`, dan bisa
dibongkar ke beberapa nama sekaligus.

```jrg tuple.jrg
fn bagi(a, b) { return (a / b, a % b) }

let t = (1, "dua", 3.0)
wok(t, t.0, t[1])
hasil, sisa := bagi(17, 5)
wok(hasil, sisa)
```
```keluaran
(1, dua, 3.0) 1 dua
3 2
```

## Dict

Dict memetakan kunci ke nilai. Kuncinya disimpan sebagai teks, jadi `d[1]` dan `d["1"]` menunjuk tempat yang sama.
Urutan cetaknya urut kunci.

```jrg dict.jrg
var stok = {"apel": 3, "jeruk": 5}
stok["mangga"] = 2
wok(stok, stok["apel"], stok.len())
wok(dict::get(stok, "durian"), dict::get_or(stok, "durian", 0))
for k, v in stok {
    wok(k, v)
}
```
```keluaran
dict {apel: 3, jeruk: 5, mangga: 2} 3 3
Option::None 0
apel 3
jeruk 5
mangga 2
```

Membaca kunci yang tidak ada lewat `d[k]` adalah galat; pakai `dict::get` (mengembalikan `Option`) atau `get_or`.

## Konversi

`as` mengubah tipe dasar. Konversi yang tidak punya jawaban tepat ditolak, bukan dipotong diam-diam.

```jrg konversi.jrg
wok(3 as float, 3.9 as int, 'A' as int, 66 as rune)
wok(int("42") + 1, float("2.5"), str(42) + "!", "17".to_int())
wok(typeof(1), typeof(1.5), typeof("a"), typeof('a'), typeof([1]), typeof({"a": 1}))
```
```keluaran
3.0 3 65 B
43 2.5 42! 17
int float string rune array dict
```

`300 as u8`, `-1 as uint`, dan `int("abc")` adalah galat. Untuk mengambil 8 bit terbawah, tulis `x & 255`.

## Nilai kosong

`nil` ada, tapi jarang dibutuhkan. Fungsi bawaan yang bisa tidak menemukan sesuatu menjawab `Option::None`, dan
`var x: int` tanpa nilai awal mendapat nilai nol tipenya (`0`, `0.0`, `""`, `false`, `[]`).
