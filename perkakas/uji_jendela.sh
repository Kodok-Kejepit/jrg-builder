#!/bin/sh
# Jendela sungguhan (mode `"tab"`) diuji di Xvfb: pikselnya dibaca
# `import` — alat yang bukan milik proyek ini — lalu dihitung, di dua mesin.
# Dulu fitur ini tidak pernah terkompilasi (258 galat, DIAGNOSTICS.md #419).
set -u
cd "$(dirname "$0")/.."
if ! command -v xvfb-run >/dev/null || ! command -v import >/dev/null || ! command -v python3 >/dev/null; then
  echo "dilewati: butuh xvfb-run, import (ImageMagick) dan python3"; exit 0
fi
# Jendela fitur BAWAAN sejak mode Canvas: yang diuji biner rilis yang sama dengan yang dipasang.
d=$(mktemp -d); trap 'rm -rf "$d"' EXIT
J=$PWD/target/release/jrgc
cat > "$d/p.jrg" <<'JRG'
let c = Canvas::new(64, 48, "piksel", "tab")
canvas::clear(c, 0x2040FF)
canvas::fill_rect(c, 10, 10, 20, 20, 0xFF0000)
var n = 0
while n < 400 && canvas::update(c) {
    time::sleep_ms(10)
    n = n + 1
}
JRG
cat > "$d/potret.sh" <<EOF2
$J $d/p.jrg \$1 & p=\$!
sleep 2
import -window root $d/potret\$1.png
kill \$p; wait \$p 2>/dev/null
EOF2
gagal=0
for x in "" --vm; do
  env -u WAYLAND_DISPLAY timeout 60 xvfb-run -a -s "-screen 0 320x240x24" sh "$d/potret.sh" $x
  hasil=$(python3 -c "
from PIL import Image
im = Image.open('$d/potret$x.png').convert('RGB')
biru = merah = 0
for p in im.getdata():
    biru += p == (0x20, 0x40, 0xFF); merah += p == (0xFF, 0, 0)
print(biru, merah)" 2>/dev/null)
  if [ "$hasil" = "2672 400" ]; then echo "ok ${x:-interp}: 2672 biru + 400 merah di layar"
  else echo "GAGAL ${x:-interp}: piksel di layar '$hasil', harusnya '2672 400'"; gagal=1; fi
done
# Putaran `std/mesin` di jendela sungguhan: berhenti sendiri lewat `dengan_selesai`,
# tepat di langkah ke-30 — bukan karena jendelanya dibunuh dari luar.
cat > "$d/m.jrg" <<'JRG'
import "std/mesin"
struct N { pub n: int }
fn maju(k, m, dt) { return N { n: k.n + 1 } }
fn lukis(c, k) { canvas::fill_rect(c, 0, 0, 8, 8, 0xFF0000) }
let akhir = mesin::baru("mesin", 32, 32, N { n: 0 }, maju, lukis)
    .dengan_selesai(|k| k.n >= 30).dengan_bunyi(|lama, baru| []).jalankan()
wok("langkah", akhir.langkah_ke, akhir.keadaan.n)
JRG
for x in "" --vm; do
  k=$(env -u WAYLAND_DISPLAY JRG_SOPAN=1 timeout 60 xvfb-run -a -s "-screen 0 320x240x24" $J "$d/m.jrg" $x 2>&1)
  if [ "$k" = "langkah 30 30" ]; then echo "ok ${x:-interp}: mesin::jalankan berhenti sendiri di langkah 30"
  else echo "GAGAL ${x:-interp}: mesin::jalankan menjawab '$k'"; gagal=1; fi
done
# Backend LLVM: dikompilasi dulu (di luar Xvfb), lalu biner aslinya dipotret dan dihitung sama.
if command -v clang >/dev/null; then
  $J "$d/p.jrg" --emit-llvm -o "$d/p_llvm" --no-run >/dev/null 2>&1
  cat > "$d/potret_llvm.sh" <<EOF2
(ulimit -v 4000000; exec $d/p_llvm) & p=\$!
sleep 2
import -window root $d/potret_llvm.png
kill \$p; wait \$p 2>/dev/null
EOF2
  env -u WAYLAND_DISPLAY timeout 60 xvfb-run -a -s "-screen 0 320x240x24" sh "$d/potret_llvm.sh"
  hasil=$(python3 -c "
from PIL import Image
im = Image.open('$d/potret_llvm.png').convert('RGB')
biru = merah = 0
for p in im.getdata():
    biru += p == (0x20, 0x40, 0xFF); merah += p == (0xFF, 0, 0)
print(biru, merah)" 2>/dev/null)
  if [ "$hasil" = "2672 400" ]; then echo "ok llvm: 2672 biru + 400 merah di layar"
  else echo "GAGAL llvm: piksel di layar '$hasil', harusnya '2672 400'"; gagal=1; fi
  k=$(env -u WAYLAND_DISPLAY JRG_SOPAN=1 timeout 300 xvfb-run -a -s "-screen 0 320x240x24" sh -c "ulimit -v 4000000; $J $d/m.jrg --emit-llvm" 2>&1)
  if [ "$k" = "langkah 30 30" ]; then echo "ok llvm: mesin::jalankan berhenti sendiri di langkah 30"
  else echo "GAGAL llvm: mesin::jalankan menjawab '$k'"; gagal=1; fi
else
  echo "dilewati llvm: butuh clang"
fi
# Tanpa layar: DITOLAK sambil menyebut sebabnya, bukan kanvas yang menutup diam-diam.
k=$(env -u WAYLAND_DISPLAY -u DISPLAY JRG_SOPAN=1 $J "$d/p.jrg" 2>&1)
case "$k" in *"tidak ada layar"*) echo "ok: tanpa layar dijelaskan";;
  *) echo "GAGAL: tanpa layar dijawab '$k'"; gagal=1;; esac
exit $gagal
