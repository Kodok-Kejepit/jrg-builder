// tests/jit_parity.rs — mesin keempat (`--jit`, `dokumentasi/jit/07-kebenaran-dan-uji.md`).
// Fase J1: `--jit` = VM berprofil; tabelnya sudah dipasang sekarang supaya tiap fase sesudahnya
// (J2 kode mesin, J3 deopt) langsung diukur terhadap seluruh `PARITAS`.

mod bersama;
use std::process::Command;

fn jrgc() -> &'static str { env!("CARGO_BIN_EXE_jrgc") }

include!("bersama/paritas.rs");

struct Hasil { stdout: String, stderr: String, sukses: bool }

fn jalankan(src: &str, arg: &[&str], env: &[(&str, &str)]) -> Hasil {
    let dir = bersama::dir_unik("jrg-jit");
    std::fs::create_dir_all(&dir).expect("bikin dir sementara");
    let file = dir.join("uji.jrg");
    std::fs::write(&file, src).expect("tulis sumber");
    let mut cmd = Command::new(jrgc());
    cmd.arg(&file).args(arg).env("JRG_SOPAN", "1").env_remove("JRG_JIT_LOG")
        .stdin(std::process::Stdio::null());
    for (k, v) in env { cmd.env(k, v); }
    let out = cmd.output().expect("jalankan jrgc");
    let _ = std::fs::remove_dir_all(&dir);
    Hasil {
        stdout: String::from_utf8_lossy(&out.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        sukses: out.status.success(),
    }
}

#[test]
fn paritas_lewat_jit() {
    let mut gagal = Vec::new();
    for (nama, src) in PARITAS {
        let i = jalankan(src, &[], &[]);
        let j = jalankan(src, &["--jit"], &[]);
        if i.sukses != j.sukses || i.stdout != j.stdout {
            gagal.push(format!("  {nama}\n     interpreter ({}): {:?}\n     jit         ({}): {:?}",
                i.sukses, i.stdout, j.sukses, j.stdout));
        }
    }
    assert!(gagal.is_empty(), "{} kasus PARITAS beda di --jit:\n{}", gagal.len(), gagal.join("\n"));
}

const FIB: &str = "fn fib(n) {\n  if n < 2 { return n }\n  return fib(n - 1) + fib(n - 2)\n}\nfn main() {\n  var t = 0\n  for i in 0..10 { t = t + i }\n  wok(fib(15), t)\n}\n";

#[test]
fn profil_t0_melihat_fungsi_panas() {
    let h = jalankan(FIB, &["--jit"], &[("JRG_JIT_LOG", "ringkas")]);
    assert!(h.sukses, "{}", h.stderr);
    assert_eq!(h.stdout, "610 45");
    // fib(15) memanggil fib tepat 1973 kali; angka lain berarti perekamnya salah hitung.
    let baris = h.stderr.lines().find(|b| b.contains(" fib:")).unwrap_or_else(|| panic!("fib tidak dilaporkan:\n{}", h.stderr));
    assert!(baris.contains("panggilan 1973,"), "{baris}");
    let utama = h.stderr.lines().find(|b| b.contains(" main:")).unwrap_or_else(|| panic!("main tidak dilaporkan:\n{}", h.stderr));
    assert!(utama.contains("putaran 10,"), "{utama}");
}

#[test]
fn profil_tidak_menyentuh_stdout() {
    let tanpa = jalankan(FIB, &["--jit"], &[]);
    let dengan = jalankan(FIB, &["--jit"], &[("JRG_JIT_LOG", "ringkas")]);
    assert_eq!(tanpa.stdout, dengan.stdout);
    assert!(tanpa.stderr.is_empty(), "tanpa JRG_JIT_LOG tidak boleh ada laporan: {}", tanpa.stderr);
}
