// src/canvas/layan.rs — `Canvas::new`, `canvas::update` dan seluruh method kanvas atas nilai netral `NilaiK`.
// Tanpa `std`: SALINAN PERSISNYA di `runtime/src/canvas/layan.rs`, dijaga tes. Interpreter/VM dan runtime LLVM
// cuma menerjemahkan nilainya di pintu masuk, jadi tiap kalimat galat kanvas punya satu definisi.
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use alloc::collections::BTreeSet;
use core::time::Duration;
#[allow(unused_imports)]
use super::matek::*;
use super::inti::{Colour, Framebuffer, KeyState, MAKS_ELEMEN};
use super::{lanjut, pena, png};
use crate::saran_inti::saran_nama_terdekat;

/// Nilai JRG sejauh yang dibaca kanvas. `tampil` di `Struktur`/`Larik`/`Lain` cuma diisi untuk tempat teks
/// yang DIRENDER (`tempat_teks`); di tempat lain kosong, karena tak pernah dibaca.
#[derive(Debug, Clone)]
pub enum NilaiK {
    Unit,
    Bool(bool),
    Int(i128),
    Float(f64),
    Str(String),
    /// Struct milik program (`nama` tanpa awalan) atau dict (`nama` = `"dict"`).
    Struktur { nama: String, field: Vec<(String, NilaiK)>, tampil: String },
    Larik { isi: Vec<NilaiK>, tampil: String },
    /// Kanvas lain; nomornya dimengerti `Tuan::sumber`.
    Kanvas(usize),
    Lain { tipe: String, tampil: String },
}

impl NilaiK {
    /// Sama dengan `Value::type_name` mesin acuan.
    pub fn nama_tipe(&self) -> String {
        match self {
            NilaiK::Unit => "unit".to_string(),
            NilaiK::Bool(_) => "bool".to_string(),
            NilaiK::Int(_) => "int".to_string(),
            NilaiK::Float(_) => "float".to_string(),
            NilaiK::Str(_) => "string".to_string(),
            NilaiK::Struktur { nama, .. } if nama == "dict" => "dict".to_string(),
            NilaiK::Struktur { nama, .. } => format!("struct:{}", nama),
            NilaiK::Larik { .. } => "array".to_string(),
            NilaiK::Kanvas(_) => "canvas".to_string(),
            NilaiK::Lain { tipe, .. } => tipe.clone(),
        }
    }

    /// Sama dengan `Display` nilai acuan.
    pub fn tampil(&self) -> String {
        match self {
            NilaiK::Unit => "()".to_string(),
            NilaiK::Bool(b) => b.to_string(),
            NilaiK::Int(n) => n.to_string(),
            NilaiK::Float(f) => tampil_pecahan(*f),
            NilaiK::Str(s) => s.clone(),
            NilaiK::Struktur { tampil, .. } | NilaiK::Larik { tampil, .. } | NilaiK::Lain { tampil, .. } => tampil.clone(),
            NilaiK::Kanvas(_) => "<canvas>".to_string(),
        }
    }

