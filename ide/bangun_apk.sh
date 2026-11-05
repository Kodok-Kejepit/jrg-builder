#!/bin/sh
# APK JRG IDE, dua versi (ide/README.md, "Dua versi APK"): play -> target/jrg-ide.apk (targetSdk 34),
# terminal -> target/jrg-ide-terminal.apk (id.jrg.ide.terminal, targetSdk 28, seperti Termux).
# Pakai: sh ide/bangun_apk.sh [play|terminal|semua] (menjalankan cargo). Nama: `nama` di jrgm.toml, atau NAMA=... terminal.
set -eu
IDE=$(cd "$(dirname "$0")" && pwd)
JRGM="$(dirname "$IDE")/target/release/jrgm"
[ -x "$JRGM" ] || JRGM=$(command -v jrgm) || { echo "jrgm tidak ditemukan (target/release/jrgm atau PATH)" >&2; exit 1; }
MAU=${1:-semua}
if pgrep -x cargo >/dev/null; then
  echo "cargo sedang jalan; \`jrgm apk\` juga menjalankan cargo. Tunggu sampai selesai, lalu ulangi." >&2
  exit 1
fi
alat_bt() {
  # Termux: zipalign dan apksigner dari pkg, di PATH.
  if command -v zipalign >/dev/null && command -v apksigner >/dev/null; then echo PATH; return; fi
  for d in "${ANDROID_HOME:-}" "${ANDROID_SDK_ROOT:-}" "$HOME/Android/Sdk" /usr/lib/android-sdk; do
    [ -n "$d" ] && [ -d "$d/build-tools" ] || continue
    b=$(ls -d "$d"/build-tools/* 2>/dev/null | sort -V | tail -1)
    [ -x "$b/apksigner" ] && { echo "$b"; return; }
  done
  echo "build-tools Android (zipalign, apksigner) tidak ditemukan" >&2; exit 1
}
# Layanan latar (varian/latar/Latar.java) jadi classes2.dex: javac dari PATH, android.jar dan d8 dari SDK yang sama
# dengan `jrgm apk`. Tanpa alatnya APK tetap jadi, cuma tanpa notifikasi latar.
dex_latar() {
  command -v javac >/dev/null || { echo "javac tidak ada" >&2; return 1; }
  jar=""; d8=$(command -v d8 || true)
  for d in "${ANDROID_HOME:-}" "${ANDROID_SDK_ROOT:-}" "$HOME/Android/Sdk" /usr/lib/android-sdk; do
    [ -n "$d" ] && [ -d "$d" ] || continue
    [ -n "$jar" ] || jar=$(ls -d "$d"/platforms/*/android.jar 2>/dev/null | sort -V | tail -1)
    [ -n "$d8" ] || d8=$(ls -d "$d"/build-tools/*/d8 "$d"/cmdline-tools/*/bin/d8 2>/dev/null | sort -V | tail -1)
  done
  [ -n "$jar" ] && [ -n "$d8" ] || { echo "android.jar atau d8 tidak ditemukan" >&2; return 1; }
  rm -rf "$1/kelas" "$1/dex" && mkdir -p "$1/kelas" "$1/dex"
  javac -source 11 -target 11 -Xlint:-options -nowarn -cp "$jar" -d "$1/kelas" "$IDE/varian/latar/Latar.java" >&2 &&
    "$d8" --min-api 26 --lib "$jar" --output "$1/dex" $(find "$1/kelas" -name '*.class') >&2 &&
    [ -f "$1/dex/classes.dex" ]
}
# Sumber bahasa JRG ikut di versi Terminal (perintah `jrg-sumber` di Ubuntu). Daftar yang BOLEH ikut, bukan yang
# dilarang: berkas nyasar di akar (zip, sesi chat, unduhan) tidak pernah terbawa. Tanpa hasil bangun dan kunci.
SUMBER_JRG="Cargo.toml Cargo.lock README.md CHANGELOG.md LICENSE CONTRIBUTING.md panduan
  .gitignore src runtime jembatan stdlib editor web examples examples_llvm examples_projects
  benchmarks perkakas rilis ide"
