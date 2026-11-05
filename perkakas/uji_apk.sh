#!/bin/sh
# `jrgc --apk`: korutin Android (penukar stack x86_64 diuji di mesin ini), susunan APK,
# perataan 16 KB, tanda tangan; kalau ada HP/emulator di adb, juga keluaran dan pikselnya.
# Rancangannya: dokumentasi/src/llvm/apk.md
set -u
cd "$(dirname "$0")/.."
J=$PWD/target/release/jrgc
d=$(mktemp -d); trap 'rm -rf "$d"' EXIT
gagal=0
sdk=""
for a in "${ANDROID_HOME:-}" "${ANDROID_SDK_ROOT:-}" "$HOME/Android/Sdk" /usr/lib/android-sdk; do
  [ -n "$a" ] && [ -d "$a/build-tools" ] && sdk="$a" && break
done
if [ -z "$sdk" ] || ! command -v clang >/dev/null; then echo "dilewati: butuh Android SDK dan clang"; exit 0; fi
bt=$(ls -d "$sdk"/build-tools/*/ 2>/dev/null | sort -V | tail -1)

# 1. Penukar stack Android (`korutin.c` tanpa swapcontext), versi x86_64, di mesin ini.
cat > "$d/g.jrg" <<'JRG'
package main
pub fn main() {
    let c = make_chan(int, 0)
    spawn { var i = 0
        while i < 3 { c <- 10 + i; time::sleep_ms(1); i = i + 1 } }
    spawn { var i = 0
        while i < 3 { c <- 20 + i; i = i + 1 } }
    var s = 0
    var n = 0
    while n < 6 { s = s + <-c; n = n + 1 }
    wok("jumlah", s)
}
JRG
acuan=$(JRG_SOPAN=1 $J "$d/g.jrg")
( ulimit -v 4000000; JRG_SOPAN=1 $J "$d/g.jrg" --emit-llvm -o "$d/g" --no-run >/dev/null 2>&1 )
cat src/llvm/primitif.c src/llvm/korutin.c src/llvm/jaga_stack.c > "$d/c.c"
clang -O2 -w -D__ANDROID__ "$d/g.ll" "$d/c.c" runtime/target/release/libjrgrt.a -lm -pthread -ldl -o "$d/g2" 2>/dev/null
k=$( ulimit -v 4000000; "$d/g2" 2>&1 )
if [ "$k" = "$acuan" ] && ! nm "$d/g2" | grep -q swapcontext; then echo "ok: korutin tanpa swapcontext menjawab '$k'"
else echo "GAGAL: korutin Android menjawab '$k', acuan '$acuan'"; gagal=1; fi

# 2. APK tanpa jembatan, dua ABI.
printf 'fn main() {\n    wok("halo dari android", 1 + 2)\n}\n' > "$d/halo.jrg"
k=$( ulimit -v 6000000; JRG_SOPAN=1 $J "$d/halo.jrg" --apk "$d/halo.apk" --apk-abi arm64-v8a,x86_64 2>&1 )
case "$k" in
  *"tidak ditemukan"*|*"belum terpasang"*) echo "dilewati: $k" | head -2; exit 0;;
esac
isi=$(unzip -Z1 "$d/halo.apk" 2>/dev/null | LC_ALL=C sort | tr '\n' ' ')
harap="AndroidManifest.xml META-INF/ANDROIDD.RSA META-INF/ANDROIDD.SF META-INF/MANIFEST.MF lib/arm64-v8a/libjrg.so lib/x86_64/libjrg.so resources.arsc "
if [ "$isi" = "$harap" ]; then echo "ok: halo.apk berisi manifes + libjrg.so dua ABI"
else echo "GAGAL: isi halo.apk '$isi'"; echo "  jrgc: $k"; gagal=1; fi
if "$bt/zipalign" -c -P 16 4 "$d/halo.apk" >/dev/null 2>&1; then echo "ok: pustaka rata 16 KB (zipalign -P 16)"
else echo "GAGAL: zipalign -c -P 16 menolak halo.apk"; gagal=1; fi
if "$bt/apksigner" verify "$d/halo.apk" >/dev/null 2>&1; then echo "ok: tanda tangan sah (apksigner verify)"
else echo "GAGAL: apksigner verify menolak halo.apk"; gagal=1; fi

# 3. APK berkanvas: jembatan ikut masuk.
cat > "$d/kanvas.jrg" <<'JRG'
let c = Canvas::new(64, 48, "kanvas", "tab")
wok("jendela:", canvas::punya_jendela())
var n = 0
while n < 300 && canvas::update(c) {
    canvas::clear(c, 0x2040FF)
    canvas::fill_rect(c, 10, 10, 20, 20, 0xFF0000)
    time::sleep_ms(10)
    n = n + 1
}
wok("selesai", n)
JRG
( ulimit -v 6000000; JRG_SOPAN=1 $J "$d/kanvas.jrg" --apk "$d/kanvas.apk" --apk-abi x86_64 >/dev/null 2>&1 )
if unzip -Z1 "$d/kanvas.apk" 2>/dev/null | grep -q '^lib/x86_64/libjrgjembatan.so$'; then echo "ok: kanvas.apk membawa jembatan"
else echo "GAGAL: kanvas.apk tanpa libjrgjembatan.so"; gagal=1; fi

# 4. Proyek `jrgm`: identitas dari `[android]` sampai ke manifes APK, ikon ikut.
M=$PWD/target/release/jrgm
( cd "$d" && $M new gameku --android >/dev/null 2>&1 )
python3 -c "
import zlib, struct
def potong(t, d): return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d))
baris = b''.join(b'\\0' + b'\\xff\\x80\\x00' * 48 for _ in range(48))
open('$d/gameku/ikon.png', 'wb').write(b'\\x89PNG\\r\\n\\x1a\\n' + potong(b'IHDR', struct.pack('>IIBBBBB', 48, 48, 8, 2, 0, 0, 0))
    + potong(b'IDAT', zlib.compress(baris)) + potong(b'IEND', b''))"
sed -i 's/^nama      = "gameku"/nama      = "Game Ku"/; s/^versi     = "0.1.0"/versi     = "1.2.3"/; s/^# ikon .*/ikon = "ikon.png"/' "$d/gameku/jrgm.toml"
( cd "$d/gameku" && ulimit -v 6000000 && JRG_SOPAN=1 $M apk --abi x86_64 >/dev/null 2>&1 )
k=$("$bt/aapt2" dump badging "$d/gameku/target/gameku.apk" 2>/dev/null | grep -oE "name='id.jrg.gameku' versionCode='1002003' versionName='1.2.3'|application: label='Game Ku' icon='res/mipmap-xxxhdpi-v4/ikon.png'" | wc -l)
o=$("$bt/aapt2" dump xmltree --file AndroidManifest.xml "$d/gameku/target/gameku.apk" 2>/dev/null | grep -c 'screenOrientation(0x0101001e)=7')
if [ "$k" = 2 ] && [ "$o" = 1 ]; then echo "ok: jrgm apk membawa nama, paket, versi 1.2.3 -> 1002003, ikon, orientasi tegak"
else echo "GAGAL: manifes jrgm apk (cocok $k/2, orientasi $o/1)"; gagal=1; fi
( cd "$d/gameku" && sed -i '/^\[android\]$/a ikom = "x.png"' jrgm.toml && JRG_SOPAN=1 $M apk >"$d/salah.txt" 2>&1 )
if grep -q 'kunci `ikom` di \[android\] tidak dikenal' "$d/salah.txt"; then echo "ok: kunci [android] yang salah ketik ditolak"
else echo "GAGAL: kunci asing di [android] tidak ditolak: $(head -2 "$d/salah.txt")"; gagal=1; fi

# 5. Pustaka C milik proyek masuk lib/<abi>/; arsitektur yang salah ditolak; APK rilis bertanda kunci pemakai.
nb=$(ls -d "${ANDROID_NDK_HOME:-/nonexistent}"/toolchains/llvm/prebuilt/linux-x86_64/bin \
  "$HOME"/Android/Sdk/ndk/*/toolchains/llvm/prebuilt/linux-x86_64/bin \
  "$sdk"/ndk/*/toolchains/llvm/prebuilt/linux-x86_64/bin 2>/dev/null | sort -V | tail -1)
g=$d/gameku
mkdir -p "$g/native/x86_64"
printf '#include <stdint.h>\nint64_t kali_dua(int64_t x) { return x * 2; }\n' > "$d/h.c"
"$nb/clang" --target=x86_64-linux-android26 -shared -fPIC "$d/h.c" -o "$g/native/x86_64/libhitung.so" 2>/dev/null
"$nb/clang" --target=aarch64-linux-android26 -shared -fPIC "$d/h.c" -o "$d/lib_arm.so" 2>/dev/null
sed -i '/^ikom = /d; /^\[android\]$/a pustaka = ["native/{abi}/libhitung.so"]' "$g/jrgm.toml"
( cd "$g" && ulimit -v 6000000 && JRG_SOPAN=1 $M apk --abi x86_64 >/dev/null 2>&1 )
if unzip -Z1 "$g/target/gameku.apk" 2>/dev/null | grep -q '^lib/x86_64/libhitung.so$'; then echo "ok: pustaka proyek masuk lib/x86_64/"
else echo "GAGAL: libhitung.so tidak ikut ke APK"; gagal=1; fi
cp "$d/lib_arm.so" "$g/native/x86_64/libhitung.so"
k=$( cd "$g" && JRG_SOPAN=1 $M apk --abi x86_64 2>&1 )
case "$k" in *"bukan untuk x86_64 (isinya: arm64-v8a)"*) echo "ok: pustaka arsitektur lain ditolak";;
  *) echo "GAGAL: pustaka arm64 di slot x86_64 tidak ditolak: $k"; gagal=1;; esac
