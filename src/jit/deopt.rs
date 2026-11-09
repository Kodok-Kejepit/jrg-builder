//! Deopt penuh J3 (`dokumentasi/jit/j3-rencana.md`): kode native yang jaganya gagal mencatat keadaan
//! bingkainya (dan bingkai native pemanggilnya), lalu VM menyusun ulang bingkai-bingkai itu dan
//! melanjutkan dari instruksi yang sama — tidak ada yang diulang dari awal.

use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::layak::Jenis;
use crate::vm::value::VmValue;
use crate::vm::vm::{CallFrame, Vm};

/// Satu bingkai native yang keluar: fungsi ke-`fid`, lanjut di `ip`, nilai stack-nya dari dasar.
pub struct Rekaman {
    pub fid: u32,
    pub ip: u32,
    pub nilai: Vec<i64>,
}

/// Rekaman satu panggilan native, paling dalam lebih dulu.
pub type Catatan = Vec<Rekaman>;

static JUMLAH: AtomicU64 = AtomicU64::new(0);

/// Berapa kali kode native keluar lewat deopt, untuk `JRG_JIT_LOG`.
pub fn jumlah() -> u64 { JUMLAH.load(Ordering::Relaxed) }

/// Dipanggil kode native di blok deopt (jalur dingin).
pub extern "C" fn catat(c: *mut Catatan, fid: i32, ip: i32, nilai: *const i64, n: i32) {
    // `c` penyangga milik `panggil::coba` yang hidup selama panggilan native; `nilai` slot stack sebanyak `n`.
    let (c, nilai) = unsafe { (&mut *c, std::slice::from_raw_parts(nilai, n as usize)) };
    c.push(Rekaman { fid: fid as u32, ip: ip as u32, nilai: nilai.to_vec() });
}

/// `JRG_JIT_STRES=deopt-acak`: tiap instruksi yang dikompilasi bisa keluar ke VM (07-kebenaran-dan-uji.md).
pub fn acak_aktif() -> bool {
    static A: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *A.get_or_init(|| std::env::var("JRG_JIT_STRES").map(|s| s.split(',').any(|m| m == "deopt-acak")).unwrap_or(false))
}

/// Undian deterministik per thread (xorshift, benih tetap): 1 kira-kira sekali tiap 16 instruksi.
pub extern "C" fn undi() -> i32 {
    thread_local! { static X: Cell<u64> = const { Cell::new(0x9E37_79B9_7F4A_7C15) }; }
    X.with(|x| {
        let mut v = x.get();
        v ^= v << 13; v ^= v >> 7; v ^= v << 17;
        x.set(v);
        (v >> 60 == 0) as i32
    })
}

/// Susun ulang bingkai VM dari rekaman; bingkai terluar adalah fungsi di `fn_idx` yang dipanggil `coba`.
pub(crate) fn pulihkan(vm: &mut Vm, fn_idx: usize, catatan: Catatan) {
    JUMLAH.fetch_add(1, Ordering::Relaxed);
    vm.stack.truncate(fn_idx + 1);
    for (k, r) in catatan.into_iter().rev().enumerate() {
        let (f, rencana) = super::modul::info(r.fid);
        let slot = if k == 0 { fn_idx } else {
            vm.stack.push(VmValue::Function(Arc::clone(&f)));
            vm.stack.len() - 1
        };
        let jenis = rencana.keadaan[r.ip as usize].as_ref().expect("ip lanjut selalu terjangkau");
        for (i, v) in r.nilai.into_iter().enumerate() {
            vm.stack.push(match jenis[i] {
                Jenis::Bulat => VmValue::Int(v as i128),
                Jenis::Bool => VmValue::Bool(v != 0),
                Jenis::Fungsi(c) => VmValue::Function(Arc::clone(&rencana.callee[c])),
            });
        }
        let argc = f.arity as u8;
        vm.dorong_bingkai(CallFrame { func: f, ip: r.ip as usize, slot, defers: Vec::new(), argc, closure: None });
    }
}
