#!/usr/bin/env python3
"""Mesin Praktik JRG di situs (`docs/aset/jrg-web.js`): `tulis` menulis asetnya,
`periksa` memastikan ia masih interpreter yang sama dengan `jrgc`."""
import base64, hashlib, json, os, re, shutil, subprocess, sys, tempfile
from pathlib import Path

AKAR = Path(__file__).resolve().parent.parent
ASET = AKAR / "docs" / "aset" / "jrg-web.js"
WASM = AKAR / "web" / "target" / "wasm32-unknown-unknown" / "release" / "jrg_web.wasm"
SUMBER = AKAR / "docs" / "_sumber"
JRGC = AKAR / "target" / "release" / "jrgc"


def masukan():
    """Semua yang menentukan isi wasm-nya — sengaja lebih lebar dari perlu:
    sidik yang basi karena perubahan tak berpengaruh cuma meminta build ulang."""
    for dasar in ("src", "stdlib", "web"):
        for p in sorted((AKAR / dasar).rglob("*")):
            if p.is_file() and "target" not in p.relative_to(AKAR).parts:
                yield p
    for f in ("Cargo.toml", "Cargo.lock", "perkakas/bangun_praktik.sh"):
        yield AKAR / f


def sidik():
    h = hashlib.sha256()
    for p in masukan():
        h.update(str(p.relative_to(AKAR)).encode() + b"\0" + p.read_bytes() + b"\0")
    return h.hexdigest()


def tulis():
    pekerja = (AKAR / "web" / "pekerja.js").read_text(encoding="utf-8")
    fungsi = pekerja[pekerja.index("function pekerjaJrg"):].strip()
    ASET.write_text(
        "// docs/aset/jrg-web.js — DIBANGUN `perkakas/bangun_praktik.sh`; jangan disunting.\n"
        "// Interpreter JRG yang sama dengan `jrgc` (crate `web/`), dalam WebAssembly.\n"
        "window.JRG_WEB = {\n"
        f'  sidik: "{sidik()}",\n'
        f"  pekerja: {fungsi},\n"
        f'  wasm: "{base64.b64encode(WASM.read_bytes()).decode()}"\n'
        "};\n", encoding="utf-8")
    print(f"{ASET.relative_to(AKAR)} ditulis ({ASET.stat().st_size // 1024} KB)")


# ── korpus ─────────────────────────────────────────────────────────────────

KHUSUS = [
    ("angka besar", 'wok(170141183460469231731687303715884105727, 9223372036854775807 * 2)'),
    ("pecahan", 'wok(0.1 + 0.2, 1.0 / 3.0, 2.0, 1e20, 5.0e-7, -0.0)'),
    ("nil dan Option", 'wok(nil, Option::Some(3), Option::None, [1, 2].first(), [].first())'),
    ("dict dan struct", 'struct P { pub x: int, pub y: string }\n'
                        'wok({"b": 2, "a": 1}, P { x: 1, y: "dua" })'),
    ("hook to_string", 'struct V { pub x: int }\n'
                       'impl V { fn to_string(self) -> string { return "V<" + str(self.x) + ">" } }\n'
                       'wok(V { x: 3 }, [V { x: 1 }])'),
    ("closure", 'let tambah = |a| |b| a + b\nwok(tambah(2)(3))'),
    ("match dan pola", 'fn f(x) { return match x { 0 => "nol", n @ 1..=9 => "kecil " + str(n), _ => "besar" } }\n'
                       'wok(f(0), f(7), f(99))'),
    ("rune dan teks", "wok('A', \"héllo\".len(), str::upper(\"abc\"), str::chars(\"ab\"))"),
    ("hof", 'wok([1, 2, 3].map(|x| x * 2).filter(|x| x > 2), arr::reduce([1, 2, 3], |a, b| a + b, 0))'),
    ("for dua nama", 'for i, x in [10, 20] { wok(i, x) }'),
    ("format angka", 'wok(str::format("{:.2} dan {}", 3.14159, "x"))'),
    ("import std", 'import "std/set"\npub fn main() { let s = set::from([1, 2, 2]); wok(s.len()) }'),
    ("assert_eq array", 'assert_eq([1, 2], [1, 2])\nwok("lulus")'),
    ("rekursi sedang", 'fn f(n) { return n == 0 $ 0 : 1 + f(n - 1) }\nwok(f(300))'),
    ("acak dalam rentang", 'let x = rand::int(1, 6)\nwok(x >= 1 && x <= 6, typeof(rand::float()))'),
    ("jam", 'let t = time::now()\nwok(typeof(t), t > 1700000000)'),
    ("galat konversi", 'wok("mulai")\nwok(int("abc"))'),
    ("galat di fungsi", 'fn dalam(a) { return a[9] }\nfn luar() { return dalam([1]) }\nwok(luar())'),
    ("galat parse", 'pub fn main() {\n    wok(1 + )\n}'),
    ("galat lex", 'wok("tidak ditutup)'),
    ("galat nama", 'let jumlah = 1\nwok(jumlas)'),
    ("panik", 'wok("a")\npanik("sengaja")'),
    ("keluar", 'wok("a")\nexit(3)'),
    ("keluar nol", 'wok("a")\nexit(0)'),
    ("peringatan tanpa kurung", 'fn f() { wok("x") }\npub fn main() {\n    f\n    wok("selesai")\n}'),
    ("main mengembalikan Err", 'pub fn main() { return Result::Err("gagal") }'),
    ("kanvas tanpa berkas", 'let c = Canvas::new(20, 10)\nc.fill_rect(0, 0, 5, 5, Warna { r: 255, g: 0, b: 0 })\n'
                            'wok(c.width(), c.get_pixel(1, 1), c.update(), c.delta_time() >= 0.0)'),
]

