/* JRG IDE — linux/jrglinux.c: pemasang rootfs Linux untuk versi Terminal (ide/README.md, "Linux").
   Klien HTTP JRG belum bisa HTTPS dan menyimpan badan di memori, jadi unduhan 30 MB, SHA-256 dan
   pembongkaran tar.gz dikerjakan di sini. zlib dimuat saat jalan (dlopen), tanpa bendera tautan. */
#define _GNU_SOURCE
#include <dirent.h>
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <netdb.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <unistd.h>
#include <zlib.h>

static void mati(const char *f, const char *a) {
  fprintf(stderr, f, a);
  fputc('\n', stderr);
  exit(1);
}

/* ---------- SHA-256 (FIPS 180-4) ---------- */
typedef struct { uint32_t h[8]; uint8_t b[64]; uint64_t n; } Sha;
static const uint32_t K[64] = {
  0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,
  0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
  0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,
  0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
  0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,
  0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2};
#define ROR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))
static void sha_blok(Sha *s, const uint8_t *p) {
  uint32_t w[64], a, b, c, d, e, f, g, h;
  for (int i = 0; i < 16; i++) w[i] = (uint32_t)p[4*i] << 24 | (uint32_t)p[4*i+1] << 16 | (uint32_t)p[4*i+2] << 8 | p[4*i+3];
  for (int i = 16; i < 64; i++) {
    uint32_t s0 = ROR(w[i-15], 7) ^ ROR(w[i-15], 18) ^ (w[i-15] >> 3), s1 = ROR(w[i-2], 17) ^ ROR(w[i-2], 19) ^ (w[i-2] >> 10);
    w[i] = w[i-16] + s0 + w[i-7] + s1;
  }
  a = s->h[0]; b = s->h[1]; c = s->h[2]; d = s->h[3]; e = s->h[4]; f = s->h[5]; g = s->h[6]; h = s->h[7];
  for (int i = 0; i < 64; i++) {
    uint32_t t1 = h + (ROR(e, 6) ^ ROR(e, 11) ^ ROR(e, 25)) + ((e & f) ^ (~e & g)) + K[i] + w[i];
    uint32_t t2 = (ROR(a, 2) ^ ROR(a, 13) ^ ROR(a, 22)) + ((a & b) ^ (a & c) ^ (b & c));
    h = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
  }
  s->h[0] += a; s->h[1] += b; s->h[2] += c; s->h[3] += d; s->h[4] += e; s->h[5] += f; s->h[6] += g; s->h[7] += h;
}
static void sha_mulai(Sha *s) {
  static const uint32_t awal[8] = {0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19};
  memcpy(s->h, awal, sizeof awal);
  s->n = 0;
}
static void sha_isi(Sha *s, const uint8_t *p, size_t n) {
  while (n) {
    size_t i = s->n % 64, m = 64 - i < n ? 64 - i : n;
    memcpy(s->b + i, p, m);
    s->n += m; p += m; n -= m;
    if (s->n % 64 == 0) sha_blok(s, s->b);
  }
}
static void sha_hex(Sha *s, char out[65]) {
  uint64_t bit = s->n * 8;
  uint8_t pad = 0x80, nol = 0, panjang[8];
  sha_isi(s, &pad, 1);
  while (s->n % 64 != 56) sha_isi(s, &nol, 1);
  for (int i = 0; i < 8; i++) panjang[i] = (uint8_t)(bit >> (56 - 8 * i));
  sha_isi(s, panjang, 8);
  for (int i = 0; i < 8; i++) sprintf(out + 8 * i, "%08x", s->h[i]);
}
static int sha_berkas(const char *jalur, char out[65]) {
  FILE *f = fopen(jalur, "rb");
  if (!f) return -1;
  Sha s; uint8_t buf[65536]; size_t n;
  sha_mulai(&s);
  while ((n = fread(buf, 1, sizeof buf, f)) > 0) sha_isi(&s, buf, n);
  fclose(f);
  sha_hex(&s, out);
  return 0;
}

