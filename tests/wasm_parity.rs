//! Mesin kelima (`--emit-wasm`): program JRG lewat WebAssembly (WASI, node) harus menjawab sama dengan interpreter,
//! keluaran DAN kode keluar. Alat WASI tidak ada = tes dilewati dengan alasan dicetak, KECUALI `JRG_WASI_WAJIB=1`
//! (CI), yang membuatnya gagal. Rancangan: `dokumentasi/wasm/rencana.md`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn jrgc() -> &'static str { env!("CARGO_BIN_EXE_jrgc") }

struct Hasil { kode: i32, stdout: String, stderr: String }

fn jalan(args: &[&str], dir: &Path) -> Hasil {
    let o = Command::new(jrgc()).args(args).current_dir(dir).env("JRG_SOPAN", "1").output().expect("jrgc jalan");
    Hasil {
        kode: o.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

fn folder(nama: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("jrg-wasm-uji-{}-{}", nama, std::process::id()));
    std::fs::create_dir_all(&d).expect("folder sementara");
    d
}

/// `None` = alat WASI tidak ada dan tidak diwajibkan (tes dilewati, alasannya dicetak).
fn wasm(berkas: &Path, kerja: &Path) -> Option<Hasil> {
    let keluar = kerja.join(format!("{}.wasm", berkas.file_stem().unwrap().to_string_lossy()));
    let h = jalan(&[berkas.to_str().unwrap(), "--emit-wasm", "-o", keluar.to_str().unwrap()], berkas.parent().unwrap());
    if h.kode != 0 && (h.stderr.contains("butuh wasi-sysroot") || h.stderr.contains("butuh compiler-rt wasm")
        || h.stderr.contains("rustup target add wasm32-wasip1") || h.stderr.contains("`node` tidak ada")) {
        if std::env::var_os("JRG_WASI_WAJIB").is_some() { panic!("alat --emit-wasm tidak ada, padahal JRG_WASI_WAJIB:\n{}", h.stderr); }
        eprintln!("DILEWATI: alat --emit-wasm tidak ada di mesin ini:\n{}", h.stderr);
        return None;
    }
    Some(h)
}

fn tulis(kerja: &Path, nama: &str, isi: &str) -> PathBuf {
    let p = kerja.join(nama);
    std::fs::write(&p, isi).expect("tulis program");
    p
}

#[test]
fn konformans_lewat_wasm_sama_dengan_interpreter() {
    let akar = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("conformance");
    let kerja = folder("konformans");
    let mut berkas: Vec<PathBuf> = std::fs::read_dir(&akar).unwrap().flatten().map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jrg")).collect();
    berkas.sort();
    let mut beda = Vec::new();
    for b in &berkas {
        let acuan = jalan(&[b.to_str().unwrap()], &akar);
        let Some(w) = wasm(b, &kerja) else { return };
        if (acuan.kode, &acuan.stdout) != (w.kode, &w.stdout) {
            beda.push(format!("{}: kode {} / {}\n--- interpreter\n{}\n--- wasm\n{}{}", b.display(), acuan.kode, w.kode,
                acuan.stdout, w.stdout, w.stderr));
        }
    }
    let _ = std::fs::remove_dir_all(&kerja);
    assert!(beda.is_empty(), "{} dari {} berkas konformans beda lewat --emit-wasm:\n{}", beda.len(), berkas.len(), beda.join("\n"));
}

/// Closure di larik: `%JrgFn` 8 byte di wasm32 (`CodegenOptions::ptr32`); dengan ukuran 64 bit elemen kedua dibaca di
/// offset yang salah dan panggilannya trap.
const CLOSURE_DI_LARIK: &str = r#"fn main() {
    var fs = []
    for i in 0..5 { fs = fs + [|| i * 10] }
    var t = 0
    for _, f in fs { t = t + f() }
    let g = |a, b| a + b
    var gs = [g, g]
    wok(t, fs[3](), gs[1](2, 3))
}
"#;