arsip_sumber() {
  ada=""
  for f in $SUMBER_JRG; do [ -e "$(dirname "$IDE")/$f" ] && ada="$ada ./$f"; done
  # shellcheck disable=SC2086
  tar -C "$(dirname "$IDE")" --exclude-vcs --exclude=target --exclude=__pycache__ --exclude=node_modules --exclude=venv \
      --exclude=.venv --exclude=.bangun --exclude='*/*/.*' --exclude='*.apk' \
      --exclude='*.aab' --exclude='*.idsig' --exclude='*.keystore' --exclude='*.jks' --exclude='*.bundle' --exclude='*.zip' \
      --exclude='*.jsonl' --exclude=a.out --exclude=a.out.ll --exclude='*.kate-swp' --exclude='*.swp' --exclude=./ide/linux/proot \
      --exclude=./ide/curl_response --transform 's,^\.,jrg-lang,' -czf "$1" $ada
}
# Runtime mesin Native untuk Ubuntu di versi Terminal (aarch64 glibc): jrgc milik APK + clang Ubuntu menautkannya.
# Lewat rustup (laptop) atau Ubuntu proot-distro ber-cargo (Termux); tanpa keduanya, dibangun di HP saat pertama dipakai.
jrgrt_glibc() {
  rt="$(dirname "$IDE")/runtime"; t=aarch64-unknown-linux-gnu
  if command -v rustup >/dev/null && rustup target list --installed 2>/dev/null | grep -qx $t; then
    (cd "$rt" && CARGO_TARGET_DIR="$IDE/target/jrgrt" cargo build --release --locked -q --target $t) >&2 &&
      cp "$IDE/target/jrgrt/$t/release/libjrgrt.a" "$1" && return 0
  fi
  [ "$(uname -m)" = aarch64 ] && command -v proot-distro >/dev/null || return 1
  tmp="$IDE/target/jrgrt-tamu"
  rm -rf "$tmp" && mkdir -p "$tmp" && cp -r "$rt/Cargo.toml" "$rt/Cargo.lock" "$rt/src" "$tmp/"
  proot-distro login ubuntu -- env -i PATH=/usr/sbin:/usr/bin:/sbin:/bin HOME=/root \
      sh -c "command -v cargo >/dev/null && cd '$tmp' && cargo build --release --offline -q" >&2 &&
    cp "$tmp/target/release/libjrgrt.a" "$1"
}
if [ "$MAU" = play ] || [ "$MAU" = semua ]; then
  (cd "$IDE" && "$JRGM" apk)
  echo "siap: $IDE/target/jrg-ide.apk ($(du -h "$IDE/target/jrg-ide.apk" | cut -f1))"
