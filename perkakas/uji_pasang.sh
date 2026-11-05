#!/bin/sh
# Menguji `rilis/pasang.sh` terhadap rilis PALSU di berkas lokal.
set -eu
akar=$(cd "$(dirname "$0")/.." && pwd)
kerja=$(mktemp -d)
trap 'rm -rf "$kerja"' EXIT INT TERM

gagal=0
lapor() { if [ "$1" = ok ]; then printf '  ok   %s\n' "$2"; else printf '  GAGAL %s\n' "$2"; gagal=$((gagal+1)); fi; }

[ -x "$akar/target/release/jrgc" ] || { echo "butuh target/release/jrgc — jalankan cargo build --release"; exit 1; }

# Rilis palsu
r="$kerja/rel"; mkdir -p "$r/jrg-9.9.9-linux-x86_64" "$r/latest/download"
cp "$akar/target/release/jrgc" "$akar/target/release/jrgm" "$r/jrg-9.9.9-linux-x86_64/"
printf 'runtime tiruan' > "$r/jrg-9.9.9-linux-x86_64/libjrgrt.a"
( cd "$r" && tar czf jrg-9.9.9-linux-x86_64.tar.gz jrg-9.9.9-linux-x86_64 )
sha=$(sha256sum "$r/jrg-9.9.9-linux-x86_64.tar.gz" | cut -d' ' -f1)
uk=$(wc -c < "$r/jrg-9.9.9-linux-x86_64.tar.gz")
cp "$r/jrg-9.9.9-linux-x86_64.tar.gz" "$r/latest/download/"
plat=$(uname -s | tr 'A-Z' 'a-z')-$(uname -m)
cat > "$r/latest/download/manifes.json" <<JSON
{ "skema": 1, "versi": "9.9.9", "berkas": [
  { "platform": "$plat", "nama": "jrg-9.9.9-linux-x86_64.tar.gz",
    "sha256": "$sha", "ukuran": $uk, "isi": ["jrgc","jrgm"] } ] }
JSON

# Stub python3 yang selalu gagal, untuk memaksa jalur cadangan.
mkdir -p "$kerja/tanpa_py"
printf '#!/bin/sh\nexit 127\n' > "$kerja/tanpa_py/python3"
chmod +x "$kerja/tanpa_py/python3"

jalan() {  # $1 = label, $2 = PATH tambahan, sisanya argumen pasang.sh
  label=$1; tambah=$2; shift 2
  env PATH="$tambah$PATH" JRG_SUMBER="file://$r" sh "$akar/rilis/pasang.sh" "$@" 2>&1
}

for mode in "dengan-python3:" "tanpa-python3:$kerja/tanpa_py:"; do
  nama=${mode%%:*}; tambah=${mode#*:}
  echo "$nama"

  # 1. Jalur bahagia
  rm -rf "$kerja/bin" "$kerja/lib"
  jalan "$nama" "$tambah" --ke "$kerja/bin" >/dev/null 2>&1 || true
  if [ -x "$kerja/bin/jrgc" ] && [ -x "$kerja/bin/jrgm" ] && [ -f "$kerja/lib/jrg/libjrgrt.a" ]; then
    lapor ok "memasang jrgc, jrgm, dan pustaka --emit-llvm ke ../lib/jrg"
  else
    lapor x "memasang jrgc, jrgm, dan pustaka --emit-llvm ke ../lib/jrg"
  fi

  # 2. Checksum meleset -> TIDAK memasang apa pun
  cp "$r/latest/download/jrg-9.9.9-linux-x86_64.tar.gz" "$kerja/asli.tgz"
  printf 'rusak' >> "$r/latest/download/jrg-9.9.9-linux-x86_64.tar.gz"
  rm -rf "$kerja/bin2"
  keluaran=$(jalan "$nama" "$tambah" --ke "$kerja/bin2" || true)
  if echo "$keluaran" | grep -q "CHECKSUM TIDAK COCOK" && [ ! -d "$kerja/bin2" ]; then
    lapor ok "menolak arsip rusak, nol berkas mendarat"
  else
    lapor x "menolak arsip rusak, nol berkas mendarat"
  fi
  cp "$kerja/asli.tgz" "$r/latest/download/jrg-9.9.9-linux-x86_64.tar.gz"

  # 3. Platform tidak ada di manifes -> menyebut yang ada
  cp "$r/latest/download/manifes.json" "$kerja/m.json"
  sed "s/\"$plat\"/\"platform-lain\"/" "$kerja/m.json" > "$r/latest/download/manifes.json"
  keluaran=$(jalan "$nama" "$tambah" --ke "$kerja/bin3" || true)
  if echo "$keluaran" | grep -q "platform-lain"; then
    lapor ok "menolak platform tak ada, menyebut yang ada"
  else
    lapor x "menolak platform tak ada, menyebut yang ada"
  fi
  cp "$kerja/m.json" "$r/latest/download/manifes.json"

  # 4. Manifes tidak terambil -> berbicara, bukan diam
  keluaran=$(env PATH="$tambah$PATH" JRG_SUMBER="file:///tidak/ada" \
    sh "$akar/rilis/pasang.sh" --ke "$kerja/bin4" 2>&1 || true)
  if echo "$keluaran" | grep -q "manifes tidak bisa diambil"; then
    lapor ok "menolak manifes yang tidak terambil"
  else
    lapor x "menolak manifes yang tidak terambil"
  fi
done

# 5-6. Termux (tiruan lewat `$PREFIX`): biner Android, mendarat di `$PREFIX/bin`, `--ke` tetap menang.
echo "termux"
pre="$kerja/com.termux/files/usr"; mkdir -p "$pre/bin"
cp "$r/latest/download/manifes.json" "$kerja/m.json"
sed "s/\"$plat\"/\"android-$(uname -m)\"/" "$kerja/m.json" > "$r/latest/download/manifes.json"
env PREFIX="$pre" JRG_SUMBER="file://$r" sh "$akar/rilis/pasang.sh" >/dev/null 2>&1 || true
if [ -x "$pre/bin/jrgc" ] && [ -x "$pre/bin/jrgm" ]; then lapor ok "Termux memasang biner android ke \$PREFIX/bin"
else lapor x "Termux memasang biner android ke \$PREFIX/bin"; fi
rm -rf "$kerja/bin5"
env PREFIX="$pre" JRG_SUMBER="file://$r" sh "$akar/rilis/pasang.sh" --ke "$kerja/bin5" >/dev/null 2>&1 || true
if [ -x "$kerja/bin5/jrgc" ]; then lapor ok "Termux tetap mengikuti --ke"; else lapor x "Termux tetap mengikuti --ke"; fi
cp "$kerja/m.json" "$r/latest/download/manifes.json"

echo
if [ "$gagal" -eq 0 ]; then echo "pemasang: 10 pemeriksaan, semuanya lulus"; else
  echo "pemasang: $gagal pemeriksaan GAGAL"; exit 1; fi
