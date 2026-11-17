//! Mesin kelima: program JRG → WebAssembly (`--emit-wasm`, `wasm32-wasip1`) lewat IR backend LLVM yang sama
//! (`CodegenOptions::ptr32`); runtime dan `primitif.c` dikompilasi ulang untuk WASI, korutin lewat `korutin_wasm.c`.
//! Alat WASI dicari di `JRG_WASI_DIR` atau `~/.cache/jrgm/wasi`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Triple Rust dan clang untuk runtime dan objek program.
pub const SASARAN: &str = "wasm32-wasip1";

/// Alat yang dibutuhkan penyusun wasm.
pub struct Alat {
    clang: PathBuf,
    /// `wasm-ld`, atau `rust-lld` yang dipanggil `-flavor wasm`.
    penaut: (PathBuf, bool),
    sysroot: PathBuf,
    builtins: PathBuf,
}

fn dir_wasi() -> PathBuf {
    if let Some(d) = std::env::var_os("JRG_WASI_DIR").filter(|d| !d.is_empty()) { return PathBuf::from(d); }
    let rumah = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    rumah.join(".cache").join("jrgm").join("wasi")
}

/// Subfolder pertama `dir` yang namanya berawalan `awal` (urut nama, yang terbaru di belakang).
fn sub_terbaru(dir: &Path, awal: &str) -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(awal)) && p.is_dir()).collect();
    v.sort();
    v.pop()
}

/// `rust-lld` milik toolchain Rust (penaut wasm yang ikut rustup), kalau `wasm-ld` tidak ada.
fn rust_lld() -> Option<PathBuf> {
    let out = Command::new("rustc").args(["--print", "sysroot"]).output().ok()?;
    let akar = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
    let rustlib = akar.join("lib").join("rustlib");
    std::fs::read_dir(&rustlib).ok()?.flatten()
        .map(|e| e.path().join("bin").join("rust-lld")).find(|p| p.exists())
}

fn di_path(nama: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|ps| std::env::split_paths(&ps).map(|d| d.join(nama)).find(|p| p.is_file()))
}

/// Mencari semua alat; galatnya menyebut apa yang kurang dan cara mendapatkannya.
pub fn cari_alat() -> Result<Alat, String> {
    let clang = crate::llvm::find_clang()?;
    let wasi = dir_wasi();
    let sysroot = sub_terbaru(&wasi, "wasi-sysroot-").filter(|d| d.join("lib").join(SASARAN).join("libc.a").exists())
        .ok_or_else(|| format!(
            "--emit-wasm butuh wasi-sysroot (wasi-sdk) di {}: unduh `wasi-sysroot-<versi>.tar.gz` dan \
             `libclang_rt-<versi>.tar.gz` dari rilis github.com/WebAssembly/wasi-sdk lalu ekstrak di sana \
             (atau set `JRG_WASI_DIR`)", wasi.display()))?;
    let builtins = sub_terbaru(&wasi.join("rt"), "libclang_rt-")
        .map(|d| d.join(format!("wasm32-unknown-{}", SASARAN.trim_start_matches("wasm32-"))).join("libclang_rt.builtins.a"))
        .filter(|p| p.exists())
        .ok_or_else(|| format!("--emit-wasm butuh compiler-rt wasm (`libclang_rt-<versi>.tar.gz` wasi-sdk) di {}",
            wasi.join("rt").display()))?;
    let penaut = di_path("wasm-ld").map(|p| (p, false))
        .or_else(|| rust_lld().map(|p| (p, true)))
        .ok_or("--emit-wasm butuh penaut wasm: `wasm-ld` (lld) di PATH, atau `rust-lld` dari rustup")?;
    Ok(Alat { clang, penaut, sysroot, builtins })
}

fn jalankan_alat(cmd: &mut Command, apa: &str) -> Result<(), String> {
    let out = cmd.output().map_err(|e| format!("gagal menjalankan {}: {}", apa, crate::host::galat_io(&e)))?;
    if out.status.success() { return Ok(()); }
    Err(format!("{} gagal:\n{}", apa, String::from_utf8_lossy(&out.stderr)))
}

/// `main` JRG di IR berbentuk `i32 main()`; wasi-libc memanggilnya lewat `__main_void`.
fn ganti_main(ir: &str) -> String {
    ir.replacen("@\"main\"()", "@\"__main_void\"()", 1)
}

