#!/bin/sh
# perkakas/bangun_praktik.sh — bangun mesin Praktik JRG (crate `web/`) lalu tulis
# `docs/aset/jrg-web.js`: wasm-nya, badan pekerjanya, dan sidik sumbernya.
set -eu
AKAR=$(cd "$(dirname "$0")/.." && pwd)
cd "$AKAR/web"
# Path mesin ini tidak boleh ikut terbit lewat string lokasi panik.
RUSTFLAGS="--remap-path-prefix=$HOME=~ --remap-path-prefix=$AKAR=jrg" \
    cargo build --release --target wasm32-unknown-unknown -j "${JOBS:-1}"
python3 "$AKAR/perkakas/praktik.py" tulis
