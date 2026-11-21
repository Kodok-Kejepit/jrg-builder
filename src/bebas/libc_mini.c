// libc_mini.c — "libc mini" untuk biner JRG tanpa sistem operasi (x86_64, `jrgc --bebas`).
// Keluaran: COM1 = stdout (juga layar VGA), COM2 = stderr. Keluar: port isa-debug-exit QEMU, lalu `hlt`.
#include <stddef.h>
#include <stdint.h>
#include <stdarg.h>

// ── port I/O ─────────────────────────────────────────────────────────────
static inline void keluar8(uint16_t p, uint8_t v) { __asm__ volatile("outb %0, %1" :: "a"(v), "Nd"(p)); }
static inline uint8_t masuk8(uint16_t p) { uint8_t v; __asm__ volatile("inb %1, %0" : "=a"(v) : "Nd"(p)); return v; }
static inline void keluar32(uint16_t p, uint32_t v) { __asm__ volatile("outl %0, %1" :: "a"(v), "Nd"(p)); }

// ── serial ───────────────────────────────────────────────────────────────
static void serial_pasang(uint16_t b) {
    keluar8(b + 1, 0x00); keluar8(b + 3, 0x80); keluar8(b + 0, 0x01); keluar8(b + 1, 0x00);
    keluar8(b + 3, 0x03); keluar8(b + 4, 0x0B);  /* FIFO tidak disentuh: mengaturnya membuang byte masukan yang sudah tiba */
}
static void serial_tulis(uint16_t b, char c) {
    while (!(masuk8(b + 5) & 0x20)) {}
    keluar8(b, (uint8_t)c);
}

// ── layar VGA teks 80x25 ─────────────────────────────────────────────────
static volatile uint16_t *const VGA = (volatile uint16_t *)0xB8000;
static int vga_x, vga_y;
/* Konsol framebuffer (mesin UEFI: tidak ada layar teks VGA): glyph 8x16 dari `fon_8x16.h`, latar hitam. */
static volatile uint32_t *fb;
static uint32_t fb_lebar, fb_tinggi, fb_pitch, fb_kol, fb_bar, fb_x, fb_y;
static int glyph_untuk(uint32_t kode) {
    int lo = 0, hi = (int)(sizeof FON_PETA / sizeof FON_PETA[0]) - 1;
    while (lo <= hi) {
        int m = (lo + hi) / 2;
        if (FON_PETA[m].kode == kode) return FON_PETA[m].glyph;
        if (FON_PETA[m].kode < kode) lo = m + 1; else hi = m - 1;
    }
    return kode == '?' ? 0 : glyph_untuk('?');
}
static void fb_gulir(void) {
    uint32_t baris = fb_pitch / 4 * 16;
    volatile uint32_t *a = fb;
    for (uint32_t i = 0; i < (fb_bar - 1) * baris; i++) a[i] = a[i + baris];
    for (uint32_t i = (fb_bar - 1) * baris; i < fb_bar * baris; i++) a[i] = 0;
}
static void fb_tulis(uint32_t kode) {
    if (kode == '\n') { fb_x = 0; fb_y++; }
    else if (kode == '\r') fb_x = 0;
    else if (kode == '\b') { if (fb_x) fb_x--; kode = ' '; fb_tulis(kode); fb_x--; return; }
    else {
        const unsigned char *g = FON_GLYPH[glyph_untuk(kode)];
        for (uint32_t y = 0; y < 16; y++)
            for (uint32_t x = 0; x < 8; x++)
                fb[(fb_y * 16 + y) * (fb_pitch / 4) + fb_x * 8 + x] = (g[y] >> (7 - x)) & 1 ? 0xC0C0C0 : 0;
        if (++fb_x == fb_kol) { fb_x = 0; fb_y++; }
    }
    if (fb_y == fb_bar) { fb_gulir(); fb_y = fb_bar - 1; }
}

