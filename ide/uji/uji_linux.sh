#!/bin/sh
# Uji Linux versi Terminal (ide/README.md, "Linux"). A: pemasang C (ide/linux/jrglinux.c) dengan arsip buatan, selalu.
# B-D: jrg-ide untuk Termux, lalu pasang Ubuntu lewat /api/linux, sesi PTY dan tombol Jalankan di dalamnya, kalau proot dan arsip Ubuntu amd64
# ada di ~/.cache/jrgm/ide-linux/ (proot statis dari proot.gitlab.io; arsip dari cdimage.ubuntu.com). Dari akar repo.
set -u
cd "$(dirname "$0")/../.."
J=${JRG_UJI_JRGC:-$PWD/target/release/jrgc}
cc=$(command -v cc || command -v gcc || command -v clang || true)
if [ -z "$cc" ] || ! command -v python3 >/dev/null; then echo "dilewati: Linux (butuh cc dan python3)"; exit 0; fi
d=$(mktemp -d); p=""; ps=""
trap 'kill $p $ps 2>/dev/null; chmod -R u+rwx "$d" 2>/dev/null; rm -rf "$d"; [ -n "$sumber" ] && rm -f ide/linux/jrg-lang.tar.gz' EXIT
sumber=""
gagal=0
cek() {  # cek <benar?> <penjelasan>
  if [ "$1" = 1 ]; then echo "ok: $2"; else echo "GAGAL: $2"; gagal=1; fi
}
"$cc" -O2 -o "$d/jl" ide/linux/jrglinux.c -ldl 2>"$d/cc.txt" || { echo "GAGAL: jrglinux.c tidak terkompilasi"; cat "$d/cc.txt"; exit 1; }

# A. Arsip buatan: folder 0555, symlink, hard link, nama > 100 huruf (PAX), lalu dua arsip jahat.
python3 - "$d" <<'EOF'
import io, sys, tarfile
d = sys.argv[1]
def tambah(t, nama, isi=None, **k):
    i = tarfile.TarInfo(nama)
    for a, v in k.items(): setattr(i, a, v)
    t.addfile(i)
with tarfile.open(d + '/a.tar.gz', 'w:gz', format=tarfile.PAX_FORMAT) as t:
    tambah(t, './usr', type=tarfile.DIRTYPE, mode=0o555)
    i = tarfile.TarInfo('./usr/env'); i.size = 5; i.mode = 0o755; t.addfile(i, io.BytesIO(b'halo\n'))
    tambah(t, './bin', type=tarfile.SYMTYPE, linkname='usr')
    tambah(t, './usr/env2', type=tarfile.LNKTYPE, linkname='./usr/env')
    i = tarfile.TarInfo('./' + 'p' * 120 + '/berkas'); i.size = 2; t.addfile(i, io.BytesIO(b'ok'))
with tarfile.open(d + '/jahat.tar.gz', 'w:gz') as t:
    i = tarfile.TarInfo('../keluar'); i.size = 1; t.addfile(i, io.BytesIO(b'x'))
with tarfile.open(d + '/lewat.tar.gz', 'w:gz') as t:
    tambah(t, 'tautan', type=tarfile.SYMTYPE, linkname=d)
    i = tarfile.TarInfo('tautan/tembus'); i.size = 1; t.addfile(i, io.BytesIO(b'x'))
EOF
"$d/jl" tarik "$d/a.tar.gz" "$d/r" >/dev/null 2>"$d/e.txt"
isi=$(cat "$d/r/usr/env" "$d/r/usr/env2" "$d/r/bin/env" 2>/dev/null | tr -d '\n')
cek "$([ "$isi" = halohalohalo ] && [ -L "$d/r/bin" ] && [ ! -L "$d/r/usr/env2" ] && echo 1)" "pemasang: berkas, symlink, hard link jadi salinan"
cek "$([ "$(cat "$d/r/$(printf 'p%.0s' $(seq 120))/berkas" 2>/dev/null)" = ok ] && echo 1)" "pemasang: nama panjang (PAX)"
cek "$([ "$(stat -c %a "$d/r/usr")" = 755 ] && echo 1)" "pemasang: folder 0555 tetap bisa ditulis pemiliknya (jadi 755)"
"$d/jl" tarik "$d/jahat.tar.gz" "$d/j" >/dev/null 2>"$d/e.txt"; k=$?
cek "$([ $k != 0 ] && grep -q 'keluar dari rootfs' "$d/e.txt" && [ ! -e "$d/keluar" ] && echo 1)" "pemasang: entri ../ ditolak"
"$d/jl" tarik "$d/lewat.tar.gz" "$d/l" >/dev/null 2>"$d/e.txt"; k=$?
cek "$([ $k != 0 ] && grep -q 'melewati symlink' "$d/e.txt" && [ ! -e "$d/tembus" ] && echo 1)" "pemasang: entri lewat symlink ditolak"
cek "$([ "$("$d/jl" sha256 "$d/a.tar.gz")" = "$(sha256sum "$d/a.tar.gz" | cut -d' ' -f1)" ] && echo 1)" "pemasang: SHA-256 sama dengan sha256sum"
chmod 555 "$d/r/usr"; "$d/jl" hapus "$d/r"
cek "$([ ! -e "$d/r" ] && echo 1)" "pemasang: hapus pohon berisi folder tanpa izin tulis"

