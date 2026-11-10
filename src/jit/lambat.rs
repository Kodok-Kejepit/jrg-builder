//! Jalur lambat J4 (`dokumentasi/jit/j4-rencana.md`): kode native membaca dan menulis nilai milik VM
//! (`Jenis::Nilai`) lewat fungsi di sini, dengan aturan VM yang sama. Gagal (status 1) = belum dikerjakan
//! apa pun yang terlihat, jadi kode native deopt di instruksi itu dan VM yang menjalankannya.

use std::cell::{Cell, RefCell};

use super::layak::Jenis;
use crate::vm::value::VmValue;
use crate::vm::vm::{Vm, VmError};

thread_local! {
    /// VM dan basis bingkai terluar yang sedang dijalankan kode native di thread ini.
    static KONTEKS: Cell<(*mut Vm, usize)> = const { Cell::new((std::ptr::null_mut(), 0)) };
    /// Galat dari instruksi yang dijalankan `langkah`; diambil `panggil::jalankan`.
    static GALAT: RefCell<Option<VmError>> = const { RefCell::new(None) };
}

/// Galat `langkah` terakhir di thread ini (status `GAGAL` kode native).
pub(crate) fn ambil_galat() -> VmError {
    GALAT.with(|g| g.borrow_mut().take()).unwrap_or_else(|| VmError::new("bug di jrgc: galat jalur lambat JIT hilang"))
}

/// VM dan basis bingkai terluar. Kode native berjalan di dalam `panggil::jalankan`, yang tidak
/// menyentuh VM sampai kode itu kembali.
fn konteks() -> (&'static mut Vm, usize) {
    let (vm, basis) = KONTEKS.with(|k| k.get());
    assert!(!vm.is_null(), "bug di jrgc: jalur lambat JIT tanpa VM");
    (unsafe { &mut *vm }, basis)
}

/// Posisi `p` bingkai terluar ada di stack VM (yang lebih pendek diisi `nil`).
fn sediakan(vm: &mut Vm, basis: usize, p: usize) -> &mut VmValue {
    if vm.stack.len() <= basis + p { vm.stack.resize(basis + p + 1, VmValue::Nil); }
    &mut vm.stack[basis + p]
}

fn nilai_skalar(j: Jenis, v: i64) -> Option<VmValue> {
    match j {
        Jenis::Bulat => Some(VmValue::Int(v as i128)),
        Jenis::Bool => Some(VmValue::Bool(v != 0)),
        Jenis::Pecahan => Some(VmValue::Float(f64::from_bits(v as u64))),
        Jenis::Kosong => Some(VmValue::Nil),
        _ => None,
    }
}

/// VM menjalankan instruksi `ip` bingkai terluar. Stack VM dipotong ke `t` posisi dan `ambil` operand
/// teratas diisi dari `nilai` (posisi `Nilai` sudah di sana); sesudahnya tingginya harus `sesudah`.
/// 0 = selesai; 2 = galat (disimpan untuk `ambil_galat`, bingkainya sudah dalam keadaan VM).
pub extern "C" fn langkah(ip: i64, jenis: *const Jenis, t: i64, ambil: i64, sesudah: i64, nilai: *const i64) -> i32 {
    let (vm, basis) = konteks();
    let (t, ambil) = (t as usize, ambil as usize);
    // Tabel jenis milik `Rencana` dan slot nilai bingkai native, masing-masing `t` posisi.
    let (jenis, nilai) = unsafe { (std::slice::from_raw_parts(jenis, t), std::slice::from_raw_parts(nilai, t)) };
    // Yang di atas `t` sudah dibuang kode native: dilepas di sini, sebelum ada yang menghitung pemegangnya.
    vm.stack.truncate(basis + t);
    while vm.stack.len() < basis + t { vm.stack.push(VmValue::Nil); }
    for i in t - ambil..t {
        if let Some(v) = nilai_skalar(jenis[i], nilai[i]) { vm.stack[basis + i] = v; }
    }
    let d = vm.frames.len();
    vm.frames.last_mut().expect("bingkai terluar").ip = ip as usize;
    if let Err(e) = vm.run_until_inner::<true, true>(d - 1) {
        GALAT.with(|g| *g.borrow_mut() = Some(e));
        return 2;
    }
    let ok = vm.frames.len() == d && vm.frames[d - 1].ip == ip as usize + 1 && vm.stack.len() == basis + sesudah as usize;
    assert!(ok, "bug di jrgc: langkah JIT di ip {ip} meninggalkan VM dalam bentuk lain");
    0
}

