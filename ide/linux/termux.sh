#!/data/data/com.termux/files/usr/bin/sh
# jrg-ide — perintah Termux untuk bertukar berkas dengan JRG IDE lewat pelayannya di 127.0.0.1 (ide/README.md,
# "Termux"). Pasang: curl -s http://127.0.0.1:@PORT@/termux.sh > $PREFIX/bin/jrg-ide && chmod +x $PREFIX/bin/jrg-ide
set -eu
PORT=${JRG_IDE_PORT:-@PORT@}
U="http://127.0.0.1:$PORT"
command -v curl >/dev/null || { echo "jrg-ide: butuh curl (pkg install curl)" >&2; exit 1; }
hidup() { curl -fs -o /dev/null --max-time 3 "$U/api/info"; }
hidup || { echo "jrg-ide: JRG IDE tidak menjawab di port $PORT; buka aplikasinya dulu (JRG_IDE_PORT untuk port lain)" >&2; exit 1; }
# Jalur dalam URL: tiap byte jadi %XX kecuali '/'.
enc() { printf '%s' "$1" | od -An -v -tx1 | tr -d '\n' | sed -e 's/ 2f/\//g' -e 's/ \([0-9a-f][0-9a-f]\)/%\1/g'; }
sah() {
    case "$1" in ''|/*|*'"'*|*\\*|*..*) echo "jrg-ide: nama di IDE tidak sah: $1" >&2; exit 2;; esac
}
cmd=${1:-bantuan}
[ $# -gt 0 ] && shift
case "$cmd" in
    ls|daftar)
        curl -fsS "$U/api/berkas" | grep -o '"nama":"[^"]*"' | sed 's/"nama":"\(.*\)"/\1/' | grep "^${1:-}" || true ;;
    ambil|get)
        [ $# -ge 1 ] || { echo "pakai: jrg-ide ambil <nama-di-ide> [tujuan]" >&2; exit 2; }
        sah "$1"
        t=${2:-$(basename "$1")}
        [ -d "$t" ] && t="$t/$(basename "$1")"
        curl -fsS "$U/berkas/$(enc "$1")" -o "$t" || { echo "jrg-ide: $1 tidak ada di IDE" >&2; exit 1; }
        echo "$1 -> $t" ;;
    kirim|put)
        [ $# -ge 1 ] && [ -f "$1" ] || { echo "pakai: jrg-ide kirim <berkas> [nama-di-ide]" >&2; exit 2; }
        n=${2:-$(basename "$1")}
        sah "$n"
        d=$(mktemp -d "${TMPDIR:-/tmp}/jrg-ide.XXXXXX")
        trap 'rm -rf "$d"' EXIT
        split -b 393216 -a 4 "$1" "$d/p."
        [ -s "$1" ] || : > "$d/p.aaaa"
        awal=true
        for p in "$d"/p.*; do
            { printf '{"nama":"%s","awal":%s,"isi":"' "$n" "$awal"; base64 < "$p" | tr -d '\n'; printf '"}'; } > "$d/badan"
            curl -fsS -X POST --data-binary @"$d/badan" "$U/api/unggah" >/dev/null || { echo "jrg-ide: gagal mengirim $1" >&2; exit 1; }
            awal=false
        done
        echo "$1 -> IDE:$n" ;;
    *)
        cat <<X
jrg-ide — tukar berkas dengan JRG IDE (port $PORT)
  jrg-ide ls [awalan]                daftar berkas di IDE
  jrg-ide ambil <nama> [tujuan]      salin dari IDE ke Termux
  jrg-ide kirim <berkas> [nama]      salin dari Termux ke IDE (nama boleh berfolder: proyek/a.py)
Folder bersama lewat memori HP: ~/storage/shared/JRG-bersama (termux-setup-storage),
yang di Ubuntu JRG IDE terlihat sebagai ~/bersama (setup-storage).
X
        [ "$cmd" = bantuan ] || [ "$cmd" = help ] || exit 2 ;;
esac
