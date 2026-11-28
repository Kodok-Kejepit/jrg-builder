// src/canvas/mod.rs — `Canvas::` di interpreter dan VM: pegangan bersama, penyaji (jendela, peramban), dan
// penerjemah `Value` ↔ `NilaiK` di depan inti bersama (`inti`, `layan`, `lanjut`, `pena`, `png`, `png_baca`), yang
// dipakai apa adanya oleh runtime LLVM. Biner LLVM meminjam `Penyaji` lewat jembatan (`jrg_kanvas_saji_*`).

#![allow(dead_code, unused_imports)]

use std::sync::{Arc, Mutex};
use std::collections::HashSet;

use crate::interpreter::Value;

pub mod matek;
pub mod inti;
pub mod layan;
pub mod lanjut;
pub mod pena;
pub mod png;
pub mod png_baca;
#[cfg(feature = "canvas")]
#[cfg_attr(target_os = "android", path = "jendela_android.rs")]
pub mod jendela;
#[cfg(not(target_arch = "wasm32"))]
pub mod web;

pub use inti::*;
pub use layan::{Kanvas, NilaiK, MODE_KANVAS, TOMBOL, kenali_tombol, pesan_ukuran_bukan_angka};

/// Tampilan sebuah kanvas: jendela `"tab"` dan peramban `"web"`. Mode `"polos"`/`"png"` tidak butuh apa pun di sini.
#[derive(Default)]
pub struct Penyaji {
    #[cfg(feature = "canvas")]
    pub window: Option<jendela::Jendela>,
    #[cfg(not(target_arch = "wasm32"))]
    pub web: Option<web::Web>,
}

