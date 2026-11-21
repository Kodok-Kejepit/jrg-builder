// tests/bebas.rs — kernel `jrgc --bebas` (tanpa sistem operasi) boot di QEMU dan menjawab seperti mesin acuan.
// Dilewati dengan catatan kalau `qemu-system-x86_64` tidak ada; CI memasangnya.

use std::process::{Command, Stdio};

fn jrgc() -> &'static str { env!("CARGO_BIN_EXE_jrgc") }

fn ada_qemu() -> bool {
    let ada = Command::new("qemu-system-x86_64").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status()
        .map_or(false, |s| s.success());
    // Di CI QEMU dipasang: yang hilang di sana = gagal, bukan tes yang dilewati diam-diam.
    assert!(ada || std::env::var_os("JRG_QEMU_WAJIB").is_none(), "JRG_QEMU_WAJIB diset tapi qemu-system-x86_64 tidak ada");
    ada
}

struct Hasil { keluaran: String, galat: String, kode: i32 }

fn jalankan(src: &str, mode: &[&str]) -> Hasil {
    static URUT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = URUT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("jrg-uji-bebas-{}-{}", std::process::id(), n));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("kernel.jrg");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(jrgc()).arg(&f).args(mode).args(if mode.is_empty() { vec![] } else {
        vec!["-o".to_string(), dir.join("kernel.elf").display().to_string()] })
        .env("JRG_BAHASA", "id").env("JRG_SOPAN", "1").stdin(Stdio::null()).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    Hasil { keluaran: String::from_utf8_lossy(&out.stdout).into(), galat: String::from_utf8_lossy(&out.stderr).into(),
        kode: out.status.code().unwrap_or(-1) }
}

