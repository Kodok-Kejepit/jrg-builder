#!/bin/sh
# proot (termux/proot + patch di patches/) dan loader-nya untuk ABI Android, dengan NDK; hasilnya <abi>/lib*.so di
# sebelah skrip ini. Berasal dari Woksole (scripts/build-engine.sh). Butuh git, curl, python3, gawk, make;
# variabel: ANDROID_NDK_HOME (atau ANDROID_HOME/ndk/<versi>), JOBS, FORCE=1, ABI="arm64-v8a x86_64".
set -eu

PROOT_REPO=https://github.com/termux/proot.git
PROOT_TAG=v5.1.107.96
PROOT_COMMIT=a179d3e8a4e045aaa1fb8cc3284f23509d96d353
TALLOC_VERSION=2.4.2
TALLOC_SHA256=85ecf9e465e20f98f9950a52e9a411e14320bc555fa257d87697b7e7a9b1d8a6
API=24

ROOT="$(cd "$(dirname "$0")" && pwd)"
CACHE="$ROOT/.bangun"
JNILIBS="$ROOT"
JOBS="${JOBS:-$(nproc 2>/dev/null || echo 4)}"

die() { echo "build-engine: $*" >&2; exit 1; }

for tool in git curl python3 gawk make; do
    command -v "$tool" >/dev/null || die "butuh '$tool' di host"
done

find_ndk() {
    for d in "${ANDROID_NDK_HOME:-}" "${ANDROID_NDK_ROOT:-}" "${ANDROID_NDK_LATEST_HOME:-}"; do
        [ -n "$d" ] && [ -d "$d/toolchains/llvm" ] && { echo "$d"; return; }
    done
    sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
    if [ -z "$sdk" ] && [ -f "$ROOT/local.properties" ]; then
        sdk="$(sed -n 's/^sdk\.dir=//p' "$ROOT/local.properties")"
    fi
    [ -n "$sdk" ] && ls -d "$sdk"/ndk/* 2>/dev/null | sort -V | tail -n 1
}
NDK="$(find_ndk)"
[ -n "$NDK" ] || die "NDK tidak ditemukan; set ANDROID_NDK_HOME"
TC="$(ls -d "$NDK"/toolchains/llvm/prebuilt/*/bin | head -n 1)"
echo "NDK: $NDK"

mkdir -p "$CACHE"

# ---- Sumber ---------------------------------------------------------------
TALLOC_TAR="$CACHE/talloc-$TALLOC_VERSION.tar.gz"
if [ ! -f "$TALLOC_TAR" ]; then
    for url in "https://www.samba.org/ftp/talloc/talloc-$TALLOC_VERSION.tar.gz" \
               "https://download.samba.org/pub/talloc/talloc-$TALLOC_VERSION.tar.gz"; do
        curl -fsSL --retry 3 "$url" -o "$TALLOC_TAR.part" && mv "$TALLOC_TAR.part" "$TALLOC_TAR" && break
    done
    [ -f "$TALLOC_TAR" ] || die "gagal mengunduh talloc"
fi
echo "$TALLOC_SHA256  $TALLOC_TAR" | sha256sum -c - >/dev/null || { rm -f "$TALLOC_TAR"; die "checksum talloc salah"; }

PROOT_SRC="$CACHE/proot-src"
if [ ! -d "$PROOT_SRC/.git" ]; then
    rm -rf "$PROOT_SRC"
    git clone -q --depth 1 --branch "$PROOT_TAG" "$PROOT_REPO" "$PROOT_SRC"
fi
got="$(git -C "$PROOT_SRC" rev-parse HEAD)"
case "$got" in "$PROOT_COMMIT"*) ;; *) die "commit proot tak terduga: $got" ;; esac

