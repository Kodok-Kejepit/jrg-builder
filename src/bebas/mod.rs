//! `jrgc --bebas`: program JRG jadi kernel x86_64 tanpa sistem operasi (multiboot, boot lewat QEMU atau GRUB).
//! Codegen dan runtime sama dengan `--emit-llvm`; yang diganti cuma lapisan di bawah libc (`libc_mini.c`).
//! Rancangan dan batasnya: `dokumentasi/src/bebas/mod.md`.

pub mod cakram;

use std::path::{Path, PathBuf};
use std::process::Command;

const BOOT: &str = include_str!("boot_x86_64.S");
const PENAUT: &str = include_str!("kernel_x86_64.ld");
const LIBC_MINI: &str = concat!(include_str!("fon_8x16.h"), include_str!("libc_mini.c"));
const BERKAS_RAM: &str = include_str!("berkas_ram.c");

/// Isi folder `--berkas` sebagai tabel `jrg_berkas_awal` (`berkas_ram.c`): data lewat `.incbin`, tanpa disalin
/// jadi teks; jalurnya relatif terhadap folder itu, berawalan `/`.
fn tulis_berkas_awal(akar: &Path, keluar: &Path) -> Result<(), String> {
    fn jalan(d: &Path, akar: &Path, daftar: &mut Vec<(String, Option<PathBuf>)>) -> Result<(), String> {
        let mut isi: Vec<_> = std::fs::read_dir(d).map_err(|e| format!("--berkas: tidak bisa membaca {} ({})",
            d.display(), crate::host::galat_io(&e)))?.flatten().map(|e| e.path()).collect();
        isi.sort();
        for p in isi {
            let rel = format!("/{}", p.strip_prefix(akar).unwrap_or(&p).to_string_lossy());
            if p.is_dir() { daftar.push((rel, None)); jalan(&p, akar, daftar)?; }
            else if p.is_file() { daftar.push((rel, Some(p))); }
        }
        Ok(())
    }
    let mut daftar = Vec::new();
    jalan(akar, akar, &mut daftar)?;
    let kutip = |s: &str| s.bytes().map(|b| format!("{}", b)).collect::<Vec<_>>().join(",");
    let mut s = String::from(".section .rodata\n.p2align 3\n");
    for (i, (jalur, berkas)) in daftar.iter().enumerate() {
        s += &format!("jalur_{i}: .byte {},0\n", kutip(jalur));
        if let Some(b) = berkas {
            let abs = std::fs::canonicalize(b).map_err(|e| format!("--berkas: tidak bisa membaca {} ({})", b.display(), crate::host::galat_io(&e)))?;
            s += &format!(".p2align 3\nisi_{i}: .incbin \"{}\"\nisi_{i}_akhir:\n", abs.display().to_string().replace('\\', "\\\\").replace('"', "\\\""));
        }
    }
    s += ".p2align 3\n.globl jrg_berkas_awal\njrg_berkas_awal:\n";
    for (i, (_, berkas)) in daftar.iter().enumerate() {
        match berkas {
            Some(_) => s += &format!(".quad jalur_{i}, isi_{i}, isi_{i}_akhir - isi_{i}, 0\n"),
            None => s += &format!(".quad jalur_{i}, 0, 0, 1\n"),
        }
    }
    s += &format!(".globl jrg_berkas_awal_n\njrg_berkas_awal_n: .quad {}\n", daftar.len());
    tulis(keluar, &s)
}

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
pub fn kompilasi(ir: &str, keluar: &Path, berkas: Option<&Path>, verbose: bool) -> Result<PathBuf, String> {
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
        let ram = dir.join("berkas_ram.c");
        tulis(&ram, BERKAS_RAM)?;
        tulis(&rt, concat!(include_str!("../llvm/primitif.c"), "\n", include_str!("../llvm/korutin.c"), "\n",
            include_str!("../llvm/jaga_stack.c")))?;
        let susun = |sumber: &Path, tambahan: &[&str]| -> Result<PathBuf, String> {
            let o = sumber.with_extension("o");
            jalankan_alat(Command::new(&clang).arg(format!("--target={SASARAN}")).args(BENDERA).args(tambahan)
                .arg("-c").arg(sumber).arg("-o").arg(&o), "clang (--bebas)")
                .map_err(|e| format!("clang gagal menyusun {} untuk --bebas:\n{}", sumber.display(), e))?;
            Ok(o)
        };
        let mut objek = vec![susun(&boot, &[])?, susun(&libc, &["-ffreestanding"])?, susun(&ram, &[])?,
            susun(&rt, &["-DJRG_BEBAS", "-w"])?, susun(&ll, &[])?];
        if let Some(akar) = berkas {
            let s = dir.join("berkas_awal.S");
            tulis_berkas_awal(akar, &s)?;
            objek.push(susun(&s, &[])?);
        }
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
        "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-no-reboot", "-m", "2048",
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
            // Akhir masukan diteruskan sebagai 0x04: serial tidak punya EOF, `read` di kernel membacanya begitu.
            if n == 0 { let _ = m.write_all(&[4]); break; }
            if m.write_all(&b[..n]).is_err() { break; }
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

fn di_path(nama: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| std::env::split_paths(&path).map(|d| d.join(nama)).find(|p| p.is_file()))
}

