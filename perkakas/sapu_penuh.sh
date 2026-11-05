#!/bin/sh
# Sapuan lengkap SESUDAH suite: konformans, contoh, modul std, dan pemasang.
set -u
akar=$(cd "$(dirname "$0")/.." && pwd)
cd "$akar"
jrgc=./target/release/jrgc
[ -x "$jrgc" ] || { echo "butuh target/release/jrgc"; exit 1; }

gagal=0
bab() { printf '\n== %s ==\n' "$1"; }

# Jalankan perintah, cetak keluarannya menjorok, dan hitung kegagalannya. Status
# `cmd | sed` adalah status `sed` — dulu dua tahap di bawah tidak pernah bisa gagal.
tahap() {
  keluaran=$("$@" 2>&1); rc=$?
  [ -n "$keluaran" ] && printf '%s\n' "$keluaran" | sed 's/^/  /'
  [ $rc -eq 0 ] || gagal=$((gagal+1))
}

# Dibunuh batas waktu (124) DIULANG sebelum dicatat: di bawah beban penuh —
# `cargo test` yang sedang mengompilasi, misalnya — sebuah program yang …
coba() {  # $1=batas detik, sisanya perintah
  batas=$1; shift
  timeout "$batas" "$@" >/dev/null 2>&1 && return 0
  rc=$?
  [ $rc -ne 124 ] && return $rc
  sleep 2
  timeout $((batas * 2)) "$@" >/dev/null 2>&1 && return 0
  rc=$?
  [ $rc -eq 124 ] && echo "  (dibunuh batas waktu dua kali — mesin sedang sibuk?)" >&2
  return $rc
}

bab "konformans (11 berkas x 2 backend)"
for x in "" "--vm"; do
  n=0
  for f in tests/conformance/*.jrg; do
    coba 30 $jrgc "$f" $x || { echo "  GAGAL ${x:-interp} $f (rc=$?)"; n=$((n+1)); }
  done
  printf '  %-7s %s\n' "${x:-interp}" "$([ $n -eq 0 ] && echo 'semua lulus' || echo "$n gagal")"
  gagal=$((gagal+n))
done

bab "contoh"
for x in "" "--vm"; do
  n=0
  for f in examples/*.jrg; do
    timeout 8 env -u DISPLAY -u WAYLAND_DISPLAY $jrgc "$f" $x >/dev/null 2>&1
    rc=$?
    [ $rc -eq 0 ] && continue
    # 124 = diblokir batas waktu, memang begitu rancangannya (server).
    [ $rc -eq 124 ] && continue
    case "$f" in
      # Berkas pustaka tanpa `main` — galatnya BENAR.
      */helpers.jrg) continue ;;
    esac
    echo "  GAGAL ${x:-interp} $f (rc=$rc)"; n=$((n+1))
  done
  printf '  %-7s %s\n' "${x:-interp}" "$([ $n -eq 0 ] && echo 'nol kegagalan nyata' || echo "$n gagal")"
  gagal=$((gagal+n))
done

bab "modul std/ bisa di-import"
n=0
for m in stdlib/*.jrg stdlib/mesin/*.jrg; do
  nama=${m#stdlib/}; nama=${nama%.jrg}; [ "$nama" = mesin/mesin ] && nama=mesin
  printf 'package main\nimport "std/%s"\npub fn main() { wok("ok") }\n' "$nama" > /tmp/_sapu_std.jrg
  for x in "" "--vm"; do
    coba 8 $jrgc /tmp/_sapu_std.jrg $x || { echo "  GAGAL std/$nama ${x:-interp}"; n=$((n+1)); }
  done
done
rm -f /tmp/_sapu_std.jrg
printf '  %s\n' "$([ $n -eq 0 ] && echo 'semua modul bisa di-import di dua backend' || echo "$n gagal")"
gagal=$((gagal+n))

bab "tes std/ lewat --uji: interpreter sama dengan --emit-llvm"
if command -v "${JRGC_CLANG:-clang}" >/dev/null 2>&1; then
  n=0
  for f in stdlib/uji/*.jrg; do
    a=$(JRG_SOPAN=1 timeout 300 $jrgc "$f" --uji 2>&1); ra=$?
    l=$(JRG_SOPAN=1 JRGC_BATAS_MEMORI=3000 timeout 600 $jrgc "$f" --uji --emit-llvm -o /tmp/_sapu_uji.bin 2>&1); rl=$?
    [ "$a" = "$l" ] && [ $ra -eq $rl ] || { echo "  BEDA $f (rc $ra/$rl)"; n=$((n+1)); }
  done
  rm -f /tmp/_sapu_uji.bin
  printf '  %s\n' "$([ $n -eq 0 ] && echo 'semua sama' || echo "$n beda")"
  gagal=$((gagal+n))
else
  echo "  (dilewati: butuh clang)"
fi

bab "contoh di situs dokumentasi"
if command -v python3 >/dev/null 2>&1; then
  tahap python3 perkakas/cek_contoh_dokumentasi.py
else
  echo "  (dilewati: butuh python3)"
fi

# Situs yang terbit harus hasil generatornya: suntingan tangan di `docs/` hilang
# di bangunan berikutnya. Penjaganya dulu ada tapi tidak dijalankan siapa pun.
bab "situs = hasil generatornya"
if command -v python3 >/dev/null 2>&1; then
  keluaran=$(python3 perkakas/cek_situs.py 2>&1); rc=$?
  printf '%s\n' "$keluaran" | tail -1 | sed 's/^/  /'
  [ $rc -eq 0 ] || { printf '%s\n' "$keluaran" | grep '^  -' ; gagal=$((gagal+1)); }
else
  echo "  (dilewati: butuh python3)"
fi

bab "pemasang rilis"
keluaran=$(sh perkakas/uji_pasang.sh 2>&1); rc=$?
printf '%s\n' "$keluaran" | tail -1 | sed 's/^/  /'
[ $rc -eq 0 ] || gagal=$((gagal+1))

bab "mesin Praktik JRG (situs) sama dengan jrgc"
if command -v python3 >/dev/null 2>&1; then
  tahap python3 perkakas/praktik.py periksa
else
  echo "  (dilewati: butuh python3)"
fi

bab "jendela sungguhan (mode \"tab\") di Xvfb"
tahap sh perkakas/uji_jendela.sh

bab "APK Android (jrgc --apk)"
tahap sh perkakas/uji_apk.sh

bab "JRG IDE (ide/) di peramban"
tahap sh perkakas/ide/uji_ide.sh

bab "angka yang bergerak"
printf '  builtin       : %s\n' "$($jrgc --daftar-builtin | wc -l)"
printf '  fungsi std/   : %s\n' "$(grep -ch '^pub fn ' stdlib/*.jrg stdlib/mesin/*.jrg | awk '{s+=$1} END {print s}')"
printf '  jrgc -h       : %s baris\n' "$($jrgc -h 2>&1 | wc -l)"
printf '  jrgm -h       : %s baris\n' "$(./target/release/jrgm -h 2>&1 | wc -l)"

printf '\n'
[ "$gagal" -eq 0 ] && echo "SAPUAN BERSIH" || echo "SAPUAN: $gagal kegagalan nyata"
exit $([ "$gagal" -eq 0 ] && echo 0 || echo 1)