impl Penyaji {
    /// Buka mode `"tab"`/`"web"` (`Canvas::new`); gagal membuka DILAPORKAN — dulu `.ok()` menelannya dan
    /// `update` pertama menutup kanvas tanpa pesan.
    #[allow(unreachable_code)]
    pub fn buka(&mut self, mode: &str, judul: &str, lebar: usize, tinggi: usize) -> Result<(), String> {
        let _ = (judul, lebar, tinggi);
        match mode {
            "tab" => {
                #[cfg(feature = "canvas")]
                { self.window = Some(jendela::Jendela::buka(judul, lebar, tinggi).map_err(|e| format!("Canvas::new: {}", e))?); }
                #[cfg(not(feature = "canvas"))]
                return Err("Canvas::new: mode `\"tab\"` butuh jendela, dan biner ini dibangun tanpa \
                            fitur `canvas` — bangun dengan `cargo build --release` (fitur bawaan) \
                            atau pakai mode `\"web\"`/`\"png\"`".to_string());
            }
            "web" => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let w = web::Web::buka(judul, lebar, tinggi).map_err(|e| format!("Canvas::new: {}", e))?;
                    crate::host::galat_baris(&format!("kanvas `{}` tampil di {}", judul, w.alamat));
                    self.web = Some(w);
                }
                #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                return Err("Canvas::new: mode `\"web\"` tidak tersedia di peramban".to_string());
                // WASI preview1 tidak bisa membuka soket pendengar.
                #[cfg(all(target_arch = "wasm32", not(target_os = "unknown")))]
                return Err("Canvas::new: mode `\"web\"` tidak tersedia di WASI, karena ia tidak bisa membuka soket; \
                            pakai mode `\"png\"`".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    /// Tampilkan satu bingkai di tiap tampilan dan gabungkan masukannya (`canvas::update`).
    pub fn sajikan(&mut self, piksel: &[u32]) -> Result<layan::Sajian, String> {
        // Di wasm tanpa jendela kedua sumber masukan di bawah tidak ada, jadi ini tidak pernah diubah.
        #[cfg_attr(all(target_arch = "wasm32", not(feature = "canvas")), allow(unused_mut))]
        let mut m: Option<layan::Masukan> = None;
        #[cfg(feature = "canvas")]
        if let Some(j) = self.window.as_ref() {
            let laporan = j.tampilkan(piksel)?;
            if let Some(e) = laporan.galat { return Err(e); }
            if !laporan.terbuka {
                self.window = None;
                return Ok(layan::Sajian::Tertutup);
            }
            m = Some(layan::Masukan {
                ditekan: laporan.ditekan,
                ditahan: laporan.ditahan.into_iter().collect(),
                tetikus: laporan.tetikus,
                tombol: Some(laporan.tombol_tetikus),
                jari: laporan.sentuhan,
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(w) = self.web.as_ref() {
            w.kirim(piksel);
            let (tekan, tahan, tetikus, tombol) = {
                let mut b = w.bersama.lock().unwrap_or_else(|p| p.into_inner());
                (std::mem::take(&mut b.ditekan), b.ditahan.clone(), b.tetikus, b.tombol_tetikus)
            };
            // `Escape` menutup, sama dengan jendela `"tab"`.
            if tahan.contains("Escape") { self.web = None; return Ok(layan::Sajian::Tertutup); }
            match m.as_mut() {
                Some(m) => { m.ditekan.extend(tekan); m.ditahan.extend(tahan); }
                None => m = Some(layan::Masukan {
                    ditekan: tekan.into_iter().collect(),
                    ditahan: tahan.into_iter().collect(),
                    tetikus: Some(tetikus),
                    tombol: Some(tombol),
                    jari: None,
                }),
            }
        }
        let _ = piksel;
        Ok(m.map_or(layan::Sajian::Tanpa, layan::Sajian::Masukan))
    }

    pub fn judul(&self, judul: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(w) = self.web.as_ref() { w.bersama.lock().unwrap_or_else(|p| p.into_inner()).judul = judul.to_string(); }
        #[cfg(feature = "canvas")]
        if let Some(j) = self.window.as_ref() { j.judul(judul); }
        let _ = judul;
    }

    /// Pelayan mode `web` ikut berhenti dan portnya dilepas (dulu hidup sampai proses berakhir).
    pub fn tutup(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        { self.web = None; }
        #[cfg(feature = "canvas")]
        { self.window = None; }
    }
}

impl layan::Pembuka for Penyaji {
    fn sekarang(&self) -> std::time::Duration { crate::host::monoton() }
    fn buka(&mut self, mode: &str, judul: &str, lebar: usize, tinggi: usize) -> Result<(), String> {
        Penyaji::buka(self, mode, judul, lebar, tinggi)
    }
}

/// Satu kanvas milik interpreter/VM: keadaannya (`Kanvas`) plus tampilannya.
pub struct CanvasInner {
    pub k: Kanvas,
    pub saji: Penyaji,
}

impl std::ops::Deref for CanvasInner {
    type Target = Kanvas;
    fn deref(&self) -> &Kanvas { &self.k }
}

impl std::ops::DerefMut for CanvasInner {
    fn deref_mut(&mut self) -> &mut Kanvas { &mut self.k }
}

impl std::fmt::Debug for CanvasInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CanvasInner({}x{}, open={}, frame={})",
            self.k.width, self.k.height, self.k.is_open, self.k.frame_idx)
    }
}

pub type CanvasHandle = Arc<Mutex<CanvasInner>>;

pub fn canvas_new(width: usize, height: usize, title: &str) -> CanvasHandle {
    let k = Kanvas::new(width, height, title, crate::host::monoton());
    Arc::new(Mutex::new(CanvasInner { k, saji: Penyaji::default() }))
}

pub type CanvasResult<T> = Result<T, String>;

// ─── Value ↔ NilaiK ────────────────────────────────────────────────────────

/// `render`: tempat teks yang DIRENDER, jadi yang bukan skalar cukup dibawa tampilannya.
fn ke_nilai(v: &Value, render: bool, kanvas: &mut Vec<CanvasHandle>) -> NilaiK {
    match v {
        Value::Unit => NilaiK::Unit,
        Value::Bool(b) => NilaiK::Bool(*b),
        Value::Int(n) => NilaiK::Int(*n),
        Value::Float(f) => NilaiK::Float(*f),
        Value::Str(s) => NilaiK::Str(String::clone(s)),
        _ if render => NilaiK::Lain { tipe: v.type_name(), tampil: format!("{}", v) },
        Value::Struct { name, fields } => NilaiK::Struktur {
            nama: name.clone(),
            field: fields.iter().map(|(k, x)| (k.to_string(), ke_nilai(x, false, kanvas))).collect(),
            tampil: String::new(),
        },
        Value::Array(a) => NilaiK::Larik { isi: a.iter().map(|x| ke_nilai(x, false, kanvas)).collect(), tampil: String::new() },
        Value::Canvas(h) => { kanvas.push(Arc::clone(h)); NilaiK::Kanvas(kanvas.len() - 1) }
        lain => NilaiK::Lain { tipe: lain.type_name(), tampil: String::new() },
    }
}

fn dari_nilai(n: NilaiK) -> Value {
    match n {
        NilaiK::Unit => Value::Unit,
        NilaiK::Bool(b) => Value::Bool(b),
        NilaiK::Int(i) => Value::Int(i),
        NilaiK::Float(f) => Value::Float(f),
        NilaiK::Str(s) => Value::Str(Arc::new(s)),
        NilaiK::Struktur { nama, field, .. } => {
            let fields: std::collections::HashMap<String, Value> =
                field.into_iter().map(|(k, v)| (k, dari_nilai(v))).collect();
            Value::Struct { name: nama, fields: fields.into() }
        }
        // `panggil` tidak pernah menjawab ketiganya.
        NilaiK::Larik { .. } | NilaiK::Kanvas(_) | NilaiK::Lain { .. } => Value::Unit,
    }
}

fn galat(e: String) -> crate::interpreter::Signal {
    crate::interpreter::Signal::RuntimeError(crate::interpreter::RuntimeError::new(e))
}

// ─── Mesin: jam, berkas, penyaji, kanvas lain ──────────────────────────────

struct TuanJrgc<'a> {
    saji: &'a mut Penyaji,
    sendiri: &'a CanvasHandle,
    lain: &'a [CanvasHandle],
}

impl layan::Tuan for TuanJrgc<'_> {
    fn sekarang(&self) -> std::time::Duration { crate::host::monoton() }

