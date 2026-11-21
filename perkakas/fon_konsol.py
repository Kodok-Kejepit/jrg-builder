#!/usr/bin/env python3
"""Menurunkan src/bebas/fon_8x16.h dari fon konsol Lat15-Fixed16 (turunan X11 misc-fixed, domain publik).

Pakai: python3 perkakas/fon_konsol.py [berkas.psf.gz]  — menulis ulang src/bebas/fon_8x16.h.
"""
import gzip, os, sys

sumber = sys.argv[1] if len(sys.argv) > 1 else '/usr/share/consolefonts/Lat15-Fixed16.psf.gz'
d = gzip.open(sumber).read()
assert d[:2] == b'\x36\x04', 'harus PSF1'
mode, tinggi = d[2], d[3]
n = 512 if mode & 1 else 256
assert tinggi == 16 and mode & 2, 'harus 8x16 dengan tabel Unicode'
glyph = d[4:4 + n * 16]
peta = {}
i, g = 4 + n * 16, 0
while g < n:
    u = int.from_bytes(d[i:i + 2], 'little')
    i += 2
    if u == 0xFFFF:
        g += 1
    elif u == 0xFFFE:
        pass
    else:
        peta.setdefault(u, g)
baris = ['// fon_8x16.h — DIBUAT perkakas/fon_konsol.py dari Lat15-Fixed16 (X11 misc-fixed, domain publik).',
         '// Jangan disunting tangan: jalankan ulang generatornya.',
         'static const unsigned char FON_GLYPH[%d][16] = {' % n]
for k in range(n):
    baris.append('    {%s},' % ','.join('0x%02X' % b for b in glyph[k * 16:(k + 1) * 16]))
baris.append('};')
baris.append('static const struct { unsigned short kode; unsigned short glyph; } FON_PETA[%d] = {' % len(peta))
pas = sorted(peta.items())
for k in range(0, len(pas), 8):
    baris.append('    ' + ' '.join('{0x%04X,%d},' % p for p in pas[k:k + 8]))
baris.append('};')
akar = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
open(os.path.join(akar, 'src/bebas/fon_8x16.h'), 'w').write('\n'.join(baris) + '\n')
print('glyph', n, 'kode', len(peta))
