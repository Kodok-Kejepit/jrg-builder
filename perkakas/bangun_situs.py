#!/usr/bin/env python3
"""Bangun situs docs/ dari docs/_sumber/.

  dokumentasi/*.md     satu berkas per halaman dokumentasi
  matematika.json      isi kelas Matematika x JRG
  beranda.html         isi halaman depan
  kelas-pembuka.html   pembuka di halaman pertama kelas

Aset tulisan tangan (situs.css, situs.js, kelas.js, praktik.js) tinggal di docs/aset/ dan
tidak disentuh; yang ditulis ulang cuma halaman HTML, aset/cari.js dan aset/kelas-data.js.

Pakai dari akar repo:  python3 perkakas/bangun_situs.py [--keluar DIR]
"""
import html
import json
import re
import subprocess
import sys
from pathlib import Path

AKAR = Path(__file__).resolve().parent.parent
SUMBER = AKAR / "docs" / "_sumber"
DOCS = AKAR / "docs"

VERSI = "0.2.0"
REPO = "https://github.com/Kodok-Kejepit/jrg"

URUTAN_DOK = [
    ("Mulai", ["index", "pasang", "tur"]),
    ("Bahasa", ["nilai", "kendali", "fungsi", "tipe", "pola", "galat", "modul", "konkuren"]),
    ("Pustaka", ["pustaka", "web", "grafis", "game", "ffi"]),
    ("Perkakas", ["jrgc", "jrgm", "editor", "android"]),
    ("Di balik layar", ["mesin", "memori", "diagnostik", "benchmark", "batasan", "rilis"]),
]

esc = html.escape


def jumlah_builtin() -> str:
    """Jumlah builtin dari `jrgc` yang terbangun — angka di situs diturunkan, tidak diketik."""
    jrgc = AKAR / "target" / "release" / "jrgc"
    o = subprocess.run([str(jrgc), "--daftar-builtin"], capture_output=True, text=True, check=True)
    return str(len([b for b in o.stdout.splitlines() if b.strip()]))


ANGKA = {}


def tulis(p: Path, isi: str):
    if "{BUILTIN}" in isi:
        ANGKA.setdefault("builtin", jumlah_builtin())
        isi = isi.replace("{BUILTIN}", ANGKA["builtin"])
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(isi, encoding="utf-8")


def slug(teks: str) -> str:
    t = re.sub(r"<[^>]+>", "", teks).lower()
    t = re.sub(r"[`*]", "", t)
    t = re.sub(r"[^a-z0-9]+", "-", t).strip("-")
    return t or "bagian"


# ── pewarna sintaks ─────────────────────────────────────────────────────

KATA_KUNCI = set("""package import as pub fn let var const mut return if else while for in loop break continue
match struct enum trait impl self Self spawn go select defer move extern type where true false nil""".split())
TIPE = set("int uint float bool string rune byte i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 usize isize".split())
POLA_JRG = re.compile(r'''
  (?P<kom>//[^\n]*|/\*.*?\*/)
| (?P<str>"""(?:.|\n)*?"""|«(?:.|\n)*?»|"(?:\\.|[^"\\\n])*"|`[^`]*`|'(?:\\.|[^'\\\n])')
| (?P<ang>\b0x[0-9a-fA-F_]+\b|\b\d[\d_]*(?:\.\d[\d_]*)?(?:e[+-]?\d+)?\b)
| (?P<nama>[A-Za-z_][A-Za-z0-9_]*)
''', re.X | re.S)


