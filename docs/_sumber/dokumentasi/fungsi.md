---
judul: Fungsi dan closure
ringkas: Mendeklarasikan fungsi, parameter yang fleksibel, fungsi sebagai nilai, dan closure.
---
## Mendeklarasikan fungsi

```jrg fungsi.jrg
fn luas(p: int, l: int) -> int {
    return p * l
}

fn keliling(p, l) {
    return 2 * (p + l)
}

fn kali_dua(x) = x * 2

wok(luas(3, 4), keliling(3, 4), kali_dua(5))
```
```keluaran
12 14 10
```

Anotasi tipe di parameter dan nilai kembalian boleh ditulis atau dilewati. `fn f(x) = ekspresi` adalah bentuk pendek
untuk fungsi yang cuma mengembalikan satu ekspresi. Fungsi boleh dipanggil sebelum barisnya, dan fungsi yang
dideklarasikan di dalam fungsi lain juga boleh (tapi ia tidak melihat variabel lokal fungsi luarnya; untuk itu pakai
closure).

## Parameter

```jrg parameter.jrg
fn sapa(nama, salam = "halo") {
    return salam + ", " + nama
}

fn total(angka...) {
    return arr::sum(angka)
}

fn bagi(pembilang, penyebut) {
    return pembilang / penyebut
}

wok(sapa("Ani"), sapa("Budi", "selamat pagi"))
wok(total(), total(1, 2, 3))
wok(total(...[4, 5, 6]))
wok(bagi(penyebut: 2, pembilang: 10))
```
```keluaran
halo, Ani selamat pagi, Budi
0 6
15
5
```

- **Nilai bawaan** (`salam = "halo"`) dihitung saat dipanggil, dan boleh memakai parameter sebelumnya.
- **Pengumpul** (`angka...`) menerima sisa argumen sebagai array; kalau tidak ada, isinya `[]`.
- **Sebar** (`...arr`) kebalikannya: isi array jadi argumen satu per satu.
- **Argumen bernama** (`penyebut: 2`) boleh diurutkan bebas dan dicampur dengan argumen posisi.

Parameter boleh berupa pola: `fn jarak((x1, y1), (x2, y2))` membongkar dua tuple langsung di kepala fungsi.

## Mengembalikan beberapa nilai

Kembalikan tuple, lalu bongkar di pemanggil:

```jrg kembali.jrg
fn statistik(xs) {
    return (arr::min(xs), arr::max(xs), arr::sum(xs) / len(xs))
}

kecil, besar, rata := statistik([4, 8, 15, 16, 23, 42])
wok(kecil, besar, rata)
```
```keluaran
4 42 18
```

## Closure

Closure adalah fungsi tanpa nama yang ditulis di tempat: `|x| x * x`, atau `|a, b| { … }` kalau badannya lebih dari
satu ekspresi.

```jrg closure.jrg
let kuadrat = |x| x * x
let nilai = [1, 2, 3, 4, 5]
wok(nilai.map(kuadrat))
wok(nilai.filter(|x| x % 2 == 1))
wok(nilai.reduce(|a, b| a + b, 0))
wok(nilai.map(|x| x * 10).filter(|x| x > 20))
```
```keluaran
[1, 4, 9, 16, 25]
[1, 3, 5]
15
[30, 40, 50]
```

Closure **mengingat variabel**, bukan salinan nilainya. Perubahan di dalam terlihat di luar, dan sebaliknya:

```jrg tangkap.jrg
var hitung = 0
let naik = || { hitung += 1 }
naik()
naik()
wok(hitung)

fn pembuat_tambah(n) {
    return |x| x + n
}
let tambah10 = pembuat_tambah(10)
wok(tambah10(5))
```
```keluaran
2
15
```

Kalau kamu ingin closure memotret nilainya saat dibuat, tulis `move`:

```jrg move.jrg
var n = 1
let lihat = || n
let foto = move || n
n = 9
wok(lihat(), foto())
```
```keluaran
9 1
```

## Fungsi sebagai nilai

Fungsi bernama, closure, fungsi bawaan, dan konstruktor varian enum semuanya nilai yang bisa disimpan dan dioper.

```jrg nilai-fungsi.jrg
fn ganda(x) = x * 2

let langkah = [ganda, |x| x + 1, math::abs]
var v = -5
for _, f in langkah {
    v = f(v)
}
wok(v)
wok([3, -1, 2].map(math::abs), ["a", "b"].map(str::upper))
wok(4 |> ganda |> ganda)
wok(terap(math::max, [3, 9]))
```
```keluaran
9
[3, 1, 2] [A, B]
16
9
```

`x |> f` sama dengan `f(x)`, berguna untuk merangkai langkah dari kiri ke kanan. `terap(f, args)` memanggil `f` dengan
isi array sebagai argumennya.

Dua fungsi tidak bisa dibandingkan dengan `==`; JRG menolaknya karena tidak ada jawaban yang benar untuk "apakah dua
fungsi sama".

## Rekursi

```jrg rekursi.jrg
fn fib(n) {
    return n < 2 $ n : fib(n - 1) + fib(n - 2)
}
wok(fib(20))
```
```keluaran
6765
```

Kedalaman rekursi dibatasi besar stack: sekitar 20 ribu lapis di interpreter, sampai sejuta di `--vm`, dan ratusan
ribu di biner `--emit-llvm`. Melewatinya menghasilkan galat yang menyebut fungsinya, bukan crash. Untuk rekursi yang
berulang-ulang, pakai `fn` bernama, bukan closure yang memanggil dirinya sendiri.