# B. Pelayan IDE; C. jrg-ide dari Termux (linux/termux.sh) lewat pelayan itu; D. Ubuntu sungguhan kalau ada di cache.
command -v curl >/dev/null || { echo "dilewati: pelayan (butuh curl)"; exit $gagal; }
# Arsip sumber JRG buatan bangun_apk.sh, versi mini (perintah jrg-sumber dan tombol Lainnya > Linux).
if [ ! -e ide/linux/jrg-lang.tar.gz ]; then
  mkdir -p "$d/sm/jrg-lang" && echo "sumber-jrg" >"$d/sm/jrg-lang/README.md" && tar -C "$d/sm" -czf ide/linux/jrg-lang.tar.gz jrg-lang && sumber=1
fi
c=$HOME/.cache/jrgm/ide-linux
port=$((49000 + $$ % 900)); pa=$((port + 1))
( cd "$c" 2>/dev/null && exec python3 -m http.server $pa --bind 127.0.0.1 ) >/dev/null 2>&1 &
ps=$!
mkdir -p "$d/h"
( cd ide && exec env HOME="$d/h" PORT=$port JRG_JRGC="$J" JRG_SOPAN=1 JRG_IDE_SEMBUNYI=python JRG_IDE_PROOT="$c/proot" JRG_IDE_JRGLINUX="$d/jl" \
    JRG_IDE_LINUX_URL="http://127.0.0.1:$pa/" "$J" src/main.jrg ) >"$d/pelayan.txt" 2>&1 &
p=$!
u=http://127.0.0.1:$port
i=0; while [ $i -lt 80 ] && ! curl -s -o /dev/null "$u/api/linux"; do i=$((i + 1)); sleep 0.25; done
curl -s "$u/termux.sh" >"$d/jrg-ide"
head -c 1000000 /dev/urandom >"$d/acak.bin"
( cd "$d" && sh ./jrg-ide kirim acak.bin tx/acak.bin >/dev/null && sh ./jrg-ide ls tx && mkdir -p kembali && sh ./jrg-ide ambil tx/acak.bin kembali ) >"$d/tx.txt" 2>&1
cek "$(grep -qx 'tx/acak.bin' "$d/tx.txt" && cmp -s "$d/acak.bin" "$d/kembali/acak.bin" && grep -q "PORT=\${JRG_IDE_PORT:-$port}" "$d/jrg-ide" && echo 1)" "Termux: jrg-ide kirim, ls, ambil berkas biner 1 MB utuh lewat pelayan"
# Tautan di folder berkas (setup-storage lama, atau buatan pemakai): putus dan ke folder luar tidak boleh menggagalkan daftar.
mkdir -p "$d/h/jrg-ide/st" && ln -s /tidak/ada "$d/h/jrg-ide/st/putus" && ln -s "$d" "$d/h/jrg-ide/st/luar"
k=$(curl -s -o "$d/b.json" -w '%{http_code}' "$u/api/berkas"); f=$(curl -s "$u/api/folder")
cek "$([ "$k" = 200 ] && grep -q tx/acak.bin "$d/b.json" && ! grep -q 'st/' "$d/b.json" && case "$f" in *st/luar*) ;; *) echo 1;; esac)" "berkas: tautan putus dan tautan ke folder luar dilewati daftar berkas dan pohon"
JRG_IDE_PORT=1 sh "$d/jrg-ide" ls >/dev/null 2>"$d/e.txt"; k=$?
cek "$([ $k != 0 ] && grep -q 'tidak menjawab' "$d/e.txt" && echo 1)" "Termux: IDE yang tidak jalan dilaporkan"
arsip=$(ls "$c"/ubuntu-base-24.04.5-base-amd64.tar.gz 2>/dev/null || true)
if [ ! -x "$c/proot" ] || [ -z "$arsip" ] || [ "$(uname -m)" != x86_64 ]; then
  echo "dilewati: Ubuntu sungguhan (taruh proot dan ubuntu-base-24.04.5-base-amd64.tar.gz di $c)"; exit $gagal