def warnai_jrg(kode: str) -> str:
    keluar, i = [], 0
    for m in POLA_JRG.finditer(kode):
        keluar.append(esc(kode[i:m.start()]))
        t = m.group(0)
        if m.lastgroup == "kom":
            keluar.append(f'<span class="k-kom">{esc(t)}</span>')
        elif m.lastgroup == "str":
            keluar.append(f'<span class="k-str">{esc(t)}</span>')
        elif m.lastgroup == "ang":
            keluar.append(f'<span class="k-ang">{esc(t)}</span>')
        else:
            sisa = kode[m.end():m.end() + 2]
            if t in KATA_KUNCI:
                kelas = "k-kunci"
            elif t in TIPE:
                kelas = "k-tipe"
            elif sisa == "::":
                kelas = "k-ns"
            elif sisa[:1] == "(":
                kelas = "k-fn"
            elif t[:1].isupper():
                kelas = "k-tipe"
            else:
                kelas = ""
            keluar.append(f'<span class="{kelas}">{esc(t)}</span>' if kelas else esc(t))
        i = m.end()
    keluar.append(esc(kode[i:]))
    return "".join(keluar)


def warnai_sh(kode: str) -> str:
    baris = []
    for b in kode.split("\n"):
        k = re.search(r"(^|\s)#", b)
        if k:
            baris.append(esc(b[:k.start()]) + f'<span class="k-kom">{esc(b[k.start():])}</span>')
        else:
            baris.append(esc(b))
    return "\n".join(baris)


def blok_kode(kode: str, bahasa: str, nama: str = "", keluaran=None, jalan=False, lokal=False) -> str:
    kode = kode.rstrip("\n")
    if bahasa in ("jrg", "jrg-cuplikan"):
        isi = warnai_jrg(kode)
    elif bahasa == "sh":
        isi = warnai_sh(kode)
    else:
        isi = esc(kode)
    label = nama or {"jrg": "jrg", "jrg-cuplikan": "jrg", "sh": "terminal"}.get(bahasa, bahasa)
    atribut = " data-jalan" if jalan else ""
    keluar = [f'<figure class="kode"{atribut}>',
              f'<figcaption><span>{esc(label)}</span>'
              + ('<span class="lokal" title="Butuh thread, berkas, jaringan, atau pustaka sistem yang tidak ada di peramban">'
                 'jalankan dengan jrgc</span>' if lokal else '') + '</figcaption>',
              f'<pre><code>{isi}</code></pre>']
    if keluaran is not None:
        keluar.append(f'<pre class="keluaran"><code>{esc(keluaran.rstrip(chr(10)))}</code></pre>')
    keluar.append("</figure>")
    return "".join(keluar)


# ── markdown secukupnya ─────────────────────────────────────────────────

def inline(t: str) -> str:
    potong = []

    def simpan(s):
        potong.append(s)
        return f"\x00{len(potong) - 1}\x00"
    t = re.sub(r"`([^`]+)`", lambda m: simpan(f"<code>{esc(m.group(1))}</code>"), t)
    t = re.sub(r"</?[a-z][^>]*>", lambda m: simpan(m.group(0)), t)
    t = esc(t, quote=False)
    t = re.sub(r"\[([^\]]+)\]\(([^)\s]+)\)", lambda m: f'<a href="{m.group(2)}">{m.group(1)}</a>', t)
    t = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", t)
    t = re.sub(r"(?<![\w*])\*(?!\s)(.+?)(?<!\s)\*(?![\w*])", r"<em>\1</em>", t)
    return re.sub(r"\x00(\d+)\x00", lambda m: potong[int(m.group(1))], t)


AWAL_BLOK = re.compile(r"(```|#{2,3}\s|\||>|-\s|\d+\.\s|<[a-z])")


