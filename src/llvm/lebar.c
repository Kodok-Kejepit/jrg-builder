// lebar.c — lebar tampil satu karakter (`term::lebar`) untuk runtime `--emit-llvm`; tabelnya `lebar_tabel.h`,
// dirangkai tepat sebelum berkas ini. Aturannya sama dengan `src/lebar.rs`.
static int jrg_lebar_di(const uint32_t (*t)[2], int64_t n, uint32_t cp) {
    int64_t lo = 0, hi = n - 1;
    while (lo <= hi) {
        int64_t m = (lo + hi) / 2;
        if (cp < t[m][0]) hi = m - 1; else if (cp > t[m][1]) lo = m + 1; else return 1;
    }
    return 0;
}
int64_t jrg_lebar_kode(uint32_t cp) {
    if (cp >= 0x20 && cp < 0x7f) return 1;
    if (cp < 0x300) return (cp < 0x20 || (cp >= 0x7f && cp < 0xa0)) ? 0 : 1;
    if (jrg_lebar_di(JRG_LEBAR_NOL, (int64_t)(sizeof JRG_LEBAR_NOL / sizeof JRG_LEBAR_NOL[0]), cp)) return 0;
    if (jrg_lebar_di(JRG_LEBAR_DUA, (int64_t)(sizeof JRG_LEBAR_DUA / sizeof JRG_LEBAR_DUA[0]), cp)) return 2;
    return 1;
}