/* ---------- unduh: HTTP/1.1 polos, ikut pengalihan, badan biasa atau chunked ---------- */
static int sambung(const char *host, const char *port) {
  struct addrinfo hint = {0}, *ai, *p;
  hint.ai_socktype = SOCK_STREAM;
  int e = getaddrinfo(host, port, &hint, &ai);
  if (e) mati("tidak bisa menemukan server: %s (periksa internet)", host);
  int fd = -1;
  for (p = ai; p; p = p->ai_next) {
    fd = socket(p->ai_family, p->ai_socktype, p->ai_protocol);
    if (fd < 0) continue;
    struct timeval tv = {60, 0};
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof tv);
    setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof tv);
    if (connect(fd, p->ai_addr, p->ai_addrlen) == 0) break;
    close(fd); fd = -1;
  }
  freeaddrinfo(ai);
  if (fd < 0) mati("tidak bisa tersambung ke %s", host);
  return fd;
}
typedef struct { int fd; char b[65536]; size_t i, n; } Baca;
static int bc(Baca *r) {
  if (r->i == r->n) {
    ssize_t k = recv(r->fd, r->b, sizeof r->b, 0);
    if (k <= 0) return -1;
    r->i = 0; r->n = (size_t)k;
  }
  return (unsigned char)r->b[r->i++];
}
static int baris(Baca *r, char *out, size_t maks) {
  size_t n = 0; int c;
  while ((c = bc(r)) >= 0 && c != '\n') if (n + 1 < maks) out[n++] = (char)c;
  if (n && out[n - 1] == '\r') n--;
  out[n] = 0;
  return c < 0 && n == 0 ? -1 : 0;
}
static void unduh(const char *url, const char *tujuan) {
  char u[4096], kini[4096];
  snprintf(u, sizeof u, "%s", url);
  for (int lompat = 0; lompat < 6; lompat++) {
    if (strncmp(u, "http://", 7)) mati("cuma http:// yang bisa diunduh di sini (sidiknya dipatok di IDE): %s", u);
    char host[256], port[8] = "80", *jalur = strchr(u + 7, '/');
    size_t hl = jalur ? (size_t)(jalur - u - 7) : strlen(u + 7);
    if (hl >= sizeof host) mati("nama server terlalu panjang: %s", u);
    memcpy(host, u + 7, hl); host[hl] = 0;
    char *tk = strchr(host, ':');
    if (tk) { *tk = 0; snprintf(port, sizeof port, "%s", tk + 1); }
    int fd = sambung(host, port);
    char minta[2600];
    int m = snprintf(minta, sizeof minta, "GET %s HTTP/1.1\r\nHost: %s\r\nUser-Agent: JRG-IDE\r\nAccept-Encoding: identity\r\nConnection: close\r\n\r\n", jalur ? jalur : "/", host);
    if (send(fd, minta, (size_t)m, 0) != m) mati("permintaan ke %s gagal dikirim", host);
    Baca *r = calloc(1, sizeof *r);
    r->fd = fd;
    char h[4096];
    if (baris(r, h, sizeof h) < 0) mati("server %s tidak menjawab", host);
    int kode = 0;
    sscanf(h, "HTTP/%*s %d", &kode);
    long long total = -1; int chunked = 0; kini[0] = 0;
    while (baris(r, h, sizeof h) == 0 && h[0]) {
      if (!strncasecmp(h, "content-length:", 15)) total = atoll(h + 15);
      else if (!strncasecmp(h, "transfer-encoding:", 18) && strcasestr(h, "chunked")) chunked = 1;
      else if (!strncasecmp(h, "location:", 9)) { char *v = h + 9; while (*v == ' ') v++; snprintf(kini, sizeof kini, "%s", v); }
    }
    if (kode >= 300 && kode < 400 && kini[0]) {
      if (kini[0] == '/') snprintf(u, sizeof u, "http://%s%s%s%s", host, tk ? ":" : "", tk ? port : "", kini);
      else snprintf(u, sizeof u, "%s", kini);
      close(fd); free(r);
      continue;
    }
    if (kode != 200) { char k[16]; snprintf(k, sizeof k, "%d", kode); mati("server menjawab HTTP %s", k); }
    char sementara[4096];
    snprintf(sementara, sizeof sementara, "%s.sebagian", tujuan);
    FILE *f = fopen(sementara, "wb");
    if (!f) mati("tidak bisa menulis %s", sementara);
    long long dapat = 0, lapor = 0, sisa = chunked ? 0 : total;
    for (;;) {
      if (chunked && sisa == 0) {
        if (dapat > 0 && baris(r, h, sizeof h) < 0) break;
        if (baris(r, h, sizeof h) < 0) mati("unduhan terputus%s", "");
        sisa = strtoll(h, NULL, 16);
        if (sisa == 0) break;
      }
      if (r->i == r->n) {
        ssize_t k = recv(fd, r->b, sizeof r->b, 0);
        if (k < 0) mati("unduhan terputus (waktu habis atau jaringan hilang)%s", "");
        if (k == 0) break;
        r->i = 0; r->n = (size_t)k;
      }
      size_t ada = r->n - r->i;
      if (sisa >= 0 && (long long)ada > sisa) ada = (size_t)sisa;
      if (fwrite(r->b + r->i, 1, ada, f) != ada) mati("penyimpanan penuh saat menulis %s", sementara);
      r->i += ada; dapat += (long long)ada;
      if (sisa > 0) sisa -= (long long)ada;
      if (!chunked && sisa == 0) break;
      if (dapat - lapor >= 262144) { lapor = dapat; printf("unduh %lld %lld\n", dapat, total); fflush(stdout); }
    }
    fclose(f); close(fd); free(r);
    if (total >= 0 && dapat != total) { remove(sementara); mati("unduhan terpotong; coba lagi%s", ""); }
    if (rename(sementara, tujuan)) mati("tidak bisa memindah %s", sementara);
    printf("unduh %lld %lld\n", dapat, total);
    return;
  }
  mati("terlalu banyak pengalihan: %s", url);
}