/// Kernel → image disk UEFI (`--img`): GRUB EFI mandiri (`grub-mkstandalone`, kernel dan `grub.cfg` di memdisknya)
/// sebagai `EFI/BOOT/BOOTX64.EFI` di partisi Sistem EFI. Ditulis ke flashdisk (`dd`), boot di mesin UEFI.
pub fn buat_img(kernel: &Path, img: &Path, verbose: bool) -> Result<(), String> {
    let grub = di_path("grub-mkstandalone").ok_or("--img butuh `grub-mkstandalone` (paket grub-efi / grub2-common) \
        dan modul GRUB `x86_64-efi`")?;
    if !Path::new("/usr/lib/grub/x86_64-efi").is_dir() {
        return Err("--img butuh modul GRUB `x86_64-efi` (/usr/lib/grub/x86_64-efi, paket grub-efi-amd64-bin)".into());
    }
    let dir = crate::host::dir_sementara("jrgc-img")
        .map_err(|e| format!("gagal membuat folder sementara: {}", crate::host::galat_io(&e)))?;
    let hasil = (|| {
        let (cfg, efi) = (dir.join("grub.cfg"), dir.join("BOOTX64.EFI"));
        tulis(&cfg, &format!("set timeout=0\nset default=0\ninsmod all_video\nset gfxpayload=keep\nmenuentry \"JRG\" {{\n  \
            multiboot /boot/kernel.elf JRG_BAHASA={}\n  boot\n}}\n", crate::terjemah::aktif().kode()))?;
        // Salinan kernel yang meminta framebuffer (bit 2 kepala multiboot): mesin UEFI tidak punya layar teks VGA.
        // Kernel biasa tidak memintanya, karena loader multiboot QEMU (`-kernel`) menolak permintaan itu dengan keluhan.
        let mut elf = std::fs::read(kernel).map_err(|e| format!("gagal membaca {}: {}", kernel.display(), crate::host::galat_io(&e)))?;
        let kepala = elf.windows(4).take(8192).position(|w| w == 0x1BADB002u32.to_le_bytes())
            .ok_or("kepala multiboot tidak ditemukan di kernel")?;
        let bendera = u32::from_le_bytes(elf[kepala + 4..kepala + 8].try_into().unwrap()) | 4;
        elf[kepala + 4..kepala + 8].copy_from_slice(&bendera.to_le_bytes());
        elf[kepala + 8..kepala + 12].copy_from_slice(&0u32.wrapping_sub(0x1BADB002u32.wrapping_add(bendera)).to_le_bytes());
        let kernel = dir.join("kernel.elf");
        std::fs::write(&kernel, &elf).map_err(|e| format!("gagal menulis {}: {}", kernel.display(), crate::host::galat_io(&e)))?;
        let mut c = Command::new(grub);
        c.args(["-O", "x86_64-efi", "--modules=multiboot normal configfile memdisk tar all_video efi_gop video_fb",
            "--locales=", "--fonts=",
            "--themes="]).arg("-o").arg(&efi)
            .arg(format!("boot/grub/grub.cfg={}", cfg.display())).arg(format!("boot/kernel.elf={}", kernel.display()));
        if verbose { eprintln!("[jrgc] --img: grub-mkstandalone"); }
        jalankan_alat(&mut c, "grub-mkstandalone").map_err(|e| format!("grub-mkstandalone gagal:\n{}", e))?;
        let isi = std::fs::read(&efi).map_err(|e| format!("gagal membaca {}: {}", efi.display(), crate::host::galat_io(&e)))?;
        std::fs::write(img, cakram::buat(&isi))
            .map_err(|e| format!("gagal menulis {}: {}", img.display(), crate::host::galat_io(&e)))
    })();
    let _ = std::fs::remove_dir_all(&dir);
    hasil
}

/// Firmware UEFI QEMU (OVMF) untuk menguji image `--img`.
fn cari_ovmf() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("JRG_OVMF") { return Some(PathBuf::from(p)); }
    ["/usr/share/ovmf/OVMF.fd", "/usr/share/OVMF/OVMF.fd", "/usr/share/qemu/OVMF.fd"].iter().map(PathBuf::from).find(|p| p.is_file())
}

/// Boot image `--img` di QEMU lewat OVMF, seperti mesin UEFI sungguhan; keluaran firmware dan GRUB ikut tampil.
pub fn jalankan_img(img: &Path) -> Result<i32, String> {
    let qemu = cari_qemu().ok_or("--img menguji image-nya di `qemu-system-x86_64`, dan itu tidak ada di PATH")?;
    let ovmf = cari_ovmf().ok_or("--img menguji image-nya lewat firmware UEFI OVMF (paket ovmf, atau `JRG_OVMF`); \
        pakai `--no-run` untuk menyusun saja")?;
    let kvm = std::fs::OpenOptions::new().read(true).write(true).open("/dev/kvm").is_ok();
    let status = Command::new(qemu).arg("-bios").arg(ovmf).arg("-drive").arg(format!("file={},format=raw", img.display()))
        .args(["-display", "none", "-monitor", "none", "-serial", "stdio", "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04",
            "-no-reboot", "-m", "512", "-cpu", "max", "-machine", if kvm { "accel=kvm" } else { "accel=tcg" }])
        .stdin(std::process::Stdio::null()).status()
        .map_err(|e| format!("gagal menjalankan QEMU: {}", crate::host::galat_io(&e)))?;
    match status.code() {
        Some(k) if k % 2 == 1 => Ok(k >> 1),
        _ => Err("image tidak sampai menjalankan kernel (firmware atau GRUB berhenti lebih dulu)".into()),
    }
}
