//! `jrgc --bebas`: program JRG jadi kernel x86_64 tanpa sistem operasi (multiboot, boot lewat QEMU atau GRUB).
//! Codegen dan runtime sama dengan `--emit-llvm`; yang diganti cuma lapisan di bawah libc (`libc_mini.c`).
//! Rancangan dan batasnya: `dokumentasi/src/bebas/mod.md`.

use std::path::{Path, PathBuf};
use std::process::Command;

const BOOT: &str = include_str!("boot_x86_64.S");
const PENAUT: &str = include_str!("kernel_x86_64.ld");
const LIBC_MINI: &str = include_str!("libc_mini.c");

/// Semua bagian disusun untuk x86_64 Linux (ABI dan header sama), lalu ditautkan tanpa pustaka sistem.
const SASARAN: &str = "x86_64-unknown-linux-gnu";
const BENDERA: &[&str] = &["-O2", "-fno-pic", "-fno-pie", "-mno-red-zone", "-fno-stack-protector",
    "-D_FORTIFY_SOURCE=0", "-fno-asynchronous-unwind-tables", "-Wno-override-module"];

fn tulis(p: &Path, isi: &str) -> Result<(), String> {
    std::fs::write(p, isi).map_err(|e| format!("gagal menulis {}: {}", p.display(), crate::host::galat_io(&e)))
}

fn jalankan_alat(cmd: &mut Command, apa: &str) -> Result<String, String> {
    let out = cmd.output().map_err(|e| format!("gagal menjalankan {}: {}", apa, crate::host::galat_io(&e)))?;
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() { return Err(err); }
    Ok(err)
}

/// Pustaka pembantu kompilator (`__udivti3`, `__addtf3`, …) yang dipakai runtime Rust.
fn pustaka_pembantu(clang: &Path) -> Result<PathBuf, String> {
    for rt in [None, Some("--rtlib=compiler-rt")] {
        let mut c = Command::new(clang);
        c.arg(format!("--target={SASARAN}"));
        if let Some(r) = rt { c.arg(r); }
        if let Ok(o) = c.arg("-print-libgcc-file-name").output() {
            let p = PathBuf::from(String::from_utf8_lossy(&o.stdout).trim());
            if p.is_file() { return Ok(p); }
        }
    }
    Err("mode --bebas butuh pustaka pembantu kompilator (libgcc.a atau compiler-rt builtins) untuk x86_64; \
         pasang `gcc` atau `libclang-rt-dev`".into())
}

/// libm glibc statis (`libm-*.a`), supaya hasil pecahan kernel sama dengan proses di mesin ini; tanpanya fungsi
/// matematika yang dipanggil saat jalan jadi simbol hilang yang disebut namanya.
fn libm_statis() -> Option<PathBuf> {
    let dir = Path::new("/usr/lib/x86_64-linux-gnu");
    let mut calon: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("libm-") && n.ends_with(".a")))
        .collect();
    calon.sort();
    calon.pop()
}

/// Simbol yang tidak ditemukan penaut: fitur yang belum punya lapisan di mode ini, disebut namanya.
fn pesan_simbol_hilang(err: &str) -> Option<String> {
    let mut nama: Vec<String> = Vec::new();
    for baris in err.lines() {
        let Some(i) = baris.find("undefined reference to `").map(|i| i + 24)
            .or_else(|| baris.find("undefined symbol: ").map(|i| i + 18)) else { continue };
        let n = baris[i..].trim_end_matches('\'').trim().to_string();
        if !nama.contains(&n) { nama.push(n); }
    }
    if nama.is_empty() { return None; }
    Some(format!("program ini memakai bagian runtime yang belum ada di mode --bebas: {}\n  catatan: mode tanpa \
        sistem operasi belum punya berkas, proses, jaringan, atau jembatan builtin `jrgc`",
        nama.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", ")))
}

