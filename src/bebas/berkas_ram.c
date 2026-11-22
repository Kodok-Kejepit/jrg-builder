// berkas_ram.c — sistem berkas di RAM untuk kernel `--bebas` (initramfs): isinya dari folder `--berkas`, bisa
// ditulis, hilang saat mati. Disusun dengan header glibc supaya `struct stat`/`dirent` sama dengan yang dibaca runtime.
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

/* Isi awal, ditanam `jrgc` (`berkas_awal.S`); tanpa `--berkas` tabelnya kosong. */
typedef struct { const char *jalur; const unsigned char *isi; uint64_t n; uint64_t dir; } JrgBerkasAwal;
extern const JrgBerkasAwal jrg_berkas_awal[] __attribute__((weak));
extern const uint64_t jrg_berkas_awal_n __attribute__((weak));
int getentropy(void *b, size_t n);

typedef struct { char *jalur; int dir, ada; unsigned char *isi; size_t n, kap; } Simpul;
static Simpul *simpul;
static size_t simpul_n, simpul_kap;
typedef struct { Simpul *s; size_t pos; int buka, tulis, acak; } Pegangan;
static Pegangan pegangan[256];

/* Jalur absolut ternormalkan (`/a/b`, akar `/`): `.`, `..` dan garis miring ganda diselesaikan. */
static char *normal(const char *p) {
    size_t n = strlen(p);
    char *o = malloc(n + 2), *seg[256];
    size_t m = 0;
    char *salin = malloc(n + 1);
    if (!o || !salin) return 0;
    memcpy(salin, p, n + 1);
    for (char *t = salin, *s; (s = t) != 0;) {
        t = strchr(s, '/');
        if (t) *t++ = 0;
        if (!*s || strcmp(s, ".") == 0) continue;
        if (strcmp(s, "..") == 0) { if (m) m--; continue; }
        if (m < 256) seg[m++] = s;
    }
    size_t k = 0;
    for (size_t i = 0; i < m; i++) { o[k++] = '/'; size_t l = strlen(seg[i]); memcpy(o + k, seg[i], l); k += l; }
    if (!k) o[k++] = '/';
    o[k] = 0;
    free(salin);
    return o;
}

static Simpul *cari(const char *p) {
    char *j = normal(p);
    Simpul *h = 0;
    for (size_t i = 0; j && i < simpul_n; i++) if (simpul[i].ada && strcmp(simpul[i].jalur, j) == 0) { h = &simpul[i]; break; }
    free(j);
    return h;
}

static int induk_ada(const char *j) {
    const char *t = strrchr(j, '/');
    if (t == j) return 1;
    char *ind = malloc((size_t)(t - j) + 1);
    memcpy(ind, j, (size_t)(t - j)); ind[t - j] = 0;
    Simpul *s = cari(ind);
    free(ind);
    return s && s->dir;
}

static Simpul *buat(const char *p, int dir) {
    if (simpul_n == simpul_kap) {
        size_t k = simpul_kap ? simpul_kap * 2 : 64;
        Simpul *b = realloc(simpul, k * sizeof *b);
        if (!b) return 0;
        simpul = b; simpul_kap = k;
    }
    Simpul *s = &simpul[simpul_n++];
    memset(s, 0, sizeof *s);
    s->jalur = normal(p); s->dir = dir; s->ada = 1;
    return s;
}

static int siap;
static void pasang(void) {
    if (siap) return;
    siap = 1;
    buat("/", 1);
    buat("/tmp", 1);              /* seperti OS biasa: tempat berkas sementara */
    if (!jrg_berkas_awal) return;
    for (uint64_t i = 0; i < jrg_berkas_awal_n; i++) {
        const JrgBerkasAwal *a = &jrg_berkas_awal[i];
        Simpul *s = cari(a->jalur);
        if (!s) s = buat(a->jalur, (int)a->dir);
        if (!a->dir && s) {
            s->isi = malloc(a->n ? a->n : 1); s->n = s->kap = a->n;
            if (a->n) memcpy(s->isi, a->isi, a->n);
        }
    }
}

static int gagal(int e) { errno = e; return -1; }