def markdown(sumber: str):
    """HTML, daftar (level, id, judul), dan teks polos untuk pencarian."""
    baris = sumber.split("\n")
    keluar, judul, teks = [], [], []
    i = 0
    dipakai = set()

    def id_unik(t):
        s0 = s = slug(t)
        n = 2
        while s in dipakai:
            s, n = f"{s0}-{n}", n + 1
        dipakai.add(s)
        return s

    def ambil_pagar(mulai):
        j = mulai + 1
        while j < len(baris) and not baris[j].startswith("```"):
            j += 1
        return "\n".join(baris[mulai + 1:j]), j + 1

    while i < len(baris):
        b = baris[i]
        if not b.strip():
            i += 1
            continue
        m = re.match(r"```(\S*)\s*(.*)$", b)
        if m:
            bahasa, info = m.group(1) or "teks", m.group(2).split()
            lokal = "lokal" in info
            nama = " ".join(x for x in info if x != "lokal")
            kode, i = ambil_pagar(i)
            keluaran = None
            k = i
            while k < len(baris) and not baris[k].strip():
                k += 1
            if k < len(baris) and re.match(r"```keluaran\b", baris[k]):
                keluaran, i = ambil_pagar(k)
            keluar.append(blok_kode(kode, bahasa, nama, keluaran, jalan=bahasa == "jrg" and not lokal, lokal=lokal))
            teks.append(kode)
            continue
        m = re.match(r"(#{2,3})\s+(.*)$", b)
        if m:
            lv = len(m.group(1))
            teks_j, id_tulis = m.group(2), None
            mi = re.search(r"\s*\{#([\w-]+)\}$", teks_j)
            if mi:
                teks_j, id_tulis = teks_j[:mi.start()], mi.group(1)
            j_html = inline(teks_j)
            hid = id_tulis or id_unik(teks_j)
            dipakai.add(hid)
            judul.append((lv, hid, re.sub(r"<[^>]+>", "", j_html)))
            keluar.append(f'<h{lv} id="{hid}"><a class="jangkar" href="#{hid}">{j_html}</a></h{lv}>')
            teks.append(teks_j)
            i += 1
            continue
        if b.startswith("|"):
            rows = []
            while i < len(baris) and baris[i].startswith("|"):
                rows.append([c.strip().replace("\\|", "|") for c in re.split(r"(?<!\\)\|", baris[i].strip()[1:-1])])
                i += 1
            kepala, isi = rows[0], [r for r in rows[1:] if not re.match(r"^:?-+:?$", r[0])]
            h = "".join(f"<th>{inline(c)}</th>" for c in kepala)
            r = "".join("<tr>" + "".join(f"<td>{inline(c)}</td>" for c in row) + "</tr>" for row in isi)
            keluar.append(f'<div class="tabel"><table><thead><tr>{h}</tr></thead><tbody>{r}</tbody></table></div>')
            teks.append(" ".join(" ".join(r) for r in rows))
            continue
        if b.startswith(">"):
            isi = []
            while i < len(baris) and baris[i].startswith(">"):
                isi.append(baris[i][1:].strip())
                i += 1
            keluar.append(f'<aside class="catatan"><p>{inline(" ".join(isi))}</p></aside>')
            teks.append(" ".join(isi))
            continue
        if re.match(r"(-|\d+\.)\s", b):
            berurut = bool(re.match(r"\d+\.", b))
            butir = []
            while i < len(baris) and (re.match(r"(-|\d+\.)\s", baris[i]) or
                                       (baris[i].startswith("  ") and butir)):
                if re.match(r"(-|\d+\.)\s", baris[i]):
                    butir.append(re.sub(r"^(-|\d+\.)\s+", "", baris[i]))
                else:
                    butir[-1] += " " + baris[i].strip()
                i += 1
            tag = "ol" if berurut else "ul"
            keluar.append(f"<{tag}>" + "".join(f"<li>{inline(x)}</li>" for x in butir) + f"</{tag}>")
            teks.extend(butir)
            continue
        if re.match(r"<[a-z]", b):
            j = i
            while j < len(baris) and baris[j].strip():
                j += 1
            keluar.append("\n".join(baris[i:j]))
            i = j
            continue
        para = []
        while i < len(baris) and baris[i].strip() and not AWAL_BLOK.match(baris[i]):
            para.append(baris[i].strip())
            i += 1
        keluar.append(f"<p>{inline(' '.join(para))}</p>")
        teks.append(" ".join(para))
    polos = re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", " ".join(teks)))
    return "\n".join(keluar), judul, polos


