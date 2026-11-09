//! JRGJIT, mesin eksekusi keempat (rancangan: `dokumentasi/jit/`). Fase J1: `--jit` menjalankan
//! VM dengan perekam profil (panggilan, putaran, tipe per situs) — bahan untuk J2 dan seterusnya.

pub mod profil;
pub mod laporan;

use std::sync::atomic::{AtomicBool, Ordering};

static AKTIF: AtomicBool = AtomicBool::new(false);

/// Dinyalakan sekali oleh `jrgc --jit` sebelum program dikompilasi.
pub fn aktifkan() { AKTIF.store(true, Ordering::Relaxed); }

#[inline]
pub fn aktif() -> bool { AKTIF.load(Ordering::Relaxed) }