# ---- Build per ABI --------------------------------------------------------
TALLOC_ANSWERS='Checking uname sysname type: "Linux"
Checking uname machine type: "dontcare"
Checking uname release type: "dontcare"
Checking uname version type: "dontcare"
Checking simple C program: OK
building library support: OK
Checking for large file support: OK
Checking for -D_FILE_OFFSET_BITS=64: OK
Checking for WORDS_BIGENDIAN: OK
Checking for C99 vsnprintf: OK
Checking for HAVE_SECURE_MKSTEMP: OK
rpath library support: OK
-Wl,--version-script support: FAIL
Checking correct behavior of strtoll: OK
Checking correct behavior of strptime: OK
Checking for HAVE_IFACE_GETIFADDRS: OK
Checking for HAVE_IFACE_IFCONF: OK
Checking for HAVE_IFACE_IFREQ: OK
Checking getconf LFS_CFLAGS: OK
Checking for large file support without additional flags: OK
Checking for working strptime: OK
Checking for HAVE_SHARED_MMAP: OK
Checking for HAVE_MREMAP: OK
Checking for HAVE_INCOHERENT_MMAP: OK
Checking getconf large file support flags work: OK'

for abi in ${ABI:-arm64-v8a x86_64}; do
    case "$abi" in
        arm64-v8a) triple=aarch64-linux-android ;; armeabi-v7a) triple=armv7a-linux-androideabi ;;
        x86_64) triple=x86_64-linux-android ;; x86) triple=i686-linux-android ;; *) die "ABI tidak dikenal: $abi" ;;
    esac
    out="$JNILIBS/$abi"
    if [ -s "$out/libproot.so" ] && [ "${FORCE:-0}" != 1 ]; then continue; fi
    echo "== mesin untuk $abi"

    work="$CACHE/work-$abi"
    rm -rf "$work"
    mkdir -p "$work/prefix/lib" "$work/prefix/include"
    CC="$TC/$triple$API-clang"

    # talloc: hanya objek pustakanya, ditautkan statis ke proot.
    mkdir -p "$work/talloc"
    tar -xzf "$TALLOC_TAR" -C "$work/talloc" --strip-components=1
    (
        cd "$work/talloc"
        printf '%s\n' "$TALLOC_ANSWERS" > cross-answers.txt
        CC="$CC" AR="$TC/llvm-ar" ./configure --disable-rpath --disable-python \
            --cross-compile --cross-answers=cross-answers.txt >build.log 2>&1 \
            || { tail -n 30 build.log >&2; exit 1; }
        make -j"$JOBS" >>build.log 2>&1 || { tail -n 30 build.log >&2; exit 1; }
        obj="$(grep -l "_talloc_free" bin/default/talloc.c.*.o | head -n 1)"
        "$TC/llvm-ar" rcs "$work/prefix/lib/libtalloc.a" "$obj"
        cp talloc.h "$work/prefix/include/"
    )

    # proot + patch (alasannya di patches/*.patch dan SUMBER.md)
    cp -r "$PROOT_SRC" "$work/proot"
    for p in "$ROOT"/patches/proot-*.patch; do
        git -C "$work/proot" apply "$p" || die "patch gagal: $p"
    done
    (
        cd "$work/proot"
        CC="$CC" STRIP="$TC/llvm-strip" OBJCOPY="$TC/llvm-objcopy" OBJDUMP="$TC/llvm-objdump" \
        CPPFLAGS="-I$work/prefix/include -DARG_MAX=131072 -DVERSION=\\\"${PROOT_TAG#v}-woksole\\\"" \
        LDFLAGS="-L$work/prefix/lib" \
        PROOT_UNBUNDLE_LOADER=/woksole-loader-unset \
            make -C src -j"$JOBS" >build.log 2>&1 || { tail -n 30 build.log >&2; exit 1; }
    )

    mkdir -p "$out"
    "$TC/llvm-strip" -o "$out/libproot.so" "$work/proot/src/proot"
    "$TC/llvm-strip" -o "$out/libproot-loader.so" "$work/proot/src/loader/loader"
    if [ -f "$work/proot/src/loader/loader-m32" ]; then
        "$TC/llvm-strip" -o "$out/libproot-loader32.so" "$work/proot/src/loader/loader-m32"
    fi
    chmod 755 "$out"/lib*.so
    rm -rf "$work"
    ls -la "$out"
done
