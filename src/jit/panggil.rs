//! Pintu dari `Vm::call_value`: jaga di pintu masuk (semua argumen bulat 64 bit), kompilasi sekali per
//! fungsi, jalankan kode native; MUNDUR = `false`, dan VM menjalankan panggilan itu seperti biasa.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::turun::{BERHASIL, TERLALU_DALAM};
use crate::vm::value::{VmFunction, VmValue};
use crate::vm::vm::Vm;

/// `true` = panggilan sudah selesai di kode native dan hasilnya ada di puncak stack.
pub(crate) fn coba(vm: &mut Vm, func: &Arc<VmFunction>, argc: usize, fn_idx: usize) -> bool {
    if argc != func.arity as usize || argc > 16 { return false; }
    // Sesudah satu rantai rekursi terlalu dalam, bingkai VM di atasnya tidak mencoba native lagi.
    if vm.frames.len() >= vm.jit_dilarang { return false; }
    if vm.jit_dilarang != usize::MAX { vm.jit_dilarang = usize::MAX; }
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
    let mut keluar = 0i64;
    // Kode ini dibangkitkan `turun.rs` dengan tanda tangan `KodeJit::masuk` persis.
    let st = unsafe { (kode.masuk)(args.as_ptr(), 0, &mut keluar) } as i64;
    if st == BERHASIL {
        vm.stack.truncate(fn_idx);
        vm.stack.push(VmValue::Int(keluar as i128));
        return true;
    }
    if st == TERLALU_DALAM { vm.jit_dilarang = vm.frames.len() + 1; }
    false
}
