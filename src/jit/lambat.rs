//! Jalur lambat J4 (`dokumentasi/jit/j4-rencana.md`): kode native membaca dan menulis nilai milik VM
//! (`Jenis::Nilai`) lewat fungsi di sini, dengan aturan VM yang sama. Gagal (status 1) = belum dikerjakan
//! apa pun yang terlihat, jadi kode native deopt di instruksi itu dan VM yang menjalankannya.

use std::cell::Cell;

use crate::vm::value::VmValue;
use crate::vm::vm::Vm;

thread_local! {
    /// VM dan basis bingkai terluar yang sedang dijalankan kode native di thread ini.
    static KONTEKS: Cell<(*mut Vm, usize)> = const { Cell::new((std::ptr::null_mut(), 0)) };
}

/// Pasang konteks selama `f` (kode native) berjalan; konteks sebelumnya dipulihkan.
pub(crate) fn dengan_konteks<T>(vm: *mut Vm, basis: usize, f: impl FnOnce() -> T) -> T {
    let lama = KONTEKS.with(|k| k.replace((vm, basis)));
    let h = f();
    KONTEKS.with(|k| k.set(lama));
    h
}

/// Slot lokal `rel` bingkai terluar sebagai larik (bukan sel tangkapan): `f` atas isinya.
fn atas_larik<T>(rel: i64, f: impl FnOnce(&mut VmValue) -> Option<T>) -> Option<T> {
    let (vm, basis) = KONTEKS.with(|k| k.get());
    if vm.is_null() { return None; }
    // Kode native berjalan di dalam `panggil::jalankan`, yang tidak menyentuh VM sampai kode itu kembali.
    let vm = unsafe { &mut *vm };
    let tempat = vm.stack.get_mut(basis.checked_add(usize::try_from(rel).ok()?)?)?;
    if !matches!(tempat, VmValue::Array(_)) { return None; }
    f(tempat)
}

/// Indeks dengan aturan `norm_idx` (negatif dari belakang); di luar batas = `None` (VM yang menolak).
fn indeks(i: i64, n: usize) -> Option<usize> {
    let i = if i < 0 { n as i128 + i as i128 } else { i as i128 };
    (0..n as i128).contains(&i).then_some(i as usize)
}

/// `IndeksLokal` atas larik berelemen bulat 64 bit: 0 = `*keluar` terisi.
pub extern "C" fn baca(rel: i64, i: i64, keluar: *mut i64) -> i32 {
    let v = atas_larik(rel, |t| {
        let VmValue::Array(a) = t else { return None };
        let g = a.lock().unwrap_or_else(|p| p.into_inner());
        match g.get(indeks(i, g.len())?)? {
            VmValue::Int(n) => i64::try_from(*n).ok(),
            _ => None,
        }
    });
    match v {
        Some(n) => { unsafe { *keluar = n }; 0 }
        None => 1,
    }
}

/// `TulisIndexLokal`: elemen ke-`i` = nilai berjenis `jenis` (1 bulat, 2 bool, 5 pecahan); 0 = tertulis.
pub extern "C" fn tulis(rel: i64, i: i64, nilai: i64, jenis: i32) -> i32 {
    let v = match jenis {
        1 => VmValue::Int(nilai as i128),
        2 => VmValue::Bool(nilai != 0),
        5 => VmValue::Float(f64::from_bits(nilai as u64)),
        _ => return 1,
    };
    let ok = atas_larik(rel, |t| {
        // Aturan `PinjamLokal`: isi dipisahkan dari pemegang lain sebelum ditulis (tidak terlihat bila gagal).
        t.pisahkan();
        let VmValue::Array(a) = t else { return None };
        let mut g = a.lock().unwrap_or_else(|p| p.into_inner());
        let k = indeks(i, g.len())?;
        g[k] = v;
        Some(())
    });
    if ok.is_some() { 0 } else { 1 }
}
