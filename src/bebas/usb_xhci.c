
// ── papan ketik USB: pengandar xHCI minimal (tanpa interupsi, tanpa hub) ───────────────────────────────────
// Rancangan di dokumentasi/src/bebas/mod.md "Papan ketik USB". Disambung di belakang libc_mini.c.
static inline uint32_t masuk32(uint16_t p) { uint32_t v; __asm__ volatile("inl %1, %0" : "=a"(v) : "Nd"(p)); return v; }
static uint32_t pci_baca(int b, int d, int f, int o) {
    keluar32(0xCF8, 0x80000000u | (uint32_t)b << 16 | (uint32_t)d << 11 | (uint32_t)f << 8 | (uint32_t)(o & 0xFC));
    return masuk32(0xCFC);
}
static void pci_tulis(int b, int d, int f, int o, uint32_t v) {
    keluar32(0xCF8, 0x80000000u | (uint32_t)b << 16 | (uint32_t)d << 11 | (uint32_t)f << 8 | (uint32_t)(o & 0xFC));
    keluar32(0xCFC, v);
}
static void tunda_us(uint64_t us) { uint64_t a = tsc(); while (tsc() - a < us * tsc_per_us) __asm__ volatile("pause"); }

typedef struct { uint64_t p; uint32_t s, c; } Trb;
typedef struct { volatile Trb *t; uint32_t i, siklus; } Cincin;
#define TRB_N 64
static volatile uint8_t *xc, *xo, *xr;          /* register kapabilitas, operasional, runtime */
static volatile uint32_t *xdb;                   /* bel pintu */
static uint32_t ctx_uk;                          /* 32 atau 64 byte per konteks */
static volatile uint64_t *dcbaa;
static Cincin perintah, kejadian;
static int usb_slot, usb_dci, usb_siap, usb_lambat;
static Cincin cincin_ep0, cincin_int;
static volatile uint8_t *laporan;                /* 8 byte laporan boot HID */
static uint8_t tekan_lalu[6];

#define R32(b, o) (*(volatile uint32_t *)((b) + (o)))
#define R64(b, o) (*(volatile uint64_t *)((b) + (o)))
static void *dma_baru(void) { return bingkai_baru(); }   /* 4 KiB nol, terpeta identitas (fisik = maya) */

static void cincin_buat(Cincin *c) {
    c->t = dma_baru(); c->i = 0; c->siklus = 1;
    volatile Trb *l = &c->t[TRB_N - 1];
    l->p = (uint64_t)(uintptr_t)c->t; l->s = 0; l->c = (6u << 10) | 2;     /* Link, Toggle Cycle */
}
static uint64_t cincin_taruh(Cincin *c, uint64_t p, uint32_t s, uint32_t ctl) {
    volatile Trb *t = &c->t[c->i];
    t->p = p; t->s = s;
    __asm__ volatile("" ::: "memory");
    t->c = (ctl & ~1u) | c->siklus;
    uint64_t alamat = (uint64_t)(uintptr_t)t;
    if (++c->i == TRB_N - 1) {
        volatile Trb *l = &c->t[TRB_N - 1];
        l->c = (l->c & ~1u) | c->siklus;
        c->i = 0; c->siklus ^= 1;
    }
    return alamat;
}
/* Kejadian berikutnya (0 kalau belum ada); `ERDP` dimajukan. */
static int kejadian_ambil(Trb *keluar) {
    volatile Trb *t = &kejadian.t[kejadian.i];
    if ((t->c & 1) != kejadian.siklus) return 0;
    keluar->p = t->p; keluar->s = t->s; keluar->c = t->c;
    if (++kejadian.i == TRB_N) { kejadian.i = 0; kejadian.siklus ^= 1; }
    R64(xr, 0x38) = (uint64_t)(uintptr_t)&kejadian.t[kejadian.i] | 8;
    return 1;
}
/* Tunggu kejadian berjenis `jenis` (33 = perintah selesai, 32 = transfer); kode selesai, atau -1 kalau lewat. */
static int tunggu(int jenis, Trb *keluar, uint64_t batas_us) {
    uint64_t a = tsc();
    while (tsc() - a < batas_us * tsc_per_us) {
        Trb e;
        if (kejadian_ambil(&e)) {
            if ((int)((e.c >> 10) & 63) == jenis) { if (keluar) *keluar = e; return (int)(e.s >> 24); }
            continue;
        }
        __asm__ volatile("pause");
    }
    return -1;
}
static int perintah_kirim(uint64_t p, uint32_t ctl, Trb *e) {
    cincin_taruh(&perintah, p, 0, ctl);
    xdb[0] = 0;
    return tunggu(33, e, 500000);
}
static volatile uint8_t *ctx(volatile uint8_t *dasar, int i) { return dasar + (size_t)i * ctx_uk; }

