// Pengganti `korutin.c` (dijaga `__unix__`) dan jembatan builtin di WebAssembly (`--emit-wasm`): yang berjalan
// selalu `main`, dan yang butuh korutin lain atau builtin lewat jembatan DITOLAK dengan kalimat yang menyebut jalan
// keluarnya, bukan dijawab diam-diam (`primitif.c` menyebut jembatan yang tak ditaut "tag tak dikenal").
#include <stdint.h>
#include <string.h>
#include <time.h>

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

static void jrg_wasm_belum(const char *apa) { jrg_wasm_belum_karena(apa, "WebAssembly belum punya pindah stack"); }

// Yang sedang berjalan selalu `main`: tidak ada goroutine yang bisa berhenti sendirian, tidak ada badan tugas.
int jrg_ko_gugur(int tanya) { (void)tanya; return 0; }
int jrg_masa_gagal(const char *s, int64_t n) { (void)s; (void)n; return 0; }
int64_t jrg_ko_lain(void) { return 0; }
void jrg_ko_serah(void) {}
void jrg_ko_tunggu_semua(void) {}
int jrg_ko_di_stack(uintptr_t alamat) { (void)alamat; return 1; }
int jrg_ko_selamatkan(void (*fn)(void)) { (void)fn; return 0; }

void jrg_ko_tidur(int64_t mikro) {
    if (mikro <= 0) return;
    struct timespec t = { (time_t)(mikro / 1000000), (long)(mikro % 1000000) * 1000 };
    nanosleep(&t, 0);
}

void jrg_ko_spawn(void (*fn)(void *), void *arg) { (void)fn; (void)arg; jrg_wasm_belum("goroutine (`spawn`/`go`)"); }

void *jrg_chan_new(int64_t kap, int64_t esize) { (void)kap; (void)esize; jrg_wasm_belum("channel"); return 0; }
int32_t jrg_chan_try_send(void *c, const void *v, int64_t esize) { (void)c; (void)v; (void)esize; jrg_wasm_belum("channel"); return 0; }
void jrg_chan_send(void *c, const void *v, int64_t esize) { (void)c; (void)v; (void)esize; jrg_wasm_belum("channel"); }
int32_t jrg_chan_try_recv(void *c, void *out, int64_t esize) { (void)c; (void)out; (void)esize; jrg_wasm_belum("channel"); return 0; }
void jrg_chan_recv(void *c, void *out, int64_t esize) { (void)c; (void)out; (void)esize; jrg_wasm_belum("channel"); }
void jrg_chan_close(void *c) { (void)c; jrg_wasm_belum("channel"); }
int64_t jrg_chan_info(const void *c, int32_t apa) { (void)c; (void)apa; jrg_wasm_belum("channel"); return 0; }

void *jrg_masa_baru(void (*jalan)(void *), void *env, void *kode, int64_t esize, int32_t tugas) {
    (void)jalan; (void)env; (void)kode; (void)esize; (void)tugas;
    jrg_wasm_belum("`Future`/`Task`");
    return 0;
}
void jrg_masa_ambil(void *c, void *out, const char *nama, int64_t nl) { (void)c; (void)out; (void)nama; (void)nl; jrg_wasm_belum("`Future`/`Task`"); }
int32_t jrg_masa_coba(void *c, void *out, int64_t mikro, const char *nama, int64_t nl) {
    (void)c; (void)out; (void)mikro; (void)nama; (void)nl;
    jrg_wasm_belum("`Future`/`Task`");
    return 0;
}
int32_t jrg_masa_lindung(void (*jalan)(void *), void *env, const char **pesan, int64_t *n) {
    (void)jalan; (void)env; (void)pesan; (void)n;
    jrg_wasm_belum("`--uji` dan handler `dok::`");
    return 0;
}
int32_t jrg_masa_selesai(void *c) { (void)c; jrg_wasm_belum("`Future`/`Task`"); return 0; }
int64_t jrg_masa_balap(void **daftar, int64_t n) { (void)daftar; (void)n; jrg_wasm_belum("`Future`/`Task`"); return 0; }

void *jrg_atom_baru(int64_t awal) { (void)awal; jrg_wasm_belum("`Atomic`"); return 0; }
void *jrg_kunci_baru(void) { jrg_wasm_belum("`Mutex`"); return 0; }
void jrg_kunci_kunci(void *k, int32_t dijaga, const char *nama, int64_t nl) { (void)k; (void)dijaga; (void)nama; (void)nl; jrg_wasm_belum("`Mutex`"); }
void jrg_kunci_lepas(void *k, const char *nama, int64_t nl) { (void)k; (void)nama; (void)nl; jrg_wasm_belum("`Mutex`"); }

// Jembatan builtin milik `jrgc` (`libjrgjembatan`) belum ditaut di wasm: yang kuat ini menggantikan simbol lemah di
// `primitif.c`, jadi galatnya menyebut builtin yang dipanggil.
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
