---
judul: Patterns and match
menu: Patterns
ringkas: Taking values apart and choosing a branch by their shape.
---
## match

`match` tries each arm from the top and runs the first one that matches. `_` matches anything.

```jrg match.jrg
fn describe(x) {
    return match x {
        0 => "zero",
        n @ 1..=9 => "one digit: " + str(n),
        n if n < 0 => "negative",
        _ => "big",
    }
}
wok(describe(0), describe(7), describe(-3), describe(42))
```
```keluaran
zero one digit: 7 negative big
```

| Pattern | Matches |
|---|---|
| `0`, `"teks"`, `'a'`, `true` | exactly that value |
| `1..=9`, `'a'..='z'` | a value inside the range |
| `a \| b` | either of them |
| `n` | anything, binding it to `n` |
| `n @ pattern` | `pattern`, binding the whole value to `n` |
| `pattern if condition` | `pattern`, when `condition` is true |
| `(a, b)` | a two-element tuple |
| `[a, b]`, `[]`, `[head, ...rest]`, `[a, ..., z]` | an array of that shape |
| `Point { x, y: 0 }` | a struct whose field `y` is 0 |
| `Shape::Circle(r)`, `Some(v)`, `Err(e)` | an enum variant, unpacking its contents |

## Array patterns

```jrg array-patterns.jrg
fn shape(a) {
    return match a {
        [] => "empty",
        [x] => "one: " + str(x),
        [first, ..., last] => str(first) + " to " + str(last),
    }
}
wok(shape([]))
wok(shape([5]))
wok(shape([1, 2, 3, 4]))
```
```keluaran
empty
one: 5
1 to 4
```

## Patterns in let and for

Patterns also work in `let`, in a `for` header, and in function parameters.

```jrg unpack.jrg
let (a, b) = (1, 2)
let [p, q, ...rest] = [10, 20, 30, 40]
wok(a, b, p, q, rest)

let scores = [("Ani", 90), ("Budi", 75)]
for _, (name, score) in scores {
    wok(name, score >= 80 $ "A" : "B")
}

struct Point { pub x: int, pub y: int }
let Point { x, y } = Point { x: 3, y: 4 }
wok(x * y)
```
```keluaran
1 2 10 20 [30, 40]
Ani A
Budi B
12
```

## let … else

When the pattern might not match, `let pattern = value else { … }` runs the `else` block. That block must leave the
flow (`return`, `break`, or `continue`), so after that line the names are guaranteed to be bound.

```jrg let-else.jrg
fn price(menu, name) {
    let Some(p) = dict::get(menu, name) else {
        return "no " + name
    }
    return name + " $" + str(p)
}

let menu = {"noodles": 6, "soup": 5}
wok(price(menu, "soup"))
wok(price(menu, "pizza"))
```
```keluaran
soup $5
no pizza
```

## ??

`a ?? b` opens an `Option`: its contents when `Some`, `b` when `None`. The right side is only evaluated when needed.

```jrg fallback.jrg
let colors = {"sky": "blue"}
wok(dict::get(colors, "sky") ?? "?", dict::get(colors, "grass") ?? "?")
wok([3, 9, 4].find(|x| x > 5) ?? 0, [1, 2].find(|x| x > 5) ?? 0)
```
```keluaran
blue ?
9 0
```

## Bare variants

In the file that declares an enum, its variants may be written without the type name, as long as no other enum in
that file has a variant with the same name.

```jrg bare.jrg
enum Light { Red, Yellow, Green }

fn may_go(l) {
    return match l {
        Green => true,
        _ => false,
    }
}
wok(may_go(Green), may_go(Light::Red))
```
```keluaran
true false
```