/* Layar teks VGA memakai CP437: UTF-8 didekode, kode yang punya padanan CP437 dipetakan, sisanya `?`. */
static const struct { uint16_t u; uint8_t b; } CP437[] = {
    {0x00A0, 0xFF}, {0x00A1, 0xAD}, {0x00A2, 0x9B}, {0x00A3, 0x9C}, {0x00A5, 0x9D}, {0x00AA, 0xA6},
    {0x00AB, 0xAE}, {0x00AC, 0xAA}, {0x00B0, 0xF8}, {0x00B1, 0xF1}, {0x00B2, 0xFD}, {0x00B5, 0xE6},
    {0x00B7, 0xFA}, {0x00BA, 0xA7}, {0x00BB, 0xAF}, {0x00BC, 0xAC}, {0x00BD, 0xAB}, {0x00BF, 0xA8},
    {0x00C4, 0x8E}, {0x00C5, 0x8F}, {0x00C6, 0x92}, {0x00C7, 0x80}, {0x00C9, 0x90}, {0x00D1, 0xA5},
    {0x00D6, 0x99}, {0x00DC, 0x9A}, {0x00DF, 0xE1}, {0x00E0, 0x85}, {0x00E1, 0xA0}, {0x00E2, 0x83},
    {0x00E4, 0x84}, {0x00E5, 0x86}, {0x00E6, 0x91}, {0x00E7, 0x87}, {0x00E8, 0x8A}, {0x00E9, 0x82},
    {0x00EA, 0x88}, {0x00EB, 0x89}, {0x00EC, 0x8D}, {0x00ED, 0xA1}, {0x00EE, 0x8C}, {0x00EF, 0x8B},
    {0x00F1, 0xA4}, {0x00F2, 0x95}, {0x00F3, 0xA2}, {0x00F4, 0x93}, {0x00F6, 0x94}, {0x00F7, 0xF6},
    {0x00F9, 0x97}, {0x00FA, 0xA3}, {0x00FB, 0x96}, {0x00FC, 0x81}, {0x00FF, 0x98}, {0x0192, 0x9F},
    {0x0393, 0xE2}, {0x0398, 0xE9}, {0x03A3, 0xE4}, {0x03A6, 0xE8}, {0x03A9, 0xEA}, {0x03B1, 0xE0},
    {0x03B4, 0xEB}, {0x03B5, 0xEE}, {0x03C0, 0xE3}, {0x03C3, 0xE5}, {0x03C4, 0xE7}, {0x03C6, 0xED},
    {0x207F, 0xFC}, {0x20A7, 0x9E}, {0x2219, 0xF9}, {0x221A, 0xFB}, {0x221E, 0xEC}, {0x2229, 0xEF},
    {0x2248, 0xF7}, {0x2261, 0xF0}, {0x2264, 0xF3}, {0x2265, 0xF2}, {0x2310, 0xA9}, {0x2320, 0xF4},
    {0x2321, 0xF5}, {0x2500, 0xC4}, {0x2502, 0xB3}, {0x250C, 0xDA}, {0x2510, 0xBF}, {0x2514, 0xC0},
    {0x2518, 0xD9}, {0x251C, 0xC3}, {0x2524, 0xB4}, {0x252C, 0xC2}, {0x2534, 0xC1}, {0x253C, 0xC5},
    {0x2550, 0xCD}, {0x2551, 0xBA}, {0x2552, 0xD5}, {0x2553, 0xD6}, {0x2554, 0xC9}, {0x2555, 0xB8},
    {0x2556, 0xB7}, {0x2557, 0xBB}, {0x2558, 0xD4}, {0x2559, 0xD3}, {0x255A, 0xC8}, {0x255B, 0xBE},
    {0x255C, 0xBD}, {0x255D, 0xBC}, {0x255E, 0xC6}, {0x255F, 0xC7}, {0x2560, 0xCC}, {0x2561, 0xB5},
    {0x2562, 0xB6}, {0x2563, 0xB9}, {0x2564, 0xD1}, {0x2565, 0xD2}, {0x2566, 0xCB}, {0x2567, 0xCF},
    {0x2568, 0xD0}, {0x2569, 0xCA}, {0x256A, 0xD8}, {0x256B, 0xD7}, {0x256C, 0xCE}, {0x2580, 0xDF},
    {0x2584, 0xDC}, {0x2588, 0xDB}, {0x258C, 0xDD}, {0x2590, 0xDE}, {0x2591, 0xB0}, {0x2592, 0xB1},
    {0x2593, 0xB2}, {0x25A0, 0xFE}
};
static uint32_t utf_kode; static int utf_sisa;
static void vga_byte(char c);
static void vga_tulis(char c) {
    unsigned char b = (unsigned char)c;
    if (fb) {
        if (b < 0x80) { utf_sisa = 0; fb_tulis(b); return; }
        if ((b & 0xC0) == 0x80) {
            if (!utf_sisa) { fb_tulis('?'); return; }
            utf_kode = (utf_kode << 6) | (b & 0x3F);
            if (!--utf_sisa) fb_tulis(utf_kode);
            return;
        }
        utf_sisa = b >= 0xF0 ? 3 : b >= 0xE0 ? 2 : 1;
        utf_kode = b & (b >= 0xF0 ? 0x07 : b >= 0xE0 ? 0x0F : 0x1F);
        return;
    }
    if (b < 0x80) { utf_sisa = 0; vga_byte(c); return; }
    if ((b & 0xC0) == 0x80) {
        if (!utf_sisa) { vga_byte('?'); return; }
        utf_kode = (utf_kode << 6) | (b & 0x3F);
        if (--utf_sisa) return;
        char k = '?';
        if (utf_kode == 0x2014 || utf_kode == 0x2013) k = '-';
        for (size_t i = 0; i < sizeof CP437 / sizeof CP437[0]; i++) if (CP437[i].u == utf_kode) { k = (char)CP437[i].b; break; }
        vga_byte(k);
        return;
    }
    utf_sisa = b >= 0xF0 ? 3 : b >= 0xE0 ? 2 : 1;
    utf_kode = b & (b >= 0xF0 ? 0x07 : b >= 0xE0 ? 0x0F : 0x1F);
}
static void vga_byte(char c) {
    if (c == '\n') { vga_x = 0; vga_y++; }
    else if (c == '\r') { vga_x = 0; }
    else { VGA[vga_y * 80 + vga_x] = (uint16_t)(uint8_t)c | 0x0700; if (++vga_x == 80) { vga_x = 0; vga_y++; } }
    if (vga_y == 25) {
        for (int i = 0; i < 80 * 24; i++) VGA[i] = VGA[i + 80];
        for (int i = 80 * 24; i < 80 * 25; i++) VGA[i] = 0x0720;
        vga_y = 24;
    }
}

// ── memori dasar ─────────────────────────────────────────────────────────
void *memcpy(void *d, const void *s, size_t n) { char *a = d; const char *b = s; while (n--) *a++ = *b++; return d; }
void *memmove(void *d, const void *s, size_t n) {
    char *a = d; const char *b = s;
    if (a < b) { while (n--) *a++ = *b++; } else { a += n; b += n; while (n--) *--a = *--b; }
    return d;
}
void *memset(void *d, int c, size_t n) { char *a = d; while (n--) *a++ = (char)c; return d; }
int memcmp(const void *x, const void *y, size_t n) {
    const unsigned char *a = x, *b = y;
    for (; n; n--, a++, b++) if (*a != *b) return *a - *b;
    return 0;
}
int bcmp(const void *x, const void *y, size_t n) { return memcmp(x, y, n); }
void *memchr(const void *s, int c, size_t n) {
    const unsigned char *a = s;
    for (; n; n--, a++) if (*a == (unsigned char)c) return (void *)a;
    return 0;
}
size_t strlen(const char *s) { size_t n = 0; while (s[n]) n++; return n; }
int strcmp(const char *a, const char *b) { while (*a && *a == *b) { a++; b++; } return (unsigned char)*a - (unsigned char)*b; }
int strncmp(const char *a, const char *b, size_t n) {
    for (; n; n--, a++, b++) { if (*a != *b || !*a) return (unsigned char)*a - (unsigned char)*b; }
    return 0;
}
char *strchr(const char *s, int c) { for (;; s++) { if (*s == (char)c) return (char *)s; if (!*s) return 0; } }
char *strrchr(const char *s, int c) { const char *r = 0; for (;; s++) { if (*s == (char)c) r = s; if (!*s) return (char *)r; } }
char *strncpy(char *d, const char *s, size_t n) { size_t i = 0; for (; i < n && s[i]; i++) d[i] = s[i]; for (; i < n; i++) d[i] = 0; return d; }

