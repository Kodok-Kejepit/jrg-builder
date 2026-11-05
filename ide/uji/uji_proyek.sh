#!/bin/sh
# Uji proyek jrgm (ide/README.md, "Proyek"): rangka jrgm new, Jalankan proyek C tanpa tests/, dan peringatan
# jrgm lint di pemeriksa (modul sendirian tidak dituduh fungsinya tak dipanggil).
set -u
cd "$(dirname "$0")/../.."
J=${JRG_UJI_JRGC:-$PWD/target/release/jrgc}
command -v curl >/dev/null || { echo "dilewati: proyek (butuh curl)"; exit 0; }
d=$(mktemp -d); p=""
port=$((48000 + $$ % 900)); u=http://127.0.0.1:$port
trap 'kill $p 2>/dev/null; rm -rf "$d"' EXIT
gagal=0
cek() { if [ "$1" = 1 ]; then echo "ok: $2"; else echo "GAGAL: $2"; gagal=1; fi; }
( cd ide && exec env HOME="$d" PORT=$port JRG_JRGC="$J" JRG_SOPAN=1 "$J" src/main.jrg ) >"$d/log" 2>&1 &
p=$!
i=0; while [ $i -lt 120 ] && ! curl -s -o /dev/null "$u/api/info"; do i=$((i + 1)); sleep 0.25; done
post() { curl -s -X POST "$u$1" -d "$2"; }
a=$(post /api/proyek/baru '{"nama":"kalku","bahasa":"c"}')
b=$(post /api/proyek/baru '{"nama":"kalku","bahasa":"c"}')
c=$(post /api/proyek/baru '{"nama":"x","bahasa":"cobol"}')
cek "$(case "$a$b$c" in *'"utama":"src/main.c"'*'sudah ada'*'tidak dikenal'*) echo 1;; esac)" "proyek baru dari jrgm new; nama kembar dan bahasa asing ditolak"
if command -v cc >/dev/null; then
  id=$(post /api/mulai '{"nama":"kalku/src/main.c","akar":"kalku","masukan":"","mesin":"interp","mode":"","argumen":[]}' | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
  i=0; h=""; while [ $i -lt 60 ]; do h=$(post /api/baca "{\"id\":$id}"); case "$h" in *'"selesai":true'*) break;; esac; i=$((i + 1)); sleep 0.25; done
  cek "$(case "$h" in *'1 + 2 = 3'*'"sukses":true'*) echo 1;; esac)" "proyek C dijalankan tanpa uji di tests/ ($id)"
fi
post /api/simpan '{"nama":"lt.jrg","isi":"fn tak() {\n    1\n}\n\npub fn main() {\n    let y = 5\n    wok(1)\n}\n"}' >/dev/null
post /api/simpan '{"nama":"m/bantu.jrg","isi":"package bantu\n\npub fn f() {\n    1\n}\n"}' >/dev/null
l=$(post /api/periksa '{"nama":"lt.jrg"}'); m=$(post /api/periksa '{"nama":"m/bantu.jrg"}')
cek "$(case "$l" in *'"baris":1,"jenis":"peringatan"'*'"baris":6,"jenis":"peringatan"'*) case "$m" in *'"masalah":[]'*) echo 1;; esac;; esac)" "peringatan jrgm lint di pemeriksa; modul sendirian bersih"
[ $gagal = 0 ] || { echo "$l"; echo "$m"; tail -n 5 "$d/log"; }
exit $gagal
