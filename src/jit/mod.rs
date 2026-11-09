//! JRGJIT, mesin eksekusi keempat (rancangan: `dokumentasi/jit/`). J1: VM berprofil; J2: fungsi bulat
//! murni dikompilasi lewat Cranelift (`j2-rencana.md`); J3: jaga yang gagal deopt ke VM di instruksi
//! yang sama, bingkai native disusun ulang jadi bingkai VM (`j3-rencana.md`).

pub mod profil;
pub mod laporan;
#[cfg(all(feature = "jit", not(target_arch = "wasm32")))]
pub mod layak;
#[cfg(all(feature = "jit", not(target_arch = "wasm32")))]
mod turun;
#[cfg(all(feature = "jit", not(target_arch = "wasm32")))]
pub(crate) mod modul;
#[cfg(all(feature = "jit", not(target_arch = "wasm32")))]
pub(crate) mod panggil;
#[cfg(all(feature = "jit", not(target_arch = "wasm32")))]
pub(crate) mod deopt;

use std::sync::atomic::{AtomicBool, Ordering};

static AKTIF: AtomicBool = AtomicBool::new(false);

/// Dinyalakan sekali oleh `jrgc --jit` sebelum program dikompilasi.
pub fn aktifkan() { AKTIF.store(true, Ordering::Relaxed); }

#[inline]
pub fn aktif() -> bool { AKTIF.load(Ordering::Relaxed) }

/// Pintu masuk kode native sebuah fungsi: `(args, kedalaman, keluar, catatan deopt) -> status`.
#[derive(Clone, Copy)]
pub struct KodeJit {
    pub masuk: unsafe extern "C" fn(*const i64, i32, *mut i64, *mut u8) -> i32,
}

impl std::fmt::Debug for KodeJit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("<kode jit>") }
}

/// Panggilan ke-berapa sebuah fungsi dikompilasi (`JRG_JIT_AMBANG`, bawaan 2; 0 = mode stres).
pub fn ambang() -> u32 {
    static A: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *A.get_or_init(|| std::env::var("JRG_JIT_AMBANG").ok().and_then(|s| s.parse().ok()).unwrap_or(2))
}
