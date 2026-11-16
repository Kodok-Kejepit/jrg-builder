---
judul: Web and data
ringkas: An HTTP server, templates, cookies and sessions, WebSocket, an HTTP client, and SQLite.
---
Everything a web application needs is in the `dok::` and `http::` namespaces, with no extra library.

## The server

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

`dok::serve` blocks and handles each connection in its own thread. The third argument sets the address. The default
is `"127.0.0.1"`, so it is only reachable from the same machine (or through a proxy such as Caddy/nginx on that
machine); write `"0.0.0.0"` to open it to the network.

`dok::router_static` does not serve files or folders whose name starts with a dot (`.env`, `.git/`, `.htpasswd`):
they get a 404 like a missing file, so a secret that ended up in the public folder does not leak. The exception is
`.well-known/`, which is public on purpose (for example ACME certificate challenges). Files are streamed from disk.

| Request (`req`) | Contents |
|---|---|
| `method`, `path` | `"GET"`, `"/sapa/Ani"` |
| `params` | the `:name` parts of the route pattern, read as `req.params.nama` |
| `query` | the pairs from `?a=1&b=2`; read with `dok::query_get(req.query, "a")` |
| `headers` | read with `dok::header_get(req.headers, "Content-Type")` |
| `body`, `body_bytes` | the body as text; if it is not UTF-8 text, `body` is `nil` and the bytes are in `body_bytes` |
| `remote_addr` | the requester's socket address, `"127.0.0.1:51234"` — the `X-Forwarded-For` header is written by the client itself, so use this for rate limits and logs, unless you are behind your own proxy |

A handler may return text (which becomes `200 text/plain`), the result of `dok::text`, `dok::html`,
`dok::json_response`, `dok::redirect`, `dok::response(code, body)`, `dok::binary(bytes)`, or a `Result` (an `Err`
becomes a 500).

### Middleware

`dok::router_use` runs before the route is looked up; returning `nil` means continue, returning a response means stop
there. `dok::router_after` runs after the handler and may change its response.

```jrg-cuplikan
dok::router_use(r, |req| {
    if str::starts_with(req.path, "/admin") && dok::cookie_get(req, "sesi").is_none() {
        return dok::response(401, "perlu masuk dulu")
    }
    return nil
})
dok::router_after(r, |req, resp| dok::header_set(resp, "X-Frame-Options", "DENY"))
```

## Templates

`dok::render` fills in `{{ name }}` and escapes its HTML, so user input is safe to display. `{{& name }}` inserts it as
is. There are also `{{#if}}…{{else}}…{{/if}}` and `{{#each}}…{{/each}}`.

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

HTML escaping only protects text and quoted attribute values, so `dok::render` reads where a `{{ name }}` sits. A
value in a URL attribute (`href`, `src`, `action`, …) whose scheme is not `http`, `https`, `mailto`, or `tel` is
replaced with `#jrg-url-ditolak`. A value inside `<script>`/`<style>`, in an `on*`/`style` attribute, in an unquoted
attribute, or in the position of an attribute name is an `Err`: put the data in a quoted `data-*` attribute.

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

`dok::render_file(file, data)` reads the template from a file.

## Cookies, sessions, and passwords

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

- A session is **signed**, not encrypted: anyone can read it, but nobody can change it without the key. Keep the key
  in an environment variable, not in the code.
- A session carries a signed **timestamp**: `dok::session_decode` refuses a session older than 14 days, or than the
  third argument in seconds (`dok::session_decode(cookie, KUNCI, 3600)` = one hour). A stolen cookie does not last
  forever.
- `password_hash` uses PBKDF2-HMAC-SHA256 with a random salt per password. `dok::password_needs_rehash(hash)` says
  when a stored hash was made with other settings, so it can be re-hashed after a successful login.
- `dok::cookie_set(resp, name, value, options)` sets a cookie with `HttpOnly` and `SameSite=Lax` by default;
  `dok::cookie_get(req, name)` reads it.

### CSRF tokens

A form that changes something (`POST`, `PUT`, `DELETE`) carries a token the server made for that session. Another
site can make the victim's browser submit a form, but cannot read the token.

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

- The third argument, the **binding**, is the session id: another session's token is refused, so an attacker cannot use
  a token from their own session. An empty binding is refused.
- A token lives 12 hours; the fourth argument of `csrf_verify` changes that (in seconds).
- The secret is at least 32 bytes. Put the token in an `<input type="hidden">` through `dok::render`, and check it
  before doing the request.

## WebSocket

WebSocket routes live on the same router as ordinary HTTP routes.

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

`ws_recv` waits for the next message and answers `None` when the connection closes. A binary message arrives as a
byte array. A request from a browser whose `Origin` names a different host from `Host` gets a 403 (the port may
differ), so a page on another site cannot open a WebSocket on the visitor's behalf. A client that stays silent is
asked with a `ping` and released if it does not answer.

## The HTTP client

```jrg-cuplikan
match http::get("https://api.github.com/repos/Kodok-Kejepit/jrg") {
    Ok(r) => wok(r.status, json::parse(r.body)["stargazers_count"]),
    Err(e) => wok("gagal:", e),
}

http::post("https://contoh.org/api", json::stringify({"a": 1}), "application/json")
http::unduh("https://contoh.org/besar.zip", "besar.zip")
```

`http::get`, `post`, `put`, `delete`, and `request` answer a `Result` holding `status`, `headers`, `body`, and `ok`.
HTTPS uses the system's libssl and needs TLS 1.2 or newer. `http::unduh` writes straight to a file, so it is safe for
large downloads, and it does not follow a redirect from `https://` to `http://`. `http::tenggat(seconds)` sets a total
deadline per request, so a server that trickles its answer cannot hold the caller forever.

If the URL comes from a user (link previews, webhooks, image downloaders), turn on the SSRF guard once at the start:

```jrg-cuplikan
http::hanya_publik(true)
http::get("http://169.254.169.254/latest/meta-data/")   // Err: link-local address, cloud metadata
```

After that, loopback, private, link-local, and cloud-metadata addresses are refused. What is checked is the address
DNS returned and that is actually connected to, on every redirect too, so a name pointing at `127.0.0.1` does not get
through.

## SQLite

The SQLite driver is loaded from the system's `libsqlite3`. `dok::db_tersedia()` tells whether the library is there.

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

Values are always bound through `?`, so text from a user is never read as an SQL command. Each result row is a dict,
and its columns are read by name.

## Low-level networking

`dok::tcp_listen`, `tcp_accept`, `tcp_connect`, `tcp_send`, `tcp_recv_line` for TCP, and `dok::udp_bind`,
`udp_send_to`, `udp_recv_from` for UDP. `dok::log_info` and friends print levelled logs, coloured in a terminal.
