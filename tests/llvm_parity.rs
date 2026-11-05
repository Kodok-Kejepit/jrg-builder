// tests/llvm_parity.rs

mod bersama;
use std::process::Command;

/// Tiap biner `--emit-llvm` dari tes ini berbatas `JRGC_BATAS_MEMORI` (bawaan 2048 MiB; `=` kosong mematikannya),
/// supaya kasus yang rakus gagal "memori habis" alih-alih menghabiskan memori mesin.
fn jrgc() -> &'static str {
    static PASANG: std::sync::Once = std::sync::Once::new();
    PASANG.call_once(|| if std::env::var_os("JRGC_BATAS_MEMORI").is_none() { std::env::set_var("JRGC_BATAS_MEMORI", "2048") });
    env!("CARGO_BIN_EXE_jrgc")
}

/// Apakah `clang` tersedia? Backend LLVM memanggilnya untuk menghasilkan
/// native code, jadi tanpanya tidak ada yang bisa diukur.
fn ada_clang() -> bool {
    let nama = std::env::var("JRGC_CLANG").unwrap_or_else(|_| "clang".to_string());
    Command::new(&nama).arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

struct Hasil {
    keluaran: String,
    galat: String,
    sukses: bool,
    /// Kode keluar apa adanya; lewat LLVM itu kode keluar biner hasil kompilasinya.
    kode: Option<i32>,
}

/// Direktori sementara unik per pemanggilan — tes berjalan paralel.
fn dir_uji(tag: &str) -> std::path::PathBuf {
    let d = crate::bersama::dir_unik(&format!("jrg-llvm-{}", tag));
    std::fs::create_dir_all(&d).expect("bikin dir sementara");
    d
}

fn jalankan_interp(src: &str) -> Hasil {
    let dir = dir_uji("interp");
    let file = dir.join("uji.jrg");
    std::fs::write(&file, src).expect("tulis sumber");
    let out = Command::new(jrgc()).arg(&file).env("JRG_SOPAN", "1")
        .env_remove("DISPLAY").env_remove("WAYLAND_DISPLAY")
        .output().expect("jalankan jrgc");
    let _ = std::fs::remove_dir_all(&dir);
    Hasil {
        keluaran: String::from_utf8_lossy(&out.stdout).trim().to_string(),
        galat: String::from_utf8_lossy(&out.stderr).to_string(),
        sukses: out.status.success(),
        kode: out.status.code(),
    }
}

/// Kompilasi lewat `--emit-llvm` lalu jalankan binernya.
fn jalankan_llvm(src: &str) -> Hasil {
    let dir = dir_uji("emit");
    let file = dir.join("uji.jrg");
    std::fs::write(&file, src).expect("tulis sumber");
    let out = Command::new(jrgc())
        .arg(&file).arg("--emit-llvm").arg("-o").arg(dir.join("uji.bin"))
        .env("JRG_SOPAN", "1").env_remove("DISPLAY").env_remove("WAYLAND_DISPLAY")
        .output().expect("jalankan jrgc --emit-llvm");
    let _ = std::fs::remove_dir_all(&dir);
    Hasil {
        keluaran: String::from_utf8_lossy(&out.stdout).trim().to_string(),
        galat: String::from_utf8_lossy(&out.stderr).to_string(),
        sukses: out.status.success(),
        kode: out.status.code(),
    }
}

/// Jalankan lewat `--vm`. Hanya dipakai sapuan tiga-mesin di bawah.
fn jalankan_vm(src: &str) -> Hasil {
    let dir = dir_uji("vm");
    let file = dir.join("uji.jrg");
    std::fs::write(&file, src).expect("tulis sumber");
    let out = Command::new(jrgc()).arg(&file).arg("--vm").env("JRG_SOPAN", "1")
        .output().expect("jalankan jrgc --vm");
    let _ = std::fs::remove_dir_all(&dir);
    Hasil {
        keluaran: String::from_utf8_lossy(&out.stdout).trim().to_string(),
        galat: String::from_utf8_lossy(&out.stderr).to_string(),
        sukses: out.status.success(),
        kode: out.status.code(),
    }
}

/// Nama-nama dari bahasa lain, **dibaca dari sumbernya** alih-alih ditulis
/// ulang.
fn nama_bahasa_lain() -> Vec<String> {
    let src = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/interpreter/value.rs"))
        .expect("baca value.rs");
    let mulai = src.find("pub fn padanan_bahasa_lain").expect("fungsi padanan ada");
    let badan = &src[mulai..];
    let habis = badan.find("\n}").expect("akhir fungsi");
    let badan = &badan[..habis];

    let mut nama = Vec::new();
    for baris in badan.lines() {
        // Tiap lengan berbentuk `"a" | "b" => "penjelasan",` — yang diambil cuma
        // sisi KIRI `=>`, jadi teks penjelasannya tidak ikut terpanen.
        let Some(kiri) = baris.split("=>").next() else { continue };
        if !baris.contains("=>") { continue; }
        let mut sisa = kiri;
        while let Some(a) = sisa.find('"') {
            let sisa2 = &sisa[a + 1..];
            let Some(b) = sisa2.find('"') else { break };
            nama.push(sisa2[..b].to_string());
            sisa = &sisa2[b + 1..];
        }
    }
    nama
}

/// **Sebuah kata dari bahasa lain dijawab kalimat yang SAMA di tiga mesin.**
/// **Bulat lawan pecahan menjawab sama di tiga mesin** (Fase LLVM-16): tiap …
#[test]
fn bulat_lawan_pecahan_sama_di_tiga_mesin() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let operand = ["3", "2.5", "3.0", "bi", "pf", "fb()", "ff()"];
    let operator = ["+", "-", "*", "/", "%", "**", "==", "!=", "<", "<=", ">", ">="];
    let mut badan = String::new();
    for op in operator { for a in operand { for b in operand {
        badan.push_str(&format!("    wok(\"{} {} {} =\", {} {} {})\n", a, op, b, a, op, b));
    } } }
    let src = format!("package main\nfn fb() -> int {{ return 3 }}\nfn ff() -> float {{ return 2.5 }}\n\
                       pub fn main() {{\n    let bi = 3\n    let pf = 2.5\n{}}}\n", badan);
    let (i, v, l) = (jalankan_interp(&src), jalankan_vm(&src), jalankan_llvm(&src));
    assert!(i.sukses, "acuan gagal:\n{}", i.galat);
    assert_eq!(i.keluaran, v.keluaran, "acuan dan --vm beda");
    assert!(l.sukses, "LLVM menolak:\n{}", l.galat);
    let beda: Vec<String> = i.keluaran.lines().zip(l.keluaran.lines())
        .filter(|(a, b)| a != b).map(|(a, b)| format!("  acuan {}  |  llvm {}", a, b)).collect();
    assert!(beda.is_empty(), "{} baris beda:\n{}", beda.len(), beda.join("\n"));
    assert_eq!(i.keluaran.lines().count(), operand.len() * operand.len() * operator.len());
}

/// `zip::` (Fase AMAN-15) sama di tiga mesin: buat, daftar, baca, bongkar; bongkar ulang tidak menimpa,
/// dan arsip zip-slip buatan `zipfile` Python ditolak tanpa satu berkas pun tertulis — juga yang sah di dalamnya.
#[test]
fn zip_sama_di_tiga_mesin_dan_tidak_menulis_ke_luar() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    const SLIP: &str = "UEsDBBQAAAAAAAAAIQBH3dx5AgAAAAIAAAAIAAAAYW1hbi50eHRva1BLAwQUAAAAAAAAACEAzH1w+wUAAAAFAAAAEAAAAC4uLy4uL2tlbHVhci50eHRqYWhhdFBLAQIUAxQAAAAAAAAAIQBH3dx5AgAAAAIAAAAIAAAAAAAAAAAAAACkgQAAAABhbWFuLnR4dFBLAQIUAxQAAAAAAAAAIQDMfXD7BQAAAAUAAAAQAAAAAAAAAAAAAACkgSgAAAAuLi8uLi9rZWx1YXIudHh0UEsFBgAAAAACAAIAdAAAAFsAAAAAAA==";
    let jalan = |mesin: &str, f: fn(&str) -> Hasil| {
        let d = dir_uji(&format!("zip-{}", mesin));
        let src = format!(r#"package main
pub fn main() {{
    let d = "{d}"
    wok(zip::create(d + "/a.zip", {{"baca.md": "halo\n" + str::repeat("ulang ", 50), "data/": "", "data/b.bin": [0, 1, 255]}}))
    for _, e in zip::list(d + "/a.zip")? {{ wok(e.nama, e.ukuran, e.folder, e.mampat <= e.ukuran) }}
    wok(zip::read(d + "/a.zip", "data/b.bin"), str::len(zip::read_text(d + "/a.zip", "baca.md")?))
    wok(zip::read_text(d + "/a.zip", "data/b.bin"))
    wok(zip::extract(d + "/a.zip", d + "/k"), fs::read(d + "/k/baca.md") == zip::read_text(d + "/a.zip", "baca.md")?)
    wok(zip::extract(d + "/a.zip", d + "/k"))
    wok(zip::create(d + "/b.zip", {{"../luar": "x"}}))
    fs::write_base64(d + "/slip.zip", "{SLIP}")?
    wok(zip::extract(d + "/slip.zip", d + "/s"), fs::exists(d + "/s/aman.txt"))
}}
"#, d = d.display());
        let h = f(&src);
        let tak_tertulis = !d.join("../keluar.txt").exists() && !d.join("keluar.txt").exists();
        let _ = std::fs::remove_dir_all(&d);
        (h.sukses, h.galat, h.keluaran.replace(&d.display().to_string(), "D"), tak_tertulis)
    };
    let (i, v, l) = (jalan("i", jalankan_interp), jalan("v", jalankan_vm), jalan("l", jalankan_llvm));
    assert!(i.0, "acuan gagal:\n{}", i.1);
    for kata in ["Result::Ok(3)", "data/b.bin 3 false true", "[0, 1, 255]", "bukan teks UTF-8",
                 "`baca.md`: sudah ada (0 berkas", "zip-slip", "Result::Ok(2) true"] {
        assert!(i.2.contains(kata), "keluaran acuan tidak memuat `{}`:\n{}", kata, i.2);
    }
    assert!(i.2.contains("../../keluar.txt") && i.2.contains("false"), "{}", i.2);
    assert_eq!(i.2, v.2, "acuan dan --vm beda");
    assert!(l.0, "LLVM gagal:\n{}", l.1);
    assert_eq!(i.2, l.2, "acuan dan LLVM beda");
    assert!(i.3 && v.3 && l.3, "arsip zip-slip menulis ke luar foldernya");
}

/// `tar::` (Fase AMAN-22) sama di tiga mesin: buat `.tar.gz`, daftar, baca, bongkar; arsip tar-slip dan arsip
/// bertautan simbolik buatan `tarfile` Python ditolak tanpa satu berkas pun tertulis; `tar` sistem membaca buatan kita.
#[test]
fn tar_sama_di_tiga_mesin_dan_tidak_menulis_ke_luar() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    const SLIP: &str = "H4sIAAAAAAAC/+3UPQrDMAyGYR2lJ3Cc4LTn0RAodX/AlaHHr5Kp7dIphpD3QSChRdMnvek92MtkRdEdU1q6++1u+Jjn/Sn1gxyiNFCfpsVPyj49smDHQui88nStWtb6A//zP37nv4/kv5GLntWIAQAAAAAAAAAAAAAAwGa9AeFNbOwAKAAA";
    const SYM: &str = "H4sIAAAAAAAC/+3RMQrCQBCG0TlKTqBxWdzzLGIhSArdQI7vYhlso0jea/7ppvhanVudYlNjd875vd16P9ylpBJDOl7bJTY2P1t99JexT/d6mw5tab/tf1r3z2mM4StNdt5/CQAAAAAAAAAA4F+9AAlcC88AKAAA";
    let jalan = |mesin: &str, f: fn(&str) -> Hasil| {
        let d = dir_uji(&format!("tar-{}", mesin));
        let src = format!(r#"package main
pub fn main() {{
    let d = "{d}"
    let a = d + "/a.tar.gz"
    wok(tar::create(a, {{"baca.md": "halo\n" + str::repeat("ulang ", 50), "data/": "", "data/b.bin": [0, 1, 255]}}))
    for _, e in tar::list(a)? {{ wok(e.nama, e.ukuran, e.folder, e.jenis) }}
    wok(tar::read(a, "data/b.bin"), str::len(tar::read_text(a, "baca.md")?))
    wok(tar::read_text(a, "data/b.bin"))
    wok(tar::extract(a, d + "/k"), fs::read(d + "/k/baca.md") == tar::read_text(a, "baca.md")?)
    wok(tar::extract(a, d + "/k"))
    wok(tar::create(d + "/b.tar", {{"../luar": "x"}}))
    fs::write_base64(d + "/slip.tgz", "{SLIP}")?
    wok(tar::extract(d + "/slip.tgz", d + "/s"), fs::exists(d + "/s/aman.txt"))
    fs::write_base64(d + "/sym.tgz", "{SYM}")?
    for _, e in tar::list(d + "/sym.tgz")? {{ wok(e) }}
    wok(tar::extract(d + "/sym.tgz", d + "/t"), fs::exists(d + "/t/lain.txt"))
}}
"#, d = d.display());
        let h = f(&src);
        let tak_tertulis = !d.join("../keluar.txt").exists() && !d.join("keluar.txt").exists();
        // `tar` sistem membaca arsip buatan kita dengan isi yang sama.
        let sistem = std::fs::create_dir_all(d.join("x")).is_ok() && Command::new("tar")
            .args(["-xzf", "a.tar.gz", "-C", "x"]).current_dir(&d).status().map_or(true, |s| s.success()
                && std::fs::read(d.join("x/data/b.bin")).ok() == Some(vec![0, 1, 255]));
        let _ = std::fs::remove_dir_all(&d);
        (h.sukses, h.galat, h.keluaran.replace(&d.display().to_string(), "D"), tak_tertulis && sistem)
    };
    let (i, v, l) = (jalan("i", jalankan_interp), jalan("v", jalankan_vm), jalan("l", jalankan_llvm));
    assert!(i.0, "acuan gagal:\n{}", i.1);
    for kata in ["Result::Ok(3)", "data/b.bin 3 false berkas", "data/ 0 true folder", "[0, 1, 255]", "bukan teks UTF-8",
                 "`baca.md`: sudah ada (0 berkas", "Result::Ok(2) true", "../../keluar.txt", "symlink ke `/etc`",
                 "tautan: /etc"] {
        assert!(i.2.contains(kata), "keluaran acuan tidak memuat `{}`:\n{}", kata, i.2);
    }
    assert_eq!(i.2.matches(") false").count(), 2, "arsip jahat menulis sebagian:\n{}", i.2);
    assert_eq!(i.2, v.2, "acuan dan --vm beda");
    assert!(l.0, "LLVM gagal:\n{}", l.1);
    assert_eq!(i.2, l.2, "acuan dan LLVM beda");
    assert!(i.3 && v.3 && l.3, "arsip tar-slip menulis ke luar foldernya, atau `tar` sistem menolak buatan kita");
}

include!("bersama/paritas.rs");

/// Program `PARITAS` yang LLVM JALANKAN tapi menjawab lain — dicatat dengan sebabnya sampai
/// ditutup. Daftar ini swa-retraksi: entri yang sudah sama memerahkan tesnya sebagai kabar baik.
const PARITAS_BEDA_DI_LLVM: &[(&str, &str)] = &[
    // Kosong sejak Fase URUTAN-LLVM (entri terakhirnya, tulisan yang hilang di `m[0] = m[0] + g()`,
    // ditutup). Entri berikutnya: ("nama kasus PARITAS", "sebabnya").
];

/// Modul yang di-`import` jalan lewat LLVM dan menjawab sama dengan dua mesin lain (Fase
/// MODUL-LLVM): alias ganda atas satu berkas berbagi satu `var`, modul yang mengimpor modul,
/// nama sama di dua berkas, method dan `fn` bersarang milik modul; yang privat ditolak sama.
#[test]
fn modul_pemakai_sama_di_tiga_mesin() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let dir = dir_uji("modul");
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let tulis = |n: &str, isi: &str| std::fs::write(dir.join(n), isi).unwrap();
    tulis("bantu.jrg", "package bantu\nvar HITUNG = 0\nconst BATAS = 10\nlet SALAM = \"halo\"\n\
        fn _rahasia() { return 1 }\nfn naik() { HITUNG = HITUNG + 1; return HITUNG }\n\
        fn dua(x) { return x * 2 + _rahasia() }\nfn batas() { return BATAS }\n");
    tulis("sub/alat.jrg", "package alat\nimport \"bantu\"\nstruct Kotak { pub isi: int = 0 }\n\
        impl Kotak { fn dobel(self) { return self.isi * 2 } }\n\
        fn pakai(n) { let k = Kotak { isi: n }\n return k.dobel() + bantu::dua(n) }\n\
        fn kali(a, b) { fn dalam(x, y) { return x * y }\n return dalam(a, b) }\n\
        fn rusak() { let a = [1]\n return a[5] }\n");
    tulis("utama.jrg", "package main\nimport \"bantu\"\nimport \"bantu\" as b2\nimport \"sub/alat\"\n\
        fn dua(x) { return x + 1000 }\npub fn main() {\n\
        wok(bantu::dua(4), bantu::naik(), b2::naik(), bantu::batas(), bantu::BATAS, bantu::SALAM)\n\
        wok(dua(1), alat::pakai(3), alat::kali(2, 5)) }\n");
    tulis("privat.jrg", "package main\nimport \"bantu\"\npub fn main() { wok(bantu::_rahasia()) }\n");
    tulis("var.jrg", "package main\nimport \"bantu\"\npub fn main() { wok(bantu::HITUNG) }\n");
    tulis("lingkar_a.jrg", "package lingkar_a\nimport \"lingkar_b\"\nfn a() { return 1 }\n");
    tulis("lingkar_b.jrg", "package lingkar_b\nimport \"lingkar_a\"\nfn b() { return 2 }\n");
    tulis("lingkar.jrg", "package main\nimport \"lingkar_a\"\npub fn main() { wok(lingkar_a::a()) }\n");
    let jalan = |berkas: &str, mode: &[&str]| {
        let out = Command::new(jrgc()).arg(dir.join(berkas)).args(mode)
            .env("JRG_SOPAN", "1").current_dir(&dir).output().expect("jalankan jrgc");
        (String::from_utf8_lossy(&out.stdout).trim().to_string(),
         String::from_utf8_lossy(&out.stderr).to_string(), out.status.success())
    };
    let bin = dir.join("uji.bin");
    let llvm: Vec<&str> = vec!["--emit-llvm", "-o", bin.to_str().unwrap()];
    let mut masalah = Vec::new();
    let (i, _, oki) = jalan("utama.jrg", &[]);
    let (v, _, _) = jalan("utama.jrg", &["--vm"]);
    let (l, galat, okl) = jalan("utama.jrg", &llvm);
    if !oki || i != "9 1 2 10 10 halo\n1001 13 10" { masalah.push(format!("interpreter: {:?}", i)); }
    if v != i { masalah.push(format!("--vm beda: {:?}", v)); }
    if !okl || l != i { masalah.push(format!("llvm beda: {:?}\n{}", l, galat)); }
    for (berkas, kunci) in [("privat.jrg", "privat milik modul `bantu`"),
                            ("var.jrg", "adalah `var` milik modul `bantu`"),
                            ("lingkar.jrg", "lingkar_a")] {
        for mode in [&[][..], &llvm[..]] {
            let (_, g, ok) = jalan(berkas, mode);
            if ok || !g.contains(kunci) {
                masalah.push(format!("{} ({}): harus ditolak memuat {:?}, dapat: {}", berkas,
                    if mode.is_empty() { "interpreter" } else { "llvm" }, kunci, g.lines().next().unwrap_or("")));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(masalah.is_empty(), "{} masalah modul lewat LLVM:\n{}", masalah.len(), masalah.join("\n"));
}

/// Nama tipe dan trait milik berkasnya di TIGA mesin (Fase TRAIT-RUANG): LLVM dulu memegang
/// satu ruang tipe, jadi `Titik` milik dua berkas ditolak dan `impl a::Suara` memakai trait lain.
#[test]
fn tipe_dan_trait_bernama_sama_di_dua_berkas_sama_di_tiga_mesin() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let dir = dir_uji("ruang");
    std::fs::create_dir_all(&dir).unwrap();
    let tulis = |n: &str, isi: &str| std::fs::write(dir.join(n), isi).unwrap();
    tulis("ta.jrg", "pub struct Titik { pub x: int }\npub enum Warna { Merah, Rgb(int) }\npub fn buat() { return Titik { x: 1 } }\npub fn w() { return Warna::Rgb(7) }\npub fn nama(t: Titik) -> string { return typeof(t) }\n");
    tulis("tipe.jrg", "package main\nimport \"ta\"\nstruct Titik { y: int }\nenum Warna { Biru }\npub fn main() {\n    let p = ta::buat()\n    wok(p, p.x, typeof(p), ta::nama(p))\n    let q = Titik { y: 2 }\n    wok(q, typeof(q))\n    wok(ta::w(), Warna::Biru)\n    wok(ta::Titik { x: 9 })\n    wok(str(p))\n}\n");
    tulis("a.jrg", "pub trait Suara { fn bunyi(self) -> string = \"a-default\" }\npub struct A { pub x: int }\nimpl Suara for A {}\npub fn coba() { return A { x: 1 }.bunyi() }\n");
    tulis("b.jrg", "pub trait Suara { fn bunyi(self) -> string = \"b-default\" }\npub struct B { pub x: int }\nimpl Suara for B {}\npub fn coba() { return B { x: 1 }.bunyi() }\n");
    tulis("c.jrg", "pub trait Wajib { fn harus(self) -> string\n  fn sapa(self) -> string = \"halo \" + self.harus() }\n");
    tulis("trait.jrg", "package main\nimport \"a\"\nimport \"b\"\ntrait Suara { fn bunyi(self) -> string = \"main-default\" }\nstruct M { x: int }\nimpl Suara for M {}\npub fn main() {\n    wok(a::coba(), b::coba(), M { x: 1 }.bunyi())\n}\n");
    tulis("impl.jrg", "package main\nimport \"a\"\nimport \"c\"\ntrait Suara { fn bunyi(self) -> string = \"main\" }\nstruct K { x: int }\nstruct L { x: int }\nimpl a::Suara for K {}\nimpl Wajib for L { fn harus(self) -> string = \"L\" }\npub fn main() {\n    wok(K { x: 1 }.bunyi(), L { x: 2 }.sapa())\n}\n");
    let jalan = |berkas: &str, mode: &[&str]| {
        let out = Command::new(jrgc()).arg(dir.join(berkas)).args(mode)
            .env("JRG_SOPAN", "1").current_dir(&dir).output().expect("jalankan jrgc");
        (String::from_utf8_lossy(&out.stdout).trim().to_string(),
         String::from_utf8_lossy(&out.stderr).to_string())
    };
    let bin = dir.join("uji.bin");
    let llvm: Vec<&str> = vec!["--emit-llvm", "-o", bin.to_str().unwrap()];
    let mut masalah = Vec::new();
    for (berkas, harap) in [("tipe.jrg", "Titik {x: 1} 1 struct:Titik struct:Titik"),
                            ("trait.jrg", "a-default b-default main-default"),
                            ("impl.jrg", "a-default halo L")] {
        let (i, _) = jalan(berkas, &[]);
        let (v, _) = jalan(berkas, &["--vm"]);
        let (l, g) = jalan(berkas, &llvm);
        if !i.starts_with(harap) { masalah.push(format!("{}: acuan {:?}", berkas, i)); }
        if v != i { masalah.push(format!("{}: --vm {:?}", berkas, v)); }
        if l != i { masalah.push(format!("{}: llvm {:?}\n{}", berkas, l, g)); }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(masalah.is_empty(), "{} masalah:\n{}", masalah.len(), masalah.join("\n"));
}

/// `Option` bersarang menjawab sama di TIAP kompilasi. Dijalankan berulang karena cacatnya
/// (Fase SIMPUL-2, #559) ada di urutan `HashMap`: `Option<Option<int>>` sesekali didaftarkan
/// sebelum `Option<int>`, dan satu jalan yang kebetulan benar meloloskannya dua pipeline.
#[test]
fn opsi_bersarang_deterministik() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let src = "package main\nfn bungkus(x) { return Some(Some(x)) }\npub fn main() {\n\
        let dua = Some(Some(5))\n if let Some(dalam) = dua { if let Some(v) = dalam { wok(v) } }\n\
        let tiga = Some(Some(Some(\"x\")))\n match tiga { Some(Some(Some(s))) => wok(s), _ => wok(\"tidak\") }\n\
        match bungkus(7) { Some(Some(n)) => wok(n + 1), _ => wok(0) }\n\
        let f = |x| x > 0 $ Some(Some(x * 2)) : None\n wok(f(4), f(-1)) }";
    let harap = jalankan_interp(src);
    assert!(harap.sukses, "acuan gagal:\n{}", harap.galat);
    for putaran in 0..10 {
        let l = jalankan_llvm(src);
        assert!(l.sukses, "putaran {}: LLVM gagal:\n{}", putaran, l.galat);
        assert_eq!(l.keluaran, harap.keluaran, "putaran {}: jawaban LLVM berubah antar-kompilasi", putaran);
    }
}

/// JRG engine (`std/mesin`) di LLVM: ketiga contoh game diputar tanpa jendela dan keluarannya
/// harus sama persis dengan interpreter (Fase MESIN-LLVM).
#[test]
fn contoh_game_mesin_sama_di_llvm() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let akar = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut salah = Vec::new();
    for nama in ["game_tangkap", "game_lompat", "game_menu"] {
        let src = std::fs::read_to_string(akar.join("examples").join(format!("{}.jrg", nama))).expect("baca contoh");
        let (i, l) = (jalankan_interp(&src), jalankan_llvm(&src));
        if !i.sukses || i.keluaran.is_empty() { salah.push(format!("{}: interpreter gagal\n{}", nama, i.galat)); continue; }
        if l.keluaran != i.keluaran || !l.sukses {
            salah.push(format!("{}:\n  acuan: {:?}\n  llvm:  {:?}\n  galat: {}", nama, i.keluaran, l.keluaran,
                l.galat.lines().take(6).collect::<Vec<_>>().join("\n         ")));
        }
    }
    assert!(salah.is_empty(), "contoh game berbeda di LLVM:\n{}", salah.join("\n"));
}

/// Perkakas, bukan tes: `JRG_EKSPOR_PARITAS=<dir> cargo test --test llvm_parity -- --ignored
/// ekspor_paritas` menulis tiap program `PARITAS` ke `<dir>/NNN.jrg` plus `daftar.txt`.
#[test]
#[ignore]
fn ekspor_paritas() {
    let Ok(dir) = std::env::var("JRG_EKSPOR_PARITAS") else { return };
    std::fs::create_dir_all(&dir).expect("buat direktori ekspor");
    let mut daftar = String::new();
    for (i, (nama, src)) in PARITAS.iter().enumerate() {
        std::fs::write(format!("{}/{:03}.jrg", dir, i), src).expect("tulis program");
        daftar.push_str(&format!("{:03}\t{}\n", i, nama));
    }
    std::fs::write(format!("{}/daftar.txt", dir), daftar).expect("tulis daftar");
}

/// Rekursi yang DALAM tapi berhenti dijawab LLVM dengan benar — penjaga stack tidak boleh
/// berupa penghitung kedalaman. Rekursi ekor jalan berapa pun dalamnya; yang bukan ekor
/// memakai stack sungguhan sejak `+` diperiksa (±500 ribu tingkat di stack 8 MiB).
#[test]
fn rekursi_dalam_yang_berhenti_tetap_dijawab_llvm() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let l = jalankan_llvm("package main\nfn d(n: int) -> int { if n == 0 { return 0 }\n return d(n - 1) + 1 }\n\
        fn genap(n: int) -> bool { if n == 0 { return true }\n return ganjil(n - 1) }\n\
        fn ganjil(n: int) -> bool { if n == 0 { return false }\n return genap(n - 1) }\n\
        pub fn main() { wok(d(300000), genap(5000000)) }");
    assert!(l.sukses, "LLVM gagal:\n{}", l.galat);
    assert_eq!(l.keluaran.trim(), "300000 true");
}

/// Galat saat jalan keluar dengan kode yang sama di interpreter dan LLVM (dulu 101 di LLVM).
#[test]
fn kode_keluar_galat_sama_dengan_acuan() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    for src in [
        "package main\npub fn main() { let a = [1]\n wok(a[5]) }",
        "package main\npub fn main() { panik(\"x\") }",
        "package main\nfn f(n: int) -> int { return f(n + 1) + 1 }\npub fn main() { wok(f(0)) }",
        "package main\npub fn main() { let x: Option<int> = None\n wok(x.unwrap()) }",
        "package main\npub fn main() { assert(1 == 2) }",
    ] {
        let (i, l) = (jalankan_interp(src), jalankan_llvm(src));
        assert!(i.kode == Some(1) && l.kode == i.kode, "{}: interpreter {:?}, LLVM {:?}\n{}", src, i.kode, l.kode, l.galat);
    }
}

/// `main -> int`: nilainya kode keluar di TIGA mesin (Fase KODE-MAIN), dengan aturan `exit` (di luar 0–255 gagal
/// sesudah `defer`); anotasi lain ditolak satu kalimat; tanpa anotasi nilainya dibuang, tapi `Err`/`None` dilaporkan.
#[test]
fn main_bulat_jadi_kode_keluar_di_tiga_mesin() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    for (src, kode, keluar, galat) in [
        ("package main\npub fn main() -> int { wok(\"a\")\n 3 }", 3, "a", ""),
        // Tanpa anotasi nilai ekornya dibuang — `fs::delete(f)` di baris terakhir bukan kode keluar.
        ("package main\npub fn main() { let x = 5\n x }", 0, "", ""),
        ("package main\npub fn main() { wok(1)\n [1, 2] }", 0, "1", ""),
        ("package main\npub fn main() { let r: Result<int, string> = Err(\"x\")\n r }", 1, "", "`Result::Err` merambat keluar dari `main`"),
        ("package main\npub fn main() -> u8 { return 9 }", 9, "", ""),
        ("package main\npub fn main() -> Option<int> { Some(3) }", 0, "", ""),
        ("package main\npub fn main() -> int { defer wok(\"tunda\")\n 256 }", 1, "tunda", "`main` mengembalikan 256 — kode keluar harus 0 sampai 255"),
        ("package main\npub fn main() -> int { -1 }", 1, "", "`main` mengembalikan -1"),
        ("package main\npub fn main() -> [2]int { [1, 2] }", 1, "", "`main` cuma boleh mengembalikan bilangan bulat 0 sampai 255"),
        ("package main\npub fn main() -> string { \"x\" }", 1, "", "bukan `string`"),
    ] {
        let hasil = [("interpreter", jalankan_interp(src)), ("--vm", jalankan_vm(src)), ("LLVM", jalankan_llvm(src))];
        for (mesin, h) in &hasil {
            assert!(h.kode == Some(kode) && h.keluaran == keluar && h.galat.contains(galat),
                "{} ({}): kode {:?}, keluaran {:?}\n{}", src, mesin, h.kode, h.keluaran, h.galat);
        }
        assert_eq!(hasil[0].1.galat, hasil[1].1.galat, "{}: stderr interpreter lawan --vm", src);
    }
}

/// Rekursi tak berujung lewat callback HOF (rekursi RUST, bukan bingkai JRG) dilaporkan oleh penjaga sisa stack
/// di dua mesin acuan, bukan mematikan proses (Fase REKURSI-LEGA: batas hitungan 10 000 dibuka).
#[test]
fn rekursi_lewat_callback_dilaporkan_bukan_crash() {
    let src = "package main\nfn g(n: int) -> int { return [n].map(|x| g(x + 1))[0] }\npub fn main() { wok(\"m\")\n wok(g(0)) }";
    for (mesin, h) in [("interpreter", jalankan_interp(src)), ("--vm", jalankan_vm(src))] {
        assert!(h.kode == Some(1) && h.keluaran == "m" && h.galat.contains("rekursi terlalu dalam di fungsi"),
            "{}: kode {:?}, keluaran {:?}\n{}", mesin, h.kode, h.keluaran, h.galat);
    }
}

/// Biner yang mati oleh sinyal disebut sinyalnya dan keluar 128+N (#876); dulu kode 1 tanpa pesan.
#[test]
fn biner_yang_mati_oleh_sinyal_disebut() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let l = jalankan_llvm("package main\nextern \"C\" {\n fn abort()\n}\npub fn main() { wok(\"sebelum\")\n abort() }");
    assert_eq!((l.keluaran.as_str(), l.kode), ("sebelum", Some(134)), "{}", l.galat);
    assert!(l.galat.contains("berhenti oleh sinyal 6 (SIGABRT)"), "{}", l.galat);
}

/// `JRGC_BATAS_MEMORI` membatasi biner yang dijalankan `--emit-llvm`: program rakus berhenti "memori habis"
/// sesudah keluarannya, dan nilai yang bukan bilangan MiB positif ditolak sebelum apa pun jalan.
#[test]
fn batas_memori_biner_llvm_ditegakkan() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let dir = dir_uji("batas");
    let file = dir.join("uji.jrg");
    std::fs::write(&file, "package main\npub fn main() { wok(\"mulai\")\n var s = \"x\"\n for i in 0..40 { s = s + s }\n wok(len(s)) }")
        .expect("tulis sumber");
    let jalan = |batas: &str| {
        let out = Command::new(jrgc()).arg(&file).arg("--emit-llvm").arg("-o").arg(dir.join("uji.bin"))
            .env("JRG_SOPAN", "1").env("JRGC_BATAS_MEMORI", batas).output().expect("jalankan jrgc");
        (String::from_utf8_lossy(&out.stdout).trim().to_string(), String::from_utf8_lossy(&out.stderr).to_string(),
         out.status.code())
    };
    let (k, g, kode) = jalan("300");
    assert!(k == "mulai" && kode == Some(1) && g.contains("memori habis"), "{:?} {:?}\n{}", k, kode, g);
    for salah in ["0", "abc", "-5", "1.5"] {
        let (k, g, kode) = jalan(salah);
        assert!(k.is_empty() && kode != Some(0) && g.contains("JRGC_BATAS_MEMORI harus bilangan MiB"), "{}: {:?}\n{}", salah, k, g);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--uji` di `--emit-llvm` melapor persis seperti interpreter: baris hasil, `(baris N)` dari situs galat (tanpa
/// baris untuk galat di modul lain), `defer` sebelum laporannya, filter, berkas tanpa tes, kode keluar.
#[test]
fn uji_di_llvm_seperti_acuan() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let dir = dir_uji("ujillvm");
    let tes = "package main\nimport \"std/base32\"\nfn dua(x) { return x * 2 }\nfn main() { wok(\"main tidak jalan\") }\n\
        fn test_lulus() { assert_eq(dua(2), 4) }\nfn test_gagal() {\n wok(\"sebelum\")\n assert_eq(dua(2), 5)\n}\n\
        fn uji_panik() { panik(\"meledak\") }\nfn test_indeks() {\n let a = [1, 2]\n wok(a[5])\n}\n\
        fn test_defer() {\n defer wok(\"defer jalan\")\n assert(false)\n}\n\
        fn test_goroutine() {\n let c = make_chan(int, 1)\n spawn { c <- 42 }\n assert_eq(<-c, 42)\n}\n\
        fn test_modul() { base32::encode(3.5) }\nfn test_kembali() { return 7 }\nfn test_akhir() { wok(\"akhir\") }\n";
    std::fs::write(dir.join("t.jrg"), tes).expect("tulis sumber");
    std::fs::write(dir.join("kosong.jrg"), "package main\nfn main() { wok(1) }\n").expect("tulis sumber");
    let jalan = |berkas: &str, llvm: bool, ekstra: &[&str]| {
        let mut c = Command::new(jrgc());
        c.arg(dir.join(berkas)).arg("--uji").args(ekstra).env("JRG_SOPAN", "1");
        if llvm { c.arg("--emit-llvm").arg("-o").arg(dir.join("u.bin")); }
        let o = c.output().expect("jalankan jrgc");
        (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), o.status.code())
    };
    for (berkas, ekstra) in [("t.jrg", &[][..]), ("t.jrg", &["--uji-filter", "lulus"][..]), ("kosong.jrg", &[][..])] {
        let (a, l) = (jalan(berkas, false, ekstra), jalan(berkas, true, ekstra));
        assert_eq!(a, l, "{} {:?}: interpreter (kiri) lawan --emit-llvm (kanan)", berkas, ekstra);
    }
    let (k, _, kode) = jalan("t.jrg", false, &[]);
    assert!(kode == Some(1) && k.contains("  GAGAL test_gagal (baris 8) — assert_eq gagal")
        && k.contains("defer jalan\n  GAGAL test_defer (baris 17)") && k.contains("  GAGAL test_modul — base32::encode")
        && !k.contains("main tidak jalan") && k.ends_with("--- 9 test: 4 lulus, 5 gagal ---\n"), "{}", k);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Kebuntuan yang terlihat dari goroutine `spawn` atau badan Future mengakhiri programnya SEKALI.
/// Dulu goroutine itu saja yang tamat, lalu kebuntuan yang sama ditemukan lagi sampai stack habis.
#[test]
fn kebuntuan_dari_goroutine_dilaporkan_sekali() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    for src in [
        "package main\npub fn main() { let m = Mutex::new()\n let c = make_chan(int)\n m.lock()\n spawn { m.lock()\n c <- 1 }\n wok(\"mulai\")\n wok(<-c) }",
        "package main\npub fn main() { let m = Mutex::new()\n m.lock()\n let f = Future::new(|| { m.lock()\n 1 })\n wok(\"mulai\")\n wok(future::await(f)) }",
    ] {
        let l = jalankan_llvm(src);
        assert!(!l.sukses && l.keluaran == "mulai", "{}\n{}", src, l.galat);
        assert_eq!(l.galat.matches("kebuntuan: semua goroutine sedang menunggu channel atau kunci").count(), 1, "{}\n{}", src, l.galat);
        assert!(!l.galat.contains("goroutine error") && !l.galat.contains("rekursi"), "{}\n{}", src, l.galat);
    }
}

/// **Tiap program `PARITAS` yang LLVM jalankan menjawab sama dengan acuan**
/// (Fase LLVM-16).
/// Bangun biner `--emit-llvm` tanpa menjalankannya. `None` kalau kompilasinya gagal.
fn bangun_llvm(dir: &std::path::Path, nama: &str, src: &str) -> Option<std::path::PathBuf> {
    let file = dir.join(format!("{}.jrg", nama));
    std::fs::write(&file, src).expect("tulis sumber");
    let bin = dir.join(format!("{}.bin", nama));
    let out = Command::new(jrgc()).arg(&file).arg("--emit-llvm").arg("-o").arg(&bin).arg("--no-run")
        .env("JRG_SOPAN", "1").env_remove("DISPLAY").env_remove("WAYLAND_DISPLAY")
        .output().expect("jalankan jrgc --emit-llvm");
    out.status.success().then_some(bin)
}

/// Waktu tercepat dari `kali` putaran, dengan batas memori: bentuk kuadratik di backend ini
/// juga kuadratik dalam MEMORI, dan tanpa batas ia menghabiskan mesinnya sebelum gagal.
fn tercepat_biner(bin: &std::path::Path, kali: usize) -> Option<f64> {
    let mut terbaik: Option<f64> = None;
    for _ in 0..kali {
        let t = std::time::Instant::now();
        let out = Command::new("sh").arg("-c").arg("ulimit -v 3000000; exec \"$0\"").arg(bin)
            .output().expect("jalankan biner");
        if !out.status.success() { return None; }
        let d = t.elapsed().as_secs_f64();
        terbaik = Some(terbaik.map_or(d, |x: f64| x.min(d)));
    }
    terbaik
}

/// (nama, kepala, badan `main` dengan `N` di tempat ukurannya, N kecil, batas rasio untuk 8x N).
/// Linear berarti sekitar 8x, kuadratik 64x; yang O(1) terhadap N dibatasi 4x.
const BIAYA_LLVM: &[(&str, &str, &str, usize, f64)] = &[
    ("a = a + [x]", "", "var a: []int = []\n var i = 0\n while i < N { a = a + [i]\n i += 1 }\n wok(len(a))", 200_000, 24.0),
    ("a = a + [[x, x]]", "", "var a: [][]int = []\n var i = 0\n while i < N { a = a + [[i, i]]\n i += 1 }\n wok(len(a))", 100_000, 24.0),
    ("A = A + [x] atas var modul", "var A: []int = []\n", "var i = 0\n while i < N { A = A + [i]\n i += 1 }\n wok(len(A))", 200_000, 24.0),
    ("a = a + [x] lewat tangkapan closure", "",
     "var a: []int = []\n let f = |x: int| { a = a + [x] }\n var i = 0\n while i < N { f(i)\n i += 1 }\n wok(len(a))", 100_000, 24.0),
    ("p.xs = p.xs + [x]", "struct P { pub xs: []int }\n",
     "var p = P { xs: [] }\n var i = 0\n while i < N { p.xs = p.xs + [i]\n i += 1 }\n wok(len(p.xs))", 200_000, 24.0),
    ("q = q.push(x)", "struct Q { pub xs: []int }\nimpl Q { pub fn push(self, x: int) -> Q { self.xs += [x]\n return self } }\n",
     "var q = Q { xs: [] }\n var i = 0\n while i < N { q = q.push(i)\n i += 1 }\n wok(len(q.xs))", 200_000, 24.0),
    ("s = s + \"x\" melewati satu chunk arena", "", "var s = \"\"\n var i = 0\n while i < N { s = s + \"x\"\n i += 1 }\n wok(len(s))", 500_000, 24.0),
    // Fase AMAN-34 (#918): `x = [.., x, ..]` memindahkan `x` — dulu disalin utuh tiap langkah (20 ribu langkah 3 GB).
    ("a = [a, x] membungkus dirinya", "", "var a = []\n var i = 0\n while i < N { a = [a, i]\n i += 1 }\n wok(len(a))", 50_000, 24.0),
    // Fase AMAN-35 (#922): rune ditempel lewat penyangga stack — dulu tiap rune teks arena baru, `s` disalin.
    ("s = s + rune", "", "let cs = str::chars(str::repeat(\"ab,\", N))\n var s = \"\"\n for _, c in cs { s = s + c }\n wok(len(s))", 50_000, 24.0),
    ("x = arr::pop(x)[0] di antara push", "", "var a: []string = []\n for _, i in range(N) { a = arr::push(a, \"a\")\n a = arr::pop(a)[0]\n a = arr::push(a, \"b\") }\n wok(len(a))", 50_000, 24.0),
    // Fase AMAN-35 (#923): `jrg_str_next_char` dulu memvalidasi SELURUH teks tiap langkah.
    ("for c in s atas teks", "", "let s = str::repeat(\"ab,\", N)\n var k = 0\n for _, c in s { if c == 'b' { k += 1 } }\n wok(k)", 100_000, 24.0),
    // Fase AMAN-35 (#923): fungsi yang punya closure dulu menyalin `x` untuk sisi kanan yang memanggil apa pun.
    ("x = arr::push(x, arr::map(.., closure))", "", "var x = [[0]]\n for _, i in range(N) { x = arr::push(x, arr::map([i], |y| y + 1)) }\n wok(len(x))", 20_000, 24.0),
    ("std/csv::parse", "import \"std/csv\"\n", "var b: []string = []\n for _, i in range(N) { b = arr::push(b, «${i},\\\"a,b\\\",x») }\n wok(len(csv::parse(str::join(b, \"\\n\"))))", 5_000, 24.0),
    ("d[k] = v", "", "var d: dict<int> = dict::new()\n var i = 0\n while i < N { d[str(i)] = i\n i += 1 }\n wok(len(d))", 50_000, 24.0),
    // Fase AMAN-38: `x = f(x, ..)` yang cuma mengembalikan `x` tidak lagi menyalin struct berisi dict, juga
    // lewat bongkar dan fungsi modul — dulu tiap panggilan menyalin dict-nya, dan arena tidak membebaskannya.
    ("x = f(x, ..) atas struct berisi dict", "struct P { pub d: dict = dict::new() }\nfn tulis(p, k) {\n p.d[k] = 1\n return p\n}\n",
     "var p = P {}\n for _, i in range(N) { p = tulis(p, str(i)) }\n wok(len(p.d))", 5_000, 24.0),
    // Fase AMAN-38 (#938): lokal di fungsi yang punya closure, tapi tidak disebut closure itu, dipinjamkan.
    ("argumen dari fungsi ber-closure ke parameter yang cuma dibaca", "fn satu(x: []int) -> int { return x[0] }\n",
     "let a = arr::map(arr::fill(0, N), |y| y + 1)\n var i = 0\n var s = 0\n while i < 200000 { s += satu(a)\n i += 1 }\n wok(s)", 2_000, 4.0),
    ("x, _ = m::f(x, ..): std/batas::coba", "import \"std/batas\"\n",
     "var p = batas::baru(5, 0.001, N)\n for _, i in range(N) { p, _ = batas::coba(p, «k${i}», i) }\n wok(batas::jumlah_kunci(p))", 2_000, 24.0),
    ("array sebagai argumen ke parameter yang cuma dibaca", "fn pjg(x: []int) -> int { return len(x) + x[0] }\n",
     "var a = arr::fill(0, N)\n var i = 0\n var s = 0\n while i < 200000 { s += pjg(a)\n i += 1 }\n wok(s)", 2_000, 4.0),
    ("struct berisi array sebagai argumen", "struct P { pub xs: []int }\nfn pjg(p: P) -> int { return len(p.xs) }\n",
     "var p = P { xs: arr::fill(0, N) }\n var i = 0\n var s = 0\n while i < 200000 { s += pjg(p)\n i += 1 }\n wok(s)", 2_000, 4.0),
    ("array bersarang sebagai argumen, satu elemennya dibaca", "fn satu(x: [][]int, i: int) -> int { return x[i][0] }\n",
     "var a: [][]int = []\n var i = 0\n while i < N { a = a + [[i]]\n i += 1 }\n var s = 0\n i = 0\n while i < 100000 { s += satu(a, i % 1000)\n i += 1 }\n wok(s)", 1_000, 4.0),
];

/// Padanan `memory_model.rs`/`vm_complexity.rs` untuk backend LLVM: ukuran dikali 8, lalu
/// rasio waktunya dibandingkan dengan batasnya.
#[test]
fn biaya_di_llvm_tetap_linear() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let dir = dir_uji("biaya");
    let mut temuan = Vec::new();
    for (i, (nama, kepala, badan, n, batas)) in BIAYA_LLVM.iter().enumerate() {
        let sumber = |n: usize| format!("package main\n{}pub fn main() {{\n {}\n}}\n", kepala, badan.replace('N', &n.to_string()));
        let (Some(kecil), Some(besar)) = (bangun_llvm(&dir, &format!("k{}", i), &sumber(*n)),
                                          bangun_llvm(&dir, &format!("b{}", i), &sumber(*n * 8))) else {
            temuan.push(format!("  {}: tidak terkompilasi", nama));
            continue;
        };
        let Some(a) = tercepat_biner(&kecil, 3) else { temuan.push(format!("  {}: gagal di N = {} (batas memori 3 GB)", nama, n)); continue };
        let Some(b) = tercepat_biner(&besar, 3) else {
            temuan.push(format!("  {}: gagal di N = {} (kehabisan memori berarti menyalin tiap langkah)", nama, n * 8));
            continue;
        };
        let rasio = b / a;
        if rasio > *batas {
            temuan.push(format!("  {}: {:.1}x untuk 8x N (batas {}x; {:.1} ms -> {:.1} ms)", nama, rasio, batas, a * 1e3, b * 1e3));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(temuan.is_empty(), "biaya di backend LLVM tumbuh lebih cepat dari ukurannya:\n{}", temuan.join("\n"));
}

/// Sumber yang sama menghasilkan IR yang sama. Dulu penyimpul lokal bertumbuh membaca urutan `HashMap`,
/// jadi pemanggil dan badan `std/csv::from_dicts` bisa sepakat beda tipe: 7 dari 12 kompilasi rusak (#972).
#[test]
fn ir_sama_tiap_kompilasi() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let dir = dir_uji("ir-tetap");
    let bungkus = dir.join("clang");
    std::fs::write(&bungkus, "#!/bin/sh\nfor a; do case $a in *.ll) cp \"$a\" \"$JRG_SALIN_IR\";; esac; done\nexit 1\n")
        .expect("tulis pembungkus");
    let _ = Command::new("chmod").arg("+x").arg(&bungkus).status();
    let mut korpus: Vec<(String, String)> = vec![("csv to_dicts/from_dicts".into(),
        "package main\nimport \"std/csv\"\npub fn main() {\n    let d = csv::to_dicts([[\"nama\"], [\"ani\"]])\n    \
         wok(csv::from_dicts([\"nama\"], d))\n}\n".into())];
    for nama in ["collections", "functions", "semantik_nilai"] {
        let p = format!("{}/tests/conformance/{}.jrg", env!("CARGO_MANIFEST_DIR"), nama);
        korpus.push((nama.into(), std::fs::read_to_string(&p).expect("baca konformans")));
    }
    let mut temuan = Vec::new();
    for (nama, src) in &korpus {
        let file = dir.join("p.jrg");
        std::fs::write(&file, src).expect("tulis sumber");
        let ir: Vec<Vec<u8>> = (0..3).map(|k| {
            let salin = dir.join(format!("ir{}.ll", k));
            let _ = Command::new(jrgc()).arg(&file).arg("--emit-llvm").arg("-o").arg(dir.join("p.bin")).arg("--no-run")
                .env("JRGC_CLANG", &bungkus).env("JRG_SALIN_IR", &salin).env("JRG_SOPAN", "1").output();
            std::fs::read(&salin).unwrap_or_default()
        }).collect();
        if ir[0].is_empty() { temuan.push(format!("  {}: IR tidak tertulis", nama)); }
        else if ir[0] != ir[1] || ir[0] != ir[2] { temuan.push(format!("  {}: IR berbeda antar kompilasi", nama)); }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(temuan.is_empty(), "IR backend LLVM tidak tetap:\n{}", temuan.join("\n"));
}

#[test]
fn tiap_program_paritas_yang_jalan_di_llvm_menjawab_sama() {
    if !ada_clang() { eprintln!("lewati: clang tidak ada"); return; }
    let kerja = std::sync::Mutex::new(PARITAS.iter().collect::<Vec<_>>());
    let temuan = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| loop {
                let Some((nama, src)) = kerja.lock().unwrap().pop() else { break };
                let l = jalankan_llvm(src);
                let tercatat = PARITAS_BEDA_DI_LLVM.iter().any(|(n, _)| n == nama);
                // Ditolak LLVM: celah backend yang keras, boleh — tapi entri tercatatnya basi. Biner yang MATI
                // karena cacat jrgc (tag sampah, ukuran karangan) bukan penolakan: itu jawaban rusak.
                if !l.sukses {
                    let rusak = l.galat.contains("bug di jrgc")
                        || (l.galat.contains("memori habis") && jalankan_interp(src).sukses);
                    if rusak {
                        temuan.lock().unwrap().push(format!("  RUSAK di LLVM: {}\n    {}", nama,
                            l.galat.lines().find(|b| b.contains("error")).unwrap_or("").trim()));
                    }
                    if tercatat {
                        temuan.lock().unwrap().push(format!(
                            "  KABAR BAIK, sekarang DITOLAK LLVM — hapus dari PARITAS_BEDA_DI_LLVM: {}", nama));
                    }
                    continue;
                }
                let i = jalankan_interp(src);
                let sama = i.sukses && i.keluaran == l.keluaran;
                if sama == tercatat {
                    temuan.lock().unwrap().push(if sama {
                        format!("  KABAR BAIK, sekarang sama — hapus dari PARITAS_BEDA_DI_LLVM: {}", nama)
                    } else {
                        format!("  {}\n    acuan: {:?}\n    llvm:  {:?}", nama, i.keluaran, l.keluaran)
                    });
                }
            });
        }
    });
    let mut temuan = temuan.into_inner().unwrap();
    temuan.sort();
    assert!(temuan.is_empty(), "{} program PARITAS lewat LLVM:\n{}", temuan.len(), temuan.join("\n"));
}

#[test]
fn kata_bahasa_lain_dijawab_sama_di_tiga_mesin() {
    if !ada_clang() {
        eprintln!("LEWAT: `clang` tidak ada, sapuan tiga mesin tidak bisa dijalankan");
        return;
    }
    let nama = nama_bahasa_lain();
    assert!(nama.len() >= 20,
        "pemanenan nama dari `padanan_bahasa_lain` mengecil ({} nama) — \
         sebuah sapuan yang tidak menyapu apa pun lulus dengan sempurna", nama.len());

    let mut beda = Vec::new();
    for n in &nama {
        // Nama yang berakhiran `_` adalah ejaan yang memang tidak bisa ditulis
        // sebagai kata kunci JRG di posisi ini; dilewati dengan sebab itu.
        if n.ends_with('_') { continue; }
        let src = format!("package main\npub fn main() {{ wok({}) }}\n", n);
        let i = jalankan_interp(&src);
        let v = jalankan_vm(&src);
        let l = jalankan_llvm(&src);
        let baris = |h: &Hasil| h.galat.lines().next().unwrap_or("").to_string();
        let (bi, bv, bl) = (baris(&i), baris(&v), baris(&l));
        if i.sukses || v.sukses || l.sukses {
            beda.push(format!("  `{}` DITERIMA di salah satu mesin", n));
        } else if bi != bv || bi != bl {
            beda.push(format!("  `{}`\n    int : {}\n    vm  : {}\n    llvm: {}", n, bi, bv, bl));
        }
    }
    assert!(beda.is_empty(),
        "{} kata dari bahasa lain dijawab berbeda antar-mesin:\n{}",
        beda.len(), beda.join("\n"));
}

// ─── Tabel 1: harus identik
// ────────────────────────────────────────────────

const PARITAS_LLVM: &[(&str, &str)] = &[
    // Fase BAWAAN-HOF (#868): builtin langsung sebagai callback HOF dan di `terap` dulu ditolak LLVM.
    ("builtin sebagai callback: map, filter, sort_by_key, each(wok), terap", r#"package main
fn f() {
 let b = [-1, -2].map(abs)
 wok("di f", b)
 return 7
}
pub fn main() { wok("hasil", f())
 wok(arr::map(["a"], str::upper))
wok(["1", "a"].filter(str::is_numeric))
 wok([-3, 1, -2].sort_by_key(abs))
wok(terap(abs, [-5]))
 wok("sebelum")
 [1, 2].each(wok)
 wok("sesudah") }"#),
    // Fase TERNARY-DUGA + POLA-ATAU (#866–#867): dulu "butuh tipe yang bisa disimpulkan" dan
    // "binding tidak didukung di dalam alternasi" di LLVM.
    ("ternary: tipe dari cabang pertama yang bernilai", r#"package main
pub fn main() { let n = 7; wok(n > 3 $ -1 : 1); wok(n > 3 $ !false : true); wok(n > 3 $ ~1 : 0); wok(n > 3 $ 'z' : 'y') }"#),
    ("pola alternatif yang mengikat: tuple, bersarang, array, guard", r#"for _, t in [(2, 5), (1, 7), (9, 9)] { wok(match t { (1, a) | (a, 5) => a, _ => 0 }) }
wok(match (1, (2, 3)) { (x, (y, 3) | (3, y)) => x + y, _ => 0 })
wok(match [1, 9] { [a, 9] | [9, a] => a, _ => 0 })
wok(match (2, 5) { (1, a) | (a, 5) if a > 1 => a * 10, _ => 0 })"#),
    ("pola alternatif yang mengikat: enum, struct, @, sisa array", r#"enum E { A(int), B(int), C }
struct P { pub x: int, pub y: int }
for _, e in [E::A(1), E::B(2), E::C] { wok(match e { E::A(n) | E::B(n) => n, E::C => -1 }) }
let p = P { x: 1, y: 4 }
wok(match p { P { x: 1, y: v } | P { x: v, y: 1 } => v, _ => 0 })
wok(match 3 { n @ (1 | 3) => n * 2, _ => 0 })
wok(match [0, 1, 2] { [0, ...r] | [9, ...r] => r, _ => [] })"#),
    // Fase POLA-LET (#864): pola literal, struct, `@` dan tuple yang bisa gagal di `let`/`for` dulu ditolak LLVM.
    ("let: pola yang bisa gagal lewat pencocok match", r#"package main
struct T { pub x: int, pub y: int = 2 }
pub fn main() {
 let (a, 5) = (1, 5)
 wok(a)
 let T { x, y } = T { x: 5 }
 wok(x, y)
 let n @ 1..=9 = 4
 wok(n)
 for _, (p, 1) in [(3, 1), (4, 1)] { wok(p) }
}"#),
    // Fase SAMBUNG-CAMPUR (#865): `[]bertag + []int` dulu ditolak "tipe elemennya beda".
    ("sambung array berjenis elemen berbeda", r#"package main
pub fn main() { var h = [1]
 let f = || { h = [0]; return [7] }
 h = h + [2]
 h = h.push(3)
 h = h + [4] + [5]
 wok(h)
 var g = len(env::args()) > 99 $ [1] : [2.5]
 wok(g + [3], [4] + g) }"#),
    // Fase JOIN-METODE (#863): `[1, 2].join("-")` dulu ditolak LLVM; objeknya dievaluasi sekali.
    ("join sebagai method atas array bukan teks", r#"package main
fn xs() -> []int { wok("sekali")
 return [1, 2] }
pub fn main() { wok([1, 2].join("-"), xs().join(", "), [1.5, 2.0].join("|")) }"#),
    // Fase RUNE-TEKS (#862): `rune` di argumen teks `str::` dulu ditolak LLVM ("rune sebagai string").
    ("str: rune sebagai teks satu karakter di semua pintu", r#"package main
pub fn main() {
    let r = "abc".char_at(1)
    wok(str::contains("abc", r), str::starts_with("bc", r), str::ends_with("ab", r))
    wok(str::replace("abc", r, "X"), str::split("abcbd", r), str::index_of("abc", r))
    wok(str::repeat(r, 3), str::upper(r), str::count("bbb", r))
    wok(str::pad_left("7", 3, r), str::join(["x", "y"], r))
}"#),
    // Fase SEBAR-DICT (#861): tuple kosong dan sebar struct/dict berjenis lain ke `dict { .. }` dulu ditolak LLVM.
    ("potong: tuple kosong", r#"package main
pub fn main() { let t = (1, 2)
 let k = t[1:1]
 wok(typeof(k), k, len(k), k == t[0:0])
 wok(t[2:], t[:0]) }"#),
    ("sebar ke literal dict: basis struct dan dict berjenis lain", r#"package main
struct Q { a: int, b: int = 3 }
pub fn main() { wok(dict { a: 9, ...Q { a: 1 } }, dict { z: "t", ...Q { a: 1 } })
 let d = dict { a: 1 }
 wok(dict { s: "x", ...d }, dict::merge(d, {"a": "y"}))
 wok(dict { ...Q { a: 2 } }) }"#),
    // Fase MINMAX-CAMPUR (#860): bulat lawan pecahan dulu ditolak LLVM; jawabannya argumen aslinya.
    ("math::max/min bulat lawan pecahan: seri, NaN, typeof", r#"package main
pub fn main() {
 wok(math::max(1, 2.5), math::max(3, 2.5), math::max(2, 2.0), math::max(2.0, 2))
 wok(math::min(1, 2.5), math::min(3, 2.5), math::min(2, 2.0), math::min(2.0, 2))
 let a = math::max(1, 2.5)
 wok(typeof(a), typeof(math::max(3, 2.5)))
 wok(math::max(1, math::NAN), math::min(math::NAN, 1))
}"#),
    // Fase KUNCI-VARIABEL (#859): `p[k]` dengan kunci dari variabel dulu ditolak LLVM saat kompilasi.
    ("struct: indeks teks dari variabel, campur dan setipe, privat dari dalam method", r#"package main
struct P { pub x: int, pub nama: string, rahasia: int = 7 }
struct Q { pub a: int, pub b: int }
impl P { fn baca(self, k) { return self[k] } }
pub fn main() {
    let p = P { x: 1, nama: "a" }
    for _, k in ["x", "nama"] { wok(p[k], typeof(p[k])) }
    wok(p.baca("rahasia"))
    let q = Q { a: 3, b: 4 }
    var t = 0
    for _, k in ["a", "b"] { t = t + q[k] }
    wok(t)
}"#),
    // Fase TUPLE-INDEKS (#856): `t[i]` dengan `i` dari variabel dulu ditolak LLVM saat kompilasi.
    ("tuple: indeks dari variabel, setipe (baca/tulis) dan campur tipe", r#"package main
pub fn main() {
    var t: (int, int) = (1, 2)
    var i: int = 0
    wok(t[i])
    i = -1
    wok(t[i])
    t[i] = 9
    wok(t)
    let c = (1, "dua", 3.5)
    for k in 0..3 { wok(c[k], typeof(c[k])) }
    var j = -3
    wok(c[j])
}"#),
    // Fase BAWAAN-NILAI (#855): fungsi pustaka sebagai nilai dulu ditolak LLVM ("dipakai sebagai NILAI").
    ("fungsi bawaan sebagai nilai", r#"package main
pub fn main() {
    wok([1.0, 4.0].map(math::sqrt))
    let f = str::upper
    wok(f("ab"), typeof(f), f)
    wok(arr::map(["a", "bc"], len))
    let g = math::max
    wok(g(1, 5))
}"#),
    // Fase KTOR-GENERIK (#854): dulu "belum mendukung konstruktor sebagai nilai" di LLVM.
    ("konstruktor generik, Some, Ok, Err sebagai nilai", r#"package main
enum G<T> { V(T), C }
enum P<A, B> { Dua(A, B) }
pub fn main() {
    let f = G::V
    wok(f(3), f("x"), typeof(f), f)
    let s = Some
    let o = Ok
    let e = Err
    wok(s(1), o("a"), e(2.5), typeof(s), s)
    wok(arr::map([1, 2], Some))
    let d = P::Dua
    wok(d(1, "b"))
}"#),
    // Fase WARIS-GENERIK (#853): mewarisi struct generik dulu ditolak LLVM ("field warisannya tidak bisa diratakan").
    ("warisan: induk generik, method dan default warisan", r#"package main
struct Dasar<T> { pub isi: T, pub n: int = 1 }
struct Anak: Dasar { pub tambah: int }
struct Cucu: Anak { pub z: bool = true }
impl<T> Dasar<T> { fn ambil(self) -> T { self.isi } fn dua(self) -> int { self.n * 2 } }
pub fn main() {
    let a = Anak { isi: "x", tambah: 2 }
    wok(a, a.isi, a.n, a.tambah)
    wok(a.ambil(), a.dua())
    let b = Anak { isi: 5, tambah: 1, n: 7 }
    wok(b, b.dua(), b == b)
    let c = Cucu { isi: [1, 2], tambah: 0 }
    wok(c, c.ambil())
}"#),
    // Fase ENUM-GENERIK (#852): enum generik milik program dulu "enum `E` tidak dikenal" di LLVM.
    ("enum generik: konstruktor, anotasi, fungsi dan impl generik, rekursif, varian ber-field", r#"package main
enum Pilih<T> { Ada(T), Kosong }
enum Daftar<T> { Isi(T, Daftar<T>), Habis }
enum Dua<A, B> { Kiri(A), Kanan(B), Titik { x: A, y: B } }
fn buka<T>(p: Pilih<T>, d: T) -> T { match p { Pilih::Ada(x) => x, Pilih::Kosong => d } }
fn panjang<T>(l: Daftar<T>) -> int { match l { Daftar::Isi(_, s) => 1 + panjang(s), Daftar::Habis => 0 } }
impl<T> Pilih<T> {
    fn ada(self) -> bool { match self { Pilih::Ada(_) => true, Pilih::Kosong => false } }
}
pub fn main() {
    let a = Pilih::Ada(5)
    let b: Pilih<string> = Pilih::Kosong
    wok(a, b, typeof(a))
    wok(buka(a, 0), buka(b, "x"), buka(Pilih::Ada("y"), "z"))
    wok(a.ada(), b.ada())
    var l: Daftar<int> = Daftar::Habis
    for i in 0..4 { l = Daftar::Isi(i, l) }
    wok(l, panjang(l))
    let d: Dua<int, string> = Dua::Kanan("k")
    let e: Dua<int, string> = Dua::Titik { x: 1, y: "s" }
    wok(d, e, Dua::Kiri(2.5))
    wok(a == Pilih::Ada(5), a == Pilih::Ada(6))
    match e { Dua::Titik { x, y } => wok(x, y), _ => wok("lain") }
    let xs = [Pilih::Ada(1), Pilih::Kosong, Pilih::Ada(3)]
    wok(xs)
}"#),
    ("enum generik: varian unit tanpa konteks dan varian telanjang", r#"package main
enum G<T> { V(T), C }
pub fn main() {
    let k = G::C
    wok(k)
    let m = V(3)
    wok(m, C)
}"#),
    // Fase JSON-JEMBATAN (#851): dulu "belum bisa disandikan jadi JSON di backend `--emit-llvm`".
    ("json: struct, enum, tuple dan wadahnya", r#"package main
struct N { pub v: int, pub kiri: Option<N> }
enum E { A, B(int) }
pub fn main() { let n = N { v: 1, kiri: Some(N { v: 2, kiri: None }) }
 wok(json::stringify(n), json::stringify([E::A, E::B(3)]), json::stringify((1, "x")))
 wok(json::pretty(n))
 wok(json::pretty({"k": E::B(1)}, 4), json::stringify([1, 2])) }"#),
    // Fase ENUM-JEMBATAN (#850): enum milik program dulu tidak bisa menyeberang jembatan, jadi struct
    // apa pun yang memuat `Option` ditolak; argumen struct/enum langsung ditolak saat kompilasi.
    ("jembatan: struct berisi enum, enum dan struct rekursif", r#"package main
struct Q { pub v: int, pub o: Option<int> }
struct N { pub v: int, pub kiri: Option<N> }
enum Bentuk { Titik, Lingkar(float), Kotak { w: int, h: int }, Garis(int, int) }
pub fn main() { let c = Chan::new(1)
 c <- Q { v: 1, o: Some(5) }
 let r = <-c
 wok(r.v, r.o)
 wok(dok::json_encode(N { v: 1, kiri: Some(N { v: 2, kiri: None }) }))
 wok(dok::json_encode([Bentuk::Titik, Bentuk::Lingkar(1.5), Bentuk::Kotak { w: 2, h: 3 }, Bentuk::Garis(4, 5)]))
 wok(ptr::get(ptr::new(Bentuk::Garis(1, 2))), dok::json_encode((1, "a"))) }"#),
    // Fase KOTAK-LLVM (#849): tipe yang memuat dirinya sebaris dulu ditolak LLVM (#836); field yang
    // menutup siklusnya sekarang kotak di heap — salinan tidak berbagi, tampil/`==`/pola lewat isinya.
    ("tipe rekursif: enum memuat dirinya", r#"package main
enum L { Kosong, Isi(int, L) }
fn jumlah(l: L) -> int { match l { L::Kosong => 0, L::Isi(x, sisa) => x + jumlah(sisa) } }
pub fn main() { var l = L::Kosong
 for i in 0..100 { l = L::Isi(i, l) }
 wok(jumlah(l))
 var k = L::Kosong
 for i in 0..3 { k = L::Isi(i, k) }
 let m = k
 wok(k, m == k, L::Isi(1, L::Kosong) == L::Isi(2, L::Kosong)) }"#),
    ("tipe rekursif: struct lewat Option, salinan tidak berbagi", r#"package main
struct N { pub v: int, pub kiri: Option<N> }
impl N { fn jumlah(self) -> int { match self.kiri { Some(k) => self.v + k.jumlah(), None => self.v } } }
pub fn main() { var a = N { v: 1, kiri: Some(N { v: 2, kiri: Some(N { v: 3, kiri: None }) }) }
 var c = a
 c.v = 9
 var m = N { v: 7, kiri: None }
 let n = N { v: 8, kiri: Some(m) }
 m.v = 70
 wok(a, c.v, a.jumlah(), a == c)
 wok(n, m)
 a.kiri = None
 wok(a, c.jumlah())
 match n.kiri { Some(N { v, kiri }) => wok(v, kiri), None => wok("kosong") } }"#),
    ("tipe rekursif: struct generik, varian ber-field, tuple", r#"package main
struct T<X> { pub v: X, pub a: Option<T<X>> }
enum Pohon { Daun(int), Cabang { kiri: Pohon, kanan: Pohon } }
struct P { pub x: (int, Option<P>) }
fn total(p: Pohon) -> int { match p { Pohon::Daun(x) => x, Pohon::Cabang { kiri, kanan } => total(kiri) + total(kanan) } }
pub fn main() { let t = T { v: 1, a: Some(T { v: 2, a: None }) }
 wok(t)
 let p = Pohon::Cabang { kiri: Pohon::Daun(1), kanan: Pohon::Cabang { kiri: Pohon::Daun(2), kanan: Pohon::Daun(3) } }
 wok(p, total(p))
 var q = P { x: (1, Some(P { x: (2, None) })) }
 var r = q
 r.x.0 = 9
 wok(q, r)
 let v = len(env::args()) > 99 $ 1 : t
 wok(v, [t, t][1].v) }"#),
    // Fase POTONG-BERTAG (#848): potongan atas nilai bertag dulu ditolak saat kompilasi.
    ("potong: nilai bertag berisi teks", "package main\npub fn main() { let v = len(env::args()) > 99 $ 5 : \"halo dunia\"\n wok(v[0:4], v[5:len(v)], v[:2])\n let s = crypto::hex_encode(\"ab\")\n wok(s[1:3]) }"),
    // Fase BAGI-LLVM: `gcd` atas MIN dulu menjawab bertanda salah (`-2`).
    ("math::gcd dan lcm atas bilangan negatif", "package main\npub fn main() { let m = -9223372036854775807 - 1\n wok(math::gcd(m, 6))\n wok(math::gcd(-12, 18))\n wok(math::lcm(-4, 6))\n wok(math::lcm(0, 5))\n wok(math::gcd(0, 0)) }"),
    // Fase BAGI-LLVM: `akhir - awal` di luar 64 bit dulu memberi panjang karangan (`1`, bukan `18`).
    ("range: selisih di luar 64 bit", "package main\npub fn main() { var a = 9000000000000000000\n var b = -9000000000000000000\n wok(len(range(a, b, -1000000000000000000)))\n var t = 0\n for i in range(9223372036854775800, 9223372036854775807) { t = t + 1 }\n wok(t)\n wok(range_inclusive(6, 3))\n wok(range(10, 0, -4)) }"),
    // Fase BAGI-LLVM: `srem` atas MIN dan -1 tak terdefinisi; dulu menjawab `1`.
    // Fase HOF-LLVM (#832): bukan fungsi atas koleksi kosong tidak pernah dipanggil, jadi tidak gagal.
    ("hof: bukan fungsi atas koleksi kosong", "package main\nfn nol() -> int { 1 }\npub fn main() { var a: []int = []\n wok(a.map(5), a.reduce(0, 1), a.filter(true))\n wok(a.all(|| 1), a.find(nol), a.fold(2, |s| s)) }"),
    // Fase GESER-LLVM (#831): geseran yang muat di 64 bit, juga tak bertanda dan sampai MIN.
    ("geser: hasil yang muat di 64 bit", "package main\npub fn main() { var k = 63\n wok(-1 << k)\n var j = 62\n wok(1 << j, -1 << j)\n var c: u64 = 1\n var m: u64 = 63\n wok(c << m)\n wok(-8 >> k) }"),
    ("bagi: sisa MIN % -1 nol", "package main\npub fn main() { var m = -9223372036854775807 - 1\n var n = -1\n wok(m % n)\n m %= -1\n wok(m)\n wok(-7 % n)\n wok(-7 / n) }"),
    // Fase POTONG: dulu ditolak saat kompilasi ("tidak bisa slicing pada tipe").
    ("potong: tuple berbatas literal", "package main\npub fn main() { let t = (1, \"x\", 2.5)\n let u = t[1:]\n wok(u)\n wok(u.1 + 1.0)\n wok(t[-1:])\n wok(t[:2]) }"),
    // Fase KUNCI-LLVM (#824): `len`/`is_empty` sebagai field atas teks dan array; method skalar tanpa `()`.
    ("indeks teks atas struct", r#"package main
struct P { pub x: int, pub n: string, y: int = 3 }
impl P { fn baca(self) -> int { self["y"] * 2 } }
pub fn main() {
  var p = P { x: 1, n: "a" }
  wok(p["x"] + 1, p["n"], typeof(p["n"]))
  p["x"] = 5
  p["x"] += 2
  wok(p.x, p.baca())
}
"#),
    ("len dan is_empty sebagai field", r#"package main
pub fn main() {
  let s = "éa"
  wok(s.len, s.is_empty, "".is_empty)
  let a = [1, 2]
  wok(a.is_empty, a.len)
  let f = 2.5
  wok(f.floor)
}"#),
    // Fase KUNCI-LLVM (#823): method skalar acuan lewat `math::`, `to_string` atas skalar.
    ("method skalar int dan float", r#"package main
pub fn main() {
  let f = 2.7
  let i = -3
  let r: i32 = -4
  let u: u8 = 9
  wok(f.ceil(), f.round(), (-1.5).abs(), f.pow(2), f.ln(), f.sqrt(), f.is_finite(), f.is_inf())
  wok(i.abs(), r.abs(), u.abs(), i.to_string() + "!", f.to_string(), true.to_string(), 'q'.to_string())
}"#),
    // Fase KUNCI-LLVM (#822): method acuan yang dulu cuma ada di bentuk namespace.
    ("method repeat atas teks dan set atas dict", r#"package main
pub fn main() {
  let s = "ab"
  wok(s.repeat(3), "x".repeat(0))
  let d = {"a": 1}
  wok(d.set("b", 2), d)
}"#),
    // Fase KUNCI-LLVM (#821): penerima method yang bukan variabel dibaca sebagai nilai.
    ("method atas None, const dan varian telanjang", r#"package main
enum W { Merah, Biru }
impl W { fn nama(self) -> string = match self { W::Merah => "m", W::Biru => "b" } }
const K = "ab"
pub fn main() {
  wok(None.unwrap_or(7))
  wok(K.upper())
  wok(Merah.nama())
  wok(None.is_none())
}"#),
    // Fase SEBAR-TIPE: field yang ditulis literalnya tidak dibaca dari basis; field bertag dibuka.
    ("sebar dari struct lain berbeda tipe field", r#"package main
struct A { pub x: int, pub y: string }
struct B { pub x: int, pub y: int }
struct D { pub x: tuple, pub y: tuple }
struct E { pub x: int, pub y: tuple }
pub fn main() {
    let b = B { x: 1, y: 2 }
    let a = A { y: "teks", ...b }
    wok(a.x, a.y)
    let d = D { x: 3, y: "dy" }
    let a4 = A { ...d }
    wok(a4.x, a4.y)
    let e = E { ...a }
    wok(e.x, e.y, typeof(e.y))
}
"#),
    // Fase PTR-LLVM: `Ptr` sebagai tipe field, parameter dan anotasi; pegangan `jawa::` di dalamnya.
    ("Ptr sebagai tipe field dan anotasi", r#"package main
struct Kotak { pub p: Ptr, pub n: int }
struct Sel { pub h: Ptr }
fn ambil() -> Ptr { ptr::new(3) }
fn baca(h: Ptr) -> int { jawa::get(h) }
pub fn main() {
    let k = Kotak { p: ptr::new(5), n: 1 }
    ptr::set(k.p, 9)
    var q = k
    ptr::set(q.p, 11)
    wok(ptr::get(k.p), k.n)
    let p: Ptr = ambil()
    let xs: []Ptr = [p, ptr::new(4)]
    wok(ptr::get(p), typeof(p), ptr::get(xs[1]))
    let a: Ptr = jawa::alloc(7)
    wok(baca(a), typeof(a))
    let s = Sel { h: jawa::alloc(8) }
    jawa::set(s.h, 9)
    wok(jawa::get(s.h))
    jawir::free(a)
    wok(jawa::is_valid(a))
}
"#),
    // Fase PEGANGAN-BERTAG: `Atomic`/`Mutex` di dalam nilai bertag, bentuk namespace dan method.
    ("pegangan sinkron di dalam nilai bertag", r#"package main
fn pilih(n: int) { if n == 0 { return Mutex::new() } if n == 1 { return [7, 8] } return Atomic::new(2) }
pub fn main() {
    let m = pilih(0)
    mutex::lock(m)
    wok("kunci", typeof(m))
    mutex::unlock(m)
    wok(mutex::with_lock(m, || 5), m.with_lock(|| 6))
    let b = pilih(2)
    atomic::add(b, 3)
    wok(atomic::cas(b, 5, 10), atomic::get(b))
    let c = b
    atomic::set(c, 1)
    wok(b.get(), b.add(1), b.cas(2, 9), b.get())
    wok(pilih(1).get(1))
}
"#),
    // Fase KEPUTUSAN-1: `-2 ** 2` = -4, indeks negatif di builtin dari belakang.
    ("pangkat lawan minus, indeks negatif di builtin", r#"package main
pub fn main() {
    let x = 3
    wok(-2 ** 2, -x ** 2, (-2) ** 2, ~2 ** 2, -2 ** 3 ** 2, -128 as i8, - 2 ** 2 + 1, -2.0 ** 2)
    let a = [10, 20, 30]
    wok(a.get(-1), a.get(-4), a.get_or(-1, 0), a.get_or(-9, 0))
    wok(a.remove(-1), a.insert(-1, 9), arr::insert(a, -3, 5), a.swap(-1, 0), "abc".char_at(-1))
}
"#),
    // Fase JAWA-LLVM: `jawa::array(n, isi)` dengan isi awal.
    ("jawa::array dengan isi awal", r#"package main
pub fn main() {
    let a = jawa::array(3, 7)
    jawa::array_set(a, 1, 9)
    wok(jawa::array_get(a, 0), jawa::array_get(a, 1), jawa::array_len(a), jawa::array_to_list(a))
    jawir::free(a)
    wok(jawa::is_valid(a))
}
"#),
    // Fase PETA-OPSI: `map`/`map_err`/`and_then` atas Option dan Result, termasuk yang mengganti tipe isinya.
    ("peta atas Option dan Result", r##"package main
fn bagi(a: float, b: float) -> Result<float, string> {
    if b == 0.0 { return Err("nol") }
    return Ok(a / b)
}
fn cari(xs: []int, x: int) -> Option<int> {
    for i, v in xs { if v == x { return Some(i) } }
    return None
}
fn ke_teks(n: int) -> string { return "#" + str(n) }
pub fn main() {
    wok(Some(2).map(|x| x + 1), cari([1, 2], 9).map(|i| i * 10))
    wok(bagi(20.0, 4.0).map(fn(v) => v * 2.0), bagi(1.0, 0.0).map(|v| v * 2.0))
    wok(bagi(1.0, 0.0).map_err(|s| s + "!"), bagi(2.0, 1.0).map_err(|s| s + "!"))
    wok(cari([5, 6], 6).and_then(|i| cari([0, 1, 2], i)), cari([5], 6).and_then(|i| Some(i)))
    wok(bagi(8.0, 2.0).and_then(|v| bagi(v, 2.0)), bagi(8.0, 0.0).and_then(|v| bagi(v, 2.0)))
    wok(cari([3, 4], 4).map(ke_teks), Some(1).map(|x| x + 1).map(|x| x * 5))
    let urutan = Some(1).map(|x| { wok("di dalam f"); x })
    wok(urutan)
}
"##),
    // Fase KTOR-LLVM: konstruktor enum non-generik sebagai nilai — tampilan, `typeof`, HOF, `|>`.
    ("konstruktor sebagai nilai", r#"package main
enum E { A(int), B, C(int, string) }
enum W { K(string), L }
fn terap(f: fn(string) -> W, x: string) -> W { return f(x) }
fn dua(x: int) -> int { return x * 2 }
pub fn main() {
    let f = E::A
    wok(f, typeof(f), [f], str(f))
    wok(f(3), arr::map([1, 2], E::A))
    let g = E::C
    wok(g(1, "x"))
    let k = K
    wok(k("a"), "b" |> W::K, terap(W::K, "c"))
    let fs = [W::K, W::K]
    wok(fs[1]("d"))
    wok(dua, typeof(dua), typeof(|x| x))
}
"#),
    // Fase SUBSTR: awal `str::substr` negatif, dan eksponen negatif TERTULIS menjawab pecahan.
    ("substr awal negatif, pangkat negatif tertulis", r#"package main
fn separuh(x: int) { return x ** -1 }
pub fn main() {
    let s = "halo dunia"
    wok(str::substr(s, -5, 3), s.substr(-5), str::substr(s, -10, 4), "[" + str::substr(s, 10) + "]", str::substr("héllo", -4, 2))
    wok(2 ** -1, 3 ** -2, typeof(2 ** -1), (-2) ** -3, 10 ** -3 * 1000.0, separuh(4))
    let x = 5
    let h = x ** -2
    wok(h, typeof(h))
}
"#),
    // Fase JRGM-2: `http::` menjawab `Result` (dulu galat saat jalan di tiga mesin).
    ("http: gagal sambung dan URL tak sah jadi Err", r#"package main
pub fn main() {
    match http::get("http://127.0.0.1:1/x") { Ok(j) => wok("ok"), Err(e) => wok("err", str::contains(e, "sambungan ditolak")) }
    wok(http::get("http://127.0.0.1:abc/"))
}
"#),
    // Fase PEGANGAN-LLVM: dulu entri `BELUM_DIDUKUNG_LLVM` ("`i64` sebagai `Atomic`").
    ("pegangan dibandingkan dengan jenis lain menjawab false", r#"package main
pub fn main() {
 let a = Atomic::new(1)
 let m = Mutex::new()
 let c = make_chan(int)
 wok(a == 1, a != 1, m == "x", c == nil, 1 == a, a == m)
}
"#),
    // Fase KALIMAT: `jawa::sizeof` atas nilai biasa menaksir ukurannya, dulu ditolak LLVM.
    ("jawa::sizeof atas nilai yang bukan pegangan", r#"package main
pub fn main() {
 wok(jawa::sizeof(5), jawa::sizeof("abc"), jawa::sizeof([1,2]), jawa::sizeof(2.5), jawa::sizeof(true))
 let b = jawa::buffer(4)
 wok(jawa::sizeof(b))
 wok(jawa::sizeof({"a": 1}))
}
"#),
    // Fase HEKS: buffer dari teks heksa lahir di runtime LLVM, bukan di registri jembatan.
    ("jawa::buffer_from_hex dibaca buffer_get dan buffer_to_hex", r#"package main
pub fn main() {
 let b = jawa::buffer_from_hex("00ff41")
 wok(jawa::buffer_get(b, 1))
 wok(jawa::buffer_len(b))
 let c = jawa::buffer(3)
 wok(jawa::buffer_get(c, 0))
}
"#),
    ("jawa::buffer_to_hex atas buffer dari teks heksa", r#"package main
pub fn main() {
 let c = jawa::buffer(2)
 jawa::buffer_set(c, 0, 171)
 wok(jawa::buffer_to_hex(c))
 wok(jawa::buffer_to_hex(jawa::buffer_from_hex("ABcd")))
 wok(jawa::buffer_len(jawa::buffer_from_hex("")))
}
"#),
    // Fase SALIN-MODUL: nilai milik `var` modul yang dibaca lewat penunjuk disalin kalau yang
    // dipanggil sesudahnya bisa menulis `var` modul. Yang pertama dulu entri `DIVERGENSI_LLVM`.
    ("operan array var modul ditulis operan berikutnya",
     "package main\nvar A = [1, 2]\nfn ubah() -> []int { A[0] = 4; return [5] }\npub fn main() { let x = A + ubah()\n wok(x) }"),
    ("self milik var modul tidak ikut tulisan fungsi lain, operan kiri dibaca lebih dulu", r#"package main
struct K { pub n: int, pub xs: []int }
var G = K { n: 1, xs: [1] }
fn ubah() { G = K { n: 99, xs: [9, 9] } }
impl K {
    fn f(self) -> int {
        let a = self.n
        ubah()
        return a + self.n + len(self.xs)
    }
    fn baca(self) -> int { return self.n }
}
var A = [1, 2, 3]
fn tulis() -> int { A[0] = 4; return 5 }
fn ubah2() -> []int { A[0] = 7; return [5] }
pub fn main() {
    wok(G.f())
    wok(G.baca())
    wok(A + [tulis()])
    wok(A + ubah2())
    let x = A + [1]
    wok(x)
}
"#),
    // Fase RUANG-LLVM: pola varian atas nilai yang pasti bukan enum tidak cocok, bukan ditolak.
    ("pola varian telanjang dan berawalan atas struct jatuh ke lengan lain", r#"package main
enum E { X(int), Y }
struct P { a: int }
pub fn main() {
 let p = P { a: 1 }
 wok(match p { Y => "y", _ => "lain" })
 wok(match p { E::Y => "y", _ => "lain" })
}
"#),
    // Fase GENAP: pintu byte dan `fs::` yang dulu tidak ada di sini, lewat jembatan.
    ("crypto: array byte dan aes_decrypt_bytes", r#"package main
pub fn main() {
    let k = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
    let iv = "000102030405060708090a0b0c0d0e0f"
    let s = crypto::aes_encrypt([0, 255, 65], k, iv).unwrap()
    wok(crypto::aes_decrypt(s, k, iv))
    wok(crypto::aes_decrypt_bytes(s, k, iv))
    wok(crypto::aes_decrypt_bytes(crypto::aes_encrypt("hé", k, iv).unwrap(), k, iv))
    wok(crypto::aes_decrypt_bytes(s, "00", iv))
    wok(crypto::base64_encode([0, 255, 65]), crypto::base64_encode("hé"))
    wok(crypto::hex_encode([0, 255]), crypto::hex_encode("A"))
    wok(crypto::sha256([104, 105]) == crypto::sha256("hi"), crypto::md5([]) == crypto::md5(""))
    wok(crypto::crc32([97]), crypto::crc32("a"), crypto::hmac_sha256([104], "x") == crypto::hmac_sha256("h", "x"))
    wok(crypto::base64_encode(crypto::base64_decode_bytes("AP9B").unwrap()))
}
"#),
    ("fs: try_read dan pintu base64 lewat jembatan", r#"package main
pub fn main() {
    let d = fs::join(fs::temp_dir(), "jrg-l3")
    fs::mkdir(d)
    let f = fs::join(d, "b.bin")
    wok(fs::write_base64(f, "AP9B"))
    wok(fs::read_base64(f))
    wok(fs::try_read(f))
    wok(fs::try_read(fs::join(d, "hilang")))
    wok(fs::append_base64(f, "Qg=="))
    wok(fs::size(f))
    fs::write(fs::join(d, "t.txt"), "hé")
    wok(fs::try_read(fs::join(d, "t.txt")))
    fs::delete(d)
    wok(fs::exists(d))
}
"#),
    // Fase UTF8-LLVM: berkas bukan UTF-8 dibaca sebagai teks dijawab `Err`, bukan teks kosong; `env::exe` ada.
    ("fs: berkas bukan UTF-8 lewat file::, env::exe", r#"package main
pub fn main() {
    let f = fs::join(fs::temp_dir(), "jrg-l3-utf8.bin")
    fs::write_base64(f, "//4K")
    wok(file::read(f))
    wok(file::read_lines(f))
    wok(fs::try_read(f))
    fs::copy(f, f + ".2")
    wok(fs::size(f + ".2"))
    fs::delete(f + ".2")
    fs::write(f, "hé\nya")
    wok(file::read_lines(f), fs::read(f))
    fs::delete(f)
    wok(env::exe().len() > 0, fs::exists(env::exe()))
}
"#),
    ("dok: BLOB SQLite jadi array byte, TEXT bukan UTF-8 jadi Err", r#"package main
pub fn main() {
    if !dok::db_tersedia() { wok("tanpa sqlite")  return }
    let db = dok::db_open(":memory:").unwrap()
    wok(dok::db_query(db, "SELECT x'00ff41' AS b, 'é' AS t, 3 AS n"))
    wok(dok::db_query(db, "SELECT CAST(x'ff41' AS TEXT) AS rusak"))
    wok(dok::db_query(db, "SELECT CAST(CAST(x'ff41' AS TEXT) AS BLOB) AS rusak"))
    dok::db_close(db)
}
"#),
    ("dok: tcp_recv tidak membelah karakter, pintu byte TCP dan UDP", r#"package main
fn port(l) {
    let a = dok::tcp_local_addr(l)
    let p = str::split(a, ":")
    return str::to_int(p[p.len() - 1])
}
fn port_udp(u) {
    let p = str::split(dok::udp_local_addr(u), ":")
    return str::to_int(p[p.len() - 1])
}
pub fn main() {
    let l = dok::tcp_listen("127.0.0.1", 0).unwrap()
    let k = dok::tcp_connect("127.0.0.1", port(l), 2000).unwrap()
    let c = dok::tcp_accept(l).unwrap()
    dok::tcp_send(k, "aaaé€b").unwrap()
    wok(dok::tcp_recv(c, 4))
    wok(dok::tcp_recv(c, 4))
    wok(dok::tcp_recv(c, 1))
    wok(dok::tcp_recv(c, 4))
    dok::tcp_send(k, [97, 255, 98]).unwrap()
    wok(dok::tcp_recv(c, 8))
    wok(dok::tcp_recv(c, 8))
    wok(dok::tcp_recv_bytes(c, 8))
    dok::tcp_send(k, [0, 200, 10]).unwrap()
    wok(dok::tcp_recv_line(c))
    dok::tcp_send(k, [226, 130]).unwrap()
    dok::tcp_close(k)
    wok(dok::tcp_recv(c, 8))
    wok(dok::tcp_recv(c, 8))
    dok::tcp_close(c)
    dok::tcp_close_listener(l)
    let u = dok::udp_bind("127.0.0.1", 0).unwrap()
    let v = dok::udp_bind("127.0.0.1", 0).unwrap()
    let pu = port_udp(u)
    dok::udp_send_to(v, "127.0.0.1", pu, "héé").unwrap()
    wok(dok::udp_recv_from(u, 64).unwrap().data)
    dok::udp_send_to(v, "127.0.0.1", pu, "héé").unwrap()
    wok(dok::udp_recv_from(u, 2).unwrap().data)
    dok::udp_send_to(v, "127.0.0.1", pu, [1, 255]).unwrap()
    wok(dok::udp_recv_from(u, 64))
    wok(dok::udp_recv_bytes_from(u, 64).unwrap().data)
}
"#),
    ("crypto: dekode byte bukan teks", r#"package main
pub fn main() {
    wok(crypto::base64_decode_bytes("AP9B"))
    wok(crypto::base64_decode("AP9B"))
    wok(crypto::hex_decode_bytes("00ff"))
}
"#),
    // Fase SEBAR: basis bertipe lain sah selama field-nya bagian dari sasaran.
    ("basis sebar bertipe struct lain yang field-nya bagian dari sasaran", r#"package main
struct B { pub x: int, pub y: int = 7 }
struct H { pub x: int }
struct K: H { pub k: int }
struct N { pub nama: string, pub x: int }
struct M { pub x: int, pub nama: string, pub z: int = 3 }
pub fn main() {
    let h = H { x: 4 }
    wok(K { k: 1, ...h })
    wok(B { ...h })
    let n = N { nama: "nn", x: 8 }
    wok(M { ...n })
    wok(M { x: 1, ...n })
    let b = B { x: 1, y: 2 }
    wok(B { x: 5, ...b })
}"#),
    // Fase BINER: LLVM dulu menjawab "tipe `V` tidak dikenal" untuk varian yang ditulis telanjang.
    ("literal varian ber-field yang ditulis telanjang", r#"package main
enum E { V { a: int, b: int }, K }
fn lihat(e: E) -> int {
    match e { E::V { a, b } => a + b, E::K => 0 }
}
pub fn main() {
    let e = V { a: 1, b: 2 }
    wok(lihat(e))
    wok(lihat(V { b: 5, a: 4 }))
    wok(V { a: 7, b: 8 })
}"#),
    // Fase PULIH: `A { ...a }` dulu ditolak parser di tiga mesin.
    ("literal struct yang isinya cuma basis sebar",
     "package main\nstruct A { pub x: int, pub y: int }\npub fn main() {\n let a = A { x: 1, y: 2 }\n wok(A { ...a })\n let b = A { ...a }\n wok(b.x + b.y)\n}"),
    // Fase WAJIB: urutan evaluasi default field sama di tiga mesin.
    ("default field struct: hanya yang tidak diisi, sesudah field tertulis", r#"package main
fn catat(s: string) -> int { wok(s); return 1 }
struct A { pub x: int = catat("default A.x"), pub y: int = catat("default A.y"), pub z: int = catat("default A.z") }
struct B: A { pub w: int = catat("default B.w"), pub y: int = catat("default B.y") }
pub fn main() {
    wok(A { y: catat("tulis y") })
    let a = A { x: 5, y: 6, z: 7 }
    wok(A { y: catat("tulis y2"), ...a })
    wok(B { z: catat("tulis z") })
    wok(B {})
}
"#),
    // Fase BONGKAR: `q, v = q.pop()` di biner berjalan di tempat `q`. 200 ribu elemen —
    // bentuk dua pernyataan kehabisan memori di ukuran ini.
    ("std/queue, deque, heap: q, v = q.pop() atas 200 ribu elemen", r#"package main
import "std/queue"
import "std/deque"
import "std/heap"
pub fn main() {
    var q: queue::Queue<int> = queue::new()
    var d: deque::Deque<int> = deque::new()
    var h: heap::Heap<int, int> = heap::new()
    var s = 0
    var v: Option<int> = None
    for i in 0..200000 { q = q.push(i)
      d = d.push_front(i) }
    for i in 0..20000 { h = h.push((i * 7919) % 10007) }
    for i in 0..200000 { q, v = q.pop()
      s = s + (v ?? 0)
      d, v = d.pop_back()
      s = s + (v ?? 0) }
    for i in 0..20000 { h, v = h.pop()
      s = s + (v ?? 0) }
    wok(s); wok(q.len()); wok(d.len()); wok(h.len()); wok(q.pop()[1])
}
"#),
    // Fase TUTUP: array literal yang meluruh jadi slice DISALIN ke heap. Dulu slice-nya
    // menunjuk stack fungsinya, dan `return [v, v + 1]` menjawab sampah.
    ("array literal jadi slice lalu keluar dari fungsi: return, field, Some", r#"package main
fn f(v: int) -> []int { return [v, v + 1] }
fn g(v: int) -> []int { let a = [v, v * 2]
  return a }
fn kotor(n: int) -> int { var t = [9, 9, 9, 9, 9, 9, 9, 9]
  return t[n] + n }
struct K { pub xs: []int }
fn h(v: int) -> K { return K { xs: [v, 7] } }
fn o(v: int) -> Option<[]int> { return Some([v]) }
pub fn main() {
    let a = f(3)
    let b = g(5)
    let c = h(4)
    let d = o(6)
    wok(kotor(1))
    wok(a); wok(b); wok(c); wok(d)
}
"#),
    ("match di ekor fungsi dengan lengan array beda panjang; return [v] lawan return []", r#"package main
fn ta(o) {
    match o {
        Some(v) => [v],
        None    => [],
    }
}
fn tb(o) {
    match o {
        Some(v) => return [v, v],
        None => return [],
    }
}
pub fn main() { wok(ta(Some(3))); wok(typeof(ta(Some(3)))); let k: Option<int> = None
  wok(ta(k)); wok(tb(Some(4))); wok(tb(k)) }
"#),
    ("separuh tipe tak terlihat jadi nilai bertag: None, Ok, Err, var x = nil", r#"package main
pub fn main() {
    var kosong = nil
    kosong = 5
    wok(kosong); kosong = "teks"; wok(kosong)
    var x = None
    wok(x); x = Some(3); wok(x); x = Some("teks"); wok(x)
    var r = Ok(4)
    wok(r); r = Err("rusak"); wok(r)
    let e = Err(5)
    wok(e); wok(typeof(e)); wok(e == Err(5)); wok(None == None)
    match x { Some(v) => wok(v), None => wok("kosong") }
    let daftar = [None, Some(1)]
    wok(daftar); wok(len(daftar))
    wok(x ?? "bawaan"); let y = None
    wok(y ?? 7)
}
"#),
    ("x = f(x, ..) dan nama tertangkap: closure melihat nilai LAMA selagi fungsinya jalan", r#"package main
struct K { pub xs: []int }
impl K {
    pub fn terap(self, f: fn() -> int) -> K { self.xs += [1]
      self.xs += [f()]
      return self }
}
fn pakai(a: []int, f: fn() -> int) -> []int { a = arr::push(a, 1)
  a = arr::push(a, f())
  return a }
pub fn main() {
    var p = K { xs: [5] }
    let g = || len(p.xs)
    p = p.terap(g)
    wok(p.xs)
    var a = [5]
    let h = || len(a)
    a = pakai(a, h)
    wok(a)
    var b = [1]
    let b0 = b
    b = pakai(b, || 0)
    wok(b); wok(b0)
}
"#),
    ("x = f(x, ..): dict::set, dict::remove, fungsi pemakai — tanpa alias", r#"package main
struct K { pub xs: []int }
fn tambah(k, x) { k.xs += [x]
  return k }
fn isi(d, k, v) { d[k] = v
  return d }
fn dorong(a, x) { a = arr::push(a, x)
  return a }
fn rek(d, n) { if n == 0 { return d }
  d = isi(d, n, n * n)
  d = rek(d, n - 1)
  return d }
fn banyak(a, xs...) { for _, x in xs { a = a + [x] }
  return a }
fn bawaan(a, x = 7) { a = a + [x]
  return a }
fn gagal(d, i) { let a = [1]
  d["x"] = a[i]
  return d }
pub fn main() {
    var k = K { xs: [1] }
    let k0 = k
    var k1 = k
    k = tambah(k, 2)
    k = tambah(tambah(k, 3), 4)
    wok(k); wok(k0); wok(k1)
    var d = {"a": 1}
    let d0 = d
    d = dict::set(d, "b", 2)
    d = isi(d, "c", 3)
    d = dict::remove(d, "a")
    d = dict::remove(d, "tak ada")
    wok(d); wok(d0)
    let f = || { d = dict::set(d, "f", 0) }
    f()
    wok(d)
    var a = [1]
    let a0 = a
    a = dorong(a, 2)
    a = banyak(a, 3, 4)
    a = bawaan(a)
    a = bawaan(a, 9)
    wok(a); wok(a0)
    wok(rek({"0": 0}, 4))
    var e = {"z": 1}
    e = rek(e, 2)
    wok(e)
    var s = "ab"
    s = str::upper(s)
    wok(s)
    var t = {"k": 1}
    wok("sebelum")
}
"#),
    // Fase BERTIPE: penyalin-dalam menembus struct bersarang dan payload enum; nilai yang
    // dikembalikan dari `self` pinjaman, dari pengikat pola, dan dari `var` modul disalin.
    // Dua puluh dari 34 baris ini dulu dijawab salah tanpa pesan.
    ("alias: kembalian dari self, struct bersarang, enum, pola, var modul", r#"package main
struct K { pub a: []int, pub b: []int }
impl K {
    pub fn sama(self) -> K { return K { a: self.a, b: self.b } }
    pub fn dorong(self, x: int) -> K { return K { a: arr::push(self.a, x), b: self.b } }
    pub fn isi(self) -> []int { return self.a }
}
fn lewat(k: K) -> K { return k }
fn ambil(k: K) -> []int { return k.b }
pub fn main() {
    var k = K { a: [1, 2], b: [7, 8] }
    var m = k.sama()
    m.a[0] = 9; m.b[1] = 0
    wok(k); wok(m)
    var n = k.dorong(3)
    n.b[0] = 100; n.a[0] = 50
    wok(k); wok(n)
    var x = k.isi()
    x[1] = 77
    wok(k); wok(x)
    var l = lewat(k)
    l.a[0] = -1
    var y = ambil(k)
    y[0] = -2
    wok(k); wok(l); wok(y)
    let p = k.dorong(4)
    let r = k.dorong(5)
    wok(p.a); wok(r.a)
    var t = (k.a, k.b)
    t[0][0] = 31
    wok(k); wok(t)
    kedua()
    ketiga()
}
struct D { pub d: dict<int>, pub n: K, pub o: Option<[]int> }
impl D {
    pub fn diri(self) -> D { return self }
    pub fn dik(self) -> dict<int> { self.d }
    pub fn dalam(self) -> []int = self.n.a
    pub fn pasang(self) -> (K, []int) { return (self.n, self.n.b) }
    pub fn pilih(self, c: bool) -> []int { if c { return self.n.a }
      return self.n.b }
    pub fn opsi(self) -> []int { match self.o { Some(v) => return v, None => return [] } }
}
pub fn kedua() {
    var d = D { d: {"a": 1}, n: K { a: [1, 2], b: [3] }, o: Some([5, 6]) }
    var e = d.diri()
    e.n.a[0] = 9; e.d["a"] = 2
    var f = d.dik()
    f["a"] = 3; f["z"] = 0
    var g = d.dalam()
    g[1] = 8
    var h = d.pasang()
    h[0].a[0] = 7; h[1][0] = 6
    var i = d.pilih(true)
    i[0] = 5
    var j = d.opsi()
    j[0] = 4
    wok(d); wok(e); wok(f); wok(g); wok(h); wok(i); wok(j)
}
var G = [1, 2]
enum Bentuk { Daftar([]int), Kosong }
fn ambil_g() { return G }
fn isi_bentuk(b: Bentuk) -> []int { match b { Bentuk::Daftar(xs) => return xs, Bentuk::Kosong => return [] } }
pub fn ketiga() {
    var x = ambil_g()
    x[0] = 9
    wok(G); wok(x)
    var o = Some([1, 2])
    var p = o
    match p { Some(v) => { v[0] = 7
        wok(v) } None => {} }
    wok(o); wok(p)
    var b = Bentuk::Daftar([3, 4])
    var c = b
    var z = isi_bentuk(c)
    z[0] = 0
    wok(isi_bentuk(b)); wok(isi_bentuk(c)); wok(z)
    var d1 = D { d: {"a": 1}, n: K { a: [1, 2], b: [3] }, o: Some([5, 6]) }
    var d2 = d1
    d2.n.a[0] = 99
    let ds = [d1, d2]
    var d3 = ds[0]
    d3.n.b[0] = 55
    wok(d1.n); wok(d2.n); wok(ds[0].n); wok(d3.n)
    let tp = (K { a: [1], b: [2] }, Some([3]))
    var tq = tp
    tq[0].a[0] = 4
    wok(tp); wok(tq)
}
"#),
    ("std/queue, deque, heap bertipe; parameter tipe tak terlihat jadi nilai bertag", r#"package main
import "std/queue"
import "std/deque"
import "std/heap"
struct Titik { pub x: int }
pub fn main() {
    var q: queue::Queue<Titik> = queue::new()
    q = q.push(Titik { x: 1 }).push(Titik { x: 2 })
    wok(q.len()); wok(q.pop()[1]); wok(q.peek())
    let s = queue::from(["a", "b"]).push("c")
    wok(s); wok(s.to_array()); s.each(|x| wok(x + "!"))
    var d: deque::Deque<float> = deque::new()
    d = d.push_back(1.5).push_front(0.5)
    wok(d); wok(d.pop_back()[1]); wok(d.peek_front())
    var h = heap::new()
    h = h.push(3).push(1).push(2)
    wok(h); wok(h.pop()[1]); wok(h.to_sorted_array())
    var bebas = queue::new()
    bebas = bebas.push(1).push("dua").push(3.5)
    wok(bebas); wok(bebas.pop()[1])
}
"#),
    ("method tanpa anotasi atas struct generik: closure tanpa hasil, struct baru dari penduga", r#"package main
struct S<T> { pub d: []T }
impl<T> S<T> {
    pub fn ubah(self, f) {
        var k = []
        for _, v in self.d { k = arr::push(k, f(v)) }
        return S { d: k }
    }
    pub fn tiap(self, f) { for _, v in self.d { f(v) } }
}
struct S2<T> { pub d: dict<T> }
impl<T> S2<T> {
    pub fn ubah(self, f) {
        var d = dict::new()
        for _, v in dict::values(self.d) {
            let y = f(v)
            d = dict::set(d, y, y)
        }
        return S2 { d: d }
    }
}
pub fn main() { let a = S { d: [1, 2] }
  a.tiap(|x| wok(x))
  wok(a.ubah(|x| x * 2).d)
  let b = S2 { d: {"1": 1, "2": 2} }
  wok(b.ubah(|x| x * 2).d) }
"#),
    // Fase GENERIK: struct generik dimonomorfisasi per susunan argumen tipe.
    ("struct generik: literal, method, fungsi generik, tipe dari tempat hasil", r#"package main
struct Kotak<T> { pub isi: []T, pub nama: string }
impl<T> Kotak<T> {
    pub fn tambah(self, x: T) -> Kotak<T> { return Kotak { isi: arr::push(self.isi, x), nama: self.nama } }
    pub fn banyak(self) -> int { return len(self.isi) }
    pub fn pertama(self) -> Option<T> { return arr::first(self.isi) }
}
fn baru<T>(nama: string) -> Kotak<T> { return Kotak { isi: [], nama: nama } }
fn id<T>(x: T) -> T { return x }
pub fn main() {
    var k: Kotak<int> = baru("angka")
    k = k.tambah(3)
    k = k.tambah(4)
    wok(k.banyak()); wok(k.pertama()); wok(k); wok(typeof(k))
    var t: Kotak<string> = baru("teks")
    t = t.tambah("a")
    wok(t); wok(id(5)); wok(id("x"))
    let l = Kotak { isi: [1.5], nama: "pecahan" }
    wok(l.tambah(2.5))
}
"#),
    ("struct generik: dua parameter, impl<U>, operator, hook, field fn, bersarang", r#"package main
struct Pasang<A, B> { pub kiri: A, pub kanan: B }
struct Tumpuk<T> { isi: []T, pub nama: string = "tumpukan" }
struct Titik { pub x: int, pub y: int }
struct Pilih<T> { pub f: fn(T) -> int, pub bawaan: Option<T> }
impl<A, B> Pasang<A, B> {
    pub fn tukar(self) -> Pasang<B, A> { return Pasang { kiri: self.kanan, kanan: self.kiri } }
    pub fn to_string(self) -> string { return "<" + str(self.kiri) + " | " + str(self.kanan) + ">" }
}
impl<U> Tumpuk<U> {
    pub fn dorong(self, x: U) -> Tumpuk<U> { return Tumpuk { isi: arr::push(self.isi, x), nama: self.nama } }
    pub fn ambil(self) -> (Tumpuk<U>, Option<U>) {
        let h = arr::pop(self.isi)
        return (Tumpuk { isi: h[0], nama: self.nama }, h[1])
    }
    pub fn tinggi(self) -> int { return len(self.isi) }
    pub fn eq(self, lain: Tumpuk<U>) -> bool { return self.isi == lain.isi }
}
impl<T> Pilih<T> {
    pub fn nilai(self, x: T) -> int { let f = self.f
      return f(x) }
}
fn kosong<T>() -> Tumpuk<T> { return Tumpuk { isi: [] } }
fn pasang<A, B>(a: A, b: B) -> Pasang<A, B> { return Pasang { kiri: a, kanan: b } }
fn pertama<T>(xs: []T) -> Option<T> { return arr::first(xs) }
fn jumlah(t: Tumpuk<int>) -> int { var s = 0
  var k = t
  for i in 0..t.tinggi() { let r = k.ambil()
    k = r[0]
    s = s + (r[1] ?? 0) }
  return s }
pub fn main() {
    let p = pasang(1, "satu")
    wok(p); wok(p.tukar()); wok(p.tukar().kiri); wok(typeof(p))
    var t: Tumpuk<Titik> = kosong()
    t = t.dorong(Titik { x: 1, y: 2 }).dorong(Titik { x: 3, y: 4 })
    wok(t.tinggi()); wok(t.nama)
    let r = t.ambil()
    wok(r[1]); wok(r[0].tinggi())
    var a: Tumpuk<int> = kosong()
    for i in 0..5 { a = a.dorong(i * i) }
    wok(jumlah(a)); wok(a == a.dorong(1)); wok(a == a)
    let daftar = [pasang(1, 2.5), pasang(3, 4.5)]
    wok(daftar[1].kanan); wok(len(daftar)); wok(daftar)
    let pl = Pilih { f: |s: string| len(s), bawaan: Some("xy") }
    wok(pl.nilai("halo")); wok(pl.bawaan)
    wok(pertama([7, 8])); wok(pertama(["a"]))
    let dalam = pasang(pasang(1, 2), [pasang("a", true)])
    wok(dalam.kiri.kanan); wok(dalam.kanan[0].kiri)
}
"#),
    ("std/heap bertipe: Heap<T, K> lewat tiga mesin", r#"package main
import "std/heap"
pub fn main() { let h = heap::from([5,3,8,1,9,2]); wok(h.len()); wok(h.peek()); wok(h.to_sorted_array()); let r = h.pop(); wok(r[1]); wok(r[0].to_sorted_array()); wok(heap::from_by(["ccc","a","bb"], |s| len(s)).to_sorted_array())
  var k: heap::Heap<string, string> = heap::new()
  k = k.push("b").push("a")
  wok(k); wok(k.pop()[1]); wok(typeof(k))
  let kosong: heap::Heap<int, int> = heap::new()
  wok(kosong.pop()[1]); wok(kosong.peek()) }
"#),
    ("std/mesin/tween: closure dan fungsi bernama ke field fn(float) -> float", r#"package main
import "std/mesin/tween"
pub fn main() {
    var t = tween::baru(0, 10, 2, tween::masuk_kuad)
    t = t.maju(1); wok(t.nilai())
    let u = tween::baru(1.5, 3, 4.0, tween::pantul).maju(3)
    wok(u.nilai()); wok(tween::antara(0, 8, 0.25, tween::keluar_kubik)); wok(tween::linear(2)); wok(tween::sinus(0.5))
}
"#),
    ("math::sign: NaN tetap NaN, tanda mengikuti jenis argumennya", r#"package main
pub fn main() { wok(math::sign(math::NAN), math::sign(-2.5), math::sign(0.0), math::sign(-3), math::sign(7), math::sign(0)) }
"#),
    // Fase SERPIH-3: tiga bentuk yang dulu ditolak atau mematikan program di LLVM.
    ("bilangan bulat beda lebar dan tanda: dibandingkan dan dihitung menurut nilainya", r#"package main
pub fn main() {
    let x: i32 = 5
    let y: i64 = 5
    wok(x == y, x < y, x + y, typeof(x + y))
    let a: u8 = 200
    let b: i8 = -100
    wok(a + b, a - b, a * b, a > b, a == b)
    let c: u64 = 18446744073709551615
    let d: i64 = -1
    wok(c == d, c > d, d < c, d != c)
    let e: i16 = 300
    let f: i64 = 9000000000
    wok(e * f, f / e, f % e)
}
"#),
    ("defer di badan closure: jalan saat closure pulang, lewat return maupun ekor", r#"package main
fn pakai(f: fn(int) -> int) -> int { return f(3) }
pub fn main() {
    var jml = 0
    let g = |a: int| -> int {
        defer wok("keluar", a)
        if a > 2 { return a * 10 }
        jml += a
        a
    }
    wok(g(1), g(5), jml)
    wok(pakai(|x| { defer wok("dalam"); x + 1 }))
}
"#),
    ("goroutine yang gagal berhenti sendirian: defer-nya jalan, main lanjut", r#"package main
fn gagal() {
    defer wok("defer goroutine")
    let a = [1]
    wok(a[5])
}
pub fn main() {
    spawn { gagal() }
    time::sleep(0.05)
    wok("main lanjut")
}
"#),
    // Fase SAPU-29: temuan dua sapuan builtin (contoh benar, dan argumen angka berganti jenis).
    ("env::set tidak menjawab apa-apa; handle jawa:: tampil dengan kind-nya", r#"package main
pub fn main() {
    wok(typeof(env::set("JRG_SAPU29", 1)), env::get("JRG_SAPU29"))
    let p = jawa::alloc(5)
    let a = jawa::array(2)
    let b = jawa::buffer(2)
    wok(p, a, b)
    wok(p.id, a.kind, typeof(b))
    jawa::array_set(a, 0, 9)
    jawa::array_push(a, 4)
    wok(jawa::array_get(a, 0), jawa::array_get(a, 2), jawa::array_len(a))
    wok(math::degrees(180), math::radians(3.5), math::is_finite(1), math::log(8, 2))
}
"#),
    // Fase PINJAM-LLVM: argumen ke parameter yang cuma dibaca tidak disalin lagi, jadi yang
    // DIKEMBALIKAN dari pinjaman (self, parameter, elemen `for`) harus nilai sendiri.
    ("tugas yang gagal dan tidak pernah diambil hasilnya tidak menghentikan program", r#"package main
fn bagi(a: int, b: int) -> int { return a / b }
pub fn main() {
    let f = Future::new(bagi, 6, 0)
    let g = Task::spawn(bagi, 6, 2)
    wok("jalan terus")
    wok(task::join(g))
    time::sleep(0.02)
    wok(future::is_done(f))
}
"#),
    ("`Self` di tanda tangan dan badan method adalah tipe penerimanya", r#"package main
struct T { pub x: int }
impl T { fn baru(x: int) -> Self { return Self { x: x } }
 fn tambah(self, n: int) -> Self { return Self { x: self.x + n } }
 fn sama(self, o: Self) -> bool { return self.x == o.x }
 fn daftar(self) -> []Self { return [self, Self { x: 0 }] } }
struct A { pub x: int }
struct B: A { pub y: int }
impl A { fn buat(self) { return Self { x: self.x + 1 } } }
struct K<T> { pub v: T }
impl K<T> { fn lagi(self) -> Self { return Self { v: self.v } } }
pub fn main() { let t = T::baru(3)
 wok(t.tambah(4).x, t.sama(T::baru(3)), t.daftar())
 let a = A { x: 1 }
 wok(a.buat(), typeof(a.buat()))
 let k = K { v: 2 }
 wok(k.lagi())
 }
"#),
    ("Option::ok, Result::ok dan Result::err menjawab Option; isinya nilai sendiri", r#"package main
pub fn main() { let o: Option<int> = Some(3)
 let n: Option<int> = None
 let r: Result<int, string> = Ok(4)
 let e: Result<int, string> = Err("x")
 wok(Option::ok(o), Option::ok(n), o.ok(), n.ok())
 wok(r.ok(), r.err(), e.ok(), e.err(), typeof(e.err()))
 wok(e.err().unwrap() + "y", r.ok().unwrap() + 1)
 wok(Result::ok(r), Result::err(e))
 let ra: Result<[]int, string> = Ok([1, 2])
 var x = ra.ok().unwrap()
 x[0] = 9
 wok(x, ra) }
"#),
    ("predikat HOF memakai truthiness; enumerate/zip/list telanjang; str::sub satu argumen", r#"package main
pub fn main() { wok(str::sub("a,b"), str::slice("a,b"), str::substring("a,b"))
 wok(list("ab"), list([1, 2]), typeof(list("ab")))
 wok(arr::count([0, 1, 2], |x| x), arr::count_if([0, 1, 2], |x| x), arr::find([0, 1, 2], |x| x), arr::find_idx([0, 3], |x| x), arr::partition([0, 1, 2], |x| x))
 wok(arr::filter([0, 1, 2, 0, 3], |x| x), arr::filter(["", "a"], |s| s), [0.0, 1.5].filter(|x| x))
 wok(arr::any(["", "a"], |s| s), arr::all(["", "a"], |s| s), [1, 2].any(|x| x - 1), [[], [1]].filter(|a| a))
 wok(enumerate([10, 20]), zip([1, 2], ["a", "b"]))
 wok(arr::count([1, 2, 3], |x| x > 1), arr::partition([1, 2, 3], |x| x % 2 == 0)) }
"#),
    ("jawa::array_pop, array_to_list, buffer_fill, buffer_to_hex, sizeof", r#"package main
pub fn main() { let a = jawa::array(2)
 jawa::array_set(a, 0, 5)
 jawa::array_set(a, 1, 6)
 jawa::array_push(a, 7)
 wok(jawa::array_to_list(a), typeof(jawa::array_to_list(a)))
 let p = jawa::array_pop(a)
 wok(p, typeof(p), jawa::array_len(a))
 wok(jawa::array_pop(a), jawa::array_pop(a))
 let b = jawa::buffer(3)
 let r = jawa::buffer_fill(b, 171)
 wok(r, typeof(r), jawa::buffer_to_hex(b), jawa::buffer_get(b, 2))
 wok(jawa::sizeof(a), jawa::sizeof(b), jawa::sizeof(jawa::alloc(1)), typeof(jawa::sizeof(b)))
 }
"#),
    ("json::set dan json::merge menjawab objek baru", r#"package main
pub fn main() { let o = {"a": 1, "b": 2}
 let p = json::set(o, "c", 3)
 wok(p, o, json::set(o, "a", 9))
 wok(json::merge(o, {"b": 7, "z": 1}), json::merge({"a": 1}, {"a": 1}))
 let j = json::parse("{\"k\": [1, 2], \"s\": \"x\"}")
 wok(json::set(j, "n", 5))
 wok(json::stringify(json::merge(j, json::parse("{\"s\": 2}"))))
 }
"#),
    ("pinjaman: yang dikembalikan dari self atau parameter yang cuma dibaca adalah nilai sendiri", r#"package main
struct Q { pub xs: []int, pub ys: [][]int }
struct K { pub xs: []int }
impl Q {
    pub fn a(self) -> Option<[]int> { return Some(self.xs) }
    pub fn b(self) -> ([]int, int) { return (self.xs, 1) }
    pub fn c(self, k: bool) -> []int { return k $ self.xs : self.xs }
    pub fn d(self) -> []int { self.xs }
    pub fn e(self) -> [][]int { return [self.xs] }
    pub fn g(self) -> []int { return match 1 { _ => self.xs } }
    pub fn h(self) -> int { for i, y in self.ys { y[0] = 5 }
        return 0 }
    pub fn k(self) -> fn() -> int { return || self.xs[0] }
    pub fn m(self) -> ([]int, []int) { return self.xs, self.ys[0] }
}
fn pa(p: []int) -> Option<[]int> { return Some(p) }
fn pb(p: []int) -> K { return K { xs: p } }
fn pc(p: [][]int) -> []int { return p[0] }
fn pd(p: []int) -> []int { p }
fn pe(p: [][]int, i: int) -> []int { if i > 0 { return p[1] }
    return arr::first(p).unwrap() }
fn pf(p: [][]int) -> []int { for _, y in p { return y }
    return [] }
fn pg(p: []int) -> int { return p[0] + len(p) }
fn ph(k: K) -> []int { return k.xs }
fn baca(p: [][]int) -> int { return pg(p[1]) + pg(pc(p)) }
pub fn main() {
    var q = Q { xs: [1, 2], ys: [[1]] }
    var a = q.a().unwrap()
    a[0] = 9
    var b = q.b()
    b.0[0] = 9
    var c = q.c(true)
    c[0] = 9
    var d = q.d()
    d[0] = 9
    var e = q.e()
    e[0][0] = 9
    var g = q.g()
    g[0] = 9
    let _ = q.h()
    let k = q.k()
    q.xs[0] = 40
    var m = q.m()
    m.0[1] = 9
    m.1[0] = 9
    wok(q.xs, q.ys, k())
    var x: []int = [1, 2]
    var xx: [][]int = [[1, 2], [3]]
    var ra = pa(x).unwrap()
    ra[0] = 9
    var rb = pb(x)
    rb.xs[0] = 9
    var rc = pc(xx)
    rc[0] = 9
    var rd = pd(x)
    rd[0] = 9
    var re = pe(xx, 0)
    re[0] = 9
    var re2 = pe(xx, 1)
    re2[0] = 9
    var rf = pf(xx)
    rf[0] = 9
    var kk = K { xs: [7, 8] }
    var rh = ph(kk)
    rh[0] = 9
    wok(x, xx, kk.xs, pg(x), baca(xx))
}
"#),
    ("elemen yang diambil bawaan dan elemen for adalah salinan; tumbuh lewat tangkapan closure", r#"package main
struct K { pub xs: []int }
pub fn main() {
    var ys: [][]int = [[1], [2]]
    var a = ys.first().unwrap()
    a[0] = 10
    var c = arr::get(ys, 1).unwrap()
    c[0] = 12
    var d = arr::last(ys).unwrap()
    d[0] = 13
    var e = arr::find(ys, |v: []int| true).unwrap()
    e[0] = 14
    var g = arr::get_or(ys, 0, [0])
    g[0] = 16
    var dflt: []int = [0]
    var g2 = arr::get_or(ys, 9, dflt)
    g2[0] = 16
    var o = Some(ys[0])
    var u = o.unwrap()
    u[0] = 17
    var u2 = o.unwrap_or([0])
    u2[0] = 18
    var kosong: Option<[]int> = None
    var u3 = kosong.unwrap_or(dflt)
    u3[0] = 18
    var dd: dict<[]int> = dict::new()
    dd["a"] = [1]
    var v = dict::get(dd, "a").unwrap()
    v[0] = 19
    for i, y in ys { y[0] = 5 }
    for _, y in ys { var z = y
        z[0] = 6 }
    var jumlah = 0
    for _, y in ys { jumlah += y[0] }
    wok(ys, o, dd, dflt, jumlah)
    var a2: []int = []
    let tambah = |x: int| { a2 = a2 + [x] }
    for i in 0..5 { tambah(i) }
    a2 += [9]
    tambah(7)
    let beku = move || len(a2)
    let awal = move || a2[0]
    let tulis = move |v: int| { a2[0] = v
        return a2[0] }
    a2 = a2 + [1]
    a2[0] = 44
    wok(a2, beku(), awal(), tulis(3), a2[0])
}
"#),
    ("arr::swap: salinan dengan dua elemen ditukar", r#"package main
pub fn main() { let a = [1, 2, 3]
  wok(arr::swap(a, 0, 2)); wok(arr::swap(a, 1, 1)); wok(a); wok(arr::swap(["x", "y"], 1, 0)) }
"#),
    // Fase PETA: dict LLVM jadi tabel hash (primitif C). Yang dikunci: kepemilikan (salinan,
    // parameter, elemen array, iterasi sambil menyisip), urutan tampil, dan 60 ribu kunci acak —
    // ukuran yang dulu menghabiskan memori.
    ("dict tabel hash: alias, urutan, hapus, gabung, banding, 60 ribu kunci", r#"package main
fn isi(d, n) { for i in 0..n { d[str(i * 3 % 17)] = i }
  return d }
fn tambah(d) { d["zz"] = 1
  return len(d) }
pub fn main() {
    var d = {"m": 0}
    for i in 0..30 { d[str((i * 7) % 31)] = i }
    wok(len(d)); wok(d["21"]); wok(dict::has(d, "30")); wok(dict::has(d, "31")); wok(dict::get(d, "7")); wok(dict::get(d, "x"))
    wok(dict::keys(d))
    d["0"] = 100; d["a"] = 5
    wok(d)
    var e = d
    e["baru"] = 1
    for i in 0..40 { e[str(i + 100)] = i }
    wok(len(d)); wok(len(e)); wok(dict::has(d, "baru")); wok(dict::has(e, "139")); wok(dict::has(d, "139"))
    wok(tambah(d)); wok(len(d)); wok(dict::has(d, "zz"))
    let f = isi(d, 20)
    wok(len(f)); wok(len(d)); wok(f["16"]); wok(dict::has(d, "16"))
    var n = 0
    for k, v in d { d[k + "!"] = v
      n = n + 1 }
    wok(n); wok(len(d))
    for k in d { if len(k) > 2 { d = dict::remove(d, k) } }
    wok(dict::keys(d))
    var g = dict::merge(d, {"q": 9, "0": 1})
    g["k"] = 2
    wok(g == d); wok(len(g)); wok(dict::values(g)[0])
    var h = {"x": 1}
    var i2 = {"x": 1}
    h["b"] = 2; h["a"] = 3
    i2["a"] = 3; i2["b"] = 2
    wok(h == i2); wok(h); wok(i2)
    let xs = [h, i2]
    var y = xs[1]
    y["c"] = 0
    wok(xs); wok(y)
    var kosong = dict::remove({"a": 1}, "a")
    kosong["p"] = 1
    wok(kosong)
    var besar = {"s": 0}
    for i in 0..60000 { besar[str(i * 7919 % 100003)] = i }
    var s = 0
    for i in 0..60000 { s = s + besar[str(i * 7919 % 100003)] }
    wok(len(besar)); wok(s); wok(dict::keys(besar)[1]); wok(besar[dict::keys(besar)[4000]])
    besar["zzz"] = 1
    wok(dict::has(besar, "zzz")); wok(dict::has(besar, "99999x")); wok(besar[str(59999 * 7919 % 100003)])
}
"#),
    ("dict: alias lewat fungsi, closure, iterasi, salinan", r#"package main

fn ubah(d) { d["a"] = 99
  d["z"] = 5
  return d["a"] }
fn buat() { var d = {"a": 1}
  return d }
pub fn main() {
    var d = {"a": 1, "b": 2}
    wok(ubah(d)); wok(d)
    var e = d
    e["a"] = 7; e["c"] = 3
    wok(d); wok(e)
    let xs = [d, d]
    var y = xs[0]
    y["b"] = 50
    wok(xs); wok(y)
    var f = buat()
    f["q"] = 1
    wok(f); wok(buat())
    let g = || { d["a"] = 1000 }
    g()
    wok(d)
    for k, v in d { d[k] = v + 1 }
    wok(d)
    var h = dict::remove(d, "a")
    h["a"] = 0
    wok(d); wok(h); wok(dict::merge(d, h)); wok(dict::keys(h)); wok(dict::values(h)); wok(d == e); wok(dict::has(h, "b")); wok(dict::get(h, "zz"))
}
"#),
    ("dict: anotasi `dict`/`dict<V>`, kunci bukan teks, dict::set", r#"package main
pub fn main() { var d: dict<string, int> = {"a": 1}
 var e: dict<int> = {"b": 2}
 var f: dict = {"c": 3}
 wok(d, e, f)
 var g = {1: "x", 20: "y"}
 wok(g); wok(g[1]); wok(g["20"]); wok(dict::has(g, 1)); g[2.5] = "p"; g[true] = "q"
 let r = 'k'
 g[r] = "rune"
 wok(g); wok(dict::get(g, 2.5)); g = dict::set(g, 7, "tujuh"); wok(g); wok(dict::keys(g)) }
"#),
    ("dict telanjang di field struct, dict::set di tempat, dict::get_or", r#"package main
struct Set { pub d: dict }
fn dari(a) { var d = dict::new()
  for _, x in a { d = dict::set(d, x, x) }
  return Set { d: d } }
fn hitung(teks) { var c = dict::new()
  for _, k in str::split(teks, " ") { c = dict::set(c, k, dict::get_or(c, k, 0) + 1) }
  return c }
pub fn main() {
    let s = dari([3, 1, 2, 3, 1])
    wok(dict::len(s.d)); wok(dict::values(s.d)); wok(dict::has(s.d, 2)); wok(dict::has(s.d, 9))
    let t = dari(["b", "a"])
    wok(dict::keys(t.d)); wok(t.d)
    var d = {"a": 1}
    let e = dict::set(d, "b", 2)
    wok(d); wok(e)
    wok(hitung("a b a c a b"))
    var besar = dict::new()
    for i in 0..3000 { besar = dict::set(besar, i * 7 % 3001, i) }
    wok(len(besar)); wok(besar[7])
}
"#),
    ("argumen method berparameter tanpa anotasi dievaluasi SEKALI", r#"package main
struct P { pub n: int }
impl P { pub fn m(self, x) { return x + self.n } }
fn efek() { wok("efek")
 return 1 }
pub fn main() { let p = P { n: 1 }
 wok(p.m(efek())) }
"#),
    ("arr::sort atas elemen bertag: angka campur, teks, stabil", r#"package main
struct K { pub isi: array }
pub fn main() {
    wok(arr::sort(K { isi: [3, 1.5, 2, 2.0, 1] }.isi)); wok(arr::sort(K { isi: ["b", "a", "c"] }.isi))
    wok(arr::sort(K { isi: [true, false] }.isi)); let e: []int = []
    wok(arr::sort(K { isi: e }.isi))
}
"#),
    // Fase BUNGKUS: `array<T>` yang masuk field `array` telanjang dikotak per elemen, dan
    // nilai bertag yang menuju variabel bertipe dibuka dengan pemeriksaan tag.
    ("array bertipe masuk field `array` telanjang, lalu dibaca kembali", r#"package main
struct Kotak { pub isi: array }
fn jumlah(k) { var s = 0
  for _, x in k.isi { s = s + x }
  return s }
pub fn main() {
    let a = [1, 2, 3]
    let k = Kotak { isi: a }
    wok(k.isi); wok(len(k.isi)); wok(k.isi[1]); wok(jumlah(k))
    let t = Kotak { isi: ["a", "b"] }
    wok(t.isi); wok(typeof(t.isi[0])); wok(typeof(k.isi[0]))
    var b = [4.5]
    b = arr::push(b, 2.5)
    let f = Kotak { isi: b }
    wok(f.isi); wok(a)
    var s = 0
    s = k.isi[2]
    wok(s + 1)
    var u = 7
    u = t.isi[0]
    wok(u)
    let e: []int = []
    wok(Kotak { isi: e }.isi)
}
"#),
    // Fase SEPARUH: `Ok`/`Err`/`None` yang separuh tipenya datang dari `return` LAIN di fungsi
    // tanpa anotasi, termasuk di dalam tuple.
    ("separuh tipe: Ok dan Err, Some dan None, tuple berisi None", r#"package main
fn bagi(a, b) {
    if b == 0 { return Err("bagi nol") }
    return Ok(a / b)
}
fn cari(xs, x) {
    if len(xs) == 0 { return None }
    for i, v in xs { if v == x { return Some(i) } }
    return None
}
fn pasang(n) {
    if n < 0 { return (n, Option::None) }
    return (n, Some("ada"))
}
fn ekst(s) {
    match str::index_of(s, ".") { None => return Option::None, Some(i) => if i < 1 { return Option::None }, }
    return Option::Some(str::sub(s, 1, len(s)))
}
pub fn main() {
    wok(bagi(6, 3)); wok(bagi(1, 0))
    match bagi(9, 3) { Ok(v) => wok(v), Err(e) => wok(e) }
    wok(cari([4, 5, 6], 6)); wok(cari([1], 7)); wok(cari([], 1))
    wok(pasang(-1)); wok(pasang(2)); wok(ekst("a.b")); wok(ekst("ab"))
}
"#),
    ("arr:: bentuk namespace: pop, first, last, get, get_or, take, skip", r#"package main
fn pecah(daftar, n) {
    var keluar = []
    var bagian = []
    for i in daftar {
        bagian = arr::push(bagian, daftar[i])
        if bagian.len() == n { keluar = arr::push(keluar, bagian)
            bagian = [] }
    }
    return keluar
}
fn kiri(ps) { var k = []
    for i in ps { let p = ps[i]
        k = arr::push(k, p.0) }
    return k }
pub fn main() {
    let a = [3, 4, 5]
    wok(arr::pop(a)); wok(arr::first(a)); wok(arr::last(a)); wok(arr::get(a, 1)); wok(arr::get(a, 9)); wok(arr::get_or(a, 9, 7))
    let e: []int = []
    wok(arr::pop(e)); wok(arr::last(e))
    wok(arr::take(a, 2)); wok(arr::take(a, 9)); wok(arr::take(a, 0)); wok(arr::skip(a, 1)); wok(arr::skip(a, 9))
    wok(pecah([1, 2, 3, 4], 2)); wok(kiri([(1, "a"), (2, "b")]))
}
"#),
    ("str::join, str::sub, dan `for j, c in teks`", r#"package main
fn potong(p) { var k = []
    for _, b in str::split(p, "/") { if len(b) > 0 { k = arr::push(k, b) } }
    return k }
pub fn main() {
    let n = "abé.cd"
    wok(str::join(["a", "b"], "-")); wok(str::join([], "-")); wok(["x", "y"].join("+")); wok(str::join(potong("/usr//lib/"), "/"))
    wok(str::sub(n, 1, 3)); wok(n.sub(4, len(n))); wok(str::sub(n, 2, 2)); wok(str::substring(n, 0, 2))
    for j, c in n { wok_(j); wok_(" "); wok_(c); wok_(" "); wok(c == '.') }
}
"#),
    // Fase SIMPUL-1: closure tanpa anotasi yang diserahkan ke fungsi tanpa anotasi — tipe
    // parameternya dibaca dari cara BADAN fungsi itu memakainya, termasuk di modul `std/`.
    ("closure tanpa anotasi ke fungsi pemakai: map, filter, lipat, panggil dua kali", r#"package main
fn petakan(daftar, f) { var keluar = []
  for _, x in daftar { keluar = arr::push(keluar, f(x)) }
  return keluar }
fn saring(daftar, pred) { var keluar = []
  for _, x in daftar { if pred(x) { keluar = arr::push(keluar, x) } }
  return keluar }
fn lipat(daftar, awal, f) { var a = awal
  for _, x in daftar { a = f(a, x) }
  return a }
fn dua_kali(f, x) { return f(f(x)) }
pub fn main() {
  let k = 10
  wok(petakan([1, 2, 3], |x| x * 2 + k))
  wok(petakan(["a", "b"], |s| s + "!"))
  wok(saring([1, 2, 3, 4], |x| x % 2 == 0))
  wok(lipat([1, 2, 3], 0, |a, x| a + x))
  wok(dua_kali(|x| x + 3, 4))
}"#),
    // Fase TUMBUH-LLVM: pertumbuhan di tempat tidak boleh membocorkan alias ke nama lain,
    // ke pemanggil, ke global yang sudah disalin, atau antar-putaran loop.
    ("tumbuh di tempat: salinan, parameter, global, loop, closure, a + a", r#"package main
var G = []
fn tumbuh(d, x) { d = d + [x]
  d = arr::push(d, x * 2)
  return d }
fn isi_global(n) { for i in 0..n { G = arr::push(G, i) } }
fn bikin(n) { var k = []
  for i in 0..n { k = arr::push(k, i * i) }
  return k }
pub fn main() {
  var a = [1, 2]
  var b = a
  a = a + [3]
  a += [4, 5]
  b = arr::push(b, 99)
  wok(a, b)
  a = a + a
  wok(a, len(a))
  let c = tumbuh(a, 7)
  wok(len(a), len(c), c[len(c) - 1])
  var d = bikin(5)
  var e = d
  d = arr::push(d, -1)
  e[0] = 42
  wok(d, e)
  for r in 0..3 { var k = []
    for i in 0..r + 2 { k = arr::push(k, r * 10 + i) }
    k = arr::concat(k, [r])
    wok(k) }
  isi_global(4)
  let h = G
  isi_global(2)
  wok(G, h)
  var t = ["a"]
  for i in 0..3 { t = arr::push(t, "s" + str(i)) }
  let tambah = |x| { a = arr::push(a, x) }
  tambah(1000)
  wok(t, a[len(a) - 1], len(a))
  var kosong = []
  kosong = kosong + []
  kosong = arr::push(kosong, true)
  wok(kosong)
}"#),
    // Fase SIMPUL-3: nama yang diikat pola `match` punya tipe di penyimpul (`Some(v) => v`), dan
    // field `array` tanpa tipe elemen disimpan bertag — satu tata letak untuk isi apa pun.
    ("pola match berpengikat di penyimpul, iterator::filter_map, field array telanjang", r#"package main
import "std/iterator"
struct Kotak { pub isi: array, pub n: int = 0 }
impl Kotak {
  fn tambah(self, x) { return Kotak { isi: arr::push(self.isi, x), n: self.n + 1 } }
  fn panjang(self) { return len(self.isi) }
}
fn pilih(d, f) { var k = []
  for _, x in d { match f(x) { Some(v) => { k = arr::push(k, v) }
    None => {} } }
  return k }
fn atau_nol(d, f) { var k = []
  for _, x in d { let r = f(x)
    k = arr::push(k, r ?? 0) }
  return k }
pub fn main() {
  let a = [3, 1, 2]
  wok(pilih(a, |x| x > 1 $ Some(x * 10) : None), atau_nol(a, |x| x > 1 $ Some(x) : None))
  wok(iterator::filter_map(a, |x| x > 1 $ Option::Some(x * 10) : Option::None))
  var k = Kotak { isi: [] }
  k = k.tambah(1)
  k = k.tambah(2)
  wok(k.panjang(), k.n, k.isi, k.isi[0], k)
}"#),
    // Fase SIMPUL-2: fungsi yang mengembalikan `Some`/`None` tanpa anotasi (tata letaknya dibawa
    // pulang dari salinan penduga), `for … in range`, dan parameter-fungsi yang DITERUSKAN.
    ("closure tanpa anotasi: Option dari penduga, range, penerusan, std/iterator fold/find", r#"package main
import "std/iterator"
fn cari(daftar, pred) { for i in daftar { if pred(daftar[i]) { return Some(daftar[i]) } }
  return None }
fn isi(n, f) { var keluar = []
  for _, i in range(0, n) { keluar = arr::push(keluar, f(i)) }
  return keluar }
fn lipat(d, f, awal) { var a = awal
  for _, x in d { a = f(a, x) }
  return a }
fn lipat2(d, f, awal) { return lipat(d, f, awal) }
pub fn main() {
  let a = [3, 1, 2]
  wok(cari(a, |x| x < 3), cari(a, |x| x > 9))
  wok(isi(4, |i| i * i))
  wok(lipat2(a, |s, x| s + x, 0), iterator::fold(a, |s, x| s * x, 1), iterator::reduce(a, |s, x| s + x, 0))
  wok(iterator::find(a, |x| x > 1), iterator::any(a, |x| x == 2), iterator::all(a, |x| x > 0))
}"#),
    ("std/iterator lewat LLVM: map, filter, each dengan closure tanpa anotasi", r#"package main
import "std/iterator"
pub fn main() {
    wok(iterator::map([1, 2], |x| x * 2))
    wok(iterator::filter([1, 2], |x| x > 1))
    wok(typeof(iterator::each([1], |x| x)))
}"#),
    ("panik: pesan dirakit, keluaran sebelumnya utuh — lihat GAGAL_SAAT_JALAN", r#"package main
fn aman(x) { if x < 0 { panik("negatif: " + str::to_string(x)) }
  return x * 2 }
pub fn main() { wok(aman(4)) }"#),
    // Fase LEBAR-LLVM: tepat di batas masih jalan, dan yang sengaja membungkus ditulis.
    ("int 64 bit: tepi yang muat, dan math::wrap_*", r#"package main
pub fn main() {
    var x = 9223372036854775806
    x += 1
    var m = -9223372036854775807
    m = m - 1
    wok(x, m, -x, math::abs(-x), [9223372036854775806, 1].sum())
    var h: uint = 1469598103934665603
    for i in 0..5 { h = math::wrap_mul(h ^ (i as uint), 1099511628211) }
    wok(h, math::wrap_sub(0, 1), math::wrap_add(18446744073709551615, 2))
}"#),
    // Fase DUGA: tipe kembalian fungsi tanpa anotasi dibaca dari penurunan badannya.
    ("kembalian tanpa anotasi: match berpola pengikat, ??, fungsi pustaka", r#"package main
enum E { A(int), B }
fn g(e) { return match e { E::A(n) => n, E::B => -1 } }
fn m(o: Option<int>) { return o ?? 5 }
fn p(x: int) { return math::abs(x) }
pub fn main() { wok(g(E::A(3)), g(E::B), m(Option::Some(1)), m(Option::None), p(-5), typeof(p(-5))) }"#),
    ("kembalian tanpa anotasi: fn bersarang, method, default, tuple, rekursi", r#"package main
var G = 7
struct K { pub v: int = 0 }
impl K {
    fn ambil(self) { return self.v }
    fn pasangan(self) { return (self.v, "k") }
}
fn h(a, b = a + 1) { return b }
fn k(a = G) { return a }
fn luar() { let r = kedua(1); fn kedua(x) { return x + 100 }; return r }
fn fakt(n) { if n < 2 { return 1 }
    return n * fakt(n - 1) }
pub fn main() {
    wok(K { v: 5 }.ambil(), K { v: 2 }.pasangan(), h(3), k(), k(99), luar(), fakt(10))
}"#),
    // Fase PACU-3: literal diperiksa lebarnya, dan `-2^63` tetap sah sebagai operand `-`.
    ("literal tepi 64 bit", "package main\npub fn main() { let x: u64 = 18446744073709551615\n let y: i8 = -128\n wok(-9223372036854775808, 9223372036854775807, x, y) }"),
    // Fase LLVM-17: `self` dulu penunjuk ke milik pemanggil, `wok` mencetak di tengah evaluasi
    // argumennya, tiap nilai fungsi `function`, dan geseran >= lebar adalah poison.
    ("penerima disalin, urutan cetak, typeof closure, geseran", r#"package main
struct P { pub n: int, pub xs: []int }
impl P {
    fn naik(self) -> int { self.n += 1; return self.n }
    fn dua(self) -> int { return self.naik() + self.n }
    fn isi(self) -> P { self.xs[0] = 7; self.n = 3; return self }
    fn baca(self) -> int { return self.n + self.xs[0] }
    fn lewat_closure(self) -> int { let f = || { self.n = 50; return self.n }; return f() + self.n }
}
fn tambah(a: int, b: int) -> int { return a + b }
fn f(n: int) -> int { wok_("[", n, "]"); return n }
pub fn main() {
    var p = P { n: 1, xs: [1, 2] }
    wok(p.naik(), p.n, p.dua(), p.n)
    let q = p.isi()
    wok(p, q, p.baca(), q.baca())
    wok(p.lewat_closure(), p.n)
    wok(f(1), f(2))
    let g = |x: int| x + 1
    let k = 5
    let h = |x: int| x + k
    let t = tambah
    wok(typeof(g), typeof(h), typeof(tambah), typeof(t), g(1), h(1), t(1, 2))
    let n = 70
    let z = 0
    wok(-8 >> n, 8 >> n, z << n, 1 << 3, 256 >> 4, -256 >> 4, 5 << 0)
}"#),
    ("method default trait, hook tampilan darinya, dan teks + agregat", r#"package main
struct B { pub n: int }
impl Bentuk for B { fn luas(self) -> int { return self.n * self.n } }
trait Bentuk {
    fn luas(self) -> int
    fn sapa(self) -> string = "dari trait"
    fn to_string(self) -> string { return "Bentuk luas " + str(self.luas()) }
    fn dobel(self) -> int { return self.luas() * 2 }
}
struct A { pub n: int }
impl Bentuk for A {
    fn luas(self) -> int { return self.n }
    fn sapa(self) -> string { return "milik A" }
}
struct P { pub a: int }
enum E { X(int), Y }
struct V { pub variant: string }
pub fn main() {
    let a = A { n: 3 }
    let b = B { n: 4 }
    wok(a.sapa(), b.sapa(), a.dobel(), b.dobel())
    wok(a, b, [a], "x" + b, a.to_string())
    let p = P { a: 1 }
    wok("x" + p, "x" + [1, 2], "x" + (1, "a"), "x" + E::X(2), "x" + E::Y, [1] + "y", p + "z")
    let v = V { variant: "None" }
    wok(match v { None => "none", _ => "lain" }, match p { Y => "y", _ => "lain" })
}"#),
    // Fase LLVM-16: tanpa anotasi, kembalian ternary/`if … else`/`match` dulu ditebak `()`
    // dan nilainya dibuang tanpa kata — `wok(f(0), f(1))` mencetak `() ()`.
    ("kembalian ternary, if, dan match tanpa anotasi", r#"package main
fn f(n) { return n == 0 $ true : false }
fn g(e) { return match e { 1 => 10, _ => 20 } }
fn h(n) { if n > 0 { "pos" } else { "neg" } }
fn k(n) { return if n > 0 { 1.5 } else { 2.5 } }
pub fn main() {
    wok(f(0), f(1), g(1), g(2), h(1), h(-1), k(1))
}"#),
    // `math::pow` selalu pecahan, dan min/max memakai rumus acuan `a < b $ a : b`.
    ("math: pow selalu pecahan, min dan max seperti acuan", r#"package main
pub fn main() {
    wok(math::pow(2, 10), pow(2, 3), math::pow(2, -1), math::pow(2.0, 0.5))
    wok(math::min(math::NAN, 1.0), math::min(1.0, math::NAN), math::max(math::NAN, 1.0))
    wok(math::min(-0.0, 0.0), math::min(0.0, -0.0), math::max(3, 2), math::min(2.5, 1.5))
}"#),
    // Fase SERPIH: `a, _ = …` dulu ditolak LLVM ("`_` bukan variabel yang bisa dialamati").
    ("pembuang: `_ = f()` dan `a, _ = …` membuang nilainya", r#"package main
fn f() -> int {
    wok("f")
    return 5
}
pub fn main() {
    _ = f()
    var a = 1
    a, _ = 7, f()
    wok(a)
}"#),
    // Fase RUANG: fungsi milik program menang atas bawaan bernama sama, di tiga mesin —
    // dulu LLVM menjawab `abs(-5)` dengan bawaannya dan menolak `len(1, 2, 3)`.
    (
        "fungsi milik program menang atas bawaan bernama sama",
        "package main\nfn len(a, b, c) { return a + b + c }\nfn abs(x) { return 7 }\npub fn main() {\n    wok(len(1, 2, 3), abs(-5), ::len(\"ab\"), ::abs(-5))\n}\n",
    ),
    // Fase TERM: `exit` satu aturan di tiga mesin.
    (
        "exit: kode 0 sesudah keluaran, sisanya tidak jalan",
        "package main\npub fn main() {\n    wok(\"sebelum\")\n    exit(0)\n    wok(\"tidak sampai\")\n}\n",
    ),
    // `()` yang ditulis adalah unit, di tiga mesin (dulu tuple kosong).
    (
        "unit: () yang ditulis sama dengan unit yang dijawab fungsi",
        "package main\nfn f() { }\npub fn main() {\n    wok(typeof(()), f() == (), () == ())\n}\n",
    ),
    // Fase KEMBALIAN: yang tidak menjawab apa-apa menjawab `unit` — LLVM sudah
    // begitu sejak dulu, dua mesin acuan menjawab `nil`.
    (
        "jawa::set dan jawir::free menjawab unit",
        r#"package main
pub fn main() {
    let p = jawa::alloc(1)
    wok(typeof(jawa::set(p, 2)), jawa::get(p))
    let a = jawa::array(2)
    wok(typeof(jawa::array_set(a, 0, 7)), typeof(jawir::free(p)))
    let u = jawa::array_push(a, 1)
    wok(1, u, typeof(u))
}
"#,
    ),
    // Fase KURUNG: penjaganya menyentuh pintu yang dilewati SETIAP pemanggilan
    // method, jadi sisi positifnya dikunci berdampingan.
    (
        "pemanggilan method dan pseudo-field tetap jalan",
        r#"package main
struct P { pub a: int }
impl P { fn dua(self) { return self.a * 2 } }
pub fn main() {
    let p = P { a: 5 }
    wok(p.dua())
    let s = "halo"
    wok(s.len, s.upper())
}
"#,
    ),
    // Fase POLA-FIELD: sisi POSITIF penjaga pola field. Penjaga yang menolak
    // semuanya juga menghijaukan tabel penolakan, jadi batasnya dikunci di sini.
    (
        "pola sebagian field tetap cocok",
        r#"package main
struct P { pub a: int, pub b: int }
pub fn main() {
    let p = P { a: 7, b: 2 }
    match p { P { a } => wok(a), _ => wok("lain") }
    let q = P { a: 8, b: 3 }
    match q { P { a, .. } => wok(a), _ => wok("lain") }
}
"#,
    ),
    // `byte` bilangan 8 bit tak bertanda di ketiga mesin (Fase KONVERSI) —
    // kasus ini dulu satu-satunya isi `DIVERGENSI_LLVM`.
    (
        "byte: bilangan 8 bit, bukan karakter",
        r#"package main
pub fn main() {
    wok(65 as byte)
    wok((65 as byte) == 65)
}"#,
    ),
    // Tipe hasil dari ANOTASI method struct, di fungsi tanpa anotasi dan di
    // closure. Dulu `f` dipancarkan `void`: nilainya dibuang dan `typeof`
    // menjawab `unit` — hasil salah yang diam.
    (
        "method struct: tipe hasil dari anotasinya",
        r#"package main
struct P { pub n: int }
impl P {
    fn dua(self) -> int { return self.n * 2 }
    fn besar(self) -> bool { return self.n > 1 }
}
fn f(t: P) { return t.dua() }
fn g(t: P) { return P::dua(t) }
pub fn main() {
    let xs = [P { n: 1 }, P { n: 2 }]
    wok(f(P { n: 3 }), typeof(f(P { n: 3 })), g(P { n: 4 }))
    wok(xs.map(|t| t.dua()), xs.map(|t: P| t.dua()), xs.filter(|t| t.besar()))
    wok(arr::map(xs, |t| t.dua()), xs.map(|t| P::dua(t)))
}"#,
    ),
    // Nilai FUNGSI di parameter: tipe hasilnya dari tipenya. Dulu `jalankan`
    // ditebak `()` dan `assert_eq(jalankan(..), 3)` di `closures.jrg` LULUS
    // karena `() == 3` dikompilasi jadi `true` — dua cacat saling menutupi.
    (
        "nilai fungsi di parameter: tipe hasil dari tipenya",
        r#"package main
fn jalankan(f) { return f() }
pub fn main() {
    var arg = 1
    wok(jalankan(|| 3), typeof(jalankan(|| 3)))
    assert_eq(jalankan(|| { arg = arg * 3; return arg }), 3)
    wok(arg)
}"#,
    ),
    // `()` lawan nilai lain: `false`, seperti acuan. Dulu literal `6` menyesuaikan
    // diri ke `()` dan `kosong() == 6` jadi konstanta `true`.
    (
        "unit lawan nilai lain: == lintas tipe",
        r#"package main
fn kosong() { }
pub fn main() {
    let u = kosong()
    wok(u == 6, u != 6, kosong() == 6, kosong() == "teks", kosong() != 0)
}"#,
    ),
    // Bentuk path atas method ber-`self`. Di sini penerimanya diketahui saat
    // KOMPILASI, jadi pemeriksaannya gratis dan tepat.
    (
        "path method di llvm",
        r#"package main
struct P { pub n: int }
impl P { pub fn tambah(self, k: int) -> int { return self.n + k } }
pub fn main() {
    let p = P{n: 1}
    wok(P::tambah(p, 4))
    wok(p.tambah(4))
}
"#,
    ),
    (
        "sebar array di llvm",
        r#"package main
pub fn main() {
    let a = [2, 3]
    let b = [1, ...a, 4]
    wok(b)
    wok(len(b))
}
"#,
    ),
    // `"""` — pembatas ASCII untuk template, di backend ketiga.
    (
        "tiga kutip di llvm",
        r#"package main
pub fn main() {
    let n = 7
    wok("""nilai ${n}
baris dua""")
    wok(«lama ${n * 2}»)
    wok("biasa", "")
}"#,
    ),
    // ── `str::format`: aturan template yang sama dengan interpreter ──────
    ("format: dua desimal",       "package main\npub fn main() { wok(str::format(\"{:.2}\", 3.14159)) }"),
    ("format: nol desimal",       "package main\npub fn main() { wok(str::format(\"{:.0}\", 2.7)) }"),
    ("format: int dapat desimal", "package main\npub fn main() { wok(str::format(\"{:.2}\", 5)) }"),
    ("format: campur",           "package main\npub fn main() { wok(str::format(\"{} = {:.3}\", \"pi\", 3.14159265)) }"),
    ("format: {x} tetap teks",   "package main\npub fn main() { wok(str::format(\"{x} {}\", 1)) }"),
    // Closure `move`. Di backend ini tangkapan berupa kotak heap dan `env`
    // menyimpan PENUNJUK ke tiap kotak; `move` menyimpan penunjuk ke kotak
    // BARU berisi salinan. Kedua arah diuji terpisah.
    ("move llvm: berbagi (biasa)",  "package main\npub fn main() { var n = 1\n let f = || n\n n = 9\n wok(f()) }"),
    ("move llvm: snapshot",         "package main\npub fn main() { var n = 1\n let f = move || n\n n = 9\n wok(f()) }"),
    ("move llvm: dalam->luar putus","package main\npub fn main() { var n = 1\n let f = move || { n = 5\n return n }\n wok(f())\n wok(n) }"),
    ("move llvm: mutasi bertahan",  "package main\npub fn main() { var n = 0\n let f = move || { n = n + 1\n return n }\n wok(f())\n wok(f())\n wok(n) }"),
    ("move llvm: dua nama",         "package main\npub fn main() { var a = 1\n var b = 2\n let f = move || a + b\n a = 10\n b = 20\n wok(f()) }"),
    ("move llvm: tanpa tangkapan",  "package main\npub fn main() { let f = move || 7\n wok(f()) }"),

    // Operator yang didefinisikan pengguna.
    ("operator llvm: a + b",   "package main\nstruct V { pub x: int }\nimpl V {\n  fn add(self, o) -> V { return V{x: self.x + o.x} }\n  fn sub(self, o) -> V { return V{x: self.x - o.x} }\n  fn mul(self, o) -> V { return V{x: self.x * o.x} }\n  fn neg(self) -> V { return V{x: 0 - self.x} }\n  fn eq(self, o) -> bool { return self.x == o.x }\n  fn cmp(self, o) -> int { return self.x - o.x }\n  fn to_string(self) -> string { return \"V\" + fmt::sprint(self.x) }\n}\npub fn main() { wok(V{x:1} + V{x:2}) }"),
    ("operator llvm: a - b",   "package main\nstruct V { pub x: int }\nimpl V {\n  fn add(self, o) -> V { return V{x: self.x + o.x} }\n  fn sub(self, o) -> V { return V{x: self.x - o.x} }\n  fn mul(self, o) -> V { return V{x: self.x * o.x} }\n  fn neg(self) -> V { return V{x: 0 - self.x} }\n  fn eq(self, o) -> bool { return self.x == o.x }\n  fn cmp(self, o) -> int { return self.x - o.x }\n  fn to_string(self) -> string { return \"V\" + fmt::sprint(self.x) }\n}\npub fn main() { wok(V{x:5} - V{x:2}) }"),
    ("operator llvm: -a",      "package main\nstruct V { pub x: int }\nimpl V {\n  fn add(self, o) -> V { return V{x: self.x + o.x} }\n  fn sub(self, o) -> V { return V{x: self.x - o.x} }\n  fn mul(self, o) -> V { return V{x: self.x * o.x} }\n  fn neg(self) -> V { return V{x: 0 - self.x} }\n  fn eq(self, o) -> bool { return self.x == o.x }\n  fn cmp(self, o) -> int { return self.x - o.x }\n  fn to_string(self) -> string { return \"V\" + fmt::sprint(self.x) }\n}\npub fn main() { wok(-V{x:7}) }"),
    ("operator llvm: a == b",  "package main\nstruct V { pub x: int }\nimpl V {\n  fn add(self, o) -> V { return V{x: self.x + o.x} }\n  fn sub(self, o) -> V { return V{x: self.x - o.x} }\n  fn mul(self, o) -> V { return V{x: self.x * o.x} }\n  fn neg(self) -> V { return V{x: 0 - self.x} }\n  fn eq(self, o) -> bool { return self.x == o.x }\n  fn cmp(self, o) -> int { return self.x - o.x }\n  fn to_string(self) -> string { return \"V\" + fmt::sprint(self.x) }\n}\npub fn main() { wok(V{x:1} == V{x:1}); wok(V{x:1} == V{x:2}) }"),
    ("operator llvm: a != b",  "package main\nstruct V { pub x: int }\nimpl V {\n  fn add(self, o) -> V { return V{x: self.x + o.x} }\n  fn sub(self, o) -> V { return V{x: self.x - o.x} }\n  fn mul(self, o) -> V { return V{x: self.x * o.x} }\n  fn neg(self) -> V { return V{x: 0 - self.x} }\n  fn eq(self, o) -> bool { return self.x == o.x }\n  fn cmp(self, o) -> int { return self.x - o.x }\n  fn to_string(self) -> string { return \"V\" + fmt::sprint(self.x) }\n}\npub fn main() { wok(V{x:1} != V{x:2}) }"),
    ("operator llvm: a < b",   "package main\nstruct V { pub x: int }\nimpl V {\n  fn add(self, o) -> V { return V{x: self.x + o.x} }\n  fn sub(self, o) -> V { return V{x: self.x - o.x} }\n  fn mul(self, o) -> V { return V{x: self.x * o.x} }\n  fn neg(self) -> V { return V{x: 0 - self.x} }\n  fn eq(self, o) -> bool { return self.x == o.x }\n  fn cmp(self, o) -> int { return self.x - o.x }\n  fn to_string(self) -> string { return \"V\" + fmt::sprint(self.x) }\n}\npub fn main() { wok(V{x:1} < V{x:2}); wok(V{x:3} < V{x:2}) }"),
    ("operator llvm: a >= b",  "package main\nstruct V { pub x: int }\nimpl V {\n  fn add(self, o) -> V { return V{x: self.x + o.x} }\n  fn sub(self, o) -> V { return V{x: self.x - o.x} }\n  fn mul(self, o) -> V { return V{x: self.x * o.x} }\n  fn neg(self) -> V { return V{x: 0 - self.x} }\n  fn eq(self, o) -> bool { return self.x == o.x }\n  fn cmp(self, o) -> int { return self.x - o.x }\n  fn to_string(self) -> string { return \"V\" + fmt::sprint(self.x) }\n}\npub fn main() { wok(V{x:3} >= V{x:3}) }"),
    ("operator llvm: rantai",  "package main\nstruct V { pub x: int }\nimpl V {\n  fn add(self, o) -> V { return V{x: self.x + o.x} }\n  fn sub(self, o) -> V { return V{x: self.x - o.x} }\n  fn mul(self, o) -> V { return V{x: self.x * o.x} }\n  fn neg(self) -> V { return V{x: 0 - self.x} }\n  fn eq(self, o) -> bool { return self.x == o.x }\n  fn cmp(self, o) -> int { return self.x - o.x }\n  fn to_string(self) -> string { return \"V\" + fmt::sprint(self.x) }\n}\npub fn main() { wok(V{x:1} + V{x:2} + V{x:3}) }"),
    ("operator llvm: bawaan tak tersentuh", "package main\nstruct V { pub x: int }\nimpl V {\n  fn add(self, o) -> V { return V{x: self.x + o.x} }\n  fn sub(self, o) -> V { return V{x: self.x - o.x} }\n  fn mul(self, o) -> V { return V{x: self.x * o.x} }\n  fn neg(self) -> V { return V{x: 0 - self.x} }\n  fn eq(self, o) -> bool { return self.x == o.x }\n  fn cmp(self, o) -> int { return self.x - o.x }\n  fn to_string(self) -> string { return \"V\" + fmt::sprint(self.x) }\n}\npub fn main() { wok(1 + 2); wok(-5); wok(1 < 2) }"),
    ("mut llvm: var",          "package main\npub fn main() { var x = 1\n x = 2\n wok(x) }"),
    ("mut llvm: let mut",      "package main\npub fn main() { let mut x = 1\n x = 2\n wok(x) }"),

    // Hook tampilan `to_string`.
    ("hook llvm: wok",            "package main\nstruct P { pub n: int }\nimpl P { fn to_string(self) -> string { return \"P#\" + fmt::sprint(self.n) } }\npub fn main() { wok(P{n:1}) }"),
    ("hook llvm: wok_",           "package main\nstruct P { pub n: int }\nimpl P { fn to_string(self) -> string { return \"P#\" + fmt::sprint(self.n) } }\npub fn main() { wok_(P{n:1}); wok(\"\") }"),
    ("hook llvm: fmt::sprint",    "package main\nstruct P { pub n: int }\nimpl P { fn to_string(self) -> string { return \"P#\" + fmt::sprint(self.n) } }\npub fn main() { wok(fmt::sprint(P{n:1})) }"),
    ("hook llvm: dalam array",    "package main\nstruct P { pub n: int }\nimpl P { fn to_string(self) -> string { return \"P#\" + fmt::sprint(self.n) } }\npub fn main() { wok([P{n:1}, P{n:2}]) }"),
    ("hook llvm: dalam tuple",    "package main\nstruct P { pub n: int }\nimpl P { fn to_string(self) -> string { return \"P#\" + fmt::sprint(self.n) } }\npub fn main() { wok((P{n:1}, 7)) }"),
    ("hook llvm: di field",       "package main\nstruct P { pub n: int }\nimpl P { fn to_string(self) -> string { return \"P#\" + fmt::sprint(self.n) } }\nstruct Q { pub p: P }\npub fn main() { wok(Q{p: P{n:1}}) }"),
    ("hook llvm: eksplisit",      "package main\nstruct P { pub n: int }\nimpl P { fn to_string(self) -> string { return \"P#\" + fmt::sprint(self.n) } }\npub fn main() { wok(P{n:1}.to_string()) }"),
    ("hook llvm: tanpa hook",     "package main\nstruct P { pub n: int }\nimpl P { fn to_string(self) -> string { return \"P#\" + fmt::sprint(self.n) } }\nstruct R { pub m: int }\npub fn main() { wok(R{m:2}) }"),
    ("hook llvm: typeof tetap",   "package main\nstruct P { pub n: int }\nimpl P { fn to_string(self) -> string { return \"P#\" + fmt::sprint(self.n) } }\npub fn main() { wok(typeof(P{n:1})) }"),

    // `let … else` dan parameter pengumpul.
    ("let else dan parameter pengumpul", r#"package main
fn satu(o: Option<int>) -> int {
    let Some(x) = o else { return -1 }
    return x * 10
}
fn banyak(o: Option<(int, int)>) -> int {
    let Some((a, b)) = o else { return -1 }
    return a + b
}
fn jumlah(xs...) -> int {
    var t = 0
    for i in xs { t = t + xs[i] }
    return t
}
pub fn main() {
    wok(satu(Some(3)), satu(None))
    wok(banyak(Some((3, 4))), banyak(None))
    wok(jumlah(1, 2, 3), jumlah())
    wok(match 5 { x => x }, match 9 { n @ _ => n })
}"#),

    // ── pola pengikat, koalesensi, dan varian unit telanjang ────────────
    ("pola pengikat dan koalesensi", r#"package main
enum E { A(int), B }
var n = 0
fn mahal() -> int { n = n + 1; return 9 }
pub fn main() {
    wok(match 5 { m @ 1..=9 => m * 10, _ => 0 })
    wok(match 50 { m @ 1..=9 => m * 10, _ => 0 })
    let o: Option<int> = Some(3)
    wok(match o { None => 0, Some(v) => v })
    let e = E::A(3)
    wok(match e { B => 0, E::A(v) => v })

    let k: Option<int> = None
    let b: Option<int> = None
    let c: Option<int> = Some(7)
    wok(o ?? 9, k ?? 9, k ?? b ?? c ?? 0)

    wok(o ?? mahal(), n)
    wok(o.unwrap_or(mahal()), n)
}"#),

    // ── sintaks yang diturunkan di parser ───────────────────────────────
    ("if let dan while let", r#"package main
pub fn main() {
    let o = Some(3)
    if let Some(x) = o { wok("ada", x) }
    let k: Option<int> = None
    if let Some(x) = k { wok(x) } else { wok("kosong") }
    wok(if let Some(v) = o { v } else { 0 })
    if let [a, b] = [1, 2] { wok(a, b) }

    var n = 0
    while let Some(x) = Some(n) {
        n = n + 1
        if x > 2 { break }
        wok("n", x)
    }
}"#),
    ("penugasan jamak dan for klasik `:=`", r#"package main
pub fn main() {
    var a = 1
    var b = 2
    a, b = b, a
    wok(a, b)
    c, d := 5, 6
    wok(c, d)
    for i := 0; i < 3; i = i + 1 { wok(i) }
}"#),
    ("else boleh memimpin barisnya sendiri", r#"package main
pub fn main() {
    if 1 > 2 { wok("a") }
    else if 1 > 0 { wok("b") }
    else { wok("c") }
}"#),

    // ── closure ─────────────────────────────────────────────────────────
    ("closure: tangkapan bermutasi terlihat dua arah", r#"package main
pub fn main() {
    var x = 1
    let ubah = || { x = 2 }
    wok(x)
    ubah()
    wok(x)
}"#),
    ("closure: dua closure berbagi satu pengikatan", r#"package main
pub fn main() {
    var n = 0
    let naik = || { n = n + 1 }
    let lihat = || n
    naik(); naik(); naik()
    wok(n, lihat())
}"#),
    ("closure: lolos keluar dari bingkai asalnya", r#"package main
fn bikin() -> fn() -> int {
    var n = 0
    return || { n = n + 1; return n }
}
pub fn main() {
    let c = bikin()
    wok(c(), c(), c())
    let d = bikin()
    wok(d())
}"#),
    ("closure: tiap putaran loop mengikat ulang", r#"package main
pub fn main() {
    var fs = []
    for i in 0..3 { fs = fs + [|| i] }
    wok(fs[0](), fs[1](), fs[2]())
}"#),
    ("closure: tipe parameter dibaca dari situs pemanggilan", r#"package main
pub fn main() {
    var p = 1
    let byparam = |p| p
    wok(byparam(5), p)
    let dua = |a, b| a + b
    wok(dua(3, 4))
}"#),
    ("closure: menangkap var tingkat modul", r#"package main
var g = 0
pub fn main() {
    let gn = || { g = g + 1; return g }
    wok(gn(), gn(), g)
}"#),
    // Fungsi BERSARANG bukan closure: ia tidak melihat lokal fungsi luarnya
    // (diukur — acuannya menolak dengan "variabel belum dikenal") dan di-hoist …
    ("fungsi bersarang: hoist per blok dan saling panggil", r#"package main
fn luar() -> int {
    wok(a(2))
    fn a(x: int) -> int { return b(x) + 1 }
    fn b(x: int) -> int { return x * 10 }
    return a(3)
}
pub fn main() {
    wok(luar())
    fn dalam(x: int) -> int { return x * 3 }
    wok(dalam(2))
    if true {
        fn blok(x: int) -> int { return x + 100 }
        wok(blok(1))
    }
}"#),
    ("fungsi bersarang: dimonomorfisasi per tipe argumen", r#"package main
pub fn main() {
    fn ganda(x) { return x + x }
    wok(ganda(3), ganda("ab"))
}"#),

    // ── jawa:: / jawir:: ────────────────────────────────────────────────
    ("jawa: sel tunggal, array, buffer, dan handle salinan", r#"package main
pub fn main() {
    let p = jawa::alloc(42)
    wok(jawa::get(p), jawa::is_valid(p), typeof(p))
    jawa::set(p, 99)
    wok(jawa::get(p))
    jawir::free(p)
    wok(jawa::is_valid(p))

    let a = jawa::array(3)
    jawa::array_set(a, 0, 10)
    let a2 = jawa::array_push(a, 30)
    wok(jawa::array_len(a), jawa::array_get(a, 0), jawa::array_get(a, 3), a2)

    let b = jawa::buffer(4)
    jawa::buffer_set(b, 0, 255)
    wok(jawa::buffer_len(b), jawa::buffer_get(b, 0))

    let asli = jawa::alloc(1)
    let salinan = asli
    jawa::set(salinan, 123)
    wok(jawa::get(asli))
    jawir::free(salinan)
    wok(jawa::is_valid(asli), jawa::is_valid(salinan))
}"#),

    ("fungsi bernama sebagai nilai", r#"package main
fn tambah(a: int, b: int) -> int { return a + b }
fn kali(a: int, b: int) -> int { return a * b }
fn pakai(f: fn(int, int) -> int, x: int, y: int) -> int { return f(x, y) }
pub fn main() {
    let f = tambah
    var g = tambah
    g = kali
    wok(f(2, 3), g(2, 3))
    wok(pakai(tambah, 4, 5), pakai(kali, 4, 5))
}"#),

    // ── defer ───────────────────────────────────────────────────────────
    ("defer: urutan LIFO dan pembekuan argumen", r#"package main
fn urutan() {
    defer wok("A")
    defer wok("B")
    defer wok("C")
    wok("badan")
}
fn beku() {
    var x = 1
    defer wok("beku:", x)
    x = 99
    wok("akhir:", x)
}
pub fn main() { urutan(); beku() }"#),
    ("defer: lingkup fungsi, bukan blok", r#"package main
fn dalam_blok() {
    if true {
        defer wok("dalam if")
        wok("di if")
    }
    wok("sesudah if")
}
fn dalam_loop() {
    for i in 0..3 { defer wok("loop", i) }
    wok("sesudah loop")
}
pub fn main() { dalam_blok(); dalam_loop() }"#),
    ("defer: fungsi yang dipanggil dibekukan saat didaftarkan",
     "package main\nfn a() { wok(\"a\") }\nfn b() { wok(\"b\") }\npub fn main() { var f = a\n defer f()\n f = b\n defer f()\n wok(\"badan\") }"),
    ("defer: tetap jalan lewat return lebih awal", r#"package main
fn dg_return() -> int {
    defer wok("defer jalan")
    return 7
}
fn kondisional(c: bool) {
    if c { defer wok("hanya kalau true") }
    wok("selesai", c)
}
pub fn main() {
    wok(dg_return())
    kondisional(true)
    kondisional(false)
}"#),

    // ── method tingkat-tinggi ───────────────────────────────────────────
    ("hof: map filter reduce find any all", r#"package main
pub fn main() {
    let a = [1, 2, 3]
    wok(a.map(|x| x * 2))
    wok(a.filter(|x| x > 1))
    wok(a.reduce(|acc, x| acc + x, 0))
    wok(a.any(|x| x > 2), a.all(|x| x > 0))
    wok(a.find(|x| x > 1))
    wok(a.find(|x| x > 9))
    wok(a.map(|x| x * 2).filter(|x| x > 2))
    let s = ["a", "bb"]
    wok(s.map(|x| x.len()))
}"#),

    // ── parameter default ───────────────────────────────────────────────
    ("parameter default dievaluasi di bingkai callee", r#"package main
fn a(x: int, y: int = 10) -> int { return x + y }
fn b(x: int, y: int = x * 2) -> int { return x + y }
var hit = 0
fn efek() -> int { hit = hit + 1; return 100 }
fn c(x: int, y: int = efek()) -> int { return x + y }
pub fn main() {
    wok(a(5), a(5, 1))
    wok(b(3), b(3, 1))
    wok(c(1), c(1, 2), c(1))
    wok(hit)
}"#),

    // ── var tingkat modul ───────────────────────────────────────────────
    ("var tingkat modul: urutan inisialisasi dan mutasi", r#"package main
var n = 0
var pesan = "halo"
var xs = [1, 2, 3]
var turunan = n + 5
fn naik() { n = n + 1 }
pub fn main() {
    wok(n, pesan, xs, turunan)
    naik(); naik()
    wok(n)
    pesan = pesan + " dunia"
    xs = xs + [4]
    wok(pesan, xs)
}"#),

    // ── range ───────────────────────────────────────────────────────────
    ("range: satu, dua, dan tiga argumen", r#"package main
pub fn main() {
    wok(range(3))
    wok(range(1, 4))
    wok(range(0, 10, 3))
    wok(range(0))
    wok(typeof(range(3)))
}"#),

    // ── aliasing: penunjuk DI DALAM agregat ─────────────────────────────
    ("aliasing: slice di dalam struct ikut disalin", r#"package main
struct Kotak { pub isi: []int }
pub fn main() {
    var k1 = Kotak { isi: [9, 9] }
    var k2 = k1
    k2.isi[0] = 88
    wok(k1.isi[0], k2.isi[0])
}"#),
    ("aliasing: array di dalam array ikut disalin", r#"package main
pub fn main() {
    var a1 = [[1], [2]]
    var a2 = a1
    a2[0] = a2[0] + [3]
    wok(a1[0], a2[0])
}"#),

    // ── method string ───────────────────────────────────────────────────
    ("method string: split, contains, ubah huruf", r#"package main
pub fn main() {
    wok("a,b,c".split(","))
    wok("abc".split(""))
    wok("a,,b".split(","))
    wok("a,b,c".split(",").len())
    wok("halo".to_uppercase(), "HALO".to_lowercase())
    wok("halo".contains("al"), "halo".contains("z"))
    wok("halo".starts_with("ha"), "halo".ends_with("lo"))
    wok("a-b".replace("-", "+"))
    wok("  x  ".trim())
}"#),

    // ── nama internal tidak boleh bocor ─────────────────────────────────
    ("nama enum termangel tidak bocor ke keluaran", r#"package main
enum Warna { Merah, Biru }
pub fn main() {
    let o: Option<int> = Some(3)
    wok(o, typeof(o))
    let w = Warna::Merah
    wok(w, typeof(w))
    let r: Result<int, string> = Ok(1)
    wok(r, typeof(r))
}"#),

    // ── dict ────────────────────────────────────────────────────────────
    ("dict: literal, baca, dan cetak", r#"package main
pub fn main() {
    let d = {"b": 2, "a": 1, "c": 3}
    wok(d)
    wok(d["a"], d["b"], d["c"])
    wok(typeof(d), len(d))
}"#),
    ("dict: kunci kembar, yang terakhir menang", r#"package main
pub fn main() {
    wok({"a": 1, "b": 2, "a": 9})
}"#),
    ("dict: namespace dict::", r#"package main
pub fn main() {
    let d = {"a": 1, "b": 2}
    wok(dict::len(d), dict::is_empty(d))
    wok(dict::has(d, "a"), dict::has(d, "z"))
    wok(dict::keys(d), dict::values(d))
    wok(dict::get(d, "a").unwrap())
    wok(dict::remove(d, "a"))
    wok(dict::merge(d, {"z": 9}))
}"#),
    ("dict: tulis lewat indeks, tumbuh dan menimpa", r#"package main
pub fn main() {
    var d = {"a": 1}
    d["b"] = 2
    d["a"] = 9
    wok(d, len(d))
}"#),
    ("dict: iterasi menyebut KUNCI, bukan indeks", r#"package main
pub fn main() {
    let d = {"x": 10, "y": 20}
    for k in d { wok(k, d[k]) }
    for k, v in d { wok(k, v) }
}"#),
    ("dict: kesamaan tidak bergantung urutan penulisan", r#"package main
pub fn main() {
    wok({"a": 1, "b": 2} == {"b": 2, "a": 1})
    wok({"a": 1} == {"a": 2}, {"a": 1} == {"a": 1, "b": 2})
    wok({"a": 1} != {"a": 2})
}"#),
    ("dict: nilai array, panjang boleh beda", r#"package main
pub fn main() {
    let d = {"a": [1, 2], "b": [3]}
    wok(d)
    wok(d["a"], d["b"])
}"#),
    ("dict: penyerahan menyalin, bukan berbagi", r#"package main
pub fn main() {
    var a = {"k": 1}
    var b = a
    b["k"] = 99
    b["baru"] = 5
    wok(a, b)
}"#),
    ("aritmatika int", r#"package main
pub fn main() {
    var a: int = 17
    var b: int = 5
    wok(a + b, a - b, a * b, a / b, a % b)
}"#),

    ("presedensi dan tanda", r#"package main
pub fn main() {
    wok(2 + 3 * 4, (2 + 3) * 4, -7 / 2, -7 % 2)
}"#),

    ("float dibagi dan dicetak", r#"package main
pub fn main() {
    var x: f64 = 7.0
    var y: f64 = 2.0
    wok(x / y, x * y, x - y)
}"#),

    ("bool dan operator logika", r#"package main
pub fn main() {
    var t: bool = true
    var f: bool = false
    wok(t && f, t || f, !t)
}"#),

    ("perbandingan", r#"package main
pub fn main() {
    var a: int = 3
    var b: int = 9
    wok(a < b, a > b, a <= b, a >= b, a == b, a != b)
}"#),

    ("if else berantai", r#"package main
fn kelas(n: int) -> int {
    if n < 10 { return 1 }
    if n < 100 { return 2 }
    return 3
}
pub fn main() {
    wok(kelas(5), kelas(50), kelas(500))
}"#),

    ("while dan akumulator", r#"package main
pub fn main() {
    var i: int = 0
    var s: int = 0
    while i < 10 { s = s + i; i = i + 1 }
    wok(s)
}"#),

    ("for range", r#"package main
pub fn main() {
    var s: int = 0
    for i in 0..5 { s = s + i }
    wok(s)
}"#),

    ("rekursi fib", r#"package main
fn fib(n: int) -> int {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}
pub fn main() { wok(fib(15)) }"#),

    ("break dan continue", r#"package main
pub fn main() {
    var s: int = 0
    var i: int = 0
    while i < 20 {
        i = i + 1
        if i % 2 == 0 { continue }
        if i > 11 { break }
        s = s + i
    }
    wok(s, i)
}"#),

    ("array indeks dan len", r#"package main
pub fn main() {
    var a: [4]int = [10, 20, 30, 40]
    wok(a[0], a[3], len(a))
}"#),

    ("array ditulis di tempat", r#"package main
pub fn main() {
    var a: [3]int = [1, 2, 3]
    a[1] = 99
    wok(a[0], a[1], a[2])
}"#),

    ("string dan cetak", r#"package main
pub fn main() {
    var s: string = "halo"
    wok(s, len(s))
}"#),

    ("struct dan field", r#"package main
struct Titik { pub x: int, pub y: int }
pub fn main() {
    var p: Titik = Titik { x: 3, y: 4 }
    wok(p.x, p.y)
}"#),

    ("method dengan self", r#"package main
struct Kotak { pub w: int, pub h: int }
impl Kotak {
    fn luas(self) -> int { return self.w * self.h }
}
pub fn main() {
    var k: Kotak = Kotak { w: 3, h: 5 }
    wok(k.luas())
}"#),

    ("enum dan match", r#"package main
enum Warna { Merah, Hijau, Biru }
fn kode(w: Warna) -> int {
    match w {
        Warna::Merah => { return 1 }
        Warna::Hijau => { return 2 }
        Warna::Biru  => { return 3 }
    }
}
pub fn main() {
    wok(kode(Warna::Merah), kode(Warna::Hijau), kode(Warna::Biru))
}"#),

    ("match pada int dengan default", r#"package main
fn nama(n: int) -> int {
    match n {
        0 => { return 100 }
        1 => { return 200 }
        _ => { return 900 }
    }
}
pub fn main() { wok(nama(0), nama(1), nama(7)) }"#),

    ("fungsi banyak argumen", r#"package main
fn campur(a: int, b: int, c: int) -> int { return a * 100 + b * 10 + c }
pub fn main() { wok(campur(1, 2, 3)) }"#),

    ("cast int ke float dan balik", r#"package main
pub fn main() {
    var i: int = 7
    var f: f64 = i as f64 / 2.0
    wok(f, f as int)
}"#),

    ("operator bit", r#"package main
pub fn main() {
    var a: int = 12
    var b: int = 10
    wok(a & b, a | b, a ^ b, a << 2, a >> 2)
}"#),

    ("ternary dolar", r#"package main
pub fn main() {
    var n: int = 7
    wok(n > 5 $ 111 : 222)
}"#),

    ("loop bersarang", r#"package main
pub fn main() {
    var s: int = 0
    var i: int = 0
    while i < 4 {
        var j: int = 0
        while j < 4 { s = s + i * j; j = j + 1 }
        i = i + 1
    }
    wok(s)
}"#),

    // ── truthiness ─────────────────────────────────────────────────────
    ("truthiness sebagai kondisi", r#"package main
pub fn main() {
    if 2 { wok("dua") } else { wok("SALAH") }
    var n: int = 5
    if n { wok("n") } else { wok("SALAH") }
    var z: int = 0
    if z { wok("SALAH") } else { wok("nol") }
    if "x" { wok("isi") } else { wok("SALAH") }
    if "" { wok("SALAH") } else { wok("kosong") }
}"#),

    ("konversi bool eksplisit", r#"package main
pub fn main() {
    wok(bool(0), bool(1), bool(-5), bool(0.0), bool(1.5))
    wok(bool(""), bool("x"), bool("0"), bool(true), bool(false))
}"#),

    // NaN falsy — `fcmp one`, bukan `une`. Perbedaan satu huruf yang cuma
    // kelihatan kalau NaN benar-benar diuji, dan sudah pernah menyimpang
    // antara interpreter dan VM.
    ("NaN falsy, inf truthy", r#"package main
pub fn main() {
    var n: f64 = math::NAN
    var i: f64 = math::INF
    wok(bool(n), bool(i))
    if n { wok("SALAH") } else { wok("NaN falsy") }
}"#),

    ("konstanta math lengkap", r#"package main
pub fn main() {
    wok(math::PI, math::E, math::TAU, math::INF, math::NAN)
}"#),

    // ── aturan indeks ──────────────────────────────────────────────────
    ("indeks negatif pada array, baca dan tulis", r#"package main
pub fn main() {
    var a: [4]int = [10, 20, 30, 40]
    wok(a[-1], a[-4], a[0], a[3])
    a[-1] = 99
    a[-4] = 11
    wok(a[3], a[0])
}"#),

    ("indeks negatif pada slice", r#"package main
fn ambil(a: array, i: int) -> int { return a[i] }
pub fn main() {
    var a: [5]int = [1, 2, 3, 4, 5]
    wok(ambil(a[0:4], -1), ambil(a[1:4], -2))
}"#),

    // Elemen tuple bertipe beda-beda, jadi indeksnya wajib konstan di
    // backend statis. Yang diuji di sini bahwa yang konstan BEKERJA dan
    // jawabannya sama.
    ("indeks tuple termasuk negatif", r#"package main
pub fn main() {
    var t: (int, string, bool) = (1, "dua", true)
    wok(t[0], t[1], t[2], t[-1], t[-3])
}"#),

    // Indeks string menghitung KARAKTER, bukan byte. Kalau per byte, `u[1]`
    // akan mengembalikan separuh karakter — sebuah rune yang bukan karakter
    // apa pun, tanpa error.
    ("indeks string per karakter, bukan byte", r#"package main
pub fn main() {
    var s: string = "halo"
    wok(s[0], s[3], s[-1])
    var u: string = "aé漢z"
    wok(len(u), u[0], u[1], u[2], u[3], u[-1])
}"#),

    // ── rune sebagai tipe tersendiri ───────────────────────────────────
    ("rune dicetak sebagai karakter", r#"package main
pub fn main() {
    var c: rune = 'h'
    wok(c, typeof(c), c as int)
    wok(65 as rune, typeof(65 as rune))
    wok('a' < 'b', 'a' == 'a', 'b' > 'a')
    var u: rune = '漢'
    wok(u, u as int)
}"#),

    // ── string: konkatenasi, pengulangan, slicing ──────────────────────
    ("konkatenasi string dua arah", r#"package main
pub fn main() {
    wok("n=" + 5, "f=" + 1.5, "b=" + true, "r=" + 'h')
    wok(5 + "=n", "a" + "b")
}"#),

    ("pengulangan string dua arah", r#"package main
pub fn main() {
    wok("ab" * 3, "ab" * 0, 3 * "ab")
}"#),

    // Slicing string menghitung KARAKTER. Kalau per byte, `u[1:3]` akan
    // memotong di tengah karakter dan menghasilkan byte yang bukan UTF-8
    // sah — string rusak, tanpa error.
    ("slicing string per karakter", r#"package main
pub fn main() {
    var s: string = "halo dunia"
    wok(s[0:4], s[5:], s[:4], s[-5:])
    var u: string = "aé漢z"
    wok(u[1:3], len(u[1:3]), u[:2], u[2:])
}"#),

    // Batas slice negatif — aturan yang sama dengan indeks, dan sebelumnya
    // dipanikkan di backend ini persis seperti indeks dulu.
    ("batas slice negatif pada array", r#"package main
fn cetak(a: array) { for i in 0..len(a) { wok_(a[i]) }; wok("") }
pub fn main() {
    var a: [5]int = [1, 2, 3, 4, 5]
    cetak(a[-3:])
    cetak(a[:-2])
    cetak(a[1:-1])
}"#),

    ("konkatenasi array", r#"package main
pub fn main() {
    var a: [2]int = [1, 2]
    var b: [2]int = [3, 4]
    var c = a + b
    wok(len(c), c[0], c[1], c[2], c[3])
    var s1: [2]string = ["a", "b"]
    var s2: [1]string = ["c"]
    var s3 = s1 + s2
    wok(len(s3), s3[0], s3[2])
}"#),

    // ── inferensi tipe ─────────────────────────────────────────────────
    ("parameter tanpa anotasi, banyak tipe", r#"package main
fn cetak(v) { wok(v) }
fn gandakan(x) -> int { return x * 2 }
pub fn main() {
    cetak(7)
    cetak("teks")
    cetak(1.5)
    cetak(true)
    wok(gandakan(21))
}"#),

    ("tipe return disimpulkan dari badan", r#"package main
fn bikin_array() {
    var lokal = [1, 2, 3]
    return lokal
}
fn jumlah(a: int, b: int) { return a + b }
fn nama() { return "jrg" }
fn ubah_dalam(a) { a[0][0] = 777; return a }
pub fn main() {
    var u = bikin_array()
    u[0] = 999
    wok(u[0], u[1], jumlah(2, 3), nama())
    var c: [2][2]int = [[1, 2], [3, 4]]
    let hasil = ubah_dalam(c)
    wok(hasil[0][0], c[0][0])
}"#),

    ("parameter method tanpa anotasi", r#"package main
struct K { pub n: int }
impl K {
    fn tambah(self, x) -> int { return self.n + x }
    fn gabung(self, s) -> string { return s + "!" }
}
pub fn main() {
    var k: K = K { n: 1 }
    wok(k.tambah(2), k.gabung("hai"))
}"#),

    // `match` tanpa konteks tipe: sebelumnya dianggap tak bernilai, jadi
    // `let z = if c { match .. } else { 5 }` ditolak dengan menuduh `if`-nya.
    ("match menyimpulkan tipe dari arm-nya", r#"package main
pub fn main() {
    let z = if true { match 1 { 1 => 111, _ => 0 } } else { 5 }
    wok(z)
    let w = match 2 { 1 => "satu", _ => "lain" }
    wok(w)
}"#),

    // ── pola array ─────────────────────────────────────────────────────
    ("pola array di match", r#"package main
fn klas(a: array) -> string {
    return match a {
        []           => "kosong",
        [x]          => "satu:" + x,
        [x, y]       => "dua:" + x + "," + y,
        [k, ...sisa] => "kepala:" + k + " sisa:" + len(sisa),
    }
}
pub fn main() {
    var e: [0]int = []
    var s: [1]int = [7]
    var d: [2]int = [1, 2]
    var b: [4]int = [1, 2, 3, 4]
    wok(klas(e), klas(s), klas(d), klas(b))
    match b { [p, ..., z] => wok("ujung:", p, z), _ => wok("?") }
    wok(match [1, 2, 3] { [k, ...] => k, _ => -1 })
    wok(match [[1, 2], [3, 4]] { [[a, b], [c, d]] => a + b + c + d, _ => -1 })
    wok(match [1, 99, 3] { [1, _, 3] => "cocok", _ => "tidak" })
    wok(match [2, 99, 3] { [1, _, 3] => "cocok", _ => "tidak" })
}"#),

    ("pola array di let", r#"package main
pub fn main() {
    let [m, n] = [10, 20]
    wok(m, n)
    let [h, ...t] = [1, 2, 3]
    wok(h, len(t), t)
}"#),

    // ── `nil` ──────────────────────────────────────────────────────────
    ("nil: cetak, banding, truthiness", r#"package main
pub fn main() {
    wok(typeof(nil), «${nil}», sprint(nil), nil)
    wok(nil == nil, nil != nil, nil == 1, 1 == nil)
    wok(bool(nil))
}"#),

    // ── template literal ───────────────────────────────────────────────
    ("template literal untuk semua tipe", r#"package main
struct P { pub x: int, pub y: string }
enum W { A, B }
pub fn main() {
    var n: int = 42
    var f: f64 = 1.5
    var b: bool = true
    var s: string = "dunia"
    var r: rune = 'z'
    var a: [3]int = [1, 2, 3]
    var t: (int, string) = (7, "x")
    var p: P = P { x: 1, y: "q" }
    wok(«halo ${s}, n=${n}»)
    wok(«float ${f} bool ${b} rune ${r}»)
    wok(«array ${a} slice ${a[0:2]} tuple ${t}»)
    wok(«struct ${p} enum ${W::A}»)
    wok(«hitung ${n * 2 + 1} dan ${n > 10}»)
    wok(«bersarang ${«dalam ${n}»}»)
    wok(«»)
}"#),

    // ── namespace `arr::` ──────────────────────────────────────────────
    ("arr:: mengembalikan nilai baru", r#"package main
pub fn main() {
    var a: [5]int = [3, 1, 4, 1, 5]
    wok(arr::push(a, 9))
    wok(arr::sum(a), arr::min(a), arr::max(a))
    wok(arr::reverse(a), arr::sort(a))
    wok(arr::contains(a, 4), arr::contains(a, 7))
    wok("asli:", a)
    var k: [0]int = []
    wok(arr::sum(k), arr::reverse(k), arr::contains(k, 1))
}"#),

    ("arr:: pada float dan string", r#"package main
pub fn main() {
    var f: [3]f64 = [2.5, 1.5, 3.0]
    wok(arr::sum(f), arr::min(f), arr::max(f), arr::sort(f))
    var s: [3]string = ["c", "a", "b"]
    wok(arr::sort(s), arr::contains(s, "a"), arr::reverse(s))
}"#),

    ("arr::push menumbuhkan di dalam loop", r#"package main
pub fn main() {
    var t = []
    for i in 0..5 { t = arr::push(t, i * i) }
    wok(len(t), t, arr::sum(t))
}"#),

    // ── array bertumbuh ────────────────────────────────────────────────
    ("kumpulkan hasil di dalam loop", r#"package main
pub fn main() {
    var t = []
    for i in 0..5 { t = t + [i] }
    wok(len(t), t.len(), t[0], t[4], t == [0, 1, 2, 3, 4])
}"#),

    ("kumpulkan tuple, dan di dalam if/match", r#"package main
pub fn main() {
    var a = []
    for i in 0..3 { a = a + [(i, i * 2)] }
    var b = []
    for i in 0..5 { if i > 1 { b = b + [i] } }
    var c = []
    for i in 0..5 { match i { 3 => break, _ => { c = c + [i] } } }
    wok(len(a), len(b), len(c))
}"#),

    // Lokal BERANOTASI tidak ikut berubah — kalau seseorang menulis
    // `var a: [2000]int`, ia meminta array berukuran tetap, dan mengubahnya
    // diam-diam jadi heap akan memperlambat kode yang sudah benar.
    ("array beranotasi tetap berukuran tetap", r#"package main
pub fn main() {
    var a: [3]int = [1, 2, 3]
    wok(len(a), a.len(), a[0])
}"#),

    // ── break/continue berlabel ────────────────────────────────────────
    ("break dan continue berlabel", r#"package main
pub fn main() {
    var a = []
    'luar: for i in 0..3 {
        for j in 0..3 {
            if i == 1 && j == 1 { break 'luar }
            a = a + [(i, j)]
        }
    }
    var b = []
    'lain: for i in 0..3 {
        for j in 0..3 {
            if j == 1 { continue 'lain }
            b = b + [(i, j)]
        }
    }
    var n = 0
    'w: while true { n = n + 1; if n > 2 { break 'w } }
    var m = 0
    'l: loop { m = m + 1; if m > 4 { break 'l } }
    wok(len(a), len(b), n, m)
}"#),

    // ── `typeof` menjawab nama JRG, bukan nama internal ────────────────
    ("typeof memakai nama JRG", r#"package main
struct P { pub x: int }
enum W { A }
fn kosong() {}
pub fn main() {
    var i: int = 1
    var f: f64 = 1.5
    var r: rune = 'a'
    var a: [2]int = [1, 2]
    var t: (int, string) = (1, "a")
    var p: P = P { x: 1 }
    var u: u32 = 7
    wok(typeof(i), typeof(f), typeof(r), typeof(u))
    wok(typeof(a), typeof(a[0:1]), typeof(t), typeof(p))
    wok(typeof(W::A), typeof(kosong()), typeof("x"), typeof(true))
}"#),

    // ── memindahkan slice juga menyalin ────────────────────────────────
    ("memindahkan slice ke nama lain menyalin", r#"package main
fn ambil(a: array) -> array { return a }
pub fn main() {
    var a: [4]int = [1, 2, 3, 4]
    var s = a[0:3]
    var t = s
    t[0] = 99
    wok(s[0], t[0])
    var g = []
    for i in 0..3 { g = g + [i] }
    var h = g
    h[0] = 77
    wok(g[0], h[0])
    var u = ambil(s)
    u[1] = 55
    wok(s[1], u[1])
}"#),

    // ── slice adalah NILAI, bukan pandangan ────────────────────────────
    ("menulis lewat slice tidak menyentuh asalnya", r#"package main
pub fn main() {
    var a: [4]int = [1, 2, 3, 4]
    var s = a[0:2]
    s[0] = 99
    wok(a[0], a[1], s[0], s[1])
}"#),

    ("slice dari slice juga terpisah", r#"package main
pub fn main() {
    var a: [4]int = [1, 2, 3, 4]
    var s = a[0:3]
    var t = s[0:2]
    t[0] = 55
    wok(a[0], s[0], t[0])
}"#),

    ("fungsi yang menulis slice kiriman tidak menyentuh asalnya", r#"package main
fn ubah(s: array) { s[0] = 900 }
pub fn main() {
    var a: [4]int = [1, 2, 3, 4]
    ubah(a[0:2])
    ubah(a)
    wok(a[0], a[1])
}"#),

    // Sebuah slice bisa MELEWATI hidup frame yang membuatnya.
    ("slice dikembalikan dari fungsi tetap utuh", r#"package main
fn bikin() -> array {
    var lokal: [4]int = [11, 22, 33, 44]
    return lokal[0:3]
}
fn ganggu() -> int {
    var sampah: [8]int = [7, 7, 7, 7, 7, 7, 7, 7]
    return sampah[0]
}
pub fn main() {
    var s = bikin()
    var _ = ganggu()
    wok(len(s), s[0], s[1], s[2])
}"#),

    // ── `a += b` berarti `a = a + b` ───────────────────────────────────
    ("penugasan majemuk pada string", r#"package main
pub fn main() {
    var s: string = "a"
    s += "b"
    s += 5
    wok(s)
    var t: string = "ab"
    t *= 3
    wok(t)
}"#),

    // ── array kosong sebagai literal lentur ────────────────────────────
    ("array kosong di kedua sisi penyambungan", r#"package main
pub fn main() {
    wok(len([1] + []), len([] + [1]))
    var s: [2]string = ["a", "b"]
    wok(len(s + []), len([] + s))
}"#),

    ("array kosong sebagai argumen dan nilai", r#"package main
fn ambil(a: array) -> int { return len(a) }
pub fn main() {
    wok(ambil([]))
    var t = []
    wok(len(t))
}"#),

    // Array berukuran tetap dibandingkan dengan slice: yang dibandingkan ISINYA.
    ("array tetap dibandingkan dengan slice", r#"package main
pub fn main() {
    var xs: [4]int = [1, 2, 3, 4]
    wok(xs[2:2] == [], xs[0:2] == [])
    wok(xs == xs[0:4], xs[0:2] == xs[0:2])
    wok([] == [])
}"#),

    // ── operasi string di level byte ───────────────────────────────────
    ("operasi string dengan karakter multi-byte", r#"package main
pub fn main() {
    var s: string = "Halo Dunia aé漢z"
    wok(str::upper(s))
    wok(str::lower(s))
    wok(str::reverse(s))
    wok(str::replace(s, "a", "_"))
    wok(str::replace(s, "漢", "X"))
    wok(str::contains(s, "Dunia"), str::contains(s, "漢"), str::contains(s, "zz"))
    wok(str::starts_with(s, "Halo"), str::ends_with(s, "z"))
    wok(len(s), str::trim("  x  "))
}"#),

    // Pola kosong menyisipkan penggantinya di setiap BATAS KARAKTER,
    // termasuk kedua ujung. Runtime LLVM dulu mengembalikan stringnya apa
    // adanya — divergensi yang sudah ada sebelum pekerjaan level-byte.
    ("replace dengan pola kosong", r#"package main
pub fn main() {
    wok(str::replace("aé漢", "", "-"))
    wok(str::replace("", "", "-"))
    wok(str::replace("abc", "", "."))
}"#),

    // ── kesetaraan struktural agregat ──────────────────────────────────
    ("eq array tetap", r#"package main
pub fn main() {
    var a: [2]int = [1, 2]
    var b: [2]int = [1, 2]
    var c: [2]int = [1, 3]
    wok(a == b, a == c, a != c, a != b)
}"#),

    ("eq tuple campur tipe", r#"package main
pub fn main() {
    var a: (int, string) = (1, "a")
    var b: (int, string) = (1, "a")
    var c: (int, string) = (2, "a")
    wok(a == b, a == c)
}"#),

    ("eq unit", r#"package main
fn kosong() {}
pub fn main() {
    wok(kosong() == kosong())
}"#),

    ("eq struct field demi field", r#"package main
struct P { pub x: int, pub y: int }
pub fn main() {
    wok(P { x: 1, y: 2 } == P { x: 1, y: 2 }, P { x: 1, y: 2 } == P { x: 9, y: 2 })
}"#),

    ("eq array bersarang", r#"package main
pub fn main() {
    var a: [2][2]int = [[1, 2], [3, 4]]
    var b: [2][2]int = [[1, 2], [3, 4]]
    var c: [2][2]int = [[1, 2], [3, 9]]
    wok(a == b, a == c)
}"#),

    ("eq float dan string di dalam array", r#"package main
pub fn main() {
    var f1: [2]f64 = [1.5, 2.5]
    var f2: [2]f64 = [1.5, 2.5]
    var s1: [2]string = ["a", "b"]
    var s2: [2]string = ["a", "c"]
    wok(f1 == f2, s1 == s2)
}"#),

    // Slice: panjang harus dibandingkan LEBIH DULU. Tanpa itu loopnya
    // membaca melewati ujung yang lebih pendek.
    ("eq slice termasuk beda panjang", r#"package main
fn sama(a: array, b: array) -> bool { return a == b }
pub fn main() {
    var x: [5]int = [1, 2, 3, 4, 5]
    var y: [5]int = [1, 2, 9, 4, 5]
    wok(sama(x[0:3], y[0:2]), sama(x[0:3], y[0:3]), sama(x[0:2], y[0:2]))
}"#),

    // Jalur LOOP, bukan unroll: interpreter menerima array besar, jadi
    // meng-unroll perbandingannya akan meledakkan IR. Kasus "beda di elemen
    // 0" ikut menguji keluar-lebih-awal.
    ("eq array besar lewat loop", r#"package main
pub fn main() {
    var a: [2000]int = [7; 2000]
    var b: [2000]int = [7; 2000]
    wok(a == b)
    b[1999] = 8
    wok(a == b)
    b[1999] = 7
    b[0] = 8
    wok(a == b)
}"#),

    // Sisi POSITIF dari penegakan visibilitas: field privat harus tetap bisa
    // diakses dari dalam method tipenya sendiri, baca maupun tulis.
    ("field privat boleh diakses dari method sendiri", r#"package main
struct Akun { saldo: int = 100, pub nama: string = "a" }
impl Akun {
    fn lihat(self) -> int { return self.saldo }
    fn tambah(self) -> int {
        self.saldo = self.saldo + 5
        return self.saldo
    }
}
pub fn main() {
    var a: Akun = Akun {}
    wok(a.lihat(), a.tambah(), a.nama)
}"#),

    ("shadowing di blok", r#"package main
pub fn main() {
    var x: int = 1
    { var x: int = 2; wok(x) }
    wok(x)
}"#),

    // ── Closure yang dipanggil langsung, dan `move` di posisi HOF ────────
    ("closure langsung dipanggil",        "package main\npub fn main() { wok((|x| x * 2)(4)) }"),
    ("closure langsung, dua parameter",   "package main\npub fn main() { wok((|a, b| a + b)(3, 5)) }"),
    ("closure langsung, argumen string",  "package main\npub fn main() { wok((|s| s + \"!\")(\"halo\")) }"),
    ("closure langsung, tanpa parameter", "package main\npub fn main() { wok((|| 7)()) }"),
    ("closure langsung, anotasi menang",  "package main\npub fn main() { wok((|x: f64| x * 2.0)(1.5)) }"),
    ("closure langsung, move",            "package main\npub fn main() { var n = 1\n wok((move |x| x + n)(10)) }"),
    ("move di HOF memutus tautan",        "package main\npub fn main() { var n = 0\n let b = [1, 2, 3].map(move |x| { n = n + 1\n return x })\n wok(n, b) }"),

    // ── Tangkapan atas PARAMETER closure yang melingkupi ─────────────────
    ("tangkap: fn -> closure atas param fn",     "package main\nfn bikin(a: int) -> fn(int) -> int { return |b: int| { return a + b } }\npub fn main() { wok(bikin(3)(4)) }"),
    ("tangkap: closure atas lokal luar",         "package main\npub fn main() { let n = 3\n let c = |b: int| { return n + b }\n wok(c(4)) }"),
    ("tangkap: closure->closure atas lokal",     "package main\npub fn main() { let d = |a: int| { let m = a\n return |b: int| { return m + b } }\n wok(d(3)(4)) }"),
    ("tangkap: closure->closure atas PARAMETER", "package main\npub fn main() { let e = |a: int| { return |b: int| { return a + b } }\n wok(e(3)(4)) }"),

    // ── Closure berantai (currying) ──────────────────────────────────────
    ("currying: dua tingkat",  "package main\npub fn main() { let luar = |a| { return |b| { return a + b } }\n wok(luar(3)(4)) }"),
    ("currying: tiga tingkat", "package main\npub fn main() { let t = |a| { return |b| { return |c| { return a + b + c } } }\n wok(t(1)(2)(3)) }"),
    ("currying: tipe campur",  "package main\npub fn main() { let c = |s| { return |n| { return s + (n as string) } }\n wok(c(\"x\")(5)) }"),

    // ── Paruh `Result` dari situs panggilan LAIN ─────────────────────────
    ("result: paruh dari situs panggilan lain",
     "package main\nfn lewat(r) { let v = r?\n return Ok(v * 2) }\npub fn main() { wok(lewat(Ok(21)))\n wok(lewat(Err(\"gagal\"))) }"),

    // ── Kesetaraan enum ──────────────────────────────────────────────────
    ("enum ==: Result sama dan beda",
     "package main\npub fn main() { let a: Result<int, string> = Ok(1)\n let b: Result<int, string> = Ok(1)\n let c: Result<int, string> = Ok(2)\n wok(a == b, a == c, a != c) }"),
    ("enum ==: lintas variant",
     "package main\npub fn main() { let a: Result<int, string> = Ok(1)\n let d: Result<int, string> = Err(\"x\")\n wok(a == d, d == d) }"),
    ("enum ==: Option dengan variant unit",
     "package main\npub fn main() { let s: Option<int> = Some(3)\n let t: Option<int> = Some(4)\n let u: Option<int> = None\n wok(s == s, s == t, s == u, u == u) }"),
    ("enum ==: enum pengguna, payload jamak",
     "package main\nenum B { Lingkaran(float), Kotak(float, float), Titik }\npub fn main() { wok(B::Titik == B::Titik, B::Lingkaran(2.0) == B::Lingkaran(2.0), B::Lingkaran(2.0) == B::Lingkaran(3.0)) }"),
    ("enum ==: payload dua field",
     "package main\nenum B { Lingkaran(float), Kotak(float, float), Titik }\npub fn main() { wok(B::Kotak(1.0, 2.0) == B::Kotak(1.0, 2.0), B::Kotak(1.0, 2.0) == B::Kotak(1.0, 9.0), B::Lingkaran(2.0) == B::Titik) }"),

    // ── `d.kunci` pada sebuah dict ───────────────────────────────────────
    ("dict: baca lewat .kunci",  "package main\npub fn main() { let d = {\"a\": 1, \"b\": 2}\n wok(d[\"a\"], d.a, d.b) }"),
    ("dict: tulis lewat .kunci", "package main\npub fn main() { var d = {\"a\": 1}\n d.a = 9\n wok(d.a, d[\"a\"]) }"),
    ("dict: .kunci baru menumbuhkan", "package main\npub fn main() { var d = {\"a\": 1}\n d.c = 30\n wok(d.c, dict::len(d), d) }"),
    ("dict: .kunci dengan +=",   "package main\npub fn main() { var d = {\"a\": 1}\n d.a += 5\n wok(d.a) }"),
    // ── `dict::new()` dan method gaya `d.len()` ──────────────────────────
    ("dict::new lalu diisi",     "package main\npub fn main() { var b = dict::new()\n b[\"x\"] = \"halo\"\n wok(b, dict::len(b)) }"),
    ("dict::new tak pernah diisi","package main\npub fn main() { let k = dict::new()\n wok(dict::len(k), dict::is_empty(k), k) }"),
    ("dict method: len/is_empty/has", "package main\npub fn main() { let d = {\"a\": 1, \"b\": 2}\n wok(d.len(), d.is_empty(), d.has(\"a\"), d.has(\"z\")) }"),
    ("dict method: keys/values",  "package main\npub fn main() { let d = {\"a\": 1, \"b\": 2}\n wok(d.keys(), d.values()) }"),
    ("dict method: remove/merge", "package main\npub fn main() { let d = {\"a\": 1, \"b\": 2}\n wok(d.remove(\"a\"), d.merge({\"c\": 3})) }"),
    ("dict method: get",          "package main\npub fn main() { let d = {\"a\": 1}\n wok(d.get(\"a\"), d.get(\"z\")) }"),

    // ── Nilai bertag: dict bernilai CAMPUR TIPE ──────────────────────────
    ("any: dict campur, baca kunci tertulis",
     "package main\npub fn main() { let d = {\"nama\": \"budi\", \"umur\": 17}\n wok(d[\"nama\"], d[\"umur\"], dict::len(d)) }"),
    ("any: dict campur lewat .kunci",
     "package main\npub fn main() { let d = {nama: \"siti\", umur: 20}\n wok(d[\"nama\"], d.umur) }"),
    ("any: dict campur bersarang",
     "package main\npub fn main() { let b = {\"angka\": [1, 2], \"dalam\": {\"k\": \"v\"}}\n wok(b[\"angka\"][1], b[\"dalam\"][\"k\"]) }"),
    ("any: dict campur dengan nil",
     "package main\npub fn main() { let g = {\"ada\": 1, \"kosong\": nil}\n wok(g.get(\"ada\"), g.get(\"kosong\"), g.get(\"tak_ada\")) }"),
    ("any: Option berpayload nol",
     "package main\npub fn main() { let g = {\"ada\": 1, \"kosong\": nil}\n wok(g.get(\"kosong\") == Some(nil), g.get(\"tak_ada\").is_none()) }"),

    // ── Nilai bertag: `var` yang BERGANTI tipe elemen ────────────────────
    ("any: var berganti tipe elemen",
     "package main\nvar jejak = []\nfn catat(s) { jejak = arr::push(jejak, s) }\npub fn main() { jejak = []\n catat(\"a\")\n wok(jejak)\n jejak = []\n catat(1)\n wok(jejak) }"),
    ("any: konflik lewat defer",
     "package main\nvar jejak = []\nfn catat(s) { jejak = arr::push(jejak, s) }\nfn pakai() { defer catat(7)\n catat(\"x\") }\npub fn main() { pakai()\n wok(jejak) }"),

    // Yang TIDAK boleh ikut berubah: koleksi bertipe seragam tetap telanjang.
    // Kalau kotak bocor ke sini, keluarannya masih benar tapi jalannya sudah
    // beda — jadi yang dijaga di sini bukan sekadar jawabannya.
    ("any: dict seragam tetap seragam",
     "package main\npub fn main() { let d = {\"a\": 1, \"b\": 2}\n wok(d[\"a\"] + d[\"b\"], dict::len(d)) }"),
    ("any: var tipe elemen konsisten",
     "package main\nvar t = []\nfn tambah(x) { t = arr::push(t, x) }\npub fn main() { tambah(1)\n tambah(2)\n wok(t) }"),

    // ── Method koleksi yang mengembalikan `Option` ───────────────────────
    ("koleksi: first/last/get",
     "package main\npub fn main() { let a = [10, 20, 30]\n wok(a.first(), a.last(), a.get(1), a.get(99)) }"),
    ("koleksi: first pada kosong",
     "package main\npub fn main() { let k = []\n wok(k.first(), k.last()) }"),
    ("koleksi: Some(nil) beda dari None",
     "package main\npub fn main() { let a = [nil]\n wok(a.first(), a.first().is_some()) }"),
    ("koleksi: get_or",
     "package main\npub fn main() { let a = [10, 20]\n wok(a.get_or(1, -1), a.get_or(9, -1)) }"),
    ("koleksi: pop",
     "package main\npub fn main() { let a = [10, 20, 30]\n let (s, d) = a.pop()\n wok(s, d, a) }"),
    ("koleksi: pop pada kosong",
     "package main\npub fn main() { let k = []\n let (s, d) = k.pop()\n wok(s, d) }"),
    ("koleksi: min/max sbg method",
     "package main\npub fn main() { wok([3, 1, 2].min(), [3, 1, 2].max(), [1, 2].sum()) }"),

    // ── Namespace `str::` ────────────────────────────────────────────────
    ("str:: bentuk namespace",
     "package main\npub fn main() { wok(str::contains(\"halo\", \"al\"), str::upper(\"ab\"), str::trim(\"  x  \")) }"),
    ("str:: is_empty/char_at/repeat",
     "package main\npub fn main() { wok(str::is_empty(\"\"), str::is_empty(\"a\"), str::char_at(\"halo\", 1), str::repeat(\"ab\", 3)) }"),

    // ── Iterasi dict memberi KUNCI ───────────────────────────────────────
    ("dict: iterasi memberi kunci",
     "package main\npub fn main() { let d = {\"b\": 2, \"a\": 1}\n var t = []\n for k in d { t = t + [k] }\n wok(t) }"),
    ("dict: iterasi dua variabel",
     "package main\npub fn main() { let d = {\"b\": 2, \"a\": 1}\n var t = []\n for k, v in d { t = t + [«${k}=${v}»] }\n wok(t) }"),

    // ── Lokal SKALAR yang berganti tipe ──────────────────────────────────
    ("dinamis: int lalu float",   "package main\npub fn main() { var x = 1\n x = 2.5\n wok(x, typeof(x)) }"),
    ("dinamis: int lalu bool",    "package main\npub fn main() { var y = 1\n y = true\n wok(y, typeof(y)) }"),
    ("dinamis: float lalu int",   "package main\npub fn main() { var z = 3.9\n z = 1\n wok(z, typeof(z)) }"),
    ("dinamis: berganti di dalam if", "package main\npub fn main() { var x = 1\n if true { x = \"teks\" }\n wok(x, typeof(x)) }"),
    ("dinamis: int lalu string",  "package main\npub fn main() { var x = 1\n x = \"teks\"\n wok(x, typeof(x)) }"),
    ("dinamis: dibaca sebelum berganti", "package main\npub fn main() { var x = 1\n wok(x, typeof(x))\n x = \"teks\"\n wok(x, typeof(x)) }"),
    ("dinamis: global skalar",    "package main\nvar g = 1\npub fn main() { g = \"teks\"\n wok(g, typeof(g)) }"),

    // Aritmetika dan perbandingan atas nilai yang golongannya baru diketahui
    // saat jalan.
    ("dinamis: aritmetika setelah berganti", "package main\npub fn main() { var x = 1\n x = \"a\"\n x = 2\n wok(x + 1) }"),
    ("dinamis: int + float bertag", "package main\npub fn main() { var y = 1\n y = 2.5\n wok(y + 1, y * 2) }"),
    ("dinamis: gabung string dua arah", "package main\npub fn main() { var s = 1\n s = \"n=\"\n var t = 1\n t = \"x\"\n wok(s + 5, 5 + t) }"),
    ("dinamis: kesetaraan",       "package main\npub fn main() { var x = 1\n x = 2\n var y = 1\n y = \"teks\"\n wok(x == 2, x != 2, y == \"teks\", y == \"lain\") }"),
    ("dinamis: urutan angka",     "package main\npub fn main() { var z = 1\n z = 2.5\n wok(z > 2.0, z < 2.0, z >= 2.5) }"),
    ("dinamis: typeof dari tag",  "package main\npub fn main() { let d = {\"a\": 1, \"b\": \"t\"}\n let k = \"a\"\n wok(typeof(d[k])) }"),

    // Yang TIDAK boleh ikut berubah: lokal yang tipenya konsisten tetap
    // telanjang. Kalau kotaknya bocor ke sini, jawabannya masih benar tapi
    // jalannya sudah beda — jadi yang dijaga bukan sekadar keluarannya.
    ("dinamis: lokal konsisten tetap telanjang", "package main\npub fn main() { var x = 1\n x = 2\n x = 3\n wok(x + 1, typeof(x)) }"),
    ("dinamis: float konsisten tetap telanjang", "package main\npub fn main() { var f = 1.5\n f = 2.5\n wok(f * 2.0, typeof(f)) }"),
    // Posisi per KARAKTER, sama seperti dua backend lain — dan `find`/
    // `index_of` sebelumnya tidak ada di sini sama sekali.
    ("posisi: find per karakter",    "package main\npub fn main() { wok(str::find(\"héllo\", \"l\"), \"héllo\".index_of(\"l\")) }"),
    // Bentuk pulang-perginya lewat `Option` sejak Fase MUDAH. Kasus lamanya
    // berbunyi `s[str::find(s, "k")]` — yang persis jebakan yang ditutup: `-1`
    // adalah indeks yang SAH, jadi pencarian gagal membaca karakter terakhir.
    ("posisi: pulang-pergi s[find]",
     "package main\npub fn main() { let s = \"日本語kode\"\n       match str::find(s, \"k\") { Some(i) => wok(s[i]), None => wok(\"tidak ada\") }\n       match str::find(s, \"z\") { Some(i) => wok(s[i]), None => wok(\"tidak ada\") } }"),
    ("posisi: find tidak ketemu",    "package main\npub fn main() { wok(str::find(\"halo\", \"z\")) }"),

    // `**=` dan struct tuple keduanya diturunkan di PARSER, jadi backend ini
    // mewarisinya tanpa disentuh — dan itu justru yang perlu dibuktikan.
    ("operator **=",
     "package main\npub fn main() { var a = 2\n a **= 10\n wok(a) }"),
    ("struct tuple: konstruksi dan field",
     "package main\nstruct Titik(int, int)\n\
      pub fn main() { let t = Titik(3, 4)\n wok(t._0 + t._1) }"),

    // Pola di posisi parameter, diturunkan di parser jadi parameter biasa plus
    // `let` — jadi backend ini mewarisinya tanpa disentuh.
    ("pola di parameter",
     "package main\nfn jumlah((a, b)) -> int { return a + b }\n\
      pub fn main() { wok(jumlah((3, 4))) }"),

    // Bentuk namespace HOF dialihkan ke bentuk method yang sudah ada di backend ini, dan `fold`
    // memakai mesin `reduce` dengan letak nilai awal yang bertukar.
    ("hof namespace dan fold",
     "package main\npub fn main() { wok(arr::map([1, 2, 3], |x| x * 2))\n\
      wok(arr::filter([1, 2, 3], |x| x > 1))\n wok(arr::reduce([1, 2, 3], |a, b| a + b, 0))\n\
      wok(arr::any([1, 2], |x| x > 1))\n wok(arr::find([1, 2], |x| x > 1))\n\
      wok([1, 2, 3].fold(10, |a, b| a + b))\n wok(arr::fold([1, 2, 3], 10, |a, b| a + b)) }"),

    // ── Warisan struct (Fase LLVM-14) ──────────────────────────────────────
    (
        "warisan: field induk terbaca, dan tampilannya sama",
        r#"package main
struct Hewan { pub nama: string }
struct Anjing: Hewan { pub ras: string }
pub fn main() {
    let a = Anjing { nama: "Bo", ras: "beagle" }
    wok(a.nama, a.ras)
    wok(a)
}"#),
    (
        "warisan: nilai default milik induk ikut berlaku",
        r#"package main
struct Hewan { pub nama: string = "tanpa nama", pub kaki: int = 4 }
struct Anjing: Hewan { pub ras: string = "campuran" }
pub fn main() { let a = Anjing {}  wok(a.nama, a.kaki, a.ras) }"#),
    (
        "warisan: method induk dipanggil atas anak",
        r#"package main
struct Hewan { pub nama: string }
impl Hewan { pub fn sapa(self) -> string { return "halo " + self.nama } }
struct Anjing: Hewan { pub ras: string }
pub fn main() { wok(Anjing { nama: "Bo", ras: "beagle" }.sapa()) }"#),
    (
        "warisan: rantai tiga tingkat",
        r#"package main
struct A { pub a: int }
struct B: A { pub b: int }
struct C: B { pub c: int }
pub fn main() { let x = C { a: 1, b: 2, c: 3 }  wok(x.a + x.b + x.c) }"#),
    // Bayangan yang MENGGANTI TIPE menentukan rancangannya.
    (
        "warisan: anak membayangi field induk, tipenya ikut berganti",
        r#"package main
struct P { pub n: int }
impl P { pub fn lihat(self) -> string { return "n=" + self.n } }
struct Q: P { pub n: string }
pub fn main() { let q = Q { n: "teks" }  wok(q.n)  wok(q.lihat()) }"#),
    (
        "warisan: field privat induk lewat method warisan",
        r#"package main
struct P { rahasia: int = 5 }
impl P { pub fn baca(self) -> int { return self.rahasia } }
struct Q: P { pub x: int }
pub fn main() { wok(Q { x: 1 }.baca()) }"#),
    (
        "warisan: pola, sebar, dan hook to_string",
        r#"package main
struct Hewan { pub nama: string = "x", pub kaki: int = 4 }
impl Hewan { pub fn to_string(self) -> string { return "<" + self.nama + ">" } }
struct Anjing: Hewan { pub ras: string = "y" }
pub fn main() {
    let a = Anjing { nama: "Bo", ras: "beagle" }
    match a { Anjing { nama, ras } => wok(nama, ras) }
    let b = Anjing { nama: "Rex", ...a }
    wok(b.nama, b.kaki, b.ras)
    wok(a)
}"#),

    // Fase SINKRON-LLVM: `Atomic` dan `Mutex` adalah pegangan di backend ini juga.
    ("Atomic: baca, tulis, tambah, cas, dan salinan yang menunjuk keadaan yang sama", r#"package main
pub fn main() {
    let a = Atomic::new(5)
    wok(a, typeof(a))
    wok(atomic::add(a, 3), atomic::sub(a, 1), atomic::get(a))
    wok(atomic::set(a, 10), atomic::get(a))
    wok(atomic::cas(a, 10, 20), atomic::cas(a, 10, 30), atomic::get(a))
    let b = a
    atomic::add(b, 1)
    wok(atomic::get(a), a == b, a == Atomic::new(21), a != b)
    wok([a, b], (a, 1), "x" + str(a))
    let z = Atomic::new()
    wok(atomic::get(z))
}"#),
    ("Atomic dan Mutex sebagai field, lewat spawn dan closure", r#"package main
struct K { pub hit: Atomic, pub m: Mutex }
fn naik(a: Atomic, n: int) { for i in 0..n { atomic::add(a, 1) } }
pub fn main() {
    let k = K { hit: Atomic::new(0), m: Mutex::new() }
    let ch = make_chan(int, 4)
    for i in 0..4 {
        spawn { naik(k.hit, 100); ch <- 1 }
    }
    for i in 0..4 { <-ch }
    wok(atomic::get(k.hit), k)
    let f = |x| { atomic::add(k.hit, x) }
    wok(f(5))
    var total = 0
    let m = k.m
    wok(m, typeof(m), m == k.m, m == Mutex::new())
    wok(mutex::lock(m))
    wok(mutex::unlock(m))
    let r = mutex::with_lock(m, || { 7 * 6 })
    wok(r)
    wok(mutex::with_lock(m, || "teks"))
    wok("beres")
}"#),
    ("mutex::with_lock menjaga baca-tidur-tulis antar goroutine", r#"package main
pub fn main() {
    let m = Mutex::new()
    let p = Atomic::new(0)
    let ch = make_chan(int, 8)
    for i in 0..8 {
        spawn {
            mutex::with_lock(m, || {
                let v = atomic::get(p)
                time::sleep(0.001)
                atomic::set(p, v + 1)
            })
            ch <- i
        }
    }
    for i in 0..8 { <-ch }
    wok(atomic::get(p))
}"#),
    ("ekor badan perulangan bukan nilai kembalian fungsi tanpa anotasi", r#"package main
fn hitung(n: int) { for i in 0..n { math::abs(i) } }
fn putar(n: int) { var i = 0
    while i < n { i += 1; str::upper("a") } }
fn ulang(xs: []int) { for _, x in xs { x * 2 } }
fn cari(xs: []int) { for _, x in xs { if x > 1 { return x } }
    return -1 }
pub fn main() {
    wok(hitung(3), putar(2), ulang([1, 2]), cari([1, 5]))
}"#),
    ("Atomic dan Mutex lewat method, fungsi tanpa anotasi, Future, Task, Option dan dict", r#"package main
struct Pencacah { pub n: Atomic, pub m: Mutex }
impl Pencacah {
    fn baru() -> Pencacah { Pencacah { n: Atomic::new(0), m: Mutex::new() } }
    fn naik(self) { atomic::add(self.n, 1) }
    fn baca(self) -> int { atomic::get(self.n) }
}
fn buat() { Atomic::new(7) }
fn kunci_baru() { Mutex::new() }
fn tambah(a, n) { atomic::add(a, n) }
fn tukar(a: Atomic) { atomic::cas(a, 7, 70) }
pub fn main() {
    let p = Pencacah::baru()
    p.naik(); p.naik()
    let q = p
    q.naik()
    wok(p.baca(), q.baca(), p == q)
    let a = buat()
    wok(tambah(a, 3), tukar(a), tukar(buat()), atomic::get(a))
    let m = kunci_baru()
    let hasil = mutex::with_lock(m, || { atomic::add(a, 1); [atomic::get(a), 2] })
    wok(hasil, str(a), str(m), """${a} ${m}""")
    let f = Future::new(|| { mutex::with_lock(m, || atomic::add(a, 5)) })
    wok(future::await(f))
    var hit = Atomic::new(0)
    let t = Task::spawn(|| { for i in 0..1000 { atomic::add(hit, 2) }; atomic::get(hit) })
    wok(task::join(t), atomic::get(hit))
    hit = Atomic::new(-4)
    wok(atomic::get(hit), atomic::sub(hit, -9223372036854775807))
    let o: Option<Atomic> = Some(a)
    wok(o, typeof(o), [m] == [m], (a, m) == (a, m), (a, m) == (buat(), m))
    let d = {"x": a, "y": buat()}
    wok(d, atomic::add(d["x"], 1), atomic::get(a))
    let g = Future::new(|| { mutex::with_lock(m, || { let z = [1]; z[4] }) })
    time::sleep(0.02)
    mutex::lock(m); wok("kunci bebas lagi"); mutex::unlock(m)
}"#),

    // Fase CAMPUR: dulu ditolak (`BELUM_DIDUKUNG_LLVM`); elemennya sekarang bertag, jadi `2` tetap bulat.
    ("literal bulat ditambahkan ke array pecahan",
     "package main\npub fn main() { var f = [1.5]\n f = arr::push(f, 2)\n wok(f)\n wok(typeof(f[1]), f[1] / 4, f[0] + f[1]) }"),
    // Fase FFI-LLVM: tipe kembalian dari `return Ok(..)` saja, atau dari `?` plus `return Ok(..)`.
    ("tanpa anotasi: return Ok saja", "package main\nfn f() { return Ok(0) }\npub fn main() { match f() { Ok(v) => wok(\"ok\", v), Err(e) => wok(\"err\", e) } }"),
    ("tanpa anotasi: ? lalu return Ok", "package main\nfn cek(n: int) -> Result<int, string> { if n > 0 { return Ok(n) }\n return Err(\"negatif\") }\nfn f(n) {\n let x = cek(n)?\n return Ok(x * 2) }\npub fn main() { wok(f(3), f(-1)) }"),
    ("? di main yang berhasil", "package main\npub fn main() { let x = Result::Ok(5)?\n let y = [7].first()?\n wok(x + y) }"),
    // Fase MAIN-HASIL (#834): `main` beranotasi `Result`/`Option` dulu ditolak saat kompilasi.
    ("main beranotasi Result yang berhasil", "package main\nfn f(x: int) -> Result<int, string> { Ok(x * 2) }\npub fn main() -> Result<int, string> { wok(f(3)?)\n Ok(5) }"),
    // Fase AMAN-1 (#840): NUL dulu memotong jalur di runtime C — `mkdir("d\0x")` membuat `d`.
    ("jalur ber-NUL tidak menyentuh berkas lain", r#"package main
pub fn main() {
    let z = str(rune(0))
    wok(fs::exists("Cargo.toml" + z + "x"), fs::is_file("Cargo.toml" + z), fs::mkdir("jrg_uji_nul" + z + "x"))
    wok(fs::exists("jrg_uji_nul"), env::get("PATH" + z), fs::delete("jrg_uji_nul" + z))
}
"#),
    // Fase AMAN-21 (#899): gzip:: — CRC32, ukuran, bit cadangan, ekor liar dan data terpotong jadi Err.
    ("gzip:: dan gzip jahat", r#"package main
pub fn main() {
    let z = gzip::compress("halo gzip\n")
    wok(len(z), z[0], z[1], z[2])
    wok(gzip::decompress_text(z).unwrap() == "halo gzip\n")
    wok(gzip::decompress(gzip::compress([0, 255, 7])))
    let h = [31, 139, 8, 8, 133, 111, 229, 106, 0, 3, 104, 97, 108, 111, 46, 116, 120, 116, 0, 203, 72, 204, 201, 87, 72, 175, 202, 44, 224, 2, 0, 57, 190, 200, 48, 10, 0, 0, 0]
    wok(gzip::decompress_text(h + gzip::compress("lagi")))
    var rusak = h
    rusak[31] = 0
    wok(gzip::decompress(rusak))
    wok(gzip::decompress_text(gzip::compress([255, 254])))
    wok(gzip::decompress(h + [0]))
    wok(gzip::decompress([80, 75, 3, 4]))
    var cadangan = h
    cadangan[3] = 40
    wok(gzip::decompress(cadangan))
    wok(gzip::decompress(h[0:20]))
}
"#),
    // Fase AMAN-29 (#908): masukan di pesan galat dipotong dan kendalinya di-escape — pesan tidak tumbuh mengikuti masukan.
    ("pesan galat toml::/url:: tidak mengutip masukan utuh", r#"package main
fn main() {
    let x = str::repeat("9", 2000)
    for _, r in [toml::parse("a = " + x), toml::parse("a = " + x + "z"), toml::parse("[" + str::repeat("k", 500) + "]\n[" + str::repeat("k", 500) + "]")] {
        match r {
            Ok(_) => wok("ok"),
            Err(e) => wok(len(e), e),
        }
    }
    for _, r in [url::parse("http://h:" + x), url::parse("http://" + str::repeat("a", 300) + "%41/"), url::parse("http://h:8a")] {
        match r {
            Ok(_) => wok("ok"),
            Err(e) => wok(len(e), e),
        }
    }
}
"#),
    // Fase PANGKAT-TEPAT (#970–#971): `float ** int` dibulatkan tepat di tiga mesin; `**` atas nilai bertag.
    ("pangkat: pecahan dibulatkan tepat, nilai bertag", r#"package main
fn f(x: float, n: int) -> float { return x ** n }
pub fn main() {
    let xs = [1.1, 1.0001, 0.7, 9.87654321, 1.01]
    let ns = [10, 37, 123, 15, 1000]
    for i, x in xs { wok(f(x, ns[i])) }
    wok(1.1 ** 10, 2 ** -3, 3 ** 0.5)
    var x = nil
    x = 2
    wok(x ** 10, x ** -2, x ** 0.5)
    x = 1.1
    wok(x ** 10, x ** 2.5)
    var n = [1, 2.5]
    n[0] **= 2
    n[1] **= 2
    wok(n)
}
"#),
    // Fase TAMBAH-TEKS (#967–#968): `+` dengan teks menghasilkan teks, dua array bertag tersambung, `dict::set` berjenis lain.
    ("tambah: teks lawan apa pun, array bertag, dict::set berjenis lain", r#"package main
pub fn main() {
    var c = [1]
    c = c + "z"
    wok(c, typeof(c))
    var e = [1]
    e = e + [2] + "z"
    wok(e)
    var n = 1
    n = n + "x"
    wok(n, len(1 + "ab"))
    var s = "x"
    s = s + 1 + true + [3, 4]
    wok(s)
    var x = nil
    x = [1, 2]
    var z = nil
    z = ["q"]
    let w = x + z + [3.5]
    wok(w, len(w), w[1])
    var d = {"a": 1}
    d = dict::set(d, "b", 2)
    d = dict::set(d, 3, "tiga")
    wok(d, d["a"] + 1)
}
"#),
    // Fase ISI-ARRAY (#965–#966): elemen `var` array ditulis berjenis lain; literal wadah bersama bukan-wadah.
    ("array: elemen ditulis berjenis lain, wadah campur bukan wadah", r#"package main
pub fn main() {
    var a = [1, 2]
    a[1] = "q"
    wok(a, a[0] + 1)
    var b = [1, 2]
    b = arr::push(b, 3)
    b[0] = "q"
    wok(b)
    var c = [[1], [2]]
    c[0][0] = "q"
    wok(c)
    var e = [[1, 2], "z"]
    e[1] = [1, 1]
    wok(e, arr::max(e), arr::min(e))
    var f = [[1], {"a": 1}]
    f[0] = [3]
    wok(f)
    var g = ["a"]
    g[0] = 5
    g = arr::push(g, [1])
    wok(g)
    var h = [1, 2]
    for i in 0..2 { h[i] = 1.5 }
    wok(h)
}
"#),
    // Fase AMAN-39 (#962): literal array berisi array berbeda panjang atas variabel/tuple/Option jadi slice.
    ("array bersarang berbeda panjang atas tuple dan Option", r#"package main
pub fn main() {
    let x = 1
    let t = (1, "b")
    let p = Some(2)
    var a = [[t], [t, t], []]
    a[2] = [t]
    wok([[x], [x, x]], a, [[p], [p, p]])
    for _, r in [[("a", "b")], [("c", "d"), ("e", "f")]] { wok(len(r), r) }
}
"#),
    // Fase LABEL-LLVM (#952–#953): `str(x)` di rantai sambungan ditulis langsung kalau `x` ternyata bulat.
    ("teks: str(aritmetika/field) di rantai sambungan", r#"package main
struct R { pub id: int, pub skor: float }
pub fn main() {
    let x: int = -9223372036854775807 - 1
    let r = R { id: 7, skor: 1.5 }
    let f = 2.5
    let a = [3, 40, -5]
    wok("a" + str(x % 10) + "b" + str(r.id / 2) + str(f * 2.0) + "|" + str(x))
    wok("n" + str(r.id - 10) + "|" + str(3 * 4) + "|" + str(r.skor) + str(a[1] % 7) + str(a[2]))
    let lebar = 7
    wok("x" + str(lebar * 1000000000000) + "/" + str(0) + str(-0))
}
"#),
    // Fase POLA-BERTAG (#948–#951): pola varian enum generik, varian unit telanjang, dan `for` atas nilai bertag.
    ("bertag: pola Option/Result/enum generik", r#"package main
enum Pohon<T> { Daun(T), Cabang(T, int), Kosong }
fn cek(x) {
    match x {
        Some(1) => "satu",
        Some(v) => "some " + str(v),
        None => "none",
        Ok(v) => "ok " + str(v),
        Err(e) => "err " + e,
        Pohon::Daun(d) => "daun " + str(d),
        Cabang(a, b) => "cabang " + str(a) + " " + str(b),
        Kosong => "kosong",
        _ => "lain",
    }
}
fn belakangan() {
    let a = [Some([1, 2]), 5]
    wok(cek(a[0]))
    let b = [Some(Some(3)), "x"]
    match b[0] {
        Some(Some(n)) => wok("dalam", n),
        _ => wok("tidak"),
    }
}
pub fn main() {
    let xs = [Some(1), Some("dua"), None, 4]
    for _, x in xs { wok(cek(x)) }
    let r: Result<int, string> = Ok(7)
    let e: Result<int, string> = Err("rusak")
    let ys = [r, e, Pohon::Daun(2.5), Pohon::Cabang("c", 3), "s"]
    for _, y in ys { wok(cek(y)) }
    let k: Pohon<int> = Pohon::Kosong
    let zs = [k, 1]
    wok(cek(zs[0]))
    belakangan()
    let campur = [Some(9), "q"]
    let Some(q) = campur[0] else { return }
    wok("q", q + 1)
    wok("akhir")
}
"#),
    ("bertag: varian unit telanjang", r#"package main
enum Arah { Kiri, Kanan }
pub fn main() {
    let a = [Arah::Kiri, 1, None, Arah::Kanan]
    for _, x in a {
        match x {
            Kiri => wok("kiri"),
            Arah::Kanan => wok("kanan"),
            None => wok("none"),
            n => wok("lain", n),
        }
    }
}
"#),
    ("bertag: for atas dict, array, teks, bulat", r#"package main
pub fn main() {
    var d = dict::new()
    d["m"] = {"b": 2, "a": [1, "x"]}
    d["n"] = [5, "enam", 7.5]
    d["s"] = "héllo"
    d["k"] = 3
    for k in d["m"] { wok("kunci", k) }
    for k, v in d["m"] { wok(k, v) }
    for i, v in d["n"] {
        if i == 1 { continue }
        wok(i, v)
    }
    for i, c in d["s"] {
        if c == 'l' { break }
        wok(i, c)
    }
    for i in d["k"] { wok("n", i) }
    var e = d["n"]
    for i, v in e { e[0] = 99; wok(v) }
    wok(e)
    var jumlah = 0
    for _, v in [1, "a", 2.5] { jumlah = jumlah + 1 }
    wok(jumlah)
}
"#),
    // Fase CELAH-BERTAG (#945–#947): tulis lewat indeks ke array/dict bertag (tanpa alias), `t.0` atas tuple bertag.
    ("bertag: tulis lewat indeks, field tuple", r#"package main
pub fn main() {
    var a = []
    a = arr::push(a, ("x", 1))
    a = arr::push(a, 5)
    let t = a[0]
    wok(t.0, t.1)
    var d = dict::new()
    d["k"] = [1, "dua"]
    d["m"] = {"a": 1}
    d["n"] = 3
    var e = d["k"]
    e[1] = "tiga"
    e[-1] = [9]
    var f = d["m"]
    f["b"] = "x"
    f["a"] = 2.5
    wok(d, e, f)
    d["k"] = e
    var g = e
    g[0] = 100
    wok(d, e, g)
}
"#),
    ("bertag: tulis lewat jalur bersarang", r#"package main
pub fn main() {
    var d = dict::new()
    d["k"] = [1, "dua"]
    d["n"] = 3
    d["m"] = {"a": 1}
    d["k"][1] = "x"
    d["k"][-2] = [7]
    d["m"]["b"] = "baru"
    var a = []
    a = arr::push(a, [1, 2])
    a = arr::push(a, "s")
    a[0][1] = "y"
    let salin = d["k"]
    d["k"][0] = 0
    wok(d, a, salin)
}
"#),
    // Fase TEKS-LLVM (#942–#944): sambungan teks n-ari (bulat, `str(bulat)`, negatif, MIN), `s[i]`/`len` lewat
    // memo di IR, `reverse`/`replace` jalur cepat — atas teks ASCII dan multibyte.
    ("teks: sambungan n-ari, indeks dan len lewat memo, reverse/replace", r#"package main
fn nama() { return "fn" }
pub fn main() {
    let n = -42
    let m = -9223372036854775807 - 1
    var s = "a"
    s = s + "b" + "c"
    wok("x" + str(1) + ":" + str(-7) + str(n) + "|" + 0 + "|" + m)
    wok("" + "" + "")
    wok("p" + str(2.5) + "q" + 1.5 + 'r' + true)
    wok("t" + nama() + str("u") + str([1, 2]) + s)
    var t = ""
    for i in 0..5 { t = t + str(i) + "," + i * i + ";" }
    wok(t, len(t))
    for _, w in ["", "abc", "aé日z", "日本", "x"] {
        var o = str(len(w)) + ":"
        for i in 0..len(w) { o = o + str(w[i]) + str(str::char_at(w, i)) + str(w[-1 - i]) }
        wok(o, str::reverse(w), str::replace(w, "a", "_"), str::replace("aXbXXc", "XX", "-"))
    }
}
"#),
    // Fase AMAN-38 (#930–#936): cabang `typeof` pasti, `map_err` di penduga, `Ok(None)` lalu `Ok(Some(..))`,
    // bulat+pecahan di satu literal, `rune` di `str::`, hasil `map` berukuran tetap, dict campur di `for`.
    ("typeof pasti, map_err, separuh Result, literal bulat+pecahan, map meluruh", r#"package main
struct W { pub t: int }
fn _byte(x) {
    if typeof(x) == "string" { return str::bytes(x) }
    if typeof(x) != "array" { return [] }
    return x
}
fn _cek(w) { if w.t < 0 { return Err("negatif") } return Ok(w) }
fn _tiga(s) {
    if s == "" { return Err("kosong") }
    return Ok((1, 2, 3))
}
fn urai(s) {
    if len(s) > 10 { return Err(«panjang ${s}») }
    let (a, b, c) = _tiga(s).map_err(|e| «x ${s}: ${e}»)?
    return _cek(W { t: a + b + c - len(s) }).map_err(|e| «y ${s}: ${e}»)
}
fn cari(n) {
    if n < 0 { return Err("negatif") }
    if n == 0 { return Ok(None) }
    return Ok(Some((n * 2, ["a", "b"])))
}
fn bikin() { return Ok({"A": "1", "B": "2"}) }
fn kunci() {
    let d = bikin().map_err(|e| «x: ${e}»)?
    var k = []
    for n, _ in d { k = arr::push(k, n) }
    return Ok(k)
}
fn isi(e, kini) { return [math::min(3.0, e[0] + 0.5), kini] }
pub fn main() {
    wok(len(_byte("ab")), len(_byte([1, 2, 3])), len(_byte(7)))
    wok(urai("ab"), urai(""), urai("abcdefghijk"))
    wok(cari(2).unwrap(), cari(0).unwrap(), cari(-1))
    wok(kunci())
    let kini = 5
    let d = {"a": [1.0, 2]}
    var e = dict::has(d, "a") $ isi(d["a"], kini) : [3 as float, kini]
    wok(e, isi([2.5, 1], 9))
    wok(str::is_alphanumeric('a'), str::is_alphanumeric('-'))
    var ops = arr::map(["p", "q"], |x| ["=", x])
    ops = arr::concat(ops, arr::push([], ["+", "r"]))
    wok(ops)
    for k, v in {"x": 1, "y": "dua"} { wok(k, v) }
}
"#),
    // Fase AMAN-38 (#938): `Option<any>` ke `Option<T>`, lokal yang berganti jenis di ekor blok, dict berisi dict,
    // `arr::min`/`max` atas slice, argumen dari fungsi ber-closure.
    ("None tanpa petunjuk ke field Option, v = v[k] di ekor blok, dict berisi dict", r#"package main
struct O { pub nama: string, pub bawaan: Option<string> = None }
fn buat(n, b) { return O { nama: n, bawaan: b } }
fn ambil(data, j) {
    var v = data
    for _, k in str::split(j, ".") {
        if typeof(v) == "dict" {
            if !dict::has(v, k) { return Err("tidak ada") }
            v = v[k]
        } else {
            return Err("bukan dict")
        }
    }
    return Ok(v)
}
fn cari(cs, dari, a) {
    var i = dari
    while i < len(cs) {
        if cs[i] == a { return i }
        i += 1
    }
    return -1
}
pub fn main() {
    wok(buat("a", None), buat("b", Some("x")))
    let d = {"a": {"b": "x"}, "c": "y"}
    wok(ambil(d, "a.b"), ambil(d, "c"), ambil(d, "c.z"))
    let cs = arr::map(str::chars("bbab"), |r| str(r))
    wok(cari(cs, 0, "a"))
    var xs: []float = []
    xs = arr::push(xs, 4.5)
    xs = arr::push(xs, 2.0)
    wok(arr::min(xs), arr::max(xs))
    let ini = {"": {"nama": "jrg"}, "s": {"port": 8080, "host": "h"}}
    for b, isi in ini { for k, v in isi { wok(b, k, v) } }
}
"#),
    // Fase AMAN-37 (#926–#929): urutan atas array bertag, `sort_desc` stabil, literal campur, konstanta modul.
    ("urutan array bertag, sort_desc stabil, literal campur dari variabel", r#"package main
const NAMA = ["nol", "satu", "dua"]
pub fn main() {
    var a = []
    a = arr::push(a, 2)
    a = arr::push(a, 2.0)
    a = arr::push(a, 1)
    wok(arr::sort_desc(a), arr::sort(a))
    var h = []
    for _, k in ["pisang", "apel", "jeruk", "anggur"] {
        let n = len(k)
        let i = 0
        h = arr::push(h, [n, i, k])
    }
    wok(arr::sort(h), arr::sort_desc(h))
    var x = []
    x = [x, 1]
    var y = []
    y = [y, 2]
    var t = []
    t = [t, true]
    var f = []
    f = [f, false]
    wok(x < y, y < x, x <= x, f < t)
    wok(NAMA[1], NAMA[-1], len(NAMA))
    var s = ""
    for i, c in "aé😀中z" { s = s + str(i) + c }
    wok(s)
}
"#),
    // Fase AMAN-34 (#919): array/dict di nilai bertag ditampilkan dan dibandingkan penjelajah C tanpa rekursi.
    ("tampil array/dict bertag berisi ragam nilai", r#"package main
struct P { pub x: int, pub y: string }
enum W { Merah, Isi(int) }
pub fn main() {
    var a = []
    a = [a, 1, 2.5, "teks", 'r', true, nil]
    wok(a)
    var d = dict::new()
    d["k"] = 1
    d["s"] = 2
    var x = []
    x = [x, d, [d], P { x: 1, y: "b" }, W::Isi(3), W::Merah, Some(4), (1, "t")]
    wok(x)
    let kosong: dict<int> = dict::new()
    var y = []
    y = [y, kosong, [[]], [1.0, 2.0], -0.5]
    wok(y, str(y), len(str(y)))
    var z = []
    z = [z, [[1, 2], [3]], ["a", "b"]]
    wok(z)
}
"#),
    ("== atas array/dict bertag: urutan sisip dict, NaN, bulat lawan pecahan", r#"package main
fn f(x) { return x }
pub fn main() {
    var a = []
    var b = []
    a = [a, 1, "x"]
    b = [b, 1, "x"]
    var c = []
    c = [c, 1, "y"]
    var d = dict::new()
    d["b"] = 1
    d["a"] = 2
    var e = dict::new()
    e["a"] = 2
    e["b"] = 1
    var p = []
    p = [p, d]
    var q = []
    q = [q, e]
    var r = []
    r = [r, 2.0, math::NAN]
    var s = []
    s = [s, 2, math::NAN]
    wok(a == b, a == c, a != c, p == q, r == s, [a] == [b], a == [[], 1, "x"])
    var m = []
    m = [m, 1.0]
    var n = []
    n = [n, 1]
    wok(m == n, m == p, [] == a)
}
"#),
    // Fase AMAN-34 (#918): `x = [.., x, ..]` memindahkan `x`; salinan yang diambil sebelumnya tidak ikut berubah.
    ("x = [.., x, ..] memindahkan x, salinan lama utuh", r#"package main
pub fn main() {
    var a = []
    a = [a]
    var c = a
    a = [a, [3]]
    wok(a, c)
    var z = []
    var simpan = []
    var i = 0
    while i < 3 {
        simpan = arr::push(simpan, z)
        z = [z, i]
        i += 1
    }
    wok(z, simpan)
    var d = [1]
    let f = || len(d)
    d = [d, [2]]
    var e = [1]
    e = [e, e]
    wok(f(), d, e)
}
"#),
    // Fase AMAN-30 (#909–#910): base64 satu ejaan per isi (kelas CVE-2026-3446), `crypto::base64url_*` ketat.
    ("base64 dan base64url: satu isi satu ejaan", r#"package main
pub fn main() {
    for _, s in ["aGk=", " aGk= ", "aGk=aGk=", "aGl=", "aG k=", "a===", "ab=c", "YQ==", "YR==", "aGk"] {
        match crypto::base64_decode_bytes(s) {
            Ok(b) => wok("ok", b),
            Err(e) => wok(e),
        }
    }
    wok(crypto::base64url_encode([251, 255]), crypto::base64url_encode("hi"))
    wok(crypto::base64url_decode_bytes("-_8"), crypto::base64url_decode("aGk"))
    for _, s in ["aGk=", "a+b", "aGl", "a"] { wok(crypto::base64url_decode_bytes(s)) }
}
"#),
    // `env::os`/`env::arch`: satu daftar nama di tiga mesin (Rust `std::env::consts`, C praprosesor di runtime LLVM).
    ("env::os dan env::arch dari daftar nama yang sama", r#"package main
pub fn main() {
    let o = env::os()
    let a = env::arch()
    wok(arr::contains(["linux", "android", "macos", "windows", "freebsd", "tak-dikenal"], o))
    wok(arr::contains(["x86_64", "aarch64", "arm", "x86", "riscv64", "tak-dikenal"], a))
    wok(o, a, typeof(o))
}
"#),
    // Fase AMAN-29 (#907): `fs::safe_join` — `..`, jalur absolut, `\`, huruf drive, kendali jadi `Err`.
    ("fs::safe_join dan jalur yang keluar dari dasarnya", r#"package main
fn main() {
    let d = fs::join(fs::temp_dir(), "jrg-gabung-aman")
    fs::mkdir(fs::join(d, "statis/css"))
    fs::write(fs::join(d, "berkas.txt"), "x")
    wok(str::replace(fs::safe_join(d, "statis/css/a.css").unwrap(), d, "<d>"))
    wok(str::replace(fs::safe_join(d, "baru/belum/ada.txt").unwrap(), d, "<d>"))
    wok(fs::safe_join(d, "").unwrap() == d, fs::safe_join(d, "statis/").is_ok())
    wok(fs::join(d, "/etc/passwd"))
    for _, r in ["../rahasia", "statis/../../x", "/etc/passwd", "a\\b", "C:/w", "a//b", "./a", "a\nb"] {
        match fs::safe_join(d, r) {
            Ok(p) => wok("lolos", p),
            Err(e) => wok(str::replace(e, d, "<d>")),
        }
    }
    match fs::safe_join(fs::join(d, "berkas.txt"), "a") {
        Ok(p) => wok("lolos", p),
        Err(e) => wok(str::replace(e, d, "<d>")),
    }
    wok(fs::safe_join(fs::join(d, "tidak-ada"), "a").is_err())
}
"#),
    // Fase AMAN-28 (#906): `semver::` — awalan `v`, nol di depan, versi telanjang dan spasi di rentang jadi `Err`.
    ("semver::parse/compare/satisfies dan bentuk ambigunya", r#"package main
fn main() {
    let v = semver::parse("1.2.3-rc.10+linux.01").unwrap()
    wok(v)
    wok(v.pra[1] + 1, typeof(v.pra[0]))
    for _, p in [["1.0.0-alpha", "1.0.0-alpha.1"], ["1.0.0-beta.11", "1.0.0-beta.2"], ["1.0.0", "1.0.0-rc.1"], ["1.0.0+a", "1.0.0+b"]] {
        wok(p[0], p[1], semver::compare(p[0], p[1]).unwrap())
    }
    for _, r in [["1.9.0", "^1.2.3"], ["2.0.0", "^1.2.3"], ["0.3.0", "^0.2.3"], ["1.2.9", "~1.2.3"], ["1.3.0", ">1.2"],
                 ["2.0.0-rc.1", "<2.0.0"], ["1.3.0-beta", "^1.2.3"], ["1.2.3-beta.2", "^1.2.3-beta.1"], ["3.1.0", "<1 || >=3, <4"]] {
        wok(r[0], r[1], semver::satisfies(r[0], r[1]).unwrap())
    }
    for _, s in ["v1.2.3", "01.2.3", "1.2", "1.2.3-01", "1.2.3-a..b", "9223372036854775808.0.0", " 1.2.3"] {
        wok(semver::parse(s).unwrap_err())
    }
    for _, r in ["1.2.3", ">= 1.2 <2", "1.2.3 - 2.0.0", "1.x", ">=1,", ""] {
        wok(semver::satisfies("1.2.3", r).unwrap_err())
    }
    wok(semver::compare("1.0.0", "v1").unwrap_err())
}"#),
    // Fase AMAN-26 (#904): `toml::` — kunci/tabel kembar, tabel diperluas, bilangan di luar 64 bit jadi `Err`.
    ("toml::parse/stringify dan bentuk ambigunya", r#"package main
fn main() {
    let d = toml::parse("judul = \"jrg\"\nversi.utama = 1\n[paket]\nnama = 'x'\nangka = [1, 2.5, 0xff, -inf]\nw = 2024-02-29T10:00:00+07:00\n[[dep]]\nn = \"a\"\n[[dep]]\nn = \"b\"\n").unwrap()
    wok(d)
    wok(d.paket.angka, d.dep[1].n, typeof(d.paket.w))
    wok(toml::stringify(d).unwrap())
    for _, s in ["a = 1\na = 2", "[t]\n[t]", "a.b = 1\n[a]", "a = {b = 1}\na.c = 2", "a = [1]\n[[a]]",
                 "a = 9223372036854775808", "a = 012", "a = 1__0", "a = \"\\x41\"", "a = \"\\uD800\"",
                 "a = 1\rb = 2", "a = 2023-02-29", "a = {b = 1,}", "a = abc"] {
        wok(toml::parse(s).unwrap_err())
    }
    wok(toml::stringify(dict { a: (1, 2) }).unwrap_err())
    wok(toml::stringify([1]).unwrap_err())
    wok(toml::stringify({"a b": "\"\n", "c": [dict { d: 1 }], "e": dict {}}).unwrap())
}"#),
    // Fase AMAN-25 (#903): kepala `Host` dicocokkan dengan daftar izin; `Host` cacat dan kembar jadi `Err`.
    ("dok::host_check", r#"package main
fn req(hs) {
    return dict { headers: hs, body: "" }
}
fn main() {
    let izin = ["contoh.id", ".jrg.web.id", "127.0.0.1:8080", "[::1]:8080"]
    for _, h in ["contoh.id", "CONTOH.id.", "a.jrg.web.id:443", "jrg.web.id", "evil-jrg.web.id", "127.0.0.1:8080",
                 "127.0.0.1:9090", "localhost", "[0::1]:8080", "0x7f.0.0.1:8080", "contoh.id@evil", "contoh.id:"] {
        wok(h, dok::host_check(req([("Host", h)]), izin))
    }
    wok(dok::host_check(req([]), izin))
    wok(dok::host_check(req([("host", "contoh.id"), ("Host", "evil")]), izin))
    wok(dok::host_check(req([("X-Forwarded-Host", "contoh.id"), ("Host", "evil")]), izin))
}
"#),
    // Fase AMAN-24 (#902): badan formulir dan multipart diurai ketat, pengalihan cuma ke jalur lokal.
    ("dok::form, dok::multipart, dok::safe_redirect", r##"package main
fn req(ct, body) {
    return dict { headers: [("content-type", ct)], body: body }
}
fn main() {
    let f = dok::form(req("application/x-www-form-urlencoded; charset=UTF-8", "a=1&b=x+y&a=%C3%A9&;c=2"))
    wok(f)
    wok(dok::form(req("text/plain", "a=1")).unwrap_err())
    wok(dok::form(req("multipart/form-data; boundary=x", "")).unwrap_err())
    wok(dok::form(req("application/x-www-form-urlencoded", "a=%zz")).is_err())
    let b = "--B\r\nContent-Disposition: form-data; name=\"teks\"\r\n\r\nhalo\r\n--B\r\nContent-Disposition: form-data; name=\"f\"; filename=\"../../etc/passwd\"\r\nContent-Type: text/plain\r\n\r\nisi\r\n--B--\r\n"
    let m = dok::multipart(req("multipart/form-data; boundary=B", b)).unwrap()
    wok(len(m), m[0]["nama"], m[0]["isi"], m[1]["berkas"], m[1]["tipe"])
    wok(dok::berkas_aman(m[1]["berkas"]), dok::berkas_aman(".."), dok::berkas_aman("NUL.txt"))
    let dua = "--B\r\nContent-Disposition: form-data; name=\"a\"; filename=\"x\"; filename*=UTF-8''y\r\n\r\nz\r\n--B--"
    wok(dok::multipart(req("multipart/form-data; boundary=B", dua)).unwrap_err())
    wok(dok::multipart(req("multipart/form-data; boundary=B", "--B\r\nContent-Disposition: form-data; name=a\r\n\r\nz")).unwrap_err())
    wok(dok::multipart(req("multipart/form-data", "")).unwrap_err())
    for _, t in ["/akun", "//evil.com", "/\\evil.com", "https://evil.com", "/..//evil", "/a/../b?x=1"] {
        let r = dok::safe_redirect(t, "/")
        wok(t, dok::header_get(r.headers, "Location"))
    }
}
"##),
    // Fase AMAN-23 (#901): satu pengurai URL; URL yang dibaca lain oleh pengurai lain jadi `Err`.
    ("url:: dan URL ambigu", r##"package main
pub fn main() {
    let u = url::parse("HTTPS://Pak:rahasia@Contoh.ID/a/./b/../c?x=1&x=2#f").unwrap()
    wok(u.skema, u.host, u.port, u.jalur, u.query, u.fragmen, u.pengguna, u.sandi)
    wok(dict::has(url::parse("http://h/").unwrap(), "query"), url::parse("http://[0::1]:8080/%2e%2E/x").unwrap().host)
    for _, r in ["g", "../g", "/g", "//q/r", "?y", "#s", "", "../../../g", "HTTPS://X"] {
        wok(url::join("http://a/b/c/d;p?q", r).unwrap())
    }
    for _, s in [" http://a/", "http://a/\tb", "http://a\\@b/", "http://a@b@c/", "http:a", "/rel", "http://0x7f.1/",
                 "http://2130706433/", "http://0177.0.0.1/", "http://%31.2.3.4/", "http://ä.id/", "http://a:0/",
                 "http://[fe80::1%25e]/", "http://a/%zz", "http:///x"] {
        wok(url::parse(s).unwrap_err())
    }
    wok(url::build(dict { skema: "https", host: "a.id", port: 8443, jalur: "/x", query: "q=1" }).unwrap())
    wok(url::build(dict { skema: "https", host: "a.id/@evil" }).unwrap_err())
    wok(url::build(dict { skema: "https", host: "a.id", jalur: "x" }).unwrap_err())
    wok(url::query_parse("?a=1&a=2;b=3&&c&d=x+y%2B%C3%A9").unwrap())
    wok(url::query_parse("a=%ff").unwrap_err())
    wok(url::query_build([["a b", "x&y=z"], ["k", "é+"]]), url::query_build(dict { b: 2, a: "1" }))
    let p = url::query_build([["a", "1&admin=1"]])
    wok(p, url::query_parse(p).unwrap())
    wok(str::contains(http::get("http://u:p@127.0.0.1:1/").unwrap_err(), "kredensial"))
}
"##),
    // Fase AMAN-20 (#895–#898): penjaga SSRF menilai alamat hasil DNS, juga ejaan angka dan IPv6 pembungkus.
    ("http::hanya_publik menolak alamat internal", r#"package main
pub fn main() {
    wok(str::contains(http::get("http://127.0.0.1:1/").unwrap_err(), "sambungan ditolak"))
    http::hanya_publik(true)
    for _, u in ["http://127.0.0.1:1/", "http://localhost:1/", "http://[::1]:1/", "http://[::ffff:169.254.169.254]/",
                 "http://169.254.169.254/latest", "http://10.0.0.1/"] {
        wok(str::contains(http::get(u).unwrap_err(), "hanya_publik(true)"))
    }
    // Ejaan angka IPv4 tak baku ditolak pengurai `url::` sebelum sampai ke DNS (Fase AMAN-23).
    wok(str::contains(http::get("http://0x7f.1:1/").unwrap_err(), "tak baku"), str::contains(http::get("http://2130706433:1/").unwrap_err(), "tak baku"))
    wok(dok::http_get("http://127.0.0.1:1/").unwrap_err())
    wok(http::unduh("http://192.168.0.1/x", "x.bin").is_err())
    http::hanya_publik(false)
    wok(str::contains(http::get("http://[::1]:1/").unwrap_err(), "sambungan ditolak"))
}
"#),
    // Fase AMAN-19 (#893–#894): token CSRF terikat sesi; sandi panjang tidak dihash tiap putaran.
    ("dok::csrf_token terikat sesi, sandi panjang", r#"package main

fn main() {
    let r = "rahasia-yang-panjangnya-32-byte!"
    let t = dok::csrf_token(r, "sesi-a")
    wok(len(str::split(t, ".")), dok::csrf_verify(t, r, "sesi-a", 60))
    wok(dok::csrf_verify(t, r, "sesi-b"), dok::csrf_verify(t + "x", r, "sesi-a"))
    wok(dok::csrf_verify(t, "rahasia-lain-yang-juga-32-byte!!", "sesi-a"), dok::csrf_verify("", r, "sesi-a"))
    wok(dok::csrf_verify("1.AAAAAAAAAAAAAAAAAAAAAA.x", r, "sesi-a"))
    let h = dok::password_hash("a" * 100000, 1000)
    wok(dok::password_verify("a" * 100000, h), dok::password_verify("a" * 99999, h))
}
"#),
    // Fase AMAN-18 (#892): `dok::render` menempatkan nilai menurut konteks HTML — URL berskema berbahaya
    // diganti penanda, nilai di `<script>`, atribut `on*` dan atribut tanpa kutip ditolak.
    ("dok::render menurut konteks HTML", r#"package main
pub fn main() {
    let d = {"u": "javascript:alert(1)", "ok": "/cari?q=a&b=1", "t": "x onclick=y"}
    wok(dok::render("<a href=\"{{u}}\">{{t}}</a><a href='{{ok}}'>", d)?)
    wok(dok::render("<a href=\"{{u}}\">", {"u": "JaVa\tScRiPt:x"})?)
    wok(dok::render("<script>let s = '{{t}}'</script>", d))
    wok(dok::render("<p title={{t}}>", d))
    wok(dok::render("<b onclick=\"f('{{t}}')\">", d))
    wok(dok::render("<textarea>{{t}}</textarea><script>{{{ t }}}</script>", d)?)
}
"#),
    // Fase AMAN-17 (#891): JWT HS256 dipatok — `alg: none` dan rahasia lain ditolak, `exp` diperiksa.
    ("jwt::sign/verify dan alg none", r#"package main
pub fn main() {
    let k = "rahasia-yang-panjangnya-32-byte!"
    let t = jwt::sign({"sub": "ani", "peran": "admin", "exp": 4102444800}, k)?
    wok(t)
    let klaim = jwt::verify(t, k)?
    wok(klaim.sub, klaim.peran, klaim.exp)
    wok(jwt::verify(t, "rahasia-lain-yang-juga-32-byte!!"))
    match jwt::verify(jwt::sign({"exp": 1000}, k)?, k) { Ok(_) => wok("BAHAYA"), Err(e) => wok(str::contains(e, "kedaluwarsa")) }
    wok(jwt::verify("eyJhbGciOiJub25lIn0.eyJhZG1pbiI6dHJ1ZX0.", k))
    wok(jwt::verify("a.b", k))
}
"#),
    // Fase AMAN-16 (#889–#890): `xml::` — DTD ditolak (XXE, ledakan entitas), atribut ganda, penulis tak bisa
    // disisipi; di LLVM juga akses bersarang atas nilai bertag (`doc.atribut.id`, `parse(..)?.atribut.t`).
    ("xml::parse/stringify/escape dan serangan klasiknya", r#"package main
fn teks_semua(n) -> string {
    var hasil = ""
    for _, a in n.anak { hasil += typeof(a) == "string" $ a : teks_semua(a) }
    return hasil
}
pub fn main() {
    let doc = xml::parse("<?xml version='1.0'?><buku id='7' judul=\"Jawa &amp; Rust\"><bab n='1'>Halo <b>dunia</b></bab><![CDATA[<mentah>]]></buku>")?
    wok(doc.nama, doc.atribut.id, doc.atribut.judul, len(doc.anak))
    wok(doc.anak[0].nama, doc.anak[0].atribut.n, teks_semua(doc))
    wok(xml::stringify(doc)?)
    wok(xml::stringify({"nama": "a", "atribut": {"t": "x\"y<z>&\n"}, "anak": ["1 < 2", {"nama": "b"}]})?)
    wok(xml::parse(xml::stringify({"nama": "a", "atribut": {"t": "x\"y\nz"}})?)?.atribut.t == "x\"y\nz")
    wok(xml::escape("<a href=\"x\">&</a>"))
    wok(xml::parse("<!DOCTYPE r [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><r>&x;</r>"))
    wok(xml::parse("<a>\n  <b x='1' x='2'/>\n</a>"))
    wok(xml::parse("<a><b></a>"))
    wok(xml::stringify({"nama": "a onload=alert(1)"}))
    wok(xml::stringify({"nama": "a", "atribut": {"x><script": "1"}}))
    wok(xml::stringify({"nama": "a", "anak": ["\u0001"]}))
}
"#),
    // Fase AMAN-14 (#884–#885): pembanding waktu-tetap lewat jembatan; initializer modul yang membuat lokal.
    ("crypto::compare_digest dan initializer modul berlokal", r#"package main
let KUNCI = env::get("JRG_TIDAK_PERNAH_DISET_XYZ") ?? "bawaan"
let N = match Some(3) { Some(v) => v * 2, None => 0 }
const B = { let a = 2; a * 3 }
var C = if true { let z = 1; z + 1 } else { 0 }
pub fn main() { C += 1
    wok(KUNCI, N, B, C)
    let tanda = crypto::hmac_sha256("kunci", "pesan")
    wok(crypto::compare_digest(tanda, crypto::hmac_sha256("kunci", "pesan")))
    wok(crypto::compare_digest(tanda, crypto::hmac_sha256("kunci", "pesan!")))
    wok(crypto::compare_digest("abc", "abcd"), crypto::compare_digest("", ""))
    wok(crypto::compare_digest([1, 2, 255], [1, 2, 255]), crypto::compare_digest([1, 2], "\u0001\u0002"))
}
"#),
    // Fase AMAN-12 (#879): nama non-ASCII yang sah jadi simbol native seperti nama ASCII.
    ("nama non-ASCII yang sah", r#"package main
fn luas(Δx, Δy) { Δx * Δy }
struct Titik { pub 名前: string, pub café: int }
pub fn main() {
    let имя = "Иван"
    let 日本語 = 3
    let a日b = 4
    let σ_x = 0.5
    let t = Titik { 名前: "x", café: 7 }
    wok(имя, 日本語 + a日b, σ_x, luas(2, 3), t.名前, t.café)
    wok("pаypal", len("café"), "аdmin" == "admin")
}
"#),
    // Fase AMAN-11 (#877): `len(c)` atas channel dulu ditolak LLVM ("tidak punya panjang").
    ("len atas channel", r#"package main
fn isi(c: chan int) -> int { len(c) }
pub fn main() {
    let c = make_chan(int, 4)
    wok(len(c), c.len())
    c <- 1
    c <- 2
    wok(len(c), isi(c), chan::len(c))
    let _ = <-c
    wok(len(c) + 10)
}
"#),
    // Fase AMAN-10 (#875): `kap * ukuran` dulu membungkus jadi 0 dan kiriman menulisi penunjuk kosong (SIGSEGV).
    ("channel berkapasitas raksasa tumbuh bertahap", r#"package main
pub fn main() {
    let c = make_chan(int, 2305843009213693952)
    for i in 0..5 { c <- i }
    wok(<-c, <-c, <-c)
    var i = 5
    while i < 40 { c <- i
        i = i + 1 }
    var s = 0
    for i in 0..37 { s = s + <-c }
    wok(s, chan::len(c))
    let k = make_chan(string, 3)
    k <- "a"
    k <- "b"
    wok(<-k)
    k <- "c"
    k <- "d"
    wok(<-k, <-k, <-k)
}
"#),
    // Fase REGEX-LINEAR (#841): mesin regex lama tidak pernah mundur ke dalam pengulangan.
    ("regex: mundur, malas, escape, kecocokan kosong", r#"package main
pub fn main() {
    wok(re::test("^.*x$", "abx"), re::test("a*a", "aaa"), re::test("^(a|ab)c$", "abc"), re::test("^[\\d]+0$", "100"))
    wok(re::find_all("a+?", "aaa"), re::find("\\bab\\b", "xab ab"))
    wok(re::replace_all("x*", "abc", "-"), re::split("\\s*,\\s*", "a , b,c"), re::test("a\nb", "a\nb"), re::test("a\\nb", "a\nb"))
}
"#),
    // Fase DICT-LITERAL (#839): `dict { .. }` dulu ditolak LLVM ("butuh tipe struct"), dan dict
    // kosong (`dict {}`, `dict::from_pairs([])`) ditolak walau tempatnya menyebut tipenya.
    ("literal dict berfield, sebar, dan dict kosong", r#"package main
fn f(s: string, v: int) -> int { wok(s)
 v }
pub fn main() {
    let d = dict { a: 1, b: 2 }
    wok(d, d["a"], d == {"a": 1, "b": 2})
    let g = dict { b: f("b", 20), a: f("a", 9), ...d }
    wok(g, dict { ...d })
    var e: dict<string, int> = dict {}
    e["k"] = 4
    var h: dict<string, int> = dict::from_pairs([])
    h["z"] = 1
    var m = dict {}
    m["x"] = "teks"
    m["y"] = 2
    wok(e, h, m, len(dict {}))
    let c = dict { n: 1, s: "dua" }
    wok(c, c["s"])
}
"#),
    // Fase REKURSIF-LLVM (#837): tampil, `==` dan salin tipe yang memuat dirinya lewat slice dulu
    // diturunkan sebaris tanpa ujung — `jrgc` sendiri kehabisan stack saat kompilasi.
    ("tipe berulang lewat slice: tampil, sama, salin", r#"package main
struct N { pub v: int, pub anak: []N }
enum E { Daun(int), Cabang([]E) }
fn total(n: N) -> int { var s = n.v
 for _, a in n.anak { s += total(a) }
 s }
pub fn main() {
    let n = N { v: 1, anak: [N { v: 2, anak: [] }, N { v: 3, anak: [N { v: 4, anak: [] }] }] }
    wok(n, total(n))
    var m = n
    m.anak[0].v = 99
    wok(n.anak[0].v, m.anak[0].v, n == n, n == m)
    let e = E::Cabang([E::Daun(1), E::Cabang([E::Daun(2)])])
    let f = e
    wok(e, e == f, [e] == [f], str(n), «${e}»)
}"#),
    ("spec desimal atas nilai bertag", "package main\npub fn main() { let d = {\"a\": 1, \"b\": 2.5, \"c\": \"t\"}\n for _, k in [\"a\", \"b\"] { wok(str::format(\"{:.2}\", d[k])) } }"),
    // Fase POLIMORF: struct berbeda di satu tempat, method-nya dipilih saat jalan menurut tipe isinya —
    // default trait, method berargumen, dan method warisan.
    ("polimorfisme: method atas struct bertag", r#"package main
trait Bentuk {
    fn luas(self) -> float { return 0.0 }
    fn nama(self) -> string { return "bentuk" }
}
struct Lingkaran { r: float }
struct Persegi { s: float }
struct Hewan { pub nama_h: string = "x" }
struct Anjing: Hewan { pub ras: string = "kampung" }
impl Bentuk for Lingkaran {
    fn luas(self) -> float { return 3.0 * self.r * self.r }
    fn nama(self) -> string { return "lingkaran" }
}
impl Bentuk for Persegi {
    fn luas(self) -> float { return self.s * self.s }
}
impl Lingkaran { fn skala(self, k: float) -> float { return self.r * k } }
impl Persegi { fn skala(self, k: float) -> float { return self.s * k } }
impl Hewan { fn sapa(self) -> string { return "halo " + self.nama_h } }
fn cetak(b) { wok("-", b.nama(), b.luas(), b.skala(2.0)) }
pub fn main() {
    let xs = [Lingkaran { r: 1.0 }, Persegi { s: 3.0 }]
    for _, b in xs { cetak(b) }
    for i in xs { wok(xs[i].nama()) }
    let hs = [Anjing { nama_h: "rex" }, Hewan { nama_h: "kucing" }]
    for _, h in hs { wok(h.sapa()) }
}
"#),
    // Fase FUNGSI-JEMBATAN: fungsi JRG diserahkan ke builtin milik `jrgc` dan dipanggil balik.
    ("jembatan: closure ke ptr::update dan dok::num_solve", "package main\npub fn main() { let p = ptr::new(10)\n ptr::update(p, |n: int| n + 1)\n ptr::update(p, |n| n * 3)\n wok(ptr::get(p))\n wok(dok::num_solve(|x: float| x * x - 2.0, 0.0, 2.0)) }"),
    ("bertag: method bawaan teks dan array dipilih saat jalan", "package main\npub fn main() { let d = {\"a\": \"xyz\", \"b\": [3, 1, 2]}\n for _, k in [\"a\", \"b\"] { let v = d[k]\n wok(len(v), v.len(), v.contains(\"y\") || v.contains(1)) }\n let s = d[\"a\"]\n wok(str::len(s), s.upper()) }"),
    // Registry `jawa::` di LLVM milik runtime-nya; lewat jembatan ia dulu menjawab 0 tanpa pesan.
    ("extern C: fungsi sebagai argumen dijawab Err dari lapisan FFI", "package main\nextern \"C\" { fn atexit(f: fn()) -> i32 }\npub fn main() { let f = || 1; wok(atexit(f)) }"),
    ("ternary dan if bercabang dua separuh Result/Option tanpa anotasi", "package main\npub fn main() {\n    let r = 5 > 2 $ Result::Ok(1) : Result::Err(\"y\")\n    let s = 1 > 2 $ Result::Err(\"z\") : Result::Ok(2.5)\n    let o = 1 > 2 $ Option::Some(4) : Option::None\n    let p = 3 > 2 $ Option::None : Option::Some(\"a\")\n    wok(r, s, o, p)\n    match s { Ok(v) => wok(v * 2.0), Err(e) => wok(e + \"!\") }\n    wok(if 2 > 1 { Result::Ok([1, 2]) } else { Result::Err(5) })\n}"),
    ("dict::set dan json::set dengan nilai berjenis lain dari isinya", "package main\npub fn main() {\n    let d = {\"a\": 1}\n    wok(json::set(d, \"b\", \"teks\"), d)\n    let f = dict::set(d, \"c\", [1, 2])\n    wok(f, f[\"a\"], f[\"c\"], dict::set(d, \"z\", 9))\n}"),
    ("d[k] = v dan d.f = v berjenis lain atas dict bertipe", "package main\npub fn main() {\n    var d = {\"a\": 1}\n    d[\"b\"] = \"x\"\n    d.c = 2.5\n    wok(d, d[\"a\"] + 1)\n}"),
    ("menulis field atas nilai titipan jrgc: salinannya, bukan aslinya", "package main\npub fn main() {\n    let d = dok::json_decode(\"{\\\"a\\\": 1}\").unwrap()\n    var e = d\n    e.b = \"dua\"\n    e.a = 3\n    wok(d, e, e.b, d.a)\n}"),
    ("spread dan argumen bernama ke fungsi bernama, flatten datar, replace berpengganti bukan teks", r#"package main
fn tambah(a: int, b: int) -> int { return a + b }
fn sapa(nama: string, salam: string = "halo", tanda: string = "!") -> string { return salam + " " + nama + tanda }
fn n(x: int) -> int { wok("hitung", x); return x }
fn baca() -> []int { return [5, 6] }
pub fn main() {
    let xs = [3, 4]
    wok(tambah(...xs))
    wok(tambah(...baca()))
    wok(tambah(1, ...[2]))
    wok(terap(tambah, [1, 2]))
    var ys: []int = []
    ys = ys + [7]
    ys = ys + [8]
    wok(tambah(...ys))
    wok(sapa(...["ani"]))
    wok(sapa(...["ani", "hai"]))
    wok(sapa("budi", salam: "pagi"))
    wok(sapa("budi", tanda: "?", salam: "malam"))
    wok(sapa(nama: "cici"))
    wok(tambah(b: n(1), a: n(2)))
    wok(arr::flatten([1, 2, 3]), arr::flatten([[1], 2, [3, 4]]), arr::flatten([[1, 2], [3]]))
    wok([1, 2].flatten(), arr::flat_map([1, 2], |x| x * 10), [1, 2].flat_map(|x| [x, x]))
    wok(str::replace("a-b-c", "-", 1), "x.y".replace(".", true), "a.b".replace(".", [1, 2]))
}"#),
    ("goroutine tetap mendapat giliran selama main menunggu proses::", r#"package main
pub fn main() {
    let p = "/tmp/jrg-kor-" + str(rand::int(0, 1000000000))
    spawn { time::sleep_ms(30); fs::write(p, "x"); wok("ditulis") }
    let r = proses::keluaran("sh", ["-c", "for i in $(seq 500); do [ -f \"$1\" ] && break; sleep 0.01; done; [ -f \"$1\" ] && echo ada || echo tidak", "_", p])
    wok(r.unwrap().keluaran.trim())
    fs::delete(p)
}"#),
    ("method atas field nilai bertag", r#"package main
pub fn main() {
    let u = proses::keluaran("echo", ["hai"]).unwrap()
    wok(u.keluaran.trim(), u.keluaran.trim().len())
    let d = dok::json_decode("{\"a\": \" x \", \"b\": [1, 2]}").unwrap()
    wok(d.a.trim(), d.b.len(), dok::json_decode("{\"c\": \"q\"}").unwrap().c.upper())
}"#),
    ("d[k] = v berjenis lain atas var modul dan dict bersarang", r#"package main
var D = {"a": 1}
var E = {"n": 5}
fn isi() { D["b"] = [true] }
fn bayang() { var E = {"z": 1}; E["z"] = 2; wok(E) }
pub fn main() {
    isi()
    wok(D, D["a"])
    E["n"] = E["n"] + 1
    wok(E["n"] * 2)
    bayang()
    var e = {"p": {"q": 1}}
    e["p"]["r"] = "t"
    wok(e["p"]["q"], e["p"]["r"], len(e["p"]))
    for k, v in e["p"] { wok(k, v) }
}"#),
    ("jawa::stats dari registry biner sendiri", "package main\npub fn main() { let a = jawa::alloc(1)\n let b = jawa::array(3, 0)\n wok(jawa::stats().alive_total, jawa::stats())\n jawir::free(a)\n wok(jawa::stats().alive_total, typeof(jawa::stats()))\n jawir::free(b) }"),
];

// ─── Tabel 2: gap yang diketahui
// ───────────────────────────────────────────

const BELUM_DIDUKUNG_LLVM: &[(&str, &str, &str)] = &[
    // Fase POTONG-BERTAG (#848): tipe elemennya tidak diketahui dari tag; dulu ditolak saat kompilasi.
    ("potong: array di dalam nilai bertag",
     "package main\npub fn main() { let v = len(env::args()) > 99 $ 5 : [1, 2, 3]\n wok(v[0:2]) }",
     "di dalam nilai bertag belum didukung backend LLVM"),
    // Fase POTONG: tipe hasil `t[a:b]` bergantung pada batasnya.
    ("potong: tuple berbatas variabel", "package main\npub fn main() { let t = (1, 2, 3)\n let i = 1\n wok(t[i:]) }",
     "batas potongan tuple di backend LLVM harus angka yang sudah pasti saat kompilasi"),
    // Fase SEBAR-TIPE: dua mesin lain menyimpan `2` di field `string`; di LLVM tipe field tetap.
    ("sebar membawa field berjenis lain dari sasarannya",
     "package main\nstruct A { pub x: int, pub y: string }\nstruct B { pub x: int, pub y: int }\npub fn main() { let b = B { x: 1, y: 2 }\n let a = A { x: 5, ...b }\n wok(a.y) }",
     "field `y` dari basis sebar bertipe `int`"),
    // Fase KEPUTUSAN-1: dulu literalnya diam-diam jadi `2.0` (dua mesin lain menyimpan `2`).
    ("literal bulat di let beranotasi float",
     "package main\npub fn main() { let x: float = 7\n wok(x / 2) }",
     "ditulis `7.0`"),
    // Fase SUBSTR: eksponen negatif yang baru diketahui saat jalan; `int ** int` bertipe `int`
    // di LLVM, dua mesin lain menjawab `0.5`.
    ("pangkat bulat dengan eksponen negatif dari variabel",
     "package main\npub fn main() { let n = -1\n wok(2 ** n) }",
     "`2 ** -1`: pangkat negatif menjawab pecahan"),
    // Fase SAPU-29: acuan menjawab `nil`; LLVM dulu menjawab `0`, sekarang menolak saat jalan.
    ("jawa::array: elemen yang belum pernah diisi",
     "package main\npub fn main() { let a = jawa::array(3)\n wok(jawa::array_get(a, 1)) }",
     "belum pernah diisi"),
    ("jawa::set: rune disimpan sebagai bilangan",
     "package main\npub fn main() { let p = jawa::alloc(1)\n jawa::set(p, 'a')\n wok(jawa::get(p)) }",
     "cuma menyimpan bilangan bulat, diberi `rune`"),
    // Dua kotak berisi tuple sama panjang tapi beda tipe elemen: acuan membandingkan per elemen
    // (`true`); LLVM dulu menjawab `false`, sekarang menolak saat jalan.
    ("nilai bertag: == dua tuple beda tipe elemen",
     "package main\npub fn main() { var u = nil\n var v = nil\n u = (1, 2)\n v = (1.0, 2.0)\n wok(u == v) }",
     "membandingkan dua tuple yang tipe elemennya berbeda di tempat campur tipe belum didukung backend"),
    // Fase UMUM: struct di dalam nilai bertag dibuka sebagai struct LAIN — gagal saat jalan,
    // menyebut kedua tipenya (acuan mengganti tipe variabelnya).
    ("nilai bertag: struct dibuka sebagai struct lain",
     "package main\nstruct P { pub x: int }\nstruct Q { pub y: int }\npub fn main() { var y = nil\n y = P { x: 7 }\n let q: Q = y\n wok(q.y) }",
     "nilai bertipe `struct:P` tidak bisa mengisi tempat yang bertipe `struct:Q`"),
    // Fase KORUTIN: kebuntuan DILAPORKAN di LLVM (korutin satu utas bisa melihatnya); dua mesin
    // lain menggantung selamanya, jadi kasus ini tidak pernah dijalankan di sana.
    ("kebuntuan: menerima dari channel yang tidak punya pengirim",
     "package main\npub fn main() { let c = make_chan(int)\n wok(<-c) }",
     "kebuntuan: semua goroutine sedang menunggu channel"),
    // Fase ISI: kotak bertag mencatat isi array/dict-nya. Dulu dua program ini mencetak sampah
    // tanpa pesan (pemeriksaan tagnya cuma tahu "ini array").
    ("nilai bertag: array<int> dibuka sebagai []string", "package main\npub fn main() {\n    var x = nil\n    x = [1, 2, 3]\n    let a: []string = x\n    wok(len(a))\n    wok(a[0])\n}\n",
     "nilai bertipe `array<int>` tidak bisa mengisi tempat yang bertipe `array<string>`"),
    ("nilai bertag: isi dict campur dibuka sebagai []float", "package main\npub fn main() {\n    var d = {\"a\": 1, \"b\": \"teks\"}\n    d[\"c\"] = [1, 2, 3]\n    let a: []float = d[\"c\"]\n    wok(a)\n    let k = \"c\"\n    let b: [][]int = d[k]\n    wok(len(b))\n    wok(b)\n}\n",
     "nilai bertipe `array<int>` tidak bisa mengisi tempat yang bertipe `array<float>`"),
    // Fase BUNGKUS: nilai bertag yang tagnya tidak cocok dengan tipe variabelnya gagal saat
    // jalan menyebut tipe aslinya — acuan mengganti tipe variabelnya, backend statis tidak.
    ("nilai bertag ke parameter beranotasi tipe lain", "package main\nstruct Kotak { pub isi: array }\nfn f(n: int) { wok(n) }\npub fn main() { let t = Kotak { isi: [\"a\"] }\n f(t.isi[0]) }",
     "nilai bertipe `string` tidak bisa mengisi tempat yang bertipe `int`"),
    // Fase LEBAR-LLVM: `+`/`-`/`*`, negasi, `arr::sum` dan `math::abs` dulu MEMBUNGKUS di
    // lebar tipenya; sekarang gagal saat jalan sambil menyebut hasil yang sebenarnya.
    ("int 64 bit: penjumlahan yang meluap dilaporkan", "package main\npub fn main() { var x = 9223372036854775807\n wok(x + 1) }",
     "= 9223372036854775808 tidak muat di bilangan bulat 64 bit bertanda"),
    ("int 64 bit: perkalian dan negasi yang meluap dilaporkan", "package main\npub fn main() { var x = 4000000000\n var m = -9223372036854775807 - 1\n if x > 0 { wok(-m) }\n wok(x * x) }",
     "tidak muat di bilangan bulat 64 bit bertanda"),
    ("uint: pengurangan di bawah nol dilaporkan", "package main\npub fn main() { var x: uint = 5\n wok(x - 6) }",
     "= -1 tidak muat di bilangan bulat 64 bit tak bertanda"),
    ("arr::sum dan math::abs yang meluap dilaporkan", "package main\npub fn main() { let a = [9223372036854775807, 1]\n wok(arr::sum(a)) }",
     "tidak muat di bilangan bulat 64 bit bertanda"),
    // Fase BAGI-LLVM: dulu membungkus diam-diam (`math::lcm` menjawab angka karangan).
    ("math::lcm dan gcd di luar 64 bit dilaporkan", "package main\npub fn main() { var a = 3037000500\n var b = 3037000501\n wok(math::lcm(a, b)) }",
     "math::lcm: hasilnya 9223372040037250500 tidak muat di bilangan bulat 64 bit bertanda"),
    // Fase BAGI-LLVM: `sdiv` atas MIN dan -1 tak terdefinisi; dulu menjawab `1`.
    // Fase GESER-LLVM (#831): dulu `1 << 63` menjawab negatif dan `3 << 62` membuang bitnya diam-diam.
    ("int 64 bit: geseran yang membuang bit dilaporkan", "package main\npub fn main() { var k = 62\n wok(1 << k)\n wok(3 << k) }",
     "hasil `3 << 62` = 13835058055282163712 tidak muat di bilangan bulat 64 bit bertanda"),
    ("int 64 bit: MIN / -1 dilaporkan", "package main\npub fn main() { var m = -9223372036854775807 - 1\n var n = -1\n wok(m / n) }",
     "= 9223372036854775808 tidak muat di bilangan bulat 64 bit bertanda"),
    // Fase PACU-3: dulu dipotong diam-diam (`-1`, `44`); literalnya sekarang ditolak.
    ("literal di atas 64 bit", "package main\npub fn main() { let m = 170141183460469231731687303715884105727\n wok(m - 1) }",
     "tidak muat di `int` backend LLVM"),
    ("literal tidak muat di anotasinya", "package main\npub fn main() { let x: u8 = 300\n wok(x) }",
     "tidak muat di `u8`"),
    // Tahap 5: dulu DIVERGENSI — LLVM mengarang `D {x: 0}`, `Some(0)`, varian pertama.
    ("var struct tanpa nilai awal", "package main\nstruct D { x: int = 5 }\npub fn main() { var a: D\n wok(a) }",
     "tidak punya nilai nol"),
    ("var Option tanpa nilai awal", "package main\npub fn main() { var a: Option<int>\n wok(a) }",
     "tidak punya nilai nol"),
    ("var enum tanpa nilai awal", "package main\nenum Warna { Merah, Hijau }\npub fn main() { var a: Warna\n wok(a) }",
     "tidak punya nilai nol"),
    // "extern C: fungsi sebagai argumen" (sampai Fase FUNGSI-JEMBATAN) dan
    // "urutan pada array" (sampai Fase UMUM) dulu di sini.

    // "variabel berganti tipe" DULU di sini, dan ia satu-satunya entri di tabel
    // ini yang tidak selalu menolak: `var x = 1; x = 2.5` dikompilasi dan …

        ("channel", r#"package main
pub fn main() {
    var c: chan int = make(chan int, 1)
    wok(1)
}"#, "chan"),

    // "goroutine spawn" dulu di sini; sejak Fase KORUTIN `spawn` jalan sebagai korutin.

    // Kesetaraan enum DULU di sini, dengan alasan yang benar: payload berbagi
    // tempat antar variant, jadi membandingkan blobnya langsung menjawab SALAH.

    // Literal dict diturunkan jadi panggilan `dict::from_pairs`, jadi pesannya
    // harus mengakui `dict::` sebagai namespace stdlib yang nyata — bukan …

    // Indeks tuple non-konstan: bukan kemalasan, tapi konsekuensi tipe.

    // Dict bernilai campur tipe DULU di sini, dengan alasan yang benar: satu
    // buffer pasangan `{ %JrgStr, V }` cuma punya satu `V`, jadi nilai campur …
    // Fase SEBAR-LLVM: ke fungsi bernama milik program sudah jalan; ke NILAI fungsi belum.
    ("spread di pemanggilan nilai fungsi",
     "package main\npub fn main() { let tambah = |a, b| a + b\n wok(tambah(...[1, 2])) }",
     "spread di pemanggilan"),

    // Argumen bernama: urutannya ditentukan dari nama parameter fungsinya saat
    // JALAN. Yang dikunci di sini bahwa penolakannya menyebut sebabnya, bukan
    // nama turunan `terap_bernama` yang pemakainya tidak pernah tulis.
    ("argumen bernama ke nilai fungsi",
     "package main\npub fn main() { let bagi = |a, b| a / b\n wok(bagi(a: 10, b: 2)) }",
     "argumen bernama"),


    // Fase SINKRON-LLVM: `Atomic` di backend ini 64 bit, dan luapannya ditolak saat jalan.
    ("atomic::add melewati 64 bit", "package main\npub fn main() { let a = Atomic::new(9223372036854775807)\n wok(atomic::add(a, 1)) }",
     "atomic::add: hasil `9223372036854775807 + 1` = 9223372036854775808 tidak muat di bilangan bulat 64 bit"),
];

// ─── Tabel 3: harus DITOLAK interpreter DAN LLVM
// ───────────────────────────

/// `(nama, program, kata-kunci-interpreter, kata-kunci-llvm)`
const DITOLAK_INTERP_DAN_LLVM: &[(&str, &str, &str, &str)] = &[
    // Fase KUNCI-LLVM (#821): path yang tidak ada, dipanggil maupun sebagai nilai, kalimat acuan beserta sarannya.
    ("path: salah ketik builtin disarankan", "package main\npub fn main() {\n wok(math::sqr(4.0))\n}",
     "`math::sqr` tidak ditemukan — maksudnya `math::sqrt`?", "`math::sqr` tidak ditemukan — maksudnya `math::sqrt`?"),
    ("path: salah ketik namespace sebagai nilai", "package main\npub fn main() {\n let x = mathh::PI\n wok(x)\n}",
     "`mathh::PI` tidak ditemukan — maksudnya `math::PI`?", "`mathh::PI` tidak ditemukan — maksudnya `math::PI`?"),
    ("path: nama telanjang milik program", "package main\nfn f() { 1 }\npub fn main() {\n wok(arr::f)\n}",
     "`f` memang ada, tapi bukan sebagai isi `arr`", "`f` memang ada, tapi bukan sebagai isi `arr`"),
    ("path: nama ada di namespace lain", "package main\npub fn main() {\n wok(math::upper)\n}",
     "`math::upper` tidak ditemukan — `upper` memang ada, tapi sebagai `str::upper`", "`math::upper` tidak ditemukan — `upper` memang ada, tapi sebagai `str::upper`"),
    // Fase KUNCI-LLVM (#826): field privat lewat indeks teks.
    ("field privat dibaca lewat indeks teks (llvm)",
     "package main\nstruct P { pub x: int, y: int }\npub fn main() { let p = P { x: 1, y: 2 }\n wok(p[\"y\"]) }",
     "field `y` di `P` bersifat private", "field `y` di `P` bersifat private"),
    // Fase KUNCI-LLVM: field yang tidak ada (baca dan tulis), kalimat acuan; LLVM menolaknya saat kompilasi.
    ("struct: membaca field yang tidak ada", "package main\nstruct T { x: int }\npub fn main() {\n let t = T { x: 1 }\n wok(t.y)\n}",
     "`struct T` tidak punya method atau field bernama `y` (kalau maksudnya field/key, yang ada: x) — cek lagi penulisannya ya", "`struct T` tidak punya method atau field bernama `y` (kalau maksudnya field/key, yang ada: x) — cek lagi penulisannya ya"),
    ("struct: saran dari method warisan", "package main\nstruct A { x: int }\nimpl A { fn hitung(self) -> int { self.x } }\nstruct B: A { y: int }\npub fn main() {\n let b = B { x: 1, y: 2 }\n wok(b.hitng)\n}",
     "`struct B` tidak punya method atau field bernama `hitng` (kalau maksudnya field/key, yang ada: x, y) — maksudnya `hitung`?", "`struct B` tidak punya method atau field bernama `hitng` (kalau maksudnya field/key, yang ada: x, y) — maksudnya `hitung`?"),
    ("struct: menulis field yang tidak ada", "package main\nstruct T { x: int }\npub fn main() {\n var t = T { x: 1 }\n t.z = 3\n wok(t.x)\n}",
     "struct `T` tidak punya field `z` — maksudnya `x`?", "struct `T` tidak punya field `z` — maksudnya `x`?"),
    // Fase KALIMAT: nilai tanpa padanan JSON, kalimat penyandi yang sama di tiga mesin.
    ("json::stringify atas atomic", "package main\npub fn main() {\n let a = Atomic::new(1)\n wok(json::stringify(a))\n}",
     "`atomic` tidak punya padanan di JSON, jadi ia tidak bisa disandikan", "`atomic` tidak punya padanan di JSON, jadi ia tidak bisa disandikan"),
    ("json::stringify atas mutex", "package main\npub fn main() {\n let a = Mutex::new()\n wok(json::stringify(a))\n}",
     "`mutex` tidak punya padanan di JSON, jadi ia tidak bisa disandikan", "`mutex` tidak punya padanan di JSON, jadi ia tidak bisa disandikan"),
    ("json::stringify atas channel", "package main\npub fn main() {\n let a = make_chan(int)\n wok(json::stringify(a))\n}",
     "`channel` tidak punya padanan di JSON, jadi ia tidak bisa disandikan", "`channel` tidak punya padanan di JSON, jadi ia tidak bisa disandikan"),
    ("json::stringify atas fungsi", "package main\npub fn main() {\n let a = |x| x + 1\n wok(json::stringify(a))\n}",
     "`fungsi` tidak punya padanan di JSON, jadi ia tidak bisa disandikan", "`fungsi` tidak punya padanan di JSON, jadi ia tidak bisa disandikan"),
    // Fase SEBAR: kalimat yang sama di tiga mesin.
    ("basis sebar membawa field liar ke struct",
     "package main\nstruct A { pub x: int, pub w: int }\nstruct B { pub x: int, pub y: int = 7 }\npub fn main() {\n let a = A { x: 1, w: 2 }\n wok(B { ...a })\n}", "basis `...` membawa field `w` yang tidak dideklarasikan struct `B`", "basis `...` membawa field `w` yang tidak dideklarasikan struct `B`"),
    ("varian unit telanjang milik dua enum", "package main\nenum E { A(int), K, V { a: int } }\nenum F { A(int), K, V { a: int } }\npub fn main() {\n wok(K)\n}",
     "`K` ambigu — varian bernama itu ada di E dan F", "`K` ambigu — varian bernama itu ada di E dan F"),
    // Fase WAJIB: kalimat field wajib sama di tiga mesin (`galat_index::field_wajib*`).
    ("literal struct tanpa field wajib",
     "package main\nstruct B { x: int, pub y: int }\npub fn main() { wok(1)\n let b = B { x: 1 }\n wok(b) }",
     "struct `B`: field `y` wajib diisi (tidak ada nilai default)", "struct `B`: field `y` wajib diisi (tidak ada nilai default)"),
    ("literal struct tanpa field wajib milik induk",
     "package main\nstruct A { pub x: int }\nstruct B: A { pub y: int }\npub fn main() { wok(B { y: 2 }) }",
     "struct `B`: field `x` wajib diisi (tidak ada nilai default)", "struct `B`: field `x` wajib diisi (tidak ada nilai default)"),
    ("varian ber-field tanpa salah satu field",
     "package main\nenum E { V { a: int, b: int }, W }\npub fn main() { wok(E::V { a: 1 }) }",
     "varian `E::V` butuh field `b`", "varian `E::V` butuh field `b`"),
    ("literal varian telanjang yang melewatkan field",
     "package main\nenum E { V { a: int, b: int }, K }\npub fn main() {\n wok(V { a: 1 })\n}",
     "varian `E::V` butuh field `b`", "varian `E::V` butuh field `b`"),
    // Fase SINKRON-LLVM: kalimat `Atomic`/`Mutex` sama dengan acuan; jenis argumen yang salah pindah ke
    // `GAGAL_SAAT_JALAN` di Fase ATOM-LLVM.
    ("Atomic tidak punya urutan", "package main\npub fn main() { let a = Atomic::new(1)\n wok(a < a) }",
     "tidak bisa membandingkan `atomic` dengan `atomic`", "tidak bisa membandingkan `atomic` dengan `atomic`"),
    // Fase SAPU-29: `rune`, `bool` dan teks dulu dibaca LLVM sebagai angkanya (21 dari 1090).
    ("jenis lain di tempat angka: arr::get (llvm)", "package main\npub fn main() { wok(arr::get([1, 2], 'a')) }",
     "arr::get: butuh bilangan bulat, dapat `rune`", "arr::get: butuh bilangan bulat, dapat `rune`"),
    ("jenis lain di tempat angka: math::gcd (llvm)", "package main\npub fn main() { wok(math::gcd('a', 1)) }",
     "butuh bilangan bulat, dapat `rune`", "butuh bilangan bulat, dapat `rune`"),
    ("jenis lain di tempat angka: math::lcm (llvm)", "package main\npub fn main() { wok(math::lcm(2, 'a')) }",
     "butuh bilangan bulat, dapat `rune`", "butuh bilangan bulat, dapat `rune`"),
    ("jenis lain di tempat angka: str::repeat (llvm)", "package main\npub fn main() { wok(str::repeat(\"ab\", 'a')) }",
     "butuh bilangan bulat, dapat `rune`", "butuh bilangan bulat, dapat `rune`"),
    ("jenis lain di tempat angka: future::timeout (llvm)", "package main\npub fn main() { wok(future::timeout(Future::new(|| 1), 'a')) }",
     "dapat `rune`", "dapat `rune`"),
    ("jenis lain di tempat angka: math::degrees bool (llvm)", "package main\npub fn main() { wok(math::degrees(true)) }",
     "math::degrees: butuh angka, dapat `bool`", "math::degrees: butuh angka, dapat `bool`"),
    ("jenis lain di tempat angka: math::log teks (llvm)", "package main\npub fn main() { wok(math::log(2, \"7\")) }",
     "math::log: butuh angka, dapat `string`", "math::log: butuh angka, dapat `string`"),
    ("jenis lain di tempat angka: math::is_finite teks (llvm)", "package main\npub fn main() { wok(math::is_finite(\"7\")) }",
     "math::is_finite: butuh angka, dapat `string`", "math::is_finite: butuh angka, dapat `string`"),
    // Tahap 5: keenam kasus ini dulu DITERIMA LLVM dengan jawaban karangan (`LLVM_LEBIH_PERMISIF`).
    ("pangkat bulat yang meluap (llvm)", "wok(2 ** 4294967296)\n",
     "kelewat besar buat integer", "kelewat besar buat integer"),
    ("alamat pointer negatif (llvm)", "package main\npub fn main() { wok((-1) as *const int) }",
     "angka negatif tidak punya padanan", "cast ke tipe pointer belum didukung"),
    ("import std/ yang tidak ada (llvm)", "package main\nimport \"std/hantu\"\npub fn main() { wok(1) }",
     "modul standar 'std/hantu' tidak ada", "modul standar 'std/hantu' tidak ada"),
    ("factorial yang meluap (llvm)", "package main\npub fn main() { wok(math::factorial(40)) }",
     "kelewat besar buat integer 128 bit", "kelewat besar buat integer 128 bit"),
    ("let atas varian unit yang tidak cocok (llvm)", "let None = Option::Some(3)\nwok(\"lolos\")\n",
     "tidak cocok dengan polanya", "tidak cocok dengan polanya"),
    ("tulis lewat key dict yang belum ada (llvm)",
     "package main\npub fn main() { var d = {\"a\": {\"b\": 1}}\n d.zz.b = 5\n wok(d) }",
     "key `zz` belum ada", "key `zz` belum ada"),
    // Fase LLVM-16: `coerce` dulu memotong pecahan dan membaca `bool` sebagai angka.
    // Indeks bool/pecahan pindah ke `GAGAL_SAAT_JALAN` (#825).
    ("pecahan sebagai jumlah ulang (llvm)", "package main\npub fn main() {\n  wok(str::repeat(\"a\", 2.5))\n}",
     "butuh bilangan bulat, dapat pecahan", "str::repeat: butuh bilangan bulat, dapat pecahan `2.5`"),
    ("math::abs atas bool (llvm)", "package main\npub fn main() {\n  wok(math::abs(true))\n}",
     "math::abs: butuh angka, dapat `bool`", "math::abs: butuh angka, dapat `bool`"),
    // Fase SERPIH: `a[0:2] = …` dulu DITERIMA LLVM dan menulis ke salinan — `a` tetap.
    (
        "sasaran: potongan ditolak (llvm)",
        "package main\npub fn main() {\n  var a = [1, 2, 3]\n  a[0:2] = [9, 9]\n  wok(a)\n}",
        "potongan (`x[a:b]`) bukan sesuatu yang bisa di-assign",
        "potongan (`x[a:b]`) bukan sesuatu yang bisa di-assign",
    ),
    (
        "pembuang dibaca (llvm)",
        "package main\npub fn main() {\n  var _ = 1\n  _ = 2\n  wok(_)\n}",
        "`_` cuma untuk membuang nilai",
        "`_` cuma untuk membuang nilai",
    ),
    // Fase TERM: dulu `exit(2.5)` keluar 0 di acuan dan dikoersi ke i32 di LLVM.
    (
        "exit: kode bukan bilangan bulat ditolak (llvm)",
        "package main\npub fn main() {\n  exit(2.5)\n}",
        "kode keluar harus bilangan bulat 0 sampai 255, dapat `float`",
        "kode keluar harus bilangan bulat 0 sampai 255, dapat `float`",
    ),
    // Fase LITERAL: literal di atas `i128::MAX` ditolak PARSER, jadi tiga mesin satu
    // kalimat — dulu acuan membungkusnya jadi angka negatif tanpa sepatah kata.
    (
        "literal integer di luar 128 bit ditolak (llvm)",
        "package main\npub fn main() {\n  wok(170141183460469231731687303715884105728)\n}",
        "di luar jangkauan integer JRG",
        "di luar jangkauan integer JRG",
    ),
    // Fase BANDING-FN: dua nilai yang bisa dipanggil tidak bisa dibandingkan.
    (
        "banding dua fungsi ditolak (llvm)",
        "package main\nfn f(x: int) -> int { return x }\npub fn main() {\n  let a: fn(int) -> int = f\n  let b: fn(int) -> int = f\n  wok(a == b)\n}",
        "tidak bisa membandingkan",
        "tidak bisa membandingkan",
    ),
    (
        "banding dua fungsi: sebabnya disebut (llvm)",
        "package main\nfn f(x: int) -> int { return x }\npub fn main() {\n  let a: fn(int) -> int = f\n  let b: fn(int) -> int = f\n  wok(a != b)\n}",
        "tidak punya identitas",
        "tidak punya identitas",
    ),
    // Fase KURUNG: sebuah callable tanpa `()`. Backend ini dulu DIAM untuk
    // bentuk pernyataan dan menolak referensi method dengan kalimatnya sendiri.
    // Sekarang satu definisi untuk tiga mesin.
    (
        "fungsi sebagai pernyataan ditolak (llvm)",
        "package main\nfn k() { wok(\"EFEK\") }\npub fn main() {\n  k\n  wok(\"akhir\")\n}",
        "berdiri sendiri sebagai satu pernyataan",
        "berdiri sendiri sebagai satu pernyataan",
    ),
    (
        "referensi method tanpa kurung ditolak (llvm)",
        "package main\nstruct P { pub a: int }\nimpl P { fn dua(self) { return self.a * 2 } }\npub fn main() {\n  let p = P { a: 5 }\n  wok(typeof(p.dua))\n}",
        "itu method milik `P`, dan di sini ditulis tanpa `()`",
        "itu method milik `P`, dan di sini ditulis tanpa `()`",
    ),
    // Fase POLA-FIELD: backend ini sudah menolaknya sejak dulu, dengan kalimat
    // MILIKNYA sendiri — dua kalimat untuk satu kesalahan, di tabel yang tidak
    // bisa melihatnya. Sekarang satu definisi untuk tiga mesin.
    (
        "pola field yang tidak ada dijawab kalimat yang sama (llvm)",
        "package main\nstruct P { pub a: int, pub b: int }\npub fn main() {\n  let p = P { a: 1, b: 2 }\n  match p { P { a, zz } => wok(a), _ => wok(\"lain\") }\n}",
        "pola `P` menyebut field `zz` yang tidak ada",
        "pola `P` menyebut field `zz` yang tidak ada",
    ),
    (
        "salah ketik satu huruf disarankan di tiga mesin (llvm)",
        "package main\nstruct P { pub a: int, pub b: int }\npub fn main() {\n  let p = P { a: 1, b: 2 }\n  match p { P { ab } => wok(1), _ => wok(\"lain\") }\n}",
        "maksudnya `a`?", "maksudnya `a`?",
    ),
    // ── Fase MUDAH-2 sampai ke mesin KETIGA (Fase SAPU) ───────────────────
    (
        "null dijawab padanannya, bukan tebakan ejaan (llvm)",
        "package main\npub fn main() { wok(null) }",
        "di JRG namanya `nil`", "di JRG namanya `nil`",
    ),
    (
        "println dijawab padanannya (llvm)",
        "package main\npub fn main() { println(\"x\") }",
        "mencetak pakai `wok", "mencetak pakai `wok",
    ),
    (
        "console dijawab padanannya (llvm)",
        "package main\npub fn main() { console.log(1) }",
        "itu JavaScript", "itu JavaScript",
    ),
    // `"%d" % 5` — kebiasaan Python.
    (
        "modulo dengan string menyebut `%`, bukan koersi (llvm)",
        "package main\npub fn main() { wok(\"%d\" % 5) }",
        "di JRG `%` cuma sisa bagi", "di JRG `%` cuma sisa bagi",
    ),
    // Nama induk warisan yang tidak ada.
    (
        "induk warisan yang namanya tidak ada",
        "package main\nstruct Hewan { pub nama: string }\n\
         struct Anjing: Hewann { pub ras: string }\n\
         pub fn main() { wok(Anjing { nama: \"Bo\", ras: \"b\" }.ras) }",
        "mewarisi `Hewann`",
        "mewarisi `Hewann`",
    ),
    (
        "path method: penerima tipe lain (llvm)",
        "package main\nstruct P { pub n: int }\nstruct Q { pub n: int }\nimpl P { pub fn tambah(self, k: int) -> int { return self.n + k } }\npub fn main() { wok(P::tambah(Q{n:9}, 4)) }",
        "method milik",
        "method milik",
    ),
    // ── `str::format`: dua hasil salah yang diam, ditutup ─────────────────
    (
        "format: {0} ditolak dua-duanya",
        "package main\npub fn main() { wok(str::format(\"{0}\", 1)) }",
        "placeholder berposisi",
        "placeholder berposisi",
    ),
    (
        "format: argumen berlebih ditolak dua-duanya",
        "package main\npub fn main() { wok(str::format(\"{}\", 1, 2)) }",
        "argumen",
        "argumen",
    ),
    (
        "format: spec tak dikenal ditolak dua-duanya",
        "package main\npub fn main() { wok(str::format(\"{:>8}\", 1)) }",
        "tidak dikenal",
        "tidak dikenal",
    ),
    (
        "format: spec di nilai bukan angka ditolak dua-duanya",
        "package main\npub fn main() { wok(str::format(\"{:.2}\", \"teks\")) }",
        "cuma untuk angka",
        "cuma untuk angka",
    ),

    // `rune` sengaja BUKAN `is_int` di backend ini, supaya aritmetika rune
    // tetap ditolak seperti di acuannya. Kalau ia dijadikan int demi
    // memudahkan cast, `'a' + 1` akan diam-diam sah — dan mencetak karakter.
    (
        "aritmetika pada rune ditolak dua-duanya",
        r#"package main
pub fn main() {
    wok('a' + 1)
}"#,
        "tidak bisa `+`",
        "tidak berlaku untuk tipe `rune`",
    ),
    (
        "baca field private dari luar",
        r#"package main
struct Rahasia { nilai: int = 7 }
pub fn main() {
    var r: Rahasia = Rahasia {}
    wok(r.nilai)
}"#,
        "private",
        "private",
    ),
    // Kesetaraan agregat ditambahkan di fase ini; URUTAN sengaja TIDAK.
    (
        "tulis field private dari luar",
        r#"package main
struct Rahasia { nilai: int = 1 }
pub fn main() {
    var r: Rahasia = Rahasia {}
    r.nilai = 9
    wok(1)
}"#,
        "private",
        "private",
    ),
    // `[...5]` dulu dijawab LLVM dengan "`arr::concat` butuh array, diberi `i64`": nama turunan
    // yang tidak ditulis pemakai, dan nama tipe internal.
    (
        "sebar bukan array",
        "package main\npub fn main() { wok([...5]) }",
        "cuma bisa menyebar array — dapat `int`",
        "cuma bisa menyebar array — dapat `int`",
    ),
    // ── Nama yang bukan nilai, dan konstruktor (Fase KONSTRUKTOR) ─────────
    // `E` dan `PI` telanjang dulu konstanta matematika di backend ini saja, jadi
    // `enum E` yang dipakai sebagai nilai mencetak 2.718281828459045.
    (
        "nama enum sebagai nilai dijelaskan, bukan bilangan Euler (llvm)",
        "package main\nenum E { A(int), C }\npub fn main() { wok(E) }",
        "`E` itu nama enum (tipe)", "`E` itu nama enum (tipe)",
    ),
    (
        "PI telanjang ditolak seperti acuannya (llvm)",
        "package main\npub fn main() { wok(PI) }",
        "`PI` belum dikenal", "`PI` tidak ditemukan",
    ),
    (
        "nama struct sebagai nilai dijelaskan (llvm)",
        "package main\nstruct P { pub x: int }\npub fn main() { wok(P) }",
        "`P` itu nama struct (tipe)", "`P` itu nama struct (tipe)",
    ),
    (
        "varian ber-field sebagai nilai dijelaskan, bukan nil (llvm)",
        "package main\nenum E { V { x: int } }\npub fn main() { wok(E::V) }",
        "`E::V` itu varian ber-field", "`E::V` itu varian ber-field",
    ),
    (
        "aritas konstruktor satu kalimat (llvm)",
        "package main\nenum E { A(int), C }\npub fn main() { wok(E::A(1, 2)) }",
        "konstruktor `E::A` butuh 1 argumen, dapat 2", "konstruktor `E::A` butuh 1 argumen, dapat 2",
    ),
    // Fase JAWA-LLVM: LLVM dulu menerima kedua awalan untuk semua nama `jawa::`/`jawir::`.
    (
        "jawa::free bukan nama acuan (llvm)",
        "package main\npub fn main() { let p = jawa::alloc(1)\n jawa::free(p) }",
        "`jawa::free` tidak ditemukan", "`jawa::free` tidak ditemukan",
    ),
    (
        "task::await bukan nama acuan (llvm)",
        "package main\npub fn main() { let t = Task::spawn(|| 5)\n wok(task::await(t)) }",
        "`task::await` tidak ditemukan", "`task::await` tidak ditemukan",
    ),
    (
        "future::join bukan nama acuan (llvm)",
        "package main\npub fn main() { let f = Future::new(|| 5)\n wok(future::join(f)) }",
        "`future::join` tidak ditemukan", "`future::join` tidak ditemukan",
    ),
    (
        "jawir::alloc bukan nama acuan (llvm)",
        "package main\npub fn main() { let p = jawir::alloc(1) }",
        "`jawir::alloc` tidak ditemukan", "`jawir::alloc` tidak ditemukan",
    ),
    // Fase EJAAN-LLVM: dulu "`len` butuh 1 argumen", "konversi tipe butuh 1 argumen".
    (
        "builtin kelebihan argumen satu kalimat (llvm)",
        "package main\npub fn main() { wok(int(1, 9)) }",
        "`int` kebanyakan argumen: paling banyak 1, diberi 2",
        "`int` kebanyakan argumen: paling banyak 1, diberi 2",
    ),
    (
        "builtin kurang argumen satu kalimat (llvm)",
        "package main\npub fn main() { wok(str::upper()) }",
        "`str::upper` butuh 1 argumen, dapat 0", "`str::upper` butuh 1 argumen, dapat 0",
    ),
];

// ─── Tabel 4: divergensi yang DIKETAHUI dan tidak bisa ditutup
// ─────────────

// Program yang jalan di DUA mesin dengan jawaban berbeda, ditemukan lewat
// sapuan. Yang salah LLVM-nya (acuan sumber kebenaran); ditutup di fase LLVM.
const DIVERGENSI_LLVM: &[(&str, &str, &str, &str)] = &[
    // (`int` 64 bit yang membungkus, #346, ditutup di Fase LEBAR-LLVM.)
    // (Literal bulat yang menyesuaikan diri di tempat pecahan, ditutup di Fase KEPUTUSAN-1:
    // LLVM sekarang menolaknya, entrinya di `BELUM_DIDUKUNG_LLVM`.)
];

/// Ditolak acuan, DITERIMA LLVM dengan jawaban salah — cerminan `VM_LEBIH_PERMISIF`,
/// dari sapuan `DITOLAK_DUA_BACKEND` lewat LLVM. Kolom: nama, program, kunci pesan
/// acuan, keluaran LLVM.
const LLVM_LEBIH_PERMISIF: &[(&str, &str, &str, &str)] = &[
    // Kosong sejak tahap 5. Tabelnya dipertahankan untuk kasus berikutnya.
];

#[test]
fn llvm_lebih_permisif_masih_tercatat_akurat() {
    if lewati_tanpa_clang("llvm_lebih_permisif_masih_tercatat_akurat") { return; }
    let mut kabar = Vec::new();
    for (nama, src, kunci_acuan, keluaran_llvm) in LLVM_LEBIH_PERMISIF {
        let i = jalankan_interp(src);
        let l = jalankan_llvm(src);
        if i.sukses {
            kabar.push(format!("[{}] interpreter sekarang MENERIMA — premisnya hilang.", nama));
        } else if !i.galat.contains(kunci_acuan) {
            kabar.push(format!("[{}] interpreter menolak, tapi bukan karena `{}`:\n  {}",
                nama, kunci_acuan, i.galat.lines().next().unwrap_or("")));
        }
        if !l.sukses {
            kabar.push(format!("[{}] KABAR BAIK: LLVM sekarang menolak. Pindahkan kasusnya \
                ke `DITOLAK_INTERP_DAN_LLVM`.", nama));
        } else if l.keluaran != *keluaran_llvm {
            kabar.push(format!("[{}] LLVM sekarang menjawab {:?}, bukan {:?} — ukur ulang.",
                nama, l.keluaran, keluaran_llvm));
        }
    }
    assert!(kabar.is_empty(), "daftar `LLVM_LEBIH_PERMISIF` sudah tidak akurat:\n\n{}", kabar.join("\n\n"));
}

// ─── Tabel 5: pesan gagal SAAT JALAN harus sama ───────────────────────────

const GAGAL_SAAT_JALAN: &[(&str, &str, &str, &str)] = &[
    // Fase TAMBAH-TEKS (#969): operator aritmetika atas nilai bertag yang golongannya salah — kalimat acuan.
    ("bertag: array + int", "package main\npub fn main() {\n var x = nil\n x = [1]\n wok(\"a\")\n wok(x + 5)\n}",
     "a", "tidak bisa `+` antara `array` dan `int`"),
    ("bertag: string - int", "package main\npub fn main() {\n var x = nil\n x = \"ab\"\n wok(\"a\")\n wok(x - 1)\n}",
     "a", "tidak bisa `-` antara `string` dan `int`"),
    ("bertag: string % int", "package main\npub fn main() {\n var x = nil\n x = \"ab\"\n wok(\"a\")\n wok(x % 5)\n}",
     "a", "di JRG `%` cuma sisa bagi"),
    // Fase AMAN-29 (#908): masukan berbaris baru tidak memalsukan baris pesan, yang panjang dipotong.
    ("pesan: teks berbaris baru di-escape", "package main\npub fn main() {\n wok(\"a\")\n wok(str::to_int(\"12\\nPALSU: x\"))\n}", "a", "teks `12\\u{a}PALSU: x` bukan bilangan bulat"),
    // Fase AMAN-37 (#927): array bertag yang isinya tak sebanding: `sort` menyebut `array`, `<` menyebut daunnya.
    ("sort array bertag tak sebanding", "package main\npub fn main() {\n var e = []\n e = arr::push(e, [1, \"a\"])\n e = arr::push(e, [\"x\", 1])\n wok(\"mulai\")\n wok(arr::sort(e))\n}",
     "mulai", "arr::sort: tidak bisa membandingkan `array` dengan `array`"),
    // Fase POLA-BERTAG (#950): `match` tanpa lengan yang cocok menyebut nilainya (dipotong), seperti acuan.
    ("match tanpa lengan cocok", "package main\npub fn main() {\n let s = \"hai\"\n wok(\"m\")\n match s { \"x\" => wok(1) }\n}",
     "m", "match tidak exhaustive: tidak ada arm yang cocok untuk nilai `hai`"),
    ("match bertag tanpa lengan cocok", "package main\npub fn main() {\n let c = [Some(9), \"q\"]\n wok(\"m\")\n match c[1] { Some(v) => wok(v) }\n}",
     "m", "match tidak exhaustive: tidak ada arm yang cocok untuk nilai `q`"),
    ("match nilai panjang dipotong", "package main\npub fn main() {\n wok(\"m\")\n match str::repeat(\"ab\", 50) { \"x\" => wok(1) }\n}",
     "m", "`abababababababababababababababababababababababababababababab… (100 karakter)`"),
    // Fase POLA-BERTAG (#951): `for` atas nilai bertag yang tak bisa diiterasi — struct berkalimat sendiri.
    ("for atas struct di nilai bertag", "package main\nstruct Titik { x: int }\npub fn main() {\n let a = [Titik { x: 1 }, 2]\n wok(\"m\")\n for x in a[0] { wok(x) }\n}",
     "m", "for-in: `Titik` itu struct, bukan koleksi"),
    ("for atas bool di nilai bertag", "package main\npub fn main() {\n let b = [true, \"s\"]\n wok(\"m\")\n for x in b[0] { wok(x) }\n}",
     "m", "for-in: tidak bisa iterate `bool`"),
    // Fase CELAH-BERTAG (#945–#947): yang bukan wadah, kunci antara hilang dan field tuple di luar batas, atas nilai bertag.
    ("tulis indeks atas bertag berisi int", "package main\npub fn main() {\n var a = []\n a = arr::push(a, 5)\n a = arr::push(a, \"s\")\n var e = a[0]\n wok(\"m\")\n e[0] = 1\n}",
     "m", "`int` tidak bisa ditulis lewat `[...]` — yang bisa cuma array, dict, dan struct"),
    ("kunci antara hilang di jalur tulis bertag", "package main\npub fn main() {\n var d = dict::new()\n d[\"k\"] = [1, \"dua\"]\n d[\"n\"] = 3\n wok(\"m\")\n d[\"q\"][1] = \"x\"\n}",
     "m", "key `q` belum ada — bikin dulu isinya sebelum menulis ke dalamnya"),
    ("field tuple bertag di luar batas", "package main\npub fn main() {\n var a = []\n a = arr::push(a, (\"x\", 1))\n a = arr::push(a, 5)\n let t = a[0]\n wok(\"m\")\n wok(t.5)\n}",
     "m", "index [5] di luar batas — tuple ini isinya 2 elemen"),
    // Fase AMAN-38 (#934): operator bit dan `math::max` atas nilai bertag gagal saat jalan berkalimat acuan.
    ("operator bit atas pecahan bertag", "package main\npub fn main() {\n let a = [2.5, 1]\n wok(\"m\")\n wok(a[0] >> a[1])\n}",
     "m", "tidak bisa `>>` antara `float` dan `int` — operator bit cuma untuk bilangan bulat"),
    ("math::max atas nilai bertag tak sebanding", "package main\npub fn main() {\n let a = [\"x\", 1]\n wok(\"m\")\n wok(math::max(a[0], a[1]))\n}",
     "m", "tidak bisa membandingkan `string` dengan `int`"),
    ("< array bertag tak sebanding", "package main\npub fn main() {\n var a = []\n a = [a, 1]\n var b = []\n b = [b, \"x\"]\n wok(\"m\")\n wok(a < b)\n}",
     "m", "tidak bisa membandingkan `int` dengan `string`"),
    // Fase AMAN-34 (#919): fungsi di dalam array bertag gagal saat dibandingkan, berlokasi seperti acuan.
    ("== atas array bertag berisi fungsi", "package main\nfn f(x) { return x }\npub fn main() {\n var a = []\n a = [a, 1]\n wok(\"mulai\")\n var b = []\n b = [b, f]\n var c = []\n c = [c, f]\n wok(b == c)\n}",
     "mulai", "tidak bisa membandingkan `function` dengan `function`"),
    ("pesan: teks panjang dipotong", "package main\npub fn main() {\n wok(\"a\")\n wok(int(str::repeat(\"7\", 100) + \"x\"))\n}", "a", "…77777777777x (101 karakter)` bukan bilangan bulat"),
    ("pesan: angka JSON raksasa dipotong", "package main\npub fn main() {\n wok(\"a\")\n wok(json::parse(str::repeat(\"9\", 200)))\n}", "a", "(200 karakter)` di luar jangkauan 128 bit"),
    ("let: pola tuple berliteral tidak cocok", "package main\npub fn main() {\n wok(\"a\")\n let (a, 5) = (1, 6)\n wok(a)\n}", "a", "nilai `let` bertipe `tuple` tidak cocok dengan polanya"),
    ("struct: kunci variabel ke field privat dari luar", "package main\nstruct P { pub x: int, rahasia: int = 7 }\npub fn main() { let p = P { x: 1 }\n var k = \"rahasia\"\n wok(\"a\")\n wok(p[k]) }", "a", "field `rahasia` di `P` bersifat private"),
    ("struct: kunci variabel yang tidak ada", "package main\nstruct P { pub x: int }\npub fn main() { let p = P { x: 1 }\n var k = \"zz\"\n wok(\"a\")\n wok(p[k]) }", "a", "field `zz` tidak ditemukan di struct `P` — field yang ada: x"),
    ("tuple: indeks variabel di luar batas", "package main\npub fn main() { let t = (1, 2)\n var i = 5\n wok(\"a\")\n wok(t[i]) }", "a", "index [5] di luar batas — tuple ini isinya 2 elemen"),
    ("tuple: indeks variabel pecahan", "package main\npub fn main() { let t = (1, \"x\")\n var i = 1.5\n wok(\"a\")\n wok(t[i]) }", "a", "tidak bisa index `tuple` dengan `float`"),
    ("potong: nilai bertag berisi bilangan", "package main\npub fn main() { let w = len(env::args()) < 99 $ 5 : \"x\"\n wok(\"a\")\n wok(w[0:1]) }", "a", "tidak bisa slice `int` — yang bisa dipotong"),
    // Fase AMAN-1 (#840): `env::set` di LLVM dulu membuang semua galat, dan NUL di jalur dipotong runtime C.
    ("env::set nama ber-`=`", "package main\npub fn main() { wok(\"a\")\n env::set(\"A=B\", \"1\") }", "a", "env::set: nama variabel \"A=B\" tidak sah — tidak boleh kosong, dan tidak boleh berisi `=` atau karakter NUL"),
    ("env::set nama ber-NUL", "package main\npub fn main() { wok(\"a\")\n env::set(\"A\" + str(rune(0)) + \"B\", \"1\") }", "a", "env::set: nama variabel \"A\\0B\" tidak sah"),
    ("env::set nilai ber-NUL", "package main\npub fn main() { wok(\"a\")\n env::set(\"JRG_UJI_NUL\", \"x\" + str(rune(0))) }", "a", "env::set: nilai untuk `JRG_UJI_NUL` berisi karakter NUL"),
    ("fs::read jalur ber-NUL", "package main\npub fn main() { wok(\"a\")\n wok(fs::read(\"Cargo.toml\" + str(rune(0)) + \"x\")) }", "a", "teksnya memuat karakter NUL, yang tidak bisa diserahkan ke sistem operasi"),
    // Fase BAGI-LLVM: jumlah elemen `range` dulu dihitung di 64 bit tanpa batas — `akhir - awal`
    // meluap dan `sdiv` atas MIN dan -1 menjawab panjang karangan; sekarang batas acuan.
    ("range: jumlah di atas batas elemen", "package main\npub fn main() { wok(\"a\")\n let m = -9223372036854775807 - 1\n var s = -1\n wok(len(range(0, m, s))) }", "a", "range: minta 9223372036854775808 elemen sekaligus, batasnya 16777216"),
    ("range: langkah nol dari variabel", "package main\npub fn main() { wok(\"a\")\n var s = 0\n wok(range(1, 5, s)) }", "a", "range: langkah tidak boleh 0 — urutannya tidak akan pernah sampai ke akhir"),
    // Fase POTONG: dulu ditolak LLVM saat kompilasi dengan kalimat sendiri.
    ("potong: atas bilangan", "package main\npub fn main() { let x = 5\n wok(\"a\")\n wok(x[0:1]) }", "a", "tidak bisa slice `int` — yang bisa dipotong: array, string, tuple"),
    ("potong: atas dict", "package main\npub fn main() { let d = {\"k\": 1}\n wok(\"a\")\n wok(d[0:1]) }", "a", "tidak bisa slice `dict` — yang bisa dipotong"),
    ("potong: batas awal pecahan", "package main\npub fn main() { let a = [1, 2, 3]\n wok(\"a\")\n wok(a[1.5:2]) }", "a", "batas awal potongan harus bilangan bulat, dapat `float` — tulis `x as int`"),
    ("potong: batas akhir pecahan atas teks", "package main\npub fn main() { let s = \"abc\"\n wok(\"a\")\n wok(s[0:2.0]) }", "a", "batas akhir potongan harus bilangan bulat, dapat `float`"),
    ("potong: batas teks sesudah awal di luar jangkauan", "package main\npub fn main() { let a = [1, 2, 3]\n wok(\"a\")\n wok(a[9:\"x\"]) }", "a", "batas awal potongan [9] di luar jangkauan"),
    ("potong: batas bool", "package main\npub fn main() { let a = [1, 2, 3]\n wok(\"a\")\n wok(a[true:2]) }", "a", "batas awal potongan harus bilangan bulat, dapat `bool`"),
    ("potong: tuple di luar jangkauan", "package main\npub fn main() { let t = (1, 2, 3)\n wok(\"a\")\n wok(t[1:9]) }", "a", "batas akhir potongan [9] di luar jangkauan — panjangnya 3"),
    ("potong: tuple terbalik", "package main\npub fn main() { let t = (1, 2, 3)\n wok(\"a\")\n wok(t[2:1]) }", "a", "potongan [2:1] terbalik"),
    ("indeks: tuple konstan di luar batas", "package main\npub fn main() { let t = (1, 2)\n wok(\"a\")\n wok(t[5]) }", "a", "index [5] di luar batas — tuple ini isinya 2 elemen"),
    // Fase LOKASI-LLVM: kalimat `assert`/`assert_eq` acuan — dulu "assertion failed: `left == right`",
    // argumen ketiga `assert_eq` dibuang, dan dua array beda panjang ditolak saat kompilasi.
    ("assert tanpa pesan", "package main\npub fn main() { wok(\"mulai\")\n assert(1 == 2) }",
     "mulai", "assert gagal — kondisinya tidak terpenuhi"),
    ("assert berpesan", "package main\npub fn main() { wok(\"mulai\")\n assert(1 == 2, \"harus sama\") }",
     "mulai", "assert gagal — harus sama"),
    ("assert_eq bilangan", "package main\npub fn main() { wok(\"mulai\")\n assert_eq(1, 2) }",
     "mulai", "assert_eq gagal — kiri `1` (int), kanan `2` (int)"),
    ("assert_eq argumen ketiga", "package main\npub fn main() { wok(\"mulai\")\n assert_eq(1.5, 2.5, \"catatan\") }",
     "mulai", "kanan `2.5` (float) — catatan"),
    ("assert_eq array beda panjang", "package main\npub fn main() { wok(\"mulai\")\n assert_eq([1, 2], [1]) }",
     "mulai", "assert_eq gagal — kiri `[1, 2]` (array), kanan `[1]` (array)"),
    ("assert_eq nilai panjang dipotong", "package main\npub fn main() { wok(\"mulai\")\n assert_eq(str::repeat(\"x\", 80), \"y\") }",
     "mulai", "… (80 karakter)"),
    // Fase ARGUMEN-METODE: jumlah argumen method bawaan yang salah — dulu kurang ditolak saat
    // kompilasi dengan kalimat sendiri, dan yang LEBIH dibuang diam-diam (`s.upper(1)` → `A`).
    ("method string kelebihan argumen, sesudah argumennya", "package main\nfn g() -> int { wok(\"g\"); return 1 }\npub fn main() { wok(\"mulai\")\n let s = \"a\"\n wok(s.upper(g())) }",
     "mulai\ng", "method `upper` butuh 0 argumen, dapat 1"),
    ("method string kurang argumen", "package main\npub fn main() { wok(\"x\")\n wok(\"a-b\".split()) }",
     "x", "method `split` butuh 1 argumen (pemisah), dapat 0"),
    ("method array kelebihan argumen", "package main\npub fn main() { wok(\"x\")\n let a = [1, 2]\n wok(a.len(3)) }",
     "x", "method `len` butuh 0 argumen, dapat 1"),
    ("method dict kelebihan argumen", "package main\npub fn main() { wok(\"x\")\n let d = {\"a\": 1}\n wok(d.keys(1)) }",
     "x", "method `keys` butuh 0 argumen, dapat 1"),
    ("method Option kelebihan argumen", "package main\npub fn main() { wok(\"x\")\n let o = Some(1)\n wok(o.unwrap(1)) }",
     "x", "method `unwrap` butuh 0 argumen, dapat 1"),
    ("defer method kelebihan argumen gagal saat jalan", "package main\nfn g() -> int { wok(\"g\"); return 1 }\npub fn main() {\n let s = \"a\"\n defer s.upper(g())\n wok(\"sesudah\") }",
     "g\nsesudah", "method `upper` butuh 0 argumen, dapat 1"),
    // Fase PTR-LLVM: yang bukan pegangan `jawa::` gagal saat jalan, sesudah semua argumen dievaluasi.
    ("jawa::get atas nilai bertag berisi teks", "package main\nfn pilih(n: int) { if n > 0 { return jawa::alloc(1) } return \"x\" }\npub fn main() { wok(\"sebelum\")\n jawa::get(pilih(0)) }",
     "sebelum", "jawa::get: butuh Ptr (dari jawa::alloc/array/buffer), dapat `string`"),
    ("jawa::set atas teks sesudah argumennya", "package main\nfn f() -> int { wok(\"f jalan\"); 3 }\npub fn main() { wok(\"sebelum\")\n jawa::set(\"x\", f()) }",
     "sebelum\nf jalan", "jawa::set: butuh Ptr (dari jawa::alloc/array/buffer), dapat `string`"),
    ("jawir::free atas teks", "package main\npub fn main() { wok(\"sebelum\")\n jawir(\"x\") }",
     "sebelum", "jawir::free: butuh Ptr (dari jawa::alloc/array/buffer), dapat `string`"),
    ("jawa::get atas Ptr milik ptr::", "package main\npub fn main() { wok(\"sebelum\")\n let p: Ptr = ptr::new(1)\n wok(jawa::get(p)) }",
     "sebelum", "jawa::get: handle tidak valid (field `kind` hilang)"),
    // Fase PEGANGAN-BERTAG: nilai bertag berisi jenis lain di tempat pegangan.
    ("Atomic dari nilai bertag berisi teks", "package main\nfn pilih(b: bool) { if b { return Atomic::new(5) } return \"teks\" }\npub fn main() { let x = pilih(false)\n wok(\"sebelum\")\n wok(atomic::get(x)) }",
     "sebelum", "`atomic::get` butuh sebuah Atomic (buat dengan `Atomic::new`), dapat `string`"),
    ("Mutex dari nilai bertag berisi Atomic", "package main\nfn pilih(b: bool) { if b { return Mutex::new() } return Atomic::new(1) }\npub fn main() { let x = pilih(false)\n wok(\"sebelum\")\n mutex::lock(x) }",
     "sebelum", "`mutex::lock` butuh sebuah Mutex (buat dengan `Mutex::new`), dapat `atomic`"),
    // Fase BULAT-2: pecahan di tempat bulat yang kalimat acuannya bukan `pesan_bukan_bulat`.
    ("for atas range berpecahan", "package main\npub fn main() { wok(\"sebelum\")\n for i in range(0, 2.5) { wok(i) } }",
     "sebelum", "range: argumen ke-2 butuh bilangan bulat, dapat pecahan `2.5`"),
    ("future::timeout dengan batas pecahan", "package main\npub fn main() { let f = Future::new(|| 1)\n wok(\"sebelum\")\n wok(future::timeout(f, 2.5)) }",
     "sebelum", "future::timeout: argumen 2 harus int >= 0 (milidetik), dapat `float`"),
    ("fs::chmod dengan mode pecahan", "package main\npub fn main() { wok(\"sebelum\")\n fs::chmod(\"/tmp/jrg-tidak-ada-chmod\", 4.5) }",
     "sebelum", "argumen tidak cocok untuk `fs::chmod` — dapat (string, float), bentuk yang diterima: fs::chmod(path: string, mode: int)"),
    ("builtin jembatan dengan argumen berjenis salah", "package main\npub fn main() { wok(\"sebelum\")\n wok(crypto::random_int(1.5, 3)) }",
     "sebelum", "argumen tidak cocok untuk `crypto::random_int` — dapat (float, int), bentuk yang diterima: crypto::random_int(min, max)"),
    // Fase ATOM-LLVM: jenis argumen `Atomic`/`Mutex` yang salah dulu ditolak LLVM saat KOMPILASI.
    ("Atomic::new atas pecahan", "package main\npub fn main() { wok(\"sebelum\")\n let a = Atomic::new(1.5) }",
     "sebelum", "`Atomic::new` butuh bilangan bulat, dapat `float` — Atomic cuma menyimpan `int`"),
    ("atomic::add dengan pecahan yang dievaluasi dulu", "package main\nfn bikin() -> float { wok(\"dievaluasi\")\n return 1.5 }\npub fn main() { let a = Atomic::new(1)\n wok(\"sebelum\")\n atomic::add(a, bikin()) }",
     "sebelum\ndievaluasi", "`atomic::add` butuh bilangan bulat, dapat `float` — Atomic cuma menyimpan `int`"),
    ("atomic::add atas nilai bertag berisi teks", "package main\npub fn main() { let xs = [1, \"a\"]\n let a = Atomic::new(1)\n wok(atomic::add(a, xs[0]))\n atomic::add(a, xs[1]) }",
     "2", "`atomic::add` butuh bilangan bulat, dapat `string` — Atomic cuma menyimpan `int`"),
    ("atomic::get atas bukan Atomic", "package main\npub fn main() { wok(\"sebelum\")\n wok(atomic::get(3)) }",
     "sebelum", "`atomic::get` butuh sebuah Atomic (buat dengan `Atomic::new`), dapat `int`"),
    ("mutex::lock atas Atomic", "package main\npub fn main() { let a = Atomic::new(1)\n wok(\"sebelum\")\n mutex::lock(a) }",
     "sebelum", "`mutex::lock` butuh sebuah Mutex (buat dengan `Mutex::new`), dapat `atomic`"),
    ("mutex::with_lock dengan bukan fungsi", "package main\npub fn main() { let m = Mutex::new()\n wok(\"sebelum\")\n mutex::with_lock(m, 5) }",
     "sebelum", "nilai bertipe `int` bukan fungsi, jadi tidak bisa dipanggil"),
    // Fase POLIMORF: tipe isi yang tidak punya method-nya gagal dengan kalimat acuan, field yang ada ikut disebut.
    ("polimorfisme: struct tanpa method itu", "package main\nstruct A { x: int }\nstruct B { y: int }\nimpl A { fn f(self) -> int { return self.x } }\npub fn main() { let xs = [A { x: 1 }, B { y: 2 }]\n wok(\"sebelum\")\n for _, v in xs { wok(v.f()) } }",
     "sebelum\n1", "`struct B` tidak punya method atau field bernama `f` (kalau maksudnya field/key, yang ada: y)"),
    // Fase FFI-LLVM: `?` di `main` berhenti dengan kalimat yang sama (`pesan_main_gagal`), `defer` jalan.
    ("`?` atas Err di main", "package main\npub fn main() { defer wok(\"tunda\")\n wok(\"sebelum\")\n let x = Result::Err([1, \"a\"])?\n wok(x) }",
     "sebelum\ntunda", "program berhenti karena `Result::Err` merambat keluar dari `main`: [1, a]"),
    ("`?` atas None di main", "package main\npub fn main() { wok(\"sebelum\")\n let x = [].first()?\n wok(x) }",
     "sebelum", "program berhenti karena `Option::None` merambat keluar dari `main` — tangani"),
    ("main beranotasi Result yang pulang Err", "package main\nstruct G { pub k: int }\npub fn main() -> Result<(), G> { defer wok(\"tunda\")\n wok(\"sebelum\")\n Err(G { k: 4 }) }",
     "sebelum\ntunda", "program berhenti karena `Result::Err` merambat keluar dari `main`: G {k: 4}"),
    // Fase MAIN-HASIL: `for` atas yang tak bisa diiterasi dulu ditolak saat kompilasi berkalimat sendiri.
    ("for atas channel", "package main\npub fn main() { let c = make_chan(int, 1)\n wok(\"sebelum\")\n for v in c { wok(v) } }",
     "sebelum", "for-in: tidak bisa iterate `channel` — yang bisa: array, string, dict"),
    ("for atas struct", "package main\nstruct P { x: int }\npub fn main() { wok(\"sebelum\")\n let p = P { x: 1 }\n for v in p { wok(v) } }",
     "sebelum", "for-in: `P` itu struct, bukan koleksi"),
    ("main beranotasi Option yang pulang None", "package main\npub fn main() -> Option<int> { wok(\"sebelum\")\n return None }",
     "sebelum", "program berhenti karena `Option::None` merambat keluar dari `main` — tangani"),
    ("spec desimal atas nilai bertag yang bukan angka", "package main\npub fn main() { let d = {\"a\": 1, \"b\": \"t\"}\n wok(\"sebelum\")\n let k = \"b\"\n wok(str::format(\"{:.2}\", d[k])) }",
     "sebelum", "str::format: spec `{:.2}` cuma untuk angka, dapat `string`"),
    ("jawa::buffer_from_hex jumlah digit ganjil", "package main\npub fn main() { wok(\"sebelum\")\n wok(jawa::buffer_from_hex(\"abc\")) }",
     "sebelum", "jawa::buffer_from_hex: hex_decode: jumlah digitnya harus genap, dapat 3"),
    // Fase SEBAB: jalur yang tidak ada dulu menjawab `[]`, sama dengan direktori kosong.
    ("fs::walk atas jalur yang tidak ada", "package main\npub fn main() { wok(\"sebelum\")\n wok(fs::walk(\"/tidak/ada/jrg\")) }",
     "sebelum", "fs::walk '/tidak/ada/jrg': berkas atau direktorinya tidak ada"),
    ("fs::walk atas berkas", "package main\npub fn main() { fs::write(\"/tmp/jrg-walk-berkas.txt\", \"a\")\n wok(\"sebelum\")\n wok(fs::walk(\"/tmp/jrg-walk-berkas.txt\")) }",
     "sebelum", "fs::walk '/tmp/jrg-walk-berkas.txt': bukan direktori"),
    // Fase SINKRON-LLVM: kunci berpemilik, kalimatnya dari `src/sinkron.rs`.
    ("mutex: mengunci dua kali di goroutine yang sama", "package main\npub fn main() { wok(\"sebelum\")\n let m = Mutex::new()\n mutex::lock(m)\n mutex::lock(m) }",
     "sebelum", "mutex::lock: kunci ini sudah kamu pegang di goroutine yang sama"),
    ("mutex: with_lock lalu lock di dalamnya", "package main\npub fn main() { wok(\"sebelum\")\n let m = Mutex::new()\n mutex::with_lock(m, || { mutex::lock(m) }) }",
     "sebelum", "mutex::lock: kunci ini sudah kamu pegang di goroutine yang sama"),
    ("mutex: melepas yang tidak terkunci", "package main\npub fn main() { wok(\"sebelum\")\n let m = Mutex::new()\n mutex::unlock(m) }",
     "sebelum", "mutex::unlock: kunci ini tidak sedang terkunci — tiap `mutex::unlock` butuh satu `mutex::lock` sebelumnya"),
    ("mutex: ditinggal terkunci oleh goroutine yang selesai", "package main\npub fn main() { wok(\"sebelum\")\n let m = Mutex::new()\n let c = make_chan(int, 1)\n spawn { mutex::lock(m)\n c <- 1 }\n <-c\n time::sleep(0.05)\n mutex::lock(m) }",
     "sebelum", "mutex::lock: kunci ini ditinggal terkunci oleh goroutine yang sudah selesai"),
    ("mutex: melepas kunci milik goroutine lain", "package main\npub fn main() { wok(\"sebelum\")\n let m = Mutex::new()\n let c = make_chan(int, 1)\n spawn { mutex::lock(m)\n c <- 1\n time::sleep(0.2)\n mutex::unlock(m) }\n <-c\n mutex::unlock(m) }",
     "sebelum", "mutex::unlock: kunci ini dipegang goroutine lain — cuma yang mengunci yang boleh melepas"),
    // Fase TUNDA-LLVM: `defer` jalan saat galat membawa fungsinya keluar — bingkai terdalam
    // dulu, LIFO — dan keluarannya mendahului kalimat galat. Dulu LLVM langsung keluar.
    (
        "defer: jalan saat panik, bingkai terdalam dulu",
        "package main\nfn catat(s: string) { wok(\"tunda \" + s) }\n\
         fn dalam(a: []int, i: int) -> int { defer catat(\"d1\")\n defer catat(\"d2\")\n return a[i] }\n\
         fn luar() -> int { defer catat(\"luar\")\n let r = dalam([1, 2], 5)\n wok(\"tak sampai\")\n return r }\n\
         pub fn main() { defer catat(\"main\")\n wok(\"mulai\")\n wok(luar()) }",
        "mulai\ntunda d2\ntunda d1\ntunda luar\ntunda main",
        "index [5] di luar batas",
    ),
    (
        "defer: yang ikut gagal saat panik jadi catatan, sisanya tetap jalan",
        "package main\nfn gagal(s: string) { let a = [1]\n wok(\"coba \" + s)\n wok(a[3]) }\n\
         fn catat(s: string) { wok(\"tunda \" + s) }\n\
         fn g() -> int { defer catat(\"x\")\n defer gagal(\"y\")\n let a = [1]\n return a[9] }\n\
         pub fn main() { defer catat(\"main\")\n wok(g()) }",
        "coba y\ntunda x\ntunda main",
        "catatan: sebuah `defer` juga gagal saat membereskan: index [3] di luar batas",
    ),
    (
        "defer: yang gagal di jalur pulang biasa tidak menghentikan sisanya",
        "package main\nfn gagal(s: string) { let a = [1]\n wok(\"coba \" + s)\n wok(a[3]) }\n\
         fn catat(s: string) { wok(\"tunda \" + s) }\n\
         fn f() -> int { defer catat(\"a\")\n defer gagal(\"b\")\n defer catat(\"c\")\n return 7 }\n\
         pub fn main() { defer catat(\"main\")\n wok(f())\n wok(\"tak sampai\") }",
        "tunda c\ncoba b\ntunda a\ntunda main",
        "index [3] di luar batas",
    ),
    (
        "defer: fungsi yang dipanggil dibekukan saat didaftarkan, juga saat panik",
        "package main\nfn a() { wok(\"a\") }\nfn b() { wok(\"b\") }\n\
         pub fn main() { var f = a\n defer f()\n f = b\n let g = |s: string| wok(\"clo \" + s)\n defer g(\"1\")\n panik(\"sengaja\") }",
        "clo 1\na",
        "sengaja",
    ),
    (
        "defer: method, loop, dan assert yang gagal",
        "package main\nstruct B { pub nama: string }\nimpl B { fn tutup(self) { wok(\"tutup \" + self.nama) } }\n\
         fn catat(s: string, n: int) { wok(s + \" \" + str(n)) }\n\
         pub fn main() { let x = B { nama: \"a\" }\n defer x.tutup()\n for i in 0..2 { defer catat(\"putaran\", i) }\n assert(1 == 2, \"salah\") }",
        "putaran 1\nputaran 0\ntutup a",
        "salah",
    ),
    // Fase LAPIS: kalimat gagal builtin yang ditulis dalam JRG sama dengan kalimat acuannya.
    (
        "arr::chunks: ukuran nol",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::chunks([1, 2, 3], 0)) }",
        "sebelum",
        "arr::chunks: ukuran harus lebih dari 0",
    ),
    (
        "arr::windows: ukuran negatif",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::windows([1, 2, 3], 0 - 2)) }",
        "sebelum",
        "arr::windows: ukuran tidak boleh negatif, dapat -2",
    ),
    (
        "arr::insert: di luar batas",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::insert([1, 2, 3], 5, 9)) }",
        "sebelum",
        "arr::insert: index 5 di luar batas (panjang 3)",
    ),
    (
        "arr::remove: di luar batas",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::remove([1, 2, 3], 5)) }",
        "sebelum",
        "arr::remove: index 5 di luar batas (panjang 3)",
    ),
    (
        "arr::repeat: jumlah negatif",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::repeat([1, 2], 0 - 1)) }",
        "sebelum",
        "arr::repeat: jumlah tidak boleh negatif, dapat -1",
    ),
    // Fase AMAN-13 (#881–#882): `str::repeat`/`*` negatif dulu menjawab teks kosong di LLVM, dan
    // batas 256 MiB serta batas elemen `arr::fill`/`arr::repeat` tidak ditegakkan di sana.
    (
        "str::repeat: jumlah negatif",
        "package main\npub fn main() { wok(\"sebelum\")\n let n = 0 - 2\n wok(str::repeat(\"ab\", n)) }",
        "sebelum",
        "tidak bisa mengulang string sebanyak -2 kali — jumlah pengulangan tidak boleh negatif",
    ),
    (
        "str * n: jumlah negatif",
        "package main\npub fn main() { wok(\"sebelum\")\n let n = 0 - 3\n wok(\"ab\" * n) }",
        "sebelum",
        "tidak bisa mengulang string sebanyak -3 kali — jumlah pengulangan tidak boleh negatif",
    ),
    (
        "str::repeat: hasil di atas 256 MiB",
        "package main\npub fn main() { wok(\"sebelum\")\n let n = 200000000\n wok(len(str::repeat(\"aé\", n))) }",
        "sebelum",
        "hasil pengulangan kegedean: 2 karakter x 200000000 kali = sekitar 600000000 byte (batasnya 256 MiB)",
    ),
    (
        "arr::fill: di atas batas elemen",
        "package main\npub fn main() { wok(\"sebelum\")\n let n = 20000000\n wok(len(arr::fill(0, n))) }",
        "sebelum",
        "arr::fill: minta 20000000 elemen sekaligus, batasnya 16777216",
    ),
    (
        "arr::repeat: total di atas batas elemen",
        "package main\npub fn main() { wok(\"sebelum\")\n let n = 10000000\n wok(len(arr::repeat([1, 2], n))) }",
        "sebelum",
        "arr::repeat: minta 20000000 elemen sekaligus, batasnya 16777216",
    ),
    (
        "arr::max_by: array kosong",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::max_by([1][1:], |x| x)) }",
        "sebelum",
        "arr::max_by: array kosong tidak punya nilai terbesar",
    ),
    (
        "arr::mean: array kosong",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::mean([1][1:])) }",
        "sebelum",
        "arr::mean: array kosong tidak punya rata-rata",
    ),
    (
        "arr::max_by: kunci NaN",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::max_by([1.0, math::NAN, 2.0], |x| x)) }",
        "sebelum",
        "arr::max_by: kunci bertipe `float` dan `float` tidak bisa dibandingkan, jadi urutannya tidak terdefinisi",
    ),
    (
        "arr::sort_by_key: kunci NaN",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::sort_by_key([2.0, math::NAN, 1.0], |x| x)) }",
        "sebelum",
        "arr::sort_by_key: kunci bertipe `float` dan `float` tidak bisa dibandingkan, jadi urutannya tidak terdefinisi",
    ),
    (
        "arr::binary_search: mencari NaN",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::binary_search([1.0, 2.0, 3.0], math::NAN)) }",
        "sebelum",
        "binary_search: `2.0` (tipe float) tidak bisa dibandingkan dengan yang dicari `NaN` (tipe float)",
    ),
    (
        "arr::median: memuat NaN",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::median([1.0, math::NAN])) }",
        "sebelum",
        "arr::median: array mengandung NaN, urutannya tidak terdefinisi",
    ),
    (
        "arr::sort_desc: memuat NaN",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(arr::sort_desc([1.0, math::NAN])) }",
        "sebelum",
        "arr::sort_desc: tidak bisa membandingkan `float` dengan `float`, jadi urutannya tidak terdefinisi",
    ),
    // Fase EKOR: pad di lapis ketiga, dan bentuk method chunks/windows berkalimat sama.
    (
        "str::pad_left: isi dua karakter",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(str::pad_left(\"ab\", 5, \"xy\")) }",
        "sebelum",
        "str::pad_left: isinya harus tepat satu karakter, dapat 2 karakter",
    ),
    (
        "str::center: lebar negatif",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(str::center(\"ab\", 0 - 1)) }",
        "sebelum",
        "index tidak boleh negatif: -1",
    ),
    (
        "method chunks(0)",
        "package main\npub fn main() { wok(\"sebelum\")\n wok([1, 2, 3].chunks(0)) }",
        "sebelum",
        "arr::chunks: ukuran harus lebih dari 0",
    ),
    (
        "method windows(0)",
        "package main\npub fn main() { wok(\"sebelum\")\n wok([1, 2, 3].windows(0)) }",
        "sebelum",
        "arr::windows: ukuran harus lebih dari 0",
    ),
    // Fase KECIL: `rand::` dan `str::substr` di lapis ketiga.
    (
        "rand::int: batas terbalik",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(rand::int(5, 3)) }",
        "sebelum",
        "rand::int: hi (3) harus >= lo (5)",
    ),
    (
        "rand::choice: array kosong",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(rand::choice([1][1:])) }",
        "sebelum",
        "rand::choice: array kosong, tidak ada yang bisa dipilih",
    ),
    (
        "str::substr: awal di luar jangkauan",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(str::substr(\"halo\", 5)) }",
        "sebelum",
        "str::substr: posisi awal 5 di luar jangkauan — string ini cuma 4 karakter",
    ),
    (
        "str::substr: panjang negatif",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(str::substr(\"halo\", 1, 0 - 1)) }",
        "sebelum",
        "index tidak boleh negatif: -1",
    ),
    // Fase BERKAS: kalimat gagal `fs::`/`time::` dari lapis ketiga sama dengan acuannya.
    (
        "fs::read: berkas tidak ada",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(fs::read(\"/tidak/ada/jrg.txt\")) }",
        "sebelum",
        "fs::read '/tidak/ada/jrg.txt': berkas atau direktorinya tidak ada",
    ),
    (
        "fs::read: berkas bukan UTF-8",
        "package main\npub fn main() { let f = fs::join(fs::temp_dir(), \"jrg-l3-utf8-a.bin\")\n fs::write_base64(f, \"//4=\")\n wok(\"sebelum\")\n defer fs::delete(f)\n wok(fs::read(f)) }",
        "sebelum",
        "isinya bukan teks UTF-8 yang sah",
    ),
    (
        "fs::read_lines: berkas bukan UTF-8",
        "package main\npub fn main() { let f = fs::join(fs::temp_dir(), \"jrg-l3-utf8-b.bin\")\n fs::write_base64(f, \"QYA=\")\n wok(\"sebelum\")\n defer fs::delete(f)\n wok(fs::read_lines(f)) }",
        "sebelum",
        "isinya bukan teks UTF-8 yang sah",
    ),
    // Fase UTF8-LLVM: kalimat batas potongan sama dengan acuan — awal, akhir, lalu urutan.
    (
        "potongan: batas akhir di luar",
        "package main\npub fn main() { let a = [1, 2, 3]\n wok(\"sebelum\")\n wok(a[1:99]) }",
        "sebelum",
        "batas akhir potongan [99] di luar jangkauan — panjangnya 3, jadi batas yang sah 0..3 (atau -1..-3 kalau dihitung dari belakang)",
    ),
    (
        "potongan: batas awal negatif di luar",
        "package main\npub fn main() { let s = \"halo\"\n wok(\"sebelum\")\n wok(s[-9:99]) }",
        "sebelum",
        "batas awal potongan [-9] di luar jangkauan — panjangnya 4, jadi batas yang sah 0..4 (atau -1..-4 kalau dihitung dari belakang)",
    ),
    (
        "potongan: terbalik sesudah normalisasi",
        "package main\npub fn main() { let a = [1, 2, 3, 4]\n wok(\"sebelum\")\n wok(a[-1:1]) }",
        "sebelum",
        "potongan [3:1] terbalik — batas awal (3) lebih besar dari batas akhir (1); tulis yang kecil dulu, mis. [1:3]",
    ),
    (
        "dict: kunci yang hilang menyebut kunci yang ada",
        "package main\npub fn main() { let d = {\"b\": 1, \"a\": 2}\n wok(\"sebelum\")\n wok(d[\"zz\"]) }",
        "sebelum",
        "key `zz` tidak ada di dict ini — key yang ada: a, b",
    ),
    (
        "dict: kunci hilang di dict lebih dari 12 kunci",
        "package main\npub fn main() { var d = {\"k00\": 0}\n for i in 1..15 { d[«k${i}»] = i }\n wok(\"sebelum\")\n wok(d[\"q\"]) }",
        "sebelum",
        "dan 3 lagi",
    ),
    // Fase KUNCI-LLVM (#824): field dibaca dan ditulis atas nilai yang bukan struct.
    (
        "field: dibaca atas int",
        "package main\npub fn main() { let x = 5\n wok(\"a\")\n wok(x.foo) }",
        "a",
        "`int` tidak punya method atau field bernama `foo` — cek lagi penulisannya ya",
    ),
    (
        "field: method teks tanpa kurung",
        "package main\npub fn main() { let s = \"ab\"\n wok(s.upper) }",
        "",
        "`upper` itu method milik `string`, dan di sini ditulis tanpa `()`",
    ),
    (
        "field: indeks tuple di luar batas",
        "package main\npub fn main() { let t = (1, 2)\n wok(t.5) }",
        "",
        "index [5] di luar batas — tuple ini isinya 2 elemen",
    ),
    (
        "field: dibaca atas Option",
        "package main\npub fn main() { let o = Some(1)\n wok(o.x) }",
        "",
        "`enum:Option` tidak punya method atau field bernama `x` — cek lagi penulisannya ya",
    ),
    (
        "field: ditulis atas float sesudah sisi kanan",
        "package main\npub fn main() { var f = 1.5\n wok(\"a\")\n f.k = wok(\"rhs\") }",
        "a\nrhs",
        "`float` tidak punya field — `.k` cuma bisa ditulis ke struct atau dict",
    ),
    (
        "field: ditulis atas array",
        "package main\npub fn main() { var a = [1, 2]\n a.n = 3 }",
        "",
        "`array` tidak punya field — `.n` cuma bisa ditulis ke struct atau dict",
    ),
    (
        "indeks: teks atas struct tanpa field itu",
        "package main\nstruct P { pub x: int }\npub fn main() { let p = P { x: 1 }\n wok(\"a\")\n wok(p[\"zz\"]) }",
        "a",
        "field `zz` tidak ditemukan di struct `P` — field yang ada: x",
    ),
    // Fase KUNCI-LLVM (#825): indeks bertipe salah dan wadah yang tak bisa di-index, baca dan tulis.
    (
        "indeks: bool atas array",
        "package main\npub fn main() { wok(\"a\")\n wok([1, 2][true]) }",
        "a",
        "tidak bisa index `array` dengan `bool`",
    ),
    (
        "indeks: pecahan atas string",
        "package main\npub fn main() { wok(\"a\")\n wok(\"ab\"[1.0]) }",
        "a",
        "tidak bisa index `string` dengan `float`",
    ),
    (
        "indeks: atas int sesudah indeksnya",
        "package main\npub fn main() { let x = 5\n wok(x[wok(\"i\")]) }",
        "i",
        "tidak bisa index `int` — yang bisa di-index cuma array, string, tuple, dict dan struct",
    ),
    (
        "indeks: atas Option",
        "package main\npub fn main() { let o = Some(1)\n wok(o[0]) }",
        "",
        "tidak bisa index `enum:Option`",
    ),
    (
        "indeks: ditulis atas int sesudah sisi kanan dan indeksnya",
        "package main\npub fn main() { var x = 5\n x[wok(\"i\")] = wok(\"r\") }",
        "r\ni",
        "`int` tidak bisa ditulis lewat `[...]` — yang bisa cuma array, dict, dan struct",
    ),
    (
        "indeks: ditulis atas string",
        "package main\npub fn main() { var s = \"ab\"\n s[0] = \"x\" }",
        "",
        "`string` tidak bisa ditulis lewat `[...]`",
    ),
    (
        "indeks: ditulis ke array dengan teks",
        "package main\npub fn main() { var a = [1, 2]\n wok(\"a\")\n a[\"k\"] = 3 }",
        "a",
        "tidak bisa index `array` dengan `string`",
    ),
    (
        "indeks: ditulis bersarang atas elemen int",
        "package main\npub fn main() { var a = [1]\n a[0][1] = wok(\"r\") }",
        "r",
        "`int` tidak bisa ditulis lewat `[...]`",
    ),
    // Fase KUNCI-LLVM (#823): skalar, tuple, struct, enum, Option dan Result tanpa method itu.
    (
        "method: tidak ada atas int, sesudah argumennya",
        "package main\npub fn main() { let x = 5\n wok(x.uper(wok(\"arg\"))) }",
        "arg",
        "`int` tidak punya method atau field bernama `uper` — cek lagi penulisannya ya",
    ),
    (
        "method: math:: atas int menyebut jalan keluarnya",
        "package main\npub fn main() { let x = 5\n wok(x.sqrt()) }",
        "",
        "`int` tidak punya method atau field bernama `sqrt`",
    ),
    (
        "method: tidak ada atas tuple",
        "package main\npub fn main() { let t = (1, 2)\n wok(t.len()) }",
        "",
        "`tuple` tidak punya method atau field bernama `len` (yang ada untuk tuple cuma indeks numerik: `t.0`, `t.1`, ...)",
    ),
    (
        "method: tidak ada atas struct, saran dari method warisan",
        "package main\nstruct A { nama: string }\nimpl A { fn sapa(self) -> string { self.nama } }\nstruct B: A { umur: int }\npub fn main() { let b = B { nama: \"x\", umur: 2 }\n wok(b.sapa())\n wok(b.spa()) }",
        "x",
        "`struct B` tidak punya method atau field bernama `spa` (kalau maksudnya field/key, yang ada: nama, umur) — maksudnya `sapa`?",
    ),
    (
        "method: tidak ada atas enum",
        "package main\nenum E { A, B }\npub fn main() { let e = E::A\n wok(e.nama()) }",
        "",
        "`enum:E` tidak punya method atau field bernama `nama` — cek lagi penulisannya ya",
    ),
    (
        "method: tidak ada atas Option dan Result",
        "package main\npub fn main() { let r: Result<int, string> = Ok(1)\n wok(r.is_ok())\n let o = Some(3)\n wok(o.unwrp()) }",
        "true",
        "`enum:Option` tidak punya method atau field bernama `unwrp` — maksudnya `unwrap`?",
    ),
    // Fase KUNCI-LLVM (#822): method yang bukan method acuan gagal saat jalan, sesudah argumennya.
    (
        "method: tidak ada atas teks, sesudah argumennya",
        "package main\npub fn main() { let s = \"ab\"\n wok(s.uper(wok(\"arg\"))) }",
        "arg",
        "`string` tidak punya method atau field bernama `uper` — maksudnya `upper`?",
    ),
    (
        "method: tidak ada atas array",
        "package main\npub fn main() { let a = [1, 2]\n wok(a.lenn()) }",
        "",
        "`array` tidak punya method atau field bernama `lenn` — maksudnya `len`?",
    ),
    (
        "method: nama dict:: yang bukan method menyebut kunci yang ada",
        "package main\npub fn main() { let d = {\"new\": 1}\n wok(\"a\")\n wok(d.new(wok(\"arg\"))) }",
        "a\narg",
        "`dict` tidak punya method atau field bernama `new` (kalau maksudnya field/key, yang ada: new) — cek lagi penulisannya ya",
    ),
    (
        "method: str::join bukan method teks",
        "package main\npub fn main() { let s = \"-\"\n wok(s.join([\"a\", \"b\"])) }",
        "",
        "`string` tidak punya method atau field bernama `join` — cek lagi penulisannya ya",
    ),
    (
        "dict: field yang hilang lewat titik",
        "package main\npub fn main() { var d = {\"a\": 1}\n d[\"b\"] = \"t\"\n wok(d.b)\n wok(d.kyes) }",
        "t",
        "`dict` tidak punya method atau field bernama `kyes` (kalau maksudnya field/key, yang ada: a, b) — cek lagi penulisannya ya",
    ),
    (
        "fs::size: berkas tidak ada",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(fs::size(\"/tidak/ada/jrg.txt\")) }",
        "sebelum",
        "fs::size '/tidak/ada/jrg.txt': berkas atau direktorinya tidak ada",
    ),
    (
        "fs::write: folder tidak ada",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(fs::write(\"/tidak/ada/jrg.txt\", \"x\")) }",
        "sebelum",
        "fs::write '/tidak/ada/jrg.txt': berkas atau direktorinya tidak ada",
    ),
    (
        "time::format: arahan tak dikenal",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(time::format(0, \"%q\")) }",
        "sebelum",
        "time::format: arahan `%q` tidak dikenal.",
    ),
    (
        "time::sleep: durasi tak masuk akal",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(time::sleep(99999999999)) }",
        "sebelum",
        "time::sleep: durasi `99999999999` detik tidak masuk akal",
    ),
    // Fase JSON-LLVM: pengurai dan penyandi yang sama (`json_inti`) di runtime LLVM.
    (
        "json::parse: `tidak` bukan `true`",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(json::parse(\"tidak\")) }",
        "sebelum",
        "json::parse: harusnya `true` di posisi 0, yang tertulis `tidak`",
    ),
    (
        "json::parse: objek tak tertutup",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(json::parse(\"{\")) }",
        "sebelum",
        "json::parse: objek tidak ditutup dengan `}`",
    ),
    (
        "json::stringify: NaN",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(json::stringify([1.5, math::NAN])) }",
        "sebelum",
        "`NaN` tidak punya padanan di JSON, jadi ia tidak bisa disandikan",
    ),
    (
        "json::pretty: lebar di luar jangkauan",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(json::pretty([1], 0 - 1)) }",
        "sebelum",
        "json::pretty: lebar indentasi -1 di luar jangkauan 0 sampai 16",
    ),
    // Fase KORUTIN: galat channel dan serah-sekali Future/Task, kalimat yang sama dengan acuan.
    (
        "channel: kirim ke yang sudah ditutup",
        "package main\npub fn main() { wok(\"sebelum\")\n let c = make_chan(int, 1)\n c.close()\n c <- 3 }",
        "sebelum",
        "channel sudah ditutup: tidak bisa kirim lagi",
    ),
    (
        "channel: terima dari yang tertutup dan kosong",
        "package main\npub fn main() { wok(\"sebelum\")\n let c = make_chan(int, 1)\n c.close()\n wok(<-c) }",
        "sebelum",
        "channel sudah ditutup dan kosong: tidak akan ada nilai lagi",
    ),
    (
        "task::join dua kali",
        "package main\nfn kerja() -> int { return 42 }\npub fn main() { let t = Task::spawn(kerja)\n wok(\"sebelum\")\n let a = task::join(t)\n wok(task::join(t) + a) }",
        "sebelum",
        "task::join: hasilnya sudah diambil sebelumnya",
    ),
    (
        "future::try_get sesudah await",
        "package main\nfn kerja() -> int { return 42 }\npub fn main() { let f = Future::new(kerja)\n wok(\"sebelum\")\n let a = future::await(f)\n wok(future::try_get(f), a) }",
        "sebelum",
        "future::try_get: hasilnya sudah diambil sebelumnya",
    ),
    (
        "jawa::array_pop atas larik kosong",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(jawa::array_pop(jawa::array(0))) }",
        "sebelum",
        "jawa::array_pop: array kosong",
    ),
    (
        "jawa::buffer_fill di luar byte",
        "package main\npub fn main() { wok(\"sebelum\")\n jawa::buffer_fill(jawa::buffer(2), 256) }",
        "sebelum",
        "jawa::buffer_fill: nilai 256 di luar jangkauan byte (0..255)",
    ),
    (
        "jawa:: dengan handle jenis lain",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(jawa::buffer_to_hex(jawa::array(1))) }",
        "sebelum",
        "jawa::buffer_to_hex: handle ini hasil `jawa::array(...)`, bukan `jawa::buffer(...)`",
    ),
    // Badan Future/Task yang gagal: galatnya muncul di titik pengambilan hasil, bukan mematikan program.
    (
        "future::await atas tugas yang gagal",
        "package main\nfn bagi(a: int, b: int) -> int {\n defer wok(\"defer bagi\")\n return a / b }\npub fn main() { let g = Future::new(bagi, 6, 3)\n wok(future::await(g))\n let f = Future::new(bagi, 6, 0)\n time::sleep(0.05)\n wok(\"sebelum\")\n wok(future::await(f)) }",
        "defer bagi\n2\ndefer bagi\nsebelum",
        "Future gagal: `6 / 0`",
    ),
    (
        "task::join atas tugas yang gagal",
        "package main\nfn bagi(a: int, b: int) -> int {\n defer wok(\"defer bagi\")\n return a / b }\npub fn main() { let t = Task::spawn(bagi, 6, 0)\n time::sleep(0.05)\n wok(\"sebelum\", future::is_done(t))\n wok(task::join(t)) }",
        "defer bagi\nsebelum true",
        "Task gagal: `6 / 0`",
    ),
    (
        "future::all memuat tugas yang gagal",
        "package main\nfn bagi(a: int, b: int) -> int {\n defer wok(\"defer bagi\")\n return a / b }\npub fn main() { wok(\"sebelum\")\n wok(future::all([Future::new(bagi, 6, 2), Future::new(bagi, 6, 0)])) }",
        "sebelum\ndefer bagi\ndefer bagi",
        "Future gagal: `6 / 0`",
    ),
    (
        "future::timeout atas tugas yang gagal",
        "package main\nfn bagi(a: int, b: int) -> int {\n defer wok(\"defer bagi\")\n return a / b }\npub fn main() { let f = Future::new(bagi, 6, 0)\n time::sleep(0.05)\n wok(\"sebelum\")\n wok(future::timeout(f, 100)) }",
        "defer bagi\nsebelum",
        "Future gagal: `6 / 0`",
    ),
    (
        "future::race atas tugas yang gagal",
        "package main\nfn bagi(a: int, b: int) -> int {\n defer wok(\"defer bagi\")\n return a / b }\npub fn main() { wok(\"sebelum\")\n wok(future::race([Future::new(bagi, 6, 0)])) }",
        "sebelum\ndefer bagi",
        "Future gagal: `6 / 0`",
    ),
    // Fase UMUM: NaN di dalam array membatalkan pembandingan urutannya, seperti di dua mesin lain.
    (
        "urutan array: elemen NaN tidak terbanding",
        "package main\npub fn main() { wok(\"sebelum\")\n wok([math::NAN] < [1.0]) }",
        "sebelum",
        "tidak bisa membandingkan `float` dengan `float`",
    ),
    // `str::char_at` di luar batas dulu DIJAWAB LLVM dengan karakter nol, tanpa pesan.
    (
        "str::char_at: indeks di luar batas",
        "package main\npub fn main() { let s = \"h\u{e9}\u{e9}-abc\"\n let n = len(s) + 3\n wok(str::char_at(s, 2), s.char_at(4))\n wok(str::char_at(s, n)) }",
        "\u{e9} a",
        "str::char_at: index 10 di luar batas (panjang 7)",
    ),
    (
        "str::char_at: indeks negatif dari belakang, lalu di luar batas",
        "package main\npub fn main() { let s = \"abc\"\n let n = 0 - len(s) - 1\n wok(str::char_at(s, n + 1))\n wok(str::char_at(s, n)) }",
        "a",
        "str::char_at: index -4 di luar batas (panjang 3)",
    ),
    // Fase GENERIK: `arr::swap` di LLVM, dengan kalimat acuan untuk dua kegagalannya.
    (
        "arr::swap: indeks di luar batas",
        "package main\npub fn main() { let a = [1, 2, 3]\n let n = len(a) * 3\n wok(\"sebelum\")\n wok(arr::swap(a, 0, n)) }",
        "sebelum",
        "arr::swap: index 9 di luar batas (panjang 3)",
    ),
    (
        "arr::swap: indeks negatif dari belakang, lalu di luar batas",
        "package main\npub fn main() { let a = [1, 2, 3]\n let n = 0 - len(a) - 1\n wok(arr::swap(a, n + 1, -1))\n wok(arr::swap(a, n, 0)) }",
        "[3, 2, 1]",
        "arr::swap: index -4 di luar batas (panjang 3)",
    ),
    // Fase SISA: LLVM dulu menjawab `NaN` (max) dan `1.0` (min) tanpa pesan.
    (
        "arr::max atas pecahan yang memuat NaN",
        "package main\npub fn main() { let n = math::NAN\n wok(arr::max([n]))\n wok(arr::max([1.0, n])) }",
        "NaN",
        "arr::max: `float` dan `float` tidak bisa dibandingkan, jadi tidak ada yang bisa disebut paling kecil atau paling besar",
    ),
    (
        "arr::min atas pecahan yang memuat NaN",
        "package main\npub fn main() { let n = math::NAN\n wok(arr::min([2.0, 1.0]))\n wok(arr::min([n, 1.0, 2.5])) }",
        "1.0",
        "arr::min: `float` dan `float` tidak bisa dibandingkan, jadi tidak ada yang bisa disebut paling kecil atau paling besar",
    ),
    // Fase PETA: LLVM dulu MENERIMA dan menjawab `[1.0, 2.5, NaN]`.
    (
        "arr::sort atas pecahan yang memuat NaN",
        "package main\npub fn main() { let n = 0.0 / 0.0\n wok(\"sebelum\")\n wok(arr::sort([2.5, n, 1.0])) }",
        "sebelum",
        "arr::sort: tidak bisa membandingkan `float` dengan `float`, jadi urutannya tidak terdefinisi",
    ),
    // Fase PETA: campuran golongan di array bertag ditolak dengan kalimat acuan.
    (
        "arr::sort atas elemen bertag yang tidak saling terbanding",
        "package main\nstruct K { pub isi: array }\npub fn main() { wok(\"sebelum\")\n wok(arr::sort(K { isi: [1, \"a\"] }.isi)) }",
        "sebelum",
        "arr::sort: tidak bisa membandingkan `string` dengan `int`, jadi urutannya tidak terdefinisi",
    ),
    // Fase SEPARUH: jumlah negatif ditolak saat jalan, dengan kalimat acuan.
    (
        "arr::take: jumlah negatif, sesudah keluaran sebelumnya",
        "package main\npub fn main() { let a = [1, 2]\n wok(arr::take(a, 1))\n let n = 0 - len(a)\n wok(arr::take(a, n)) }",
        "[1]",
        "arr::take: jumlah tidak boleh negatif, dapat -2",
    ),
    // Fase SIMPUL-1: `panik` dulu "fungsi tidak ditemukan" di LLVM.
    (
        "panik: menghentikan program dengan pesannya, sesudah keluaran sebelumnya",
        "package main\npub fn main() { wok(\"sebelum\")\n panik(\"rusak: \" + str(3)) }",
        "sebelum",
        "rusak: 3",
    ),
    // Fase JAGA-STACK: dulu SIGSEGV dan core dump tanpa sepatah kata. Dua cabang, supaya
    // pengoptimal tidak bisa mengubah rekursinya jadi perulangan.
    (
        "rekursi tak berujung: stack habis dilaporkan, bukan SIGSEGV",
        "package main\nfn f(n: int) -> int { return f(n + 1) + f(n + 2) }\npub fn main() { wok(\"mulai\")\n wok(f(0)) }",
        "mulai",
        "rekursi terlalu dalam",
    ),
    // Fase SEBAR-LLVM: jumlah argumen spread diperiksa saat jalan, dengan kalimat acuan.
    (
        "spread kekurangan argumen ke fungsi bernama",
        "package main\nfn tambah(a: int, b: int) -> int { return a + b }\npub fn main() { var ys: []int = []\n ys = ys + [1]\n wok(\"sebelum\")\n wok(tambah(...ys)) }",
        "sebelum",
        "fungsi `tambah`: parameter `b` (posisi 2) wajib diisi",
    ),
    (
        "spread kebanyakan argumen ke fungsi bernama",
        "package main\nfn tambah(a: int, b: int) -> int { return a + b }\npub fn main() { var ys: []int = []\n for i in 0..3 { ys = ys + [i] }\n wok(\"sebelum\")\n wok(tambah(...ys)) }",
        "sebelum",
        "fungsi `tambah` kebanyakan argumen — dia cuma menerima 2, tapi dikasih 3",
    ),
    // Fase BULAT-LLVM: pecahan di tempat bulat builtin dulu ditolak LLVM saat kompilasi dengan kalimatnya sendiri.
    (
        "pecahan di indeks method char_at gagal saat jalan",
        "package main\npub fn main() { wok(\"sebelum\")\n wok(\"ab\".char_at(1.0)) }",
        "sebelum",
        "str::char_at: butuh bilangan bulat, dapat pecahan `1.0` — kalau memang mau dipotong, tulis eksplisit: `1.0 as int`",
    ),
    (
        "pecahan dari fungsi di argumen substr: argumennya dievaluasi dulu",
        "package main\nfn bikin() -> float { wok(\"dievaluasi\")\n return 2.0 }\npub fn main() { wok(\"sebelum\")\n wok(str::substr(\"abc\", bikin(), 1)) }",
        "sebelum\ndievaluasi",
        "str::substr: butuh bilangan bulat, dapat pecahan `2.0`",
    ),
    (
        "pecahan di range menyebut nomor argumennya",
        "package main\npub fn main() { let x = 2.5\n wok(\"sebelum\")\n wok(range(1, x)) }",
        "sebelum",
        "range: argumen ke-2 butuh bilangan bulat, dapat pecahan `2.5`",
    ),
    (
        "pecahan di indeks jawa::buffer_get bukan soal isi buffer",
        "package main\npub fn main() { let b = jawa::buffer(3)\n wok(\"sebelum\")\n wok(jawa::buffer_get(b, 1.5)) }",
        "sebelum",
        "jawa::buffer_get: butuh bilangan bulat, dapat pecahan `1.5`",
    ),
    // Fase TUMPUK: stack yang habis dulu membuang `defer` dan menggugurkan Future di LLVM.
    (
        "stack habis: defer tetap jalan, terdalam dulu",
        "package main\nfn f(n: int) -> int { return f(n + 1) + f(n + 2) }\nfn beres(x: int) { wok(x) }\npub fn main() { defer beres(1)\n defer beres(2)\n wok(\"mulai\")\n wok(f(0)) }",
        "mulai\n2\n1",
        "rekursi terlalu dalam",
    ),
    (
        "stack habis di badan Future: galatnya sampai ke yang menunggu",
        "package main\nfn f(n: int) -> int { return f(n + 1) + f(n + 2) }\npub fn main() { wok(\"mulai\")\n let t = Future::new(|| f(0))\n wok(future::await(t)) }",
        "mulai",
        "Future gagal: rekursi terlalu dalam",
    ),
    // Fase TERM: di luar 0–255 cuma 8 bit terbawahnya yang sampai — dulu 256 jadi 0.
    (
        "exit: kode di luar 0-255 gagal saat jalan, bukan dipotong",
        r#"package main
fn kode() -> int { return 256 }
pub fn main() {
    wok("sebelum")
    exit(kode())
}"#,
        "sebelum",
        "kode keluar harus 0 sampai 255, dapat 256",
    ),
    // `jawa::` mensimulasikan malloc/free, jadi use-after-free dan double-free
    // adalah KEGAGALAN yang harus dilaporkan — bukan perilaku tak terdefinisi.
    (
        "jawa: use-after-free",
        r#"package main
pub fn main() {
    let p = jawa::alloc(1)
    wok("sebelum")
    jawir::free(p)
    wok(jawa::get(p))
}"#,
        "sebelum",
        "use-after-free — pointer ini sudah di-`jawir::free`",
    ),
    (
        "jawa: double free",
        r#"package main
pub fn main() {
    let p = jawa::alloc(1)
    wok("sebelum")
    jawir::free(p)
    jawir::free(p)
}"#,
        "sebelum",
        "double free — pointer ini sudah pernah di-`jawir::free` sebelumnya",
    ),

    (
        "index array di luar batas",
        r#"package main
pub fn main() {
    wok("sebelum")
    var a: [4]int = [1, 2, 3, 4]
    wok(a[9])
}"#,
        "sebelum",
        "index [9] di luar batas — array ini isinya 4 elemen (index valid: 0..3, atau -1..-4",
    ),
    (
        "index array negatif di luar batas",
        r#"package main
pub fn main() {
    wok("sebelum")
    var a: [4]int = [1, 2, 3, 4]
    wok(a[-9])
}"#,
        "sebelum",
        "index [-9] di luar batas — array ini isinya 4 elemen",
    ),
    // Satuan yang disebut untuk string harus `karakter`, bukan `elemen` —
    // yang dihitung memang karakter, bukan byte, dan pembaca yang bingung
    // soal `s[9]` justru perlu tahu itu.
    (
        "index string di luar batas menyebut karakter",
        r#"package main
pub fn main() {
    wok("sebelum")
    var s: string = "halo"
    wok(s[9])
}"#,
        "sebelum",
        "index [9] di luar batas — string ini isinya 4 karakter",
    ),
    (
        "bagi nol",
        r#"package main
pub fn main() {
    wok("sebelum")
    var n: int = 0
    wok(1 / n)
}"#,
        "sebelum",
        "`1 / 0` — nggak bisa bagi dengan nol",
    ),
    (
        "sisa bagi nol punya kalimatnya sendiri",
        r#"package main
pub fn main() {
    wok("sebelum")
    var n: int = 0
    wok(-7 % n)
}"#,
        "sebelum",
        "`-7 % 0` — nggak bisa cari sisa bagi dengan nol",
    ),
    // Fase TAMAT (#833): dua mesin acuan dulu menunggu goroutine yang terblokir.
    (
        "main gagal tidak menunggu goroutine yang terblokir",
        "package main\npub fn main() {\n    let c = make_chan(int)\n    spawn { c <- 1 }\n    wok(\"sebelum\")\n    let a = [1]\n    wok(a[5])\n}",
        "sebelum",
        "index [5] di luar batas",
    ),
    // Fase HOF-LLVM (#832): dulu ditolak saat kompilasi dengan kalimat sendiri.
    (
        "hof: yang diserahkan bukan fungsi",
        "package main\npub fn main() {\n    wok(\"sebelum\")\n    wok([1, 2].reduce(0, 1))\n}",
        "sebelum",
        "nilai bertipe `int` bukan fungsi, jadi tidak bisa dipanggil",
    ),
    (
        "hof: closure kurang parameter",
        "package main\npub fn main() {\n    wok(\"sebelum\")\n    wok([1].map(|| 5))\n}",
        "sebelum",
        "fungsi `<closure>` kebanyakan argumen — dia cuma menerima 0, tapi dikasih 1.",
    ),
    (
        "hof: fungsi bernama kelebihan parameter",
        "package main\nfn dua(a: int, b: int) -> int { a + b }\npub fn main() {\n    wok(\"sebelum\")\n    wok([1].map(dua))\n}",
        "sebelum",
        "fungsi `dua`: parameter `b` (posisi 2) wajib diisi",
    ),
    (
        "geser: jumlah negatif",
        r#"package main
pub fn main() {
    wok("sebelum")
    var n: int = -1
    wok(8 >> n)
}"#,
        "sebelum",
        "`8 >> -1` — jumlah geseran tidak boleh negatif",
    ),
    (
        "geser: 128 kali atau lebih",
        r#"package main
pub fn main() {
    wok("sebelum")
    var n: int = 200
    wok(8 << n)
}"#,
        "sebelum",
        "`8 << 200` — integer JRG cuma 128 bit",
    ),
];

// ─── Tabel 6: berkas konformans yang LULUS lewat LLVM ─────────────────────

const KONFORMANS_LLVM: &[&str] = &[
    "closures",
    // Dua yang terakhir masuk, dan keduanya butuh NILAI BERTAG — satu-satunya
    // hal yang benar-benar menghalangi sesudah delapan celah lain ditutup.
    "collections",
    "control_flow",
    "functions",
    "indexing",
    "memori_manual",
    "pattern_matching",
    "semantik_cow",
    "semantik_nilai",
    "strings",
    "values_and_operators",
];

// ─── Tes ───────────────────────────────────────────────────────────────────

/// Pesan lewat kalau `clang` tidak ada — dicetak, tidak disembunyikan.
fn lewati_tanpa_clang(nama: &str) -> bool {
    if ada_clang() { return false; }
    eprintln!(
        "LEWAT: {} — `clang` tidak ada di PATH, jadi backend LLVM tidak bisa \
         menghasilkan native code. Set JRGC_CLANG kalau namanya lain.",
        nama
    );
    true
}

/// Divergensi yang diketahui harus MASIH ada.
#[test]
fn divergensi_llvm_yang_diketahui_masih_ada() {
    if lewati_tanpa_clang("divergensi_llvm_yang_diketahui_masih_ada") { return; }

    let mut kabar = Vec::new();
    for (nama, src, keluaran_acuan, keluaran_llvm) in DIVERGENSI_LLVM {
        let i = jalankan_interp(src);
        let l = jalankan_llvm(src);
        if !i.sukses || !l.sukses {
            kabar.push(format!(
                "[{}] salah satu backend sekarang GAGAL, bukan sekadar berbeda — \
                 kasusnya berubah sifat, periksa lagi.",
                nama
            ));
            continue;
        }
        if i.keluaran != *keluaran_acuan {
            kabar.push(format!(
                "[{}] interpreter sekarang menjawab {:?}, bukan {:?} — semantik \
                 acuannya berubah, jadi catatan di tabel ini sudah usang.",
                nama, i.keluaran, keluaran_acuan
            ));
        }
        if l.keluaran == i.keluaran {
            kabar.push(format!(
                "[{}] KABAR BAIK: dua-duanya menjawab {:?} sekarang. Pindahkan \
                 kasusnya ke `PARITAS_LLVM` dan hapus dari sini.",
                nama, i.keluaran
            ));
        } else if l.keluaran != *keluaran_llvm {
            kabar.push(format!(
                "[{}] LLVM sekarang menjawab {:?}, bukan {:?} — masih berbeda, \
                 tapi berbeda dengan cara lain. Ukur ulang sebelum mempercayai \
                 catatannya.",
                nama, l.keluaran, keluaran_llvm
            ));
        }
    }
    assert!(kabar.is_empty(), "daftar `DIVERGENSI_LLVM` sudah tidak akurat:\n\n{}", kabar.join("\n\n"));

}

/// Program yang gagal SAAT JALAN: keluaran sebelum gagal sama, dan pesan
/// gagalnya menyebut hal yang sama.
#[test]
fn pesan_gagal_saat_jalan_sama() {
    if lewati_tanpa_clang("pesan_gagal_saat_jalan_sama") { return; }

    let mut gagal = Vec::new();
    let mut berlokasi = 0;
    let mut tanpa: Vec<&str> = Vec::new();
    for (nama, src, keluaran, memuat) in GAGAL_SAAT_JALAN {
        let i = jalankan_interp(src);
        let l = jalankan_llvm(src);

        for (label, h) in [("interpreter", &i), ("llvm", &l)] {
            if h.sukses {
                gagal.push(format!("[{}] {} TIDAK gagal — kasusnya sudah usang?", nama, label));
                continue;
            }
            if h.keluaran != *keluaran {
                gagal.push(format!(
                    "[{}] {} mencetak {:?} sebelum gagal, diharap {:?} — waktu gagalnya beda",
                    nama, label, h.keluaran, keluaran
                ));
            }
            if !h.galat.contains(memuat) {
                gagal.push(format!(
                    "[{}] pesan {} tidak memuat `{}`\n  dapat: {}",
                    nama, label, memuat,
                    h.galat.lines().take(2).collect::<Vec<_>>().join(" | ")
                ));
            }
        }
        // Lokasi LLVM boleh tidak ada (lahir di runtime), tapi yang ada harus milik acuan (Fase LOKASI-LLVM).
        let lokasi = |g: &str| g.lines().find(|b| b.trim_start().starts_with("--> "))
            .map(|b| b.rsplit('/').next().unwrap_or(b).to_string());
        if lokasi(&l.galat).is_none() && lokasi(&i.galat).is_some() { tanpa.push(*nama); }
        if let Some(ll) = lokasi(&l.galat) {
            berlokasi += 1;
            if lokasi(&i.galat).as_deref() != Some(ll.as_str()) {
                gagal.push(format!("[{}] lokasi LLVM `{}`, acuan {:?}", nama, ll, lokasi(&i.galat)));
            }
        }
    }
    eprintln!("{} dari {} kasus berlokasi di LLVM; berlokasi di acuan saja: {:?}", berlokasi, GAGAL_SAAT_JALAN.len(), tanpa);
    assert!(gagal.is_empty(), "{} masalah pada pesan gagal saat jalan:\n\n{}", gagal.len(), gagal.join("\n\n"));
}

// ─── Pintu konversi: satu aturan di tiga mesin ────────────────────────────

/// (pintu, nilai) yang GAGAL di acuan — tiap kelas kegagalan dari tiap keluarga
/// pintu. Dulu LLVM menjawab `1` untuk `NaN as int`, `44` untuk `300 as u8`,
/// dan baris kosong untuk `(-1) as rune`.
const KONVERSI_GAGAL: &[(&str, &str)] = &[
    ("x as int", "0.0 / 0.0"), ("x as int", "1.0 / 0.0"), ("x as i8", "-1.0 / 0.0"),
    ("x as u8", "1e40"), ("x as int", "-1e40"), ("x as u8", "-5.9"), ("x as i8", "300.5"),
    ("x as u64", "18446744073709551616.0"), ("x as i64", "9223372036854775808.0"),
    ("x as u8", "300"), ("x as byte", "-1"), ("x as i32", "5000000000"), ("x as uint", "-1"),
    ("x as i16", "-32769"), ("x as u8", "'Ā'"), ("x as rune", "-1"), ("x as rune", "55296"),
    ("x as rune", "1114112"), ("x as rune", "5.9"), ("x as rune", "\"a\""),
    ("x as rune", "true"), ("x as int", "nil"), ("x as float", "nil"), ("x as float", "'a'"),
    ("x as int", "[1, 2]"), ("x as int", "\"abc\""), ("x as int", "\"\""),
    ("x as int", "\"1.5\""), ("x as u8", "\"300\""), ("x as u16", "\" -3 \""),
    ("x as int", "\"999999999999999999999999999999999999999999\""),
    ("x as float", "\"abc\""), ("x as float", "\"  \""),
    ("int(x)", "0.0 / 0.0"), ("uint(x)", "-5"), ("rune(x)", "-1"), ("float(x)", "\"x\""),
    ("parse_int(x)", "5.9"), ("str::to_int(x)", "true"), ("str::to_int(x)", "\"1.5\""),
    ("str::to_int(x)", "\"\""), ("str::to_int(x)", "'A'"), ("str::to_float(x)", "\"abc\""),
    ("parse_float(x)", "'A'"), ("x.to_int()", "\"x\""), ("x.to_float()", "\"\""),
];

/// Program yang BERHASIL di tiap pintu, termasuk nilai bertag — sisi positifnya,
/// karena penolakan atas segalanya juga membuat daftar di atas hijau.
const KONVERSI_BERHASIL: &str = r#"package main
pub fn main() {
    let f = 5.9
    let n = -5.9
    wok(f as int, n as int, f as u8, n as i8, int(f), uint(f))
    wok(300 as i32, 65 as u8, 65 as byte, 70000 as rune, rune(300), -128 as i8, 255 as u8)
    wok("5" as int, " 42 " as u8, "  -7" as i8, "1.5" as float, float("2"), float(3), float(true))
    wok(true as u8, false as byte, true as i8, true as float, int(true), uint(true))
    wok('A' as u8, 'A' as int, 'A' as i16, 9223372036854775807 as i64)
    wok(-9223372036854775808.0 as i64, -0.9 as u8, -0.0 as u64, 255.9 as u8)
    wok(parse_int("12"), str::to_int(" -3 "), parse_float("1.5"), str::to_float(2))
    wok("12".to_int(), " 1.5 ".to_float(), str::to_int(7))
    let u: u64 = 9223372036854775807
    wok(u as int)
    let b: i128 = 100000000000000000000
    wok(b as i128, b as u128, "99999999999999999999" as i128)
    var x = 1
    x = 2.5
    wok(x as int, int(x))
    x = "7"
    wok(x as int, x as float, str::to_int(x))
    wok((1.1 as f32) == 1.1, 16777217 as f32)
}"#;

/// Baris galat pertama tanpa awalan `error: ` — LLVM mencetak kalimatnya saja.
fn baris_galat(galat: &str) -> String {
    let b = galat.lines().next().unwrap_or("").trim();
    b.strip_prefix("error: ").unwrap_or(b).to_string()
}

/// Tiap pintu konversi menjawab SAMA dengan acuan di backend LLVM, dan yang
/// gagal gagal dengan kalimat yang sama, sesudah keluaran yang sama.
#[test]
fn pintu_konversi_sama_di_llvm() {
    if lewati_tanpa_clang("pintu_konversi_sama_di_llvm") { return; }

    let mut gagal = Vec::new();
    for (pintu, nilai) in KONVERSI_GAGAL {
        let src = format!(
            "package main\npub fn main() {{\n    var x = {}\n    wok(\"mulai\")\n    wok({})\n}}\n",
            nilai, pintu);
        let i = jalankan_interp(&src);
        let l = jalankan_llvm(&src);
        let nama = format!("{} dengan x = {}", pintu, nilai);
        if i.sukses {
            gagal.push(format!("[{}] acuan TIDAK gagal — kasusnya usang?", nama));
            continue;
        }
        if l.sukses {
            gagal.push(format!("[{}] LLVM menjawab {:?}, acuan menolak: {}",
                nama, l.keluaran, baris_galat(&i.galat)));
            continue;
        }
        if i.keluaran != "mulai" || l.keluaran != "mulai" {
            gagal.push(format!("[{}] waktu gagalnya beda: acuan {:?}, llvm {:?}",
                nama, i.keluaran, l.keluaran));
        }
        let (bi, bl) = (baris_galat(&i.galat), baris_galat(&l.galat));
        if bi != bl {
            gagal.push(format!("[{}] kalimatnya beda\n  acuan: {}\n  llvm : {}", nama, bi, bl));
        }
    }

    let i = jalankan_interp(KONVERSI_BERHASIL);
    let l = jalankan_llvm(KONVERSI_BERHASIL);
    if !i.sukses || !l.sukses || i.keluaran != l.keluaran {
        gagal.push(format!(
            "[yang berhasil] keluarannya beda\n  acuan: {:?} {}\n  llvm : {:?} {}",
            i.keluaran, baris_galat(&i.galat), l.keluaran, baris_galat(&l.galat)));
    }
    assert!(gagal.is_empty(), "{} masalah di pintu konversi LLVM:\n\n{}",
        gagal.len(), gagal.join("\n\n"));
}

/// **`exit` di LLVM: kode 0–255 sampai ke pemanggil apa adanya, dan di luar itu gagal saat
/// jalan dengan kalimat acuannya.** Dulu `exit(256)` keluar 0 — sukses — di sana.
#[test]
fn kode_keluar_sama_di_llvm() {
    if lewati_tanpa_clang("kode_keluar_sama_di_llvm") { return; }
    let mut salah = Vec::new();
    for kode in [0, 7, 255] {
        let src = format!(
            "package main\npub fn main() {{\n    wok(\"mulai\")\n    exit({})\n    wok(\"lewat\")\n}}\n", kode);
        let (i, l) = (jalankan_interp(&src), jalankan_llvm(&src));
        if i.kode != Some(kode) || l.kode != Some(kode) || i.keluaran != "mulai" || l.keluaran != "mulai" {
            salah.push(format!("[exit({})] acuan {:?} {:?}, llvm {:?} {:?} {}",
                kode, i.kode, i.keluaran, l.kode, l.keluaran, baris_galat(&l.galat)));
        }
    }
    for nilai in ["256", "0 - 1", "70000", "4294967296"] {
        let src = format!(
            "package main\nfn kode() -> int {{ return {} }}\npub fn main() {{\n    wok(\"mulai\")\n    exit(kode())\n}}\n",
            nilai);
        let (i, l) = (jalankan_interp(&src), jalankan_llvm(&src));
        let (bi, bl) = (baris_galat(&i.galat), baris_galat(&l.galat));
        if i.sukses || l.sukses || i.keluaran != "mulai" || l.keluaran != "mulai" || bi != bl
            || !bi.contains("kode keluar harus 0 sampai 255") {
            salah.push(format!("[exit({})]\n  acuan: {:?} {}\n  llvm : {:?} {}",
                nilai, i.keluaran, bi, l.keluaran, bl));
        }
    }
    assert!(salah.is_empty(), "{} masalah kode keluar di LLVM:\n\n{}", salah.len(), salah.join("\n\n"));
}

/// `int`/`uint` di backend ini 64 bit, di acuan 128 bit. Angka di antaranya
/// DITOLAK dengan sebabnya — dulu dipotong diam-diam.
#[test]
fn int_64_bit_menolak_bukan_memotong() {
    if lewati_tanpa_clang("int_64_bit_menolak_bukan_memotong") { return; }

    let kasus: &[(&str, &str, &str)] = &[
        ("x as int", "1e20", "tulis `i128`"),
        ("int(x)", "\"99999999999999999999\"", "tulis `i128`"),
        ("x as uint", "1e20", "tulis `u128`"),
        ("parse_int(x)", "\"99999999999999999999\"", "str::to_int: "),
    ];
    let mut gagal = Vec::new();
    for (pintu, nilai, memuat) in kasus {
        let src = format!(
            "package main\npub fn main() {{\n    var x = {}\n    wok(\"mulai\")\n    wok({})\n}}\n",
            nilai, pintu);
        let i = jalankan_interp(&src);
        let l = jalankan_llvm(&src);
        let nama = format!("{} dengan x = {}", pintu, nilai);
        if !i.sukses { gagal.push(format!("[{}] acuan gagal: {}", nama, baris_galat(&i.galat))); }
        if l.sukses {
            gagal.push(format!("[{}] LLVM menjawab {:?} — angka di luar 64 bit dijawab, \
                                bukan ditolak", nama, l.keluaran));
            continue;
        }
        let bl = baris_galat(&l.galat);
        if l.keluaran != "mulai" || !bl.contains("backend LLVM") || !bl.contains(memuat) {
            gagal.push(format!("[{}] penolakannya tidak menyebut sebabnya: {:?} | {}",
                nama, l.keluaran, bl));
        }
    }
    assert!(gagal.is_empty(), "{}", gagal.join("\n\n"));
}

/// Berkas konformans yang lulus lewat LLVM harus TETAP lulus.
#[test]
fn konformans_yang_lulus_lewat_llvm_tetap_lulus() {
    if lewati_tanpa_clang("konformans_yang_lulus_lewat_llvm_tetap_lulus") { return; }

    let akar = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance");
    let mut gagal = Vec::new();
    let mut kabar_baik = Vec::new();

    let mut semua: Vec<String> = std::fs::read_dir(&akar)
        .expect("baca tests/conformance")
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("jrg") {
                p.file_stem().and_then(|x| x.to_str()).map(|s| s.to_string())
            } else { None }
        })
        .collect();
    semua.sort();

    for nama in semua {
        let src = std::fs::read_to_string(akar.join(format!("{}.jrg", nama)))
            .expect("baca berkas konformans");
        let i = jalankan_interp(&src);
        let l = jalankan_llvm(&src);
        let lulus = l.sukses && i.sukses && i.keluaran == l.keluaran;

        if KONFORMANS_LLVM.contains(&nama.as_str()) {
            if !lulus {
                gagal.push(format!(
                    "[{}] TERDAFTAR lulus tapi sekarang tidak.\n  interp: {}\n  llvm  : {}",
                    nama,
                    i.keluaran.lines().last().unwrap_or("(kosong)"),
                    if l.sukses { l.keluaran.lines().last().unwrap_or("(kosong)").to_string() }
                    else { l.galat.lines().next().unwrap_or("").to_string() }
                ));
            }
        } else if lulus {
            kabar_baik.push(nama.clone());
        }
    }

    assert!(
        gagal.is_empty(),
        "{} berkas konformans mundur:\n\n{}",
        gagal.len(), gagal.join("\n\n")
    );
    assert!(
        kabar_baik.is_empty(),
        "KABAR BAIK: {} berkas konformans sekarang lulus lewat LLVM dan belum \
         terdaftar — tambahkan ke `KONFORMANS_LLVM` supaya tidak bisa mundur \
         diam-diam: {}",
        kabar_baik.len(), kabar_baik.join(", ")
    );
}

#[test]
fn paritas_interpreter_dan_llvm() {
    if lewati_tanpa_clang("paritas_interpreter_dan_llvm") { return; }

    let mut gagal = Vec::new();
    for (nama, src) in PARITAS_LLVM {
        let i = jalankan_interp(src);
        let l = jalankan_llvm(src);

        if !i.sukses {
            gagal.push(format!(
                "[{}] program uji GAGAL di interpreter — kasusnya salah tulis, \
                 bukan divergensi backend.\n  stderr: {}",
                nama, i.galat.lines().take(3).collect::<Vec<_>>().join(" | ")
            ));
            continue;
        }
        if !l.sukses {
            gagal.push(format!(
                "[{}] gagal di --emit-llvm sementara interpreter berhasil.\n  \
                 interpreter: {:?}\n  stderr llvm: {}",
                nama, i.keluaran,
                l.galat.lines().take(4).collect::<Vec<_>>().join(" | ")
            ));
            continue;
        }
        if i.keluaran != l.keluaran {
            gagal.push(format!(
                "[{}] keluaran BERBEDA — ini divergensi semantik, bukan gap fitur\n  \
                 interpreter: {:?}\n  llvm       : {:?}",
                nama, i.keluaran, l.keluaran
            ));
        }
    }

    assert!(
        gagal.is_empty(),
        "{} dari {} kasus paritas LLVM tidak cocok:\n\n{}\n\n\
         Interpreter adalah acuan semantik: kalau LLVM berbeda, LLVM yang salah.",
        gagal.len(), PARITAS_LLVM.len(), gagal.join("\n\n")
    );
}

/// Tiap `pub fn ns_nama` di `lapis.jrg` harus melayani builtin `ns::nama` yang memang ada.
#[test]
fn tiap_fungsi_lapis_bernama_builtin_yang_ada() {
    let daftar = Command::new(jrgc()).arg("--daftar-builtin").output().expect("jrgc jalan");
    let daftar = String::from_utf8_lossy(&daftar.stdout).to_string();
    let ada: Vec<&str> = daftar.lines().collect();
    let mut salah = Vec::new();
    let mut jumlah = 0;
    for baris in include_str!("../src/llvm/lapis.jrg").lines() {
        let Some(sisa) = baris.strip_prefix("pub fn ") else { continue };
        let nama: String = sisa.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        let Some((ns, item)) = nama.split_once('_') else { salah.push(nama); continue };
        jumlah += 1;
        if !ada.contains(&format!("{}::{}", ns, item).as_str()) { salah.push(nama); }
    }
    assert!(jumlah >= 40, "lapis.jrg cuma memuat {} fungsi publik — berkasnya terbaca?", jumlah);
    assert!(salah.is_empty(), "fungsi lapis tanpa builtin bernama sama: {:?}", salah);
}

/// Runtime LLVM memakai SALINAN `json_inti.rs`; dua berkas itu harus sama bita demi bita.
#[test]
fn salinan_json_inti_di_runtime_identik() {
    assert!(include_str!("../src/json_inti.rs") == include_str!("../runtime/src/json_inti.rs"),
        "runtime/src/json_inti.rs berbeda dari src/json_inti.rs — salin ulang: cp src/json_inti.rs runtime/src/");
    assert!(include_str!("../src/kutip_inti.rs") == include_str!("../runtime/src/kutip_inti.rs"),
        "runtime/src/kutip_inti.rs berbeda dari src/kutip_inti.rs — salin ulang: cp src/kutip_inti.rs runtime/src/");
}

#[test]
fn gap_llvm_ditolak_dengan_alasan_sebenarnya() {
    let mut gagal = Vec::new();
    for (nama, src, alasan) in BELUM_DIDUKUNG_LLVM {
        // Tidak perlu `clang`: yang diuji penolakan di tahap codegen, sebelum
        // ada IR yang diserahkan ke clang.
        let l = jalankan_llvm(src);
        if l.sukses {
            gagal.push(format!(
                "[{}] KABAR BAIK: sekarang berhasil di LLVM. Pindahkan kasusnya \
                 ke `PARITAS_LLVM` supaya keluarannya ikut dibandingkan.",
                nama
            ));
            continue;
        }
        if !l.galat.contains(alasan) {
            gagal.push(format!(
                "[{}] ditolak, tapi pesannya tidak menyebut `{}` — penolakan yang \
                 tidak menjelaskan sebabnya sama membingungkannya dengan salah \
                 jawab.\n  stderr: {}",
                nama, alasan,
                l.galat.lines().take(4).collect::<Vec<_>>().join(" | ")
            ));
        }
    }
    assert!(
        gagal.is_empty(),
        "{} dari {} gap LLVM tidak terlaporkan sebagaimana mestinya:\n\n{}",
        gagal.len(), BELUM_DIDUKUNG_LLVM.len(), gagal.join("\n\n")
    );
}

/// Program yang HARUS ditolak dua-duanya, tetap ditolak dua-duanya.
#[test]
fn penolakan_bersama_interpreter_dan_llvm() {
    let mut gagal = Vec::new();
    for (nama, src, alasan_i, alasan_l) in DITOLAK_INTERP_DAN_LLVM {
        let i = jalankan_interp(src);
        let l = jalankan_llvm(src);

        if i.sukses {
            gagal.push(format!(
                "[{}] interpreter MENERIMANYA — premis kasusnya hilang; acuan \
                 semantiknya yang berubah, jadi periksa dulu apakah itu memang \
                 disengaja.",
                nama
            ));
        } else if !i.galat.contains(alasan_i) {
            gagal.push(format!(
                "[{}] interpreter menolak tapi bukan karena `{}`\n  stderr: {}",
                nama, alasan_i, i.galat.lines().take(3).collect::<Vec<_>>().join(" | ")
            ));
        }

        if l.sukses {
            gagal.push(format!(
                "[{}] LLVM MENERIMANYA sementara interpreter menolak — divergensi \
                 semantik, bukan gap fitur.",
                nama
            ));
        } else if !l.galat.contains(alasan_l) {
            gagal.push(format!(
                "[{}] LLVM menolak tapi pesannya tidak menyebut `{}` — penolakan \
                 yang tidak menjelaskan sebabnya sama membingungkannya dengan \
                 salah jawab.\n  stderr: {}",
                nama, alasan_l, l.galat.lines().take(3).collect::<Vec<_>>().join(" | ")
            ));
        }
    }
    assert!(
        gagal.is_empty(),
        "{} masalah pada penolakan bersama:\n\n{}",
        gagal.len(), gagal.join("\n\n")
    );
}

// ─── Jenis tidak berganti diam-diam (Fase JENIS-LLVM) ─────────────────────

/// Pasangan (jenis tempat, jenis nilai) yang dulu dijawab LLVM dengan jenis LAIN tanpa pesan:
/// rune jadi angka, angka jadi `true`, `nil` jadi `false`, bulat jadi pecahan.
const PASANGAN_JENIS: &[(&str, &str, &str, &str)] = &[
    ("int", "7", "rune", "'r'"), ("rune", "'r'", "int", "7"),
    ("bool", "true", "int", "7"), ("bool", "true", "float", "2.5"),
    ("bool", "true", "string", "\"s\""), ("bool", "true", "rune", "'r'"),
    ("bool", "true", "nil", "nil"), ("float", "2.5", "int", "7"),
];

fn posisi_jenis(ta: &str, a: &str, b: &str) -> Vec<(&'static str, String)> {
    vec![
        ("literal array", format!("pub fn main() {{ let x = [{a}, {b}]\n wok(x, typeof(x[0]), typeof(x[1])) }}")),
        ("ekor if", format!("pub fn main() {{ let c = false\n let x = if c {{ {a} }} else {{ {b} }}\n wok(x, typeof(x)) }}")),
        ("ternary", format!("pub fn main() {{ let c = false\n let x = c $ {a} : {b}\n wok(x, typeof(x)) }}")),
        ("lengan match", format!("pub fn main() {{ let n = 2\n let x = match n {{ 1 => {a}, _ => {b} }}\n wok(x, typeof(x)) }}")),
        ("arr::push", format!("pub fn main() {{ let x = arr::push([{a}], {b})\n wok(x, typeof(x[1])) }}")),
        ("tulis elemen", format!("pub fn main() {{ var x = [{a}]\n x[0] = {b}\n wok(x, typeof(x[0])) }}")),
        ("closure", format!("pub fn main() {{ let f = |c: bool| if c {{ {a} }} else {{ {b} }}\n let x = f(false)\n wok(x, typeof(x)) }}")),
        ("get_or", format!("pub fn main() {{ let x = [{a}].get_or(5, {b})\n wok(x, typeof(x)) }}")),
        ("let beranotasi", format!("pub fn main() {{ let x: {ta} = {b}\n wok(x, typeof(x)) }}")),
        ("parameter beranotasi", format!("fn f(x: {ta}) {{ wok(x, typeof(x)) }}\npub fn main() {{ f({b}) }}")),
        ("return beranotasi", format!("fn f() -> {ta} {{ return {b} }}\npub fn main() {{ let x = f()\n wok(x, typeof(x)) }}")),
        ("field beranotasi", format!("struct S {{ pub x: {ta} }}\npub fn main() {{ let s = S {{ x: {b} }}\n wok(s.x, typeof(s.x)) }}")),
    ]
}

/// Nilai yang jenisnya lain dari tempatnya: LLVM boleh menolak dengan lantang atau menjawab
/// persis seperti acuan — tidak pernah menjawab dengan jenis lain. Dulu 136 dari 870 begitu.
#[test]
fn jenis_tidak_berganti_diam_diam_di_llvm() {
    if lewati_tanpa_clang("jenis_tidak_berganti_diam_diam_di_llvm") { return; }
    let mut masalah = Vec::new();
    for (ta, a, tb, b) in PASANGAN_JENIS {
        for (posisi, badan) in posisi_jenis(ta, a, b) {
            let src = format!("package main\n{}", badan);
            let i = jalankan_interp(&src);
            let l = jalankan_llvm(&src);
            if !l.sukses {
                if i.sukses && !l.galat.contains("masuk ke tempat yang bertipe") && !l.galat.contains("tidak bisa") {
                    masalah.push(format!("[{posisi}: {tb} ke {ta}] LLVM menolak tanpa menyebut jenisnya:\n  {}",
                        l.galat.lines().next().unwrap_or("")));
                }
                continue;
            }
            if !i.sukses {
                masalah.push(format!("[{posisi}: {tb} ke {ta}] acuan menolak, LLVM menjawab {:?}", l.keluaran));
            } else if i.keluaran != l.keluaran {
                masalah.push(format!("[{posisi}: {tb} ke {ta}] jenis berganti diam-diam\n  acuan: {:?}\n  llvm:  {:?}",
                    i.keluaran, l.keluaran));
            }
        }
    }
    assert!(masalah.is_empty(), "{} masalah jenis di LLVM:\n\n{}", masalah.len(), masalah.join("\n\n"));
}


// ─── Cabang dan tempat yang jenisnya berbeda (Fase CAMPUR) ────────────────

/// Nilai berjenis berbeda yang SAH di acuan: dulu ditolak LLVM di `if`, ternary, `match`,
/// `return`, `arr::push`, dan `var` yang berganti golongan. Sekarang nilainya bertag.
const JENIS_CAMPUR: &[&str] = &["1", "2.5", "\"a\"", "true", "nil", "[1, 2]", "({\"k\": 1})", "[\"x\"]"];

fn posisi_campur(k: usize, a: &str, b: &str) -> String {
    let pakai = |x: &str| format!("wok({x}, typeof({x}), str({x}) + \"!\")");
    match k {
        0 => format!("fn kasus() {{ for _, c in [true, false] {{ let x = if c {{ {a} }} else {{ {b} }}\n {} }} }}", pakai("x")),
        1 => format!("fn kasus() {{ for _, c in [true, false] {{ let x = c $ {a} : {b}\n {} }} }}", pakai("x")),
        2 => format!("fn kasus() {{ for _, n in [0, 1] {{ let x = match n {{ 0 => {a}, _ => {b} }}\n {} }} }}", pakai("x")),
        3 => format!("fn pilih(c) {{ if c {{ return {a} }}\n return {b} }}\n\
                      fn kasus() {{ for _, c in [true, false] {{ let x = pilih(c)\n {} }} }}", pakai("x")),
        4 => format!("fn ekor(c) {{ if c {{ {a} }} else {{ {b} }} }}\n\
                      fn kasus() {{ for _, c in [true, false] {{ let x = ekor(c)\n {} }} }}", pakai("x")),
        5 => format!("fn ekor(n) {{ match n {{ 0 => {a}, _ => {b} }} }}\n\
                      fn kasus() {{ for _, n in [0, 1] {{ let x = ekor(n)\n {} }} }}", pakai("x")),
        6 => format!("fn kasus() {{ var xs = [{a}]\n xs = arr::push(xs, {b})\n {}\n {} }}", pakai("xs"), pakai("xs[1]")),
        _ => format!("fn kasus() {{ var x = {a}\n {}\n x = {b}\n {} }}", pakai("x"), pakai("x")),
    }
}

/// Tiap posisi satu program berisi semua pasangan jenis, jadi delapan kompilasi, bukan 224.
#[test]
fn campur_jenis_dijawab_seperti_acuan() {
    if lewati_tanpa_clang("campur_jenis_dijawab_seperti_acuan") { return; }
    let nama = ["if", "ternary", "match", "return", "ekor if", "ekor match", "arr::push", "var"];
    let mut masalah = Vec::new();
    for (k, posisi) in nama.iter().enumerate() {
        let mut src = String::from("package main\n");
        let mut main = String::from("pub fn main() {\n");
        let mut n = 0;
        for (i, a) in JENIS_CAMPUR.iter().enumerate() {
            for b in &JENIS_CAMPUR[i + 1..] {
                let badan = posisi_campur(k, a, b);
                // Fungsi tiap pasangan diberi nomor supaya tidak bertabrakan.
                src += &badan.replace("kasus", &format!("kasus{n}")).replace("pilih", &format!("pilih{n}"))
                    .replace("ekor(", &format!("ekor{n}("));
                src += "\n";
                main += &format!("    wok(\"-- kasus {n}\")\n    kasus{n}()\n");
                n += 1;
            }
        }
        src += &main;
        src += "}\n";
        let i = jalankan_interp(&src);
        let l = jalankan_llvm(&src);
        if !i.sukses {
            masalah.push(format!("[{posisi}] acuan menolak programnya sendiri:\n{}", i.galat));
        } else if !l.sukses {
            masalah.push(format!("[{posisi}] LLVM menolak:\n{}", l.galat.lines().take(8).collect::<Vec<_>>().join("\n")));
        } else if i.keluaran != l.keluaran {
            let beda = i.keluaran.lines().zip(l.keluaran.lines()).find(|(p, q)| p != q);
            masalah.push(format!("[{posisi}] keluaran beda, pertama: {:?}", beda));
        }
    }
    assert!(masalah.is_empty(), "{} posisi campur jenis tidak sama dengan acuan:\n\n{}",
        masalah.len(), masalah.join("\n\n"));
}

// ─── FFI lewat jembatan (Fase FFI-LLVM) ────────────────────────────────────

fn ada_perintah(p: &str) -> bool {
    std::process::Command::new(p).arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Handle FFI yang tidak keluar dari bloknya dilepas di biner LLVM (Fase FFI-LEPAS) — worker Python tidak lagi
/// menumpuk objeknya — dan handle yang disimpan, dikembalikan, ditangkap atau dialiaskan TIDAK dilepas.
#[test]
fn handle_ffi_yang_tidak_keluar_dilepas_di_llvm() {
    if lewati_tanpa_clang("handle_ffi_yang_tidak_keluar_dilepas_di_llvm") || !ada_perintah("python3") { return; }
    for (src, harus) in [(r#"package main
fn satu() -> int {
    let h = py::ref("[1, 2]").unwrap()
    wok_("")
    return 1
}
pub fn main() {
    var n = 0
    for i in 0..300 { n = n + satu() }
    wok(py::eval("len(__import__(\"__main__\")._objs)"), n)
}
"#, Some("Result::Ok(0) 300")), (r#"package main
pub fn main() {
    var xs = []
    for i in 0..500 {
        let h = py::ref("[1, 2]").unwrap()
        xs = arr::push(xs, h)
    }
    wok(py::eval("len(__import__(\"__main__\")._objs)"), len(xs))
}
"#, Some("Result::Ok(500) 500")),
                         (r#"package main
fn buat() {
    let h = py::ref("[1, 2]").unwrap()
    return h
}
fn panjang(x) { return py::ref_method(x, "__len__").unwrap() }
pub fn main() {
    var xs = []
    for i in 0..3 {
        let h = py::ref("[" + str(i) + ", 9]").unwrap()
        xs = arr::push(xs, h)
    }
    wok(xs, panjang(xs[2]))
    let r = buat()
    wok(r, panjang(r))
    for i in 0..2 {
        let h = py::ref("{'a': " + str(i) + "}").unwrap()
        let f = || py::ref_method(h, "keys").unwrap()
        wok(f())
    }
    for i in 0..2 {
        let h = py::ref("[1, 2, 3]").unwrap()
        let g = h
        wok(g, panjang(g))
    }
    for i in 0..2 {
        let h = py::ref("'teks'").unwrap()
        if i == 1 { wok(py::ref_method(h, "upper").unwrap()) }
        wok(str(h), typeof(h), py::ref_method(h, "count", "t").unwrap())
    }
    var simpan = []
    for i in 0..2 {
        let h = py::ref("[" + str(i) + "]").unwrap()
        simpan = simpan + [h]
    }
    wok(simpan)
}
"#, None)] {
        let (i, l) = (jalankan_interp(src), jalankan_llvm(src));
        assert!(i.sukses && l.sukses, "gagal:\n{}\n{}", i.galat, l.galat);
        assert_eq!(i.keluaran, l.keluaran, "LLVM menjawab lain:\n{}", src);
        if let Some(h) = harus { assert!(l.keluaran.ends_with(h), "{:?}, harusnya diakhiri {:?}", l.keluaran, h); }
    }
}

/// `py::`/`js::`/`php::`/`lua::` di LLVM lewat pintu yang sama dengan dua mesin lain, termasuk
/// method atas handle dan `Err` dari bahasa lainnya; callback ke fungsi JRG ditolak dengan alasannya.
#[test]
fn ffi_di_llvm_seperti_acuan() {
    if lewati_tanpa_clang("ffi_di_llvm_seperti_acuan") { return; }
    let mut src = String::from("package main\npub fn main() {\n");
    if ada_perintah("python3") {
        src += "    wok(py::import(\"math\"), py::call(\"math\", \"sqrt\", 16.0), py::eval(\"nama_tak_ada\"))\n";
        src += "    match py::ref(\"[3, 1, 2]\") {\n        Ok(x) => wok(typeof(x), x.count(1), x.index(99)),\n        Err(e) => wok(e),\n    }\n";
    }
    if ada_perintah("node") {
        src += "    wok(js::eval(\"[1, 2, 3].map(x => x * 2)\"))\n";
        src += "    match js::ref(\"new Map([['a', 1]])\") {\n        Ok(m) => wok(m.get(\"a\"), m.has(\"b\")),\n        Err(e) => wok(e),\n    }\n";
    }
    if ada_perintah("php") { src += "    wok(php::call(\"strtoupper\", \"halo\"))\n"; }
    if ada_perintah("lua") { src += "    wok(lua::eval(\"return 2 ^ 10\"))\n"; }
    src += "}\n";
    let i = jalankan_interp(&src);
    let l = jalankan_llvm(&src);
    assert!(i.sukses, "acuan gagal:\n{}", i.galat);
    assert!(l.sukses, "LLVM menolak FFI:\n{}", l.galat);
    assert_eq!(i.keluaran, l.keluaran, "FFI di LLVM menjawab lain");

    // Callback ke fungsi JRG (Fase FUNGSI-JEMBATAN): sama dengan acuan.
    let mut cb = String::from("package main\npub fn main() {\n");
    if ada_perintah("python3") {
        cb += "    wok(py::call(\"builtins\", \"list\", py::call(\"builtins\", \"map\", |x: int| x * 10, [1, 2, 3])?))\n";
    }
    if ada_perintah("php") { cb += "    wok(php::call(\"array_map\", |x| x * x, [1, 2, 3]))\n"; }
    if ada_perintah("lua") { cb += "    wok(lua::call(\"string.rep\", \"ab\", 3))\n"; }
    cb += "}\n";
    let (i, l) = (jalankan_interp(&cb), jalankan_llvm(&cb));
    assert!(i.sukses && l.sukses, "callback FFI gagal:\nacuan: {}\nllvm: {}", i.galat, l.galat);
    assert_eq!(i.keluaran, l.keluaran, "callback FFI di LLVM menjawab lain");

    // Yang tetap ditolak, dengan alasannya: hitungan handle (tidak pernah dilepas di LLVM).
    for (src, alasan) in [
        ("package main\npub fn main() { wok(py::refs()) }\n", "tidak pernah dilepas"),
    ] {
        let l = jalankan_llvm(src);
        assert!(!l.sukses && l.galat.contains(alasan), "harus ditolak menyebut `{}`, dapat:\n{}", alasan, l.galat);
    }
}

/// Fase SERVE-LLVM: handler `dok::serve` dari biner `--emit-llvm` menjawab seperti acuan — termasuk
/// var modul yang ditulis handler, `req.params`, handler yang GAGAL (500, server tetap hidup), dan 404.
#[test]
fn dok_serve_di_llvm_menjawab_seperti_acuan() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::{Duration, Instant};
    if !ada_clang() { return; }
    struct Server(std::process::Child);
    impl Drop for Server { fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); } }
    let program = |port: u16| format!(r#"package main
var HIT = 0
fn salam(n) {{ return "hai " + str(n) }}
pub fn main() {{
  let r = dok::router_new()
  dok::router_get(r, "/a", |req| {{ HIT = HIT + 1; dok::text(salam(HIT)) }})
  dok::router_get(r, "/u/:id", |req: DokRequest| dok::text("id=" + req.params["id"]))
  dok::router_post(r, "/gema", |req| dok::text(str(len(req.body))))
  dok::router_get(r, "/x", |req| [1][5])
  dok::serve(r, {})
}}
"#, port);
    let minta = |port: u16, baris: &str| -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).expect("server sudah mati");
        s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        write!(s, "{}", baris).unwrap();
        let mut buf = String::new();
        let _ = s.read_to_string(&mut buf);
        let badan = buf.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        format!("{} {}", buf.split(' ').nth(1).unwrap_or("?"), badan)
    };
    let mut hasil = Vec::new();
    for llvm in [false, true] {
        let dir = dir_uji("serve");
        let mut jawab = None;
        for _ in 0..5 {
            let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
            let mut cmd = if llvm {
                let bin = bangun_llvm(&dir, "s", &program(port)).expect("program server tidak terkompilasi di LLVM");
                Command::new(bin)
            } else {
                let f = dir.join("s.jrg");
                std::fs::write(&f, program(port)).unwrap();
                let mut c = Command::new(jrgc());
                c.arg(f).env("JRG_SOPAN", "1");
                c
            };
            cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
            let mut anak = Server(cmd.spawn().unwrap());
            let batas = Instant::now() + Duration::from_secs(20);
            let mut siap = false;
            while Instant::now() < batas && anak.0.try_wait().ok().flatten().is_none() {
                if TcpStream::connect(("127.0.0.1", port)).is_ok() { siap = true; break; }
                std::thread::sleep(Duration::from_millis(50));
            }
            if !siap { continue; }
            let get = |p: &str| format!("GET {} HTTP/1.1\r\nHost: x\r\n\r\n", p);
            jawab = Some(vec![
                minta(port, &get("/a")), minta(port, &get("/a")), minta(port, &get("/u/77")),
                minta(port, &get("/x")).split(':').next().unwrap_or("").to_string(),
                minta(port, &get("/hilang")),
                minta(port, "POST /gema HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc"),
            ]);
            drop(anak);
            break;
        }
        let _ = std::fs::remove_dir_all(&dir);
        hasil.push(jawab.expect("server tidak pernah siap di lima port"));
    }
    assert_eq!(hasil[0], ["200 hai 1", "200 hai 2", "200 id=77", "500 handler error",
                          "404 404 Not Found: GET /hilang", "200 3"], "jawaban acuan");
    assert_eq!(hasil[0], hasil[1], "--emit-llvm menjawab beda dari acuan");
}
