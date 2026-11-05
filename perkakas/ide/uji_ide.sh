#!/bin/sh
# JRG IDE (`ide/`) di chromium headless: pelayannya jalan di jrgc, halamannya menjalankan
# `halo.jrg` dan `masukan.jrg` (stdin awal dan di tengah jalan), Hentikan bekerja, font-nya utuh,
# proyek multi-berkas jalan, paket bahasa terpasang, konsol hidup, dan tidak ada galat JavaScript.
set -u
cd "$(dirname "$0")/../.."
J=$PWD/target/release/jrgc
krom=$(command -v chromium || command -v chromium-browser || command -v google-chrome || true)
if [ -z "$krom" ] || ! command -v node >/dev/null || ! command -v curl >/dev/null; then
  echo "dilewati: butuh chromium, node dan curl"; exit 0
fi
d=$(mktemp -d); trap 'kill $p 2>/dev/null; rm -rf "$d"' EXIT
port=$((47000 + $$ % 900))
# Cermin paket (Pyodide, TypeScript) kalau pernah diunduh; Python disembunyikan supaya jalur paketnya yang diuji.
cermin=$HOME/.cache/jrgm/ide-paket
[ -f "$cermin/python/pyodide.asm.wasm" ] && [ -f "$cermin/ts/typescript.min.js" ] || cermin=
( cd ide && exec env HOME="$d" PORT=$port JRG_JRGC="${JRG_UJI_JRGC:-$J}" JRG_SOPAN=1 JRG_IDE_CERMIN="$cermin" JRG_IDE_SEMBUNYI="${cermin:+python}" \
    "${JRG_UJI_JRGC:-$J}" src/main.jrg ) >"$d/pelayan.txt" 2>&1 &
p=$!
i=0; while [ $i -lt 80 ] && ! curl -s -o /dev/null "http://127.0.0.1:$port/"; do i=$((i + 1)); sleep 0.25; done
gagal=0
f=web/aset/huruf/jetbrains-mono-400-600-latin.woff2
curl -s "http://127.0.0.1:$port/$f" -o "$d/huruf"
if cmp -s "ide/$f" "$d/huruf"; then echo "ok: berkas biner (font) dilayani utuh"
else echo "GAGAL: font yang dilayani berbeda dari berkasnya"; gagal=1; fi
h=$(node perkakas/ide/uji_peramban.js "$krom" "http://127.0.0.1:$port/" 2>&1)
case "$h" in
  *'"keluaran1":"Halo dari JRG IDE!\n0 -> 1'*'"keluaran2":"Siapa namamu?\nHalo, Budi! Namamu 4 huruf.\n"'*'"galat":[]'*)
    echo "ok: IDE menjalankan JRG, stdin sampai, penyorot hidup, nol galat JavaScript";;
  *) echo "GAGAL: JRG IDE di peramban: $h"; head -5 "$d/pelayan.txt"; gagal=1;;
esac
case "$h" in
  *'"tombol":"Hentikan"'*'"keluaran3":"Siapa namamu?\nSari\nHalo, Sari! Namamu 4 huruf.\n"'*'"status4":"Dihentikan"'*'"cari":"masukan.jrg:5'*)
    echo "ok: keluaran mengalir, masukan di tengah jalan sampai, Hentikan menghentikan, cari di semua berkas";;
  *) echo "GAGAL: JRG IDE di peramban: $h"; head -5 "$d/pelayan.txt"; gagal=1;;
esac
case "$h" in
  *'"lua":"tanpa"'*) echo "dilewati: Lua (tidak ada penjalan Lua di mesin ini)";;
  *'"lua":"5! = 120"'*) echo "ok: berkas Lua dijalankan penjalannya";;
  *) echo "GAGAL: berkas Lua tidak menjawab 5! = 120"; gagal=1;;
esac
case "$h" in
  *'"ts":"tanpa"'*) echo "dilewati: bahasa yang dipasang (tidak ada cermin paket di ~/.cache/jrgm/ide-paket)";;
  *'"py":"Halo, Budi!"'*'"ts":"2 barang, total 52500"'*'"six":"1.17.0"'*'"proyekPy":"Proyek cobapy|Halo, dunia!\n5! = 120"'*'"proyekTs":"Proyek cobats|dua kali 21 = 42"'*)
    echo "ok: TypeScript dan Python dipasang dari halaman lalu jalan; pustaka Python dipasang saat diimpor; proyek Python dan TypeScript multi-berkas";;
  *) echo "GAGAL: bahasa yang dipasang: $h"; gagal=1;;
esac
case "$h" in
  *'"lain":"tanpa"'*) echo "dilewati: Ruby, CoffeeScript, Scheme, Prolog (tidak ada di cermin paket)";;
  *'"lain":"Halo, Budi! / 7 kuadrat = 49 / 5! = 120 / budi kakek dari tono"'*) echo "ok: Ruby, CoffeeScript, Scheme dan Prolog dipasang dari halaman lalu jalan";;
  *) echo "GAGAL: paket bahasa lain: $h"; gagal=1;;
esac
case "$h" in
  *'"proyek":"Proyek cobajrg|3 + 4 = 7\n5! = 120"'*'"web":"IFRAME /berkas/cobaweb/index.html true"'*'"simbol":"gulir 0, ketuk 1"'*'"cariBahasa":"Go:belum bisa"'*)
    echo "ok: proyek JRG dijalankan dari berkas mana pun, pratinjau web memuat tetangganya, pita simbol tidak mengetik saat digulir, cari bahasa menjawab";;
  *) echo "GAGAL: proyek/pratinjau/pita simbol/cari bahasa: $h"; gagal=1;;
esac
case "$h" in
  *'"konsol":"~/cobajrg $|1"'*'"tema":"mint #9EF0C1 '*)
    echo "ok: konsol menjalankan shell dan jrgc, mengingat direktori, selamat dari salah sintaks, Hentikan mematikan perintahnya; tema mint";;
  *) echo "GAGAL: konsol/tema: $h"; gagal=1;;
esac
exit $gagal
