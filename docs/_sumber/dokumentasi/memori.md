---
judul: Model memori
ringkas: Nilai tidak pernah berbagi tempat, dan salinan baru dibuat saat ada yang menulis.
---
## Nilai tidak pernah berbagi

Menugaskan, mengoper ke fungsi, mengembalikan, atau membaca dari koleksi selalu menghasilkan nilai yang berdiri
sendiri. Mengubah salinan tidak pernah mengubah aslinya.

```jrg salinan.jrg
var a = [1, 2, 3]
var b = a
b[0] = 99

fn ubah(xs) {
    var ys = xs
    ys[1] = 0
    return ys
}

let c = ubah(a)
wok(a, b, c)
```
```keluaran
[1, 2, 3] [99, 2, 3] [1, 0, 3]
```

Karena tiap nilai punya tepat satu pemilik, umurnya sama dengan umur variabel pemiliknya. JRG tidak butuh pengumpul
sampah, dan tidak ada pointer yang menggantung untuk nilai biasa.

## Menyalin itu murah

Salinan sebenarnya baru dibuat saat ada yang menulis. `var b = a` tidak menyalin apa pun; `b[0] = 99` baru menyalin.
Akibatnya:

- mengoper array besar ke fungsi yang cuma membacanya tidak menyalin apa pun;
- `a = a.push(x)`, `a = a + [x]`, dan `d = dict::set(d, k, v)` mengubah di tempat, jadi menambah elemen satu per satu tetap
  linear;
- `a[i] = v` tidak bergantung pada ukuran `a`.

Yang memang mahal: menambah di depan (`a = [x] + a`), karena itu memindahkan semua elemen. Untuk antrean dua ujung,
pakai `std/deque`.

## Yang sengaja berbagi

Ada beberapa hal yang memang dirancang untuk berbagi, dan semuanya terlihat jelas di kode:

| Apa | Contoh |
|---|---|
| variabel yang ditangkap closure | `let naik = \|\| { n += 1 }` mengubah `n` milik luar |
| pegangan sinkronisasi | `Atomic`, `Mutex`, channel |
| pegangan eksplisit | `ptr::new(nilai)` |
| alokasi manual | `jawa::alloc` dan `jawir::free` |
| objek bahasa lain | handle `py::`, `js::`, `jvm::` |

```jrg pegangan.jrg
let p = ptr::new(0)
let q = p
ptr::set(q, 42)
wok(ptr::get(p))

let m = jawa::alloc(7)
jawir::free(m)
wok(jawa::is_valid(m))
```
```keluaran
42
false
```

`jawa::` meniru malloc/free tanpa pointer mentah: memakai sesudah `free` atau `free` dua kali menghasilkan pesan galat
yang jelas, bukan crash.

## Satu kebocoran yang diketahui

Closure yang memanggil dirinya sendiri lewat variabel yang ia tangkap membentuk lingkaran kepemilikan yang tidak pernah
dibebaskan. Untuk rekursi, pakai `fn` bernama.
