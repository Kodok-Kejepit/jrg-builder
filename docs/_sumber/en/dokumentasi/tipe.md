---
judul: Structs, enums, and traits
menu: Structs, enums, traits
ringkas: Making your own types, giving them methods, and sharing behaviour through traits.
---
## Struct

A struct groups named data. Fields marked `pub` can be read from outside; the rest are only visible from that type's
methods (and those of its descendants). A field may have a default, and a field without one must be filled in.

```jrg struct.jrg
struct Account {
    pub name: string,
    pub balance: int = 0,
    pin: string = "0000",
}

impl Account {
    fn new(name) -> Account {
        return Account { name: name }
    }

    fn deposit(self, n) -> Account {
        var a = self
        a.balance += n
        return a
    }

    fn check_pin(self, p) -> bool {
        return self.pin == p
    }
}

var a = Account::new("Ani")
a = a.deposit(50)
wok(a.name, a.balance, a.check_pin("1234"))
```
```keluaran
Ani 50 false
```

A function in an `impl` without `self` is called through the type name (`Account::new`). One that takes `self` is called
through a value (`a.deposit(50)`).

Inside a method, `self` is a **copy**. Changing `self` does not change the caller's value, so a method that changes
something returns a new value and the caller reassigns it: `a = a.deposit(50)`. That form does not copy large data; see
[Memory model](memori.html).

### Building from another value

`...` fills the fields that are not written from another value. It must come last.

```jrg spread.jrg
struct Point { pub x: int, pub y: int, pub z: int }

let start = Point { x: 1, y: 2, z: 3 }
let moved = Point { x: 10, ...start }
wok(moved)
```
```keluaran
Point {x: 10, y: 2, z: 3}
```

### Inheritance

A struct may inherit from one other struct. The parent's fields and methods come along.

```jrg inherit.jrg
struct Animal { pub name: string }

impl Animal {
    fn intro(self) -> string { return "I am " + self.name }
}

struct Cat: Animal { pub color: string }

let c = Cat { name: "Oyen", color: "orange" }
wok(c.intro(), c.color)
```
```keluaran
I am Oyen orange
```

## Enum

An enum is a type whose value is one of several variants. A variant may carry data, positional or named.

```jrg enum.jrg
enum Shape {
    Circle(float),
    Square(float),
    Triangle { base: float, height: float },
}

fn area(s) {
    return match s {
        Shape::Circle(r) => 3.14 * r * r,
        Shape::Square(side) => side * side,
        Shape::Triangle { base, height } => base * height / 2.0,
    }
}

wok(area(Shape::Circle(1.0)))
wok(area(Shape::Triangle { base: 4.0, height: 3.0 }))

enum Direction { North, East, South, West }
wok(Direction::North, Direction::North == Direction::West)
```
```keluaran
3.14
6.0
Direction::North false
```

`Option` and `Result` are ordinary enums that come built in; both are covered in [Option and Result](galat.html).

## Trait

A trait lists the methods a type must have. A method without a body must be provided by every `impl`; a method with
a body becomes a default that may be overridden.

```jrg trait.jrg
trait Speaks {
    fn sound(self) -> string
    fn greet(self) -> string = self.name + " says " + self.sound()
}

struct Dog { pub name: string }
struct Duck { pub name: string }

impl Speaks for Dog {
    fn sound(self) -> string { return "woof" }
}

impl Speaks for Duck {
    fn sound(self) -> string { return "quack" }
    fn greet(self) -> string { return self.name + " stays quiet" }
}

wok(Dog { name: "Doggo" }.greet())
wok(Duck { name: "Donald" }.greet())
```
```keluaran
Doggo says woof
Donald stays quiet
```

An `impl` that forgets a required method is refused before the program runs.

## Generics

Functions and structs may have type parameters. Because JRG is dynamically typed, type parameters and their trait
bounds (`<T: Speaks>`) are documentation: the names are checked, the constraint is not enforced.

```jrg generics.jrg
fn first<T>(xs: []T) -> T {
    return xs[0]
}

struct Box<T> { pub item: T }

wok(first([3, 4]), first(["a"]), Box { item: 5.5 }.item)
```
```keluaran
3 a 5.5
```

## Your own operators and display

A type can give meaning to operators through specially named methods, and decide how it is printed through
`to_string`.

```jrg operator.jrg
struct Money { pub cents: int }

impl Money {
    fn add(self, o) -> Money { return Money { cents: self.cents + o.cents } }
    fn cmp(self, o) -> int { return self.cents - o.cents }
    fn to_string(self) -> string { return "$" + str(self.cents / 100) + "." + str::pad_left(str(self.cents % 100), 2, "0") }
}

let coffee = Money { cents: 450 }
let bread = Money { cents: 325 }
wok(coffee + bread, coffee > bread)
wok([coffee, bread], """total ${coffee + bread}""")
```
```keluaran
$7.75 true
[$4.50, $3.25] total $7.75
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
