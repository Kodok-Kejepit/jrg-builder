---
judul: Functions and closures
ringkas: Declaring functions, flexible parameters, functions as values, and closures.
---
## Declaring functions

```jrg functions.jrg
fn area(w: int, h: int) -> int {
    return w * h
}

fn perimeter(w, h) {
    return 2 * (w + h)
}

fn double(x) = x * 2

wok(area(3, 4), perimeter(3, 4), double(5))
```
```keluaran
12 14 10
```

Type annotations on parameters and the return value may be written or left out. `fn f(x) = expression` is the short
form for a function that only returns one expression. A function may be called before its line, and a function may be
declared inside another function (but it does not see the outer function's locals; use a closure for that).

## Parameters

```jrg parameters.jrg
fn greet(name, greeting = "hello") {
    return greeting + ", " + name
}

fn total(numbers...) {
    return arr::sum(numbers)
}

fn divide(numerator, denominator) {
    return numerator / denominator
}

wok(greet("Ani"), greet("Budi", "good morning"))
wok(total(), total(1, 2, 3))
wok(total(...[4, 5, 6]))
wok(divide(denominator: 2, numerator: 10))
```
```keluaran
hello, Ani good morning, Budi
0 6
15
5
```

- **Defaults** (`greeting = "hello"`) are evaluated at call time, and may use earlier parameters.
- **A collector** (`numbers...`) takes the remaining arguments as an array; with none left it is `[]`.
- **Spread** (`...arr`) is the reverse: the array's elements become separate arguments.
- **Named arguments** (`denominator: 2`) may come in any order and mix with positional ones.

A parameter may be a pattern: `fn distance((x1, y1), (x2, y2))` unpacks two tuples right in the function head.

## Returning several values

Return a tuple, then unpack it at the call site:

```jrg returns.jrg
fn stats(xs) {
    return (arr::min(xs), arr::max(xs), arr::sum(xs) / len(xs))
}

lowest, highest, average := stats([4, 8, 15, 16, 23, 42])
wok(lowest, highest, average)
```
```keluaran
4 42 18
```

## Closure

A closure is a nameless function written in place: `|x| x * x`, or `|a, b| { … }` when the body is more than one
expression.

```jrg closure.jrg
let square = |x| x * x
let values = [1, 2, 3, 4, 5]
wok(values.map(square))
wok(values.filter(|x| x % 2 == 1))
wok(values.reduce(|a, b| a + b, 0))
wok(values.map(|x| x * 10).filter(|x| x > 20))
```
```keluaran
[1, 4, 9, 16, 25]
[1, 3, 5]
15
[30, 40, 50]
```

A closure **remembers the variable**, not a copy of its value. A change inside is visible outside, and the other
way round:

```jrg capture.jrg
var count = 0
let bump = || { count += 1 }
bump()
bump()
wok(count)

fn make_adder(n) {
    return |x| x + n
}
let add10 = make_adder(10)
wok(add10(5))
```
```keluaran
2
15
```

If you want the closure to take a snapshot of the value when it is created, write `move`:

```jrg move.jrg
var n = 1
let look = || n
let snapshot = move || n
n = 9
wok(look(), snapshot())
```
```keluaran
9 1
```

## Functions as values

Named functions, closures, built-in functions, and enum variant constructors are all values that can be stored and
passed around.

```jrg fn-values.jrg
fn double(x) = x * 2

let steps = [double, |x| x + 1, math::abs]
var v = -5
for _, f in steps {
    v = f(v)
}
wok(v)
wok([3, -1, 2].map(math::abs), ["a", "b"].map(str::upper))
wok(4 |> double |> double)
wok(terap(math::max, [3, 9]))
```
```keluaran
9
[3, 1, 2] [A, B]
16
9
```

`x |> f` is the same as `f(x)`, useful for chaining steps from left to right. `terap(f, args)` calls `f` with the
array's elements as its arguments.

Two functions cannot be compared with `==`; JRG refuses, because there is no correct answer to "are these two
functions the same".

## Recursion

```jrg recursion.jrg
fn fib(n) {
    return n < 2 $ n : fib(n - 1) + fib(n - 2)
}
wok(fib(20))
```
```keluaran
6765
```

Recursion depth is limited by the stack size: about 20 thousand levels on the interpreter, up to a million on `--vm`,
and hundreds of thousands in an `--emit-llvm` binary. Going past it gives an error that names the function, not a
crash. For heavy recursion, use a named `fn`, not a closure that calls itself.