    fn field(&self, k: &str) -> Option<&NilaiK> {
        match self {
            NilaiK::Struktur { field, .. } => field.iter().find(|(n, _)| n == k).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// Pecahan seperti `Display` acuan: bulat ditulis `.0`, sisanya cara Rust.
pub fn tampil_pecahan(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() { format!("{:.1}", n) } else { format!("{}", n) }
}

/// Indeks argumen method (sesudah kanvasnya) yang dirender jadi teks; `Canvas::new` memakai indeks 2.
pub fn tempat_teks(metode: &str) -> Option<usize> {
    match metode {
        "draw_text" | "draw_text_scaled" => Some(2),
        "set_title" => Some(0),
        _ => None,
    }
}

/// Hasil `Tuan::sajikan`: bingkai sudah ditampilkan di jendela/peramban (atau tidak ada tampilan).
pub enum Sajian { Tanpa, Tertutup, Masukan(Masukan) }

/// Masukan gabungan semua tampilan bingkai ini.
pub struct Masukan {
    pub ditekan: Vec<String>,
    pub ditahan: BTreeSet<String>,
    pub tetikus: Option<(f32, f32)>,
    pub tombol: Option<[bool; 3]>,
    pub jari: Option<Vec<(f32, f32)>>,
}

/// Piksel kanvas sumber `blit`.
pub enum Sumber { Diri, Lain { lebar: usize, tinggi: usize, piksel: Vec<u32> } }

/// Yang disediakan mesin: jam, berkas, tampilan, kanvas lain, dan kalimat method yang tak ada.
pub trait Tuan {
    fn sekarang(&self) -> Duration;
    /// Galatnya sudah berupa kalimat I/O mesin (`host::galat_io`).
    fn tulis_berkas(&mut self, jalur: &str, isi: &[u8]) -> Result<(), String>;
    fn galat_baris(&mut self, baris: &str);
    fn sajikan(&mut self, fb: &Framebuffer) -> Result<Sajian, String>;
    fn ganti_judul(&mut self, judul: &str);
    fn tutup(&mut self);
    fn sumber(&mut self, no: usize) -> Result<Sumber, String>;
    fn method_tak_ada(&self, nama: &str) -> String;
}

/// Pembuka mode tampilan `"tab"`/`"web"` untuk `Canvas::new`.
pub trait Pembuka {
    fn sekarang(&self) -> Duration;
    fn buka(&mut self, mode: &str, judul: &str, lebar: usize, tinggi: usize) -> Result<(), String>;
}

/// Keadaan satu kanvas, tanpa jendela dan peramban (itu milik mesinnya).
pub struct Kanvas {
    pub width:      usize,
    pub height:     usize,
    pub title:      String,
    pub fb:         Framebuffer,
    pub is_open:    bool,
    pub keys:       KeyState,
    pub mouse_x:    f32,
    pub mouse_y:    f32,
    pub mouse_btns: [bool; 3],
    /// Jari yang menempel bingkai ini (koordinat kanvas); di desktop: tetikus kiri.
    pub sentuhan:   Vec<(f32, f32)>,
    pub frame_idx:  u64,
    pub last_tick:  Duration,
    pub delta_sec:  f64,
    pub fps:        f64,
    pub headless_export_path: Option<String>,
    /// Transformasi aktif dan kotak pemotong, terpisah dari framebuffer supaya `push`/`pop` tidak menyalin gambar.
    pub st: lanjut::Keadaan,
    /// Mode `"png"`: tiap `update` menimpa berkas ini dengan bingkainya.
    pub png_berkas: Option<String>,
}

impl Kanvas {
    pub fn new(width: usize, height: usize, title: &str, sekarang: Duration) -> Self {
        Kanvas {
            width,
            height,
            title: title.to_string(),
            fb: Framebuffer::new(width, height),
            is_open: true,
            keys: KeyState::default(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_btns: [false; 3],
            sentuhan: Vec::new(),
            frame_idx: 0,
            last_tick: sekarang,
            delta_sec: 0.0,
            fps: 0.0,
            headless_export_path: None,
            st: lanjut::Keadaan::default(),
            png_berkas: None,
        }
    }
}

/// Mode tampilan yang sah untuk `Canvas::new(.., mode)`; `"default"` ejaan lain `"polos"`.
pub const MODE_KANVAS: &[&str] = &["polos", "tab", "web", "png"];

/// Pesan untuk ukuran kanvas yang bukan angka.
pub fn pesan_ukuran_bukan_angka(nama_arg: &str, tipe: &str) -> String {
    format!("Canvas::new: {} harus angka, dapat `{}`", nama_arg, tipe)
}

/// `Canvas::new(lebar, tinggi, judul?, mode?)` — SATU definisi untuk semua mesin. Dulu dua salinan diam-diam
/// mengubah ukuran 0 atau negatif jadi 1, dan ukuran raksasa mematikan prosesnya lewat alokasi yang gagal.
pub fn baru(args: &[NilaiK], p: &mut dyn Pembuka) -> Result<Kanvas, String> {
    if args.len() < 2 {
        return Err("Canvas::new butuh minimal 2 arg: Canvas::new(lebar, tinggi) \
                    atau Canvas::new(lebar, tinggi, \"judul\")".to_string());
    }
    let mut asli = [0i128; 2];
    for (i, nama) in [(0usize, "lebar"), (1, "tinggi")] {
        let n = match &args[i] {
            NilaiK::Int(n) => *n,
            NilaiK::Float(f) if f.is_finite() => f.trunc() as i128,
            NilaiK::Float(f) => return Err(format!("Canvas::new: {} harus angka yang terhingga, dapat `{}`",
                                                   nama, tampil_pecahan(*f))),
            lain => return Err(pesan_ukuran_bukan_angka(nama, &lain.nama_tipe())),
        };
        if n < 1 {
            return Err(format!("Canvas::new: {} harus paling sedikit 1, dapat {}", nama, n));
        }
        asli[i] = n;
    }
    // Ukuran yang DIMINTA yang disebut: dulu dijepit dulu, jadi 2^40 x 2 dilaporkan 16777216x2.
    let piksel = asli[0].checked_mul(asli[1]);
    if piksel.map_or(true, |p| p > MAKS_ELEMEN) {
        return Err(format!(
            "Canvas::new: kanvas {}x{} butuh {} piksel, batasnya {} — sebesar itu \
             mematikan prosesnya sebelum sempat menjawab. Cek lagi ukurannya",
            asli[0], asli[1], piksel.map_or("lebih dari 10^38".to_string(), |p| p.to_string()), MAKS_ELEMEN));
    }
    let ukuran = [asli[0] as usize, asli[1] as usize];
    // Judul adalah teks yang DITAMPILKAN, jadi nilai apa pun dirender.
    let judul = match args.get(2) {
        Some(v) => v.tampil(),
        None => "JRG Canvas".to_string(),
    };
    let mode = mode_kanvas(args.get(3))?;
    let mut k = Kanvas::new(ukuran[0], ukuran[1], &judul, p.sekarang());
    for m in &mode {
        match *m {
            "tab" | "web" => p.buka(m, &judul, ukuran[0], ukuran[1])?,
            "png" => k.png_berkas = Some(nama_berkas_png(&judul)),
            _ => {}
        }
    }
    Ok(k)
}

/// Mode dari argumen ke-4: teks atau array teks; tanpa argumen, `"polos"`.
fn mode_kanvas(v: Option<&NilaiK>) -> Result<Vec<&'static str>, String> {
    let satu = |s: &str| -> Result<&'static str, String> {
        let s = if s == "default" { "polos" } else { s };
        MODE_KANVAS.iter().copied().find(|m| *m == s).ok_or_else(|| {
            let saran = saran_nama_terdekat(s, MODE_KANVAS.iter().copied())
                .map(|x| format!(" — maksudnya `\"{}\"`?", x)).unwrap_or_default();
            format!("Canvas::new: mode `\"{}\"` tidak dikenal{}\nbantuan: mode yang ada `\"polos\"` \
                     (bawaan, tanpa layar), `\"tab\"` (jendela desktop), `\"web\"` (peramban lokal), \
                     `\"png\"` (berkas tiap bingkai); beberapa sekaligus ditulis sebagai array", s, saran)
        })
    };
    match v {
        None => Ok(vec!["polos"]),
        Some(NilaiK::Str(s)) => Ok(vec![satu(s)?]),
        Some(NilaiK::Larik { isi, .. }) if !isi.is_empty() => {
            let mut out = Vec::new();
            for x in isi.iter() {
                let NilaiK::Str(s) = x else {
                    return Err(format!("Canvas::new: tiap mode harus teks, dapat `{}`", x.nama_tipe()));
                };
                let m = satu(s)?;
                if !out.contains(&m) { out.push(m); }
            }
            Ok(out)
        }
        Some(lain) => Err(format!("Canvas::new: mode harus teks atau array teks yang tidak kosong, dapat `{}`",
                                  lain.nama_tipe())),
    }
}

/// `"Tangkap Bintang"` -> `tangkap_bintang.png`, di direktori kerja.
fn nama_berkas_png(judul: &str) -> String {
    let dasar: String = judul.chars().map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect();
    let dasar = dasar.trim_matches('_');
    format!("{}.png", if dasar.is_empty() { "kanvas" } else { dasar })
}

/// `canvas::update`: jam bingkai, tampilan dan masukannya, lalu berkas mode `"png"`/ekspor PPM.
pub fn perbarui(k: &mut Kanvas, t: &mut dyn Tuan) -> Result<bool, String> {
    if !k.is_open { return Ok(false); }

    let now = t.sekarang();
    let elapsed = now.saturating_sub(k.last_tick).as_secs_f64();
    k.delta_sec = elapsed;
    k.fps = if elapsed > 0.0 { 1.0 / elapsed } else { 0.0 };
    k.last_tick = now;
    k.frame_idx += 1;
    k.keys.pressed.clear();

    // Tiap mode lapisan tampilan atas framebuffer yang SAMA; masukannya digabung mesinnya.
    match t.sajikan(&k.fb)? {
        Sajian::Tanpa => {}
        Sajian::Tertutup => { k.is_open = false; return Ok(false); }
        Sajian::Masukan(m) => {
            k.keys.pressed.extend(m.ditekan);
            if let Some((x, y)) = m.tetikus { k.mouse_x = x; k.mouse_y = y; }
            if let Some(b) = m.tombol { k.mouse_btns = b; }
            k.keys.held = m.ditahan;
            // Tanpa layar sentuh, tombol kiri tetikus dihitung satu jari: satu kode untuk HP dan desktop.
            k.sentuhan = m.jari.unwrap_or_else(|| {
                if k.mouse_btns[0] { vec![(k.mouse_x, k.mouse_y)] } else { Vec::new() }
            });
        }
    }
    if let Some(berkas) = k.png_berkas.clone() {
        let isi = png::susun(k.width, k.height, &k.fb.pixels);
        t.tulis_berkas(&berkas, &isi).map_err(|e| format!("mode `png`: gagal menulis '{}': {}", berkas, e))?;
    }
    if let Some(base_path) = k.headless_export_path.clone() {
        let path = format!("{}_{:06}.ppm", base_path, k.frame_idx);
        // Gagal tulis dilaporkan: dulu `.ok()` menelannya, jadi bingkainya hilang diam-diam.
        t.tulis_berkas(&path, &k.fb.ppm())
            .map_err(|e| format!("set_headless_export: gagal menulis '{}': {}", path, e))?;
    }
    Ok(k.is_open)
}

/// Nama tombol yang bisa ditanyakan — persis nama varian `minifb::Key`, karena itu yang diisi tampilan ke
/// `KeyState`. Satu kosakata untuk semua jalur.
pub const TOMBOL: &[&str] = &[
    "Key0", "Key1", "Key2", "Key3", "Key4", "Key5", "Key6", "Key7", "Key8", "Key9",
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P",
    "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
    "F13", "F14", "F15",
    "Down", "Left", "Right", "Up", "Apostrophe", "Backquote", "Backslash", "Comma",
    "Equal", "LeftBracket", "Minus", "Period", "RightBracket", "Semicolon", "Slash",
    "Backspace", "Delete", "End", "Enter", "Escape", "Home", "Insert", "Menu",
    "PageDown", "PageUp", "Pause", "Space", "Tab", "NumLock", "CapsLock",
    "ScrollLock", "LeftShift", "RightShift", "LeftCtrl", "RightCtrl",
    "NumPad0", "NumPad1", "NumPad2", "NumPad3", "NumPad4", "NumPad5", "NumPad6",
    "NumPad7", "NumPad8", "NumPad9", "NumPadDot", "NumPadSlash", "NumPadAsterisk",
    "NumPadMinus", "NumPadPlus", "NumPadEnter", "LeftAlt", "RightAlt",
    "LeftSuper", "RightSuper",
];

/// Ejaan lain yang biasa ditulis orang, dan nama kanoniknya.
const TOMBOL_ALIAS: &[(&str, &str)] = &[
    (" ", "Space"), ("ArrowLeft", "Left"), ("ArrowRight", "Right"),
    ("ArrowUp", "Up"), ("ArrowDown", "Down"), ("Return", "Enter"), ("Esc", "Escape"),
    ("Shift", "LeftShift"), ("Ctrl", "LeftCtrl"), ("Control", "LeftCtrl"),
    ("Alt", "LeftAlt"), ("0", "Key0"), ("1", "Key1"), ("2", "Key2"), ("3", "Key3"),
    ("4", "Key4"), ("5", "Key5"), ("6", "Key6"), ("7", "Key7"), ("8", "Key8"),
    ("9", "Key9"),
];

/// Nama kanonik sebuah tombol, tanpa peduli huruf besar-kecil. Nama yang tidak dikenal DITOLAK: dulu ia dijawab
/// `false` selamanya, dan tak ada yang bilang.
pub fn kenali_tombol(nama: &str) -> Result<&'static str, String> {
    if let Some(t) = TOMBOL.iter().find(|t| t.eq_ignore_ascii_case(nama)) {
        return Ok(t);
    }
    if let Some((_, t)) = TOMBOL_ALIAS.iter().find(|(a, _)| a.eq_ignore_ascii_case(nama)) {
        return Ok(t);
    }
    let calon = TOMBOL.iter().copied().chain(TOMBOL_ALIAS.iter().map(|(a, _)| *a));
    let saran = saran_nama_terdekat(nama, calon)
        .map(|s| format!(" — maksudnya `{}`?", s)).unwrap_or_default();
    Err(format!(
        "`{}` bukan nama tombol yang dikenal{}\n\
         bantuan: nama tombol misalnya `Space`, `Enter`, `Escape`, `Left`/`Right`/`Up`/`Down`, \
         huruf `A`-`Z`, angka `0`-`9`, `F1`-`F15`, `LeftShift`, `LeftCtrl` \
         (huruf besar-kecil tidak berpengaruh)", nama, saran))
}

/// Warna dari nilai JRG: `0xRRGGBB` (opak), `0xAARRGGBB`, atau struct/dict ber-field `r`, `g`, `b` (dan `a`)
/// masing-masing 0 sampai 255. Yang lain DITOLAK — dulu jadi putih atau hitam diam-diam, dan `r: 300` jadi 44.
pub fn warna(v: &NilaiK) -> Result<Colour, String> {
    match v {
        NilaiK::Struktur { .. } => {
            let komponen = |k: &str, bawaan: Option<u8>| -> Result<u8, String> {
                match (v.field(k), bawaan) {
                    (Some(NilaiK::Int(n)), _) if (0..=255).contains(n) => Ok(*n as u8),
                    (Some(lain), _) => Err(format!(
                        "berisi komponen `{}` {} — tiap komponen warna harus bilangan bulat \
                         0 sampai 255", k,
                        match lain { NilaiK::Int(n) => n.to_string(), l => format!("bertipe `{}`", l.nama_tipe()) })),
                    (None, Some(b)) => Ok(b),
                    (None, None) => Err(format!(
                        "tidak punya komponen `{}` — warna butuh `r`, `g`, `b` (dan `a` opsional)", k)),
                }
            };
            Ok(Colour { r: komponen("r", None)?, g: komponen("g", None)?,
                        b: komponen("b", None)?, a: komponen("a", Some(255))? })
        }
        // `0xRRGGBB` berarti OPAK; `0xAARRGGBB` memakai alphanya sendiri.
        NilaiK::Int(n) if (0..=0xFFFF_FFFF).contains(n) => {
            let n = *n as u32;
            Ok(if n <= 0xFF_FFFF { Colour::rgb((n >> 16) as u8, (n >> 8) as u8, n as u8) }
               else { Colour::from_argb_u32(n) })
        }
        NilaiK::Int(n) => Err(format!("harus warna 0 sampai 0xFFFFFFFF, dapat {}", n)),
        lain => Err(format!(
            "harus warna — angka `0xRRGGBB`/`0xAARRGGBB` atau `Warna {{ r, g, b, a }}` — dapat `{}`",
            lain.nama_tipe())),
    }
}

/// Struct `Warna` hasil `get_pixel`.
pub fn nilai_warna(c: Colour) -> NilaiK {
    NilaiK::Struktur { nama: "Warna".to_string(), field: vec![
        ("r".to_string(), NilaiK::Int(c.r as i128)), ("g".to_string(), NilaiK::Int(c.g as i128)),
        ("b".to_string(), NilaiK::Int(c.b as i128)), ("a".to_string(), NilaiK::Int(c.a as i128)),
    ], tampil: String::new() }
}

fn nilai_posisi(x: f32, y: f32) -> NilaiK {
    NilaiK::Struktur { nama: "MousePos".to_string(), field: vec![
        ("x".to_string(), NilaiK::Float(x as f64)), ("y".to_string(), NilaiK::Float(y as f64)),
    ], tampil: String::new() }
}

/// Koordinat/ukuran bulat: bilangan bulat DIJENUHKAN ke jangkauan `i32` (dulu terbungkus, jadi `2^32` jatuh
/// di 0), pecahan terhingga dipotong, sisanya ditolak.
pub fn angka_i32(v: &NilaiK) -> Result<i32, String> {
    match v {
        NilaiK::Int(n) => Ok((*n).clamp(i32::MIN as i128, i32::MAX as i128) as i32),
        _ => angka_f64(v).map(|f| f as i32),
    }
}

/// Angka untuk menggambar. NaN dan tak hingga ditolak: dulu jadi 0 tanpa suara.
pub fn angka_f64(v: &NilaiK) -> Result<f64, String> {
    match v {
        NilaiK::Float(f) if f.is_finite() => Ok(*f),
        NilaiK::Float(f) => Err(format!("harus angka yang terhingga, dapat `{}`", tampil_pecahan(*f))),
        NilaiK::Int(n) => Ok(*n as f64),
        lain => Err(format!("harus angka, dapat `{}`", lain.nama_tipe())),
    }
}

/// Daftar titik: `[[x, y], ..]` atau rata `[x0, y0, x1, y1, ..]`.
fn ambil_titik(v: &NilaiK) -> Result<Vec<(f32, f32)>, String> {
    let NilaiK::Larik { isi: a, .. } = v else {
        return Err(format!("butuh array titik, dapat `{}`", v.nama_tipe()));
    };
    if a.is_empty() { return Ok(Vec::new()); }
    let angka = |x: &NilaiK, i: usize| -> Result<f32, String> {
        angka_f64(x).map(|f| f as f32).map_err(|e| format!("titik ke-{}: koordinatnya {}", i, e))
    };
    if let Some(NilaiK::Larik { .. }) = a.first() {
        let mut out = Vec::with_capacity(a.len());
        for (i, p) in a.iter().enumerate() {
            let NilaiK::Larik { isi: pair, .. } = p else {
                return Err(format!("titik ke-{} bukan pasangan [x,y]", i));
            };
            if pair.len() != 2 {
                return Err(format!("titik ke-{} punya {} angka, butuh tepat 2", i, pair.len()));
            }
            out.push((angka(&pair[0], i)?, angka(&pair[1], i)?));
        }
        return Ok(out);
    }
    if a.len() % 2 != 0 {
        return Err(format!(
            "daftar rata harus genap (pasangan x,y), dapat {} angka", a.len()
        ));
    }
    a.chunks(2).enumerate().map(|(i, c)| Ok((angka(&c[0], i)?, angka(&c[1], i)?))).collect()
}

/// Satu method `canvas::<metode>` atas kanvas `g` dengan argumen sesudah kanvasnya.
pub fn panggil(g: &mut Kanvas, t: &mut dyn Tuan, method: &str, args: &[NilaiK]) -> Result<NilaiK, String> {
    macro_rules! cerr {
        ($msg:expr) => { return Err(($msg).to_string()) };
        ($fmt:expr, $($arg:tt)*) => { return Err(format!($fmt, $($arg)*)) };
    }
    // Satu pintu piksel (`pena`) yang mematuhi transformasi dan clip.
    macro_rules! pena {
        () => { pena::Pena { fb: &mut g.fb, st: &g.st } };
    }

    // Argumen yang salah TIPE ditolak dengan satu kalimat (dulu jadi 0 atau putih tanpa suara). Yang HILANG
    // diberi bawaan, dan itu aman hanya karena jumlahnya sudah diperiksa pemanggilnya.
    let tolak = |idx: usize, alasan: String| -> String {
        format!("canvas::{}: argumen ke-{} {}", method, idx + 2, alasan)
    };
    let get_col = |idx: usize| -> Result<Colour, String> {
        match args.get(idx) {
            Some(v) => warna(v).map_err(|e| tolak(idx, e)),
            None => Ok(Colour::WHITE),
        }
    };
    let get_i = |idx: usize| -> Result<i32, String> {
        match args.get(idx) {
            Some(v) => angka_i32(v).map_err(|e| tolak(idx, e)),
            None => Ok(0),
        }
    };
    let get_f = |idx: usize| -> Result<f32, String> {
        match args.get(idx) {
            // Finite di 64 bit tapi tak hingga di 32 bit (`1e40`) ditolak, bukan jadi tak hingga diam-diam.
            Some(v) => angka_f64(v).and_then(|f| {
                let g = f as f32;
                if g.is_finite() { Ok(g) } else { Err(format!("angka `{}` terlalu besar untuk kanvas (pecahan 32 bit)", f)) }
            }).map_err(|e| tolak(idx, e)),
            None => Ok(0.0),
        }
    };
    let get_str = |idx: usize| -> String {
        args.get(idx).map(|v| v.tampil()).unwrap_or_default()
    };
    // Sebuah PATH menuntut string, dan tidak boleh jatuh ke `Display`.
    let get_path = |idx: usize, metode: &str| -> Result<String, String> {
        match args.get(idx) {
            Some(NilaiK::Str(s)) => Ok(s.clone()),
            Some(v) => Err(format!(
                "{}: path harus string, dapat `{}` — sebuah path tidak bisa \
                 diturunkan dari rendering sebuah nilai", metode, v.nama_tipe())),
            None => Err(format!("{}: path tidak diberikan", metode)),
        }
    };
    let nama_tombol = |idx: usize| -> Result<&'static str, String> {
        // Nama tombol adalah ISI, bukan teks yang dirender: `key_held(c, f)` dulu menanyakan tombol bernama
        // "<closure>" dan menjawab `false`.
        let NilaiK::Str(key) = &args[idx] else {
            return Err(tolak(idx, format!("harus nama tombol (string), dapat `{}`", args[idx].nama_tipe())));
        };
        kenali_tombol(key).map_err(|e| tolak(idx, e))
    };