# Yang MEMANG beda di peramban — dan harus beda dengan kalimat yang jujur.
HARUS_DI_WEB = [
    ("goroutine", 'pub fn main() {\n    spawn { wok(1) }\n}', "butuh thread"),
    ("Future", 'pub fn main() {\n    let f = Future::new(|| 1)\n    wok(future::await(f))\n}', "butuh thread"),
    ("masukan", 'pub fn main() {\n    let s = tok()\n    wok(s)\n}', "tidak ada masukan dari keyboard"),
    ("rekursi dalam", 'fn f(n) { return n == 0 $ 0 : 1 + f(n - 1) }\npub fn main() { wok(f(5000)) }', "sebab:rekursi"),
    ("kanvas ke berkas", 'let c = Canvas::new(4, 4)\nc.save_png("a.png")', "tidak didukung di sistem ini"),
]


def korpus():
    """Tiap contoh kode situs (kelas matematika dan halaman dokumentasi) plus kasus tepi."""
    data = json.loads((SUMBER / "matematika.json").read_text(encoding="utf-8"))
    daftar = []
    for bab in data["content"]:
        for b in bab.get("blocks", []):
            if b.get("type") == "code":
                daftar.append((f"kursus {bab['id']}/{b.get('filename', '?')}", b["code"], b.get("output")))
    # `lokal`: butuh thread, berkas, jaringan, atau pustaka sistem; halamannya tidak menawarkan Jalankan.
    pagar = re.compile(r"^```jrg(?!-)[ \t]*(\S*)(?![ \t]+lokal)[ \t]*\n(.*?)^```", re.S | re.M)
    for p in sorted((SUMBER / "dokumentasi").glob("*.md")):
        for m in pagar.finditer(p.read_text(encoding="utf-8")):
            daftar.append((f"dokumentasi {p.stem}/{m.group(1) or '?'}", m.group(2), None))
    if len(daftar) < 150:
        sys.exit(f"korpus cuma {len(daftar)} program; sumber situs berubah bentuk?")
    return daftar + [(f"tepi: {n}", s, None) for n, s in KHUSUS]


def klaim_cocok(klaim, asli):
    """Output yang DIKLAIM halaman kursus: keluaran persis kalau programnya
    berhasil, baris galat pertamanya kalau memang contoh galat."""
    if asli["kode"] == 0:
        return asli["keluaran"].rstrip("\n") == klaim.rstrip("\n")
    galat = [b for b in asli["galat"].splitlines() if b.startswith("error")]
    return bool(galat) and galat[0] == klaim.splitlines()[0]


