// Pindah stack korutin di WebAssembly (`--emit-wasm`) lewat Asyncify, plus penolakan builtin lewat jembatan, yang
// belum ditaut di wasm: kalimatnya menyebut jalan keluarnya, bukan dijawab diam-diam (`primitif.c` menyebutnya
// "tag tak dikenal").
#include <stdint.h>
#include <string.h>
#include <stdlib.h>

void jrg_panic(const unsigned char *ptr, int64_t len);

// `<apa> belum didukung --emit-wasm (<sebab>); jalankan ...` lewat `jrg_panic` (lokasi dan `defer` seperti galat lain).
static void jrg_wasm_belum_karena(const char *apa, const char *sebab) {
    static const char ekor[] = "; jalankan programnya dengan `jrgc`, `--vm`, `--jit` atau `--emit-llvm`";
    char pesan[320];
    size_t n = 0;
    const char *bagian[] = { apa, " belum didukung --emit-wasm (", sebab, ")" };
    for (int i = 0; i < 4; i++) {
        size_t m = strlen(bagian[i]);
        if (m > 100) m = 100;
        memcpy(pesan + n, bagian[i], m);
        n += m;
    }
    memcpy(pesan + n, ekor, sizeof ekor - 1);
    jrg_panic((const unsigned char *)pesan, (int64_t)(n + sizeof ekor - 1));
}

// ── Korutin lewat Asyncify (`wasm-opt --asyncify`) ──
// Satu unit terjemahan dengan `korutin.c`: penjadwal, channel, Future/Task dan Mutex miliknya dipakai utuh, yang
// diganti cuma pindah stack. Asyncify asimetris: korutin yang menyerah MEMBUKA stack-nya sampai `__main_void`, lalu
// `__main_void` MENGGULUNG ULANG korutin tujuan. Jadi `main` milik program juga jalan sebagai korutin di atasnya.
__attribute__((import_module("asyncify"), import_name("start_unwind"))) void jrg_asy_mulai_buka(void *data);
__attribute__((import_module("asyncify"), import_name("stop_unwind"))) void jrg_asy_henti_buka(void);
__attribute__((import_module("asyncify"), import_name("start_rewind"))) void jrg_asy_mulai_gulung(void *data);
__attribute__((import_module("asyncify"), import_name("stop_rewind"))) void jrg_asy_henti_gulung(void);

static void *jrg_sp_baca(void) {
    void *p;
    __asm__ volatile(".globaltype __stack_pointer, i32\n global.get __stack_pointer\n local.set %0" : "=r"(p));
    return p;
}
static void jrg_sp_tulis(void *p) {
    __asm__ volatile(".globaltype __stack_pointer, i32\n local.get %0\n global.set __stack_pointer" :: "r"(p));
}

// Batas stack bayangan korutin yang sedang jalan (`wasm-opt --stack-check`, `wasm::kompilasi`). Stack goroutine dari
// heap tidak berhalaman penjaga, jadi luapannya dulu menimpa blok heap lain tanpa suara. Pemuat node meneruskan
// batasnya ke `__set_stack_limits`; luapan memanggil `jrg_wasm_stack_habis`.
__attribute__((import_module("jrg"), import_name("batas_stack"))) void jrg_wasm_batas(uint32_t atas, uint32_t bawah);
extern char __stack_low, __stack_high;
static void jrg_wasm_batas_bebas(void) { jrg_wasm_batas(UINT32_MAX, 0); }
static void jrg_wasm_batas_untuk(JrgKo *k) {
    if (k == &jrg_ko_utama) jrg_wasm_batas((uint32_t)(uintptr_t)&__stack_high, (uint32_t)(uintptr_t)&__stack_low);
    else jrg_wasm_batas((uint32_t)(((uintptr_t)k->stack + JRG_KO_STACK) & ~(uintptr_t)15), (uint32_t)(uintptr_t)k->stack);
}