    match method {
        // ── Keadaan jendela ──────────────────────────────────────────────
        "is_open"    => Ok(NilaiK::Bool(g.is_open)),
        "width"      => Ok(NilaiK::Int(g.width as i128)),
        "height"     => Ok(NilaiK::Int(g.height as i128)),
        "fps"        => Ok(NilaiK::Float(g.fps)),
        "delta_time" => Ok(NilaiK::Float(g.delta_sec)),
        "frame"      => Ok(NilaiK::Int(g.frame_idx as i128)),

        "set_title" => {
            if args.is_empty() { cerr!("set_title butuh 1 arg: (judul)"); }
            let new_title = get_str(0);
            t.ganti_judul(&new_title);
            g.title = new_title;
            Ok(NilaiK::Unit)
        }

        "close" => {
            g.is_open = false;
            // Pelayan mode `web` ikut berhenti dan portnya dilepas (dulu hidup sampai proses berakhir).
            t.tutup();
            Ok(NilaiK::Unit)
        }

        "update" => perbarui(g, t).map(NilaiK::Bool),

        // ── Menggambar ───────────────────────────────────────────────────
        "clear" => {
            if args.is_empty() { cerr!("clear butuh 1 arg: (warna) — warnanya adalah seluruh isi operasi ini"); }
            let col = get_col(0)?;
            g.fb.clear(col);
            Ok(NilaiK::Unit)
        }

        "draw_pixel" => {
            if args.len() < 3 { cerr!("draw_pixel butuh 3 arg: (x, y, warna)"); }
            let (x, y) = (get_f(0)?, get_f(1)?);
            let col = get_col(2)?;
            pena!().piksel(x, y, col);
            Ok(NilaiK::Unit)
        }

        "get_pixel" => {
            if args.len() < 2 { cerr!("get_pixel butuh 2 arg: (x, y)"); }
            let (x, y) = (get_i(0)?, get_i(1)?);
            Ok(nilai_warna(g.fb.get_pixel(x, y)))
        }

        "draw_line" => {
            if args.len() < 5 { cerr!("draw_line butuh 5 arg: (x0,y0,x1,y1,warna)"); }
            let (x0,y0,x1,y1) = (get_f(0)?,get_f(1)?,get_f(2)?,get_f(3)?);
            let col = get_col(4)?;
            pena!().garis(x0, y0, x1, y1, col);
            Ok(NilaiK::Unit)
        }

        "draw_rect" => {
            if args.len() < 5 { cerr!("draw_rect butuh 5 arg: (x,y,w,h,warna)"); }
            let (x,y,w,h) = (get_f(0)?,get_f(1)?,get_f(2)?,get_f(3)?);
            let col = get_col(4)?;
            pena!().kotak(x, y, w, h, col);
            Ok(NilaiK::Unit)
        }

        "fill_rect" => {
            if args.len() < 5 { cerr!("fill_rect butuh 5 arg: (x,y,w,h,warna)"); }
            let (x,y,w,h) = (get_f(0)?,get_f(1)?,get_f(2)?,get_f(3)?);
            let col = get_col(4)?;
            pena!().isi_kotak(x, y, w, h, col);
            Ok(NilaiK::Unit)
        }

        "draw_circle" => {
            if args.len() < 4 { cerr!("draw_circle butuh 4 arg: (cx,cy,r,warna)"); }
            let (cx,cy,r) = (get_f(0)?,get_f(1)?,get_i(2)?);
            let col = get_col(3)?;
            pena!().lingkaran(cx, cy, r, col);
            Ok(NilaiK::Unit)
        }

        "fill_circle" => {
            if args.len() < 4 { cerr!("fill_circle butuh 4 arg: (cx,cy,r,warna)"); }
            let (cx,cy,r) = (get_f(0)?,get_f(1)?,get_i(2)?);
            let col = get_col(3)?;
            pena!().isi_lingkaran(cx, cy, r, col);
            Ok(NilaiK::Unit)
        }

        "draw_triangle" => {
            if args.len() < 7 { cerr!("draw_triangle butuh 7 arg: (x0,y0,x1,y1,x2,y2,warna)"); }
            let p = [(get_f(0)?, get_f(1)?), (get_f(2)?, get_f(3)?), (get_f(4)?, get_f(5)?)];
            let col = get_col(6)?;
            pena!().segitiga(p, col);
            Ok(NilaiK::Unit)
        }

        "fill_triangle" => {
            if args.len() < 7 { cerr!("fill_triangle butuh 7 arg: (x0,y0,x1,y1,x2,y2,warna)"); }
            let p = [(get_f(0)?, get_f(1)?), (get_f(2)?, get_f(3)?), (get_f(4)?, get_f(5)?)];
            let col = get_col(6)?;
            pena!().isi_segitiga(p, col);
            Ok(NilaiK::Unit)
        }

        "draw_text" => {
            if args.len() < 4 { cerr!("draw_text butuh 4 arg: (x,y,text,warna)"); }
            let (x,y) = (get_f(0)?, get_f(1)?);
            let text  = get_str(2);
            let col   = get_col(3)?;
            pena!().teks(x, y, &text, 1, col);
            Ok(NilaiK::Unit)
        }

        // ── Masukan ──────────────────────────────────────────────────────
        "key_pressed" => {
            if args.is_empty() { cerr!("key_pressed butuh 1 arg: (tombol) — tombol yang mana?"); }
            let key = nama_tombol(0)?;
            Ok(NilaiK::Bool(g.keys.pressed.contains(key)))
        }

        "key_held" => {
            if args.is_empty() { cerr!("key_held butuh 1 arg: (tombol) — tombol yang mana?"); }
            let key = nama_tombol(0)?;
            Ok(NilaiK::Bool(g.keys.held.contains(key)))
        }

        "get_mouse_pos" => Ok(nilai_posisi(g.mouse_x, g.mouse_y)),

        "jumlah_sentuhan" => Ok(NilaiK::Int(g.sentuhan.len() as i128)),

        "sentuhan" => {
            if args.is_empty() { cerr!("sentuhan butuh 1 arg: (nomor jari)"); }
            let i = match &args[0] {
                NilaiK::Int(n) => *n,
                lain => return Err(tolak(0, format!("harus nomor jari (int), dapat `{}`", lain.nama_tipe()))),
            };
            let n = g.sentuhan.len();
            let Some(&(x, y)) = usize::try_from(i).ok().and_then(|i| g.sentuhan.get(i)) else {
                return Err(tolak(0, format!("tidak ada jari ke-{} — yang menempel cuma {} (lihat \
                                             `canvas::jumlah_sentuhan`)", i, n)));
            };
            Ok(nilai_posisi(x, y))
        }

        "get_mouse_button" => {
            if args.is_empty() { cerr!("get_mouse_button butuh 1 arg: (0 kiri, 1 tengah, 2 kanan)"); }
            // Nomornya mengikuti kalimat di atas (dan peramban): dulu `"1"` berarti tombol KANAN.
            let tombol = match &args[0] {
                NilaiK::Int(n) if (0..=2).contains(n) => *n as usize,
                NilaiK::Str(s) => match s.to_lowercase().as_str() {
                    "left" | "0" => 0, "middle" | "1" => 1, "right" | "2" => 2,
                    _ => return Err(tolak(0, format!(
                        "harus tombol tetikus — 0/\"left\", 1/\"middle\", atau 2/\"right\" — dapat \"{}\"", s))),
                },
                lain => return Err(tolak(0, format!(
                    "harus tombol tetikus — 0/\"left\", 1/\"middle\", atau 2/\"right\" — dapat `{}`",
                    match lain { NilaiK::Int(n) => n.to_string(), l => l.nama_tipe() }))),
            };
            // `mouse_btns` diisi urut kiri, KANAN, tengah (lihat tampilannya).
            Ok(NilaiK::Bool(g.mouse_btns[[0, 2, 1][tombol]]))
        }

        // ── Ekspor ───────────────────────────────────────────────────────
        "save_ppm" | "save_bmp" => {
            let path = get_path(0, method)?;
            if path.is_empty() { cerr!("{}: path tidak boleh kosong", method); }
            let isi = if method == "save_ppm" { g.fb.ppm() } else { g.fb.bmp() };
            match t.tulis_berkas(&path, &isi) {
                Ok(()) => Ok(NilaiK::Bool(true)),
                Err(e) => { t.galat_baris(&format!("canvas {}: {}", method, e)); Ok(NilaiK::Bool(false)) }
            }
        }

        "set_headless_export" => {
            let path = get_path(0, "set_headless_export")?;
            g.headless_export_path = if path.is_empty() { None } else { Some(path) };
            Ok(NilaiK::Unit)
        }

        // ── Pembantu sekaligus ───────────────────────────────────────────
        "draw_grid" => {
            if args.len() < 5 { cerr!("draw_grid butuh 5 arg: (cols,rows,cell_w,cell_h,warna)"); }
            let (cols,rows) = (get_i(0)?,get_i(1)?);
            let (cw,ch)     = (get_i(2)?,get_i(3)?);
            let col          = get_col(4)?;
            let (fw, fh) = (g.width as f32, g.height as f32);
            let mut p = pena!();
            // Garis yang jatuh di luar kanvas tidak dihitung satu per satu: `cols = 1e8` dulu berputar lama
            // sekali, dan `c * cw` meluap (AGI-20261119-09).
            let sampai = |n: i32, sel: i32, ukuran: f32| -> i64 {
                if n < 0 { -1 } else if sel <= 0 { 0 } else { (n as i64).min((ukuran as i64) / sel as i64 + 1) }
            };
            for c in 0..=sampai(cols, cw, fw) { let x = (c * cw as i64) as f32; p.garis(x, 0.0, x, fh, col); }
            for r in 0..=sampai(rows, ch, fh) { let y = (r * ch as i64) as f32; p.garis(0.0, y, fw, y, col); }
            Ok(NilaiK::Unit)
        }

        "draw_crosshair" => {
            if args.len() < 4 { cerr!("draw_crosshair butuh 4 arg: (x,y,size,warna)"); }
            let (x,y,size) = (get_f(0)?,get_f(1)?,get_f(2)?);
            let col = get_col(3)?;
            let mut p = pena!();
            p.garis(x-size, y, x+size, y, col);
            p.garis(x, y-size, x, y+size, col);
            Ok(NilaiK::Unit)
        }

        // ══ Blit: menggambar satu kanvas ke atas kanvas lain ═════════════
        "blit" | "blit_region" => {
            let Some(NilaiK::Kanvas(no)) = args.first() else {
                cerr!("{}: argumen pertamanya harus Canvas sumber, dapat `{}`",
                      method, args.first().map(|v| v.nama_tipe()).unwrap_or_else(|| "tidak ada".to_string()));
            };
            // Sumber dan tujuan yang SAMA harus disalin lebih dulu.
            let (sw, sh, spix) = match t.sumber(*no)? {
                Sumber::Diri => (g.width, g.height, g.fb.pixels.clone()),
                Sumber::Lain { lebar, tinggi, piksel } => (lebar, tinggi, piksel),
            };
            // `blit(src, dx, dy)` atau `blit_region(src, sx, sy, w, h, dx, dy)`.
            let (sx, sy, w, h, dx, dy) = if method == "blit" {
                // Posisinya WAJIB.
                if args.len() < 3 { cerr!("blit butuh (sumber, x, y)"); }
                (0i32, 0i32, sw as i32, sh as i32, get_f(1)?, get_f(2)?)
            } else {
                if args.len() < 7 {
                    cerr!("blit_region butuh (sumber, sx, sy, lebar, tinggi, dx, dy)");
                }
                (get_i(1)?, get_i(2)?, get_i(3)?, get_i(4)?, get_f(5)?, get_f(6)?)
            };
            if w <= 0 || h <= 0 {
                cerr!("{}: lebar dan tinggi harus lebih dari 0, dapat {}x{}", method, w, h);
            }
            pena!().blit(&spix, sw, sh, (sx, sy, w, h), dx, dy);
            Ok(NilaiK::Bool(true))
        }

        // ══ Ekspor PNG ═══════════════════════════════════════════════════
        "save_png" => {
            let path = get_path(0, "save_png")?;
            // Sejajar dengan `save_ppm`/`save_bmp`: tanpa penjaga ini yang keluar errno mentah untuk
            // kesalahan yang sudah diketahui namanya.
            if path.is_empty() { cerr!("save_png: path tidak boleh kosong"); }
            let isi = png::susun(g.width, g.height, &g.fb.pixels);
            // Gagal tulis dilaporkan dengan sebabnya, bukan `false` polos.
            match t.tulis_berkas(&path, &isi) {
                Ok(()) => Ok(NilaiK::Bool(true)),
                Err(e) => cerr!("save_png '{}': {}", path, e),
            }
        }

        // ══ Transformasi ═════════════════════════════════════════════════
        "push"  => {
            if !g.st.simpan() {
                cerr!("push: tumpukan keadaan penuh ({} tingkat) — tiap `push` butuh `pop` pasangannya",
                      lanjut::Keadaan::MAKS_TUMPUKAN);
            }
            Ok(NilaiK::Unit)
        }
        "pop"   => {
            if !g.st.pulihkan() {
                cerr!("pop: tidak ada keadaan tersimpan — tiap `pop` butuh `push` sebelumnya");
            }
            Ok(NilaiK::Unit)
        }
        "translate" => {
            if args.len() < 2 { cerr!("translate butuh 2 arg: (dx, dy)"); }
            let (dx, dy) = (get_f(0)?, get_f(1)?);
            g.st.trafo = lanjut::Transformasi::geser(dx, dy).lalu(&g.st.trafo);
            Ok(NilaiK::Unit)
        }
        "scale" => {
            if args.is_empty() { cerr!("scale butuh 1 atau 2 arg: (s) untuk seragam, atau (sx, sy)"); }
            let (sx, sy) = (get_f(0)?, if args.len() > 1 { get_f(1)? } else { get_f(0)? });
            g.st.trafo = lanjut::Transformasi::skala(sx, sy).lalu(&g.st.trafo);
            Ok(NilaiK::Unit)
        }
        "rotate" => {
            if args.is_empty() { cerr!("rotate butuh 1 arg: (sudut dalam DERAJAT)"); }
            // Derajat, bukan radian: `rotate(90)` yang ternyata 90 radian adalah kejutan yang mahal.
            let d = get_f(0)?.to_radians();
            g.st.trafo = lanjut::Transformasi::putar(d).lalu(&g.st.trafo);
            Ok(NilaiK::Unit)
        }
        "reset_transform" => {
            g.st.trafo = lanjut::Transformasi::IDENTITAS;
            Ok(NilaiK::Unit)
        }
        "clip" => {
            if args.len() < 4 { cerr!("clip butuh 4 arg: (x,y,w,h)"); }
            let (x, y, w, h) = (get_i(0)?, get_i(1)?, get_i(2)?, get_i(3)?);
            // Jenuh, bukan meluap jadi negatif dan diam-diam memblokir semua gambar (AGI-20261119-09).
            g.st.potong = Some((x, y, x.saturating_add(w), y.saturating_add(h)));
            Ok(NilaiK::Unit)
        }
        "clip_reset" => { g.st.potong = None; Ok(NilaiK::Unit) }

        // ══ Bentuk halus (anti-alias) ════════════════════════════════════
        "line_aa" => {
            if args.len() < 5 { cerr!("line_aa butuh 5 arg: (x0,y0,x1,y1,warna)"); }
            let (x0, y0, x1, y1) = (get_f(0)?, get_f(1)?, get_f(2)?, get_f(3)?);
            let col = get_col(4)?;
            let st = g.st.clone();
            lanjut::Kuas { fb: &mut g.fb, st: &st }.garis_aa(x0, y0, x1, y1, col);
            Ok(NilaiK::Unit)
        }
        "fill_poly" | "draw_poly" => {
            if args.len() < 2 { cerr!("{} butuh 2 arg: (titik, warna)", method); }
            let titik = ambil_titik(&args[0]).map_err(|e| format!("{}: {}", method, e))?;
            let col = get_col(1)?;
            let st = g.st.clone();
            let mut k = lanjut::Kuas { fb: &mut g.fb, st: &st };
            if method == "fill_poly" { k.poligon(&titik, col) } else { k.garis_poligon(&titik, col, true) }
            Ok(NilaiK::Unit)
        }
        "fill_ellipse" | "draw_ellipse" => {
            if args.len() < 5 { cerr!("{} butuh 5 arg: (cx,cy,rx,ry,warna)", method); }
            let (cx, cy, rx, ry) = (get_f(0)?, get_f(1)?, get_f(2)?, get_f(3)?);
            let col = get_col(4)?;
            let titik = lanjut::Kuas::elips_titik(cx, cy, rx, ry, 0.0, core::f32::consts::TAU);
            let st = g.st.clone();
            let mut k = lanjut::Kuas { fb: &mut g.fb, st: &st };
            if method == "fill_ellipse" { k.poligon(&titik, col) } else { k.garis_poligon(&titik, col, true) }
            Ok(NilaiK::Unit)
        }
        "draw_arc" => {
            if args.len() < 6 { cerr!("draw_arc butuh 6 arg: (cx,cy,r,derajat_mulai,derajat_akhir,warna)"); }
            let (cx, cy, r) = (get_f(0)?, get_f(1)?, get_f(2)?);
            let (m, a) = (get_f(3)?.to_radians(), get_f(4)?.to_radians());
            let col = get_col(5)?;
            let titik = lanjut::Kuas::elips_titik(cx, cy, r, r, m, a);
            let st = g.st.clone();
            lanjut::Kuas { fb: &mut g.fb, st: &st }.garis_poligon(&titik, col, false);
            Ok(NilaiK::Unit)
        }
        "fill_round_rect" | "draw_round_rect" => {
            if args.len() < 6 { cerr!("{} butuh 6 arg: (x,y,w,h,radius,warna)", method); }
            let (x, y, w, h, r) = (get_f(0)?, get_f(1)?, get_f(2)?, get_f(3)?, get_f(4)?);
            let col = get_col(5)?;
            let titik = lanjut::kotak_bulat_titik(x, y, w, h, r);
            let st = g.st.clone();
            let mut k = lanjut::Kuas { fb: &mut g.fb, st: &st };
            if method == "fill_round_rect" { k.poligon(&titik, col) } else { k.garis_poligon(&titik, col, true) }
            Ok(NilaiK::Unit)
        }
        "draw_bezier" => {
            if args.len() < 2 { cerr!("draw_bezier butuh 2 arg: (titik_kendali, warna)"); }
            let p = ambil_titik(&args[0]).map_err(|e| format!("draw_bezier: {}", e))?;
            if p.len() < 3 { cerr!("draw_bezier butuh minimal 3 titik kendali"); }
            let col = get_col(1)?;
            let titik = lanjut::Kuas::bezier_titik(&p);
            let st = g.st.clone();
            lanjut::Kuas { fb: &mut g.fb, st: &st }.garis_poligon(&titik, col, false);
            Ok(NilaiK::Unit)
        }

        // ══ Gradien ══════════════════════════════════════════════════════
        "fill_linear_gradient" => {
            if args.len() < 6 { cerr!("fill_linear_gradient butuh 6 arg: (x0,y0,x1,y1,warna0,warna1)"); }
            let (x0, y0, x1, y1) = (get_f(0)?, get_f(1)?, get_f(2)?, get_f(3)?);
            let (c0, c1) = (get_col(4)?, get_col(5)?);
            let st = g.st.clone();
            lanjut::Kuas { fb: &mut g.fb, st: &st }.gradien_linear(x0, y0, x1, y1, c0, c1);
            Ok(NilaiK::Unit)
        }
        "fill_radial_gradient" => {
            if args.len() < 5 { cerr!("fill_radial_gradient butuh 5 arg: (cx,cy,r,warna_dalam,warna_luar)"); }
            let (cx, cy, r) = (get_f(0)?, get_f(1)?, get_f(2)?);
            let (c0, c1) = (get_col(3)?, get_col(4)?);
            let st = g.st.clone();
            lanjut::Kuas { fb: &mut g.fb, st: &st }.gradien_radial(cx, cy, r, c0, c1);
            Ok(NilaiK::Unit)
        }

        // ══ Teks berskala ════════════════════════════════════════════════
        "draw_text_scaled" => {
            if args.len() < 5 { cerr!("draw_text_scaled butuh 5 arg: (x,y,teks,skala,warna)"); }
            let (x, y) = (get_f(0)?, get_f(1)?);
            let teks = get_str(2);
            let skala = get_i(3)?;
            if skala < 1 {
                return Err(tolak(3, format!("adalah skala, dan harus 1 atau lebih, dapat {}", skala)));
            }
            let col = get_col(4)?;
            pena!().teks(x, y, &teks, skala, col);
            Ok(NilaiK::Unit)
        }

        // Kalimat yang sama dengan setiap penerima lain; sarannya diturunkan dari registry `canvas::` mesinnya.
        other => Err(t.method_tak_ada(other)),
    }
}
