---
judul: Kendali alur
ringkas: Percabangan, perulangan, dan cara keluar dari keduanya.
---
## if dan else

Kondisinya tanpa kurung, badannya selalu berkurung kurawal. `if` juga bisa dipakai sebagai nilai.

```jrg if.jrg
let n = 7
if n % 2 == 0 {
    wok("genap")
} else if n > 5 {
    wok("ganjil dan besar")
} else {
    wok("ganjil")
}

let jenis = if n > 5 { "besar" } else { "kecil" }
wok(jenis)
```
```keluaran
ganjil dan besar
besar
```

## Ternary

Bentuk pendeknya `kondisi $ jika_benar : jika_salah`. Pembukanya `$`, bukan `?`, karena `?` dipakai untuk meneruskan
galat (lihat [Option dan Result](galat.html)).

```jrg ternary.jrg
let nilai = 72
wok(nilai >= 75 $ "lulus" : "remedial")
```
```keluaran
remedial
```

## for

`for` punya dua bentuk, dan bedanya penting:

- `for i, x in koleksi`: `i` indeksnya, `x` nilainya. Pakai `_` untuk yang tidak dipakai.
- `for i in koleksi`: **satu nama selalu berarti indeks**, termasuk untuk rentang.

```jrg for.jrg
let buah = ["apel", "jeruk"]
for i, b in buah {
    wok(i, b)
}

var nilai = []
for _, angka in 5..8 {
    nilai = nilai.push(angka)
}
wok(nilai)

var indeks = []
for i in 5..8 {
    indeks = indeks.push(i)
}
wok(indeks)
```
```keluaran
0 apel
1 jeruk
[5, 6, 7]
[0, 1, 2]
```

Baris terakhir berisi `[0, 1, 2]`, bukan `[5, 6, 7]`: dengan satu nama, `i` adalah indeks ke-0, ke-1, dan ke-2 dari
rentang itu. Kalau yang kamu mau nilainya, tulis `for _, i in 5..8`.

`a..b` tidak memuat `b`, `a..=b` memuat. Bentuk dua nama juga berlaku untuk teks (indeks dan karakternya) dan dict
(kunci dan nilainya). Bentuk gaya C juga ada:

```jrg for-c.jrg
for i := 0; i < 10; i += 3 {
    wok(i)
}
```
```keluaran
0
3
6
9
```

Rentang di kepala `for` tidak pernah dijadikan array, jadi `for _, i in 0..1000000000` tidak memakan memori.

## while dan loop

```jrg while.jrg
var n = 27
var langkah = 0
while n != 1 {
    n = n % 2 == 0 $ n / 2 : 3 * n + 1
    langkah += 1
}
wok("Collatz 27:", langkah, "langkah")

var tebakan = 0
loop {
    tebakan += 1
    if tebakan * tebakan > 50 { break }
}
wok(tebakan)
```
```keluaran
Collatz 27: 111 langkah
8
```

## break dan continue berlabel

`break` dan `continue` mengenai perulangan terdekat. Untuk keluar dari perulangan luar, beri label yang diawali tanda
petik tunggal.

```jrg label.jrg
'cari: for _, i in 1..10 {
    for _, j in 1..10 {
        if i * j == 42 {
            wok("ketemu:", i, j)
            break 'cari
        }
    }
}
```
```keluaran
ketemu: 6 7
```

## match

`match` mencocokkan sebuah nilai dengan pola dan menjalankan cabang pertama yang cocok. Ia juga sebuah ekspresi.

```jrg match.jrg
fn hari(n) {
    return match n {
        0 | 6 => "akhir pekan",
        1..=5 => "hari kerja",
        _ => "bukan hari",
    }
}
wok(hari(0), hari(3), hari(9))
```
```keluaran
akhir pekan hari kerja bukan hari
```

Pola lengkapnya (tuple, array, struct, enum, penjaga `if`, pengikat `@`) ada di [Pola](pola.html).

## defer

`defer` menunda sebuah panggilan sampai fungsinya selesai, termasuk kalau fungsinya berhenti karena galat. Yang
didaftarkan terakhir jalan duluan, dan argumennya dibekukan saat baris `defer` dibaca.

```jrg defer.jrg
fn proses() {
    var berkas = "laporan.txt"
    defer wok("tutup", berkas)
    defer wok("simpan")
    berkas = "lain.txt"
    wok("kerja dengan", berkas)
}
proses()
```
```keluaran
kerja dengan lain.txt
simpan
tutup laporan.txt
```