// ── heap: kelas ukuran pangkat dua, daftar bebas per kelas ─────────────────
extern char __heap_awal[];
static char *heap_kini, *heap_akhir;
typedef struct Blok { struct Blok *lanjut; } Blok;
static Blok *bebas_kelas[48];
typedef struct { uint64_t kelas, _isi; } Kepala;   // 16 byte, data tetap rata 16

static int kelas_untuk(size_t n) { int k = 4; while (((size_t)1 << k) < n + sizeof(Kepala)) k++; return k; }
void exit(int);
/* Penangan interupsi boleh mengalokasi: heap dijaga dengan menutup interupsi sebentar. */
static inline uint64_t tutup_irq(void) { uint64_t f; __asm__ volatile("pushfq; popq %0; cli" : "=r"(f) :: "memory"); return f; }
static inline void buka_irq(uint64_t f) { if (f & 0x200) __asm__ volatile("sti" ::: "memory"); }
static void *malloc_dalam(size_t n);
void *malloc(size_t n) { uint64_t f = tutup_irq(); void *p = malloc_dalam(n); buka_irq(f); return p; }
static void *malloc_dalam(size_t n) {
    if (n > ((size_t)1 << 40)) return 0;
    int k = kelas_untuk(n);
    Kepala *h;
    if (bebas_kelas[k]) { h = (Kepala *)bebas_kelas[k]; bebas_kelas[k] = bebas_kelas[k]->lanjut; }
    else {
        size_t ukuran = (size_t)1 << k;
        if ((size_t)(heap_akhir - heap_kini) < ukuran) return 0;
        h = (Kepala *)heap_kini; heap_kini += ukuran;
    }
    h->kelas = (uint64_t)k;
    return h + 1;
}
void free(void *p) {
    if (!p) return;
    uint64_t f = tutup_irq();
    Kepala *h = (Kepala *)p - 1;
    Blok *b = (Blok *)h;
    b->lanjut = bebas_kelas[h->kelas];
    bebas_kelas[h->kelas] = b;
    buka_irq(f);
}
void *calloc(size_t a, size_t b) {
    if (b && a > (size_t)-1 / b) return 0;
    void *p = malloc(a * b);
    if (p) memset(p, 0, a * b);
    return p;
}
void *realloc(void *p, size_t n) {
    if (!p) return malloc(n);
    Kepala *h = (Kepala *)p - 1;
    size_t muat = ((size_t)1 << h->kelas) - sizeof(Kepala);
    if (n <= muat) return p;
    void *q = malloc(n);
    if (!q) return 0;
    memcpy(q, p, muat);
    free(p);
    return q;
}
int posix_memalign(void **r, size_t al, size_t n) {
    void *p = malloc(n + al);
    if (!p) return 12;
    *r = (void *)(((uintptr_t)p + al - 1) & ~(uintptr_t)(al - 1));
    return 0;
}
// ── peta halaman 4 KiB di atas 4 GiB: stack korutin dengan halaman penjaga sungguhan ─────────────────
extern uint64_t pml4[], pdpt[];
#define WILAYAH_AWAL 0x100000000ULL        /* PDPT[4..]: di atas peta identitas 4 GiB */
static uint64_t wilayah_kini = WILAYAH_AWAL;
static uint64_t *halaman_bebas;            /* bingkai 4 KiB yang dikembalikan munmap */

static void *bingkai_baru(void) {
    if (halaman_bebas) { uint64_t *h = halaman_bebas; halaman_bebas = (uint64_t *)*h; memset(h, 0, 4096); return h; }
    uintptr_t a = ((uintptr_t)heap_kini + 4095) & ~(uintptr_t)4095;
    if (a + 4096 > (uintptr_t)heap_akhir) return 0;
    heap_kini = (char *)(a + 4096);
    memset((void *)a, 0, 4096);
    return (void *)a;
}
/* Entri tabel tingkat berikutnya, dibuat kalau belum ada. */
static uint64_t *tabel_anak(uint64_t *t, unsigned i) {
    if (!(t[i] & 1)) { void *b = bingkai_baru(); if (!b) return 0; t[i] = (uint64_t)(uintptr_t)b | 3; }
    return (uint64_t *)(uintptr_t)(t[i] & 0x000FFFFFFFFFF000ULL);
}
static uint64_t *pte_untuk(uint64_t v, int buat) {
    uint64_t *pd = (pdpt[(v >> 30) & 511] & 1) || buat ? tabel_anak(pdpt, (v >> 30) & 511) : 0;
    if (!pd) return 0;
    uint64_t *pt = (pd[(v >> 21) & 511] & 1) || buat ? tabel_anak(pd, (v >> 21) & 511) : 0;
    return pt ? &pt[(v >> 12) & 511] : 0;
}
static void lepas_halaman(uint64_t v) {
    uint64_t *e = pte_untuk(v, 0);
    if (!e || !(*e & 1)) return;
    uint64_t *f = (uint64_t *)(uintptr_t)(*e & 0x000FFFFFFFFFF000ULL);
    *e = 0;
    __asm__ volatile("invlpg (%0)" :: "r"(v) : "memory");
    *f = (uint64_t)(uintptr_t)halaman_bebas; halaman_bebas = f;
}
void *mmap(void *a, size_t n, int prot, int fl, int fd, long off) {
    (void)a; (void)prot; (void)fl; (void)fd; (void)off;
    n = (n + 4095) & ~(size_t)4095;
    uint64_t v = wilayah_kini;
    if (v + n + 4096 > (512ULL << 30)) return (void *)-1;
    for (uint64_t p = 0; p < n; p += 4096) {
        uint64_t *e = pte_untuk(v + p, 1);
        void *b = e ? bingkai_baru() : 0;
        if (!b) { for (uint64_t q = 0; q < p; q += 4096) lepas_halaman(v + q); return (void *)-1; }
        *e = (uint64_t)(uintptr_t)b | 3;
    }
    wilayah_kini += n + 4096;              /* satu halaman tak terpeta di antara dua pemetaan */
    return (void *)(uintptr_t)v;
}
/* Halaman penjaga stack korutin, dicatat persis: page fault di sana = stack habis, di tempat lain = galat biasa. */
static uint64_t penjaga_awal[4096], penjaga_ukuran[4096];
static int penjaga_n;
int munmap(void *a, size_t n) {
    for (uint64_t p = 0; p < n; p += 4096) lepas_halaman((uint64_t)(uintptr_t)a + p);
    for (int i = 0; i < penjaga_n; i++)
        if (penjaga_awal[i] >= (uint64_t)(uintptr_t)a && penjaga_awal[i] < (uint64_t)(uintptr_t)a + n) {
            penjaga_awal[i] = penjaga_awal[penjaga_n - 1]; penjaga_ukuran[i] = penjaga_ukuran[--penjaga_n]; i--;
        }
    return 0;
}
/* PROT_NONE melepas halamannya dan mencatatnya sebagai penjaga; izin lain tidak berubah. */
int mprotect(void *a, size_t n, int p) {
    if (p != 0) return 0;
    for (uint64_t q = 0; q < n; q += 4096) lepas_halaman((uint64_t)(uintptr_t)a + q);
    if (penjaga_n < 4096) { penjaga_awal[penjaga_n] = (uint64_t)(uintptr_t)a; penjaga_ukuran[penjaga_n++] = n; }
    return 0;
}
/* `asli::` atas memori: bingkai fisik 4 KiB dan pemetaan di wilayah 4 KiB (di atas 4 GiB, di bawah 512 GiB).
   Jawaban negatif = kegagalan yang codegen ubah jadi galat berkalimat. */
