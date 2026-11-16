---
judul: Option, Result, and errors
menu: Option and Result
ringkas: How JRG says "might not be there" and "might fail", and how to stop a program on purpose.
---
## Option

`Option` is an answer that may be empty: `Some(value)` or `None`. A built-in function that may find nothing (`first`,
`get`, `find`, `pop`, `dict::get`, `env::get`, `str::find`) returns an `Option`, never `nil` or `-1`.

```jrg option.jrg
let ada = [4, 8, 15].find(|x| x > 5)
let kosong = [4, 8, 15].find(|x| x > 50)
wok(ada, kosong)
wok(ada.is_some(), kosong.is_none())
wok(ada.unwrap(), kosong.unwrap_or(0), kosong ?? -1)
wok(ada.map(|x| x * 2), kosong.map(|x| x * 2))
```
```keluaran
Option::Some(8) Option::None
true true
8 0 -1
Option::Some(16) Option::None
```

`unwrap()` on `None` is an error. Use `unwrap_or`, `??`, or `match` when emptiness is expected.

## Result

`Result` is an answer that may fail: `Ok(value)` or `Err(reason)`. Functions that touch the outside world
(`fs::try_read`, `http::get`, FFI, databases) return a `Result`, and your own functions may too.

```jrg result.jrg
fn bagi(a, b) -> Result<int, string> {
    if b == 0 {
        return Err("pembagi nol")
    }
    return Ok(a / b)
}

wok(bagi(10, 2), bagi(1, 0))

match bagi(9, 0) {
    Ok(v) => wok("hasil", v),
    Err(e) => wok("gagal:", e),
}

wok(bagi(8, 2).unwrap_or(-1), bagi(8, 0).unwrap_or(-1))
```
```keluaran
Result::Ok(5) Result::Err(pembagi nol)
gagal: pembagi nol
4 -1
```

## Propagating failure with ?

`expression?` opens an `Ok`, or returns its `Err` straight out of the running function. It works on `Option` too: a
`None` is passed on.

```jrg tanya.jrg
fn bagi(a, b) -> Result<int, string> {
    if b == 0 { return Err("pembagi nol") }
    return Ok(a / b)
}

fn hitung(a, b, c) -> Result<int, string> {
    let x = bagi(a, b)?
    let y = bagi(x, c)?
    return Ok(y)
}

wok(hitung(100, 5, 2))
wok(hitung(100, 0, 2))
```
```keluaran
Result::Ok(10)
Result::Err(pembagi nol)
```

## When main fails

`main` may return a `Result`. An `Err` that leaves `main` is reported as an error and the program stops with exit
code 1.

```jrg main-gagal.jrg
pub fn main() -> Result<int, string> {
    return Err("berkas konfigurasi tidak ada")
}
```
```keluaran
error: program berhenti karena `Result::Err` merambat keluar dari `main`: berkas konfigurasi tidak ada — tangani dengan `match` di dalam `main`, atau pakai `.unwrap_or(...)`, kalau kegagalan ini memang boleh terjadi
```

## Stopping a program

`panik(message)` stops the program with an error. Use it for states that should be impossible; for ordinary failure,
return a `Result`.

```jrg panik.jrg
fn umur_valid(n) {
    if n < 0 {
        panik("umur tidak boleh negatif")
    }
    return n
}

wok(umur_valid(5))
wok(umur_valid(-1))
```
```keluaran
5
error: umur tidak boleh negatif
 --> panik.jrg:3:9
  |
3 |         panik("umur tidak boleh negatif")
  |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = jejak: `umur_valid` dipanggil di panik.jrg:9:5
```

The `jejak:` (trace) line shows where the function was called from. `exit(code)` ends the program with an exit code
0–255 and no message.

## Checking assumptions

`assert(condition)` and `assert_eq(a, b)` stop the program when an assumption is false. Inside a test (see the
[Quick tour](tur.html#writing-tests)), a failing assert marks that test as failed.

```jrg assert.jrg
assert(1 + 1 == 2)
assert_eq([1, 2], [1, 2])
assert_eq(2 + 2, "4")
```
```keluaran
error: assert_eq gagal — kiri `4` (int), kanan `4` (string)
 --> assert.jrg:3:1
  |
3 | assert_eq(2 + 2, "4")
  | ^^^^^^^^^^^^^^^^^^^^^
  |
```

The message names the type of both sides ("left `4` (int), right `4` (string)"), because `4` and `"4"` print the
same.