rm -f "$g/native/x86_64/libhitung.so"; sed -i '/^pustaka = /d' "$g/jrgm.toml"
keytool -genkeypair -keystore "$g/rilis.jks" -storepass sandiuji -keypass sandiuji -alias rilis \
  -keyalg RSA -keysize 2048 -validity 2 -dname "CN=Uji Rilis JRG" >/dev/null 2>&1
printf 'kunci = "rilis.jks"\nalias = "rilis"\n' >> "$g/jrgm.toml"
( cd "$g" && ulimit -v 6000000 && JRG_KUNCI_SANDI=sandiuji JRG_SOPAN=1 $M apk --rilis --abi x86_64 >/dev/null 2>&1 )
if "$bt/apksigner" verify --print-certs "$g/target/gameku-rilis.apk" 2>/dev/null | grep -q "CN=Uji Rilis JRG"; then
  echo "ok: jrgm apk --rilis bertanda kunci pemakai, bukan debug"
else echo "GAGAL: APK rilis tidak bertanda kunci pemakai"; gagal=1; fi
k=$( cd "$g" && JRG_SOPAN=1 $M kunci 2>&1 </dev/null )
case "$k" in *"sudah ada — tidak ditimpa"*) echo "ok: jrgm kunci menolak menimpa kunci yang ada";;
  *) echo "GAGAL: jrgm kunci: $k"; gagal=1;; esac