int64_t jrg_bebas_bingkai_baru(void) { void *b = bingkai_baru(); return b ? (int64_t)(uintptr_t)b : -1; }
int64_t jrg_bebas_bingkai_lepas(int64_t f) {
    if (f & 4095) return -1;
    uint64_t *h = (uint64_t *)(uintptr_t)f; *h = (uint64_t)(uintptr_t)halaman_bebas; halaman_bebas = h;
    return 0;
}
int64_t jrg_bebas_petakan(int64_t v, int64_t f, int64_t bendera) {
    if ((v & 4095) || (f & 4095)) return -1;
    if ((uint64_t)v < WILAYAH_AWAL || (uint64_t)v >= (512ULL << 30)) return -2;
    uint64_t *e = pte_untuk((uint64_t)v, 1);
    if (!e) return -3;
    *e = ((uint64_t)f & 0x000FFFFFFFFFF000ULL) | ((uint64_t)bendera & 0x8000000000000FFFULL) | 1;
    __asm__ volatile("invlpg (%0)" :: "r"(v) : "memory");
    if ((uint64_t)v + 4096 > wilayah_kini) wilayah_kini = (uint64_t)v + 8192;
    return 0;
}
int64_t jrg_bebas_lepas_peta(int64_t v) {
    if (v & 4095) return -1;
    uint64_t *e = pte_untuk((uint64_t)v, 0);
    if (e) { *e = 0; __asm__ volatile("invlpg (%0)" :: "r"(v) : "memory"); }
    return 0;
}
static uint32_t mb_info_salin;
/* Peta memori multiboot: jumlah entri, lalu entri ke-i sebagai (awal, panjang, jenis). */
int64_t jrg_bebas_memori_n(void) {
    uint32_t *mb = (uint32_t *)(uintptr_t)mb_info_salin;
    if (!mb || !(mb[0] & 64)) return 0;
    int64_t n = 0;
    for (uint32_t p = mb[12], akhir = mb[12] + mb[11]; p < akhir; p += *(uint32_t *)(uintptr_t)p + 4) n++;
    return n;
}
int64_t jrg_bebas_memori(int64_t i, int64_t *awal, int64_t *panjang) {
    uint32_t *mb = (uint32_t *)(uintptr_t)mb_info_salin;
    if (!mb || !(mb[0] & 64) || i < 0) return -1;
    for (uint32_t p = mb[12], akhir = mb[12] + mb[11]; p < akhir; p += *(uint32_t *)(uintptr_t)p + 4, i--) {
        if (i == 0) {
            uint32_t *e = (uint32_t *)(uintptr_t)p;
            *awal = (int64_t)(e[1] | (uint64_t)e[2] << 32); *panjang = (int64_t)(e[3] | (uint64_t)e[4] << 32);
            return e[5];
        }
    }
    return -1;
}
static int di_wilayah_penjaga(uint64_t v) {
    for (int i = 0; i < penjaga_n; i++) if (v >= penjaga_awal[i] && v < penjaga_awal[i] + penjaga_ukuran[i]) return 1;
    return 0;
}

// ── errno, lingkungan, berkas: tidak ada sistem berkas ─────────────────────
extern __thread int errno;
int *__errno_location(void) { return &errno; }
#define TIDAK_ADA 38   /* ENOSYS */
/* Lingkungan = baris perintah multiboot (`NAMA=nilai` dipisah spasi), disalin saat mulai. */
static char baris_perintah[1024];
char *getenv(const char *n) {
    size_t k = strlen(n);
    for (char *p = baris_perintah; *p;) {
        while (*p == ' ') p++;
        char *awal = p;
        while (*p && *p != ' ') p++;
        if ((size_t)(p - awal) > k && memcmp(awal, n, k) == 0 && awal[k] == '=') {
            static char nilai[256];
            size_t m = (size_t)(p - awal) - k - 1;
            if (m >= sizeof nilai) m = sizeof nilai - 1;
            memcpy(nilai, awal + k + 1, m); nilai[m] = 0;
            return nilai;
        }
    }
    return 0;
}
int setenv(const char *n, const char *v, int o) { (void)n; (void)v; (void)o; errno = TIDAK_ADA; return -1; }
/* Bahasa pesan yang ditulis lapisan ini sendiri, aturannya sama dengan runtime: `JRG_BAHASA`, bawaan Inggris. */
static const char *pilih(const char *id, const char *en) {
    const char *b = getenv("JRG_BAHASA");
    return b && b[0] == 'i' && b[1] == 'd' ? id : en;
}
char *strerror(int e) { (void)e; return (char *)pilih("operasi ini tidak ada di kernel --bebas", "this operation does not exist in a --bebas kernel"); }
/* Berkas dan direktori: sistem berkas di RAM (`berkas_ram.c`). */
long jrg_ram_baca(int fd, void *b, size_t n);
long jrg_ram_tulis(int fd, const void *b, size_t n);
long sysconf(int n) { (void)n; return 1; }
int sigemptyset(void *s) { memset(s, 0, 128); return 0; }
int sigaddset(void *s, int n) { (void)s; (void)n; return 0; }
int sigdelset(void *s, int n) { (void)s; (void)n; return 0; }