static void isi_stat(const Simpul *s, struct stat *st) {
    memset(st, 0, sizeof *st);
    st->st_mode = s->dir ? (S_IFDIR | 0755) : (S_IFREG | 0644);
    st->st_size = (off_t)s->n;
    st->st_nlink = 1;
    st->st_blksize = 4096;
}

int stat(const char *p, struct stat *st) {
    pasang();
    Simpul *s = cari(p);
    if (!s) return gagal(ENOENT);
    isi_stat(s, st);
    return 0;
}
int lstat(const char *p, struct stat *st) { return stat(p, st); }

int open(const char *p, int f, ...) {
    pasang();
    int fd = 3;
    while (fd < 256 && pegangan[fd].buka) fd++;
    if (fd == 256) return gagal(EMFILE);
    if (strcmp(p, "/dev/urandom") == 0 || strcmp(p, "/dev/random") == 0) {
        pegangan[fd] = (Pegangan){ 0, 0, 1, 0, 1 };
        return fd;
    }
    Simpul *s = cari(p);
    if (!s) {
        if (!(f & O_CREAT)) return gagal(ENOENT);
        char *j = normal(p);
        int ok = j && induk_ada(j);
        free(j);
        if (!ok) return gagal(ENOENT);
        s = buat(p, 0);
        if (!s) return gagal(ENOMEM);
    } else if ((f & O_CREAT) && (f & O_EXCL)) return gagal(EEXIST);
    if (s->dir && (f & O_ACCMODE) != O_RDONLY) return gagal(EISDIR);
    if (f & O_TRUNC) s->n = 0;
    pegangan[fd] = (Pegangan){ s, (f & O_APPEND) ? s->n : 0, 1, (f & O_ACCMODE) != O_RDONLY, 0 };
    return fd;
}
int close(int fd) {
    if (fd < 3 || fd >= 256 || !pegangan[fd].buka) return gagal(EBADF);
    pegangan[fd].buka = 0;
    return 0;
}
long jrg_ram_baca(int fd, void *b, size_t n) {
    if (fd < 3 || fd >= 256 || !pegangan[fd].buka) return gagal(EBADF);
    Pegangan *h = &pegangan[fd];
    if (h->acak) { getentropy(b, n); return (long)n; }
    if (h->s->dir) return gagal(EISDIR);
    size_t sisa = h->pos < h->s->n ? h->s->n - h->pos : 0;
    if (n > sisa) n = sisa;
    memcpy(b, h->s->isi + h->pos, n);
    h->pos += n;
    return (long)n;
}
long jrg_ram_tulis(int fd, const void *b, size_t n) {
    if (fd < 3 || fd >= 256 || !pegangan[fd].buka || !pegangan[fd].tulis) return gagal(EBADF);
    Pegangan *h = &pegangan[fd];
    Simpul *s = h->s;
    if (h->pos + n > s->kap) {
        size_t k = (h->pos + n) * 2;
        unsigned char *baru = realloc(s->isi, k);
        if (!baru) return gagal(ENOSPC);
        s->isi = baru; s->kap = k;
    }
    if (h->pos > s->n) memset(s->isi + s->n, 0, h->pos - s->n);
    memcpy(s->isi + h->pos, b, n);
    h->pos += n;
    if (h->pos > s->n) s->n = h->pos;
    return (long)n;
}
int fstat(int fd, struct stat *st) {
    if (fd < 3 || fd >= 256 || !pegangan[fd].buka) return gagal(EBADF);
    if (pegangan[fd].acak) { memset(st, 0, sizeof *st); st->st_mode = S_IFCHR | 0444; return 0; }
    isi_stat(pegangan[fd].s, st);
    return 0;
}

