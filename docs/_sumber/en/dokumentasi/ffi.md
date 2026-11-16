---
judul: Other languages (FFI)
menu: Other languages
ringkas: Calling C, C++, Rust, Go, Python, JavaScript, PHP, Lua, Java, Kotlin, .NET, and WebAssembly from JRG.
---
Every door to another language answers a `Result`: the library might not be installed, and that is an ordinary
failure to handle.

## C, Rust, and Go

Declare the signature in an `extern "C"` block, then call it like an ordinary function. The library name may be
given.

```jrg c.jrg lokal
extern "C" {
    fn abs(n: i32) -> i32
    fn strlen(s: string) -> usize
}

wok(abs(-17), strlen("hello"))
```
```keluaran
Result::Ok(17) Result::Ok(5)
```

```jrg-cuplikan
extern "C" "libm.so.6" {
    fn pow(base: f64, exponent: f64) -> f64
}
extern "C" "./libcalc.so" {
    fn process(data: *mut i32, n: i32) -> i32
}
```

Rust (`#[no_mangle] pub extern "C" fn`) and Go (`-buildmode=c-shared` with `//export`) produce plain C symbols, so
they are called the same way.

## C++

`extern "C++"` computes the C++ symbol name (*mangling*) from the signature itself, so a C++ library can be called
without a wrapper. A function inside a namespace is written with its path.

```jrg-cuplikan
extern "C++" "./libmine.so" {
    fn add(a: i32, b: i32) -> i32
    fn math::mul(a: i32, b: i32) -> i32
}
```

## Python

`py::` runs Python in a separate process and talks to it, so any library that can be `import`ed can be used: numpy,
pandas, requests, PIL, and so on.

```jrg-cuplikan
fn times10(x) { return x * 10 }

wok(py::eval("sum(range(10))"))
wok(py::call("statistics", "median", [3, 1, 4, 1, 5]))
wok(py::call("builtins", "list", py::call("builtins", "map", times10, [1, 2, 3])?))

let set = py::eval("{1, 2}")?
wok(typeof(set), py::method(set, "issubset", [1, 2, 3]))
```

```sh
Result::Ok(45)
Result::Ok(3)
Result::Ok([10, 20, 30])
py:set Result::Ok(true)
```

- Values that have a counterpart (numbers, text, lists, dicts) arrive as ordinary JRG values. Those without one (sets,
  objects, DataFrames) arrive as a **handle**: its methods can be called (`df.head(5)`), it can be passed back to
  Python, and it is released by itself when no longer used.
- `py::ref(code)` asks for a handle even when the value could be converted, so large data stays on the Python side.
- JRG functions may be callbacks (`map`, `sorted(key=…)`, `df.apply`), from other Python threads too.
- Named arguments go through `py::call_kw` and `py::method_kw`.

## JavaScript, PHP, and Lua

All three use the same machinery as `py::`, so the rules are the same: a value or a handle, JRG functions as
callbacks, and a `Result` on every call.

```jrg-cuplikan
wok(js::eval("[1,2,3].map(x => x * 2)"))
wok(js::call("path", "join", "a", "b"))
wok(lua::eval("1 + 2, 'three'"), lua::call("string.upper", "lua"))
wok(php::call("array_map", |x| x * 10, [1, 2, 3]))
```

```sh
Result::Ok([2, 4, 6])
Result::Ok(a/b)
Result::Ok([3, three]) Result::Ok(LUA)
Result::Ok([10, 20, 30])
```

- `js::` needs Node. A `Promise` is awaited automatically, so `fetch` and async libraries work as they are.
- `php::` needs PHP 8. A PHP warning becomes an `Err`, `echo` goes to stderr, and Composer libraries are loaded with
  `php::require("vendor/autoload.php")`.
- `lua::` needs Lua 5.3 or newer.

## Java and Kotlin

`jvm::` loads the JVM from the installed JDK through JNI. Method signatures are written as JNI descriptors, exactly as
`javap -s` prints them.

```jrg-cuplikan
jvm::mulai(["-Djava.class.path=./classes"])
wok(jvm::panggil_statis("java.lang.Math", "abs", "(I)I", [-17]))

let sb = jvm::baru("java.lang.StringBuilder", "()V")?
jvm::panggil(sb, "append", "(Ljava/lang/String;)Ljava/lang/StringBuilder;", ["hello"])?
wok(jvm::ke_teks(sb))
jvm::lepas(sb)
```

`kotlin::` translates Kotlin's naming rules to the JVM: top-level functions (`kotlin::fungsi("Calc", "add", …)`),
`object` (`kotlin::objek`), properties (`kotlin::properti`), and companions.

## .NET

```jrg-cuplikan
dotnet::mulai("./App.runtimeconfig.json")
let add = dotnet::fungsi("./App.dll", "JrgTest, App", "Add", ["i32", "i32"], "i32")?
wok(dotnet::panggil(add, [2, 3]))
```

The method being called must be marked `[UnmanagedCallersOnly]`; after that it is treated like a C function.

## WebAssembly

`wasm::` runs a `.wasm` module with an interpreter built into JRG itself. The module can only touch its own memory,
so it suits running outside code that is not fully trusted. `wasm::batas_langkah(m, n)` caps the number of steps, so
a module that never stops is halted with an error.

```jrg-cuplikan
let m = wasm::muat("module.wasm")?
wok(wasm::ekspor(m))
wok(wasm::panggil(m, "fib", [20]))
wasm::tulis_memori(m, 200, "hello")
wok(wasm::baca_memori(m, 200, 4))
```

WASM features not yet supported (imports, `call_indirect`, SIMD) are refused at load time, naming the feature. This
is a way to run *someone else's* WebAssembly from JRG; compiling JRG programs *to* WebAssembly is the fifth engine,
still being built (see [jrgc](jrgc.html)).
