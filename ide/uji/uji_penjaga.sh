#!/bin/sh
# Uji penjaga pelayan (ide/README.md, "Pelayan di HP"): proses yang dibunuh aktivitas Android cuma penjaga,
# pelayan dan sesi terminalnya tetap hidup; versi baru mengganti pelayan lama. Juga: Origin situs lain ditolak.
set -u
cd "$(dirname "$0")/../.."
J=${JRG_UJI_JRGC:-$PWD/target/release/jrgc}
command -v curl >/dev/null || { echo "dilewati: penjaga (butuh curl)"; exit 0; }
d=$(mktemp -d); g=""
port=$((47000 + $$ % 900)); u=http://127.0.0.1:$port
trap 'kill $g 2>/dev/null; curl -s -X POST "$u/api/keluar" -d "{}" >/dev/null 2>&1; rm -rf "$d"' EXIT
gagal=0
cek() { if [ "$1" = 1 ]; then echo "ok: $2"; else echo "GAGAL: $2"; gagal=1; fi; }
jaga() {
  ( cd ide && exec env HOME="$d" PORT=$port JRG_JRGC="$J" JRG_SOPAN=1 JRG_IDE_JAGA=1 "$J" src/main.jrg ) >>"$d/penjaga.txt" 2>&1 &
  g=$!
  i=0; while [ $i -lt 120 ] && ! curl -s -o /dev/null "$u/api/jejak"; do i=$((i + 1)); sleep 0.25; done
}
pid() { curl -s "$u/api/jejak" | sed -n 's/.*"pid":\([0-9]*\).*/\1/p'; }
echo 1 >"$d/proyek.versi"
jaga; p1=$(pid)
s=$(curl -s -X POST "$u/api/pty/buka" -d '{"kolom":80,"baris":24}' | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
curl -s -X POST "$u/api/pty/tulis" -d "{\"id\":$s,\"teks\":\"echo tanda-\$((6*7))\\r\"}" >/dev/null
kill "$g"; wait "$g" 2>/dev/null; sleep 0.5
o=$(curl -s -X POST "$u/api/pty/baca" -d "{\"sesi\":[[$s,0]],\"ms\":300}")
cek "$([ -n "$p1" ] && [ "$p1" != "$g" ] && kill -0 "$p1" 2>/dev/null && case "$o" in *tanda-42*) echo 1;; esac)" "penjaga dibunuh: pelayan dan sesi terminalnya tetap hidup"
jaga
cek "$([ "$(pid)" = "$p1" ] && curl -s "$u/api/pty" | grep -q "\"id\":$s" && echo 1)" "penjaga baru memakai pelayan yang sama (sesi pulih)"
kill "$g"; wait "$g" 2>/dev/null
echo 2 >"$d/proyek.versi"
jaga; p2=$(pid)
i=0; while [ $i -lt 60 ] && { [ -z "$p2" ] || [ "$p2" = "$p1" ]; }; do i=$((i + 1)); sleep 0.25; p2=$(pid); done
cek "$([ -n "$p2" ] && [ "$p2" != "$p1" ] && ! kill -0 "$p1" 2>/dev/null && echo 1)" "versi baru: pelayan lama diganti"
k1=$(curl -s -o /dev/null -w '%{http_code}' -X POST -H 'Origin: http://situs-lain.example' "$u/api/pty/buka" -d '{}')
k2=$(curl -s -o /dev/null -w '%{http_code}' -X POST -H "Origin: $u" "$u/api/folder" -d '{"nama":"asal-sah"}')
cek "$([ "$k1" = 403 ] && [ "$k2" = 200 ] && echo 1)" "permintaan dari situs lain ditolak ($k1), dari halaman sendiri diterima ($k2)"
# Keluar lewat notifikasi milik jrgm cuma menghentikan penjaga: selagi latar aktif, pelayan menyusul.
curl -s -X POST "$u/api/latar" -d '{"aktif":true}' >/dev/null
kill "$g"; wait "$g" 2>/dev/null
i=0; while [ $i -lt 40 ] && kill -0 "$p2" 2>/dev/null; do i=$((i + 1)); sleep 0.25; done
cek "$(! kill -0 "$p2" 2>/dev/null && echo 1)" "latar aktif: penjaga berhenti, pelayan ikut berhenti"
jaga; p2=$(pid)
curl -s -X POST "$u/api/keluar" -d '{}' >/dev/null
i=0; while [ $i -lt 40 ] && kill -0 "$g" 2>/dev/null; do i=$((i + 1)); sleep 0.25; done
cek "$([ -n "$p2" ] && ! kill -0 "$p2" 2>/dev/null && ! kill -0 "$g" 2>/dev/null && [ -z "$(pid)" ] && echo 1)" "keluar: pelayan berhenti, penjaganya ikut berhenti tanpa menghidupkannya lagi"
g=""
[ $gagal = 0 ] || tail -n 5 "$d/penjaga.txt" "$d/jrg-ide-pelayan.log"
exit $gagal
