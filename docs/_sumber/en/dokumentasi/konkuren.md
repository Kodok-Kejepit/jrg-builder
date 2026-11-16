---
judul: Concurrency
ringkas: Goroutines, channels, select, Futures, and synchronisation tools.
---
JRG's concurrency model is like Go's: do work in a goroutine, then communicate through channels. Each goroutine is a
real operating-system thread, so it suits tens to hundreds of tasks, not millions.

## spawn and channels

`spawn { … }` (or `go f(x)`) runs a block in a new goroutine. `make_chan(type, capacity)` makes a channel; `c <- v`
sends, `<-c` receives.

```jrg channel.jrg lokal
fn berat(n) { return n * n }

let c = make_chan(int, 2)
spawn { c <- berat(3) }
spawn { c <- berat(4) }
wok(<-c + <-c)
```
```keluaran
25
```

Variables used inside `spawn` are copied when the goroutine is created. The program waits for every goroutine to
finish before it ends, so no output is lost because `main` finished first.

## select

`select` waits on several channels at once and takes the first one ready. A `default` arm makes it not wait.

```jrg select.jrg lokal
let hasil = make_chan(string, 0)
let selesai = make_chan(bool, 0)

spawn {
    for _, kata in ["satu", "dua", "tiga"] {
        hasil <- kata
    }
    selesai <- true
}

var jalan = true
while jalan {
    select {
        case k := <-hasil: { wok("terima", k) }
        case _ := <-selesai: { jalan = false }
    }
}
```
```keluaran
terima satu
terima dua
terima tiga
```

## Future and Task

For "start now, collect the result later", use `Future::new(function, arguments…)` and then `future::await`.

```jrg future.jrg lokal
fn hitung(n) {
    var t = 0
    for _, i in 0..n { t += i }
    return t
}

let a = Future::new(hitung, 1000)
let b = Future::new(hitung, 2000)
wok(future::await(a) + future::await(b))
```
```keluaran
2498500
```

A Future's result can only be collected once; collecting it again is an error. Others in the family:
`future::all`, `future::race`, `future::timeout`, `future::try_get`, `future::is_done`, `Task::spawn`, and
`task::join`. If the function inside a Future fails, the error shows up where the result is collected.

## Atomic and Mutex

Ordinary values never share, so for state shared between goroutines JRG has two handles: `Atomic` (an integer that
is safe to add to from many threads) and `Mutex` (a lock).

```jrg atomic.jrg lokal
let n = Atomic::new(0)
var tugas = []
for _, i in 0..4 {
    tugas = tugas.push(Task::spawn(|| {
        for _, j in 0..1000 { n.add(1) }
    }))
}
for _, t in tugas { task::join(t) }
wok(n.get())

let kunci = Mutex::new()
mutex::with_lock(kunci, || wok("di dalam kunci"))
```
```keluaran
4000
di dalam kunci
```

Copying an `Atomic` or a `Mutex` copies the handle, so every copy points at the same state. `with_lock` releases the
lock itself, including when the function fails.

## What is not there

JRG has no `async`/`await`. Its goroutines are real threads, not lightweight tasks, and `async` syntax would promise a
cost it does not have. To wait on many things at once, use channels, `select`, or `future::all`.