def baca_md(p: Path):
    t = p.read_text(encoding="utf-8")
    meta = {}
    if t.startswith("---\n"):
        akhir = t.index("\n---\n", 4)
        for b in t[4:akhir].split("\n"):
            if ":" in b:
                k, v = b.split(":", 1)
                meta[k.strip()] = v.strip()
        t = t[akhir + 5:]
    if "judul" not in meta:
        sys.exit(f"{p}: tidak punya `judul:` di kepala berkas")
    return meta, t


def polos(t: str) -> str:
    return re.sub(r"<[^>]+>|`", "", t)


# ── kerangka halaman ────────────────────────────────────────────────────

IKON = {
    "cari": '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>',
    "tema": '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z"/></svg>',
    "menu": '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h16"/></svg>',
    "git": ('<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 19c-4 1.3-4-2-6-2.5m12 4.5v-3.5c0-1 '
            '.1-1.4-.5-2 2.8-.3 5.5-1.4 5.5-6a4.6 4.6 0 0 0-1.3-3.2 4.2 4.2 0 0 0-.1-3.2s-1-.3-3.4 1.3a11.6 '
            '11.6 0 0 0-6 0C6.8 2.8 5.8 3.1 5.8 3.1a4.2 4.2 0 0 0-.1 3.2A4.6 4.6 0 0 0 4.4 9.5c0 4.6 2.7 5.7 '
            '5.5 6-.6.6-.6 1.2-.5 2V21"/></svg>'),
}


def kepala(aktif: str, naik: str) -> str:
    def tab(kunci, label, href):
        kelas = ' class="aktif" aria-current="page"' if kunci == aktif else ""
        return f'<a{kelas} href="{naik}{href}">{label}</a>'
    menu = ('<button class="tombol-ikon tombol-menu" type="button" aria-label="Buka navigasi" '
            f'data-aksi="menu">{IKON["menu"]}</button>') if aktif in ("dok", "mtk") else ""
    return f"""<header class="kepala">
  {menu}<a class="merek" href="{naik}index.html" aria-label="JRG++ beranda"><img class="terang-saja" src="{naik}aset/logo.png" alt="JRG++" width="107" height="34"><img class="gelap-saja" src="{naik}aset/logo-gelap.png" alt="JRG++" width="107" height="34"></a>
  <nav class="situs" aria-label="Situs">{tab("dok", "Dokumentasi", "web_dokumentasi/index.html")}{tab("mtk", "Matematika × JRG", "matematika_x_jrg/index.html")}{tab("profil", "Profil", "profil.html")}</nav>
  <span class="versi">v{VERSI}</span>
  <div class="kepala-kanan">
    <button class="tombol-cari" type="button" data-aksi="cari">{IKON["cari"]}<span>Cari</span><kbd>/</kbd></button>
    <button class="tombol-ikon" type="button" data-aksi="tema" aria-label="Ganti tema terang atau gelap">{IKON["tema"]}</button>
    <a class="tombol-ikon" href="{REPO}" aria-label="Kode sumber di GitHub">{IKON["git"]}</a>
  </div>
</header>"""


def halaman(judul, deskripsi, aktif, naik, isi, skrip=()):
    js = "".join(f'\n<script src="{naik}aset/{s}"></script>' for s in ("cari.js", "situs.js") + tuple(skrip))
    return f"""<!DOCTYPE html>
<html lang="id">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{esc(judul)}</title>
<meta name="description" content="{esc(deskripsi)}">
<link rel="icon" type="image/png" sizes="96x96" href="{naik}aset/favicon-96.png">
<link rel="icon" type="image/png" sizes="128x128" href="{naik}aset/favicon.png">
<link rel="apple-touch-icon" href="{naik}aset/favicon.png">
<meta property="og:image" content="https://jrg-lang.org/aset/banner.jpg">
<link rel="stylesheet" href="{naik}aset/situs.css">
<script>try{{var t=localStorage.getItem('jrg-tema');if(t)document.documentElement.dataset.tema=t}}catch(e){{}}</script>
</head>
<body data-situs="{aktif}">
<a class="lewati" href="#isi">Lewati ke isi</a>
{kepala(aktif, naik)}
{isi}
<div class="dialog-cari" hidden><div class="kotak-cari" role="dialog" aria-label="Cari">
  <input type="search" placeholder="Cari di dokumentasi dan kelas…" aria-label="Kata yang dicari">
  <ol class="hasil-cari"></ol>
</div></div>{js}
</body>
</html>
"""