# 6. jrgm mengompilasi sumber proyek: C dan C++ dalam SATU pustaka (simbol C tidak di-mangle),
#    crate Rust, izin di manifes; izin berbahaya ditolak; AAB divalidasi bundletool kalau ada.
mkdir -p "$g/native" "$g/rust/rumus/src"
printf '#include <stdint.h>\nint64_t kali_dua(int64_t x) { return x * 2; }\n' > "$g/native/hitung.c"
printf '#include <cstdint>\n#include <string>\nextern "C" int64_t panjang_teks() { return (int64_t)std::string("halo jrg").size(); }\n' > "$g/native/teks.cpp"
printf '[package]\nname = "rumus"\nversion = "0.1.0"\nedition = "2021"\n[lib]\ncrate-type = ["cdylib"]\n' > "$g/rust/rumus/Cargo.toml"
printf '#[no_mangle]\npub extern "C" fn tambah(a: i64, b: i64) -> i64 { a + b }\n' > "$g/rust/rumus/src/lib.rs"
sed -i '/^kunci = /d; /^alias = /d' "$g/jrgm.toml"
printf '#include <stdio.h>\nint main(void) { puts("halo dari program C"); return 0; }\n' > "$g/native/utama.c"
printf 'izin = ["internet"]\n[android.pustaka_c]\nlibhitung = ["native/hitung.c", "native/teks.cpp"]\n[android.pustaka_rust]\nlibrumus = "rust/rumus"\n[android.program_c]\nlibhalo_exe = ["native/utama.c"]\n' >> "$g/jrgm.toml"
( cd "$g" && ulimit -v 6000000 && JRG_SOPAN=1 $M apk --abi x86_64 >"$d/sumber.txt" 2>&1 )
rm -rf "$d/x"; mkdir -p "$d/x"; unzip -q -o "$g/target/gameku.apk" 'lib/*' -d "$d/x" 2>/dev/null
sim=$("$nb/llvm-nm" -D --defined-only "$d/x/lib/x86_64/libhitung.so" 2>/dev/null | awk '{print $3}' | grep -cxE 'kali_dua|panjang_teks')
rs=$("$nb/llvm-nm" -D --defined-only "$d/x/lib/x86_64/librumus.so" 2>/dev/null | awk '{print $3}' | grep -cx tambah)
exe=$("$nb/llvm-readelf" -l "$d/x/lib/x86_64/libhalo_exe.so" 2>/dev/null | grep -c 'Requesting program interpreter: /system/bin/linker64')
if [ "$sim" = 2 ] && [ "$rs" = 1 ] && [ "$exe" = 1 ]; then echo "ok: jrgm mengompilasi C+C++ (simbol C utuh), crate Rust, dan program C (executable) ke lib/x86_64/"
else echo "GAGAL: hasil kompilasi sumber proyek (C/C++ $sim/2, Rust $rs/1, program $exe/1): $(tail -3 "$d/sumber.txt")"; gagal=1; fi
if "$bt/aapt2" dump permissions "$g/target/gameku.apk" 2>/dev/null | grep -q "android.permission.INTERNET"; then echo "ok: izin internet masuk manifes"
else echo "GAGAL: izin internet tidak ada di manifes"; gagal=1; fi
sed -i 's/^izin = \["internet"\]/izin = ["internet", "kamera"]/' "$g/jrgm.toml"
k=$( cd "$g" && JRG_SOPAN=1 $M apk --abi x86_64 2>&1 )
case "$k" in *"izin \`kamera\` harus diminta lewat dialog"*) echo "ok: izin berbahaya ditolak, tidak dipasang diam-diam";;
  *) echo "GAGAL: izin kamera tidak ditolak: $k"; gagal=1;; esac
