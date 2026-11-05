#!/bin/sh
# Uji mesin Native lewat Ubuntu (versi Terminal, ide/README.md "Linux"): jrgc dijalankan di dalam rootfs dengan clang
# Ubuntu dan runtime glibc bawaan APK, tanpa cargo. Butuh rootfs ber-clang (JRG_UJI_ROOTFS, disalin dulu), runtime
# glibc untuk rootfs itu (JRG_UJI_JRGRT) dan proot; di Termux: proot-distro ubuntu + `pkg install clang`. Dari akar repo.
set -u
cd "$(dirname "$0")/../.."
J=${JRG_UJI_JRGC:-$PWD/target/release/jrgc}
R=${JRG_UJI_ROOTFS:-}; RT=${JRG_UJI_JRGRT:-}; PR=$(command -v proot || true)
if [ -z "$R" ] || [ ! -x "$R/usr/bin/clang" ] || [ ! -f "$RT" ] || [ -z "$PR" ] || ! command -v curl >/dev/null; then
  echo "dilewati: Native di Ubuntu (JRG_UJI_ROOTFS ber-clang, JRG_UJI_JRGRT, proot, curl)"; exit 0
fi
d=$(mktemp -d); p=""; taruh=""
port=$((47000 + $$ % 900)); u=http://127.0.0.1:$port
trap 'kill $p 2>/dev/null; [ -n "$taruh" ] && rm -rf ide/linux/jrgrt; rm -rf "$d"' EXIT
gagal=0
cek() { if [ "$1" = 1 ]; then echo "ok: $2"; else echo "GAGAL: $2"; gagal=1; fi; }
mkdir -p "$d/h/jrg-linux/ubuntu" "$d/h/jrg-ide" "$d/p" "$d/bin"
cp -a "$R" "$d/h/jrg-linux/ubuntu/rootfs" && rm -rf "$d/h/jrg-linux/ubuntu/rootfs/var/lib/jrg-ide"
echo '{"versi":"uji","seccomp":true}' >"$d/h/jrg-linux/ubuntu/meta.json"
# Pustaka APK tiruan: proot (dan loader-nya di Termux) plus jrgc di folder yang diikat ke dalam tamu.
ln -s "$PR" "$d/p/libproot.so"
[ -n "${PREFIX:-}" ] && [ -e "$PREFIX/libexec/proot/loader" ] && ln -s "$PREFIX/libexec/proot/loader" "$d/p/libproot-loader.so"
cp "$J" "$d/p/libjrgc.so"
[ -e ide/linux/jrgrt ] || { mkdir -p ide/linux/jrgrt && cp "$RT" ide/linux/jrgrt/libjrgrt.a && taruh=1; }
# Clang perangkat disembunyikan, supaya yang teruji jalur Ubuntu.
for f in $(dirname "$(command -v sh)")/*; do case $(basename "$f") in clang*|*-clang*|cc|gcc|g++|c++|cpp|cargo|rustc) ;; *) ln -s "$f" "$d/bin/";; esac; done
( cd ide && exec env PATH="$d/bin:/system/bin" HOME="$d/h" PORT=$port JRG_JRGC="$d/p/libjrgc.so" JRG_PUSTAKA="$d/p" JRG_SOPAN=1 \
    "$J" src/main.jrg ) >"$d/log" 2>&1 &
p=$!
i=0; while [ $i -lt 120 ] && ! curl -s -o /dev/null "$u/api/info"; do i=$((i + 1)); sleep 0.25; done
post() { curl -s -X POST "$u$1" -d "$2"; }
cek "$(case "$(curl -s "$u/api/info")" in *'"llvm":true'*) echo 1;; esac)" "pilihan Native muncul kalau clang ada di Ubuntu"
post /api/simpan '{"nama":"n.jrg","isi":"struct T { pub x: int }\nfn main() {\n    var s = 0\n    for i in 0..1000 { s += i }\n    wok(\"hasil\", s, T { x: 7 }.x, str::upper(\"native\"))\n}\n"}' >/dev/null
id=$(post /api/mulai '{"nama":"n.jrg","mesin":"llvm"}' | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
i=0; h=""; while [ $i -lt 240 ]; do h=$(post /api/baca "{\"id\":$id}"); case "$h" in *'"selesai":true'*) break;; esac; i=$((i + 1)); sleep 0.5; done
lib="$d/h/jrg-linux/ubuntu/rootfs/var/lib/jrg-ide/jrgrt/target"
cek "$(case "$h" in *'hasil 499500 7 NATIVE'*'"sukses":true'*) [ "$(ls "$lib")" = release ] && echo 1;; esac)" "Native di Ubuntu: runtime bawaan, tanpa cargo"
[ $gagal = 0 ] || { echo "$h"; tail -n 5 "$d/log"; }
exit $gagal