// Stack habis (bayangan, atau stack V8 lewat pemuat): dilaporkan dari stack cadangan dengan kalimat native
// (`jaga_stack.c`), keluaran sebelumnya tetap keluar. Seluruh program berhenti, bukan goroutine-nya saja.
__attribute__((export_name("jrg_wasm_stack_habis"))) void jrg_wasm_stack_habis(void) {
    static char cadangan[64 << 10] __attribute__((aligned(16)));
    jrg_sp_tulis(cadangan + sizeof cadangan);
    jrg_ko_fatal = 1;
    jrg_situs = 0;
    jrg_panic((const unsigned char *)JRG_PESAN_STACK, (int64_t)(sizeof JRG_PESAN_STACK - 1));
}

// Data Asyncify per korutin: lokal tiap bingkai yang sedang dibuka (bukan stack-nya, yang tinggal di `stack`).
#define JRG_WASM_SIMPAN ((size_t)256 << 10)
static int jrg_wasm_dibuka;

// Wajib `noinline` dan tanpa bingkai stack: kalau disisipkan clang, unwind tidak terjadi; kalau berbingkai, prolognya
// jalan ulang saat rewind dan stack pointer turun tiap perpindahan. Saat rewind fungsi ini dimasuki ULANG dari atas.
__attribute__((noinline)) static void jrg_ko_wasm_tukar(JrgKo *dari, JrgKo *ke) {
    (void)ke;
    if (!dari->ctx.tidur) {
        dari->ctx.tidur = 1;
        dari->ctx.data[0] = dari->ctx.simpan;
        dari->ctx.data[1] = dari->ctx.simpan + JRG_WASM_SIMPAN;
        dari->ctx.sp = jrg_sp_baca();
        jrg_wasm_dibuka = 1;
        jrg_asy_mulai_buka(dari->ctx.data);
    } else {
        jrg_asy_henti_gulung();
        dari->ctx.tidur = 0;
    }
}

// Bingkai `jrgc` di bawah jembatan tidak diinstrumentasi Asyncify (`wasm::tanpa_korutin`): korutin yang berganti
// dari fungsi program yang dipanggil builtin jembatan tidak bisa dilanjutkan, jadi ditolak keras di sini.
extern int32_t jrg_wasm_di_jembatan __attribute__((weak));
static int jrg_wasm_dalam_jembatan(void) { return &jrg_wasm_di_jembatan != 0 && jrg_wasm_di_jembatan > 0; }
static void jrg_wasm_cek_jembatan(void) {
    // Seluruh program berhenti, bukan goroutine-nya saja: stack-nya tidak bisa dilanjutkan.
    if (jrg_wasm_dalam_jembatan()) jrg_ko_fatal = 1;
    if (jrg_wasm_dalam_jembatan())
        jrg_wasm_belum_karena("goroutine yang berganti di dalam fungsi yang dipanggil builtin jembatan",
                              "bingkai builtin tidak bisa dihentikan sementara di WebAssembly");
}

#ifdef JRG_WASM_JEMBATAN
// Builtin jembatan yang dipanggil goroutine jalan di stack besar milik jembatan (`wasm::kompilasi` menautnya lewat
// `--wrap`): penjelajah nilai `jrgc` butuh lebih dari stack korutin 1 MiB (JSON 8 000 tingkat). Stack ini tidak
// pernah dipakai dua korutin sekaligus, karena di dalam jembatan korutin tidak boleh berganti (`jrg_wasm_cek_jembatan`).
#define JRG_WASM_STACK_JEMBATAN ((size_t)8 << 20)
int32_t __real_jrg_jembatan_panggil(const char *, int64_t, const JrgAny *, int64_t, JrgAny *, const JrgKait *);
int32_t __wrap_jrg_jembatan_panggil(const char *nama, int64_t nl, const JrgAny *args, int64_t n, JrgAny *out,
                                    const JrgKait *kait) {
    static char *stack;
    if (jrg_ko_kini == &jrg_ko_utama || jrg_wasm_dalam_jembatan()) return __real_jrg_jembatan_panggil(nama, nl, args, n, out, kait);
    if (stack == 0 && (stack = malloc(JRG_WASM_STACK_JEMBATAN)) == 0) return __real_jrg_jembatan_panggil(nama, nl, args, n, out, kait);
    uintptr_t atas = ((uintptr_t)stack + JRG_WASM_STACK_JEMBATAN) & ~(uintptr_t)15;
    void *lama = jrg_sp_baca();
    jrg_wasm_batas_bebas();
    jrg_sp_tulis((void *)atas);
    jrg_wasm_batas((uint32_t)atas, (uint32_t)(uintptr_t)stack);
    int32_t r = __real_jrg_jembatan_panggil(nama, nl, args, n, out, kait);
    jrg_wasm_batas_bebas();
    jrg_sp_tulis(lama);
    jrg_wasm_batas_untuk(jrg_ko_kini);
    return r;
}
#endif