    fn tulis_berkas(&mut self, jalur: &str, isi: &[u8]) -> Result<(), String> {
        std::fs::write(jalur, isi).map_err(|e| crate::host::galat_io(&e))
    }

    fn galat_baris(&mut self, baris: &str) { crate::host::galat_baris(baris) }

    fn sajikan(&mut self, fb: &Framebuffer) -> Result<layan::Sajian, String> { self.saji.sajikan(&fb.pixels) }

    fn ganti_judul(&mut self, judul: &str) { self.saji.judul(judul) }

    fn tutup(&mut self) { self.saji.tutup() }

    fn sumber(&mut self, no: usize) -> Result<layan::Sumber, String> {
        let h = &self.lain[no];
        if Arc::ptr_eq(h, self.sendiri) { return Ok(layan::Sumber::Diri); }
        let src = h.lock().map_err(|e| format!("canvas sumber lock: {}", e))?;
        Ok(layan::Sumber::Lain { lebar: src.k.width, tinggi: src.k.height, piksel: src.k.fb.pixels.clone() })
    }

    // Kalimat yang sama dengan setiap penerima lain, di kedua backend — dan sarannya diturunkan dari registry
    // `canvas::`, bukan dari daftar tangan.
    fn method_tak_ada(&self, nama: &str) -> String {
        let calon = crate::interpreter::calon_method("canvas");
        let saran = crate::diag::saran_nama_terdekat(nama, calon.iter().map(|s| s.as_str()));
        crate::interpreter::pesan_method_tak_ada("canvas", nama, "", saran.as_deref())
    }
}

/// `Canvas::new(lebar, tinggi, judul?, mode?)` — satu definisi (`layan::baru`) untuk semua mesin.
pub fn canvas_baru(args: &[Value]) -> Result<CanvasHandle, String> {
    let mut lain = Vec::new();
    let nilai: Vec<NilaiK> = args.iter().enumerate().map(|(i, v)| ke_nilai(v, i == 2, &mut lain)).collect();
    let mut saji = Penyaji::default();
    let k = layan::baru(&nilai, &mut saji)?;
    Ok(Arc::new(Mutex::new(CanvasInner { k, saji })))
}

/// Satu method `canvas::` dari interpreter (dan VM lewat jembatan nilainya).
pub fn dispatch_canvas_method(handle: CanvasHandle, method: &str, args: &[Value]) -> crate::interpreter::EvalResult {
    let teks = layan::tempat_teks(method);
    let mut lain = Vec::new();
    let nilai: Vec<NilaiK> = args.iter().enumerate().map(|(i, v)| ke_nilai(v, teks == Some(i), &mut lain)).collect();
    let mut g = handle.lock().map_err(|e| galat(format!("canvas lock: {}", e)))?;
    let g = &mut *g;
    let mut t = TuanJrgc { saji: &mut g.saji, sendiri: &handle, lain: &lain };
    layan::panggil(&mut g.k, &mut t, method, &nilai).map(dari_nilai).map_err(galat)
}

/// Apakah kanvas di sini disentuh jari (APK Android), bukan diklik tetikus.
pub fn layar_sentuh() -> bool {
    #[cfg(all(feature = "canvas", target_os = "android"))]
    { jendela::ada_layar() }
    #[cfg(not(all(feature = "canvas", target_os = "android")))]
    { false }
}

/// Apakah biner ini bisa membuka jendela sungguhan (`--features canvas`). Tanpanya, `canvas::update` selalu
/// `true` dan putaran game berputar tanpa layar.
pub fn punya_jendela() -> bool {
    // Fitur jendelanya ada DAN ada layar: tanpa layar, contoh game memakai `simulasi`.
    let ada = |v: &str| std::env::var_os(v).map_or(false, |x| !x.is_empty());
    #[cfg(all(feature = "canvas", target_os = "android"))]
    { let _ = ada; jendela::ada_layar() }
    #[cfg(not(all(feature = "canvas", target_os = "android")))]
    { cfg!(feature = "canvas") && (ada("DISPLAY") || ada("WAYLAND_DISPLAY")) }
}