/// Program biasa: keluaran dan kode keluar kernel sama dengan interpreter.
const SAMA: &[(&str, &str)] = &[
    ("angka, teks, struct, closure, dict", r#"struct Titik { pub x: int, pub y: int }
fn fib(n: int) -> int { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
fn main() {
  wok("Halo dari JRG tanpa OS!")
  var s = 0
  for i in 0..10 { s = s + i * i }
  wok("jumlah kuadrat:", s, "fib(20) =", fib(20))
  let t = Titik { x: 3, y: 4 }
  wok(t, [1, 2, 3].map(|x| x * 10), 3.14159, 1.0 / 3.0)
  var d = {"a": 1}
  d["b"] = 2
  wok(d, str::upper("besar"), len("héllo"), "日本".chars())
}
"#),
    ("enum, match, Option, Result dan ?", r#"enum Bentuk { Kotak(int), Lingkaran(float) }
fn luas(b: Bentuk) -> float { match b { Bentuk::Kotak(s) => (s * s) as float, Bentuk::Lingkaran(r) => 3.0 * r * r } }
fn bagi(a: int, b: int) -> Result<int, string> { if b == 0 { Err("nol") } else { Ok(a / b) } }
fn dua(a: int) -> Result<int, string> { let x = bagi(a, 2)?; Ok(x * 2) }
fn main() {
  wok(luas(Bentuk::Kotak(3)), luas(Bentuk::Lingkaran(2.0)))
  wok(bagi(7, 0), dua(9), [1, 2, 3].find(|x| x > 5))
}
"#),
    ("goroutine, channel, select dan defer", r#"fn main() {
  let c = make_chan(int)
  let selesai = make_chan(bool)
  spawn { for i in 0..5 { c <- i * i }; selesai <- true }
  var jumlah = 0
  var jalan = true
  while jalan {
    select {
      case v := <-c: jumlah = jumlah + v
      case <-selesai: jalan = false
    }
  }
  defer wok("defer terakhir")
  wok("jumlah", jumlah)
}
"#),
    ("galat saat jalan berlokasi dan kode keluar", r#"fn main() {
  wok("sebelum")
  let a = [1, 2, 3]
  wok(a[5])
}
"#),
    // libm glibc statis dengan varian IFUNC yang sama dengan proses biasa: hasil pecahan sama sampai bit terakhir.
    ("fungsi matematika atas nilai saat jalan", r#"fn main() {
  let x = 0.001 * (len(env::args()) as float)
  for i in 0..400 {
    let v = x * (i as float) * 13.7 + 0.01
    wok(math::ln(v), math::exp(v / 50.0), math::sin(v), math::cos(v), math::tan(v), math::pow(v, 1.7),
      math::atan2(v, 3.0), math::sqrt(v), math::log10(v), math::asin(math::sin(v)), math::atan(v), math::hypot(v, 2.0))
  }
}
"#),
    ("exit dengan kode", "fn main() {\n  wok(\"keluar 3\")\n  exit(3)\n}\n"),
    ("stack habis di main, defer tetap jalan", r#"fn turun(n: int) -> int { return turun(n + 1) + 1 }
fn main() {
  defer wok("defer jalan")
  wok(turun(0))
}
"#),
    ("stack habis di goroutine dan di tangkap", r#"fn turun(n: int) -> int { return turun(n + 1) + 1 }
fn main() {
  match tangkap(|| turun(0)) { Err(g) => wok("tertangkap:", str::substr(g.pesan, 0, 21)), Ok(_) => wok("?") }
  let c = make_chan(int)
  spawn { c <- turun(0) }
  wok("main lanjut")
}
"#),
];

#[test]
fn kernel_bebas_menjawab_seperti_acuan() {
    if !ada_qemu() { eprintln!("dilewati: qemu-system-x86_64 tidak ada"); return; }
    let mut gagal = Vec::new();
    for (nama, src) in SAMA {
        let a = jalankan(src, &[]);
        let b = jalankan(src, &["--bebas"]);
        // Kalimat stack habis LLVM tidak menyebut fungsinya (biner tidak menghitung kedalaman); sisanya sama persis.
        let galat_sama = if nama.contains("stack habis") { b.galat.contains("rekursi terlalu dalam") }
            else { a.galat.lines().next() == b.galat.lines().next() };
        if (a.keluaran.as_str(), a.kode) != (b.keluaran.as_str(), b.kode) || !galat_sama {
            gagal.push(format!("{nama}\n  acuan: {:?} {} {:?}\n  bebas: {:?} {} {:?}", a.keluaran, a.kode,
                a.galat.lines().next(), b.keluaran, b.kode, b.galat.lines().next()));
        }
    }
    assert!(gagal.is_empty(), "kernel --bebas berbeda dari acuan:\n{}", gagal.join("\n"));
}

/// `asli::`: perangkat keras dari JRG — port, memori VGA, CPUID, MSR, interupsi timer.
#[test]
fn asli_menyentuh_perangkat_keras() {
    if !ada_qemu() { eprintln!("dilewati: qemu-system-x86_64 tidak ada"); return; }
    let src = r#"package main
var DETAK = 0
fn teks_reg(r: int) -> string {
  var s = ""
  for i in 0..4 { s = s + str(rune((r >> (i * 8)) & 255)) }
  s
}
pub fn main() {
  let (_, b, c, d) = asli::cpuid(0, 0)
  wok("vendor ada:", len(teks_reg(b) + teks_reg(d) + teks_reg(c)) == 12)
  asli::tulis16(0xB8000 + 160 * 10, 0x1F4A)
  wok("vga:", asli::baca16(0xB8000 + 160 * 10))
  wok("paging:", (asli::cr0() >> 31) & 1, "LME:", (asli::rdmsr(0xC0000080) >> 8) & 1)
  asli::interupsi(32, |v| { DETAK = DETAK + v - 31; asli::outb(0x20, 0x20) })
  for _, p in [[0x20, 0x11], [0xA0, 0x11], [0x21, 32], [0xA1, 40], [0x21, 4], [0xA1, 2], [0x21, 1], [0xA1, 1], [0x21, 0xFE], [0xA1, 0xFF]] {
    asli::outb(p[0], p[1])
  }
  asli::outb(0x43, 0x36)
  asli::outb(0x40, 11932 & 255)
  asli::outb(0x40, 11932 >> 8)
  asli::sti()
  while DETAK < 10 { asli::hlt() }
  asli::cli()
  wok("detak timer:", DETAK)
  asli::outb(0x3F8, 256)
}
"#;
    let b = jalankan(src, &["--bebas"]);
    assert_eq!(b.keluaran, "vendor ada: true\nvga: 8010\npaging: 1 LME: 1\ndetak timer: 10\n", "galat: {}", b.galat);
    assert!(b.galat.contains("`asli::outb`: nilai 256 di luar rentang 0..=255"), "{}", b.galat);
    assert_eq!(b.kode, 1);
    // Di luar kernel: ditolak dengan alasannya, di ketiga mesin.
    for mode in [&[][..], &["--vm"][..], &["--emit-llvm"][..]] {
        let h = jalankan("fn main() {\n  asli::hlt()\n}\n", mode);
        assert!(h.galat.contains("cuma ada di kernel `jrgc --bebas`"), "{:?}: {}", mode, h.galat);
    }
}

/// Pengecualian CPU dilaporkan, bukan reset diam-diam.
#[test]
fn pengecualian_cpu_dilaporkan() {
    if !ada_qemu() { eprintln!("dilewati: qemu-system-x86_64 tidak ada"); return; }
    let b = jalankan("fn main() {\n  wok(\"a\")\n  wok(asli::baca64(0x7000000000))\n}\n", &["--bebas"]);
    assert_eq!(b.keluaran, "a\n");
    assert!(b.galat.contains("pengecualian CPU #PF page fault") && b.galat.contains("alamat 0x7000000000"), "{}", b.galat);
    assert_eq!(b.kode, 1);
}