sed -i 's/^izin = \["internet", "kamera"\]/izin = ["internet"]/' "$g/jrgm.toml"
if [ -f "${JRG_BUNDLETOOL:-$HOME/.cache/jrgm/bundletool.jar}" ]; then
  ( cd "$g" && ulimit -v 6000000 && JRG_SOPAN=1 $M apk --aab --abi x86_64 >/dev/null 2>&1 )
  if java -jar "${JRG_BUNDLETOOL:-$HOME/.cache/jrgm/bundletool.jar}" validate --bundle="$g/target/gameku.aab" >/dev/null 2>&1 \
     && unzip -Z1 "$g/target/gameku.aab" | grep -q '^base/lib/x86_64/libhitung.so$' && jarsigner -verify "$g/target/gameku.aab" >/dev/null 2>&1; then
    echo "ok: AAB sah (bundletool validate, jarsigner), pustaka ikut"
  else echo "GAGAL: AAB tidak sah"; gagal=1; fi
else
  echo "dilewati: AAB (butuh bundletool di ~/.cache/jrgm/bundletool.jar atau JRG_BUNDLETOOL)"
fi

# 7. Tampilan "web": jrgc Android ikut sebagai executable, aktivitas WebView dari javac + d8,
#    sumber proyek sebagai aset. (Build pertama sesudah sumber jrgc berubah: belasan menit.)
if command -v javac >/dev/null; then
  ( cd "$d" && $M new webku --kind web >/dev/null 2>&1 && cd webku && ulimit -v 6000000 && JRG_SOPAN=1 $M apk --abi x86_64 >"$d/web.txt" 2>&1 )
  w=$d/webku/target/webku.apk
  isi=$(unzip -Z1 "$w" 2>/dev/null | grep -cxE 'classes.dex|assets/proyek/src/main.jrg|assets/proyek/jrgm.toml|lib/x86_64/libjrgc.so')
  akt=$("$bt/aapt2" dump badging "$w" 2>/dev/null | grep -cE "launchable-activity: name='jrg.apk.Aktivitas'|uses-permission: name='android.permission.INTERNET'")
  rm -rf "$d/xw"; mkdir -p "$d/xw"; unzip -q -o "$w" 'lib/*' -d "$d/xw" 2>/dev/null
  jenis=$("$nb/llvm-readelf" -l "$d/xw/lib/x86_64/libjrgc.so" 2>/dev/null | grep -c 'Requesting program interpreter: /system/bin/linker64')
  if [ "$isi" = 4 ] && [ "$akt" = 2 ] && [ "$jenis" = 1 ] && "$bt/apksigner" verify "$w" >/dev/null 2>&1; then
    echo "ok: tampilan web membawa jrgc Android (executable), aktivitas WebView, sumber proyek, izin internet"
  else echo "GAGAL: APK tampilan web (isi $isi/4, manifes $akt/2, executable $jenis/1): $(grep -v 'syntax OK\|WARNING' "$d/web.txt" | tail -3)"; gagal=1; fi
  # Izin berbahaya BOLEH di tampilan web: aktivitasnya yang memunculkan dialognya saat jalan.
  sed -i 's/^tampilan = "web"/tampilan = "web"\nizin = ["kamera"]/' "$d/webku/jrgm.toml"
  ( cd "$d/webku" && ulimit -v 6000000 && JRG_SOPAN=1 $M apk --abi x86_64 >"$d/izin.txt" 2>&1 )
  if "$bt/aapt2" dump permissions "$w" 2>/dev/null | grep -q "android.permission.CAMERA"; then echo "ok: izin kamera masuk manifes tampilan web"
  else echo "GAGAL: izin kamera tidak masuk manifes tampilan web: $(grep -v 'syntax OK\|WARNING' "$d/izin.txt" | tail -3)"; gagal=1; fi
  # Proyek C (Fase JRGM-3): programnya executable `libprogram.so` di APK, halaman konsol yang menjalankannya.
  ( cd "$d" && $M new konsolku --c >/dev/null 2>&1 && cd konsolku && ulimit -v 6000000 && JRG_SOPAN=1 $M apk --abi x86_64 >"$d/konsol.txt" 2>&1 )
  kw=$d/konsolku/target/konsolku.apk
  isi=$(unzip -Z1 "$kw" 2>/dev/null | grep -cxE 'lib/x86_64/libprogram.so|lib/x86_64/libjrgc.so|assets/proyek/_konsol/main.jrg|assets/proyek/_konsol/index.html|assets/proyek/src/main.c')
  rm -rf "$d/xk"; mkdir -p "$d/xk"; unzip -q -o "$kw" 'lib/*' -d "$d/xk" 2>/dev/null
  jenis=$("$nb/llvm-readelf" -l "$d/xk/lib/x86_64/libprogram.so" 2>/dev/null | grep -c 'Requesting program interpreter: /system/bin/linker64')
  if [ "$isi" = 5 ] && [ "$jenis" = 1 ] && "$bt/apksigner" verify "$kw" >/dev/null 2>&1; then
    echo "ok: proyek C jadi APK konsol (program executable, halaman konsol, sumber proyek)"
  else echo "GAGAL: APK konsol proyek C (isi $isi/5, executable $jenis/1): $(grep -v 'WARNING' "$d/konsol.txt" | tail -3)"; gagal=1; fi
  # Bahasa lain: nama asing ditolak sebelum membangun; Lua dikompilasi kalau arsipnya sudah ada
  # di ~/.cache/jrgm/sumber/ (sapuan tidak mengunduh).
  sed -i 's/^tampilan = "web"/tampilan = "web"\nbahasa = ["cobol"]/' "$d/webku/jrgm.toml"
  k=$( cd "$d/webku" && JRG_SOPAN=1 $M apk --abi x86_64 2>&1 )
  case "$k" in *"bahasa \`cobol\` di [android] bahasa tidak dikenal"*"\`lua\`"*) echo "ok: bahasa tanpa resep ditolak sambil menyebut yang ada";;
    *) echo "GAGAL: bahasa asing tidak ditolak: $(echo "$k" | tail -2)"; gagal=1;; esac
  if [ -f "$HOME/.cache/jrgm/sumber/lua-5.4.7.tar.gz" ]; then
    sed -i 's/^bahasa = .*/bahasa = ["lua"]/' "$d/webku/jrgm.toml"
    ( cd "$d/webku" && ulimit -v 6000000 && JRG_SOPAN=1 $M apk --abi x86_64 >"$d/bahasa.txt" 2>&1 )
    rm -rf "$d/xb"; mkdir -p "$d/xb"; unzip -q -o "$w" 'lib/*' -d "$d/xb" 2>/dev/null
    if "$nb/llvm-readelf" -l "$d/xb/lib/x86_64/liblua_exe.so" 2>/dev/null | grep -q 'Requesting program interpreter: /system/bin/linker64'; then
      echo "ok: bahasa = [\"lua\"] membawa penjalan Lua (executable) di lib/x86_64/"
    else echo "GAGAL: penjalan Lua tidak ada di APK: $(grep -v 'syntax OK\|WARNING' "$d/bahasa.txt" | tail -3)"; gagal=1; fi
  else
    echo "dilewati: penjalan Lua (arsip sumbernya belum pernah diunduh jrgm)"
  fi