def kaki(naik: str) -> str:
    return (f'<footer class="kaki"><span class="kaki-merek"><img class="terang-saja" src="{naik}aset/logo-kaki.png" alt="JRG++" '
            f'width="150" height="44"><img class="gelap-saja" src="{naik}aset/logo-kaki-gelap.png" alt="JRG++" width="150" '
            f'height="46"><span>versi {VERSI}</span></span>'
            f'<span><a href="{naik}web_dokumentasi/pasang.html">Pasang</a> · '
            f'<a href="{naik}web_dokumentasi/rilis.html">Catatan rilis</a> · '
            f'<a href="{naik}profil.html">Profil</a> · '
            f'<a href="{REPO}">Kode sumber</a></span></footer>')


def langkah(sebelum, sesudah, label_s="Sebelumnya", label_b="Berikutnya"):
    def a(x, arah, label):
        if not x:
            return "<span></span>"
        return f'<a class="ke-{arah}" href="{x[0]}"><small>{label}</small><strong>{esc(x[1])}</strong></a>'
    return (f'<nav class="langkah" aria-label="Halaman berurutan">'
            f'{a(sebelum, "kiri", label_s)}{a(sesudah, "kanan", label_b)}</nav>')


def daftar_isi(judul):
    h2 = [j for j in judul if j[0] == 2]
    if len(h2) < 3:
        return ""
    butir = "".join(f'<li><a href="#{i}">{esc(t)}</a></li>' for _, i, t in h2)
    return f'<nav class="di-halaman" aria-label="Di halaman ini"><p>Di halaman ini</p><ol>{butir}</ol></nav>'


# ── situs dokumentasi ───────────────────────────────────────────────────

def bangun_dokumentasi(keluar: Path, cari: list):
    folder = SUMBER / "dokumentasi"
    hal = []
    for grup, nama in URUTAN_DOK:
        for n in nama:
            meta, md = baca_md(folder / f"{n}.md")
            hal.append((grup, n, meta, md))
    lebih = {p.stem for p in folder.glob("*.md")} - {h[1] for h in hal}
    if lebih:
        sys.exit(f"halaman tidak ada di URUTAN_DOK: {sorted(lebih)}")

    def menu(meta):
        return meta.get("menu", polos(meta["judul"]))

    def sidebar(aktif):
        k, grup_kini = [], None
        for grup, n, meta, _ in hal:
            if grup != grup_kini:
                if grup_kini:
                    k.append("</ul>")
                k.append(f'<p class="sisi-grup">{grup}</p><ul>')
                grup_kini = grup
            kelas = ' class="aktif" aria-current="page"' if n == aktif else ""
            k.append(f'<li><a{kelas} href="{n}.html">{esc(menu(meta))}</a></li>')
        k.append("</ul>")
        return '<aside class="sisi" aria-label="Navigasi dokumentasi"><nav>' + "".join(k) + "</nav></aside>"

    for i, (grup, n, meta, md) in enumerate(hal):
        isi_html, judul, teks = markdown(md)
        s = hal[i - 1] if i else None
        b = hal[i + 1] if i + 1 < len(hal) else None
        nav = langkah(s and (f"{s[1]}.html", menu(s[2])), b and (f"{b[1]}.html", menu(b[2])))
        ringkas = meta.get("ringkas", "")
        badan = f"""<div class="tata">
{sidebar(n)}
<main id="isi" class="isi">
<article class="artikel">
<p class="rel">{grup}</p>
<h1>{inline(meta["judul"])}</h1>
{f'<p class="ringkas">{inline(ringkas)}</p>' if ringkas else ''}
{isi_html}
</article>
{nav}
{kaki("../")}
</main>
{daftar_isi(judul)}
</div>"""
        jdl = "JawaRustGo++ — Dokumentasi" if n == "index" else f'{polos(meta["judul"])} — Dokumentasi JRG'
        tulis(keluar / "web_dokumentasi" / f"{n}.html",
              halaman(jdl, polos(ringkas or meta["judul"]), "dok", "../", badan))
        cari.append({"j": polos(meta["judul"]), "u": f"web_dokumentasi/{n}.html", "g": "Dokumentasi",
                     "k": polos(ringkas), "t": teks[:1500]})
        for lv, hid, t in judul:
            if lv == 2:
                cari.append({"j": t, "u": f"web_dokumentasi/{n}.html#{hid}", "g": menu(meta), "k": "", "t": ""})
    return len(hal)


