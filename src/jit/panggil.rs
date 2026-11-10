//! Pintu dari VM ke kode native: bingkai VM teratas dijalankan native mulai dari `ip`-nya — awal fungsi
//! sesudah `Vm::call_value` mendorong bingkai, atau kepala loop lewat OSR. Yang deopt dilanjutkan VM.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::deopt;
use super::layak::{jenis_nilai, Jenis, KEMBALI_SKALAR, KEMBALI_TIDAK};
use super::turun::{BERHASIL, BERHASIL_BOOL, BERHASIL_PECAHAN, DEOPT, KEDALAMAN_MAKS, ULANG};
use super::{KodeJit, Masuk};
use crate::vm::value::{VmFunction, VmValue};
use crate::vm::vm::Vm;

/// Titik masuk per chunk paling banyak (tiap susunan jenis berbeda dikompilasi sendiri).
const MAKS_MASUK: usize = 8;
/// Bingkai yang lebih tinggi dari ini tidak dimasuki.
const MAKS_POSISI: usize = 256;

fn kode_jenis(j: Jenis) -> u8 {
    match j {
        Jenis::Bulat => 1,
        Jenis::Bool => 2,
        Jenis::Kosong => 3,
        Jenis::Pecahan => 5,
        Jenis::Nilai | Jenis::Fungsi(_) => 4,
    }
}

/// Panggilan baru: bingkainya baru didorong `call_value`.
pub(crate) fn coba(vm: &mut Vm) {
    let Some(fr) = vm.frames.last() else { return };
    let Some(p) = fr.func.chunk.profil.as_ref() else { return };
    if p.panggilan.load(Ordering::Relaxed) < super::ambang().max(1) { return; }
    if fr.func.chunk.kode_jit.panggilan_mati.load(Ordering::Relaxed) { return; }
    // Kode yang lebih sering deopt daripada selesai cuma menambah ongkos masuk-keluar: VM saja.
    let (m, d) = (p.jit_masuk.load(Ordering::Relaxed), p.jit_deopt.load(Ordering::Relaxed));
    if d >= 64 && d * 2 > m { return; }
    let p = Arc::clone(p);
    p.jit_masuk.fetch_add(1, Ordering::Relaxed);
    if jalankan(vm, KEMBALI_SKALAR) == DEOPT { p.jit_deopt.fetch_add(1, Ordering::Relaxed); }
}

/// OSR: VM baru melompat mundur ke kepala loop di bingkai teratas.
pub(crate) fn coba_osr(vm: &mut Vm) {
    // `defer` yang sudah terdaftar dijalankan `Return` VM; kode OSR tidak pernah kembali sendiri.
    if vm.frames.last().is_none_or(|f| !f.defers.is_empty()) { return; }
    let _ = jalankan(vm, KEMBALI_TIDAK);
}

fn kode_untuk(vm: &Vm, func: &Arc<VmFunction>, ip: usize, kembali: u8, basis: usize) -> Option<KodeJit> {
    let isi = &vm.stack[basis..];
    let mut pintu = func.chunk.kode_jit.masuk.lock().unwrap_or_else(|p| p.into_inner());
    let cocok = |m: &Masuk| m.ip as usize == ip && m.kembali == kembali && m.jenis.len() == isi.len()
        && m.jenis.iter().zip(isi).all(|(k, v)| *k == kode_jenis(jenis_nilai(v)));
    if let Some(m) = pintu.iter().find(|m| cocok(m)) { return m.kode; }
    if pintu.len() >= MAKS_MASUK { return None; }
    let jenis: Vec<Jenis> = isi.iter().map(jenis_nilai).collect();
    let globals = &vm.globals;
    let cari = |nama: &str| match globals.get(nama) {
        Some(VmValue::Function(g)) => Some(Arc::clone(g)),
        _ => None,
    };
    let kode = super::modul::kompilasi(func, ip, &jenis, kembali, &cari);
    // Bentuk lain jarang menolong fungsi yang sudah gagal sekali (J6 memakai umpan tipe untuk itu).
    if kode.is_none() && kembali == KEMBALI_SKALAR {
        func.chunk.kode_jit.panggilan_mati.store(true, Ordering::Relaxed);
    }
    pintu.push(Masuk { ip: ip as u32, kembali, jenis: jenis.iter().map(|j| kode_jenis(*j)).collect(), kode });
    kode
}

/// Status kode native (`ULANG` juga untuk "tidak dijalankan").
fn jalankan(vm: &mut Vm, kembali: u8) -> i64 {
    let Some(fr) = vm.frames.last() else { return ULANG };
    let (func, ip, basis) = (Arc::clone(&fr.func), fr.ip, fr.slot + 1);
    if vm.stack.len() < basis || vm.stack.len() - basis > MAKS_POSISI { return ULANG; }
    let Some(kode) = kode_untuk(vm, &func, ip, kembali, basis) else { return ULANG };
    // Bingkai kecil (hampir semua panggilan) tidak membayar alokasi.
    let isi = &vm.stack[basis..];
    let mut kecil = [0i64; 32];
    let mut besar = Vec::new();
    let args: &mut [i64] = if isi.len() <= kecil.len() { &mut kecil[..isi.len()] } else {
        besar.resize(isi.len(), 0);
        &mut besar
    };
    for (a, v) in args.iter_mut().zip(isi) {
        *a = match v {
            VmValue::Int(n) => *n as i64,
            VmValue::Bool(b) => *b as i64,
            VmValue::Float(f) => f.to_bits() as i64,
            _ => 0,
        };
    }
    // Panggilan native ke-k sama dengan bingkai VM ke-(F+k): yang akan ditolak `periksa_kedalaman`
    // tidak dijalankan native, tapi diserahkan ke VM lewat deopt di instruksi `Call`-nya.
    let batas = match crate::interpreter::stack_menipis() {
        Some(_) => Vm::BINGKAI_MAKS,
        None => crate::interpreter::Interpreter::MAX_CALL_DEPTH,
    };
    let boleh = ((batas + 2).saturating_sub(vm.frames.len()) as i64).min(KEDALAMAN_MAKS);
    let mut keluar = 0i64;
    // Kode ini dibangkitkan `turun.rs` dengan tanda tangan `KodeJit::masuk` persis.
    let pvm: *mut Vm = vm;
    let st = super::lambat::dengan_konteks(pvm, basis, || unsafe {
        (kode.masuk)(args.as_ptr(), (KEDALAMAN_MAKS - boleh) as i32, &mut keluar)
    }) as i64;
    if st == BERHASIL || st == BERHASIL_BOOL || st == BERHASIL_PECAHAN {
        // `Return` native: bingkainya tanpa `defer` (`DeferPush` selalu titik keluar).
        if let Some(fr) = vm.ambil_bingkai() { vm.stack.truncate(fr.slot); }
        vm.stack.push(match st {
            BERHASIL => VmValue::Int(keluar as i128),
            BERHASIL_BOOL => VmValue::Bool(keluar != 0),
            _ => VmValue::Float(f64::from_bits(keluar as u64)),
        });
    } else if st == DEOPT {
        deopt::pulihkan(vm, deopt::ambil());
    }
    // ULANG: belum ada yang dijalankan; VM melanjutkan bingkainya dari `ip` yang sama.
    st
}
