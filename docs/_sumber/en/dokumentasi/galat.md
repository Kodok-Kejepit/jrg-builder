---
judul: Option, Result, and errors
menu: Option and Result
ringkas: How JRG says "might not be there" and "might fail", and how to stop a program on purpose.
---
## Option

`Option` is an answer that may be empty: `Some(value)` or `None`. A built-in function that may find nothing (`first`,
`get`, `find`, `pop`, `dict::get`, `env::get`, `str::find`) returns an `Option`, never `nil` or `-1`.

```jrg option.jrg
let found = [4, 8, 15].find(|x| x > 5)
let empty = [4, 8, 15].find(|x| x > 50)
wok(found, empty)
wok(found.is_some(), empty.is_none())
wok(found.unwrap(), empty.unwrap_or(0), empty ?? -1)
wok(found.map(|x| x * 2), empty.map(|x| x * 2))
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
fn divide(a, b) -> Result<int, string> {
    if b == 0 {
        return Err("division by zero")
    }
    return Ok(a / b)
}

wok(divide(10, 2), divide(1, 0))

match divide(9, 0) {
    Ok(v) => wok("result", v),
    Err(e) => wok("failed:", e),
}

wok(divide(8, 2).unwrap_or(-1), divide(8, 0).unwrap_or(-1))
```
```keluaran
Result::Ok(5) Result::Err(division by zero)
failed: division by zero
4 -1
```

## Propagating failure with ?

`expression?` opens an `Ok`, or returns its `Err` straight out of the running function. It works on `Option` too: a
`None` is passed on.

```jrg question.jrg
fn divide(a, b) -> Result<int, string> {
    if b == 0 { return Err("division by zero") }
    return Ok(a / b)
}

fn compute(a, b, c) -> Result<int, string> {
    let x = divide(a, b)?
    let y = divide(x, c)?
    return Ok(y)
}

wok(compute(100, 5, 2))
wok(compute(100, 0, 2))
```
```keluaran
Result::Ok(10)
Result::Err(division by zero)
```

## When main fails

`main` may return a `Result`. An `Err` that leaves `main` is reported as an error and the program stops with exit
code 1.

```jrg main-fails.jrg
pub fn main() -> Result<int, string> {
    return Err("config file not found")
}
```
```keluaran
error: the program stopped because `Result::Err` propagated out of `main`: config file not found — handle it with `match` inside `main`, or use `.unwrap_or(...)`, if this failure is allowed to happen
```

## Stopping a program

`panik(message)` stops the program with an error. Use it for states that should be impossible; for ordinary failure,
return a `Result`.

```jrg panic.jrg
fn valid_age(n) {
    if n < 0 {
        panik("age cannot be negative")
    }
    return n
}

wok(valid_age(5))
wok(valid_age(-1))
```
```keluaran
5
error: age cannot be negative
 --> panic.jrg:3:9
  |
3 |         panik("age cannot be negative")
  |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = trace: `valid_age` called at panic.jrg:9:5
```

The `trace:` line shows where the function was called from. `exit(code)` ends the program with an exit code
0–255 and no message.

## Catching an error

`tangkap(f)` runs a function that takes no arguments and answers a `Result`: `Ok` with its value, or `Err` with a
`Galat` when an error happens while it runs. The fields are `pesan` (the sentence, in the message language),
`berkas`, `baris` and `kolom` (where it happened). `defer` inside `f` still runs first.

```jrg catch.jrg
fn divide(a, b) {
    return a / b
}

match tangkap(|| divide(10, 0)) {
    Ok(v) => wok("result:", v),
    Err(g) => wok("failed at line", g.baris, "-", g.pesan),
}
wok("still running")
```
```keluaran
failed at line 2 - `10 / 0` — cannot divide by zero, the result is never defined. Check the divisor before dividing
still running
```

Every error is caught, including a stack that runs out, except `exit()`, which ends the program on purpose, and
running out of memory. Nothing is rolled back: what `f` changed before the error stays changed. `panik(g)` raises a
caught `Galat` again, with a note saying where it first happened.

## Checking assumptions

`assert(condition)` and `assert_eq(a, b)` stop the program when an assumption is false. Inside a test (see the
[Quick tour](tur.html#writing-tests)), a failing assert marks that test as failed.

```jrg assert.jrg
assert(1 + 1 == 2)
assert_eq([1, 2], [1, 2])
assert_eq(2 + 2, "4")
```
```keluaran
error: assert_eq failed — left `4` (int), right `4` (string)
 --> assert.jrg:3:1
  |
3 | assert_eq(2 + 2, "4")
  | ^^^^^^^^^^^^^^^^^^^^^
  |
```

The message names the type of both sides ("left `4` (int), right `4` (string)"), because `4` and `"4"` print the
same.
