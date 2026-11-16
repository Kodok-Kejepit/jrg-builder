---
judul: Canvas, sound, and numerics
menu: Canvas, sound, numerics
ringkas: Drawing and saving PNGs, making sound, and computing with numeric arrays.
---
## Canvas

`Canvas::new(width, height)` makes a drawing surface. Colours are written `0xRRGGBB`, and the point (0, 0) is at the
top left.

```jrg canvas.jrg lokal
let c = Canvas::new(200, 120)
c.clear(0xF4EFE4)
c.fill_round_rect(10, 10, 180, 100, 14, 0xA94F28)
c.fill_circle(60, 60, 28, 0xFFFFFF)
c.draw_text(110, 52, "JRG", 0xFFFFFF)
c.save_png("card.png")

let again = Canvas::load_png("card.png")
wok(again.width(), again.height(), again.get_pixel(60, 60))
fs::delete("card.png")
```
```keluaran
200 120 Warna {a: 255, b: 255, g: 255, r: 255}
```

What is available (57 functions, all of them callable as methods too):

- **Shapes:** lines (plain and anti-aliased), rectangles, rounded rectangles, circles, ellipses, arcs, triangles,
  polygons, Bezier curves, and scaled text.
- **Fills:** solid colour, linear gradients, and radial gradients.
- **Transforms:** `translate`, `rotate`, `scale`, with a `push`/`pop` stack, and `clip`.
- **Images:** `save_png`, `save_bmp`, `Canvas::load_png`, and `blit`/`blit_region` to paste another canvas (sprites,
  tiles).
- **Input:** keyboard keys, mouse, and touch, for a canvas that has a window.

### Windows

The fourth argument of `Canvas::new` chooses where it shows:

| Mode | Result |
|---|---|
| `"polos"` (default) | no window; the image is saved to a file |
| `"tab"` | a desktop window (needs a `--features canvas` build) |
| `"web"` | shown in the browser through a local server |
| `"png"` | every frame is written as a PNG file |

The animation loop: while `c.is_open()`, draw, then call `c.update()`. `c.delta_time()` gives the length of the last
frame in seconds. For games, the [game engine](game.html) is more comfortable.

## Sound

`suara::` makes sound as an array of samples (mono, 44,100 Hz), saves it as WAV, and plays it through PulseAudio or
PipeWire.

```jrg sound.jrg lokal
let do_ = suara::nada(261.6, 300)
let mi = suara::nada(329.6, 300)
let tune = suara::sambung(suara::sambung(do_, suara::sunyi(100)), mi)
suara::simpan_wav(tune, "tune.wav")
wok(suara::durasi_ms(tune), "ms")
fs::delete("tune.wav")
```
```keluaran
700.0 ms
```

`suara::putar(sound)` plays without waiting, `putar_tunggu` waits until it ends. `suara::tersedia()` tells whether
there is an audio output. Others: `derau` (noise), `selubung` (fade in/out), and `muat_wav`.

## Numerics: wokwok

`wokwok::` has its own n-dimensional array whose contents are stored packed in one kind of number. For element-wise
work on large data, it is far faster than an ordinary array.

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

There are 86 functions: constructors (`zeros`, `ones`, `eye`, `linspace`, `random`), element-wise operations,
aggregates (`sum`, `mean`, `std`, `var`, `argmax`), shape (`reshape`, `transpose`, `flatten`), and linear algebra
(`dot`, `matmul`, `inv`, `det`, `solve`). Summation is pairwise, so a million `0.1`s add up to exactly `100000`.

### Statistics and small matrices

For data in ordinary arrays, `dok::` has statistics (`stat_percentile`, `stat_linreg`, `stat_correlation`,
`stat_normal_cdf`), vectors (`vec_dot`, `vec_cross`, `vec_normalize`), matrices (`mat_mul`, `mat_inverse`, `mat_det`),
and number theory (`num_is_prime`, `num_prime_factors`, `num_mod_pow`).

```jrg stats.jrg
wok(dok::stat_percentile([1, 2, 3, 4, 5], 50))
wok(dok::stat_linreg([1, 2, 3], [2, 4, 6]))
wok(dok::num_prime_factors(360))
```
```keluaran
3.0
DokLinReg {intercept: 0.0, r2: 1.0, slope: 2.0}
[2, 2, 2, 3, 3, 5]
```
