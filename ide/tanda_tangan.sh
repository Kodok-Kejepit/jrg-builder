#!/bin/sh
# Menyamakan kunci tanda tangan APK antar-mesin (laptop dan Termux), supaya APK dari mana pun bisa saling timpa.
# Pakai: sh ide/tanda_tangan.sh pasang <debug.keystore>   -> jadi ~/.android/debug.keystore (yang lama disimpan .lama)
#        sh ide/tanda_tangan.sh ulang <apk>...            -> tanda tangani ulang APK jadi dengan kunci itu
set -eu
KS="$HOME/.android/debug.keystore"
sidik() { keytool -list -v -keystore "$1" -storepass android 2>/dev/null | grep -m1 'SHA256:' | sed 's/^[[:space:]]*//'; }
alat() {
  command -v "$1" 2>/dev/null && return
  for d in "${ANDROID_HOME:-}" "${ANDROID_SDK_ROOT:-}" "$HOME/Android/Sdk" /usr/lib/android-sdk; do
    [ -n "$d" ] && [ -d "$d/build-tools" ] || continue
    b=$(ls -d "$d"/build-tools/* 2>/dev/null | sort -V | tail -1)
    [ -x "$b/$1" ] && { echo "$b/$1"; return; }
  done
}
case "${1:-}" in
  pasang)
    [ -f "${2:-}" ] || { echo "berkas kunci tidak ada: ${2:-}" >&2; exit 1; }
    mkdir -p "$HOME/.android"
    if [ -f "$KS" ] && ! cmp -s "$2" "$KS"; then mv "$KS" "$KS.lama"; echo "kunci lama disimpan: $KS.lama"; fi
    cp "$2" "$KS"
    echo "terpasang: $KS"
    sidik "$KS" ;;
  ulang)
    shift
    [ -f "$KS" ] || { echo "belum ada $KS; jalankan dulu: sh $0 pasang <debug.keystore>" >&2; exit 1; }
    AS=$(alat apksigner); [ -n "$AS" ] || { echo "apksigner tidak ada (Termux: pkg install apksigner)" >&2; exit 1; }
    ZA=$(alat zipalign || true)
    for apk in "$@"; do
      t="${apk%.apk}.tmp.apk"
      if [ -n "$ZA" ]; then "$ZA" -f -P 16 4 "$apk" "$t"; else cp "$apk" "$t"; fi
      "$AS" sign --ks "$KS" --ks-pass pass:android --key-pass pass:android --ks-key-alias androiddebugkey \
          --out "$apk" "$t"
      rm -f "$t" "$apk.idsig"
      "$AS" verify "$apk" && echo "ditandatangani ulang: $apk"
    done ;;
  *) sed -n '2,4p' "$0" | sed 's/^# \{0,1\}//'; exit 1 ;;
esac
