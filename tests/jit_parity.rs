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
    let baris = h.stderr.lines().find(|b| b.contains(" fib: panggilan")).unwrap_or_else(|| panic!("fib tidak dilaporkan:\n{}", h.stderr));
    assert!(baris.contains("panggilan 1973,"), "{baris}");
    let utama = h.stderr.lines().find(|b| b.contains(" main: panggilan")).unwrap_or_else(|| panic!("main tidak dilaporkan:\n{}", h.stderr));
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

// ── J4: titik keluar, nilai milik VM di bingkai, OSR (`j4-rencana.md`) ─────────────────────────────

include!("bersama/paritas_llvm.rs");
include!("bersama/gagal_saat_jalan.rs");

/// Dua mode yang memaksa kode native: semua dikompilasi seketika, dan ditambah deopt acak.
const MODE_PAKSA: [&[(&str, &str)]; 2] = [&[STRES], &[STRES, ACAK]];

#[test]
fn paritas_llvm_lewat_jit() {
    let mut gagal = Vec::new();
    for (nama, src) in PARITAS_LLVM {
        let i = jalankan(src, &[], &[]);
        for env in MODE_PAKSA {
            let j = jalankan(src, &["--jit"], env);
            if i.sukses != j.sukses || i.stdout != j.stdout {
                gagal.push(format!("  {nama} {env:?}\n     interpreter: {:?}\n     jit:         {:?}", i.stdout, j.stdout));
            }
        }
    }
    assert!(gagal.is_empty(), "{} kasus PARITAS_LLVM beda di --jit:\n{}", gagal.len(), gagal.join("\n"));
}

/// Galatnya identik dengan VM tempat kode native berjalan (termasuk jejaknya, yang kedalamannya milik
/// VM), dan sama dengan acuan pada keluaran, status dan kalimat yang dicatat tabelnya.
#[test]
fn gagal_saat_jalan_sama_di_jit() {
    let mut gagal = Vec::new();
    for (nama, src, _, memuat) in GAGAL_SAAT_JALAN {
        let i = jalankan(src, &[], &[]);
        let v = jalankan(src, &["--vm"], &[]);
        for env in MODE_PAKSA {
            let j = jalankan(src, &["--jit"], env);
            if (v.sukses, &v.stdout, &v.stderr) != (j.sukses, &j.stdout, &j.stderr)
                || (i.sukses, &i.stdout) != (j.sukses, &j.stdout) || !j.stderr.contains(memuat) {
                gagal.push(format!("  {nama} {env:?}\n     interpreter: {:?} {:?}\n     vm:          {:?}\n     jit:         {:?} {:?}",
                    i.stdout, i.stderr, v.stderr, j.stdout, j.stderr));
            }
        }
    }
    assert!(gagal.is_empty(), "{} kasus GAGAL_SAAT_JALAN beda di --jit:\n{}", gagal.len(), gagal.join("\n"));
}

/// Loop di `main` (dipanggil sekali) naik lewat OSR sambil membawa teks yang tidak pernah disentuh native.
const OSR: &str = "fn main() {\n  let label = \"jumlah\"\n  var s = 0\n  for i in 0..300000 { s = s + (i * 3) % 7 }\n  var j = 0\n  while j < 5000 { j += 1 }\n  wok(label, s, j)\n}\n";

/// `Return` bool dari VM, dan fungsi dengan parameter teks yang keluar ke VM sesudah loop-nya.
const KELUAR: &str = "fn genap(x) { return x % 2 == 0 }\nfn tempel(s, n) {\n  var t = 0\n  for i in 0..n { t = t + i }\n  return s + str(t)\n}\nfn main() {\n  var c = 0\n  for i in 0..3000 { if genap(i) { c += 1 } }\n  wok(c, tempel(\"t=\", 100))\n  wok(tempel(\"u=\", 7))\n}\n";