# ── situs matematika ────────────────────────────────────────────────────

LABEL_CATATAN = {"tip": "Tips", "info": "Catatan", "warn": "Hati-hati", "bug": "Jebakan",
                 "fun": "Tahukah kamu", "real": "Di dunia nyata"}


def blok_mtk(b, bab_id, nomor_kuis):
    t = b.get("type")
    if t == "heading":
        lencana = {"konsep": "Konsep", "kode": "Kode JRG"}.get(b.get("badge"), b.get("badge") or "")
        lab = f'<span class="lencana">{esc(str(lencana))}</span>' if lencana else ""
        teks = b.get("text", "")
        return f'<h2 id="{slug(teks)}">{lab}{teks}</h2>', teks
    if t == "reftitle":
        return f'<h3>{b.get("text", "")}</h3>', b.get("text", "")
    if t == "paragraph":
        return f'<p>{b.get("html", "")}</p>', b.get("html", "")
    if t == "keypoint":
        jenis = b.get("subtype") or "info"
        jdl = b.get("title") or LABEL_CATATAN.get(jenis, "Catatan")
        return (f'<aside class="catatan catatan-{jenis}"><p class="catatan-judul">{jdl}</p>'
                f'<div>{b.get("html", "")}</div></aside>'), b.get("html", "")
    if t == "code":
        kode = b.get("code", "")
        return blok_kode(kode, "jrg", b.get("filename", ""), b.get("output") or None, jalan=True), kode
    if t == "mathbox":
        baris = "".join(f'<div class="rumus-{"persamaan" if l.get("kind") == "eq" else "langkah"}">{l.get("html", "")}</div>'
                        for l in b.get("lines", []))
        return f'<div class="rumus">{baris}</div>', ""
    if t == "table":
        h = "".join(f"<th>{x}</th>" for x in b.get("headers", []))
        r = "".join("<tr>" + "".join(f"<td>{c}</td>" for c in row) + "</tr>" for row in b.get("rows", []))
        return f'<div class="tabel"><table><thead><tr>{h}</tr></thead><tbody>{r}</tbody></table></div>', ""
    if t == "shapelist":
        k = "".join(f'<div class="bentuk"><b>{x.get("name", "")}</b><span>{x.get("formula", "")}</span></div>'
                    for x in b.get("items", []))
        return f'<div class="daftar-bentuk">{k}</div>', ""
    if t == "pctbar":
        p = b.get("percent", 50)
        return (f'<div class="batang-persen"><div style="width:{p}%"></div>'
                f'<span>{b.get("label", "")}</span></div>'), ""
    if t == "practice":
        return (f'<details class="latihan"><summary>Latihan mandiri</summary>'
                f'<div>{b.get("html", "")}</div></details>'), ""
    if t == "quiz":
        nomor_kuis[0] += 1
        qid = b.get("qid") or f"{bab_id}-{nomor_kuis[0]}"
        pil = "".join(
            f'<button type="button" data-benar="{1 if o.get("correct") else 0}">'
            f'<span>{"ABCDEF"[i]}</span>{o.get("text", "")}</button>'
            for i, o in enumerate(b.get("options", [])))
        return (f'<div class="kuis" data-kuis="{esc(qid)}"><p class="kuis-label">Kuis {nomor_kuis[0]}</p>'
                f'<p class="kuis-tanya">{b.get("question", "")}</p><div class="kuis-pilihan">{pil}</div>'
                f'<p class="kuis-umpan benar" hidden>{b.get("feedbackOk", "")}</p>'
                f'<p class="kuis-umpan salah" hidden>{b.get("feedbackNo", "")}</p></div>'), b.get("question", "")
    if t == "raw":
        return f'<div class="mentah">{b.get("html", "")}</div>', ""
    sys.exit(f"jenis blok matematika tidak dikenal: {t} (bab {bab_id})")


