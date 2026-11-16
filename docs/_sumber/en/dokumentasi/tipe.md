---
judul: Structs, enums, and traits
menu: Structs, enums, traits
ringkas: Making your own types, giving them methods, and sharing behaviour through traits.
---
## Struct

A struct groups named data. Fields marked `pub` can be read from outside; the rest are only visible from that type's
methods (and those of its descendants). A field may have a default, and a field without one must be filled in.

```jrg struct.jrg
struct Akun {
    pub nama: string,
    pub saldo: int = 0,
    pin: string = "0000",
}

impl Akun {
    fn baru(nama) -> Akun {
        return Akun { nama: nama }
    }

    fn setor(self, n) -> Akun {
        var a = self
        a.saldo += n
        return a
    }

    fn cek_pin(self, p) -> bool {
        return self.pin == p
    }
}

var a = Akun::baru("Ani")
a = a.setor(50)
wok(a.nama, a.saldo, a.cek_pin("1234"))
```
```keluaran
Ani 50 false
```

A function in an `impl` without `self` is called through the type name (`Akun::baru`). One that takes `self` is called
through a value (`a.setor(50)`).

Inside a method, `self` is a **copy**. Changing `self` does not change the caller's value, so a method that changes
something returns a new value and the caller reassigns it: `a = a.setor(50)`. That form does not copy large data; see
[Memory model](memori.html).

### Building from another value

`...` fills the fields that are not written from another value. It must come last.

```jrg sebar.jrg
struct Titik { pub x: int, pub y: int, pub z: int }

let awal = Titik { x: 1, y: 2, z: 3 }
let geser = Titik { x: 10, ...awal }
wok(geser)
```
```keluaran
Titik {x: 10, y: 2, z: 3}
```

### Inheritance

A struct may inherit from one other struct. The parent's fields and methods come along.

```jrg waris.jrg
struct Hewan { pub nama: string }

impl Hewan {
    fn kenalan(self) -> string { return "aku " + self.nama }
}

struct Kucing: Hewan { pub warna: string }

let k = Kucing { nama: "Oyen", warna: "oranye" }
wok(k.kenalan(), k.warna)
```
```keluaran
aku Oyen oranye
```

## Enum

An enum is a type whose value is one of several variants. A variant may carry data, positional or named.

```jrg enum.jrg
enum Bentuk {
    Lingkaran(float),
    Persegi(float),
    Segitiga { alas: float, tinggi: float },
}

fn luas(b) {
    return match b {
        Bentuk::Lingkaran(r) => 3.14 * r * r,
        Bentuk::Persegi(s) => s * s,
        Bentuk::Segitiga { alas, tinggi } => alas * tinggi / 2.0,
    }
}

wok(luas(Bentuk::Lingkaran(1.0)))
wok(luas(Bentuk::Segitiga { alas: 4.0, tinggi: 3.0 }))

enum Arah { Utara, Timur, Selatan, Barat }
wok(Arah::Utara, Arah::Utara == Arah::Barat)
```
```keluaran
3.14
6.0
Arah::Utara false
```

`Option` and `Result` are ordinary enums that come built in; both are covered in [Option and Result](galat.html).

## Trait

A trait lists the methods a type must have. A method without a body must be provided by every `impl`; a method with
a body becomes a default that may be overridden.

```jrg trait.jrg
trait Bersuara {
    fn suara(self) -> string
    fn sapa(self) -> string = self.nama + " bilang " + self.suara()
}

struct Anjing { pub nama: string }
struct Bebek { pub nama: string }

impl Bersuara for Anjing {
    fn suara(self) -> string { return "guk" }
}

impl Bersuara for Bebek {
    fn suara(self) -> string { return "kwek" }
    fn sapa(self) -> string { return self.nama + " cuma diam" }
}

wok(Anjing { nama: "Doggo" }.sapa())
wok(Bebek { nama: "Donal" }.sapa())
```
```keluaran
Doggo bilang guk
Donal cuma diam
```

An `impl` that forgets a required method is refused before the program runs.

## Generics

Functions and structs may have type parameters. Because JRG is dynamically typed, type parameters and their trait
bounds (`<T: Bersuara>`) are documentation: the names are checked, the constraint is not enforced.

```jrg generik.jrg
fn pertama<T>(xs: []T) -> T {
    return xs[0]
}

struct Kotak<T> { pub isi: T }

wok(pertama([3, 4]), pertama(["a"]), Kotak { isi: 5.5 }.isi)
```
```keluaran
3 a 5.5
```

## Your own operators and display

A type can give meaning to operators through specially named methods, and decide how it is printed through
`to_string`.

```jrg operator.jrg
struct Uang { pub rp: int }

impl Uang {
    fn add(self, o) -> Uang { return Uang { rp: self.rp + o.rp } }
    fn cmp(self, o) -> int { return self.rp - o.rp }
    fn to_string(self) -> string { return "Rp" + str(self.rp) }
}

let kopi = Uang { rp: 18000 }
let roti = Uang { rp: 12000 }
wok(kopi + roti, kopi > roti)
wok([kopi, roti], """total ${kopi + roti}""")
```
```keluaran
Rp30000 true
[Rp18000, Rp12000] total Rp30000
```

| Method | Operator |
|---|---|
| `add` `sub` `mul` `div` `rem` `pow` | `+` `-` `*` `/` `%` `**` |
| `bit_and` `bit_or` `bit_xor` `shl` `shr` | `&` `\|` `^` `<<` `>>` |
| `neg` `not` `bit_not` | `-x` `!x` `~x` |
| `eq` | `==` `!=` |
| `cmp` (returns a signed number) | `<` `<=` `>` `>=` |

`&&` and `||` cannot be overridden. `to_string` is used everywhere a value is displayed: `wok`, templates, text
concatenation, and values inside arrays, dicts, or other structs.
