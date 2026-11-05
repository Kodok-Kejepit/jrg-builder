#!/usr/bin/env python3
"""Jalankan tiap contoh kode situs (docs/_sumber) lewat interpreter dan VM.

Blok ```jrg di halaman dokumentasi dan blok kode di kelas matematika harus jalan. Kalau ada blok
keluaran sesudahnya, isinya harus sama persis dengan yang dicetak jrgc (stdout lalu stderr,
seperti terlihat di terminal), dan VM harus mencetak hal yang sama. ```jrg-cuplikan tidak dijalankan.

  python3 perkakas/cek_contoh_dokumentasi.py             periksa
  python3 perkakas/cek_contoh_dokumentasi.py --perbarui  tulis ulang keluaran dari jrgc
  python3 perkakas/cek_contoh_dokumentasi.py nilai       cuma halaman yang namanya memuat `nilai`
"""
import json, os, re, subprocess, sys, tempfile
from pathlib import Path

AKAR = Path(__file__).resolve().parent.parent
JRGC = str(AKAR / "target" / "release" / "jrgc")
SUMBER = AKAR / "docs" / "_sumber"
PERBARUI = "--perbarui" in sys.argv
SARING = [a for a in sys.argv[1:] if not a.startswith("--")]
PAGAR = re.compile(r"^```jrg(?P<cup>-cuplikan)?[ \t]*(?P<nama>[^\s]*)(?:[ \t]+lokal)?[ \t]*\n(?P<kode>.*?)^```[ \t]*\n"
                   r"(?:\s*^```keluaran[ \t]*\n(?P<keluar>.*?)^```[ \t]*$)?", re.S | re.M)
ENV = {k: v for k, v in os.environ.items() if k not in ("JRG_SOPAN", "JRG_NADA", "JRG_ULANGAN", "DISPLAY", "WAYLAND_DISPLAY")}


def jalankan(kode, nama, vm):
    with tempfile.TemporaryDirectory() as d:
        (Path(d) / nama).write_text(kode, encoding="utf-8")
        cmd = [JRGC, nama, "--sopan"] + (["--vm"] if vm else [])
        try:
            r = subprocess.run(cmd, cwd=d, env=ENV, capture_output=True, text=True, timeout=30,
                               stdin=subprocess.DEVNULL)
        except subprocess.TimeoutExpired:
            return None, "(batas waktu)"
        return r.returncode, (r.stdout + r.stderr).rstrip("\n")


def periksa(kode, nama, harapan, asal):
    rc, dapat = jalankan(kode, nama, False)
    rc_vm, dapat_vm = jalankan(kode, nama, True)
    masalah = []
    if harapan is None and rc != 0:
        masalah.append(f"gagal (rc={rc}) padahal tidak ada blok keluaran:\n{dapat[-800:]}")
    elif harapan is not None and dapat != harapan.rstrip("\n") and not PERBARUI:
        masalah.append(f"keluaran beda\n--- tertulis\n{harapan.rstrip()}\n--- jrgc\n{dapat}")
    if dapat_vm != dapat and rc_vm is not None:
        masalah.append(f"VM beda dari interpreter\n--- interpreter\n{dapat}\n--- vm\n{dapat_vm}")
    for m in masalah:
        print(f"GAGAL {asal}: {m}\n")
    return dapat, len(masalah)


def main():
    jumlah = gagal = 0
    for p in sorted((SUMBER / "dokumentasi").glob("*.md")) + [SUMBER / "beranda.html"]:
        if SARING and not any(s in p.stem for s in SARING):
            continue
        teks = p.read_text(encoding="utf-8")
        baru, geser = teks, 0
        for m in PAGAR.finditer(teks):
            if m.group("cup"):
                continue
            jumlah += 1
            nama = m.group("nama") or "contoh.jrg"
            baris = teks[:m.start()].count("\n") + 1
            dapat, n = periksa(m.group("kode"), nama, m.group("keluar"), f"{p.relative_to(AKAR)}:{baris}")
            gagal += n
            if PERBARUI and m.group("keluar") is not None and dapat != m.group("keluar").rstrip("\n"):
                a, b = m.start("keluar") + geser, m.end("keluar") + geser
                baru = baru[:a] + dapat + "\n" + baru[b:]
                geser += len(dapat) + 1 - (m.end("keluar") - m.start("keluar"))
        if PERBARUI and baru != teks:
            p.write_text(baru, encoding="utf-8")
            print(f"diperbarui: {p.relative_to(AKAR)}")
    if not SARING or any(s in "matematika" for s in SARING):
        pj = SUMBER / "matematika.json"
        data = json.loads(pj.read_text(encoding="utf-8"))
        ubah = False
        for bab in data["content"]:
            for b in bab.get("blocks", []):
                if b.get("type") != "code":
                    continue
                jumlah += 1
                nama = b.get("filename") or "contoh.jrg"
                dapat, n = periksa(b["code"], nama, b.get("output"), f"matematika {bab['id']}/{nama}")
                gagal += n
                if PERBARUI and b.get("output") is not None and dapat != b["output"].rstrip("\n"):
                    b["output"] = dapat
                    ubah = True
        if ubah:
            pj.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
            print("diperbarui: docs/_sumber/matematika.json")
    print(f"{jumlah} contoh dijalankan, {gagal} masalah")
    if not SARING and jumlah < 150:
        print("GAGAL: contohnya terlalu sedikit; pemindaiannya mungkin salah membaca sumber")
        return 1
    return 1 if gagal else 0


if __name__ == "__main__":
    sys.exit(main())