// ── keluaran dan masukan ─────────────────────────────────────────────────
long write(int fd, const void *b, size_t n) {
    if (fd > 2) return jrg_ram_tulis(fd, b, n);
    const char *c = b;
    for (size_t i = 0; i < n; i++) {
        if (fd == 2) serial_tulis(0x2F8, c[i]);
        else { serial_tulis(0x3F8, c[i]); vga_tulis(c[i]); }
    }
    return (long)n;
}
/* stdin = serial COM1 (apa adanya; 0x04 = akhir masukan, dikirim jrgc saat stdin-nya habis) ATAU papan ketik
   PS/2 (layout US, Shift, Backspace, gema ke layar, satu baris sekali seperti terminal biasa). Papan ketik cuma
   dibaca selama program tidak memasang penangan IRQ1 sendiri. */
static const char PETA_KECIL[] = "\0\0331234567890-=\b\tqwertyuiop[]\n\0asdfghjkl;'`\0\\zxcvbnm,./\0*\0 ";
static const char PETA_BESAR[] = "\0\033!@#$%^&*()_+\b\tQWERTYUIOP{}\n\0ASDFGHJKL:\"~\0|ZXCVBNM<>?\0*\0 ";
static int penangan_irq1(void);
static char baris_ketik[256];
static size_t baris_n, baris_baca;
static int shift, serial_habis;
static int ketik_satu(void) {
    uint8_t k = masuk8(0x60);
    if (k == 0x2A || k == 0x36) { shift = 1; return 0; }
    if (k == 0xAA || k == 0xB6) { shift = 0; return 0; }
    if (k & 0x80 || k >= sizeof PETA_KECIL - 1) return 0;
    char c = (shift ? PETA_BESAR : PETA_KECIL)[k];
    if (!c) return 0;
    if (c == '\b') {
        if (baris_n) { baris_n--; if (fb) vga_tulis('\b'); else if (vga_x) { vga_x--; VGA[vga_y * 80 + vga_x] = 0x0720; } }
        return 0;
    }
    if (baris_n + 1 < sizeof baris_ketik) { baris_ketik[baris_n++] = c; vga_tulis(c); }
    return c == '\n';
}
long read(int fd, void *b, size_t n) {
    if (fd > 2) return jrg_ram_baca(fd, b, n);
    if (fd != 0 || n == 0) return 0;
    for (;;) {
        if (baris_baca < baris_n && baris_ketik[baris_n - 1] == '\n') {
            *(char *)b = baris_ketik[baris_baca++];
            if (baris_baca == baris_n) baris_baca = baris_n = 0;
            return 1;
        }
        if (!serial_habis && (masuk8(0x3F8 + 5) & 1)) {
            char c = (char)masuk8(0x3F8);
            if (c == 4) { serial_habis = 1; return 0; }
            *(char *)b = c;
            return 1;
        }
        if (!penangan_irq1() && (masuk8(0x64) & 1) && !(masuk8(0x64) & 0x20)) ketik_satu();
        else if (serial_habis && penangan_irq1()) return 0;
        __asm__ volatile("pause");
    }
}

// ── waktu: TSC dikalibrasi lawan PIT, detik dari RTC CMOS ──────────────────
static uint64_t tsc_per_us = 1000, tsc_awal, rtc_awal;
static inline uint64_t tsc(void) { uint32_t a, d; __asm__ volatile("rdtsc" : "=a"(a), "=d"(d)); return ((uint64_t)d << 32) | a; }
static void kalibrasi(void) {
    // PIT kanal 2, mode 0, 11932 detak = 10 ms.
    keluar8(0x61, (uint8_t)((masuk8(0x61) & 0xFD) | 1));
    keluar8(0x43, 0xB0); keluar8(0x42, 11932 & 0xFF); keluar8(0x42, 11932 >> 8);
    uint64_t a = tsc();
    while (!(masuk8(0x61) & 0x20)) {}
    uint64_t b = tsc();
    tsc_per_us = (b - a) / 10000; if (!tsc_per_us) tsc_per_us = 1;
}
static int cmos(int r) { keluar8(0x70, (uint8_t)r); return masuk8(0x71); }
static int bcd(int v) { return (v & 15) + (v >> 4) * 10; }
static uint64_t hari_sejak_1970(int y, int m, int d) {
    y -= m <= 2; int era = y / 400; int yoe = y - era * 400;
    int doy = (153 * (m + (m > 2 ? -3 : 9)) + 2) / 5 + d - 1;
    int doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    return (uint64_t)(era * 146097 + doe - 719468);
}
static void baca_rtc(void) {
    while (cmos(0x0A) & 0x80) {}
    int s = cmos(0), mi = cmos(2), j = cmos(4), d = cmos(7), bl = cmos(8), th = cmos(9);
    if (!(cmos(0x0B) & 4)) { s = bcd(s); mi = bcd(mi); j = bcd(j); d = bcd(d); bl = bcd(bl); th = bcd(th); }
    rtc_awal = hari_sejak_1970(2000 + th, bl, d) * 86400 + (uint64_t)(j * 3600 + mi * 60 + s);
    tsc_awal = tsc();
}
struct jrg_waktu { long tv_sec, tv_nsec; };
int clock_gettime(int jam, struct jrg_waktu *w) {
    uint64_t us = (tsc() - tsc_awal) / tsc_per_us;
    w->tv_sec = (long)((jam == 0 ? rtc_awal : 0) + us / 1000000);
    w->tv_nsec = (long)(us % 1000000) * 1000;
    return 0;
}
long time(long *t) { struct jrg_waktu w; clock_gettime(0, &w); if (t) *t = w.tv_sec; return w.tv_sec; }
int nanosleep(const struct jrg_waktu *w, struct jrg_waktu *sisa) {
    uint64_t akhir = tsc() + ((uint64_t)w->tv_sec * 1000000 + (uint64_t)w->tv_nsec / 1000) * tsc_per_us;
    while (tsc() < akhir) __asm__ volatile("pause");
    if (sisa) { sisa->tv_sec = 0; sisa->tv_nsec = 0; }
    return 0;
}
int getentropy(void *b, size_t n) {
    unsigned char *c = b;
    uint64_t x = tsc() ^ 0x9E3779B97F4A7C15ULL;
    for (size_t i = 0; i < n; i++) {
        uint64_t r;
        unsigned char ok;
        __asm__ volatile("rdrand %0; setc %1" : "=r"(r), "=qm"(ok));
        if (!ok) { x ^= x << 13; x ^= x >> 7; x ^= x << 17; r = x ^ tsc(); }
        c[i] = (unsigned char)r;
    }
    return 0;
}