#[test]
fn closure_di_larik_lewat_wasm() {
    let kerja = folder("closure");
    let p = tulis(&kerja, "closure.jrg", CLOSURE_DI_LARIK);
    let acuan = jalan(&[p.to_str().unwrap()], &kerja);
    let Some(w) = wasm(&p, &kerja) else { return };
    let _ = std::fs::remove_dir_all(&kerja);
    assert_eq!((acuan.kode, acuan.stdout.as_str()), (w.kode, w.stdout.as_str()), "{}", w.stderr);
}

/// Goroutine belum didukung (`korutin_wasm.c`): ditolak saat jalan, sesudah keluaran sebelumnya, menyebut
/// `--emit-wasm` dan jalan keluarnya — tidak pernah dijawab diam-diam.
#[test]
fn goroutine_ditolak_keras_lewat_wasm() {
    let kerja = folder("goroutine");
    let p = tulis(&kerja, "spawn.jrg", "fn main() {\n    wok(\"sebelum\")\n    spawn { wok(\"di goroutine\") }\n    wok(\"sesudah\")\n}\n");
    let Some(w) = wasm(&p, &kerja) else { return };
    let _ = std::fs::remove_dir_all(&kerja);
    assert_ne!(w.kode, 0, "goroutine di wasm harus gagal:\n{}{}", w.stdout, w.stderr);
    assert_eq!(w.stdout, "sebelum\n", "keluaran sebelum penolakan harus tetap tercetak");
    assert!(w.stderr.contains("belum didukung --emit-wasm") && w.stderr.contains("--emit-llvm"), "{}", w.stderr);
}

/// Builtin lewat jembatan `jrgc` belum ditaut di wasm (`korutin_wasm.c` menggantikan simbol lemah
/// `jrg_jembatan_panggil`): galatnya menyebut builtin-nya dan lokasinya — dulu "tag nilai tidak dikenal ... bug di jrgc".
#[test]
fn builtin_jembatan_ditolak_keras_lewat_wasm() {
    let kerja = folder("jembatan");
    let p = tulis(&kerja, "jembatan.jrg", "fn main() {\n    wok(\"awal\")\n    wok(crypto::sha256(\"abc\"))\n}\n");
    let Some(w) = wasm(&p, &kerja) else { return };
    let _ = std::fs::remove_dir_all(&kerja);
    assert_ne!(w.kode, 0, "{}{}", w.stdout, w.stderr);
    assert_eq!(w.stdout, "awal\n");
    assert!(w.stderr.contains("builtin `crypto::sha256` belum didukung --emit-wasm") && w.stderr.contains("jembatan.jrg:3"),
        "{}", w.stderr);
}

/// Stack: larik lokal besar (stack bayangan wasm 8 MiB, bawaan `wasm-ld` 64 KiB meluap) dan rekursi 20 ribu tingkat
/// (`node --stack-size=3000`). Dibandingkan dengan jawaban tertulis, bukan interpreter: di runner CI interpreter
/// sendiri menolak 20 ribu tingkat (penjaga stack-nya), dan itu dulu terbaca sebagai kegagalan wasm.
const STACK_DALAM: &str = r#"fn dalam(n: int) -> int {
    if n == 0 { return 0 }
    return 1 + dalam(n - 1)
}
fn main() {
    var a: [2000]int = [7; 2000]
    var b: [2000]int = [7; 2000]
    b[1999] = 8
    wok(a == b, dalam(20000))
}
"#;

#[test]
fn stack_dalam_lewat_wasm() {
    let kerja = folder("stack");
    let p = tulis(&kerja, "stack.jrg", STACK_DALAM);
    let Some(w) = wasm(&p, &kerja) else { return };
    let _ = std::fs::remove_dir_all(&kerja);
    assert_eq!((w.kode, w.stdout.as_str()), (0, "false 20000\n"), "{}", w.stderr);
}
