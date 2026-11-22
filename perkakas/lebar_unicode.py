#!/usr/bin/env python3
# perkakas/lebar_unicode.py — tabel lebar tampil `src/lebar/tabel.rs`, dan salinan C-nya `src/llvm/lebar_tabel.h`.
#   python3 perkakas/lebar_unicode.py [--c]      tulis ulang (`--c`: salinan C saja, diturunkan dari berkas Rust)
#   python3 perkakas/lebar_unicode.py --periksa  bandingkan saja: 0 sama, 1 beda, 3 versi Unicode lain
import sys
import unicodedata as u

KELUAR = "src/lebar/tabel.rs"
KELUAR_C = "src/llvm/lebar_tabel.h"


def ke_c(rs):
    """Tabel C dari isi `tabel.rs`: rentang yang sama persis, urutan yang sama."""
    import re
    keluar = ["// src/llvm/lebar_tabel.h — DITURUNKAN `perkakas/lebar_unicode.py` dari src/lebar/tabel.rs.",
              "// Jangan disunting tangan: jalankan ulang skripnya."]
    for nama in ("NOL", "DUA"):
        isi = rs.split("pub static %s:" % nama)[1].split("];")[0]
        pas = re.findall(r"\((0x[0-9A-F]+), (0x[0-9A-F]+)\)", isi)
        keluar.append("static const uint32_t JRG_LEBAR_%s[][2] = {" % nama)
        for i in range(0, len(pas), 4):
            keluar.append("    " + " ".join("{%s, %s}," % p for p in pas[i:i + 4]))
        keluar.append("};")
    return "\n".join(keluar) + "\n"


def lebar(cp):
    """Kolom layar satu karakter: 0, 1 atau 2 (aturan `wcwidth` yang umum dipakai terminal)."""
    c = chr(cp)
    kat = u.category(c)
    # Kendali, pengganti (surrogate), pemisah baris/paragraf: tidak punya lebar tetap.
    if kat in ("Cc", "Cs", "Zl", "Zp"):
        return 0
    # Tanda hubung lunak tampil sebagai satu kolom walau berkategori Cf.
    if cp == 0x00AD:
        return 1
    # Tanda gabung dan karakter format menumpang di kolom huruf sebelumnya.
    if kat in ("Mn", "Me", "Cf"):
        return 0
    # Vokal dan konsonan akhir Jamo Hangul menyatu dengan konsonan awalnya.
    if 0x1160 <= cp <= 0x11FF:
        return 0
    if u.east_asian_width(c) in ("W", "F"):
        return 2
    return 1


def rentang(nilai):
    """Daftar (awal, akhir) inklusif dari semua titik kode yang lebarnya `nilai`."""
    hasil = []
    awal = None
    for cp in range(0x110000):
        kena = lebar(cp) == nilai
        if kena and awal is None:
            awal = cp
        elif not kena and awal is not None:
            hasil.append((awal, cp - 1))
            awal = None
    if awal is not None:
        hasil.append((awal, 0x10FFFF))
    return hasil


def tabel(nama, isi):
    baris = ["pub static %s: &[(u32, u32)] = &[" % nama]
    for i in range(0, len(isi), 4):
        potong = isi[i:i + 4]
        baris.append("    " + " ".join("(0x%04X, 0x%04X)," % p for p in potong))
    baris.append("];")
    return "\n".join(baris)


def bangun():
    return "\n".join([
        "// src/lebar/tabel.rs — DIBANGKITKAN `perkakas/lebar_unicode.py` dari `unicodedata`",
        "// milik Python. Jangan disunting tangan: jalankan ulang skripnya.",
        "",
        'pub const VERSI_UNICODE: &str = "%s";' % u.unidata_version,
        "",
        tabel("NOL", rentang(0)),
        "",
        tabel("DUA", rentang(2)),
        "",
    ])


def versi_tercatat():
    try:
        for b in open(KELUAR, encoding="utf-8"):
            if b.startswith("pub const VERSI_UNICODE"):
                return b.split('"')[1]
    except OSError:
        pass
    return None


def main():
    if "--periksa" in sys.argv:
        tercatat = versi_tercatat()
        if tercatat != u.unidata_version:
            print("versi Unicode beda: tabel %s, python %s" % (tercatat, u.unidata_version))
            return 3
        lama = open(KELUAR, encoding="utf-8").read()
        try:
            c_lama = open(KELUAR_C, encoding="utf-8").read()
        except OSError:
            c_lama = None
        if c_lama != ke_c(lama):
            print("%s tidak cocok dengan %s" % (KELUAR_C, KELUAR))
            return 1
        if lama == bangun():
            print("tabel lebar cocok dengan unicodedata %s" % u.unidata_version)
            return 0
        print("tabel lebar TIDAK cocok dengan unicodedata %s" % u.unidata_version)
        return 1
    if "--c" not in sys.argv:
        open(KELUAR, "w", encoding="utf-8").write(bangun())
        print("%s ditulis (Unicode %s)" % (KELUAR, u.unidata_version))
    open(KELUAR_C, "w", encoding="utf-8").write(ke_c(open(KELUAR, encoding="utf-8").read()))
    print("%s ditulis" % KELUAR_C)
    return 0


if __name__ == "__main__":
    sys.exit(main())
