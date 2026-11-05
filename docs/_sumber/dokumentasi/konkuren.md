---
judul: Konkurensi
ringkas: Goroutine, channel, select, Future, dan alat sinkronisasi.
---
Model konkurensi JRG mirip Go: kerjakan sesuatu di goroutine, lalu berkomunikasi lewat channel. Tiap goroutine adalah
thread sistem operasi sungguhan, jadi cocok untuk puluhan sampai ratusan tugas, bukan jutaan.

## spawn dan channel

`spawn { … }` (atau `go f(x)`) menjalankan blok di goroutine baru. `make_chan(tipe, kapasitas)` membuat channel;
`c <- v` mengirim, `<-c` menerima.

```jrg channel.jrg lokal
fn berat(n) { return n * n }

let c = make_chan(int, 2)
spawn { c <- berat(3) }
spawn { c <- berat(4) }
wok(<-c + <-c)
```
```keluaran
25
```

Variabel yang dipakai di dalam `spawn` disalin saat goroutine dibuat. Program menunggu semua goroutine selesai
sebelum berakhir, jadi tidak ada keluaran yang hilang karena `main` selesai duluan.

## select

`select` menunggu beberapa channel sekaligus dan mengambil yang pertama siap. Cabang `default` membuatnya tidak
menunggu.

```jrg select.jrg lokal
let hasil = make_chan(string, 0)
let selesai = make_chan(bool, 0)

spawn {
    for _, kata in ["satu", "dua", "tiga"] {
        hasil <- kata
    }
    selesai <- true
}

var jalan = true
while jalan {
    select {
        case k := <-hasil: { wok("terima", k) }
        case _ := <-selesai: { jalan = false }
    }
}
```
```keluaran
terima satu
terima dua
terima tiga
```

## Future dan Task

Untuk pola "mulai sekarang, ambil hasilnya nanti", pakai `Future::new(fungsi, argumen…)` lalu `future::await`.

```jrg future.jrg lokal
fn hitung(n) {
    var t = 0
    for _, i in 0..n { t += i }
    return t
}

let a = Future::new(hitung, 1000)
let b = Future::new(hitung, 2000)
wok(future::await(a) + future::await(b))
```
```keluaran
2498500
```

Hasil sebuah Future cuma bisa diambil sekali; mengambilnya lagi adalah galat. Fungsi lain di keluarga ini:
`future::all`, `future::race`, `future::timeout`, `future::try_get`, `future::is_done`, `Task::spawn`, dan
`task::join`. Kalau fungsi di dalam Future gagal, galatnya muncul di tempat hasilnya diambil.

## Atomic dan Mutex

Nilai biasa tidak pernah berbagi, jadi untuk keadaan bersama antar-goroutine JRG punya dua pegangan: `Atomic` (bilangan
bulat yang aman ditambah dari banyak thread) dan `Mutex` (kunci).

```jrg atomic.jrg lokal
let n = Atomic::new(0)
var tugas = []
for _, i in 0..4 {
    tugas = tugas.push(Task::spawn(|| {
        for _, j in 0..1000 { n.add(1) }
    }))
}
for _, t in tugas { task::join(t) }
wok(n.get())

let kunci = Mutex::new()
mutex::with_lock(kunci, || wok("di dalam kunci"))
```
```keluaran
4000
di dalam kunci
```

Menyalin `Atomic` atau `Mutex` menyalin pegangannya, jadi semua salinan menunjuk keadaan yang sama. `with_lock`
melepas kuncinya sendiri, termasuk kalau fungsinya gagal.

## Yang tidak ada

JRG tidak punya `async`/`await`. Goroutine-nya thread sungguhan, bukan tugas ringan, dan sintaks `async` akan
menjanjikan biaya yang tidak sesuai kenyataannya. Untuk menunggu banyak hal sekaligus, pakai channel, `select`, atau
`future::all`.