def bangun_matematika(keluar: Path, cari: list):
    data = json.loads((SUMBER / "matematika.json").read_text(encoding="utf-8"))
    bab = data["content"]
    pertama = bab[0]["id"]
    berkas = {c["id"]: ("index" if c["id"] == pertama else c["id"]) + ".html" for c in bab}
    per_id = {c["id"]: c for c in bab}
    grup_dari = {i: g["label"] for g in data["navGroups"] for i in g["items"]}
    dilacak = [c["id"] for c in bab if c.get("kind") in ("chapter", "project")]

    def label(c):
        return c.get("navLabel") or c.get("title", c["id"])

    soal = {}
    for c in bab:
        q = [b.get("qid") or f'{c["id"]}-{n}'
             for n, b in enumerate((b for b in c.get("blocks", []) if b.get("type") == "quiz"), 1)]
        soal[c["id"]] = q

    def sidebar(aktif):
        k = [f'<div class="progres"><div class="progres-batang"><div></div></div>'
             f'<p><span class="progres-angka">0</span> dari {len(dilacak)} bab selesai</p></div>']
        for g in data["navGroups"]:
            k.append(f'<p class="sisi-grup">{esc(g["label"])}</p><ul>')
            for i in g["items"]:
                kelas = ' class="aktif" aria-current="page"' if i == aktif else ""
                k.append(f'<li><a{kelas} href="{berkas[i]}" data-bab="{i}">{esc(label(per_id[i]))}</a></li>')
            k.append("</ul>")
        return '<aside class="sisi" aria-label="Daftar bab"><nav>' + "".join(k) + "</nav></aside>"

    for idx, c in enumerate(bab):
        nomor = [0]
        bagian, teks = [], []
        for b in c.get("blocks", []):
            h, t = blok_mtk(b, c["id"], nomor)
            bagian.append(h)
            teks.append(re.sub(r"<[^>]+>", " ", t))
        s = bab[idx - 1] if idx else None
        n = bab[idx + 1] if idx + 1 < len(bab) else None
        nav = langkah(s and (berkas[s["id"]], s["title"]), n and (berkas[n["id"]], n["title"]),
                      "Bab sebelumnya", "Bab berikutnya")
        rel = esc(grup_dari.get(c["id"], ""))
        lab = label(c).split(" — ")[0]
        if lab != c.get("title") and lab != grup_dari.get(c["id"]):
            rel = f"{esc(lab)} · {rel}" if rel else esc(lab)
        sub = c.get("subtitle")
        pembuka = (SUMBER / "kelas-pembuka.html").read_text(encoding="utf-8") if idx == 0 else ""
        tombol = ""
        if c.get("kind") == "project":
            tombol = (f'<p class="selesai-proyek"><button class="tombol" type="button" '
                      f'data-proyek="{c["id"]}">Tandai proyek selesai</button></p>')
        badan = f"""<div class="tata">
{sidebar(c["id"])}
<main id="isi" class="isi">
<article class="artikel kelas" data-bab="{c["id"]}">
<p class="rel">{rel}</p>
<h1>{esc(c.get("title", ""))}</h1>
{f'<p class="ringkas">{sub}</p>' if sub else ''}
{pembuka}
{chr(10).join(bagian)}
{tombol}
</article>
{nav}
{kaki("../")}
</main>
</div>"""
        jdl = "Matematika × JRG — belajar dari nol" if idx == 0 else f'{c.get("title")} — Matematika × JRG'
        desk = re.sub(r"<[^>]+>", "", sub or c.get("title", ""))[:180]
        tulis(keluar / "matematika_x_jrg" / berkas[c["id"]],
              halaman(jdl, desk, "mtk", "../", badan, ("kelas-data.js", "kelas.js")))
        cari.append({"j": c.get("title", ""), "u": f"matematika_x_jrg/{berkas[c['id']]}", "g": "Matematika × JRG",
                     "k": re.sub(r"<[^>]+>", "", sub or ""),
                     "t": re.sub(r"\s+", " ", " ".join(teks))[:1500]})
    tulis(keluar / "aset" / "kelas-data.js",
          "// Dibangun perkakas/bangun_situs.py.\nwindow.JRG_KELAS = "
          + json.dumps({"bab": dilacak, "soal": {i: soal[i] for i in dilacak},
                        "proyek": [c["id"] for c in bab if c.get("kind") == "project"]},
                       ensure_ascii=False) + ";\n")
    return len(bab)