/// `LoadLocal` atas lokal milik VM: posisi `ke` = isi lokal `dari` (sel dibaca isinya).
pub extern "C" fn muat(dari: i64, ke: i64) {
    let (vm, basis) = konteks();
    let v = vm.baca_lokal(basis + dari as usize).unwrap_or(VmValue::Nil);
    *sediakan(vm, basis, ke as usize) = v;
}

/// `Const`/`Unit`: posisi `ke` = salinan konstanta di `k` (milik chunk yang dikompilasi); null = `()`.
pub extern "C" fn taruh(ke: i64, k: *const VmValue) {
    let (vm, basis) = konteks();
    let v = if k.is_null() { VmValue::Unit } else { unsafe { (*k).clone() } };
    *sediakan(vm, basis, ke as usize) = v;
}

/// `AmbilLokal` atas lokal milik VM (`Vm::ambil_lokal`): posisi `ke` = isinya.
pub extern "C" fn ambil(dari: i64, ke: i64) {
    let (vm, basis) = konteks();
    let v = vm.ambil_lokal(basis + dari as usize).unwrap_or(VmValue::Nil);
    *sediakan(vm, basis, ke as usize) = v;
}

/// `StoreLocal` (`pindah` 0) / `SimpanBuang` (1): lokal `ke` = nilai posisi `dari`, berjenis `jenis`
/// (kode `panggil::kode_jenis`; 4 = milik VM, dibaca dari stack-nya).
pub extern "C" fn simpan(ke: i64, dari: i64, nilai: i64, jenis: i32, pindah: i32) {
    let (vm, basis) = konteks();
    let v = match jenis {
        1 => VmValue::Int(nilai as i128),
        2 => VmValue::Bool(nilai != 0),
        3 => VmValue::Nil,
        5 => VmValue::Float(f64::from_bits(nilai as u64)),
        _ => {
            let t = sediakan(vm, basis, dari as usize);
            if pindah != 0 { std::mem::replace(t, VmValue::Nil) } else { t.clone() }
        }
    };
    vm.simpan_lokal(basis + ke as usize, v);
}

/// Posisi `p` dilepas (`Pop` atas nilai milik VM).
pub extern "C" fn buang(p: i64) {
    let (vm, basis) = konteks();
    if let Some(v) = vm.stack.get_mut(basis + p as usize) { *v = VmValue::Nil; }
}

/// Truthiness posisi `p` (`VmValue::is_truthy`); `lepas` = posisinya dibuang sesudahnya.
pub extern "C" fn benar(p: i64, lepas: i32) -> i32 {
    let (vm, basis) = konteks();
    let Some(v) = vm.stack.get_mut(basis + p as usize) else { return 0 };
    let b = v.is_truthy();
    if lepas != 0 { *v = VmValue::Nil; }
    b as i32
}

/// Posisi `p` sebagai bulat 64 bit (`jenis` 1) atau pecahan (5) di `*keluar`; 1 = jenisnya lain.
pub extern "C" fn buka(p: i64, jenis: i32, keluar: *mut i64) -> i32 {
    let (vm, basis) = konteks();
    let v = match (vm.stack.get(basis + p as usize), jenis) {
        (Some(VmValue::Int(n)), 1) => i64::try_from(*n).ok(),
        (Some(VmValue::Float(x)), 5) => Some(x.to_bits() as i64),
        _ => None,
    };
    match v {
        Some(n) => { unsafe { *keluar = n }; 0 }
        None => 1,
    }
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