// ── qsort (sisip untuk n kecil, heap sort selebihnya — tanpa rekursi) ──────
static void tukar(char *a, char *b, size_t w) { while (w--) { char t = *a; *a++ = *b; *b++ = t; } }
static void ayak(char *p, size_t akar, size_t n, size_t w, int (*f)(const void *, const void *)) {
    for (;;) {
        size_t anak = akar * 2 + 1, besar = akar;
        if (anak < n && f(p + anak * w, p + besar * w) > 0) besar = anak;
        if (anak + 1 < n && f(p + (anak + 1) * w, p + besar * w) > 0) besar = anak + 1;
        if (besar == akar) return;
        tukar(p + akar * w, p + besar * w, w); akar = besar;
    }
}
void qsort(void *b, size_t n, size_t w, int (*f)(const void *, const void *)) {
    char *p = b;
    if (n < 2) return;
    for (size_t i = n / 2; i-- > 0;) ayak(p, i, n, w, f);
    for (size_t i = n - 1; i > 0; i--) { tukar(p, p + i * w, w); ayak(p, 0, i, w, f); }
}

// ── snprintf: %d %i %u %x %s %c %% dengan l/ll ─────────────────────────────
int vsnprintf(char *o, size_t n, const char *f, va_list ap) {
    size_t k = 0;
#define TARUH(c) do { if (k + 1 < n) o[k] = (c); k++; } while (0)
    for (; *f; f++) {
        if (*f != '%') { TARUH(*f); continue; }
        f++;
        int pjg = 0;
        while (*f == 'l') { pjg++; f++; }
        if (*f == 'z') { pjg = 2; f++; }
        if (*f == 's') { const char *s = va_arg(ap, const char *); while (*s) TARUH(*s++); }
        else if (*f == 'c') TARUH((char)va_arg(ap, int));
        else if (*f == '%') TARUH('%');
        else if (*f == 'd' || *f == 'i' || *f == 'u' || *f == 'x') {
            unsigned long long v; int neg = 0;
            if (*f == 'd' || *f == 'i') {
                long long s = pjg >= 2 ? va_arg(ap, long long) : pjg ? va_arg(ap, long) : va_arg(ap, int);
                if (s < 0) { neg = 1; v = 0ULL - (unsigned long long)s; } else v = (unsigned long long)s;
            } else v = pjg >= 2 ? va_arg(ap, unsigned long long) : pjg ? va_arg(ap, unsigned long) : va_arg(ap, unsigned);
            char t[24]; int m = 0; unsigned basis = *f == 'x' ? 16 : 10;
            do { t[m++] = "0123456789abcdef"[v % basis]; v /= basis; } while (v);
            if (neg) TARUH('-');
            while (m) TARUH(t[--m]);
        }
    }
    if (n) o[k < n ? k : n - 1] = 0;
    return (int)k;
#undef TARUH
}
int snprintf(char *o, size_t n, const char *f, ...) { va_list ap; va_start(ap, f); int r = vsnprintf(o, n, f, ap); va_end(ap); return r; }

// ── keluar ───────────────────────────────────────────────────────────────
static void (*pamit[32])(void);
static int pamit_n;
int atexit(void (*f)(void)) { if (pamit_n == 32) return -1; pamit[pamit_n++] = f; return 0; }
__attribute__((noreturn)) void _exit(int kode) {
    // isa-debug-exit: QEMU keluar dengan (nilai << 1) | 1; jrgc membaliknya.
    keluar32(0xF4, (uint32_t)kode & 0xFF);
    for (;;) __asm__ volatile("cli; hlt");
}
__attribute__((noreturn)) void exit(int kode) {
    while (pamit_n) pamit[--pamit_n]();
    _exit(kode);
}
__attribute__((noreturn)) void abort(void) { _exit(134); }
void _Unwind_Resume(void) { abort(); }

// ── IDT, TSS dan interupsi ───────────────────────────────────────────────
typedef struct {
    uint64_t r15, r14, r13, r12, r11, r10, r9, r8, rbp, rdi, rsi, rdx, rcx, rbx, rax;
    uint64_t vektor, kode, rip, cs, rflags, rsp, ss;
} JrgBingkai;
typedef struct __attribute__((packed)) { uint32_t r0; uint64_t rsp[3], r1, ist[7], r2; uint16_t r3, iomap; } JrgTss;
typedef struct __attribute__((packed)) { uint16_t lo, sel; uint8_t ist, jenis; uint16_t tengah; uint32_t hi, nol; } JrgGerbang;
extern uint64_t jrg_bebas_gdt_tss[2];
extern void *jrg_bebas_pintu[256];
extern char jrg_bebas_penjaga[];
static JrgTss tss;
static JrgGerbang idt[256];
static char ist_irq[64 * 1024] __attribute__((aligned(16)));      /* IST1: IRQ dan interupsi perangkat lunak */
static char ist_galat[64 * 1024] __attribute__((aligned(16)));    /* IST2: pengecualian CPU */
static char ist_ganda[16 * 1024] __attribute__((aligned(16)));    /* IST3: double fault */

/* Penangan JRG per vektor (`asli::interupsi`): kode dan lingkungan `%JrgFn`. */
typedef struct { void *kode, *env; } JrgFnC;
static JrgFnC penangan[256];
static int penangan_irq1(void) { return penangan[33].kode != 0; }
void jrg_bebas_pasang_penangan(int64_t v, void *kode, void *env) { penangan[v & 255].kode = kode; penangan[v & 255].env = env; }