/* ---------- tarik: tar.gz (ustar, GNU L/K, PAX) ke direktori, aturannya sama dengan Woksole ---------- */
typedef int (*f_init)(z_streamp, int, const char *, int);
typedef int (*f_inflate)(z_streamp, int);
static f_init z_init; static f_inflate z_inflate, z_end;
static void muat_zlib(void) {
  const char *nama[] = {"libz.so", "libz.so.1", NULL};
  void *h = NULL;
  for (int i = 0; nama[i] && !h; i++) h = dlopen(nama[i], RTLD_NOW);
  if (!h) mati("zlib tidak ada di perangkat ini: %s", dlerror());
  z_init = (f_init)dlsym(h, "inflateInit2_");
  z_inflate = (f_inflate)dlsym(h, "inflate");
  z_end = (f_inflate)dlsym(h, "inflateEnd");
  if (!z_init || !z_inflate || !z_end) mati("zlib tidak lengkap%s", "");
}
typedef struct { FILE *f; z_stream z; uint8_t masuk[65536]; long long dibaca, total, lapor; int habis; } Gz;
static size_t gz_baca(Gz *g, uint8_t *out, size_t n) {
  g->z.next_out = out; g->z.avail_out = (uInt)n;
  while (g->z.avail_out && !g->habis) {
    if (!g->z.avail_in) {
      size_t k = fread(g->masuk, 1, sizeof g->masuk, g->f);
      if (!k) mati("arsip terpotong%s", "");
      g->dibaca += (long long)k;
      g->z.next_in = g->masuk; g->z.avail_in = (uInt)k;
      if (g->dibaca - g->lapor >= 524288) { g->lapor = g->dibaca; printf("tarik %lld %lld\n", g->dibaca, g->total); fflush(stdout); }
    }
    int e = z_inflate(&g->z, Z_NO_FLUSH);
    if (e == Z_STREAM_END) g->habis = 1;
    else if (e != Z_OK) mati("arsip rusak (gzip)%s", "");
  }
  return n - g->z.avail_out;
}
static void gz_penuh(Gz *g, uint8_t *out, size_t n) { if (gz_baca(g, out, n) != n) mati("arsip terpotong%s", ""); }
static void gz_lewati(Gz *g, long long n) {
  uint8_t buf[65536];
  while (n > 0) { size_t m = n > (long long)sizeof buf ? sizeof buf : (size_t)n; gz_penuh(g, buf, m); n -= (long long)m; }
}
static long long angka(const uint8_t *h, int off, int len) {
  long long v = 0;
  if (h[off] & 0x80) { for (int i = off + 1; i < off + len; i++) v = (v << 8) | h[i]; return v; }
  for (int i = off; i < off + len; i++) {
    int c = h[i];
    if (c == 0 || c == ' ') { if (v) break; else continue; }
    if (c < '0' || c > '7') mati("header tar tidak sah%s", "");
    v = v * 8 + (c - '0');
  }
  return v;
}
static char *teks_gz(Gz *g, long long n) {
  if (n > (1 << 20)) mati("header tambahan tar terlalu besar%s", "");
  char *s = malloc((size_t)n + 1);
  gz_penuh(g, (uint8_t *)s, (size_t)n);
  s[n] = 0;
  gz_lewati(g, (512 - n % 512) % 512);
  return s;
}
static void pax(const char *t, char **jalur, char **tautan, long long *ukuran) {
  const char *p = t;
  while (*p) {
    long len = strtol(p, NULL, 10);
    const char *sp = strchr(p, ' ');
    if (len <= 0 || !sp) break;
    const char *akhir = p + len, *eq = strchr(sp, '=');
    if (eq && eq < akhir) {
      size_t kl = (size_t)(eq - sp - 1), vl = (size_t)(akhir - eq - 2);
      char *v = strndup(eq + 1, vl);
      if (kl == 4 && !strncmp(sp + 1, "path", 4)) { free(*jalur); *jalur = v; }
      else if (kl == 8 && !strncmp(sp + 1, "linkpath", 8)) { free(*tautan); *tautan = v; }
      else { if (kl == 4 && !strncmp(sp + 1, "size", 4)) *ukuran = atoll(v); free(v); }
    }
    p = akhir;
  }
}
/* `nama` dinormalkan di dalam `tujuan`; ".." yang keluar dan induk yang ternyata symlink ditolak. */
static char *di_dalam(const char *tujuan, const char *nama) {
  char *bagian[512]; int n = 0;
  char *salin = strdup(nama), *sp, *tok = strtok_r(salin, "/", &sp);
  for (; tok; tok = strtok_r(NULL, "/", &sp)) {
    if (!strcmp(tok, ".") || !*tok) continue;
    if (!strcmp(tok, "..")) { if (!n) mati("entri keluar dari rootfs: %s", nama); n--; continue; }
    if (n == 512) mati("jalur terlalu dalam: %s", nama);
    bagian[n++] = tok;
  }
  if (!n) { free(salin); return NULL; }
  size_t pj = strlen(tujuan) + 2;
  for (int i = 0; i < n; i++) pj += strlen(bagian[i]) + 1;
  char *out = malloc(pj);
  strcpy(out, tujuan);
  struct stat st;
  for (int i = 0; i < n; i++) {
    strcat(out, "/"); strcat(out, bagian[i]);
    if (i < n - 1 && lstat(out, &st) == 0 && S_ISLNK(st.st_mode)) mati("entri melewati symlink: %s", nama);
  }
  free(salin);
  return out;
}
static void buat_dir(char *p) {
  struct stat st;
  if (stat(p, &st) == 0 && S_ISDIR(st.st_mode)) return;
  for (char *s = p + 1; *s; s++) if (*s == '/') { *s = 0; mkdir(p, 0755); *s = '/'; }
  if (mkdir(p, 0755) && errno != EEXIST) mati("tidak bisa membuat folder %s", p);
}
static void buat_induk(const char *p) {
  char *d = strdup(p), *g = strrchr(d, '/');
  if (g && g != d) { *g = 0; buat_dir(d); }
  free(d);
}
static void hapus(const char *p);
static void bersihkan(const char *p) {
  struct stat st;
  if (lstat(p, &st) == 0) { if (S_ISDIR(st.st_mode)) hapus(p); else unlink(p); }
}
typedef struct { char *jalur; int mode; } Dir;
static void tarik(const char *arsip, const char *tujuan) {
  muat_zlib();
  Gz *g = calloc(1, sizeof *g);
  g->f = fopen(arsip, "rb");
  if (!g->f) mati("arsip tidak bisa dibuka: %s", arsip);
  struct stat st;
  fstat(fileno(g->f), &st);
  g->total = st.st_size;
  if (z_init(&g->z, 15 + 32, ZLIB_VERSION, (int)sizeof(z_stream)) != Z_OK) mati("zlib tidak bisa dimulai%s", "");
  char *dest = strdup(tujuan);
  buat_dir(dest);
  char *panjang = NULL, *tautan_panjang = NULL, *pjalur = NULL, *ptautan = NULL;
  long long pukuran = -1, entri = 0;
  Dir *dirs = NULL; size_t ndir = 0, kap = 0;
  uint8_t h[512], buf[65536];
  for (;;) {
    if (gz_baca(g, h, 512) < 512) break;
    int nol = 1;
    for (int i = 0; i < 512 && nol; i++) nol = !h[i];
    if (nol) break;
    long long simpan = angka(h, 148, 8), jumlah = 0;
    for (int i = 0; i < 512; i++) jumlah += (i >= 148 && i < 156) ? ' ' : h[i];
    if (simpan != jumlah) mati("checksum header tar salah (berkas rusak?)%s", "");
    char jenis = (char)h[156];
    long long ukuran = angka(h, 124, 12);
    int mode = (int)angka(h, 100, 8) & 07777;
    if (jenis == 'L') { free(panjang); panjang = teks_gz(g, ukuran); continue; }
    if (jenis == 'K') { free(tautan_panjang); tautan_panjang = teks_gz(g, ukuran); continue; }
    if (jenis == 'x' || jenis == 'g') { char *t = teks_gz(g, ukuran); pax(t, &pjalur, &ptautan, &pukuran); free(t); continue; }
    char nama[512], tautan[101];
    if (panjang) snprintf(nama, sizeof nama, "%s", panjang);
    else if (pjalur) snprintf(nama, sizeof nama, "%s", pjalur);
    else if (!memcmp(h + 257, "ustar", 5) && h[345]) snprintf(nama, sizeof nama, "%.155s/%.100s", (char *)h + 345, (char *)h);
    else snprintf(nama, sizeof nama, "%.100s", (char *)h);
    snprintf(tautan, sizeof tautan, "%.100s", (char *)h + 157);
    const char *tt = tautan_panjang ? tautan_panjang : ptautan ? ptautan : tautan;
    char *tts = strdup(tt);
    if (pukuran >= 0) ukuran = pukuran;
    free(panjang); free(tautan_panjang); free(pjalur); free(ptautan);
    panjang = tautan_panjang = pjalur = ptautan = NULL; pukuran = -1;
    char *t = di_dalam(dest, nama);
    if (!t) { gz_lewati(g, ukuran + (512 - ukuran % 512) % 512); free(tts); continue; }
    entri++;
    if (jenis == '0' || jenis == 0 || jenis == '7') {
      buat_induk(t);
      struct stat ls;
      if (lstat(t, &ls) == 0 && (S_ISLNK(ls.st_mode) || S_ISDIR(ls.st_mode))) bersihkan(t);
      int fd = open(t, O_WRONLY | O_CREAT | O_TRUNC, 0600);
      if (fd < 0) mati("tidak bisa menulis %s", t);
      long long sisa = ukuran;
      while (sisa > 0) {
        size_t m = sisa > (long long)sizeof buf ? sizeof buf : (size_t)sisa;
        gz_penuh(g, buf, m);
        if (write(fd, buf, m) != (ssize_t)m) mati("penyimpanan penuh saat menulis %s", t);
        sisa -= (long long)m;
      }
      close(fd);
      chmod(t, (mode_t)(mode | 0600));
      gz_lewati(g, (512 - ukuran % 512) % 512);
    } else if (jenis == '5') {
      buat_dir(t);
      if (ndir == kap) { kap = kap ? kap * 2 : 256; dirs = realloc(dirs, kap * sizeof *dirs); }
      dirs[ndir].jalur = strdup(t); dirs[ndir].mode = mode; ndir++;
      gz_lewati(g, ukuran + (512 - ukuran % 512) % 512);
    } else if (jenis == '2') {
      buat_induk(t);
      bersihkan(t);
      if (symlink(tts, t)) mati("tidak bisa membuat symlink %s", t);
      gz_lewati(g, ukuran + (512 - ukuran % 512) % 512);
    } else if (jenis == '1') {
      /* Android melarang hard link untuk aplikasi: diganti salinan berkasnya. */
      char *asal = di_dalam(dest, tts);
      buat_induk(t);
      bersihkan(t);
      FILE *a = asal ? fopen(asal, "rb") : NULL, *b = a ? fopen(t, "wb") : NULL;
      size_t k;
      if (a && b) while ((k = fread(buf, 1, sizeof buf, a)) > 0) fwrite(buf, 1, k, b);
      if (a) fclose(a);
      if (b) fclose(b);
      if (asal && stat(asal, &st) == 0) chmod(t, st.st_mode | 0600);
      free(asal);
      gz_lewati(g, ukuran + (512 - ukuran % 512) % 512);
    } else gz_lewati(g, ukuran + (512 - ukuran % 512) % 512);
    free(t); free(tts);
  }
  /* Mode folder diterapkan terakhir, yang terdalam dulu; pemilik selalu boleh masuk dan menulis. */
  while (ndir) { ndir--; chmod(dirs[ndir].jalur, (mode_t)(dirs[ndir].mode | 0700)); free(dirs[ndir].jalur); }
  z_end(&g->z, 0);
  fclose(g->f);
  printf("tarik %lld %lld\nentri %lld\n", g->total, g->total, entri);
}

