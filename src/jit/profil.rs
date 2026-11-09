//! Profil per fungsi untuk tingkat T0 (`dokumentasi/jit/02-arsitektur.md`). Hanya diisi di mode
//! `--jit`; di `--vm` biasa `Chunk::profil` kosong dan perekamnya tidak pernah dipanggil.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use crate::vm::value::VmValue;

/// Bit tipe yang teramati di satu situs (beberapa bit = situs polimorfik).
pub mod bit {
    pub const BULAT: u8 = 1;
    pub const PECAHAN: u8 = 2;
    pub const TEKS: u8 = 4;
    pub const BOOL: u8 = 8;
    pub const LARIK: u8 = 16;
    pub const PETA: u8 = 32;
    pub const NIL: u8 = 64;
    pub const LAIN: u8 = 128;
}

pub fn bit_tipe(v: &VmValue) -> u8 {
    match v {
        VmValue::Int(_) => bit::BULAT,
        VmValue::Float(_) => bit::PECAHAN,
        VmValue::Str(_) => bit::TEKS,
        VmValue::Bool(_) => bit::BOOL,
        VmValue::Array(_) | VmValue::Tuple(_) => bit::LARIK,
        VmValue::Map(_) => bit::PETA,
        VmValue::Nil | VmValue::Unit => bit::NIL,
        _ => bit::LAIN,
    }
}

#[derive(Debug)]
pub struct ProfilFungsi {
    pub nama: String,
    pub panggilan: AtomicU32,
    /// Lompatan mundur (badan loop) yang dijalankan di fungsi ini.
    pub putaran: AtomicU32,
    /// Satu bitmask tipe per instruksi; hanya situs operator yang terisi.
    pub umpan: Box<[AtomicU8]>,
}

impl ProfilFungsi {
    #[inline]
    pub fn catat_panggilan(&self) { self.panggilan.fetch_add(1, Ordering::Relaxed); }
    #[inline]
    pub fn catat_putaran(&self) { self.putaran.fetch_add(1, Ordering::Relaxed); }
    #[inline]
    pub fn catat_tipe(&self, ip: usize, b: u8) {
        if let Some(s) = self.umpan.get(ip) { s.fetch_or(b, Ordering::Relaxed); }
    }
}

/// Semua profil yang dibuat di proses ini, untuk laporan akhir.
static DAFTAR: Mutex<Vec<Arc<ProfilFungsi>>> = Mutex::new(Vec::new());

/// Profil baru untuk chunk ber-`n` instruksi; `None` di luar mode `--jit`.
pub fn baru(nama: &str, n: usize) -> Option<Arc<ProfilFungsi>> {
    if !super::aktif() { return None; }
    let p = Arc::new(ProfilFungsi {
        nama: nama.to_string(),
        panggilan: AtomicU32::new(0),
        putaran: AtomicU32::new(0),
        umpan: (0..n).map(|_| AtomicU8::new(0)).collect(),
    });
    DAFTAR.lock().unwrap_or_else(|p| p.into_inner()).push(Arc::clone(&p));
    Some(p)
}

pub fn semua() -> Vec<Arc<ProfilFungsi>> {
    DAFTAR.lock().unwrap_or_else(|p| p.into_inner()).clone()
}
