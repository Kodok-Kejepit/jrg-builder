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
    // Tiap jalan punya direktori sementara sendiri; jalurnya bukan bagian yang dibandingkan.
    let jalur = file.display().to_string();
    Hasil {
        stdout: String::from_utf8_lossy(&out.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).replace(&jalur, "<uji>"),
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

/// Sama dengan `FIB`, tapi menyentuh teks: tidak layak dikompilasi J2, jadi tiap panggilan lewat VM.
const FIB_VM: &str = "fn fib(n) {\n  let s = \"x\"\n  if n < 2 { return n }\n  return fib(n - 1) + fib(n - 2)\n}\nfn main() {\n  var t = 0\n  for i in 0..10 { t = t + i }\n  wok(fib(15), t)\n}\n";

#[test]
fn profil_t0_melihat_fungsi_panas() {
    let h = jalankan(FIB_VM, &["--jit"], &[("JRG_JIT_LOG", "ringkas")]);
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

// ── J2: fungsi bulat murni sebagai kode mesin (`dokumentasi/jit/j2-rencana.md`) ──────────────

const STRES: (&str, &str) = ("JRG_JIT_AMBANG", "0");

#[test]
fn paritas_lewat_jit_stres() {
    let mut gagal = Vec::new();
    for (nama, src) in PARITAS {
        let i = jalankan(src, &[], &[]);
        let j = jalankan(src, &["--jit"], &[STRES]);
        if i.sukses != j.sukses || i.stdout != j.stdout {
            gagal.push(format!("  {nama}\n     interpreter ({}): {:?}\n     jit stres   ({}): {:?}",
                i.sukses, i.stdout, j.sukses, j.stdout));
        }
    }
    assert!(gagal.is_empty(), "{} kasus PARITAS beda di --jit (stres):\n{}", gagal.len(), gagal.join("\n"));
}

#[test]
fn fungsi_bulat_dikompilasi() {
    let h = jalankan(FIB, &["--jit"], &[("JRG_JIT_LOG", "ringkas")]);
    assert!(h.sukses, "{}", h.stderr);
    assert_eq!(h.stdout, "610 45");
    assert!(h.stderr.lines().any(|b| b.contains("dikompilasi:") && b.contains("fib")), "fib tidak dikompilasi:\n{}", h.stderr);
}

/// Program yang hasilnya sama di interpreter dan `--jit` (stres): stdout, stderr dan status.
fn sama_dengan_interpreter(src: &str) {
    let i = jalankan(src, &[], &[]);
    let j = jalankan(src, &["--jit"], &[STRES]);
    assert_eq!((i.sukses, &i.stdout, &i.stderr), (j.sukses, &j.stdout, &j.stderr));
}

#[test]
fn luapan_mundur_ke_128_bit() {
    sama_dengan_interpreter("fn kali(a, b) { return a * b }\nfn tambah(a, b) { return a + b }\nfn main() {\n  wok(kali(3, 4))\n  wok(kali(kali(3037000500, 3037000500), 4))\n  wok(tambah(9223372036854775807, 1))\n}\n");
}

#[test]
fn bagi_nol_berkalimat_sama() {
    sama_dengan_interpreter("fn sisa(a, b) { return a % b }\nfn main() {\n  wok(sisa(7, 3), sisa(-7, 3))\n  wok(sisa(7, 0))\n}\n");
}

#[test]
fn rekursi_dalam_mundur_ke_vm() {
    let src = "fn turun(n) {\n  if n == 0 { return 0 }\n  return turun(n - 1) + 1\n}\nfn main() { wok(turun(20000)) }\n";
    let v = jalankan(src, &["--vm"], &[]);
    let j = jalankan(src, &["--jit"], &[STRES]);
    assert_eq!((v.sukses, v.stdout.as_str()), (true, "20000"));
    assert_eq!((j.sukses, j.stdout.as_str()), (true, "20000"));
}

// ── J3: deopt di tengah fungsi, bingkai native disusun ulang jadi bingkai VM (`j3-rencana.md`) ───

const ACAK: (&str, &str) = ("JRG_JIT_STRES", "deopt-acak");

#[test]
fn paritas_lewat_jit_deopt_acak() {
    let mut gagal = Vec::new();
    for (nama, src) in PARITAS {
        let i = jalankan(src, &[], &[]);
        let j = jalankan(src, &["--jit"], &[STRES, ACAK]);
        if i.sukses != j.sukses || i.stdout != j.stdout {
            gagal.push(format!("  {nama}\n     interpreter ({}): {:?}\n     deopt acak  ({}): {:?}",
                i.sukses, i.stdout, j.sukses, j.stdout));
        }
    }
    assert!(gagal.is_empty(), "{} kasus PARITAS beda di --jit (deopt acak):\n{}", gagal.len(), gagal.join("\n"));
}

/// Luapan di callee terdalam: seluruh rantai native lanjut di VM dari titik itu, hasilnya 128 bit.
const RANTAI: &str = "fn g(x) { return x * x }\nfn f(n) {\n  if n == 0 { return g(4000000000) }\n  return f(n - 1) + 1\n}\nfn main() { wok(f(5), f(30)) }\n";

#[test]
fn deopt_di_tengah_rantai_native() {
    sama_dengan_interpreter(RANTAI);
    let h = jalankan(RANTAI, &["--jit"], &[STRES, ("JRG_JIT_LOG", "ringkas")]);
    assert_eq!(h.stdout, "16000000000000000005 16000000000000000030");
    assert!(h.stderr.lines().any(|b| b == "[jit] deopt: 2"), "tiap luapan tepat satu deopt:\n{}", h.stderr);
}

#[test]
fn deopt_acak_sama_dengan_interpreter() {
    let src = "fn sisa(a, b) { return a % b }\nfn fib(n) {\n  if n < 2 { return n }\n  return fib(n - 1) + fib(n - 2)\n}\nfn main() {\n  wok(fib(18), sisa(-7, 3))\n  wok(sisa(7, 0))\n}\n";
    let i = jalankan(src, &[], &[]);
    let j = jalankan(src, &["--jit"], &[STRES, ACAK]);
    assert_eq!((i.sukses, &i.stdout, &i.stderr), (j.sukses, &j.stdout, &j.stderr));
}