static void pasang_idt(void) {
    tss.ist[0] = (uint64_t)(uintptr_t)(ist_irq + sizeof ist_irq);
    tss.ist[1] = (uint64_t)(uintptr_t)(ist_galat + sizeof ist_galat);
    tss.ist[2] = (uint64_t)(uintptr_t)(ist_ganda + sizeof ist_ganda);
    tss.iomap = sizeof tss;
    uint64_t b = (uint64_t)(uintptr_t)&tss, l = sizeof tss - 1;
    jrg_bebas_gdt_tss[0] = (l & 0xFFFF) | ((b & 0xFFFFFF) << 16) | (0x89ULL << 40) | (((l >> 16) & 0xF) << 48)
        | (((b >> 24) & 0xFF) << 56);
    jrg_bebas_gdt_tss[1] = b >> 32;
    __asm__ volatile("ltr %w0" :: "r"((uint16_t)0x18));
    for (int i = 0; i < 256; i++) {
        uint64_t a = (uint64_t)(uintptr_t)jrg_bebas_pintu[i];
        idt[i] = (JrgGerbang){ (uint16_t)a, 0x08, (uint8_t)(i == 8 ? 3 : i < 32 ? 2 : 1), 0x8E,
            (uint16_t)(a >> 16), (uint32_t)(a >> 32), 0 };
    }
    struct __attribute__((packed)) { uint16_t batas; uint64_t alamat; } r = { sizeof idt - 1, (uint64_t)(uintptr_t)idt };
    __asm__ volatile("lidt %0" :: "m"(r));
}

static const char *const NAMA_GALAT[32] = {
    "#DE pembagian", "#DB debug", "NMI", "#BP breakpoint", "#OF luapan", "#BR batas", "#UD instruksi tak sah",
    "#NM FPU tidak ada", "#DF double fault", "segmen koprosesor", "#TS TSS tak sah", "#NP segmen tidak ada",
    "#SS segmen stack", "#GP proteksi umum", "#PF page fault", "cadangan", "#MF pecahan x87", "#AC perataan",
    "#MC pemeriksaan mesin", "#XM pecahan SIMD", "#VE virtualisasi", "#CP proteksi kendali", "cadangan", "cadangan",
    "cadangan", "cadangan", "cadangan", "cadangan", "#HV", "#VC", "#SX keamanan", "cadangan",
};
void jrg_bebas_lapor_stack(void);
static void tulis_teks(int fd, const char *s) { write(fd, s, strlen(s)); }
static void tulis_heks(int fd, uint64_t v) {
    char t[19] = "0x"; int n = 2;
    for (int i = 60; i >= 0; i -= 4) { int d = (int)((v >> i) & 15); if (d || n > 2 || i == 0) t[n++] = "0123456789abcdef"[d]; }
    t[n] = 0; tulis_teks(fd, t);
}
void jrg_bebas_interupsi(JrgBingkai *f) {
    uint64_t v = f->vektor;
    if (penangan[v].kode) {
        /* Penangan JRG `fn(vektor: int)`; pengecualian yang ditangani program juga lewat sini. */
        ((void (*)(void *, int64_t))penangan[v].kode)(penangan[v].env, (int64_t)v);
        return;
    }
    if (v >= 32) return;                   /* IRQ tanpa penangan: diabaikan (EOI urusan program) */
    uint64_t cr2 = 0;
    __asm__ volatile("mov %%cr2, %0" : "=r"(cr2));
    if (v == 14 && ((cr2 >= (uint64_t)(uintptr_t)jrg_bebas_penjaga && cr2 < (uint64_t)(uintptr_t)jrg_bebas_penjaga + 0x200000)
                    || di_wilayah_penjaga(cr2))) {
        if (f->rflags & 0x200) __asm__ volatile("sti");
        jrg_bebas_lapor_stack();
    }
    tulis_teks(2, pilih("error: pengecualian CPU ", "error: CPU exception "));
    tulis_teks(2, NAMA_GALAT[v]);
    tulis_teks(2, pilih(" di RIP ", " at RIP ")); tulis_heks(2, f->rip);
    if (v == 14) { tulis_teks(2, pilih(", alamat ", ", address ")); tulis_heks(2, cr2); }
    tulis_teks(2, pilih(", kode galat ", ", error code ")); tulis_heks(2, f->kode);
    tulis_teks(2, pilih("\n  catatan: kernel `--bebas` dihentikan; galat ini datang dari perangkat keras, bukan dari kalimat JRG\n",
                        "\n  note: the `--bebas` kernel was stopped; this error came from the hardware, not from a JRG sentence\n"));
    _exit(1);
}

// ── libm glibc statis: tabel fitur CPU, IFUNC, TLS ────────────────────────
/* Tabel fitur CPU glibc (`struct cpu_features`, dibaca pemilih IFUNC libm). Cuma bit yang dibaca pemilihnya
   diisi — diukur dari kode mesin libm: leaf 1 ECX di 0x2C, leaf 7 EBX di 0x48, leaf 0x80000001 ECX di 0x6C (bagian
   "aktif"). Varian FMA/AVX dipilih seperti di proses biasa, jadi hasil pecahannya sama sampai bit terakhir. */
