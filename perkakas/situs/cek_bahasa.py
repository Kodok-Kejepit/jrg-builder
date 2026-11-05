#!/usr/bin/env python3
"""Periksa bahasa sumber situs: satu sapaan (`kamu`) dan ejaan yang sering meleset."""
import json, re, sys
from pathlib import Path

SUMBER = Path(__file__).resolve().parent.parent.parent / "docs" / "_sumber"

EJAAN = [
    ("dimana", "di mana"), ("resiko", "risiko"), ("praktek", "praktik"),
    ("analisa", "analisis"), ("tehnik", "teknik"), ("silahkan", "silakan"),
    ("merubah", "mengubah"), ("nomer", "nomor"), ("jaman", "zaman"),
    ("frekwensi", "frekuensi"), ("hutang", "utang"), ("propinsi", "provinsi"),
    ("aktifitas", "aktivitas"), ("kwalitas", "kualitas"), ("obyek", "objek"),
    ("sistim", "sistem"), ("standard", "standar"), ("ijin", "izin"),
    ("nasehat", "nasihat"), ("faham", "paham"),
]


def prosa(teks):
    teks = re.sub(r"```.*?```", " ", teks, flags=re.S)
    teks = re.sub(r"`[^`]*`|<code>.*?</code>|<pre.*?</pre>", " ", teks, flags=re.S)
    return re.sub(r"<[^>]+>|&[a-z]+;", " ", teks)


def main():
    bagian = {}
    for p in sorted((SUMBER / "dokumentasi").glob("*.md")):
        bagian[f"dokumentasi/{p.name}"] = prosa(p.read_text(encoding="utf-8"))
    for nama in ("beranda.html", "kelas-pembuka.html"):
        bagian[nama] = prosa((SUMBER / nama).read_text(encoding="utf-8"))
    data = json.loads((SUMBER / "matematika.json").read_text(encoding="utf-8"))
    for bab in data["content"]:
        teks = []
        for b in bab.get("blocks", []):
            b = {k: v for k, v in b.items() if k not in ("code", "output")}
            teks.append(json.dumps(b, ensure_ascii=False))
        bagian[f"matematika/{bab['id']}"] = prosa(" ".join(teks) + " " + bab.get("title", "") + " " + bab.get("subtitle", ""))
    salah = []
    kata = n_kamu = 0
    for nama, t in bagian.items():
        kata += len(t.split())
        n_kamu += len(re.findall(r"\b[Kk]amu\b", t))
        n = len(re.findall(r"\bAnda\b", t))
        if n:
            salah.append(f"{nama}: {n}x 'Anda' (situs ini memakai 'kamu')")
        for a, b in EJAAN:
            n = len(re.findall(r"\b" + a + r"\b", t, re.I))
            if n:
                salah.append(f"{nama}: {n}x {a!r}, seharusnya {b!r}")
    print(f"  {len(bagian)} bagian, {kata} kata, 'kamu' {n_kamu}x")
    if n_kamu == 0:
        salah.append("'kamu' tidak muncul sama sekali; pemeriksa ini mungkin membaca berkas yang salah")
    if salah:
        print(f"\n{len(salah)} MASALAH BAHASA:")
        for s in salah:
            print("  -", s)
        return 1
    print("BAHASA BERSIH")
    return 0


if __name__ == "__main__":
    sys.exit(main())
