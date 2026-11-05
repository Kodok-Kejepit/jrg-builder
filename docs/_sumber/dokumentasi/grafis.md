---
judul: Kanvas, suara, dan numerik
menu: Kanvas, suara, numerik
ringkas: Menggambar dan menyimpan PNG, membuat bunyi, dan menghitung dengan array numerik.
---
## Kanvas

`Canvas::new(lebar, tinggi)` membuat bidang gambar. Warna ditulis `0xRRGGBB`, dan titik (0, 0) ada di kiri atas.

```jrg kanvas.jrg lokal
let c = Canvas::new(200, 120)
c.clear(0xF4EFE4)
c.fill_round_rect(10, 10, 180, 100, 14, 0xA94F28)
c.fill_circle(60, 60, 28, 0xFFFFFF)
c.draw_text(110, 52, "JRG", 0xFFFFFF)
c.save_png("kartu.png")

let lagi = Canvas::load_png("kartu.png")
wok(lagi.width(), lagi.height(), lagi.get_pixel(60, 60))
fs::delete("kartu.png")
```
```keluaran
200 120 Warna {a: 255, b: 255, g: 255, r: 255}
```

Yang tersedia (57 fungsi, semuanya juga bisa dipanggil sebagai method):

- **Bentuk:** garis (biasa dan anti-alias), persegi, persegi bersudut tumpul, lingkaran, elips, busur, segitiga, poligon,
  kurva Bezier, dan teks berskala.
- **Isian:** warna polos, gradien linear, dan gradien radial.
- **Transformasi:** `translate`, `rotate`, `scale`, dengan tumpukan `push`/`pop`, dan pemotongan `clip`.
- **Gambar:** `save_png`, `save_bmp`, `Canvas::load_png`, serta `blit`/`blit_region` untuk menempel kanvas lain
  (sprite, petak).
- **Masukan:** tombol keyboard, tetikus, dan sentuhan, untuk kanvas yang punya jendela.

### Jendela

Argumen keempat `Canvas::new` memilih tempat tampilnya:

| Mode | Hasil |
|---|---|
| `"polos"` (bawaan) | tanpa jendela; gambar disimpan ke berkas |
| `"tab"` | jendela desktop (butuh build `--features canvas`) |
| `"web"` | tampil di peramban lewat server lokal |
| `"png"` | tiap bingkai ditulis jadi berkas PNG |

Putaran animasinya: selama `c.is_open()`, gambar, lalu panggil `c.update()`. `c.delta_time()` memberi lama bingkai
terakhir dalam detik. Untuk game, lebih enak memakai [mesin game](game.html).

## Suara

`suara::` membuat bunyi sebagai array sampel (mono, 44.100 Hz), menyimpannya sebagai WAV, dan memutarnya lewat
PulseAudio atau PipeWire.

```jrg suara.jrg lokal
let do_ = suara::nada(261.6, 300)
let mi = suara::nada(329.6, 300)
let lagu = suara::sambung(suara::sambung(do_, suara::sunyi(100)), mi)
suara::simpan_wav(lagu, "lagu.wav")
wok(suara::durasi_ms(lagu), "ms")
fs::delete("lagu.wav")
```
```keluaran
700.0 ms
```

`suara::putar(bunyi)` memutar tanpa menunggu, `putar_tunggu` menunggu sampai selesai. `suara::tersedia()` memberi
tahu apakah ada keluaran audio. Fungsi lain: `derau`, `selubung` (fade in/out), dan `muat_wav`.

## Numerik: wokwok

`wokwok::` punya array n-dimensi sendiri yang isinya disimpan rapat dalam satu jenis angka. Untuk perhitungan
elemen per elemen atas data besar, ia jauh lebih cepat daripada array biasa.

```jrg wokwok.jrg
let a = wokwok::arange(0.0, 1.0, 0.25)
wok(a, wokwok::sum(a), wokwok::mean(a))

let m = wokwok::array([[1.0, 2.0], [3.0, 4.0]])
wok(wokwok::matmul(m, m), wokwok::det(m), wokwok::shape(m))
wok(wokwok::mul(wokwok::linspace(0.0, 1.0, 5), 10.0))
```
```keluaran
[0.0, 0.25, 0.5, 0.75] 1.5 0.375
[[7.0, 10.0], [15.0, 22.0]] -2.0 [2, 2]
[0.0, 2.5, 5.0, 7.5, 10.0]
```

Ada 86 fungsi: pembuat array (`zeros`, `ones`, `eye`, `linspace`, `random`), operasi elemen per elemen, agregat
(`sum`, `mean`, `std`, `var`, `argmax`), bentuk (`reshape`, `transpose`, `flatten`), dan aljabar linear (`dot`,
`matmul`, `inv`, `det`, `solve`). Penjumlahannya memakai cara berpasangan, jadi sejuta angka `0.1` berjumlah tepat
`100000`.

### Statistik dan matriks kecil

Untuk data yang berupa array biasa, `dok::` punya statistik (`stat_percentile`, `stat_linreg`, `stat_correlation`,
`stat_normal_cdf`), vektor (`vec_dot`, `vec_cross`, `vec_normalize`), matriks (`mat_mul`, `mat_inverse`, `mat_det`),
dan teori bilangan (`num_is_prime`, `num_prime_factors`, `num_mod_pow`).

```jrg statistik.jrg
wok(dok::stat_percentile([1, 2, 3, 4, 5], 50))
wok(dok::stat_linreg([1, 2, 3], [2, 4, 6]))
wok(dok::num_prime_factors(360))
```
```keluaran
3.0
DokLinReg {intercept: 0.0, r2: 1.0, slope: 2.0}
[2, 2, 2, 3, 3, 5]
```