/* Transfer kendali di EP0: arah masuk kalau `bm & 0x80`. Jawab kode selesai (1 = berhasil, 13 = pendek). */
static int kendali(uint8_t bm, uint8_t req, uint16_t val, uint16_t idx, uint16_t n, volatile void *buf) {
    uint64_t setup = bm | (uint64_t)req << 8 | (uint64_t)val << 16 | (uint64_t)idx << 32 | (uint64_t)n << 48;
    uint32_t trt = n == 0 ? 0 : (bm & 0x80) ? 3 : 2;
    cincin_taruh(&cincin_ep0, setup, 8, (2u << 10) | (1u << 6) | (trt << 16));
    if (n) cincin_taruh(&cincin_ep0, (uint64_t)(uintptr_t)buf, n, (3u << 10) | ((bm & 0x80) ? 1u << 16 : 0));
    uint32_t arah_status = (n && (bm & 0x80)) ? 0 : 1u << 16;
    cincin_taruh(&cincin_ep0, 0, 0, (4u << 10) | (1u << 5) | arah_status);
    xdb[usb_slot] = 1;
    return tunggu(32, 0, 500000);
}

static const char HID_KECIL[] = "abcdefghijklmnopqrstuvwxyz1234567890\n\033\b\t -=[]\\#;'`,./";
static const char HID_BESAR[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZ!@#$%^&*()\n\033\b\t _+{}|~:\"~<>?";

/* Laporan boot: [pengubah, cadangan, 6 kode tombol]. Cuma tombol yang BARU ditekan yang diketik. */
static void laporan_olah(void) {
    int besar = (laporan[0] & 0x22) != 0;
    for (int i = 2; i < 8; i++) {
        uint8_t k = laporan[i];
        if (k < 4 || k > 0x38) continue;
        int lama = 0;
        for (int j = 0; j < 6; j++) if (tekan_lalu[j] == k) lama = 1;
        if (!lama) { char c = (besar ? HID_BESAR : HID_KECIL)[k - 4]; if (c) ketik_char(c); }
    }
    for (int i = 0; i < 6; i++) tekan_lalu[i] = laporan[i + 2];
}
static void laporan_minta(void) {
    cincin_taruh(&cincin_int, (uint64_t)(uintptr_t)laporan, 8, (1u << 10) | (1u << 5) | (1u << 2));
    xdb[usb_slot] = (uint32_t)usb_dci;
}
/* Dipanggil dari `read`: kejadian transfer papan ketik diolah, permintaan berikutnya dipasang. */
static void usb_poll(void) {
    if (!usb_siap) return;
    Trb e;
    while (kejadian_ambil(&e)) {
        if (((e.c >> 10) & 63) == 32 && (int)(e.c >> 24) == usb_slot && (int)((e.c >> 16) & 31) == usb_dci) {
            uint32_t kode = e.s >> 24;
            if (kode == 1 || kode == 13) laporan_olah();
            laporan_minta();
        }
    }
}

/* Satu port akar yang tersambung: alamat, deskriptor, konfigurasi, endpoint interupsi. 1 kalau papan ketik. */
static int usb_pasang_port(int port, uint32_t maks_slot) {
    volatile uint32_t *sc = (volatile uint32_t *)(xo + 0x400 + 0x10 * (port - 1));
    if (!(*sc & 1)) return 0;
    *sc = (*sc & 0x0E00C3E0u) | (1u << 4);                 /* reset port, bit RW1C tidak disentuh */
    for (int i = 0; i < 100 && !(*sc & (1u << 21)); i++) tunda_us(1000);
    *sc = (*sc & 0x0E00C3E0u) | (1u << 21);                /* bersihkan PRC */
    if (!(*sc & 2)) return 0;
    uint32_t laju = (*sc >> 10) & 15;
    Trb e;
    if (perintah_kirim(0, 9u << 10, &e) != 1) return 0;   /* Enable Slot */
    usb_slot = (int)(e.c >> 24);
    if (usb_slot < 1 || (uint32_t)usb_slot > maks_slot) return 0;
    volatile uint8_t *keluar = dma_baru(), *masuk = dma_baru();
    dcbaa[usb_slot] = (uint64_t)(uintptr_t)keluar;
    cincin_buat(&cincin_ep0);
    uint32_t mps = laju == 4 ? 512 : laju == 3 ? 64 : 8;
    R32(ctx(masuk, 0), 4) = 3;                                       /* tambah slot + EP0 */
    R32(ctx(masuk, 1), 0) = (1u << 27) | (laju << 20);
    R32(ctx(masuk, 1), 4) = (uint32_t)port << 16;
    R32(ctx(masuk, 2), 4) = (3u << 1) | (4u << 3) | (mps << 16);    /* CErr 3, kendali */
    R64(ctx(masuk, 2), 8) = (uint64_t)(uintptr_t)cincin_ep0.t | 1;
    R32(ctx(masuk, 2), 16) = 8;
    if (perintah_kirim((uint64_t)(uintptr_t)masuk, (11u << 10) | (uint32_t)usb_slot << 24, 0) != 1) return 0;
    volatile uint8_t *buf = dma_baru();
    int r = kendali(0x80, 6, 0x0100, 0, 18, buf);                   /* deskriptor perangkat */
    if (r != 1 && r != 13) return 0;
    r = kendali(0x80, 6, 0x0200, 0, 255, buf);                      /* deskriptor konfigurasi utuh */
    if (r != 1 && r != 13) return 0;
    int total = buf[2] | buf[3] << 8, konf = buf[5], antarmuka = -1, cocok = 0, ep = 0, mps_ep = 8, jeda = 10;
    if (total > 255) total = 255;
    /* Antarmuka HID kelas boot papan ketik (3/1/1) dan endpoint interupsi masuk pertamanya. */
    for (int i = 0; i + 1 < total && buf[i] >= 2 && !ep; i += buf[i]) {
        if (buf[i + 1] == 4) {
            cocok = buf[i + 5] == 3 && buf[i + 6] == 1 && buf[i + 7] == 1;
            if (cocok) antarmuka = buf[i + 2];
        }
        if (buf[i + 1] == 5 && cocok && (buf[i + 2] & 0x80) && (buf[i + 3] & 3) == 3) {
            ep = buf[i + 2] & 15; mps_ep = (buf[i + 4] | buf[i + 5] << 8) & 0x7FF; jeda = buf[i + 6];
        }
    }
    if (antarmuka < 0 || !ep) return 0;
    usb_dci = ep * 2 + 1;
    cincin_buat(&cincin_int);
    for (int i = 0; i < 33 * (int)ctx_uk / 4; i++) ((volatile uint32_t *)masuk)[i] = 0;
    R32(ctx(masuk, 0), 4) = 1u | (1u << usb_dci);
    R32(ctx(masuk, 1), 0) = ((uint32_t)usb_dci << 27) | (laju << 20);
    R32(ctx(masuk, 1), 4) = (uint32_t)port << 16;
    uint32_t iv = 0;
    if (laju >= 3) iv = jeda > 0 ? (uint32_t)jeda - 1 : 0;
    else { uint32_t ms = jeda ? (uint32_t)jeda : 1; iv = 3; while ((1u << (iv - 3)) < ms && iv < 10) iv++; }
    volatile uint8_t *epc = ctx(masuk, usb_dci + 1);
    R32(epc, 0) = iv << 16;
    R32(epc, 4) = (3u << 1) | (7u << 3) | ((uint32_t)mps_ep << 16);  /* interupsi masuk */
    R64(epc, 8) = (uint64_t)(uintptr_t)cincin_int.t | 1;
    R32(epc, 16) = 8 | ((uint32_t)mps_ep << 16);
    if (perintah_kirim((uint64_t)(uintptr_t)masuk, (12u << 10) | (uint32_t)usb_slot << 24, 0) != 1) return 0;
    if (kendali(0x00, 9, (uint16_t)konf, 0, 0, 0) != 1) return 0;           /* SET_CONFIGURATION */
    kendali(0x21, 0x0B, 0, (uint16_t)antarmuka, 0, 0);                      /* SET_PROTOCOL boot */
    kendali(0x21, 0x0A, 0, (uint16_t)antarmuka, 0, 0);                      /* SET_IDLE */
    laporan = dma_baru();
    usb_lambat = laju;
    laporan_minta();
    return 1;
}

/* Pengendali xHCI pertama di PCI (kelas 0C/03/30) disiapkan; papan ketik boot di port akarnya dipakai. */
static void usb_mulai(void) {
    for (int b = 0; b < 256; b++) for (int d = 0; d < 32; d++) for (int f = 0; f < 8; f++) {
        uint32_t id = pci_baca(b, d, f, 0);
        if ((id & 0xFFFF) == 0xFFFF) { if (f == 0) break; continue; }
        if ((pci_baca(b, d, f, 8) >> 8) != 0x0C0330) continue;
        uint32_t bar = pci_baca(b, d, f, 0x10);
        uint64_t fisik = bar & ~0xFu;
        if (((bar >> 1) & 3) == 2) fisik |= (uint64_t)pci_baca(b, d, f, 0x14) << 32;
        pci_tulis(b, d, f, 4, pci_baca(b, d, f, 4) | 6);        /* ruang memori + bus master */
        uint64_t maya = fisik;
        if (fisik + 0x10000 > (4ull << 30)) {                    /* di atas peta identitas: petakan 64 KiB */
            maya = (wilayah_kini + 0xFFFF) & ~0xFFFFull;
            for (uint64_t o = 0; o < 0x10000; o += 4096) jrg_bebas_petakan((int64_t)(maya + o), (int64_t)(fisik + o), 0x1A);
        }
        xc = (volatile uint8_t *)(uintptr_t)maya;
        xo = xc + xc[0];
        xr = xc + (R32(xc, 0x18) & ~0x1Fu);
        xdb = (volatile uint32_t *)(xc + (R32(xc, 0x14) & ~3u));
        uint32_t hcs1 = R32(xc, 4), hcs2 = R32(xc, 8), hcc1 = R32(xc, 0x10);
        ctx_uk = (hcc1 & 4) ? 64 : 32;
        /* Serah terima dari firmware (USB Legacy Support). */
        for (uint32_t p = (hcc1 >> 16) * 4; p; ) {
            uint32_t cap = R32(xc, p);
            if ((cap & 0xFF) == 1) {
                R32(xc, p) = cap | (1u << 24);
                for (int i = 0; i < 1000 && (R32(xc, p) & (1u << 16)); i++) tunda_us(1000);
                R32(xc, p + 4) = (R32(xc, p + 4) & ~0xE011u) | 0xE0000000u;
            }
            p = ((cap >> 8) & 0xFF) ? p + ((cap >> 8) & 0xFF) * 4 : 0;
        }
        R32(xo, 0) &= ~1u;
        for (int i = 0; i < 100 && !(R32(xo, 4) & 1); i++) tunda_us(1000);
        R32(xo, 0) |= 2;
        for (int i = 0; i < 1000 && ((R32(xo, 0) & 2) || (R32(xo, 4) & (1u << 11))); i++) tunda_us(1000);
        uint32_t maks_slot = hcs1 & 0xFF;
        R32(xo, 0x38) = maks_slot;
        dcbaa = dma_baru();
        uint32_t pad = ((hcs2 >> 27) & 31) | (((hcs2 >> 21) & 31) << 5);
        if (pad) {
            volatile uint64_t *daftar = dma_baru();
            for (uint32_t i = 0; i < pad && i < 512; i++) daftar[i] = (uint64_t)(uintptr_t)dma_baru();
            dcbaa[0] = (uint64_t)(uintptr_t)daftar;
        }
        R64(xo, 0x30) = (uint64_t)(uintptr_t)dcbaa;
        cincin_buat(&perintah);
        R64(xo, 0x18) = (uint64_t)(uintptr_t)perintah.t | 1;
        kejadian.t = dma_baru(); kejadian.i = 0; kejadian.siklus = 1;
        volatile uint64_t *erst = dma_baru();
        erst[0] = (uint64_t)(uintptr_t)kejadian.t; erst[1] = TRB_N;
        R32(xr, 0x28) = 1;
        R64(xr, 0x38) = (uint64_t)(uintptr_t)kejadian.t;
        R64(xr, 0x30) = (uint64_t)(uintptr_t)erst;
        R32(xo, 0) |= 1;
        for (int i = 0; i < 100 && (R32(xo, 4) & 1); i++) tunda_us(1000);
        tunda_us(50000);
        uint32_t port_n = hcs1 >> 24;
        for (uint32_t p = 1; p <= port_n; p++) if (usb_pasang_port((int)p, maks_slot)) { usb_siap = 1; return; }
        return;
    }
}
