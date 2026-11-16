---
judul: Built-in library
ringkas: {BUILTIN} built-in functions in namespaces, plus the std/ modules written in JRG.
---
## Finding a function

No need to memorise. From the terminal:

```sh
$ jrgc --cari sort
arr::sort
arr::sort_by
arr::sort_by_key
arr::sort_desc
heap::to_sorted_array
wokwok::argsort
wokwok::sort
$ jrgc --doc arr::sort_by_key
arr::sort_by_key(a, f) -> array
  Salinan terurut menaik menurut `f(x)`; `f` dipanggil sekali per elemen.
```

In the REPL the commands are `:cari` and `:doc`. A mistyped function name is answered with a suggestion ("did you
mean `upper`?").

Most `arr::` and `str::` functions can also be called as methods: `arr::sort(a)` is the same as `a.sort()`.

## Everyday examples

```jrg everyday.jrg lokal
wok(math::sqrt(16.0), math::gcd(12, 18), math::round(2.567))
wok(arr::unique([1, 1, 2]), arr::chunks([1, 2, 3, 4, 5], 2))
wok(str::title("ada lovelace"), str::words("one two  three"))

let j = json::parse("{\"name\": \"Ani\", \"scores\": [90, 85]}")
wok(j["name"], j["scores"][1], json::stringify({"ok": true}))

wok(re::test("[0-9]+", "code 4521"), re::replace_all("[0-9]", "x9y8", "#"))

fs::write("notes.txt", "line one\nline two\n")
wok(fs::read_lines("notes.txt"), fs::size("notes.txt"))
fs::delete("notes.txt")

wok(crypto::sha256("hello")[0:16], crypto::base64_encode("JRG"))
wok(time::format(0, "%d/%m/%Y %H:%M"))
```
```keluaran
4.0 6 3.0
[1, 2] [[1, 2], [3, 4], [5]]
Ada Lovelace [one, two, three]
Ani 85 {"ok":true}
true x#y#
[line one, line two] 18
2cf24dba5fb0a30e SlJH
01/01/1970 00:00
```

## Namespace

| Namespace | Contents |
|---|---|
| `str::` (52) | text: slice, search, replace, split, format, change case, pad left/right |
| `arr::` (65) | arrays: sort, filter, map, fold, group, chunk, zip |
| `dict::` (14) | dicts: get, set, merge, keys, values |
| `math::` (41) | trigonometry, roots, powers, rounding, GCD/LCM, factorial, wrapping arithmetic |
| `fs::` (34), `file::` (7) | files and folders; the `try_` versions answer a `Result` |
| `time::` (12) | the current time (milliseconds), date formatting, sleep |
| `rand::` (6) | random numbers, random choice, shuffle |
| `json::` (9) | read and write JSON |
| `re::` (8) | regular expressions |
| `crypto::` (20) | SHA, MD5, HMAC, AES, base64, hex |
| `env::` (5) | program arguments and environment variables |
| `proses::` (11) | running other programs |
| `term::` (9) | the terminal: raw mode, reading keys, screen size |
| `http::` (8) | an HTTP and HTTPS client, downloading to a file |
| `dok::` (123) | web server, router, cookies, sessions, templates, WebSocket, TCP/UDP, SQLite, logging, statistics |
| `canvas::` (57) | drawing: shapes, text, PNG images, transforms, windows |
| `suara::` (11) | making, saving, and playing sound |
| `wokwok::` (86) | fast n-dimensional numeric arrays |
| `chan::`, `future::`, `task::`, `atomic::`, `mutex::` | concurrency |
| `ptr::` (11), `jawa::` (21), `jawir::` (2) | shared handles, and checked manual allocation |
| `py::` `js::` `php::` `lua::` `jvm::` `kotlin::` `dotnet::` `wasm::` | other languages, see [FFI](ffi.html) |
| `fmt::` (5) | printing and building text |

The most used functions without a namespace: `wok`, `wok_`, `eprint`, `tok`, `len`, `typeof`, `str`, `int`,
`float`, `range`, `enumerate`, `zip`, `assert`, `assert_eq`, `panik`, `exit`, `sleep`, `terap`.

## The std/ modules {#modul-std}

These modules are written in JRG and embedded in `jrgc`. Use them with `import "std/name"`.

| Module | What it is for |
|---|---|
| `set` | sets: fast membership, union, intersection |
| `queue` | a queue (in at the back, out at the front) |
| `deque` | a double-ended queue |
| `heap` | a priority queue, take the smallest |
| `csv` | reading and writing CSV with correct quoting rules |
| `path` | file paths: extension, name without extension, normalising `.` and `..` |
| `functional` | `compose`, `pipe`, `partial`, `curry2`, `memoize`, `once` |
| `teks` | one vocabulary for arrays and text, beginner friendly |
| `uang` | money arithmetic without floats, Rupiah formatting |
| `tabel` | printing aligned columns |
| `option`, `result` | helpers for `Option` and `Result` |
| `iterator` | lazy iterators |
| `testing` | groups of checks reported together |
| `mesin` | a 2D game engine (12 modules), see [Game engine](game.html) |

```jrg money.jrg
import "std/uang"
import "std/tabel"

let price = uang::rupiah(19999.5)
wok(price.rupiah(), price.kali(3).rupiah())

var t = tabel::baru(["item", "qty"]).rata_kanan(1)
t = t.baris_baru(["coffee", 3]).baris_baru(["sweet tea", 12])
wok(t)
```
```keluaran
Rp 19.999,50 Rp 59.998,50
item       qty
---------  ---
coffee       3
sweet tea   12
```

Your own `.jrg` files win over these names: if there is a `set.jrg` in the program's folder, `import "set"` loads that
file, while `import "std/set"` still loads the standard library.