fi
curl -s -X POST "$u/api/linux/pasang" -d '{}' >/dev/null
i=0; while [ $i -lt 240 ]; do r=$(curl -s "$u/api/linux"); case "$r" in *'"tahap":"siap"'*|*'"tahap":"gagal"'*) break;; esac; i=$((i + 1)); sleep 0.5; done
cek "$(case "$r" in *'"tahap":"siap"'*'"terpasang":true'*) echo 1;; esac)" "Linux: unduh, sidik, bongkar, uji mesin sampai siap ($(printf '%s' "$r" | sed -n 's/.*"pesan":"\([^"]*\)".*/\1/p'))"
# Paket dasar (apt, butuh internet) dilewati: tandanya dipasang sebelum sesi pertama.
touch "$d/h/jrg-linux/ubuntu/rootfs/var/lib/jrg-ide/setup-done"
curl -s -X POST "$u/api/pty/buka" -d '{"kolom":80,"baris":24,"linux":true}' >"$d/s.txt"
sid=$(sed -n 's/.*"id":\([0-9]*\).*/\1/p' "$d/s.txt")
curl -s -X POST "$u/api/pty/tulis" -d "{\"id\":$sid,\"teks\":\". /etc/os-release; echo \\\"DI-\$ID-\$((6*7))\\\"\\r\"}" >/dev/null
i=0; while [ $i -lt 60 ]; do o=$(curl -s -X POST "$u/api/pty/baca" -d "{\"sesi\":[[$sid,0]],\"ms\":200}"); case "$o" in *DI-ubuntu-42*) break;; esac; i=$((i + 1)); done
cek "$(case "$o" in *DI-ubuntu-42*) echo 1;; esac)" "Linux: sesi terminal Ubuntu (PTY di dalam proot)"
# Rasa Linux: prompt user@host:~/berkas, sudo (sudah root), ~/berkas ke folder IDE, jrg-sumber.
if [ -n "$sumber" ]; then
  python3 -c 'import json, sys; print(json.dumps({"id": int(sys.argv[1]), "teks": "cd ~/berkas; sudo -E sh -c \x27echo SUDO-$(id -u)\x27; jrg-sumber >/dev/null && cat ~/berkas/jrg-lang/README.md\r"}))' "$sid" >"$d/t.json"
  curl -s -X POST "$u/api/pty/tulis" --data-binary @"$d/t.json" >/dev/null
  i=0; while [ $i -lt 60 ]; do o=$(curl -s -X POST "$u/api/pty/baca" -d "{\"sesi\":[[$sid,0]],\"ms\":200}"); case "$o" in *sumber-jrg*) break;; esac; i=$((i + 1)); done
  cek "$(case "$o" in *'root@'*'~/berkas'*SUDO-0*sumber-jrg*) echo 1;; esac)" "Linux: prompt root@host:~/berkas, sudo, jrg-sumber membongkar sumber JRG"
  k1=$(curl -s -X POST "$u/api/linux/sumber" -d '{}')
  rm -rf "$d/h/jrg-ide/jrg-lang"; k2=$(curl -s -X POST "$u/api/linux/sumber" -d '{}')
  cek "$(case "$k1" in *'sudah ada'*) case "$k2" in *'"ok":true'*) [ "$(cat "$d/h/jrg-ide/jrg-lang/README.md")" = sumber-jrg ] && echo 1;; esac;; esac)" "Lainnya > Linux: sumber JRG dibongkar ke berkas/jrg-lang, yang sudah ada tidak ditimpa"
fi
# Python di tamu tidak ada di arsip dasar: penjalannya diganti sh supaya jalur tombol Jalankan teruji tanpa internet.
mkdir -p "$d/h/jrg-linux/ubuntu/rootfs/usr/local/bin" && printf '#!/bin/sh\nshift\nexec /bin/sh -c "echo py-di-\\$(. /etc/os-release; echo \\$ID) \\$*" x "$@"\n' >"$d/h/jrg-linux/ubuntu/rootfs/usr/local/bin/python3"
chmod 755 "$d/h/jrg-linux/ubuntu/rootfs/usr/local/bin/python3"
printf 'print(1)\n' >"$d/h/jrg-ide/lx.py"
id=$(curl -s -X POST "$u/api/mulai" -d '{"nama":"lx.py","argumen":["a"]}' | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
i=0; k=""; while [ $i -lt 40 ]; do b=$(curl -s -X POST "$u/api/baca" -d "{\"id\":$id}"); k="$k$b"; case "$b" in *'"selesai":true'*) break;; esac; i=$((i + 1)); sleep 0.25; done
cek "$(case "$k" in *py-di-ubuntu*lx.py\ a*'"sukses":true'*) echo 1;; esac)" "Linux: tombol Jalankan memakai penjalan di Ubuntu kalau perangkat tidak punya"
curl -s -X POST "$u/api/linux/hapus" -d '{}' >/dev/null
cek "$([ ! -e "$d/h/jrg-linux/ubuntu" ] && echo 1)" "Linux: dihapus beserta isinya"
[ $gagal = 0 ] || tail -5 "$d/pelayan.txt"
exit $gagal