static char *jrg_ko_wasm_stack(JrgKo *k) {
    k->ctx.simpan = malloc(JRG_WASM_SIMPAN);
    char *st = malloc(JRG_KO_STACK);
    if (st == 0 || k->ctx.simpan == 0) { free(st); free(k->ctx.simpan); k->ctx.simpan = 0; return 0; }
    k->ctx.sp = (void *)(((uintptr_t)st + JRG_KO_STACK) & ~(uintptr_t)15);
    return st;
}

static void jrg_ko_wasm_lepas(JrgKo *k) {
    free(k->stack);
    free(k->ctx.simpan);
    k->stack = 0;
    k->ctx.simpan = 0;
}

// `main` milik program (`jrgc` menamainya ulang); `__main_void` adalah dasar tempat korutin dibuka dan digulung,
// jadi ia sendiri TIDAK diinstrumentasi Asyncify (`asyncify-removelist`). Keadaannya global, bukan di stack bayangan.
int jrg_main_program(void);

// Pintu masuk goroutine TIDAK boleh disisipkan ke `__main_void` (yang tidak diinstrumentasi): badannya bisa pindah
// korutin, dan kode yang bisa pindah wajib diinstrumentasi Asyncify.
__attribute__((noinline)) static void jrg_wasm_masuk(void) { jrg_ko_masuk(); }
static int jrg_wasm_hasil;
static void *jrg_wasm_dasar;

int __main_void(void) {
    jrg_wasm_dasar = jrg_sp_baca();
    jrg_ko_utama.ctx.simpan = malloc(JRG_WASM_SIMPAN);
    if (jrg_ko_utama.ctx.simpan == 0) jrg_wasm_belum_karena("korutin", "memori habis untuk data Asyncify");
    for (;;) {
        JrgKo *k = jrg_ko_kini;
        jrg_wasm_dibuka = 0;
        // Batas dilepas selama stack pointer dipindah, lalu dipasang untuk korutin tujuan.
        jrg_wasm_batas_bebas();
        if (k->ctx.mulai) {
            jrg_sp_tulis(k->ctx.sp);
            jrg_asy_mulai_gulung(k->ctx.data);
        } else {
            k->ctx.mulai = 1;
            if (k != &jrg_ko_utama) jrg_sp_tulis(k->ctx.sp);
        }
        jrg_wasm_batas_untuk(k);
        if (k == &jrg_ko_utama) jrg_wasm_hasil = jrg_main_program(); else jrg_wasm_masuk();
        jrg_wasm_batas_bebas();
        jrg_sp_tulis(jrg_wasm_dasar);
        if (jrg_wasm_dibuka) { jrg_asy_henti_buka(); continue; }
        // Kembali biasa cuma `main`: goroutine yang selesai menyerahkan giliran (`jrg_ko_tamat`) dan tidak kembali.
        return jrg_wasm_hasil;
    }
}

// Program tanpa pustaka gabungan runtime + jembatan (`wasm-rt/`, `JRG_WASM_JEMBATAN`): yang kuat ini menggantikan
// simbol lemah di `primitif.c`, jadi galat yang tidak mungkin terjadi pun menyebut builtin yang dipanggil.
#ifndef JRG_WASM_JEMBATAN
int32_t jrg_jembatan_panggil(const char *nama, int64_t nl, const JrgAny *args, int64_t n, JrgAny *out, const JrgKait *kait) {
    (void)args; (void)n; (void)out; (void)kait;
    char apa[96] = "builtin `";
    size_t p = strlen(apa), m = nl > 60 ? 60 : (size_t)nl;
    memcpy(apa + p, nama, m);
    apa[p + m] = '`';
    apa[p + m + 1] = 0;
    jrg_wasm_belum_karena(apa, "jembatan builtin `jrgc` belum ditaut di WebAssembly");
    return -1;
}
#endif
