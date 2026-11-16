---
judul: Values and types
ringkas: Numbers, text, arrays, tuples, dicts, and how to bind them to names.
---
## Binding names

| Form | Can be reassigned? |
|---|---|
| `let x = 1` | no |
| `const X = 1` | no |
| `var x = 1` | yes |
| `x := 1` | yes, and it declares the name |
| `let mut x = 1` | yes, same as `var` |

```jrg bind.jrg
var score = 10
score = score + 5
x := 3
x = x * 2
const LIMIT = 100
wok(score, x, LIMIT)
```
```keluaran
15 6 100
```

A name must be declared before it is assigned. `totl = 5` on a name that does not exist is refused, so a typo never
quietly creates a new variable. A name bound with `let` cannot have its contents changed either: `let a = [1, 2]`
followed by `a[0] = 9` is refused.

Type annotations may be written (`let n: int = 5`), and JRG checks that the type name exists. The type itself is not
enforced: JRG is dynamically typed, so annotations work as documentation.

## Numbers

`int` is a 128-bit integer, `float` a 64-bit floating-point number. Dividing two `int`s gives an `int` (truncated
toward zero), and `%` is the remainder.

```jrg numbers.jrg
wok(7 / 2, 7 % 2, 7.0 / 2.0, -7 / 2)
wok(2 ** 10, 2.0 ** 0.5)
wok(1_000_000, 0xff, 0b1010)
wok(0.1 + 0.2)
```
```keluaran
3 1 3.5 -3
1024 1.4142135623730951
1000000 255 10
0.30000000000000004
```

Overflow never wraps silently. `170141183460469231731687303715884105727 + 1` is an error, and so is division by zero.
For arithmetic that really should wrap (hashes, random generators), use `math::wrap_add`, `math::wrap_sub`, and
`math::wrap_mul`.

Fixed-width integer types exist too: `i8` to `i128`, `u8` to `u128`, `isize`, `usize`, and `byte` (a number 0–255).
Floating point: `f32`, `f64`.

## Text

`string` is UTF-8 text. Indices count characters, not bytes, and a single character has type `rune` (written
`'a'`).

```jrg text.jrg
let s = "Hello"
wok(s.len(), s.upper(), s[0], s[-1], s[1:3])
wok(s + " world", str::repeat("ab", 3))
wok(str::split("a,b,c", ","), "  tidy  ".trim())
```
```keluaran
5 HELLO H o el
Hello world ababab
[a, b, c] tidy
```

There are three ways to write a text literal:

```jrg literal.jrg
let name = "Ani"
let age = 20
wok("plain, with \"escapes\"\tand a tab")
wok("""hello ${name}, next year you will be ${age + 1}""")
wok(«same, short form: ${name}»)
wok(`raw: ${name} \n not processed`)
```
```keluaran
plain, with "escapes"	and a tab
hello Ani, next year you will be 21
same, short form: Ani
raw: ${name} \n not processed
```

`"""…"""` and `«…»` are *templates*: what is inside `${…}` is evaluated and inserted, and they may span several
lines. Backticks give raw text, with no escapes and no insertion.

`str::format` builds text with `{}`, including number formatting:

```jrg format.jrg
wok(str::format("{} scored {:.2}", "Ani", 91.456))
wok(str::pad_left("7", 3, "0"))
```
```keluaran
Ani scored 91.46
007
```

## Arrays

An array holds values in order. A negative index counts from the end, and an index out of range is always an
error.

```jrg array.jrg
var a = [10, 20, 30]
a[0] = 11
a = a.push(40)
wok(a, a[-1], a[1:3], len(a))
wok(a.map(|x| x * 2), a.filter(|x| x > 20), arr::sum(a))
wok(a.contains(20), a.first(), [].first())
```
```keluaran
[11, 20, 30, 40] 40 [20, 30] 4
[22, 40, 60, 80] [30, 40] 101
true Option::Some(11) Option::None
```

Methods that "change" an array, like `push`, return a new array; assign the result back (`a = a.push(x)`). That
form does not copy the whole array, so adding elements one at a time stays fast.

Anything that may find nothing (`first`, `last`, `get`, `find`, `pop`) returns an `Option`, never `nil` or `-1`. See
[Option and Result](galat.html).

## Tuples

A tuple groups several values of different types. Its elements are read with `.0`, `.1`, or `[i]`, and it can be
unpacked into several names at once.

```jrg tuple.jrg
fn divide(a, b) { return (a / b, a % b) }

let t = (1, "two", 3.0)
wok(t, t.0, t[1])
quotient, remainder := divide(17, 5)
wok(quotient, remainder)
```
```keluaran
(1, two, 3.0) 1 two
3 2
```

## Dicts

A dict maps keys to values. Keys are stored as text, so `d[1]` and `d["1"]` point at the same place. It prints in
key order.

```jrg dict.jrg
var stock = {"apple": 3, "orange": 5}
stock["mango"] = 2
wok(stock, stock["apple"], stock.len())
wok(dict::get(stock, "durian"), dict::get_or(stock, "durian", 0))
for k, v in stock {
    wok(k, v)
}
```
```keluaran
dict {apple: 3, mango: 2, orange: 5} 3 3
Option::None 0
apple 3
mango 2
orange 5
```

Reading a missing key through `d[k]` is an error; use `dict::get` (which returns an `Option`) or `get_or`.

## Conversions

`as` converts between basic types. A conversion with no exact answer is refused, never silently truncated.

```jrg convert.jrg
wok(3 as float, 3.9 as int, 'A' as int, 66 as rune)
wok(int("42") + 1, float("2.5"), str(42) + "!", "17".to_int())
wok(typeof(1), typeof(1.5), typeof("a"), typeof('a'), typeof([1]), typeof({"a": 1}))
```
```keluaran
3.0 3 65 B
43 2.5 42! 17
int float string rune array dict
```

`300 as u8`, `-1 as uint`, and `int("abc")` are errors. To take the lowest 8 bits, write `x & 255`.

## Empty values

`nil` exists, but it is rarely needed. A built-in function that may find nothing answers `Option::None`, and
`var x: int` with no initial value gets the zero value of its type (`0`, `0.0`, `""`, `false`, `[]`).
