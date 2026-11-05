#!/bin/sh
# Uji fitur IDE sejak 0.6 (0.7: alat kode; 0.8: terminal, bahasa terkompilasi) di chromium headless (pelengkap perkakas/ide/uji_ide.sh, yang tetap
# menguji 0.1 sampai 0.5). Pakai: sh ide/uji/uji.sh dari akar repo; JRG_UJI_JRGC untuk jrgc lain.
set -u
cd "$(dirname "$0")/../.."
J=${JRG_UJI_JRGC:-$PWD/target/release/jrgc}
krom=$(command -v chromium || command -v chromium-browser || command -v google-chrome || true)
if [ -z "$krom" ] || ! command -v node >/dev/null || ! command -v curl >/dev/null; then
  echo "dilewati: butuh chromium, node dan curl"; exit 0
fi
d=$(mktemp -d); trap 'kill $p 2>/dev/null; rm -rf "$d"' EXIT
port=$((48000 + $$ % 900))
( cd ide && exec env HOME="$d" PORT=$port JRG_JRGC="$J" JRG_SOPAN=1 "$J" src/main.jrg ) >"$d/pelayan.txt" 2>&1 &
p=$!
i=0; while [ $i -lt 80 ] && ! curl -s -o /dev/null "http://127.0.0.1:$port/"; do i=$((i + 1)); sleep 0.25; done
h=$(node ide/uji/peramban.js "$krom" "http://127.0.0.1:$port/" 2>&1)
gagal=0
periksa() {  # periksa <kunci> <nilai yang diharapkan> <penjelasan>
  case "$h" in
    *"\"$1\":\"$2"*) echo "ok: $3";;
    *) echo "GAGAL: $3 — $1 harusnya diawali \"$2\""; gagal=1;;
  esac
}
periksa logo "128|true|#0B1230 #B8CCFF" "logo JRG++ di kepala dan ikon tab; tema Air mengikuti logo"
periksa ikon "8|true|true|image/png|true" "ikon dan logo: bentuk, logo IDE diganti lalu dikembalikan, PNG proyek mengisi [android] ikon"
periksa logoPolos "none|false" "logo IDE bisa disembunyikan (polos) dan pilihannya tersimpan"
periksa pohon "2c,2c,2u" "pohon berkas bergaris: dua kolom di kedalaman dua, anak terakhir berujung"
periksa periksa "3:" "pemeriksa galat menandai baris 3 di talang"
periksa masalah "proyekku/src/rusak.jrg:3:19" "panel Masalah menyebut berkas:baris:kolom"
periksa simbolBerkas "struct T:2, impl T:3, fn m:4, fn f:6" "simbol berkas JRG"
periksa saran "pub fn g() { str::split()|str::split(s, pemisah) -> array" "saran ketik menulis str::split() dan menampilkan tanda tangannya"
periksa palet "argumen.jrg" "palet buka cepat"
periksa argumen "[satu, dua tiga]" "argumen program sampai ke env::args(), kutip mengelompokkan"
periksa uji "Uji · Gagal, kode 1|--- 2 test: 1 lulus, 1 gagal ---|1" "mode uji menjalankan test_*/uji_* dan menautkan barisnya"
periksa unggah "1 berkas, 700000 byte, utuh" "unggah berpotongan: berkas biner 700 KB tiba utuh"
periksa zip "a/x.txt=halo,a/b/y.txt=3" "zip folder untuk unduhan peramban"
periksa salin "proyekku/src/rusak-salinan.jrg" "gandakan berkas dari pohon"
periksa kembali "true,false,false" "tombol kembali menutup lembar dulu, lalu menyerahkan ke Android"
periksa latarGambar "2,true,true" "latar gambar dari pemilih berkas, panel jadi tembus"
periksa latarVideo "true,true,terbaca,true" "latar video: tanpa suara, berulang, terbaca"
periksa latarHapus "false,false" "latar dihapus"
periksa jendela "300|xyzfn f2989() { wok(2989) }|urung persis" "berkas 3000 baris: textarea memegang 300 baris, ketikan di baris 2990 tersimpan, urung persis"
periksa definisi "kd/b.jrg|Br 3, Kol 8" "F12: ke definisi di berkas lain, kursor di namanya"
periksa rujukan "2|kalikan" "Shift+F12: rujukan di semua berkas"
periksa gantiNama "1/0,1/0" "F2: ganti nama simbol di dua berkas"
periksa rapikanPenuh "pub fn main() {|true" "rapikan penuh lewat formatter JRG: spasi di dalam baris juga"
periksa rapikan '\"package main\\n\\npub fn main() {\\n    if true {\\n        wok(\\\"a {\\\")\\n    }\\n}\\n\"' "rapikan: indentasi menurut kurung, teks dilewati"
periksa pindahBaris 'pub fn main() {|wok(\"a {\")|if true {' "Alt+atas memindahkan baris"
periksa hapusBaris "7" "Ctrl+Shift+K menghapus baris"
periksa kurung "2" "pasangan kurung disorot"
periksa tabKeadaan "Br 3, Kol 13|true" "pindah tab: kursor dan riwayat urung tetap"
periksa cuplikanSaran "forifor rentang" "cuplikan muncul di saran"
periksa cuplikan '\"for i in 0..10 {\\n}pub fn main() {\"' "cuplikan fori ditulis, kursor di tempatnya"
periksa riwayatBeda "1- versi satu|1+ versi dua" "riwayat: beda baris dengan versi sekarang"
periksa riwayat '\"versi satu\\n\"' "riwayat: versi lama dipulihkan"
periksa terhapus "isi yang dihapus" "berkas terhapus dipulihkan"
periksa todo "kd/todo.jrg:1,kd/todo.jrg:2" "daftar TODO/FIXME, kata utuh"
periksa sesi "a.jrg,b.jrg,rapi.jrg,rw.jrg,hilang.jrg|kd/b.jrg" "tab terbuka dipulihkan sesudah muat ulang"
periksa cariPola "pub fn kali3(x) { y / 3 }" "cari dan ganti dengan pola dan kata utuh, \$1 di pengganti"
periksa paletSimbol "Br 3, Kol 1" "palet @ melompat ke simbol"
periksa profil "Profil|true|true|f,main" "jalankan dengan profil: laporan jadi kartu per fungsi"
periksa termLaci "true|none|true|true" "terminal penuh di HP: kepala disembunyikan, laci berkas dibuka dari bilah sesi dan tampil di atas terminal"
periksa termWarna "vf1 vt|600" "terminal: PTY sungguhan, warna ANSI dan tebal"
periksa termCtrl "1|130" "terminal: Ctrl dari baris tombol + c menghentikan sleep (kode 130)"
periksa termAlt "masuk|keluar|true" "terminal: layar alternatif masuk dan keluar"
periksa termUkuran "true|" "terminal: ukuran layar sampai ke stty size"
periksa termLebar "true" "terminal: huruf lebar dua sel"
periksa termTempel "true" "terminal: tempelan banyak baris tetap banyak baris"
periksa konsolTempel "true|ok" "konsol berbaris: tempelan banyak baris (kotak tumbuh, dijalankan sebagai skrip)"
periksa termBuka "dibuka.txt" "terminal: buka <berkas> membuka penyunting (OSC)"
periksa termJalan "di-terminal 42" "Ctrl+Shift+Enter menjalankan berkas di terminal"
periksa termPulih "2|sesi-2|di-terminal 42" "terminal: panel terbuka lagi, dua sesi dan isinya pulih sesudah muat ulang"
periksa termUlang "false|kd/term.jrg" "terminal: memulihkan sesi tidak menjalankan lagi perintah IDE di keluaran lamanya"
periksa termTutup "1" "terminal: sesi ditutup di pelayan juga"
periksa simbolSemua "tg/alat.jrg|Br 3, Kol 1" "Ctrl+T: simbol di semua berkas, membuka berkasnya di barisnya"
periksa tugas "tg:jrgc main.jrg --vm|true" "perintah proyek tersimpan per proyek dan jalan di terminal dari akarnya"
periksa huruf "15/15px|21/21px" "ukuran huruf: Ctrl+= dan cubit dua jari di penyunting"
periksa rapiSimpan '7:        wok(\"a\")' "rapikan saat simpan (Pengaturan)"
periksa cadangan "mint|true,false" "pengaturan dicadangkan ke berkas dan dipulihkan"
periksa tutupLain "acak.jrg" "tutup tab lain"
periksa pilihan "ok|pilihan 11" "jalankan pilihan: cuma baris terpilih, import berkasnya ikut"
periksa f8 "Br 4, Kol 12" "F8 melompat ke masalah berikutnya di berkas lain"
periksa banding "tg/pilih.jrg → tg/salah.jrg: +1 −5 baris" "bandingkan dengan berkas lain"
case "$h" in
  *'"bahasaC":"tanpa"'*) echo "dilewati: bahasa C (tidak ada kompilator C di mesin ini)";;
  *) periksa bahasaC "C 42|kd/c2.c:2" "C dikompilasi lalu dijalankan; galat kompilator bertaut ke barisnya";;
esac
case "$h" in *'"galat":[]'*) echo "ok: nol galat JavaScript";; *) echo "GAGAL: galat JavaScript: $h"; gagal=1;; esac
[ $gagal = 0 ] || { echo "$h"; head -5 "$d/pelayan.txt"; }
sh ide/uji/uji_linux.sh || gagal=1
sh ide/uji/uji_penjaga.sh || gagal=1
sh ide/uji/uji_proyek.sh || gagal=1
sh ide/uji/uji_native.sh || gagal=1
exit $gagal
