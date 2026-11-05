---
judul: Web dan data
ringkas: Server HTTP, template, cookie dan sesi, WebSocket, klien HTTP, dan SQLite.
---
Semua yang dibutuhkan aplikasi web ada di namespace `dok::` dan `http::`, tanpa pustaka tambahan.

## Server

```jrg-cuplikan server.jrg
package main

pub fn main() {
    let r = dok::router_new()

    dok::router_get(r, "/", |req| dok::html("<h1>Halo dari JRG</h1>"))
    dok::router_get(r, "/sapa/:nama", |req| dok::text("halo, " + req.params.nama))
    dok::router_get(r, "/api/kuadrat", |req| {
        let n = int(dok::query_get(req.query, "n") ?? "0")
        return dok::json_response({"n": n, "kuadrat": n * n})
    })
    dok::router_post(r, "/gema", |req| dok::text(req.body))
    dok::router_static(r, "/aset", "./publik")

    dok::serve(r, 8080)
}
```

```sh
$ curl localhost:8080/sapa/Ani
halo, Ani
$ curl "localhost:8080/api/kuadrat?n=12"
{"kuadrat":144,"n":12}
```

`dok::serve` memblokir dan menangani tiap koneksi di thread-nya sendiri. Argumen ketiga mengatur alamat (bawaannya
`0.0.0.0`; pakai `"127.0.0.1"` supaya cuma bisa diakses dari mesin sendiri).

| Request (`req`) | Isi |
|---|---|
| `method`, `path` | `"GET"`, `"/sapa/Ani"` |
| `params` | bagian `:nama` dari pola rute, dibaca `req.params.nama` |
| `query` | pasangan dari `?a=1&b=2`; ambil dengan `dok::query_get(req.query, "a")` |
| `headers` | ambil dengan `dok::header_get(req.headers, "Content-Type")` |
| `body`, `body_bytes` | badan sebagai teks; kalau bukan teks UTF-8, `body` bernilai `nil` dan isinya di `body_bytes` |

Handler boleh mengembalikan teks (jadi `200 text/plain`), hasil `dok::text`, `dok::html`, `dok::json_response`,
`dok::redirect`, `dok::response(kode, isi)`, `dok::binary(byte)`, atau `Result` (`Err` jadi 500).

### Middleware

`dok::router_use` berjalan sebelum rute dicari; mengembalikan `nil` berarti lanjut, mengembalikan respons berarti
berhenti di situ. `dok::router_after` berjalan sesudah handler dan boleh mengubah responsnya.

```jrg-cuplikan
dok::router_use(r, |req| {
    if str::starts_with(req.path, "/admin") && dok::cookie_get(req, "sesi").is_none() {
        return dok::response(401, "perlu masuk dulu")
    }
    return nil
})
dok::router_after(r, |req, resp| dok::header_set(resp, "X-Frame-Options", "DENY"))
```

## Template

`dok::render` mengisi `{{ nama }}` dan meloloskan HTML-nya, jadi isian dari pengguna aman ditampilkan. `{{& nama }}`
menyisipkan apa adanya. Ada juga `{{#if}}…{{else}}…{{/if}}` dan `{{#each}}…{{/each}}`.

```jrg template.jrg
let t = "<h1>Halo {{nama}}</h1><ul>{{#each item}}<li>{{@index}}. {{.}}</li>{{/each}}</ul>"
match dok::render(t, {"nama": "<Ani>", "item": ["kopi", "teh"]}) {
    Ok(html) => wok(html),
    Err(e) => wok("gagal:", e),
}
```
```keluaran
<h1>Halo &lt;Ani&gt;</h1><ul><li>0. kopi</li><li>1. teh</li></ul>
```

Pelolosan HTML cuma melindungi teks dan nilai atribut berkutip, jadi `dok::render` membaca di mana sebuah `{{ nama }}`
duduk. Nilai di atribut URL (`href`, `src`, `action`, …) yang berskema selain `http`, `https`, `mailto`, `tel` diganti
`#jrg-url-ditolak`. Nilai di dalam `<script>`/`<style>`, di atribut `on*`/`style`, di atribut tanpa tanda kutip, atau
di tempat nama atribut adalah `Err`: taruh datanya di atribut `data-*` yang berkutip.

```jrg konteks.jrg
let d = {"u": "javascript:alert(1)", "nama": "Ani"}
wok(dok::render("<a href=\"{{u}}\">{{nama}}</a>", d).unwrap())
match dok::render("<p title={{nama}}>", d) {
    Ok(html) => wok(html),
    Err(e) => wok("ditolak:", str::contains(e, "tanpa tanda kutip")),
}
```
```keluaran
<a href="#jrg-url-ditolak">Ani</a>
ditolak: true
```

`dok::render_file(berkas, data)` membaca templatnya dari berkas.

## Cookie, sesi, dan sandi

```jrg sesi.jrg
let KUNCI = "kunci-sesi-yang-panjang-dan-acak"

let cookie = dok::session_encode({"user": "budi"}, KUNCI)
match dok::session_decode(cookie, KUNCI) {
    Ok(d) => wok("masuk sebagai", d.user),
    Err(e) => wok("ditolak:", e),
}

let simpanan = dok::password_hash("sandi123")
wok(dok::password_verify("sandi123", simpanan), dok::password_verify("tebakan", simpanan))
```
```keluaran
masuk sebagai budi
true false
```