else
  echo "dilewati: tampilan web (butuh javac)"
fi

# 8. Di perangkat sungguhan, cuma kalau diminta (`JRG_UJI_ADB=1`): adb menyalakan servernya sendiri.
adb=$sdk/platform-tools/adb
[ -x "$adb" ] || adb=$(command -v adb || true)
if [ "${JRG_UJI_ADB:-}" = 1 ] && [ -n "$adb" ] && "$adb" get-state >/dev/null 2>&1; then
  abi=$("$adb" shell getprop ro.product.cpu.abi | tr -d '\r')
  ( ulimit -v 6000000; JRG_SOPAN=1 $J "$d/halo.jrg" --apk "$d/halo.apk" --apk-abi "$abi" >/dev/null 2>&1 )
  "$adb" logcat -c; "$adb" install -r "$d/halo.apk" >/dev/null
  "$adb" shell am start -W -n id.jrg.halo/android.app.NativeActivity >/dev/null; sleep 3
  k=$("$adb" logcat -d -s jrg:I | grep -o 'halo dari android 3' | head -1)
  if [ "$k" = "halo dari android 3" ]; then echo "ok $abi: keluaran program sampai di logcat"
  else echo "GAGAL $abi: logcat tidak memuat keluaran program"; gagal=1; fi
  "$adb" uninstall id.jrg.halo >/dev/null 2>&1
else
  echo "dilewati: perangkat (JRG_UJI_ADB=1 dengan HP/emulator di adb)"
fi
exit $gagal