char _dl_x86_cpu_features[4096] __attribute__((aligned(64)));
char jrg_bebas_pakai_xsave;
static void cpuid(uint32_t l, uint32_t s, uint32_t r[4]) {
    __asm__ volatile("cpuid" : "=a"(r[0]), "=b"(r[1]), "=c"(r[2]), "=d"(r[3]) : "a"(l), "c"(s));
}
static void pasang_fitur_cpu(void) {
    uint32_t r1[4], r7[4] = {0}, re[4] = {0}, maks[4];
    cpuid(0, 0, maks); cpuid(1, 0, r1);
    if (maks[0] >= 7) cpuid(7, 0, r7);
    cpuid(0x80000000u, 0, re);
    if (re[0] >= 0x80000001u) cpuid(0x80000001u, 0, re); else re[2] = 0;
    /* AVX diaktifkan seperti OS: CR4.OSXSAVE lalu XCR0 = x87 | SSE | AVX. */
    int avx = (r1[2] >> 26 & 1) && (r1[2] >> 28 & 1);
    if (avx) {
        uint64_t cr4; __asm__ volatile("mov %%cr4, %0" : "=r"(cr4)); cr4 |= 1u << 18;
        __asm__ volatile("mov %0, %%cr4" :: "r"(cr4));
        __asm__ volatile("xsetbv" :: "c"(0), "a"(7), "d"(0));
        jrg_bebas_pakai_xsave = 1;
    }
    uint32_t ecx1 = r1[2] & ((1u << 19) | (1u << 20) | (avx ? (1u << 12) | (1u << 28) : 0));
    uint32_t ebx7 = r7[1] & (avx ? 1u << 5 : 0), ecxe = re[2] & (avx ? 1u << 16 : 0);
    memcpy(_dl_x86_cpu_features + 0x2C, &ecx1, 4);
    memcpy(_dl_x86_cpu_features + 0x48, &ebx7, 4);
    memcpy(_dl_x86_cpu_features + 0x6C, &ecxe, 4);
}
__thread int errno;
typedef struct { uint64_t ofs, info; int64_t tambah; } JrgRela;
extern JrgRela __rela_iplt_start[], __rela_iplt_end[];
extern char __tdata_awal[], __tdata_akhir[], __tbss_akhir[], __tls_rata[];
static void pasang_ifunc(void) {
    for (JrgRela *r = __rela_iplt_start; r < __rela_iplt_end; r++) {
        if ((r->info & 0xFFFFFFFF) != 37) continue;           /* R_X86_64_IRELATIVE */
        uint64_t (*pemilih)(void) = (uint64_t (*)(void))(uintptr_t)r->tambah;
        *(uint64_t *)(uintptr_t)r->ofs = pemilih();
    }
}
/* Varian II x86_64: FS menunjuk TCB tepat sesudah blok TLS; fs:0 penunjuk dirinya, fs:0x28 canary. */
static void pasang_tls(void) {
    uint64_t rata = (uint64_t)(uintptr_t)__tls_rata; if (rata < 16) rata = 16;
    uint64_t ukuran = (uint64_t)(__tbss_akhir - __tdata_awal), data = (uint64_t)(__tdata_akhir - __tdata_awal);
    uint64_t blok = (ukuran + rata - 1) & ~(rata - 1);
    char *awal = malloc(blok + 256 + rata);
    char *tp = (char *)((((uintptr_t)awal + rata - 1) & ~(uintptr_t)(rata - 1)) + blok);
    memset(tp - blok, 0, blok + 256);
    memcpy(tp - blok, __tdata_awal, data);
    ((uint64_t *)tp)[0] = (uint64_t)(uintptr_t)tp;
    getentropy(tp + 0x28, 8);
    __asm__ volatile("wrmsr" :: "c"(0xC0000100u), "a"((uint32_t)(uintptr_t)tp), "d"((uint32_t)((uintptr_t)tp >> 32)));
}
__attribute__((noreturn)) void __stack_chk_fail(void) {
    tulis_teks(2, pilih("error: stack rusak (canary stack-protector berubah)\n", "error: stack corrupted (the stack-protector canary changed)\n"));
    _exit(1);
}

// ── mulai ────────────────────────────────────────────────────────────────
int main(int, char **);
extern void (*__init_awal[])(void);
extern void (*__init_akhir[])(void);
void jrg_bebas_mulai(uint32_t mb) {
    mb_info_salin = mb;
    serial_pasang(0x3F8); serial_pasang(0x2F8);
    /* Framebuffer dari multiboot (GRUB EFI): RGB 32 bit di bawah 4 GiB jadi konsol; selain itu layar teks VGA. */
    uint32_t *m = (uint32_t *)(uintptr_t)mb;
    if (mb && (m[0] & (1u << 12))) {
        uint64_t alamat = (uint64_t)m[22] | ((uint64_t)m[23] << 32);
        uint8_t bpp = ((uint8_t *)m)[108], jenis = ((uint8_t *)m)[109];
        if (jenis == 1 && bpp == 32 && alamat + (uint64_t)m[24] * m[26] <= (4ull << 30)) {
            fb = (volatile uint32_t *)(uintptr_t)alamat;
            fb_pitch = m[24]; fb_lebar = m[25]; fb_tinggi = m[26];
            fb_kol = fb_lebar / 8; fb_bar = fb_tinggi / 16;
            for (uint32_t i = 0; i < fb_tinggi * (fb_pitch / 4); i++) fb[i] = 0;
        }
    }
    if (!fb) for (int i = 0; i < 80 * 25; i++) VGA[i] = 0x0720;
    // Heap: sesudah kernel sampai batas memori atas dari multiboot (paling banyak 4 GiB terpeta).
    if (mb && (*(uint32_t *)(uintptr_t)mb & 4)) {
        const char *c = (const char *)(uintptr_t)((uint32_t *)(uintptr_t)mb)[4];
        size_t i = 0;
        while (c[i] && i + 1 < sizeof baris_perintah) { baris_perintah[i] = c[i]; i++; }
    }
    uint64_t atas = 64u << 20;
    if (mb && (*(uint32_t *)(uintptr_t)mb & 1)) atas = ((uint64_t)((uint32_t *)(uintptr_t)mb)[2] + 1024) * 1024;
    if (atas > (4ull << 30)) atas = 4ull << 30;
    heap_kini = __heap_awal; heap_akhir = (char *)(uintptr_t)atas;
    /* Halaman penjaga di bawah stack utama dilepas (PD halaman 2 MiB di peta identitas). */
    extern uint64_t pd[];
    uint64_t g = (uint64_t)(uintptr_t)jrg_bebas_penjaga;
    pd[g >> 21] = 0;
    __asm__ volatile("invlpg (%0)" :: "r"(g) : "memory");
    pasang_idt();
    pasang_tls();
    pasang_fitur_cpu();
    pasang_ifunc();
    kalibrasi(); baca_rtc();
    /* Konstruktor dipanggil seperti glibc: (argc, argv, envp). */
    static char *argv[] = { (char *)"kernel", 0 }, *envp[] = { 0 };
    for (void (**f)(void) = __init_awal; f < __init_akhir; f++) ((void (*)(int, char **, char **))*f)(1, argv, envp);
    exit(main(1, argv));
}
int sigprocmask(int h, const void *s, void *o) { (void)h; (void)s; (void)o; return 0; }