#[test]
fn osr_dan_titik_keluar() {
    for (src, harus, nama) in [(OSR, "jumlah 899997 5000", &["main"][..]), (KELUAR, "1500 t=4950\nu=21", &["genap", "tempel"][..])] {
        sama_dengan_interpreter(src);
        let i = jalankan(src, &[], &[]);
        assert_eq!(i.stdout, harus);
        let h = jalankan(src, &["--jit"], &[ACAK, ("JRG_JIT_LOG", "ringkas")]);
        assert_eq!(h.stdout, harus, "{}", h.stderr);
        let h = jalankan(src, &["--jit"], &[("JRG_JIT_LOG", "ringkas")]);
        assert_eq!(h.stdout, harus);
        let jadi = h.stderr.lines().find(|b| b.contains("dikompilasi:")).unwrap_or_else(|| panic!("tidak ada yang dikompilasi:\n{}", h.stderr));
        for n in nama { assert!(jadi.contains(n), "{n} tidak dikompilasi: {jadi}"); }
    }
}

/// J4 langkah 2: pecahan (NaN, tak hingga, `-0.0`, campur bulat), `/` bulat memotong, `5 / 0` lewat VM.
const PECAHAN: &str = "fn ukur(x, y) {\n  var s = 0.0\n  var i = 0\n  while i < 2000 {\n    s = s + x * 1.5 - y / 3\n    if s > 1000.0 { s = s - 999.5 }\n    i += 1\n  }\n  return s\n}\nfn bagi(a, b) { return a / b }\nfn banding(a, b) { return a < b }\nfn main() {\n  let nan = 0.0 / 0.0\n  wok(ukur(2.5, 7), ukur(1, 2.0), ukur(-0.0, 0))\n  wok(bagi(7, 2), bagi(-7, 2), bagi(7, -2), bagi(7.0, 2), bagi(1.0, 0.0), bagi(-1, 0.0))\n  wok(banding(nan, 1.0), banding(1.0, nan), nan == nan, nan != nan, banding(2, 2.5))\n  var t = 0.0\n  for i in 0..100000 { t = t + i * 0.5 }\n  wok(t, -t, 0.0 * -1.0)\n  wok(bagi(5, 0))\n}\n";

#[test]
fn pecahan_dan_bagi() {
    sama_dengan_interpreter(PECAHAN);
    let i = jalankan(PECAHAN, &[], &[]);
    let j = jalankan(PECAHAN, &["--jit"], &[STRES, ACAK]);
    assert_eq!((i.sukses, &i.stdout, &i.stderr), (j.sukses, &j.stdout, &j.stderr));
    let h = jalankan(PECAHAN, &["--jit"], &[("JRG_JIT_LOG", "ringkas")]);
    let jadi = h.stderr.lines().find(|b| b.contains("dikompilasi:")).unwrap_or_default();
    for n in ["ukur", "main"] { assert!(jadi.contains(n), "{n} tidak dikompilasi:\n{}", h.stderr); }
}

/// J4 langkah 3: larik milik VM dibaca dan ditulis lewat jalur lambat — salin-saat-tulis, elemen campur,
/// indeks negatif, dan indeks di luar batas (kalimat VM).
const LARIK: &str = "fn isi(a, n) {\n  for i in 0..n { a[i] = a[i] * 2 + i }\n  return a\n}\nfn main() {\n  var a = [0; 2000]\n  for k in 0..30 {\n    for i in 0..2000 { a[i] = a[i] + i % 13 + k }\n  }\n  wok(a[1999], a[-1], a[0], len(a))\n  var b = a\n  b[0] = 77\n  wok(a[0], b[0])\n  var c = [1, 2.5, \"x\"]\n  for i in 0..2 { c[i] = c[i] }\n  wok(c)\n  let d = isi([1, 2, 3], 3)\n  wok(d)\n  var e = [5, 6]\n  for i in 0..3 { e[i] = i }\n}\n";

#[test]
fn larik_lewat_jalur_lambat() {
    sama_dengan_interpreter(LARIK);
    let i = jalankan(LARIK, &[], &[]);
    let j = jalankan(LARIK, &["--jit"], &[STRES, ACAK]);
    assert_eq!((i.sukses, &i.stdout, &i.stderr), (j.sukses, &j.stdout, &j.stderr));
    assert_eq!(i.stdout, "735 735 435 2000\n435 77\n[1, 2.5, x]\n[2, 5, 8]");
    let h = jalankan(LARIK, &["--jit"], &[("JRG_JIT_LOG", "ringkas")]);
    assert!(h.stderr.lines().any(|b| b.contains("dikompilasi:") && b.contains("main")), "{}", h.stderr);
}
