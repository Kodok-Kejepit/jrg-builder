---
judul: Modules and import
menu: Modules
ringkas: Splitting a program into several files, and using the std/ library.
---
## One file, one module

Every `.jrg` file is a module. Another file uses it through `import`, and calls what is inside with the `name::`
prefix.

```jrg-cuplikan geometri.jrg
package geometri

pub const PI = 3.14159

pub fn luas_lingkaran(r: float) -> float {
    return PI * r * r
}

fn _bantu() { return 1 }

pub struct Titik { pub x: float, pub y: float }

impl Titik {
    pub fn jarak(self, lain: Titik) -> float {
        return math::sqrt((self.x - lain.x) ** 2.0 + (self.y - lain.y) ** 2.0)
    }
}
```

```jrg-cuplikan main.jrg
package main

import "geometri"
import "geometri" as g

pub fn main() {
    wok(geometri::luas_lingkaran(2.0))
    let a = g::Titik { x: 0.0, y: 0.0 }
    let b = geometri::Titik { x: 3.0, y: 4.0 }
    wok(a.jarak(b), g::PI)
}
```

```sh
$ jrgc main.jrg
12.56636
5.0 3.14159
```

## What is visible from outside

- What an importer can use: functions, top-level `const`/`let`, structs, enums with their variants, and the methods of
  types the module owns.
- A name starting with `_` is private. `geometri::_bantu()` is refused with a message that says why.
- A top-level `var` stays with its module. If its value has to be read from outside, provide a reader function.
- A file only sees the modules it imports itself. Modules imported by another module are not visible.

A file is one module however many times it is imported, under whatever alias.

## Where import looks

`import "geometri"` looks for `geometri.jrg` in the importing file's folder, then in the project's dependency folders
(see [jrgm](jrgm.html)), then in folders added with `jrgc -I folder`. Nested paths work too: `import "alat/teks"`
loads `alat/teks.jrg` as the module `teks`.

## The std/ library

The standard library modules are written in JRG and embedded in `jrgc`, so they work anywhere without installing
anything.

```jrg std.jrg
import "std/set"
import "std/csv"

let s = set::from([3, 1, 3, 2])
wok(s.len(), s.has(3))

let baris = csv::parse("nama,kota\n\"Ani, S.Kom\",Bandung")
wok(baris[1][0])
```
```keluaran
3 true
Ani, S.Kom
```

The list of modules is in the [Built-in library](pustaka.html#modul-std).

## Shadowing built-in names

When a program has a function with the same name as a built-in, the program's wins. `::name` always points at the
language's built-in.

```jrg bayang.jrg
fn len(a, b, c) { return a + b + c }

wok(len(1, 2, 3), ::len("abc"))
```
```keluaran
6 3
```