/* ---------- hapus: pohon folder tanpa mengikuti symlink; folder tanpa izin tulis dibuka dulu ---------- */
static void hapus(const char *p) {
  struct stat st;
  if (lstat(p, &st)) return;
  if (!S_ISDIR(st.st_mode)) { unlink(p); return; }
  chmod(p, 0700);
  DIR *d = opendir(p);
  if (d) {
    struct dirent *e;
    while ((e = readdir(d))) {
      if (!strcmp(e->d_name, ".") || !strcmp(e->d_name, "..")) continue;
      size_t n = strlen(p) + strlen(e->d_name) + 2;
      char *anak = malloc(n);
      snprintf(anak, n, "%s/%s", p, e->d_name);
      hapus(anak);
      free(anak);
    }
    closedir(d);
  }
  rmdir(p);
}

int main(int argc, char **argv) {
  setvbuf(stdout, NULL, _IOLBF, 0);
  if (argc == 4 && !strcmp(argv[1], "unduh")) unduh(argv[2], argv[3]);
  else if (argc == 3 && !strcmp(argv[1], "sha256")) {
    char hex[65];
    if (sha_berkas(argv[2], hex)) mati("tidak bisa membaca %s", argv[2]);
    printf("%s\n", hex);
  } else if (argc == 4 && !strcmp(argv[1], "tarik")) tarik(argv[2], argv[3]);
  else if (argc == 3 && !strcmp(argv[1], "hapus")) hapus(argv[2]);
  else {
    fprintf(stderr, "pakai: jrglinux unduh <url> <berkas> | sha256 <berkas> | tarik <arsip.tar.gz> <folder> | hapus <folder>\n");
    return 2;
  }
  return 0;
}
