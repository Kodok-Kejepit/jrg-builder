---
judul: Bahasa lain (FFI)
menu: Bahasa lain
ringkas: Memanggil C, C++, Rust, Go, Python, JavaScript, PHP, Lua, Java, Kotlin, .NET, dan WebAssembly dari JRG.
---
Semua pintu ke bahasa lain menjawab `Result`: pustakanya bisa saja tidak terpasang, dan itu kegagalan yang wajar
ditangani.

## C, Rust, dan Go

Deklarasikan tanda tangannya di blok `extern "C"`, lalu panggil seperti fungsi biasa. Nama pustaka boleh disebut.

```jrg c.jrg lokal
extern "C" {
    fn abs(n: i32) -> i32
    fn strlen(s: string) -> usize
}

wok(abs(-17), strlen("halo"))
```
```keluaran
Result::Ok(17) Result::Ok(4)
```

```jrg-cuplikan
extern "C" "libm.so.6" {
    fn pow(basis: f64, pangkat: f64) -> f64
}
extern "C" "./libhitung.so" {
    fn proses(data: *mut i32, n: i32) -> i32
}
```

Rust (`#[no_mangle] pub extern "C" fn`) dan Go (`-buildmode=c-shared` dengan `//export`) menghasilkan simbol C biasa,
jadi dipanggil dengan cara yang sama.

## C++

`extern "C++"` menghitung sendiri nama simbol C++ (*mangling*) dari tanda tangannya, jadi pustaka C++ bisa dipanggil
tanpa membungkusnya. Fungsi di dalam namespace ditulis dengan jalurnya.

```jrg-cuplikan
extern "C++" "./libmilik.so" {
    fn add(a: i32, b: i32) -> i32
    fn math::mul(a: i32, b: i32) -> i32
}
```

## Python

`py::` menjalankan Python di proses terpisah dan berbicara dengannya, jadi pustaka apa pun yang bisa di-`import`
bisa dipakai: numpy, pandas, requests, PIL, dan seterusnya.

```jrg-cuplikan
fn kali10(x) { return x * 10 }

wok(py::eval("sum(range(10))"))
wok(py::call("statistics", "median", [3, 1, 4, 1, 5]))
wok(py::call("builtins", "list", py::call("builtins", "map", kali10, [1, 2, 3])?))

let himpunan = py::eval("{1, 2}")?
wok(typeof(himpunan), py::method(himpunan, "issubset", [1, 2, 3]))
```

```sh
Result::Ok(45)
Result::Ok(3)
Result::Ok([10, 20, 30])
py:set Result::Ok(true)
```

- Nilai yang punya padanan (angka, teks, list, dict) datang sebagai nilai JRG biasa. Yang tidak punya (set, objek,
  DataFrame) datang sebagai **handle**: method-nya bisa dipanggil (`df.head(5)`), ia bisa dioper lagi ke Python, dan
  dilepas sendiri saat tidak dipakai.
- `py::ref(kode)` meminta handle walau nilainya bisa dikonversi, supaya data besar tetap di sisi Python.
- Fungsi JRG boleh jadi callback (`map`, `sorted(key=…)`, `df.apply`), juga dari thread Python lain.
- Argumen bernama lewat `py::call_kw` dan `py::method_kw`.

## JavaScript, PHP, dan Lua

Tiga-tiganya memakai mesin yang sama dengan `py::`, jadi aturannya sama: nilai atau handle, fungsi JRG sebagai
callback, dan `Result` di setiap panggilan.

```jrg-cuplikan
wok(js::eval("[1,2,3].map(x => x * 2)"))
wok(js::call("path", "join", "a", "b"))
wok(lua::eval("1 + 2, 'tiga'"), lua::call("string.upper", "lua"))
wok(php::call("array_map", |x| x * 10, [1, 2, 3]))
```

```sh
Result::Ok([2, 4, 6])
Result::Ok(a/b)
Result::Ok([3, tiga]) Result::Ok(LUA)
Result::Ok([10, 20, 30])
```

- `js::` butuh Node. `Promise` ditunggu otomatis, jadi `fetch` dan pustaka async bisa dipakai apa adanya.
- `php::` butuh PHP 8. Peringatan PHP menjadi `Err`, `echo` dialihkan ke stderr, dan pustaka Composer dimuat dengan
  `php::require("vendor/autoload.php")`.
- `lua::` butuh Lua 5.3 ke atas.

## Java dan Kotlin

`jvm::` memuat JVM dari JDK yang terpasang lewat JNI. Tanda tangan metode ditulis sebagai deskriptor JNI, persis
seperti yang dicetak `javap -s`.

```jrg-cuplikan
jvm::mulai(["-Djava.class.path=./kelas"])
wok(jvm::panggil_statis("java.lang.Math", "abs", "(I)I", [-17]))

let sb = jvm::baru("java.lang.StringBuilder", "()V")?
jvm::panggil(sb, "append", "(Ljava/lang/String;)Ljava/lang/StringBuilder;", ["halo"])?
wok(jvm::ke_teks(sb))
jvm::lepas(sb)
```

`kotlin::` menerjemahkan aturan nama Kotlin ke JVM: fungsi tingkat atas (`kotlin::fungsi("Hitung", "tambah", …)`),
`object` (`kotlin::objek`), properti (`kotlin::properti`), dan companion.

## .NET

```jrg-cuplikan
dotnet::mulai("./App.runtimeconfig.json")
let tambah = dotnet::fungsi("./App.dll", "UjiJrg, App", "Tambah", ["i32", "i32"], "i32")?
wok(dotnet::panggil(tambah, [2, 3]))
```

Metode yang dipanggil harus bertanda `[UnmanagedCallersOnly]`; sesudah itu ia diperlakukan seperti fungsi C.

## WebAssembly

`wasm::` menjalankan modul `.wasm` dengan interpreter yang ada di dalam JRG sendiri. Modulnya cuma bisa menyentuh
memorinya sendiri, jadi cocok untuk menjalankan kode dari luar yang tidak sepenuhnya dipercaya.

```jrg-cuplikan
let m = wasm::muat("modul.wasm")?
wok(wasm::ekspor(m))
wok(wasm::panggil(m, "fib", [20]))
wasm::tulis_memori(m, 200, "halo")
wok(wasm::baca_memori(m, 200, 4))
```

Fitur WASM yang belum didukung (impor, `call_indirect`, SIMD) ditolak saat dimuat, dengan nama fiturnya.