def jalankan_web(sumber):
    with tempfile.TemporaryDirectory() as d:
        f = Path(d) / "daftar.json"
        f.write_text(json.dumps(sumber), encoding="utf-8")
        r = subprocess.run(["node", "--stack-size=984",
                            str(AKAR / "perkakas" / "situs" / "jalankan_praktik.js"), str(ASET), str(f)],
                           capture_output=True, text=True, timeout=600)
        if r.returncode != 0:
            sys.exit("runner node gagal:\n" + r.stderr[-2000:])
        return json.loads(r.stdout)


# Di peramban sungguhan: pemuat malas, kompilasi, pekerja dari Blob, `file://`.
DI_PERAMBAN = [
    ('pub fn main() {\n    wok("Halo, dunia JRG!")\n}', lambda h: h["kode"] == 0 and h["teks"] == "Halo, dunia JRG!\n"),
    ('pub fn main() {\n    wok(int("abc"))\n}',
     lambda h: h["kode"] == 1 and "--> praktik.jrg:2:9" in h["teks"] and "^^^" in h["teks"]),
    ('pub fn main() {\n    spawn { wok(1) }\n}', lambda h: "butuh thread" in h["teks"] and "praktik.jrg:2:5" in h["teks"]),
    ('fn f(n) { return n == 0 $ 0 : 1 + f(n - 1) }\npub fn main() { wok(f(5000)) }', lambda h: h["sebab"] == "rekursi"),
]

SUNTIKAN = """<script src="../aset/praktik.js"></script>
<script>
(async function () {
  const hasil = [];
  for (const sumber of %s) {
    try { const h = await window.JRG_PRAKTIK.jalankan(sumber);
          hasil.push({kode: h.kode, sebab: h.sebab, teks: h.potongan.map(function (p) { return p[1]; }).join('')}); }
    catch (e) { hasil.push({galat: String(e)}); }
  }
  const d = document.createElement('div'); d.id = 'HASIL-PRAKTIK'; d.textContent = JSON.stringify(hasil);
  document.body.appendChild(d);
})();
</script>"""


def periksa_peramban(salah):
    krom = shutil.which("chromium") or shutil.which("chromium-browser") or shutil.which("google-chrome")
    if not krom or not shutil.which("node"):
        print("chromium/node tidak ada — pemeriksaan peramban DILEWATI (bukan lulus)")
        return
    hal = AKAR / "docs" / "matematika_x_jrg" / "ch1.html"
    # Salinannya di direktori yang sama: asetnya dirujuk `../aset/...`.
    salinan = hal.with_name("__praktik__" + hal.name)
    try:
        salinan.write_text(hal.read_text(encoding="utf-8").replace(
            "</body>", SUNTIKAN % json.dumps([s for s, _ in DI_PERAMBAN]) + "</body>"), encoding="utf-8")
        r = subprocess.run(["node", str(AKAR / "perkakas" / "situs" / "praktik_peramban.js"), krom,
                            salinan.as_uri()], capture_output=True, text=True, timeout=180)
    finally:
        salinan.unlink(missing_ok=True)
    try:
        hasil = json.loads(r.stdout)
    except ValueError:
        salah.append(f"peramban: tidak ada hasil ({r.stdout[-200:]!r} {r.stderr[-300:]!r})")
        return
    for (sumber, cocok), h in zip(DI_PERAMBAN, hasil):
        if not cocok(h):
            salah.append(f"peramban: {sumber.splitlines()[-2 if len(sumber.splitlines()) > 1 else 0][:50]!r} -> {h}")
    print(f"peramban (chromium, file://): {len(hasil)} program lewat halaman terbit")


def jalankan_asli(sumber):
    env = {k: v for k, v in os.environ.items() if k not in ("JRG_SOPAN", "JRG_NADA", "JRG_ULANGAN", "DISPLAY", "WAYLAND_DISPLAY")}
    with tempfile.TemporaryDirectory() as d:
        (Path(d) / "praktik.jrg").write_text(sumber, encoding="utf-8")
        try:
            r = subprocess.run([str(JRGC), "praktik.jrg"], cwd=d, env=env, capture_output=True,
                               text=True, timeout=60, stdin=subprocess.DEVNULL)
        except subprocess.TimeoutExpired:
            return {"kode": None, "keluaran": "", "galat": "(dibunuh batas waktu)"}
        return {"kode": r.returncode, "keluaran": r.stdout, "galat": r.stderr}


