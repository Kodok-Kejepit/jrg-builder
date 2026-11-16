---
judul: Control flow
ringkas: Branches, loops, and how to leave both.
---
## if and else

The condition has no parentheses, and the body always has braces. `if` can also be used as a value.

```jrg if.jrg
let n = 7
if n % 2 == 0 {
    wok("even")
} else if n > 5 {
    wok("odd and big")
} else {
    wok("odd")
}

let size = if n > 5 { "big" } else { "small" }
wok(size)
```
```keluaran
odd and big
big
```

## Ternary

The short form is `condition $ if_true : if_false`. It opens with `$`, not `?`, because `?` is used to propagate
errors (see [Option and Result](galat.html)).

```jrg ternary.jrg
let score = 72
wok(score >= 75 $ "pass" : "retake")
```
```keluaran
retake
```

## for

`for` has two forms, and the difference matters:

- `for i, x in collection`: `i` is the index, `x` the value. Use `_` for the one you do not need.
- `for i in collection`: **a single name always means the index**, ranges included.

```jrg for.jrg
let fruit = ["apple", "orange"]
for i, f in fruit {
    wok(i, f)
}

var values = []
for _, number in 5..8 {
    values = values.push(number)
}
wok(values)

var indices = []
for i in 5..8 {
    indices = indices.push(i)
}
wok(indices)
```
```keluaran
0 apple
1 orange
[5, 6, 7]
[0, 1, 2]
```

The last line holds `[0, 1, 2]`, not `[5, 6, 7]`: with one name, `i` is the 0th, 1st, and 2nd index of that
range. If you want the values, write `for _, i in 5..8`.

`a..b` excludes `b`, `a..=b` includes it. The two-name form also works on text (index and character) and dicts (key
and value). A C-style form exists too:

```jrg for-c.jrg
for i := 0; i < 10; i += 3 {
    wok(i)
}
```
```keluaran
0
3
6
9
```

A range in a `for` header is never turned into an array, so `for _, i in 0..1000000000` uses no memory.

## while and loop

```jrg while.jrg
var n = 27
var steps = 0
while n != 1 {
    n = n % 2 == 0 $ n / 2 : 3 * n + 1
    steps += 1
}
wok("Collatz 27:", steps, "steps")

var guess = 0
loop {
    guess += 1
    if guess * guess > 50 { break }
}
wok(guess)
```
```keluaran
Collatz 27: 111 steps
8
```

## Labeled break and continue

`break` and `continue` apply to the nearest loop. To leave an outer loop, give it a label that starts with a single
quote.

```jrg label.jrg
'search: for _, i in 1..10 {
    for _, j in 1..10 {
        if i * j == 42 {
            wok("found:", i, j)
            break 'search
        }
    }
}
```
```keluaran
found: 6 7
```

## match

`match` matches a value against patterns and runs the first arm that matches. It is also an expression.

```jrg match.jrg
fn day(n) {
    return match n {
        0 | 6 => "weekend",
        1..=5 => "weekday",
        _ => "not a day",
    }
}
wok(day(0), day(3), day(9))
```
```keluaran
weekend weekday not a day
```

The full set of patterns (tuples, arrays, structs, enums, `if` guards, `@` bindings) is in [Patterns](pola.html).

## defer

`defer` delays a call until the function finishes, including when the function stops because of an error. The
last one registered runs first, and its arguments are frozen when the `defer` line is read.

```jrg defer.jrg
fn work() {
    var file = "report.txt"
    defer wok("close", file)
    defer wok("save")
    file = "other.txt"
    wok("working with", file)
}
work()
```
```keluaran
working with other.txt
save
close report.txt
```
