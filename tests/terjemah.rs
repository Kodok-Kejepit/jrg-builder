// tests/terjemah.rs — bahasa pesan (Fase DWIBAHASA): Inggris bawaan, `--bahasa`/`JRG_BAHASA`/`jrgm language`,
// dua mesin sama dalam bahasa Inggris, dan keluaran PROGRAM tidak pernah ikut diterjemahkan.

mod bersama;
use std::path::{Path, PathBuf};
use std::process::Command;

include!("bersama/gagal_saat_jalan.rs");

fn jrgc() -> &'static str { env!("CARGO_BIN_EXE_jrgc") }
fn jrgm() -> &'static str { env!("CARGO_BIN_EXE_jrgm") }

/// Rumah kosong: tanpa berkas setelan, tanpa `JRG_BAHASA` (yang dipasang `.cargo/config.toml` untuk tes lain).
fn perintah(bin: &str, rumah: &Path) -> Command {
    let mut c = Command::new(bin);
    c.env_remove("JRG_BAHASA").env_remove("JRG_SOPAN").env_remove("JRG_NADA").env("HOME", rumah)
        .env("XDG_CONFIG_HOME", rumah.join(".config")).env_remove("APPDATA").current_dir(rumah);
    c
}

fn jalan(c: &mut Command) -> String {
    let o = c.output().expect("jalankan");
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn rumah_baru(isi: &str) -> PathBuf {
    let d = bersama::dir_unik("jrg-terjemah");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("p.jrg"), isi).unwrap();
    d
}

const SALAH: &str = "package main\npub fn main() {\n    wok(\"teks program tidak ada yang diubah\")\n    let a = [1, 2]\n    wok(a[5])\n}\n";

#[test]
fn bahasa_inggris_bawaan_dan_bisa_dipilih() {
    let r = rumah_baru(SALAH);
    let en = jalan(perintah(jrgc(), &r).arg("p.jrg").arg("--sopan"));
    assert!(en.contains("is out of bounds") && en.contains("--> p.jrg:5:"), "bawaan bukan Inggris:\n{en}");
    // Keluaran program milik pemakai, dalam bahasa apa pun: tidak pernah diterjemahkan.
    assert!(en.starts_with("teks program tidak ada yang diubah\n"), "keluaran program ikut berubah:\n{en}");

    let id = jalan(perintah(jrgc(), &r).args(["p.jrg", "--sopan", "--bahasa", "id"]));
    assert!(id.contains("di luar batas"), "--bahasa id tidak berlaku:\n{id}");
    let env = jalan(perintah(jrgc(), &r).args(["p.jrg", "--sopan"]).env("JRG_BAHASA", "id"));
    assert_eq!(env, id, "JRG_BAHASA=id beda dengan --bahasa id");

    // `jrgm language id` berlaku untuk semua jalan berikutnya; `--bahasa` tetap menang.
    let tulis = jalan(perintah(jrgm(), &r).args(["language", "id"]));
    assert!(r.join(".config/jrg/bahasa").exists(), "berkas setelan tidak ditulis: {tulis}");
    assert_eq!(jalan(perintah(jrgc(), &r).args(["p.jrg", "--sopan"])), id, "setelan `jrgm language id` diabaikan");
    assert_eq!(jalan(perintah(jrgc(), &r).args(["p.jrg", "--sopan", "--bahasa", "en"])), en);
    assert!(jalan(perintah(jrgm(), &r).arg("language")).contains("id"));
    jalan(perintah(jrgm(), &r).args(["language", "en"]));
    assert_eq!(jalan(perintah(jrgc(), &r).args(["p.jrg", "--sopan"])), en);

    let salah = jalan(perintah(jrgc(), &r).args(["p.jrg", "--bahasa", "klingon"]));
    assert!(salah.contains("klingon") && salah.contains("en, id"), "bahasa tak dikenal tidak disebut:\n{salah}");
    let _ = std::fs::remove_dir_all(&r);
}

/// Kata Indonesia yang tidak mungkin ada di kalimat Inggris; nama, nilai dan kode di antara backtick dibuang dulu.
fn masih_indonesia(baris: &str) -> bool {
    const KATA: [&str; 16] = ["tidak", "bisa", "harus", "yang", "dengan", "belum", "pakai", "untuk", "kalau",
        "cuma", "sudah", "butuh", "dapat", "bukan", "karena", "tanpa"];
    let mut bersih = String::new();
    let mut dalam = false;
    for c in baris.chars() {
        if c == '`' { dalam = !dalam; continue; }
        if !dalam { bersih.push(c.to_ascii_lowercase()); }
    }
    let kata: Vec<&str> = bersih.split(|c: char| !c.is_alphanumeric()).collect();
    KATA.iter().filter(|k| kata.contains(k)).count() >= 2
}

/// Seluruh tabel `GAGAL_SAAT_JALAN` dalam bahasa Inggris: interpreter dan VM menjawab sama persis, dan baris
/// galatnya tidak lagi berbahasa Indonesia. Kasus yang memang mencetak teks Indonesia milik programnya dilewati
/// lewat `masih_indonesia` yang cuma membaca baris `error`/`help`/`note`.
#[test]
fn galat_saat_jalan_inggris_sama_di_dua_mesin() {
    let r = rumah_baru("");
    let (mut total, mut indo) = (0, Vec::new());
    for (nama, sumber, _, _) in GAGAL_SAAT_JALAN {
        let sumber = if sumber.contains("fn main") { sumber.to_string() }
                     else { format!("package main\npub fn main() {{\n{sumber}\n}}\n") };
        std::fs::write(r.join("p.jrg"), &sumber).unwrap();
        let a = jalan(perintah(jrgc(), &r).args(["p.jrg", "--sopan"]));
        let b = jalan(perintah(jrgc(), &r).args(["p.jrg", "--sopan", "--vm"]));
        // Kedalaman stack yang habis memang beda antar mesin; sisanya harus sama persis.
        let rata = |t: &str| t.lines().map(|l| if l.contains(" more calls") { "… and N more calls" } else { l })
            .collect::<Vec<_>>().join("\n");
        assert_eq!(rata(&a), rata(&b), "{nama}: interpreter dan VM beda dalam bahasa Inggris");
        for l in a.lines().filter(|l| l.starts_with("error") || l.trim_start().starts_with("= ")) {
            total += 1;
            if masih_indonesia(l) { indo.push(format!("{nama}: {l}")); }
        }
    }
    let _ = std::fs::remove_dir_all(&r);
    assert!(total > 200, "cuma {total} baris galat terbaca — tabelnya tidak terbaca utuh?");
    assert!(indo.len() * 50 <= total, "{} dari {total} baris galat masih Indonesia:\n  {}", indo.len(), indo.join("\n  "));
}

#[test]
fn bantuan_cli_ikut_bahasa() {
    let r = rumah_baru("");
    let en = jalan(perintah(jrgc(), &r).arg("-h"));
    assert!(en.contains("Run on the bytecode VM") && !en.contains("Jalankan lewat"), "{en}");
    let id = jalan(perintah(jrgc(), &r).args(["--bahasa", "id", "-h"]));
    assert!(id.contains("Jalankan lewat bytecode VM"), "{id}");
    let m = jalan(perintah(jrgm(), &r).arg("--help"));
    assert!(m.contains("Create a new JawaRustGo++ project"), "{m}");
    let _ = std::fs::remove_dir_all(&r);
}