fi
if [ "$MAU" = terminal ] || [ "$MAU" = semua ]; then
  V="$IDE/target/varian-terminal"
  rm -rf "$V" && mkdir -p "$V"
  mkdir -p "$V/linux"
  cp -r "$IDE/src" "$IDE/web" "$IDE/contoh" "$IDE/aset" "$V/"
  cp -r "$IDE/linux/tamu" "$IDE/linux/jrglinux.c" "$IDE/linux/termux.sh" "$V/linux/"
  echo terminal > "$V/varian.txt"
  arsip_sumber "$V/linux/jrg-lang.tar.gz"
  echo "sumber JRG: $(du -h "$V/linux/jrg-lang.tar.gz" | cut -f1)"
  mkdir -p "$V/linux/jrgrt"
  if jrgrt_glibc "$V/linux/jrgrt/libjrgrt.a"; then
    for s in llvm-strip strip; do command -v $s >/dev/null && $s --strip-debug "$V/linux/jrgrt/libjrgrt.a" 2>/dev/null && break; done
    echo "runtime Native (Ubuntu): $(du -h "$V/linux/jrgrt/libjrgrt.a" | cut -f1)"
  else
    rm -rf "$V/linux/jrgrt"
    echo "peringatan: tanpa runtime Native bawaan; di HP dibangun cargo Ubuntu saat pertama dipakai" >&2
  fi
  # Linux (ide/README.md, "Linux"): pemasang rootfs dikompilasi jrgm; proot jadi ikut di lib/<abi>/ lewat tambal_manifes.py.
  # Port sendiri: dua versi bisa dipasang berdampingan, dan pelayan yang sama-sama di 8080 saling mengunci.
  LATAR=""
  grep -a -q FOREGROUND_SERVICE_SPECIAL_USE "$JRGM" && LATAR='"latar", '
  NAMA=$(printf '%s' "${NAMA:-JRG IDE Terminal}" | tr -d '"\\<>&#')
  sed -e "s#^nama *=.*#nama     = \"$NAMA\"#" -e 's/^paket *=.*/paket    = "id.jrg.ide.terminal"/' -e 's/^port *=.*/port     = 8081/' \
      -e "s/^izin *= *\\[/izin     = [$LATAR\"penyimpanan\", /" -e '/^abaikan *=/d' "$IDE/jrgm.toml" > "$V/jrgm.toml"
  printf '\n[android.program_c]\nlibjrglinux_exe = ["linux/jrglinux.c"]\n' >> "$V/jrgm.toml"
  (cd "$V" && "$JRGM" apk)
  BT=$(alat_bt)
  if [ "$BT" = PATH ]; then ZA=zipalign; AS=apksigner; else ZA="$BT/zipalign"; AS="$BT/apksigner"; fi
  TAMBAH=""
  ABI_APK=$(python3 -c 'import sys, zipfile; print(" ".join(n.split("/")[1] for n in zipfile.ZipFile(sys.argv[1]).namelist() if n.endswith("/libjrgc.so")))' "$V/target/jrg-ide.apk")
  for abi in $ABI_APK; do
    [ -d "$IDE/linux/proot/$abi" ] || { echo "proot untuk $abi tidak ada di ide/linux/proot (bangun-proot.sh)" >&2; exit 1; }
    for f in "$IDE/linux/proot/$abi"/lib*.so; do TAMBAH="$TAMBAH lib/$abi/$(basename "$f")=$f"; done
  done
  # Notifikasi latar: dari jrgm sendiri kalau sudah bisa (izin `latar`), selain itu layanan sisipan IDE.
  LATAR_SISIP=""
  if [ -z "$LATAR" ]; then
    if dex_latar "$V/latar"; then LATAR_SISIP=--latar; TAMBAH="$TAMBAH classes2.dex=$V/latar/dex/classes.dex"
    else echo "peringatan: tanpa layanan latar (notifikasi, Keluar, Wake lock)" >&2; fi
  fi
  # shellcheck disable=SC2086
  python3 "$IDE/varian/tambal_manifes.py" "$V/target/jrg-ide.apk" "$V/sdk28.apk" 28 $LATAR_SISIP $TAMBAH
  "$ZA" -f -P 16 4 "$V/sdk28.apk" "$V/rata.apk"
  "$AS" sign --ks "$HOME/.android/debug.keystore" --ks-pass pass:android --key-pass pass:android \
      --ks-key-alias androiddebugkey --out "$IDE/target/jrg-ide-terminal.apk" "$V/rata.apk" 2>/dev/null
  rm -f "$IDE/target/jrg-ide-terminal.apk.idsig"
  "$AS" verify "$IDE/target/jrg-ide-terminal.apk" 2>/dev/null
  echo "siap: $IDE/target/jrg-ide-terminal.apk ($(du -h "$IDE/target/jrg-ide-terminal.apk" | cut -f1))"
fi