- Sesi **ditandatangani**, tidak dienkripsi: isinya bisa dibaca siapa pun, tapi tidak bisa diubah tanpa kuncinya.
  Simpan kuncinya di variabel lingkungan, bukan di kode.
- `password_hash` memakai PBKDF2-HMAC-SHA256 dengan garam acak per sandi.
- `dok::cookie_set(resp, nama, nilai, opsi)` memasang cookie dengan `HttpOnly` dan `SameSite=Lax` secara bawaan;
  `dok::cookie_get(req, nama)` membacanya.

### Token CSRF

Formulir yang mengubah sesuatu (`POST`, `PUT`, `DELETE`) membawa token yang dibuat server untuk sesi itu. Situs lain
bisa membuat peramban korban mengirim formulir, tapi tidak bisa membaca tokennya.

```jrg csrf.jrg
let RAHASIA = "rahasia-csrf-yang-panjangnya-32b"

let token = dok::csrf_token(RAHASIA, "sesi-ani")
wok(dok::csrf_verify(token, RAHASIA, "sesi-ani"))
wok(dok::csrf_verify(token, RAHASIA, "sesi-budi"))
```
```keluaran
true
false
```

- Argumen ketiganya, **ikatan**, adalah id sesi: token sesi lain ditolak, jadi penyerang tidak bisa memakai token
  dari sesinya sendiri. Ikatan kosong ditolak.
- Token berumur 12 jam; argumen keempat `csrf_verify` mengubahnya (dalam detik).
- Rahasianya paling sedikit 32 byte. Taruh tokennya di `<input type="hidden">` lewat `dok::render`, dan periksa sebelum
  mengerjakan permintaan.

## WebSocket

Rute WebSocket hidup di router yang sama dengan rute HTTP biasa.

```jrg-cuplikan
dok::router_ws(r, "/gema", |koneksi, req| {
    while true {
        match dok::ws_recv(koneksi) {
            Some(pesan) => dok::ws_send(koneksi, "gema: " + pesan),
            None => { break }
        }
    }
})
```

`ws_recv` menunggu pesan berikutnya dan menjawab `None` saat koneksinya ditutup. Pesan biner datang sebagai array byte.

## Klien HTTP

```jrg-cuplikan
match http::get("https://api.github.com/repos/Kodok-Kejepit/jrg") {
    Ok(r) => wok(r.status, json::parse(r.body)["stargazers_count"]),
    Err(e) => wok("gagal:", e),
}

http::post("https://contoh.org/api", json::stringify({"a": 1}), "application/json")
http::unduh("https://contoh.org/besar.zip", "besar.zip")
```

`http::get`, `post`, `put`, `delete`, dan `request` menjawab `Result` berisi `status`, `headers`, `body`, dan `ok`.
HTTPS memakai libssl milik sistem. `http::unduh` menulis langsung ke berkas, jadi aman untuk unduhan besar, dan tidak
mengikuti pengalihan dari `https://` ke `http://`.

Kalau URL-nya datang dari pemakai (pratinjau tautan, webhook, pengunduh gambar), nyalakan penjaga SSRF sekali di awal:

```jrg-cuplikan
http::hanya_publik(true)
http::get("http://169.254.169.254/latest/meta-data/")   // Err: alamat link-local, metadata awan
```

Sesudahnya alamat loopback, privat, link-local dan metadata awan ditolak. Yang diperiksa alamat hasil DNS yang
benar-benar disambungi, juga di tiap pengalihan, jadi nama yang menunjuk `127.0.0.1` tidak lolos.

## SQLite

Driver SQLite dimuat dari `libsqlite3` milik sistem. `dok::db_tersedia()` memberi tahu apakah pustakanya ada.

```jrg sqlite.jrg lokal
let db = dok::db_open(":memory:")?
dok::db_exec(db, "CREATE TABLE murid(id INTEGER PRIMARY KEY, nama TEXT, nilai REAL)")?
dok::db_exec(db, "INSERT INTO murid(nama, nilai) VALUES (?, ?)", ["budi", 8.5])?
dok::db_exec(db, "INSERT INTO murid(nama, nilai) VALUES (?, ?)", ["ani", 9.25])?

for _, m in dok::db_query(db, "SELECT nama, nilai FROM murid ORDER BY nilai DESC")? {
    wok(m.nama, m.nilai)
}

let jahat = "budi'; DROP TABLE murid; --"
wok(dok::db_query(db, "SELECT * FROM murid WHERE nama = ?", [jahat])?)
dok::db_close(db)
```
```keluaran
ani 9.25
budi 8.5
[]
```

Nilai selalu diikat lewat `?`, jadi teks dari pengguna tidak pernah dibaca sebagai perintah SQL. Satu baris hasil
adalah dict, dan kolomnya dibaca dengan namanya.

## Jaringan tingkat rendah

`dok::tcp_listen`, `tcp_accept`, `tcp_connect`, `tcp_send`, `tcp_recv_line` untuk TCP, dan `dok::udp_bind`,
`udp_send_to`, `udp_recv_from` untuk UDP. `dok::log_info` dan kawan-kawannya mencetak log bertingkat, dengan warna di
terminal.