/// IR program → kernel ELF multiboot di `keluar`.
pub fn kompilasi(ir: &str, keluar: &Path, verbose: bool) -> Result<PathBuf, String> {
    // Pintu titipan (`::tampil`, `::typeof`, …) disebut IR tapi cuma terjangkau lewat nilai dari jembatan; yang
    // menentukan adalah panggilan jembatan sungguhan, yang ditandai codegen.
    if crate::llvm::PAKAI_JEMBATAN.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("program ini memanggil builtin lewat jembatan `jrgc` (mis. `dok::`, `crypto::`, `re::`, `http::`, \
            `json::` atas struct), dan jembatan itu butuh sistem operasi — belum ada di mode --bebas".into());
    }
    if !cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        return Err("mode --bebas sementara cuma dibangun dari Linux x86_64 (runtime-nya disusun untuk ABI itu)".into());
    }
    let clang = crate::llvm::find_clang()?;
    let runtime = crate::llvm::ensure_runtime(verbose, None)?;
    let pembantu = pustaka_pembantu(&clang)?;
    let dir = crate::host::dir_sementara("jrgc-bebas")
        .map_err(|e| format!("gagal membuat folder sementara: {}", crate::host::galat_io(&e)))?;
    let hasil = (|| {
        let (ll, boot, ld, libc, rt) = (dir.join("program.ll"), dir.join("boot.S"), dir.join("kernel.ld"),
            dir.join("libc_mini.c"), dir.join("runtime.c"));
        tulis(&ll, ir)?;
        tulis(&boot, BOOT)?;
        tulis(&ld, PENAUT)?;
        tulis(&libc, LIBC_MINI)?;
        tulis(&rt, concat!(include_str!("../llvm/primitif.c"), "\n", include_str!("../llvm/korutin.c"), "\n",
            include_str!("../llvm/jaga_stack.c")))?;
        let susun = |sumber: &Path, tambahan: &[&str]| -> Result<PathBuf, String> {
            let o = sumber.with_extension("o");
            jalankan_alat(Command::new(&clang).arg(format!("--target={SASARAN}")).args(BENDERA).args(tambahan)
                .arg("-c").arg(sumber).arg("-o").arg(&o), "clang (--bebas)")
                .map_err(|e| format!("clang gagal menyusun {} untuk --bebas:\n{}", sumber.display(), e))?;
            Ok(o)
        };
        let objek = [susun(&boot, &[])?, susun(&libc, &["-ffreestanding"])?, susun(&rt, &["-DJRG_BEBAS", "-w"])?,
            susun(&ll, &[])?];
        let mut taut = Command::new(&clang);
        taut.arg(format!("--target={SASARAN}")).args(["-nostdlib", "-static", "-Wl,-z,noexecstack",
            "-Wl,--no-warn-rwx-segments"]).arg(format!("-Wl,-T,{}", ld.display())).args(&objek)
            .arg(&runtime).args(libm_statis()).arg(&pembantu).arg("-o").arg(keluar);
        if verbose { eprintln!("[jrgc] --bebas: menautkan kernel {}", keluar.display()); }
        jalankan_alat(&mut taut, "penaut (--bebas)")
            .map_err(|e| pesan_simbol_hilang(&e).unwrap_or_else(|| format!("penaut gagal menyusun kernel:\n{}", e)))?;
        Ok(keluar.to_path_buf())
    })();
    let _ = std::fs::remove_dir_all(&dir);
    hasil
}

fn cari_qemu() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("JRG_QEMU") { return Some(PathBuf::from(p)); }
    std::env::var_os("PATH").and_then(|path| std::env::split_paths(&path)
        .map(|d| d.join("qemu-system-x86_64")).find(|p| p.is_file()))
}

/// Menjalankan kernel di QEMU: COM1 ke stdout, COM2 ke stderr, kode keluar lewat `isa-debug-exit`.
/// `layar`: jendela VGA QEMU ikut dibuka (bawaan tanpa jendela).
pub fn jalankan(kernel: &Path, layar: bool) -> Result<i32, String> {
    let qemu = cari_qemu().ok_or("--bebas menjalankan kernelnya di `qemu-system-x86_64`, dan itu tidak ada di \
        PATH (atau `JRG_QEMU`); pakai `--no-run -o kernel.elf` untuk menyusun saja, lalu boot lewat GRUB")?;
    let mut c = Command::new(qemu);
    c.arg("-kernel").arg(kernel).args(["-monitor", "none", "-serial", "stdio", "-chardev", "file,id=galat,path=/dev/stderr,append=on", "-serial", "chardev:galat",
        "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-no-reboot", "-m", "512",
        "-cpu", "max"]);
    // KVM kalau boleh dibuka (cepat); tanpa itu TCG, tanpa peringatan QEMU yang mengotori stderr program.
    let kvm = std::fs::OpenOptions::new().read(true).write(true).open("/dev/kvm").is_ok();
    c.args(["-machine", if kvm { "accel=kvm" } else { "accel=tcg" }]);
    if !layar { c.args(["-display", "none"]); }
    // Baris perintah multiboot = lingkungan kernel (`getenv` di `libc_mini.c`): bahasa pesan mengikuti jalan ini.
    c.arg("-append").arg(format!("JRG_BAHASA={}", crate::terjemah::aktif().kode()));
    // QEMU membuat stdin/stdout `stdio`-nya non-blocking; kalau itu milik bersama dengan pemanggil, tulisan pemanggil
    // sesudahnya gagal diam-diam. Jadi lewat pipa sendiri, disalin dua utas.
    use std::io::{Read, Write};
    c.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
    let mut anak = c.spawn().map_err(|e| format!("gagal menjalankan QEMU: {}", crate::host::galat_io(&e)))?;
    let mut masuk = anak.stdin.take();
    std::thread::spawn(move || {
        let mut b = [0u8; 4096];
        let mut stdin = std::io::stdin();
        while let (Ok(n), Some(m)) = (stdin.read(&mut b), masuk.as_mut()) {
            if n == 0 || m.write_all(&b[..n]).is_err() { break; }
        }
        drop(masuk);
    });
    let mut keluaran = anak.stdout.take().expect("stdout QEMU dipipakan");
    let salin = std::thread::spawn(move || {
        let mut b = [0u8; 4096];
        let mut out = std::io::stdout();
        while let Ok(n) = keluaran.read(&mut b) {
            if n == 0 || out.write_all(&b[..n]).and_then(|_| out.flush()).is_err() { break; }
        }
    });
    let status = anak.wait().map_err(|e| format!("gagal menunggu QEMU: {}", crate::host::galat_io(&e)))?;
    let _ = salin.join();
    match status.code() {
        // `isa-debug-exit` menulis (kode << 1) | 1.
        Some(k) if k % 2 == 1 => Ok(k >> 1),
        Some(0) => {
            eprintln!("jrgc: kernel berhenti tanpa memanggil keluar (CPU di-reset — biasanya triple fault)");
            Ok(1)
        }
        Some(k) => Err(format!("QEMU berhenti dengan kode {k} sebelum kernel jalan")),
        None => Err("QEMU berhenti oleh sinyal".into()),
    }
}
