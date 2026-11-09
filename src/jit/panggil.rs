//! Pintu dari `Vm::call_value`: jaga di pintu masuk (semua argumen bulat 64 bit), kompilasi sekali per
//! fungsi, jalankan kode native. Kode yang deopt dilanjutkan VM dari bingkai yang disusun ulang.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::deopt;
use super::turun::{BERHASIL, DEOPT, KEDALAMAN_MAKS};
use crate::vm::value::{VmFunction, VmValue};
use crate::vm::vm::Vm;

/// `true` = panggilan sudah ditangani: hasilnya di puncak stack, atau bingkainya sudah dilanjutkan VM.
pub(crate) fn coba(vm: &mut Vm, func: &Arc<VmFunction>, argc: usize, fn_idx: usize) -> bool {
    if argc != func.arity as usize || argc > 16 { return false; }
    let Some(p) = func.chunk.profil.as_ref() else { return false };
    if p.panggilan.load(Ordering::Relaxed) < super::ambang().max(1) { return false; }
    let kode = *func.chunk.kode_jit.get_or_init(|| {
        let globals = &vm.globals;
        let cari = |nama: &str| match globals.get(nama) {
            Some(VmValue::Function(g)) => Some(Arc::clone(g)),
            _ => None,
        };
        super::modul::kompilasi(func, &cari)
    });
    let Some(kode) = kode else { return false };
    let mut args = [0i64; 16];
    for (i, v) in vm.stack[fn_idx + 1..fn_idx + 1 + argc].iter().enumerate() {
        match v {
            VmValue::Int(n) => match i64::try_from(*n) { Ok(n) => args[i] = n, Err(_) => return false },
            _ => return false,
        }
    }
    // Panggilan native ke-k sama dengan bingkai VM ke-(F+k): yang akan ditolak `periksa_kedalaman`
    // tidak dijalankan native, tapi diserahkan ke VM lewat deopt di instruksi `Call`-nya.
    let batas = match crate::interpreter::stack_menipis() {
        Some(_) => Vm::BINGKAI_MAKS,
        None => crate::interpreter::Interpreter::MAX_CALL_DEPTH,
    };
    let boleh = ((batas + 1).saturating_sub(vm.frames.len()) as i64).min(KEDALAMAN_MAKS);
    let mut keluar = 0i64;
    // Kode ini dibangkitkan `turun.rs` dengan tanda tangan `KodeJit::masuk` persis.
    let st = unsafe {
        (kode.masuk)(args.as_ptr(), (KEDALAMAN_MAKS - boleh) as i32, &mut keluar)
    } as i64;
    if st == BERHASIL {
        vm.stack.truncate(fn_idx);
        vm.stack.push(VmValue::Int(keluar as i128));
        return true;
    }
    if st == DEOPT {
        deopt::pulihkan(vm, fn_idx, deopt::ambil());
        return true;
    }
    // ULANG di bingkai terluar: belum ada yang dijalankan, VM memanggil seperti biasa.
    false
}
