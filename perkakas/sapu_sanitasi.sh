#!/bin/sh
# Penyapu sanitizer runtime C backend LLVM: `sh perkakas/sapu_sanitasi.sh ubsan|asan [--suite]`.
# Tiap program `--emit-llvm` dikompilasi lewat pembungkus `JRGC_CLANG` ber-`-fsanitize`; korpusnya diadu dengan
# interpreter, dan laporan sanitizer apa pun (`runtime error:`, `AddressSanitizer`) adalah kegagalan.
set -u
mode=${1:-}
case $mode in
  ubsan) bendera="-fsanitize=undefined -fno-sanitize-recover=all"; tanda=__ubsan_handle ;;
  asan) bendera="-fsanitize=address"; tanda=__asan_report ;;
  *) echo "pakai: sh perkakas/sapu_sanitasi.sh ubsan|asan [--suite]" >&2; exit 2 ;;
esac
suite=${2:-}
akar=$(cd "$(dirname "$0")/.." && pwd)
jrgc=$akar/target/release/jrgc
clang=${CLANG_ASLI:-clang}
command -v "$clang" >/dev/null 2>&1 || { echo "butuh clang" >&2; exit 2; }
kerja=$(mktemp -d)
trap 'rm -rf "$kerja"' EXIT
printf '#!/bin/sh\nexec %s "$@" -g %s\n' "$clang" "$bendera" > "$kerja/clang"
chmod +x "$kerja/clang"

# Laporan ke berkas (`log_path`), jadi keluaran tetap bisa diadu persis. ASan: SIGSEGV milik penjaga stack, bocor
# tidak dihitung (arena tak dibebaskan), tanpa batas ruang alamat (shadow bertera-byte), tanpa stack palsu (bingkai
# di heap membuat penjaga stack tidak mengenali stack yang habis).
mkdir -p "$kerja/log"
export JRGC_CLANG="$kerja/clang" JRG_SOPAN=1 JRGC_BATAS_MEMORI=
export ASAN_OPTIONS=log_path=$kerja/log/asan:handle_segv=0:detect_leaks=0:detect_stack_use_after_return=0
export UBSAN_OPTIONS=log_path=$kerja/log/ubsan:print_stacktrace=1:halt_on_error=1
gagal=0

# Laporan yang terkumpul sejak pemanggilan terakhir; berkasnya lalu dihapus. Peringatan `swapcontext` bukan temuan.
laporan() {
  ada=1
  for lg in "$kerja"/log/*; do
    [ -f "$lg" ] || continue
    if grep -E 'runtime error:|ERROR: AddressSanitizer|SUMMARY:' "$lg" >/dev/null; then
      ada=0; sed -n 1,40p "$lg"
    fi
    rm -f "$lg"
  done
  return $ada
}

# Alatnya diperiksa dulu: biner yang tidak membawa instrumen sanitizer membuat sapuan ini lulus tanpa arti.
printf 'package main\npub fn main() { wok("ok") }\n' > "$kerja/a.jrg"
"$jrgc" "$kerja/a.jrg" --emit-llvm -o "$kerja/a.bin" --no-run >/dev/null 2>&1 || { echo "GAGAL: kompilasi lewat pembungkus"; exit 1; }
nm "$kerja/a.bin" 2>/dev/null | grep -q "$tanda" || { echo "GAGAL: biner tidak membawa $tanda"; exit 1; }
# ... dan harus bisa melihat cacat: program C yang memang cacat wajib meninggalkan laporan.
printf '#include <stdlib.h>\nint main(int c, char **v) { int *p = malloc(4); int x = 2147483647; x += c; return p[c] + x; }\n' > "$kerja/c.c"
"$kerja/clang" "$kerja/c.c" -o "$kerja/c.bin" && { "$kerja/c.bin" >/dev/null 2>&1; true; }
laporan >/dev/null || { echo "GAGAL: cacat contoh tidak terlaporkan"; exit 1; }
echo "== $mode: instrumen terpasang dan melapor"

periksa() { # periksa <berkas> [opsi..]: keluaran dan kode LLVM sama dengan interpreter, tanpa laporan sanitizer
  f=$1; shift
  a=$(timeout 300 "$jrgc" "$f" "$@" 2>&1); ra=$?
  l=$(timeout 900 "$jrgc" "$f" "$@" --emit-llvm -o "$kerja/p.bin" 2>&1); rl=$?
  if laporan; then
    echo "  LAPORAN di atas: $f"; gagal=$((gagal+1))
  elif [ "$a" != "$l" ] || [ $ra -ne $rl ]; then
    echo "  BEDA $f (rc $ra/$rl)"; [ -n "${RINCI:-}" ] && printf "%s\n---\n%s\n" "$a" "$l" | head -20; gagal=$((gagal+1))
  fi
}

echo "== $mode: tests/conformance, examples_llvm, stdlib/uji"
for f in "$akar"/tests/conformance/*.jrg; do periksa "$f"; done
for f in "$akar"/examples_llvm/*.jrg; do
  # Kernel yang cuma mengukur waktu (fib) atau mencetak angka waktu tidak diadu keluarannya.
  grep -q 'time::' "$f" && continue
  periksa "$f"
done
for f in "$akar"/stdlib/uji/*.jrg; do periksa "$f" --uji; done
echo "  selesai, $gagal gagal"

if [ "$suite" = --suite ]; then
  # Biner tes LLVM yang sama dengan suite, tiap program lewat pembungkus. Di bawah ASan dua tes dilewati karena
  # memakai batas ruang alamat (`ulimit -v`, `JRGC_BATAS_MEMORI`), yang bentrok dengan shadow ASan.
  lewati=""
  [ "$mode" = asan ] && lewati="--skip biaya_di_llvm_tetap_linear --skip batas_memori_biner_llvm_ditegakkan"
  for t in llvm_parity llvm_diagnostik; do
    echo "== $mode: cargo test --test $t"
    (cd "$akar" && env -u JRG_SOPAN cargo test --release -j 1 --no-fail-fast --test $t -- $lewati) > "$kerja/t.log" 2>&1
    r=$?
    grep '^test result' "$kerja/t.log"
    if laporan; then echo "  LAPORAN di atas: --test $t"; gagal=$((gagal+1)); fi
    if [ $r -ne 0 ]; then
      grep -E '^test .* FAILED' "$kerja/t.log" | head -40
      sed -n '/^---- /,/^failures:$/p' "$kerja/t.log" | head -150
      gagal=$((gagal+1))
    fi
  done
fi

echo "GAGAL=$gagal"
[ $gagal -eq 0 ]
