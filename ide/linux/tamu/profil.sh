# Ditulis JRG IDE tiap sesi Linux dimulai; ubahan pribadi taruh di ~/.bashrc di luar blok jrg-ide.
export LANG="${LANG:-C.UTF-8}"
export PAGER="${PAGER:-less}"
case "$-" in *i*) ;; *) return 0 2>/dev/null || exit 0 ;; esac

# Perintah IDE dari dalam Linux, lewat OSC 7770 yang dibaca halaman (sama dengan shell Android).
buka() {
    [ $# -gt 0 ] || { echo 'pakai: buka <berkas>...' >&2; return 2; }
    for _f in "$@"; do
        if [ -f "$_f" ]; then case "$_f" in /*) ;; *) _f="$PWD/$_f" ;; esac; printf '\033]7770;buka;%s\007' "$_f"
        else echo "buka: $_f bukan berkas" >&2; fi
    done
}
berkas() { cd "@AKAR@/$1" || return; }
alias ide=buka

if [ -n "$JRG_LINUX_LOGIN" ]; then
    unset JRG_LINUX_LOGIN
    if [ ! -f "$HOME/.hushlogin" ]; then
        printf '\nSelamat datang di \033[1m@DISTRO@\033[0m (GNU/Linux %s @ARSITEKTUR@) di JRG IDE\n\n' "$(uname -r)"
        printf '  pkg install <paket>   pasang paket (apt dan sudo juga bisa)\n'
        printf '  ~/berkas              folder berkas IDE (perintah: berkas)\n'
        printf '  buka <berkas>         buka di penyunting IDE\n'
        printf '  setup-storage         memori HP: /sdcard, ~/storage\n'
        [ -x /usr/local/bin/jrg-sumber ] && [ -f /usr/local/share/jrg-ide/jrg-lang.tar.gz ] && printf '  jrg-sumber            sumber bahasa JRG ke ~/berkas/jrg-lang\n'
        printf '  exit                  tutup sesi\n\n'
        printf '  Sembunyikan pesan ini: touch ~/.hushlogin\n\n'
    fi
    # Satu sesi saja yang menjalankan setup (apt memakai kunci eksklusif).
    if [ ! -f /var/lib/jrg-ide/setup-done ] && mkdir /var/lib/jrg-ide/setup.lock 2>/dev/null; then
        printf '\033[1mMenyiapkan paket dasar (sekali saja, butuh internet)...\033[0m\n'
        pkg setup || printf '\nSetup belum selesai. Jalankan "pkg setup" saat online.\n\n'
        rmdir /var/lib/jrg-ide/setup.lock 2>/dev/null
    fi
fi
