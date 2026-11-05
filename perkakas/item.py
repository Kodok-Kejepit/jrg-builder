#!/usr/bin/env python3
"""Bandingkan daftar item Rust sebelum dan sesudah sebuah berkas dipecah.

    python3 perkakas/item.py <asli.rs> <hasil1.rs> [hasil2.rs ...]

Yang dihitung: item tingkat atas (fn/struct/enum/trait/type/const/static/impl)
dan anggota `impl` pada kedalaman satu. Sebuah item yang HILANG berarti kode
yang lenyap tanpa suara — kompilasi tetap bisa hijau kalau yang hilang tidak
pernah dipanggil dari luar berkasnya, jadi kompilator bukan penjaga di sini.

Baris `TAMBAH` yang berisi `impl X` sebanyak jumlah berkas baru memang wajar:
tiap berkas hasil pecahan membuka blok `impl`-nya sendiri.
"""
import re, sys
def items(paths):
    out=[]
    for p in paths:
        for l in open(p):
            m=re.match(r'^\s*(?:pub(?:\(crate\))?\s+)?(?:#\[[^\]]*\]\s*)*(?:pub(?:\(crate\))?\s+)?(fn|struct|enum|trait|type|const|static|impl)\s+([A-Za-z_][A-Za-z0-9_]*)', l)
            if m and (l.startswith('    ') or not l.startswith(' ')):
                # hanya kedalaman 0 (item bebas) dan 4 (anggota impl)
                ind=len(l)-len(l.lstrip())
                if ind in (0,4): out.append(f'{m.group(1)} {m.group(2)}')
    return sorted(out)
a=items([sys.argv[1]]); b=items(sys.argv[2:])
from collections import Counter
ca,cb=Counter(a),Counter(b)
print(f'sebelum={len(a)} sesudah={len(b)}')
h=ca-cb; t=cb-ca
print('HILANG:', sorted(h.elements()) or 'tidak ada')
print('TAMBAH:', sorted(t.elements()) or 'tidak ada')
