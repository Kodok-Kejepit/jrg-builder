#!/usr/bin/env python3
"""Periksa situs docs/ hasil bangun: tautan, ukuran, kerangka halaman, bahasa, tata letak, dan
bahwa isinya memang hasil perkakas/bangun_situs.py (suntingan tangan di docs/ hilang saat dibangun ulang)."""
import re, subprocess, sys, tempfile
from pathlib import Path

DOCS = Path(__file__).resolve().parent.parent / "docs"
salah = []

halaman = sorted(p for p in DOCS.rglob("*.html") if "_sumber" not in p.parts)
print(f"{len(halaman)} halaman diperiksa")

for p in halaman:
    t = p.read_text(encoding="utf-8")
    rel = p.relative_to(DOCS)
    for m in re.finditer(r'(?:href|src)="([^"#:]+)(?:#[^"]*)?"', t):
        tuj = m.group(1).split("?")[0]
        if tuj.startswith(("http", "//", "data:", "mailto:")):
            continue
        if not (p.parent / tuj).resolve().exists():
            salah.append(f"{rel}: tautan mati -> {tuj}")
    for m in re.finditer(r'href="([\w-]+\.html)#([\w-]+)"', t):
        tuj = (p.parent / m.group(1))
        if tuj.exists() and f'id="{m.group(2)}"' not in tuj.read_text(encoding="utf-8"):
            salah.append(f"{rel}: jangkar mati -> {m.group(1)}#{m.group(2)}")
    kb = len(t.encode()) // 1024
    if kb > 60:
        salah.append(f"{rel}: {kb} KB, terlalu besar untuk satu halaman")
    for wajib, ket in (('class="kepala"', "kepala situs"), ('id="isi"', "isi utama"), ("--kertas", "gaya")):
        if wajib not in t:
            salah.append(f"{rel}: tidak punya {ket}")

for grup in ("web_dokumentasi", "matematika_x_jrg"):
    hal = [p for p in halaman if p.relative_to(DOCS).parts[0] == grup]
    besar = max(len(p.read_text(encoding="utf-8").encode()) for p in hal) // 1024 if hal else 0
    print(f"  {grup:20} {len(hal):>3} halaman, terbesar {besar} KB")

for alat in ("cek_bahasa.py", "ukur_tata_letak.py"):
    r = subprocess.run([sys.executable, str(Path(__file__).parent / "situs" / alat)], capture_output=True, text=True)
    print("\n" + r.stdout.strip())
    if r.returncode != 0:
        salah.append(f"{alat} gagal")

with tempfile.TemporaryDirectory() as tmp:
    r = subprocess.run([sys.executable, str(Path(__file__).parent / "bangun_situs.py"), "--keluar", tmp],
                       capture_output=True, text=True)
    if r.returncode != 0:
        salah.append("generator gagal jalan: " + (r.stderr.strip().splitlines() or [""])[-1])
    else:
        dibandingkan = 0
        for q in sorted(Path(tmp).rglob("*")):
            if not q.is_file():
                continue
            rel = q.relative_to(tmp)
            dibandingkan += 1
            asli = DOCS / rel
            if not asli.exists():
                salah.append(f"{rel}: dihasilkan generator tapi tidak ada di docs/")
            elif asli.read_bytes() != q.read_bytes():
                salah.append(f"{rel}: beda dari hasil generator; sunting sumbernya di docs/_sumber, lalu bangun ulang")
        dihasilkan = {q.relative_to(tmp) for q in Path(tmp).rglob("*.html")}
        for p in halaman:
            if p.relative_to(DOCS) not in dihasilkan:
                salah.append(f"{p.relative_to(DOCS)}: tidak dihasilkan generator (sisa lama?)")
        if dibandingkan < 60:
            salah.append(f"cuma {dibandingkan} berkas dibandingkan; generatornya berhenti menulis sebagian situs?")
        print(f"\ngenerator: {dibandingkan} berkas dibangun ulang dan dibandingkan")

if salah:
    print(f"\n{len(salah)} MASALAH:")
    for s in salah[:40]:
        print("  -", s)
    sys.exit(1)
print("\nSITUS BERSIH")