int mkdir(const char *p, mode_t m) {
    (void)m;
    pasang();
    if (cari(p)) return gagal(EEXIST);
    char *j = normal(p);
    int ok = j && induk_ada(j);
    free(j);
    if (!ok) return gagal(ENOENT);
    return buat(p, 1) ? 0 : gagal(ENOMEM);
}
static int punya_anak(const Simpul *d) {
    size_t l = strlen(d->jalur);
    for (size_t i = 0; i < simpul_n; i++) {
        const Simpul *s = &simpul[i];
        if (s->ada && s != d && strncmp(s->jalur, d->jalur, l) == 0 && (l == 1 || s->jalur[l] == '/')) return 1;
    }
    return 0;
}
int rmdir(const char *p) {
    pasang();
    Simpul *s = cari(p);
    if (!s) return gagal(ENOENT);
    if (!s->dir) return gagal(ENOTDIR);
    if (punya_anak(s)) return gagal(ENOTEMPTY);
    s->ada = 0;
    return 0;
}
int unlink(const char *p) {
    pasang();
    Simpul *s = cari(p);
    if (!s) return gagal(ENOENT);
    if (s->dir) return gagal(EISDIR);
    s->ada = 0;
    return 0;
}
int rename(const char *a, const char *b) {
    pasang();
    Simpul *s = cari(a);
    if (!s) return gagal(ENOENT);
    if (s->dir && punya_anak(s)) return gagal(EXDEV);
    Simpul *t = cari(b);
    if (t && t != s) t->ada = 0;
    free(s->jalur);
    s->jalur = normal(b);
    return 0;
}
int chmod(const char *p, mode_t m) { (void)m; pasang(); return cari(p) ? 0 : gagal(ENOENT); }
ssize_t readlink(const char *p, char *b, size_t n) { (void)b; (void)n; pasang(); return gagal(cari(p) ? EINVAL : ENOENT); }
char *getcwd(char *b, size_t n) { if (!b || n < 2) { errno = ERANGE; return 0; } b[0] = '/'; b[1] = 0; return b; }

/* Direktori: daftar nama anak langsung, dibuat saat `opendir`. */
struct __dirstream { struct dirent *isi; size_t n, i; };
DIR *opendir(const char *p) {
    pasang();
    Simpul *d = cari(p);
    if (!d) { errno = ENOENT; return 0; }
    if (!d->dir) { errno = ENOTDIR; return 0; }
    DIR *h = calloc(1, sizeof *h);
    size_t l = strlen(d->jalur);
    for (size_t i = 0; h && i < simpul_n; i++) {
        const Simpul *s = &simpul[i];
        if (!s->ada || s == d || strncmp(s->jalur, d->jalur, l) != 0) continue;
        const char *ekor = s->jalur + (l == 1 ? 1 : l);
        if (l != 1 && *ekor != '/') continue;
        if (*ekor == '/') ekor++;
        if (strchr(ekor, '/')) continue;
        struct dirent *b = realloc(h->isi, (h->n + 1) * sizeof *b);
        if (!b) break;
        h->isi = b;
        memset(&b[h->n], 0, sizeof *b);
        strncpy(b[h->n].d_name, ekor, sizeof b[h->n].d_name - 1);
        b[h->n].d_type = s->dir ? DT_DIR : DT_REG;
        h->n++;
    }
    return h;
}
struct dirent *readdir(DIR *h) { return h && h->i < h->n ? &h->isi[h->i++] : 0; }
int closedir(DIR *h) { if (h) { free(h->isi); free(h); } return 0; }

/* `fopen`/`fread` (dipakai runtime untuk /dev/urandom dan setelan bahasa): di atas pegangan yang sama. */
typedef struct { int fd; } JrgFile;
void *fopen(const char *p, const char *m) {
    int f = m[0] == 'r' ? O_RDONLY : m[0] == 'a' ? (O_WRONLY | O_CREAT | O_APPEND) : (O_WRONLY | O_CREAT | O_TRUNC);
    int fd = open(p, f);
    if (fd < 0) return 0;
    JrgFile *h = malloc(sizeof *h);
    if (!h) { close(fd); return 0; }
    h->fd = fd;
    return h;
}
size_t fread(void *b, size_t a, size_t n, void *f) {
    long r = jrg_ram_baca(((JrgFile *)f)->fd, b, a * n);
    return r > 0 && a ? (size_t)r / a : 0;
}
int fclose(void *f) { if (f) { close(((JrgFile *)f)->fd); free(f); } return 0; }
