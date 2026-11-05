---
judul: Mesin game
ringkas: std/mesin, mesin game 2D di atas kanvas, yang bisa diuji tanpa membuka jendela.
---
`import "std/mesin"` memberi mesin game 2D yang ditulis dalam JRG. Sebuah game terdiri dari tiga hal: keadaan awal,
fungsi `perbarui` yang menghitung keadaan berikutnya, dan fungsi `gambar`.

## Game terkecil

```jrg bola.jrg
package main
import "std/mesin"
import "std/mesin/warna"

struct Bola { pub x: float }

fn perbarui(k, m, dt) {
    var s = k
    s.x = k.x + m.arah("Left", "Right", "Up", "Down").x * 60.0 * dt
    return s
}

fn gambar(c, k) {
    canvas::fill_circle(c, k.x, 40, 6, warna::MERAH)
}

pub fn main() {
    let g = mesin::baru("Bola", 160, 80, Bola { x: 80.0 }, perbarui, gambar)
    if canvas::punya_jendela() {
        g.jalankan()
    } else {
        wok(g.simulasi([["Right"], ["Right"]]).keadaan.x)
    }
}
```
```keluaran
82.0
```

Di komputer yang punya layar, `g.jalankan()` membuka jendela dan bola bisa digeser dengan panah. Di tempat lain,
`simulasi` memajukan game dua langkah dengan tombol `Right` ditahan, lalu mencetak posisinya.

## Tiga aturan

1. **Keadaan adalah nilai.** `perbarui(keadaan, masukan, dt)` mengembalikan keadaan baru; tidak ada yang diubah di
   tempat. Karena itu game bisa diputar ulang dan diuji.
2. **Langkah waktu tetap**, bawaannya 1/60 detik (ubah dengan `dengan_dt`). Fisikanya sama di komputer cepat dan
   lambat.
3. **Bisa jalan tanpa jendela.** `simulasi(rekaman)` menerima daftar tombol yang ditahan di tiap langkah.
   `dengan_rekam()` merekam permainan sungguhan, dan `putar(rekaman)` mengulangnya sampai ke keadaan yang sama, jadi
   bug yang muncul saat bermain bisa dijadikan tes.

Field struct keadaan perlu `pub`, karena `perbarui` dan `gambar` adalah fungsi biasa, bukan method.

## Modul

| Modul | Isi |
|---|---|
| `std/mesin` | `mesin::baru`, `langkah`, `simulasi`, `jalankan`, `dengan_latar`, `dengan_dt`, `dengan_rekam`, `putar` |
| `std/mesin/vek` | `Vek2` dengan operator `+ - * /`, `panjang`, `normal`, `putar`, `sudut`, `lerp`, `batasi` |
| `std/mesin/warna` | warna bernama, `rgb`, `rgba`, `hex`, `campur`, `gelap`, `terang`, `alfa` |
| `std/mesin/masukan` | `tahan`, `tekan`, `lepas`, `arah`, tetikus, dan tombol layar untuk HP |
| `std/mesin/fisika` | `Kotak`, `Lingkaran`, `tabrak`, `dorong_keluar`, `Benda` |
| `std/mesin/kamera` | geser dan zoom, `ikuti`, `batasi`, konversi layar ke dunia |
| `std/mesin/peta` | peta petak dari baris teks, tabrakan per petak, `gerak` yang berhenti tepat di tepi |
| `std/mesin/sprite` | lembar sprite, gambar dicerminkan, animasi bingkai |
| `std/mesin/partikel` | sistem partikel dengan acak berbenih, jadi bisa diulang |
| `std/mesin/tween` | kurva pelonggaran (nama dari easings.net) |
| `std/mesin/ui` | `label`, `tombol`, `bilah`, dan `menu` yang dinavigasi keyboard |
| `std/mesin/adegan` | tumpukan adegan: menu, permainan, layar jeda |

Tiap modul dipanggil dengan nama terakhir jalurnya: `import "std/mesin/vek"` memberi `vek::`.

## Di HP

```jrg-cuplikan
if canvas::layar_sentuh() {
    g = g.dengan_tombol_layar(["Left", "Right", "Space"])
}
```

Tombol layar digambar di pita bawah layar dan dibaca seperti tombol keyboard, jadi `perbarui` tidak perlu diubah.
Game JRG bisa dibungkus jadi APK dengan `jrgm apk` (lihat [Aplikasi Android](android.html)).

## Bunyi

Bunyi diturunkan dari perubahan keadaan, bukan diputar dari dalam `perbarui`, supaya putar ulang tetap sunyi:

```jrg-cuplikan
fn bunyi(lama, baru) {
    if baru.skor > lama.skor {
        return [suara::nada(880, 80, "kotak")]
    }
    return []
}

let g = mesin::baru("Tangkap", 160, 120, awal, perbarui, gambar).dengan_bunyi(bunyi)
```