# ── beranda ─────────────────────────────────────────────────────────────

def bangun_beranda(keluar: Path):
    isi = (SUMBER / "beranda.html").read_text(encoding="utf-8")
    isi = isi.replace("{VERSI}", VERSI).replace("{REPO}", REPO)
    isi = re.sub(r"^```.*?^```$(?:\s*^```keluaran.*?^```$)?", lambda m: markdown(m.group(0))[0],
                 isi, flags=re.S | re.M)
    badan = f'<main id="isi" class="beranda">\n{isi}\n{kaki("")}\n</main>'
    tulis(keluar / "index.html", halaman(
        "JawaRustGo++ — bahasa pemrograman",
        "JawaRustGo++ (JRG): bahasa pemrograman dengan pesan galat berbahasa Indonesia, "
        "tiga mesin eksekusi, dan kelas matematika dari nol.",
        "beranda", "", badan))


def bangun_profil(keluar: Path, cari: list):
    isi = (SUMBER / "profil.html").read_text(encoding="utf-8").replace("{REPO}", REPO)
    badan = f'<main id="isi" class="profil">\n{isi}\n{kaki("")}\n</main>'
    tulis(keluar / "profil.html", halaman(
        "Muhamad Surya Widianto — pembuat JawaRustGo++",
        "Profil Muhamad Surya Widianto, pembuat bahasa pemrograman JawaRustGo++ (JRG).",
        "profil", "", badan))
    cari.append({"j": "Profil pembuat", "u": "profil.html", "g": "Profil",
                 "k": "Muhamad Surya Widianto, pembuat JawaRustGo++", "t": re.sub(r"<[^>]+>", " ", isi)[:800]})


def main():
    keluar = DOCS
    if "--keluar" in sys.argv:
        keluar = Path(sys.argv[sys.argv.index("--keluar") + 1]).resolve()
    cari = []
    n_dok = bangun_dokumentasi(keluar, cari)
    n_mtk = bangun_matematika(keluar, cari)
    bangun_beranda(keluar)
    bangun_profil(keluar, cari)
    tulis(keluar / "aset" / "cari.js",
          "// Dibangun perkakas/bangun_situs.py.\nwindow.JRG_CARI = " + json.dumps(cari, ensure_ascii=False) + ";\n")
    print(f"dokumentasi {n_dok} halaman, matematika {n_mtk} halaman, {len(cari)} entri cari")


if __name__ == "__main__":
    main()