def periksa():
    salah = []
    if not ASET.exists():
        sys.exit(f"{ASET.relative_to(AKAR)} tidak ada — jalankan `sh perkakas/bangun_praktik.sh`")
    teks = ASET.read_text(encoding="utf-8")
    m = re.search(r'sidik: "([0-9a-f]{64})"', teks)
    if not m or m.group(1) != sidik():
        salah.append("mesin Praktik BASI: dibangun dari sumber yang lain dari sekarang — "
                     "jalankan `sh perkakas/bangun_praktik.sh`")
    wasm = base64.b64decode(re.search(r'wasm: "([A-Za-z0-9+/=]+)"', teks).group(1))
    for rahasia in {str(Path.home()), str(AKAR)}:
        if rahasia.encode() in wasm:
            salah.append(f"wasm-nya membawa path mesin ini (`{rahasia}`) — situsnya publik")
    print(f"mesin Praktik: {len(wasm) // 1024} KB wasm, aset {ASET.stat().st_size // 1024} KB")

    if not shutil.which("node"):
        print("node tidak ada — pemeriksaan diferensial DILEWATI (bukan lulus)")
    elif not JRGC.exists():
        print(f"{JRGC.relative_to(AKAR)} tidak ada — pemeriksaan diferensial DILEWATI (bukan lulus)")
    else:
        daftar = korpus()
        web = jalankan_web([s for _, s, _ in daftar] + [s for _, s, _ in HARUS_DI_WEB])
        klaim_diperiksa = 0
        for (nama, s, klaim), w in zip(daftar, web):
            a = jalankan_asli(s)
            if klaim is not None:
                klaim_diperiksa += 1
                if not klaim_cocok(klaim, a):
                    salah.append(f"[{nama}] output yang diklaim halaman kursus BASI:\n"
                                 f"    klaim: {klaim[:160]!r}\n    jrgc : {a['keluaran'][:160]!r} {a['galat'][:160]!r}")
            beda = [k for k in ("kode", "keluaran", "galat") if a[k] != w[k]]
            if w["sebab"] or beda:
                salah.append(f"[{nama}] berbeda dari jrgc di {beda or 'sebab'} (sebab web: {w['sebab']})\n"
                             f"    jrgc: {a['kode']} {a['keluaran'][:120]!r} {a['galat'][:200]!r}\n"
                             f"    web : {w['kode']} {w['keluaran'][:120]!r} {w['galat'][:200]!r}")
        for (nama, _, memuat), w in zip(HARUS_DI_WEB, web[len(daftar):]):
            if memuat.startswith("sebab:"):
                if w["sebab"] != memuat[len("sebab:"):]:
                    salah.append(f"[{nama}] harusnya berhenti karena {memuat}, sebab web: {w['sebab']}")
            elif memuat not in w["galat"] or w["kode"] == 0:
                salah.append(f"[{nama}] di peramban harusnya gagal menyebut `{memuat}`, dapat: "
                             f"kode {w['kode']} {w['galat'][:200]!r}")
        # Lantai: diferensial yang tidak membandingkan apa pun lulus dengan sempurna.
        if len(daftar) < 100:
            salah.append(f"cuma {len(daftar)} program dibandingkan — korpusnya menyusut")
        print(f"diferensial: {len(daftar)} program sama persis dengan jrgc "
              f"(keluaran, galat, kode keluar), {len(HARUS_DI_WEB)} beda yang disengaja")
        print(f"klaim kursus: {klaim_diperiksa} output yang ditulis halaman diperiksa terhadap jrgc")
        if klaim_diperiksa < 100:
            salah.append(f"cuma {klaim_diperiksa} klaim output kursus diperiksa — datanya menyusut")

    periksa_peramban(salah)

    if salah:
        print(f"\n{len(salah)} MASALAH:")
        for s in salah[:40]:
            print("  -", s)
        sys.exit(1)
    print("PRAKTIK BERSIH")


if __name__ == "__main__":
    perintah = sys.argv[1:]
    if perintah == ["tulis"]:
        tulis()
    elif perintah == ["periksa"]:
        periksa()
    else:
        sys.exit("pakai: python3 perkakas/praktik.py tulis|periksa")