/// Menyusun IR (`CodegenOptions::ptr32`) jadi `keluar` (.wasm) dan pemuatnya `keluar.mjs`.
pub fn kompilasi(ir: &str, keluar: &Path, verbose: bool) -> Result<PathBuf, String> {
    let alat = cari_alat()?;
    let runtime = crate::llvm::ensure_runtime(verbose, Some(SASARAN)).map_err(|e| {
        format!("{e}\n  bantuan: runtime wasm butuh target Rust `{SASARAN}` (`rustup target add {SASARAN}`)")
    })?;
    let dir = crate::host::dir_sementara("jrgc-wasm")
        .map_err(|e| format!("gagal membuat folder sementara: {}", crate::host::galat_io(&e)))?;
    let (ll, prog_o) = (dir.join("program.ll"), dir.join("program.o"));
    std::fs::write(&ll, ganti_main(ir)).map_err(|e| format!("gagal menulis {}: {}", ll.display(), crate::host::galat_io(&e)))?;
    let sysroot = format!("--sysroot={}", alat.sysroot.display());
    let susun = |sumber: &Path, objek: &Path| -> Result<(), String> {
        jalankan_alat(Command::new(&alat.clang).arg(format!("--target={SASARAN}")).arg(&sysroot).arg("-O2")
            .arg("-Wno-override-module").arg("-c").arg(sumber).arg("-o").arg(objek), "clang (wasm)")
    };
    susun(&ll, &prog_o)?;
    // Primitif runtime yang sama dengan `--emit-llvm` (satu definisi); korutin diganti versi WASM.
    let c = dir.join("pengantar.c");
    std::fs::write(&c, concat!(include_str!("../llvm/primitif.c"), "\n", include_str!("../llvm/jaga_stack.c"), "\n",
        include_str!("../llvm/korutin_wasm.c")))
        .map_err(|e| format!("gagal menulis {}: {}", c.display(), crate::host::galat_io(&e)))?;
    let c_o = dir.join("pengantar.o");
    susun(&c, &c_o)?;
    let lib = alat.sysroot.join("lib").join(SASARAN);
    let mut taut = Command::new(&alat.penaut.0);
    if alat.penaut.1 { taut.args(["-flavor", "wasm"]); }
    // Stack bayangan 8 MiB seperti stack utama native (bawaan wasm-ld 64 KiB: larik `[2000]int` lokal sudah meluap).
    taut.args(["-m", "wasm32", "--gc-sections", "-z", "stack-size=8388608"]).arg(format!("-L{}", lib.display())).arg(lib.join("crt1-command.o"))
        .arg(&prog_o).arg(&c_o).arg(&runtime).arg("-lc").arg(&alat.builtins).arg("-o").arg(keluar);
    let hasil = jalankan_alat(&mut taut, "penaut wasm");
    let _ = std::fs::remove_dir_all(&dir);
    hasil?;
    let pemuat = pemuat_untuk(keluar);
    std::fs::write(&pemuat, PEMUAT_NODE).map_err(|e| format!("gagal menulis {}: {}", pemuat.display(), crate::host::galat_io(&e)))?;
    if verbose { eprintln!("[jrgc] wasm: {} (pemuat {})", keluar.display(), pemuat.display()); }
    Ok(keluar.to_path_buf())
}

/// `program.wasm` → `program.wasm.mjs`.
pub fn pemuat_untuk(wasm: &Path) -> PathBuf {
    let mut n = wasm.as_os_str().to_os_string();
    n.push(".mjs");
    PathBuf::from(n)
}

/// Pemuat node: WASI bawaan node, argumen sesudah nama berkas wasm, kode keluar dari `proc_exit`. Stack V8 bawaan
/// (±1 MiB) menolak rekursi yang jalan di native, jadi `jalankan` memberi `--stack-size` (jauh di bawah stack OS).
const PEMUAT_NODE: &str = r#"// Dibuat jrgc --emit-wasm. Pakai: node --stack-size=3000 <program>.wasm.mjs [argumen...]
import fs from 'node:fs';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { WASI } from 'node:wasi';
const wasm = fileURLToPath(import.meta.url).replace(/\.mjs$/, '');
const wasi = new WASI({ version: 'preview1', args: [wasm, ...process.argv.slice(2)], env: process.env,
  preopens: { '/': '/' }, returnOnExit: true });
const mod = await WebAssembly.compile(fs.readFileSync(wasm));
const inst = await WebAssembly.instantiate(mod, { wasi_snapshot_preview1: wasi.wasiImport });
process.exitCode = wasi.start(inst);
"#;

/// Menjalankan `wasm` lewat node; kode keluarnya kode keluar program.
pub fn jalankan(wasm: &Path, args: &[String]) -> Result<i32, String> {
    let node = di_path("node").ok_or("--emit-wasm menjalankan programnya lewat `node` (WASI bawaan node 20+), \
        dan `node` tidak ada di PATH; pakai `--no-run` untuk menyusun saja")?;
    let status = Command::new(node).args(["--no-warnings", "--stack-size=3000"]).arg(pemuat_untuk(wasm)).args(args).status()
        .map_err(|e| format!("gagal menjalankan node: {}", crate::host::galat_io(&e)))?;
    if let Some(k) = status.code() { return Ok(k); }
    // Mati oleh sinyal (stack native habis: SIGSEGV): disebut, dengan 128+N seperti shell; dulu jadi kode 1 tanpa pesan.
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        let n = status.signal().unwrap_or(0);
        eprintln!("jrgc: program wasm berhenti oleh sinyal {n}{}", if n == 11 { " (SIGSEGV, kemungkinan stack habis)" } else { "" });
        Ok(128 + n)
    }
    #[cfg(not(unix))]
    Err("program wasm berhenti tanpa kode keluar".into())
}
