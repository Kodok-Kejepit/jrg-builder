#!/usr/bin/env python3
"""Pemindai kelas "kunci dipegang, lalu nilai yang sama dikunci lagi"."""

import os, re, sys

# `Mutex` Rust tidak reentrant: mengunci Map yang sedang dipegang thread yang
# sama menggantung selamanya, tanpa pesan. Keluarannya KANDIDAT untuk dibaca,
# bukan vonis — penjelasan lengkap di dokumentasi/perkakas/kunci_ganda.md.

PENJAGA_LET = re.compile(
    r'let\s+(?:mut\s+)?(\w+)\s*=\s*([\w\.]+?)\.lock\(\)\s*'
    r'(?:\.unwrap\(\)|\.unwrap_or_else\(\|\w+\|\s*\w+\.into_inner\(\)\)|\.expect\([^)]*\))?\s*;')
# Temporer di scrutinee `match`/`if let`/`while let` hidup sampai bloknya selesai (edisi 2021).
PENJAGA_SCRUTINEE = re.compile(r'\b(match|if\s+let|while\s+let)\b.*?([\w\.]+?)\.lock\(\)')
POLA_LUAR = [
    re.compile(r'\bmatch\s+&?\(?\s*(\w+)'),
    re.compile(r'\bif\s+let\s+.*Map\(\s*(?:ref\s+)?\w+\s*\)\s*=\s*&?(\w+)'),
]


def nama_luar(baris, i, akar):
    """Nama nilai yang Map-nya dikunci: `match induk { VmValue::Map(m) => …`."""
    for k in range(i, max(-1, i - 8), -1):
        if re.search(r'Map\(\s*(?:ref\s+)?%s\s*\)' % re.escape(akar), baris[k]):
            for kk in range(k, max(-1, k - 40), -1):
                for pola in POLA_LUAR:
                    m = pola.search(baris[kk])
                    if m:
                        return m.group(1)
            return None
    return None


def pindai(jalur):
    temuan = []
    baris = open(jalur, encoding="utf-8").read().split("\n")
    for i, isi in enumerate(baris):
        kode = isi.split("//")[0]
        m = PENJAGA_LET.search(kode)
        if m:
            jenis, akar = "let " + m.group(1), m.group(2)
        else:
            m = PENJAGA_SCRUTINEE.search(kode)
            if not (m and kode.rstrip().endswith("{")):
                continue
            jenis, akar = m.group(1).split()[0], m.group(2)
        luar = nama_luar(baris, i, akar.split(".")[-1])
        kedalaman = 0
        for j in range(i, min(i + 120, len(baris))):
            s = baris[j].split("//")[0]
            if j > i:
                if jenis.startswith("let ") and re.search(r'\bdrop\(\s*%s\s*\)' % re.escape(jenis[4:]), s):
                    break
                alasan = None
                if re.search(r'\b%s\.lock\(\)' % re.escape(akar), s):
                    alasan = "dikunci lagi"
                elif luar and re.search(
                        r'\b{0}\s*\.\s*(type_name_penuh|to_string|lock)\b|\{{\}}.*\b{0}\b|[\(&,]\s*{0}\b'
                        .format(re.escape(luar)), s):
                    alasan = "`%s` dipakai" % luar
                if alasan:
                    temuan.append("%s:%d [%s atas `%s`] %s -> %d: %s"
                                  % (jalur, i + 1, jenis, akar, alasan, j + 1, s.strip()[:95]))
            kedalaman += s.count("{") - s.count("}")
            if j > i and kedalaman <= 0:
                break
    return temuan


def main():
    semua = []
    for akar, dirs, files in os.walk("src"):
        dirs[:] = [d for d in dirs if d != "target"]
        for f in sorted(files):
            if f.endswith(".rs"):
                semua += pindai(os.path.join(akar, f))
    print("%d kandidat" % len(semua))
    for t in semua:
        print(t)
    return 1 if semua else 0


if __name__ == "__main__":
    sys.exit(main())
