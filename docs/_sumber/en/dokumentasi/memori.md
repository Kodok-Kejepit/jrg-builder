---
judul: Memory model
ringkas: Values never share storage, and a copy is only made when someone writes.
---
## Values never share

Assigning, passing to a function, returning, or reading from a collection always gives a value that stands on its
own. Changing a copy never changes the original.

```jrg copies.jrg
var a = [1, 2, 3]
var b = a
b[0] = 99

fn change(xs) {
    var ys = xs
    ys[1] = 0
    return ys
}

let c = change(a)
wok(a, b, c)
```
```keluaran
[1, 2, 3] [99, 2, 3] [1, 0, 3]
```

Because every value has exactly one owner, it lives exactly as long as the variable that owns it. JRG needs no
garbage collector, and ordinary values have no dangling pointers.

## Copying is cheap

The real copy is only made when someone writes. `var b = a` copies nothing; `b[0] = 99` is what copies. As a
result:

- passing a large array to a function that only reads it copies nothing;
- `a = a.push(x)`, `a = a + [x]`, and `d = dict::set(d, k, v)` change in place, so adding elements one at a time
  stays linear;
- `a[i] = v` does not depend on the size of `a`.

What really is expensive: adding at the front (`a = [x] + a`), because it moves every element. For a double-ended
queue, use `std/deque`.

## What shares on purpose

A few things are designed to share, and all of them are plain to see in the code:

| What | Example |
|---|---|
| variables captured by a closure | `let naik = \|\| { n += 1 }` changes the outer `n` |
| synchronisation handles | `Atomic`, `Mutex`, channels |
| explicit handles | `ptr::new(value)` |
| manual allocation | `jawa::alloc` and `jawir::free` |
| objects from other languages | `py::`, `js::`, `jvm::` handles |

```jrg handles.jrg
let p = ptr::new(0)
let q = p
ptr::set(q, 42)
wok(ptr::get(p))

let m = jawa::alloc(7)
jawir::free(m)
wok(jawa::is_valid(m))
```
```keluaran
42
false
```

`jawa::` imitates malloc/free without raw pointers: use after `free`, or `free` twice, gives a clear error message,
not a crash.

## One known leak

A closure that calls itself through a variable it captured forms an ownership cycle that is never freed. For
recursion, use a named `fn`.
