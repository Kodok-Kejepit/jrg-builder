---
judul: Game engine
ringkas: std/mesin, a 2D game engine on top of the canvas, which can be tested without opening a window.
---
`import "std/mesin"` gives a 2D game engine written in JRG. A game is three things: an initial state, an `update` function
that computes the next state, and a `draw` function. The engine's own API keeps its Indonesian names (`mesin::baru`,
`simulasi`, `jalankan`).

## The smallest game

```jrg ball.jrg
package main
import "std/mesin"
import "std/mesin/warna"

struct Ball { pub x: float }

fn update(state, input, dt) {
    var s = state
    s.x = state.x + input.arah("Left", "Right", "Up", "Down").x * 60.0 * dt
    return s
}

fn draw(c, state) {
    canvas::fill_circle(c, state.x, 40, 6, warna::MERAH)
}

pub fn main() {
    let g = mesin::baru("Ball", 160, 80, Ball { x: 80.0 }, update, draw)
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

On a computer with a screen, `g.jalankan()` opens a window and the ball moves with the arrow keys. Elsewhere,
`simulasi` advances the game two steps with `Right` held, then prints the position.

## Three rules

1. **State is a value.** `update(state, input, dt)` returns a new state; nothing is changed in place. That is why
   a game can be replayed and tested.
2. **A fixed time step**, 1/60 second by default (change it with `dengan_dt`). The physics is the same on fast and
   slow computers.
3. **It runs without a window.** `simulasi(recording)` takes the list of keys held at each step. `dengan_rekam()`
   records a real play session, and `putar(recording)` replays it to the same state, so a bug that shows up while
   playing can become a test.

The state struct's fields need `pub`, because `update` and `draw` are plain functions, not methods.

## Modules

| Module | Contents |
|---|---|
| `std/mesin` | `mesin::baru`, `langkah`, `simulasi`, `jalankan`, `dengan_latar`, `dengan_dt`, `dengan_rekam`, `putar` |
| `std/mesin/vek` | `Vek2` with operators `+ - * /`, `panjang`, `normal`, `putar`, `sudut`, `lerp`, `batasi` |
| `std/mesin/warna` | named colours, `rgb`, `rgba`, `hex`, `campur`, `gelap`, `terang`, `alfa` |
| `std/mesin/masukan` | `tahan`, `tekan`, `lepas`, `arah`, mouse, and on-screen buttons for phones |
| `std/mesin/fisika` | `Kotak`, `Lingkaran`, `tabrak`, `dorong_keluar`, `Benda` |
| `std/mesin/kamera` | pan and zoom, `ikuti`, `batasi`, screen-to-world conversion |
| `std/mesin/peta` | tile maps from lines of text, per-tile collision, a `gerak` that stops exactly at the edge |
| `std/mesin/sprite` | sprite sheets, mirrored images, frame animation |
| `std/mesin/partikel` | a particle system with seeded randomness, so it can be replayed |
| `std/mesin/tween` | easing curves (names from easings.net) |
| `std/mesin/ui` | `label`, `tombol`, `bilah`, and a keyboard-navigated `menu` |
| `std/mesin/adegan` | a scene stack: menu, gameplay, pause screen |

Each module is used by the last part of its path: `import "std/mesin/vek"` gives `vek::`.

## On a phone

```jrg-cuplikan
if canvas::layar_sentuh() {
    g = g.dengan_tombol_layar(["Left", "Right", "Space"])
}
```

On-screen buttons are drawn in a strip at the bottom and read like keyboard keys, so `update` does not change. A
JRG game can be packaged as an APK with `jrgm apk` (see [Android apps](android.html)).

## Sound

Sound is derived from state changes, not played from inside `update`, so a replay stays silent:

```jrg-cuplikan
fn sounds(old, new) {
    if new.score > old.score {
        return [suara::nada(880, 80, "kotak")]
    }
    return []
}

let g = mesin::baru("Catch", 160, 120, start, update, draw).dengan_bunyi(sounds)
```
