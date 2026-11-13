//! Jalur lambat J4 (`dokumentasi/jit/j4-rencana.md`): kode native membaca dan menulis nilai milik VM
//! (`Jenis::Nilai`) lewat fungsi di sini, dengan aturan VM yang sama. Gagal (status 1) = belum dikerjakan
//! apa pun yang terlihat, jadi kode native deopt di instruksi itu dan VM yang menjalankannya.

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use super::layak::Jenis;
use crate::vm::opcode::OpCode;
use crate::vm::value::VmValue;
use crate::vm::vm::{Vm, VmError};

thread_local! {
    /// VM dan basis bingkai terluar yang sedang dijalankan kode native di thread ini.
    static KONTEKS: Cell<(*mut Vm, usize)> = const { Cell::new((std::ptr::null_mut(), 0)) };
    /// Panggilan langsung yang sedang berjalan (`masuk`/`keluar`): kedalaman bingkai pemanggil, basisnya,
    /// dan posisi mutlak slot fungsi callee.
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

/// Posisi `p` bingkai terluar ada di stack VM (yang lebih pendek diisi `nil`). Dipanggil hampir tiap
/// pembantu: di-inline, yang jarang (stack lebih pendek dari puncak) di fungsi terpisah.
#[inline(always)]
fn sediakan(vm: &mut Vm, basis: usize, p: usize) -> &mut VmValue {
    let i = basis + p;
    // Posisi baru di puncak (sering): satu `push`; lebih tinggi dari itu (jarang): `resize` terpisah.
    if vm.stack.len() == i { vm.stack.push(VmValue::Nil); } else if vm.stack.len() < i { tumbuh(vm, i); }
    &mut vm.stack[i]
}

/// Celahnya biasanya satu-dua posisi (argumen native callee ringan yang tidak ditulis `masuk_ringan`):
/// `push` berulang, bukan `resize` (yang menyalin `VmValue::Nil` lewat `clone` umum, ±65 instruksi).
#[cold]
#[inline(never)]
fn tumbuh(vm: &mut Vm, i: usize) {
    vm.stack.reserve(i + 1 - vm.stack.len());
    while vm.stack.len() <= i { vm.stack.push(VmValue::Nil); }
}

fn nilai_skalar(j: Jenis, v: i64) -> Option<VmValue> {
    if j == Jenis::Lebar { return None; }
    skalar_kode(j.kode() as i32, v)
}

/// Satu definisi isi native → nilai VM menurut `Jenis::kode`; `None` = milik VM (4) atau `Lebar` (6).
pub(crate) fn skalar_kode(kode: i32, nilai: i64) -> Option<VmValue> {
    Some(match kode {
        1 => VmValue::Int(nilai as i128),
        2 => VmValue::Bool(nilai != 0),
        3 => VmValue::Nil,
        5 => VmValue::Float(f64::from_bits(nilai as u64)),
        // Kode rune selalu lahir dari `char` (`karakter`, `buka` kode 7).
        7 => VmValue::Rune(char::from_u32(nilai as u32).expect("kode rune dari char")),
        _ => return None,
    })
}

/// Satu situs `Cara::Bangun`, disiapkan saat kompilasi: struct (label + bentuk prototipe), larik, atau tuple.
pub enum SitusBangun {
    Struktur(crate::vm::value::Label, Arc<crate::vm::peta::Bentuk>),
    Larik,
    Tuple,
    /// `BuildEnum`: tipe dan varian milik prototipenya, payloadnya satu posisi.
    Varian(Arc<str>, Arc<str>),
}

impl SitusBangun {
    /// Hidup selama kode yang memakainya (kode JIT tidak pernah dibebaskan).
    pub fn bocor(s: SitusBangun) -> &'static SitusBangun { Box::leak(Box::new(s)) }
}

/// Literal atas posisi `t - n .. t` (angka native dari tumpahan `nilai`, sisanya dipindah dari stack VM);
/// hasilnya di posisi `t - n`, dibangun lewat definisi yang sama dengan VM.
pub extern "C" fn bangun(situs: *const SitusBangun, jenis: *const Jenis, t: i64, n: i64, nilai: *const i64) {
    let (vm, basis) = konteks();
    let (t, n) = (t as usize, n as usize);
    let jenis = unsafe { std::slice::from_raw_parts(jenis, t) };
    let nilai = unsafe { std::slice::from_raw_parts(nilai, t) };
    let mut isi = Vec::with_capacity(n);
    for i in t - n..t {
        isi.push(match nilai_skalar(jenis[i], nilai[i]) {
            Some(v) => v,
            None => std::mem::replace(sediakan(vm, basis, i), VmValue::Nil),
        });
    }
    let v = match unsafe { &*situs } {
        SitusBangun::Struktur(label, bentuk) =>
            VmValue::Map(crate::vm::value::Peta::struktur_arc(label.clone(), Arc::clone(bentuk), isi)),
        SitusBangun::Tuple => VmValue::tuple(isi),
        SitusBangun::Larik => VmValue::array(isi),
        SitusBangun::Varian(tipe, varian) =>
            VmValue::varian_berisi(Arc::clone(tipe), Arc::clone(varian), isi.pop().expect("satu payload")),
    };
    *sediakan(vm, basis, t - n) = v;
}

/// `Typeof; Const; Eq` atas posisi `p` milik VM: nama jenisnya (`VmValue::type_name`, sama dengan opcode
/// `Typeof`) sama dengan teks konstanta `nama`. Posisinya dilepas.
pub extern "C" fn jenis_sama(p: i64, nama: *const VmValue) -> i32 {
    let (vm, basis) = konteks();
    let v = std::mem::replace(sediakan(vm, basis, p as usize), VmValue::Nil);
    matches!(unsafe { &*nama }, VmValue::Str(n) if v.type_name() == n.as_str()) as i32
}

/// `Len` atas posisi `p` milik VM (`Vm::vm_len`): panjangnya di `*keluar`, posisinya dilepas; 1 = galat,
/// belum ada yang berubah (VM yang melaporkannya).
pub extern "C" fn panjang(p: i64, keluar: *mut i64) -> i32 {
    let (vm, basis) = konteks();
    let i = basis + p as usize;
    let Some(v) = vm.stack.get(i) else { return 1 };
    match vm.vm_len(v, None) {
        Ok(n) => { unsafe { *keluar = n as i64 }; vm.stack[i] = VmValue::Nil; 0 }
        Err(_) => 1,
    }
}

/// `str::char_at(teks, i)` atas posisi `p` milik VM: kode karakternya; −1 = jalur bawaan umum (indeks negatif
/// atau di luar batas, bukan teks — yang juga melapor galatnya). Prefiks ASCII lewat indeks byte.
pub extern "C" fn karakter(p: i64, i: i64) -> i64 {
    if i < 0 { return -1; }
    let (vm, basis) = konteks();
    let Some(VmValue::Str(s)) = vm.stack.get(basis + p as usize) else { return -1 };
    let (b, i) = (s.as_bytes(), i as usize);
    if i < b.len() && b[..=i].is_ascii() { return b[i] as i64; }
    s.chars().nth(i).map_or(-1, |c| c as i64)
}

/// `karakter` atas teks di lokal `slot` (salinan maya, `Jenis::Salinan`), dibaca di tempat.
pub extern "C" fn karakter_lokal(slot: i64, i: i64) -> i64 {
    if i < 0 { return -1; }
    lihat_lokal(slot, |v| {
        let VmValue::Str(s) = v else { return -1 };
        let (b, i) = (s.as_bytes(), i as usize);
        if i < b.len() && b[..=i].is_ascii() { return b[i] as i64; }
        s.chars().nth(i).map_or(-1, |c| c as i64)
    }).unwrap_or(-1)
}

/// VM menjalankan instruksi `ip` bingkai terluar. Stack VM dipotong ke `t` posisi dan `tumpah` posisi
/// teratas diisi dari `nilai` (posisi `Nilai` sudah di sana). `Some(sesudah)`: tinggi yang diharapkan,
/// juga untuk melengkapi `ReturnNil` bersarang. Hasilnya kedalaman bingkai; galat disimpan (`ambil_galat`).
fn satu(ip: i64, jenis: *const Jenis, t: i64, tumpah: i64, sesudah: Option<usize>, nilai: *const i64) -> Option<usize> {
    let (vm, basis) = konteks();
    let (t, ambil) = (t as usize, tumpah as usize);
    // Tabel jenis milik `Rencana` dan slot nilai bingkai native, masing-masing `t` posisi.
    let (jenis, nilai) = unsafe { (std::slice::from_raw_parts(jenis, t), std::slice::from_raw_parts(nilai, t)) };
    // Yang di atas `t` sudah dibuang kode native: dilepas di sini, sebelum ada yang menghitung pemegangnya.
    vm.stack.truncate(basis + t);
    while vm.stack.len() < basis + t { vm.stack.push(VmValue::Nil); }
    for i in t - ambil..t {
        if let Some(v) = nilai_skalar(jenis[i], nilai[i]) { vm.stack[basis + i] = v; }
    }
    let d = vm.frames.len();
    // Opcode yang badannya fungsi `Vm` sendiri dijalankan langsung, tanpa masuk loop dispatch.
    let fr = vm.frames.last_mut().expect("bingkai terluar");
    let ins = fr.func.chunk.code[ip as usize];
    let sp = if ins.span == 0 { None } else { fr.func.chunk.spans.get(ins.span as usize).copied() };
    fr.ip = ip as usize + 1;
    let pertama = match ins.op {
        OpCode::Call => vm.call_value_at(ins.arg as usize, sp, true),
        OpCode::GetMethod => vm.op_get_method(ins.arg, sp),
        OpCode::PeriksaArgMetode => vm.op_periksa_arg_metode(ins.arg, sp),
        OpCode::BuildStruct => vm.op_build_struct(ins.arg, sp),
        OpCode::BuildArray | OpCode::BuildTuple => { vm.op_build_larik(ins.arg, ins.op == OpCode::BuildTuple); Ok(()) }
        OpCode::SetIndex => vm.op_set_index(sp),
        OpCode::Typeof => vm.op_typeof(sp),
        OpCode::Len => vm.op_len(sp),
        OpCode::CopyVal => vm.op_copy_val(sp),
        OpCode::Dup => { let v = vm.stack[vm.stack.len() - 1].clone(); vm.stack.push(v); Ok(()) }
        OpCode::Swap => { let n = vm.stack.len(); vm.stack.swap(n - 1, n - 2); Ok(()) }
        _ => {
            vm.frames[d - 1].ip = ip as usize;
            vm.run_until_inner::<true, true>(d - 1)
        }
    };
    // `Call` atas fungsi VM mendorong bingkai: badannya dijalankan sampai kembali ke bingkai ini.
    // `ReturnNil` yang kembali tepat ke kedalaman itu tidak mendorong apa pun: jawabannya `unit`
    // (aturan `Vm::call_value_immediate`).
    let hasil = pertama.and_then(|_| {
        if vm.frames.len() <= d { return Ok(()); }
        vm.run_until_inner::<true, false>(d)?;
        if vm.frames.len() == d && sesudah.is_some_and(|n| vm.stack.len() + 1 == basis + n) { vm.stack.push(VmValue::Unit); }
        Ok(())
    });
    if let Err(e) = hasil {
        GALAT.with(|g| *g.borrow_mut() = Some(e));
        return None;
    }
    let ok = vm.frames.len() == d && sesudah.is_none_or(|n| vm.frames[d - 1].ip == ip as usize + 1 && vm.stack.len() == basis + n);
    assert!(ok, "bug di jrgc: langkah JIT {:?} di ip {ip} meninggalkan VM dalam bentuk lain (bingkai {}/{d}, ip {}, stack {}/{:?})",
        vm.frames.last().map(|f| f.func.chunk.code[ip as usize].op), vm.frames.len(), vm.frames.last().map_or(0, |f| f.ip),
        vm.stack.len(), sesudah.map(|n| basis + n));
    Some(d)
}

/// Satu instruksi yang tidak melompat; sesudahnya tingginya harus `sesudah`. 0 = selesai, 2 = galat.
pub extern "C" fn langkah(ip: i64, jenis: *const Jenis, t: i64, tumpah: i64, sesudah: i64, nilai: *const i64) -> i32 {
    if satu(ip, jenis, t, tumpah, Some(sesudah as usize), nilai).is_some() { 0 } else { 2 }
}

/// Satu instruksi yang bercabang (`Cara::Cabang`): `ambil_bool` = `bool` di puncak diambil dan
/// dikembalikan (0/1), selain itu `ip` berikutnya; -1 = galat.
pub extern "C" fn langkah_cabang(ip: i64, jenis: *const Jenis, t: i64, tumpah: i64, nilai: *const i64, ambil_bool: i64) -> i64 {
    let Some(d) = satu(ip, jenis, t, tumpah, None, nilai) else { return -1 };
    let (vm, _) = konteks();
    if ambil_bool == 0 { return vm.frames[d - 1].ip as i64; }
    match vm.stack.pop() {
        Some(VmValue::Bool(b)) => b as i64,
        lain => panic!("bug di jrgc: langkah cabang JIT di ip {ip} menaruh {lain:?}, bukan bool"),
    }
}

/// Panggilan langsung (`Cara::Langsung`): callee dan `argc` argumennya (posisi `t - argc - 1..t`) ke stack
/// VM, bingkai callee didorong dengan aturan `Vm::call_value`, dan basis konteks jadi milik callee.
/// 0 = jalankan kode native callee; 1 = VM yang menjalankannya; 2 = galat (`ambil_galat`).
pub extern "C" fn masuk(
    ip: i64, jenis: *const Jenis, t: i64, argc: i64, nilai: *const i64, fungsi: *const VmValue, titip: *mut [usize; 3],
) -> i32 {
    let (vm, basis) = konteks();
    let (t, argc) = (t as usize, argc as usize);
    let jenis = unsafe { std::slice::from_raw_parts(jenis, t) };
    let nilai = unsafe { std::slice::from_raw_parts(nilai, t) };
    vm.stack.truncate(basis + t);
    while vm.stack.len() < basis + t { vm.stack.push(VmValue::Nil); }
    for i in t - argc..t {
        if let Some(v) = nilai_skalar(jenis[i], nilai[i]) { vm.stack[basis + i] = v; }
    }
    let d = vm.frames.len();
    let fr = vm.frames.last_mut().expect("bingkai terluar");
    fr.ip = ip as usize + 1;
    let slot = basis + t - argc - 1;
    // Callee maya (`Jenis::Fungsi`): dari rencana, posisinya tidak ditulis; selain itu dari stack VM.
    let callee = match unsafe { fungsi.as_ref() } { Some(f) => f.clone(), None => vm.stack[slot].clone() };
    let VmValue::Function(func) = callee else { panic!("bug di jrgc: panggilan langsung JIT bukan atas fungsi") };
    if vm.periksa_kedalaman(&func.name, None).is_err() {
        // Span cuma untuk kalimat galatnya.
        let fr = vm.frames.last().expect("bingkai terluar");
        let sp = fr.func.chunk.code.get(ip as usize).and_then(|i| (i.span != 0).then(|| fr.func.chunk.spans.get(i.span as usize).copied()).flatten());
        let e = vm.periksa_kedalaman(&func.name, sp).expect_err("diperiksa barusan");
        GALAT.with(|g| *g.borrow_mut() = Some(e));
        return 2;
    }
    if let Some(p) = &func.chunk.profil { p.catat_panggilan(); }
    vm.dorong_bingkai(crate::vm::vm::CallFrame { func, ip: 0, slot, defers: Vec::new(), argc: argc as u8, closure: None });
    // `(kedalaman, basis, slot)` dititipkan di bingkai native pemanggil sampai `keluar` (tidak bertumpuk di sana).
    unsafe { *titip = [d, basis, slot] };
    // Rekursi native sedalam itu diteruskan VM (lihat `panggil::jalankan`).
    let ukuran = crate::interpreter::Interpreter::INTERPRETER_STACK_SIZE;
    if crate::interpreter::stack_terpakai().is_some_and(|t| t > ukuran / 2) { return 1; }
    KONTEKS.with(|k| k.set((vm, slot + 1)));
    0
}

/// `masuk` untuk callee `layak::ringan`: basis konteks digeser, bingkai VM-nya belum didorong — argumen native
/// dibaca callee dari penyangganya, jadi baru ditulis ke stack VM kalau bingkainya diwujudkan. `titip` =
/// (kedalaman, basis, slot, fungsi, argc, jenis, nilai, t, ip) untuk `keluar_ringan`.
pub extern "C" fn masuk_ringan(
    ip: i64, jenis: *const Jenis, t: i64, argc: i64, nilai: *const i64, fungsi: *const VmValue, titip: *mut [usize; 9],
) -> i32 {
    let (vm, basis) = konteks();
    let (t, argc) = (t as usize, argc as usize);
    // Posisi argumen native cukup `nil` (isinya ditulis `keluar_ringan` kalau bingkainya diwujudkan), supaya
    // tulisan pertama callee di atasnya lewat jalur `push` `sediakan`, bukan `tumbuh`.
    vm.stack.truncate(basis + t);
    while vm.stack.len() < basis + t { vm.stack.push(VmValue::Nil); }
    let slot = basis + t - argc - 1;
    if let VmValue::Function(func) = unsafe { &*fungsi } {
        if let Some(p) = &func.chunk.profil { p.catat_panggilan(); }
    }
    let d = vm.frames.len();
    unsafe { *titip = [d, basis, slot, fungsi as usize, argc, jenis as usize, nilai as usize, t, ip as usize] };
    KONTEKS.with(|k| k.set((vm, slot + 1)));
    0
}

/// `keluar` untuk `masuk_ringan`. Hasil biasa: tanpa bingkai yang perlu dilepas. Deopt, belum mulai, atau
/// galat: bingkai callee diwujudkan dulu persis seperti `masuk`, lalu jalur `keluar` biasa.
pub extern "C" fn keluar_ringan(st: i64, hasil: i64, titip: *const [usize; 9]) -> i32 {
    use super::turun::{BERHASIL, BERHASIL_BOOL, BERHASIL_NILAI, BERHASIL_PECAHAN, BERHASIL_UNIT};
    let (vm, _) = konteks();
    let [_, basis, slot, fungsi, argc, jenis, nilai, t, ip] = unsafe { *titip };
    let v = match st {
        BERHASIL => VmValue::Int(hasil as i128),
        BERHASIL_BOOL => VmValue::Bool(hasil != 0),
        BERHASIL_PECAHAN => VmValue::Float(f64::from_bits(hasil as u64)),
        BERHASIL_NILAI => std::mem::replace(&mut vm.stack[slot + 1 + hasil as usize], VmValue::Nil),
        BERHASIL_UNIT => VmValue::Unit,
        _ => {
            // Yang `masuk` biasa kerjakan: stack sampai posisi pemanggil (`Return` VM membaca panjangnya), `ip`
            // pemanggil sesudah panggilan, argumen native di stack VM. Posisi callee di atasnya tidak dipotong.
            while vm.stack.len() < basis + t { vm.stack.push(VmValue::Nil); }
            vm.frames.last_mut().expect("bingkai terluar").ip = ip + 1;
            let (jenis, nilai) = unsafe {
                (std::slice::from_raw_parts(jenis as *const Jenis, t), std::slice::from_raw_parts(nilai as *const i64, t))
            };
            for i in t - argc..t {
                if let Some(v) = nilai_skalar(jenis[i], nilai[i]) { *sediakan(vm, basis, i) = v; }
            }
            let VmValue::Function(func) = (unsafe { &*(fungsi as *const VmValue) }) else { unreachable!("dijamin turun.rs") };
            vm.dorong_bingkai(crate::vm::vm::CallFrame {
                func: Arc::clone(func), ip: 0, slot, defers: Vec::new(), argc: argc as u8, closure: None,
            });
            return keluar(st, hasil, titip as *const [usize; 3]);
        }
    };
    KONTEKS.with(|k| k.set((vm, basis)));
    // Hasilnya tepat di slot callee, juga kalau stack belum sepanjang itu (`masuk_ringan` tidak memanjangkannya).
    vm.stack.truncate(slot);
    while vm.stack.len() < slot { vm.stack.push(VmValue::Nil); }
    vm.catat_hasil(&v);
    vm.stack.push(v);
    0
}

/// Akhir panggilan langsung: status kode callee (`turun::BERHASIL`…; `ULANG` juga untuk "VM yang
/// menjalankan"), basis konteks pemanggil dipulihkan, hasilnya di slot fungsi callee. 0 = selesai; 2 = galat.
pub extern "C" fn keluar(st: i64, hasil: i64, titip: *const [usize; 3]) -> i32 {
    use super::turun::{BERHASIL, BERHASIL_BOOL, BERHASIL_NILAI, BERHASIL_PECAHAN, BERHASIL_UNIT, DEOPT, GAGAL};
    let (vm, _) = konteks();
    let [d, basis, slot] = unsafe { *titip };
    KONTEKS.with(|k| k.set((vm, basis)));
    let v = match st {
        BERHASIL => VmValue::Int(hasil as i128),
        BERHASIL_BOOL => VmValue::Bool(hasil != 0),
        BERHASIL_PECAHAN => VmValue::Float(f64::from_bits(hasil as u64)),
        BERHASIL_NILAI => std::mem::replace(&mut vm.stack[slot + 1 + hasil as usize], VmValue::Nil),
        BERHASIL_UNIT => VmValue::Unit,
        GAGAL => return 2,
        _ => {
            // Deopt di callee: bingkainya disusun ulang lalu VM menyelesaikannya; `ULANG` = belum mulai.
            if st == DEOPT { super::deopt::ambil_spek_baru(); super::deopt::pulihkan(vm, super::deopt::ambil()); }
            if let Err(e) = vm.run_until_inner::<true, false>(d) {
                GALAT.with(|g| *g.borrow_mut() = Some(e));
                return 2;
            }
            // `Return` VM menaruh hasilnya di slot; `ReturnNil` tidak menaruh apa pun (`unit`).
            let v = if vm.stack.len() > slot { vm.stack.pop().unwrap_or(VmValue::Unit) } else { VmValue::Unit };
            vm.stack.truncate(slot);
            vm.stack.push(v);
            // Spekulasi callee gagal: salinan kodenya di pemanggil basi, jadi pemanggil keluar ke VM juga.
            return if st == DEOPT && super::deopt::ambil_spek_baru() { 3 } else { 0 };
        }
    };
    if let Some(fr) = vm.ambil_bingkai() { vm.stack.truncate(fr.slot); }
    vm.catat_hasil(&v);
    vm.stack.push(v);
    0
}

/// `Call` atas fungsi bawaan di posisi `t - argc - 1` (milik VM): argumennya dari nilai native atau
/// posisi VM (diambil), hasilnya di posisi fungsinya (`Vm::panggil_native`). 0 = selesai, 2 = galat.
pub extern "C" fn bawaan(ip: i64, jenis: *const Jenis, t: i64, argc: i64, nilai: *const i64, f: i64) -> i32 {
    let (vm, basis) = konteks();
    let (t, argc) = (t as usize, argc as usize);
    let jenis = unsafe { std::slice::from_raw_parts(jenis, t) };
    let nilai = unsafe { std::slice::from_raw_parts(nilai, t) };
    // Callee-nya fungsi bawaan maya (`Jenis::Bawaan`): alamatnya dari konstanta, bukan dari stack VM.
    let f: crate::vm::value::VmNativeFn = unsafe { std::mem::transmute(f as usize) };
    if let Some(v) = bawaan_cepat(vm, basis, jenis, nilai, t, argc, f as usize) {
        vm.catat_hasil(&v);
        *sediakan(vm, basis, t - argc - 1) = v;
        return 0;
    }
    let mut args = Vec::with_capacity(argc);
    for i in t - argc..t {
        args.push(match nilai_skalar(jenis[i], nilai[i]) {
            Some(v) => v,
            None => std::mem::replace(sediakan(vm, basis, i), VmValue::Nil),
        });
    }
    let fr = vm.frames.last_mut().expect("bingkai terluar");
    let sp = fr.func.chunk.code.get(ip as usize).and_then(|i| (i.span != 0).then(|| fr.func.chunk.spans.get(i.span as usize).copied()).flatten());
    fr.ip = ip as usize + 1;
    match vm.panggil_native(f, args, sp, true) {
        Ok(v) => {
            vm.catat_hasil(&v);
            *sediakan(vm, basis, t - argc - 1) = v;
            0
        }
        Err(e) => {
            GALAT.with(|g| *g.borrow_mut() = Some(e));
            2
        }
    }
}

/// Bawaan yang dijawab di tempat, tanpa larik argumen dan jembatan nilai; `None` = jalur umum (yang juga
/// melapor galatnya). Posisi milik VM saja yang sah dibaca dari stack VM (posisi native bisa basi).
fn bawaan_cepat(vm: &Vm, basis: usize, jenis: &[Jenis], nilai: &[i64], t: usize, argc: usize, f: usize) -> Option<VmValue> {
    fn alamat(nama: &str) -> usize {
        match crate::vm::bawaan::bawaan(nama) { Some(VmValue::NativeFn(f)) => f as usize, _ => 0 }
    }
    static ALAMAT: std::sync::OnceLock<[usize; 5]> = std::sync::OnceLock::new();
    let [char_at, int, uint, teks, len] = *ALAMAT.get_or_init(|| {
        [alamat("str::char_at"), alamat("int"), alamat("uint"), alamat("str"), alamat("len")]
    });
    match argc {
        // `str(n)` atas bulat native: tampilan `VmValue::Int` yang sama dengan bawaan `str`.
        1 if f == teks && jenis[t - 1] == Jenis::Bulat => Some(VmValue::str(VmValue::Int(nilai[t - 1] as i128).to_string())),
        // `len(teks)` milik VM: `Vm::vm_len`, definisi yang sama dengan opcode `Len`.
        1 if f == len && jenis[t - 1].di_vm() => match vm.stack.get(basis + t - 1) {
            Some(v @ VmValue::Str(_)) => vm.vm_len(v, None).ok().map(|n| VmValue::Int(n as i128)),
            _ => None,
        },
        // `str::char_at(s, i)`, `i` tak negatif yang ada: prefiks ASCII (per kata mesin) lewat indeks byte.
        2 if f == char_at && jenis[t - 1] == Jenis::Bulat && nilai[t - 1] >= 0 && jenis[t - 2].di_vm() => {
            let Some(VmValue::Str(s)) = vm.stack.get(basis + t - 2) else { return None };
            let (b, i) = (s.as_bytes(), nilai[t - 1] as usize);
            if i < b.len() && b[..=i].is_ascii() { return Some(VmValue::Rune(b[i] as char)); }
            s.chars().nth(i).map(VmValue::Rune)
        }
        // `rune as int` / `as uint`: kode karakternya (selalu muat).
        1 if (f == int || f == uint) && jenis[t - 1].di_vm() => match vm.stack.get(basis + t - 1) {
            Some(VmValue::Rune(c)) => Some(VmValue::Int(*c as i128)),
            _ => None,
        },
        _ => None,
    }
}

/// Aritmetika/banding instruksi `ip` atas posisi `t - 2`, `t - 1` (angka native atau milik VM, diambil),
/// lewat `Vm::op_biner`. Aritmetika: hasilnya di posisi `t - 2`, 0 = selesai. Banding: 0/1. 2 = galat.
pub extern "C" fn biner(ip: i64, jenis: *const Jenis, t: i64, nilai: *const i64) -> i32 {
    let (vm, basis) = konteks();
    let t = t as usize;
    let jenis = unsafe { std::slice::from_raw_parts(jenis, t) };
    let nilai = unsafe { std::slice::from_raw_parts(nilai, t) };
    let mut ambil = |i: usize| nilai_skalar(jenis[i], nilai[i])
        .unwrap_or_else(|| std::mem::replace(sediakan(vm, basis, i), VmValue::Nil));
    let (a, b) = (ambil(t - 2), ambil(t - 1));
    let fr = vm.frames.last_mut().expect("bingkai terluar");
    let ins = fr.func.chunk.code[ip as usize];
    let sp = if ins.span == 0 { None } else { fr.func.chunk.spans.get(ins.span as usize).copied() };
    fr.ip = ip as usize + 1;
    match vm.op_biner(ins.op, a, b, sp) {
        Ok(v) if matches!(ins.op, OpCode::Eq | OpCode::Ne | OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge) => v.is_truthy() as i32,
        Ok(v) => { *sediakan(vm, basis, t - 2) = v; 0 }
        Err(e) => {
            GALAT.with(|g| *g.borrow_mut() = Some(e));
            2
        }
    }
}

/// `Propagate` (`?`) atas posisi `p`: 0 = `Ok`/`Some`, posisinya jadi payload; 1 = `Err`/`None`, tidak
/// disentuh (fungsinya kembali dengan nilai itu); 2 = bukan `Result`/`Option` (VM yang melapor).
pub extern "C" fn sebar(p: i64) -> i32 {
    let (vm, basis) = konteks();
    let v = sediakan(vm, basis, p as usize);
    match Vm::tagged_parts(v) {
        Some((ty, var, isi)) if ty == "Result" || ty == "Option" => match var.as_str() {
            "Ok" | "Some" => { *v = isi.unwrap_or(VmValue::Nil); 0 }
            "Err" | "None" => 1,
            _ => 2,
        },
        _ => 2,
    }
}

/// `LoadLocal` atas lokal milik VM: posisi `ke` = isi lokal `dari` (sel dibaca isinya).
pub extern "C" fn muat(dari: i64, ke: i64) {
    let (vm, basis) = konteks();
    // Di loop posisinya biasanya masih memegang penyimpanan yang sama dari putaran sebelumnya.
    if let (Some(a), Some(b)) = (vm.stack.get(basis + dari as usize), vm.stack.get(basis + ke as usize)) {
        if sama_penyimpanan(a, b) { return; }
    }
    let v = vm.baca_lokal(basis + dari as usize).unwrap_or(VmValue::Nil);
    *sediakan(vm, basis, ke as usize) = v;
}

/// Dua nilai berbagi penyimpanan yang sama (isinya identik); sel tangkapan tidak termasuk.
fn sama_penyimpanan(a: &VmValue, b: &VmValue) -> bool {
    match (a, b) {
        (VmValue::Str(x), VmValue::Str(y)) => Arc::ptr_eq(x, y),
        (VmValue::Array(x), VmValue::Array(y)) | (VmValue::Tuple(x), VmValue::Tuple(y)) => Arc::ptr_eq(x, y),
        (VmValue::Map(x), VmValue::Map(y)) => Arc::ptr_eq(x, y),
        (VmValue::Function(x), VmValue::Function(y)) => Arc::ptr_eq(x, y),
        _ => false,
    }
}

/// `Const`/`Unit`: posisi `ke` = salinan konstanta di `k` (milik chunk yang dikompilasi); null = `()`.
pub extern "C" fn taruh(ke: i64, k: *const VmValue) {
    let (vm, basis) = konteks();
    // Callee bawaan di loop: posisinya biasanya sudah memegang fungsi yang sama dari putaran sebelumnya.
    if let (Some(VmValue::NativeFn(k)), Some(VmValue::NativeFn(ada))) = (unsafe { k.as_ref() }, vm.stack.get(basis + ke as usize)) {
        if *k as usize == *ada as usize { return; }
    }
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
    let v = match skalar_kode(jenis, nilai) {
        Some(v) => v,
        None => {
            let t = sediakan(vm, basis, dari as usize);
            if pindah != 0 { std::mem::replace(t, VmValue::Nil) } else { t.clone() }
        }
    };
    vm.simpan_lokal(basis + ke as usize, v);
}

/// `CekEnum` atas posisi `p` milik VM dengan pola `desc` (konstanta chunk): 0/1 = hasilnya, posisinya
/// dilepas; 2 = jumlah sub-polanya salah, belum ada yang berubah (VM yang melaporkannya).
pub extern "C" fn pola(p: i64, situs: *const SitusPola) -> i32 {
    let (vm, basis) = konteks();
    let v = sediakan(vm, basis, p as usize);
    match unsafe { &*situs }.cocok(v) {
        Ok(b) => { *v = VmValue::Nil; b as i32 }
        Err(_) => 2,
    }
}

/// Satu situs pola enum (`CekEnum`): deskriptornya dan tembolok hasil per varian. Kunci = alamat nama varian
/// milik nilainya (satu `Arc` cuma dimiliki satu tipe+varian; referensi kuatnya dibocorkan, jadi alamatnya
/// tidak dipakai ulang) | bit payload, isi = hasilnya; `Err` (sub-pola salah jumlah) tidak pernah ditembolok.
pub struct SitusPola {
    desc: *const VmValue,
    // Delapan: situs `CekEnum` melihat semua varian enum-nya, bukan cuma yang dicocokkannya.
    tembolok: [std::sync::atomic::AtomicUsize; 8],
}

impl SitusPola {
    /// Hidup selama kode yang memakainya (kode JIT tidak pernah dibebaskan).
    pub fn bocor(desc: &VmValue) -> &'static SitusPola {
        Box::leak(Box::new(SitusPola { desc, tembolok: Default::default() }))
    }

    #[inline]
    fn cocok(&self, v: &VmValue) -> Result<bool, String> {
        use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
        let kunci = match v {
            VmValue::Map(m) => match &m.label {
                crate::vm::value::Label::Enum { varian, payload, .. } =>
                    Some((Arc::as_ptr(varian) as *const u8 as usize | (payload.is_some() as usize) << 1, varian)),
                _ => None,
            },
            _ => None,
        };
        if let Some((k, _)) = kunci {
            for e in &self.tembolok {
                let isi = e.load(Acquire);
                if isi & !1 == k { return Ok(isi & 1 == 1); }
            }
        }
        let hasil = crate::vm::cocok_enum(v, unsafe { &*self.desc })?;
        if let Some((k, varian)) = kunci {
            if let Some(e) = self.tembolok.iter().find(|e| e.load(Relaxed) == 0) {
                if e.compare_exchange(0, k | hasil as usize, Release, Relaxed).is_ok() {
                    std::mem::forget(Arc::clone(varian));
                }
            }
        }
        Ok(hasil)
    }
}

/// `AmbilPayload`: posisi `p` diganti payload enumnya; 1 = tidak berpayload (VM yang melaporkannya).
pub extern "C" fn payload(p: i64) -> i32 {
    let (vm, basis) = konteks();
    let v = sediakan(vm, basis, p as usize);
    let Some((_, _, Some(isi))) = v.bagian_enum() else { return 1 };
    *v = isi.clone();
    0
}

/// Isi lokal `slot` bingkai terluar dibaca di tempat (sel lewat kuncinya); `None` = di luar stack.
fn lihat_lokal<T>(slot: i64, f: impl FnOnce(&VmValue) -> T) -> Option<T> {
    let (vm, basis) = konteks();
    Some(match vm.stack.get(basis + slot as usize)? {
        VmValue::Cell(c) => f(&c.lock().unwrap_or_else(|p| p.into_inner())),
        v => f(v),
    })
}

/// `LoadLocal` + `CekEnum` tanpa menyalin lokalnya: 0/1 = hasil; 2 = VM yang menjalankan keduanya.
pub extern "C" fn pola_lokal(slot: i64, situs: *const SitusPola) -> i32 {
    lihat_lokal(slot, |v| unsafe { &*situs }.cocok(v).map_or(2, |b| b as i32)).unwrap_or(2)
}

/// `LoadLocal` + `AmbilPayload`: posisi `ke` = payload lokal `slot`; 1 = VM yang menjalankan keduanya.
pub extern "C" fn payload_lokal(slot: i64, ke: i64) -> i32 {
    let Some(Some(isi)) = lihat_lokal(slot, |v| match v.bagian_enum() {
        Some((_, _, Some(isi))) => Some(isi.clone()),
        _ => None,
    }) else { return 1 };
    let (vm, basis) = konteks();
    *sediakan(vm, basis, ke as usize) = isi;
    0
}

/// Satu situs `x.f` yang digabung: nama, privat, jenis spekulasi, dan tembolok bentuk (monomorfik — diisi
/// sekali, memegang referensi kuat ke bentuknya, jadi alamatnya tidak pernah dipakai bentuk lain).
pub struct SitusField {
    pub nama: *const String,
    pub privat: bool,
    pub jenis: i32,
    bentuk: std::sync::atomic::AtomicPtr<crate::vm::peta::Bentuk>,
    indeks: std::sync::atomic::AtomicUsize,
}

impl SitusField {
    /// Hidup selama kode yang memakainya (kode JIT tidak pernah dibebaskan).
    pub fn bocor(nama: &String, privat: bool, jenis: i32) -> &'static SitusField {
        Box::leak(Box::new(SitusField {
            nama, privat, jenis,
            bentuk: std::sync::atomic::AtomicPtr::new(std::ptr::null_mut()),
            indeks: std::sync::atomic::AtomicUsize::new(0),
        }))
    }

    #[inline]
    fn baca<'a>(&self, isi: &'a crate::vm::peta::IsiPeta, nama: &str) -> Option<&'a VmValue> {
        use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
        let crate::vm::peta::IsiPeta::Rapat { bentuk, nilai } = isi else { return isi.get(nama) };
        let p = Arc::as_ptr(bentuk) as *mut crate::vm::peta::Bentuk;
        if self.bentuk.load(Acquire) == p { return nilai.get(self.indeks.load(Relaxed)); }
        let i = bentuk.posisi(nama)?;
        if self.bentuk.load(Relaxed).is_null() {
            self.indeks.store(i, Relaxed);
            let kuat = Arc::into_raw(bentuk.clone()) as *mut crate::vm::peta::Bentuk;
            if self.bentuk.compare_exchange(std::ptr::null_mut(), kuat, Release, Relaxed).is_err() {
                unsafe { drop(Arc::from_raw(kuat)) }
            }
        }
        nilai.get(i)
    }
}

/// `LoadLocal` + `GetField` tanpa menyalin lokalnya. 4 = angka berjenis `situs.jenis` (1 bulat, 5 pecahan)
/// di `keluar`; 0 = nilai di posisi `ke`; 1 = gagal sesudah lokalnya ditaruh di `ke` (VM yang melapor).
pub extern "C" fn field_lokal(slot: i64, situs: *const SitusField, keluar: *mut i64, ke: i64) -> i32 {
    let (vm, basis) = konteks();
    let situs = unsafe { &*situs };
    let (nama, jenis) = (unsafe { &*situs.nama }, situs.jenis);
    // Angka yang cocok dengan spekulasi dibaca di tempat; yang lain disalin.
    let angka = |v: &VmValue| match (v, jenis) {
        (VmValue::Int(n), 1) => i64::try_from(*n).ok(),
        (VmValue::Float(x), 5) => Some(x.to_bits() as i64),
        _ => None,
    };
    // Jalur panas: struct milik lokal (bukan sel), bukan privat, angka yang cocok spekulasi — tanpa `VmValue`.
    if !situs.privat {
        if let Some(VmValue::Map(m)) = vm.stack.get_mut(basis + slot as usize) {
            let n = match crate::vm::value::Peta::isi_tunggal(m) {
                Some(isi) => situs.baca(isi, nama).and_then(angka),
                None => situs.baca(&m.lock().unwrap_or_else(|p| p.into_inner()), nama).and_then(angka),
            };
            if let Some(n) = n {
                unsafe { *keluar = n };
                return 4;
            }
        }
    }
    let cepat: Option<Result<i64, VmValue>> = {
        let vm: &Vm = vm;
        let lihat = |v: &VmValue| match v {
            VmValue::Map(m) if !situs.privat || vm.periksa_visibilitas(v, nama, None).is_ok() =>
                situs.baca(&m.lock().unwrap_or_else(|p| p.into_inner()), nama).map(|x| angka(x).ok_or_else(|| x.clone())),
            _ => None,
        };
        match vm.stack.get(basis + slot as usize) {
            Some(VmValue::Cell(c)) => lihat(&c.lock().unwrap_or_else(|p| p.into_inner())),
            Some(v) => lihat(v),
            None => None,
        }
    };
    let v = match cepat {
        Some(Ok(n)) => { unsafe { *keluar = n }; return 4; }
        Some(Err(v)) => v,
        None => {
            let obj = vm.baca_lokal(basis + slot as usize).unwrap_or(VmValue::Nil);
            *sediakan(vm, basis, ke as usize) = obj;
            if field(ke, nama, situs.privat as i32) != 0 { return 1; }
            let i = basis + ke as usize;
            if let Some(n) = angka(&vm.stack[i]) { unsafe { *keluar = n }; return 4; }
            return 0;
        }
    };
    *sediakan(vm, basis, ke as usize) = v;
    0
}

/// Field angka lain milik lokal `slot` yang dibaca sesudahnya di blok yang sama, sekali kunci (`situs[j]`
/// tidak privat). Bit `j` hasilnya = `keluar[j]` berisi angka berjenis `situs[j].jenis`; sisanya dibaca
/// lewat `field_lokal` biasa di tempatnya.
pub extern "C" fn field_lokal_banyak(slot: i64, situs: *const *const SitusField, n: i64, keluar: *mut i64) -> i64 {
    let (vm, basis) = konteks();
    let Some(VmValue::Map(m)) = vm.stack.get_mut(basis + slot as usize) else { return 0 };
    let kunci;
    let g: &crate::vm::peta::IsiPeta = match crate::vm::value::Peta::isi_tunggal(m) {
        Some(isi) => isi,
        None => { kunci = m.lock().unwrap_or_else(|p| p.into_inner()); &kunci }
    };
    let mut bit = 0i64;
    for j in 0..n as usize {
        let s = unsafe { &**situs.add(j) };
        let n = s.baca(g, unsafe { &*s.nama }).and_then(|v| match (v, s.jenis) {
            (VmValue::Int(n), 1) => i64::try_from(*n).ok(),
            (VmValue::Float(x), 5) => Some(x.to_bits() as i64),
            _ => None,
        });
        if let Some(n) = n { unsafe { *keluar.add(j) = n }; bit |= 1 << j; }
    }
    bit
}

/// `LoadLocal` yang langsung dibuka (`buka`) tanpa menyalin lokalnya ke stack VM.
pub extern "C" fn buka_lokal(slot: i64, jenis: i32, keluar: *mut i64) -> i32 {
    let n = lihat_lokal(slot, |v| match (v, jenis) {
        (VmValue::Int(n), 1) => i64::try_from(*n).ok(),
        (VmValue::Float(x), 5) => Some(x.to_bits() as i64),
        _ => None,
    }).flatten();
    match n {
        Some(n) => { unsafe { *keluar = n }; 0 }
        None => 1,
    }
}

/// `IterAda` atas iterator di posisi `slot` (tiga posisi milik VM): 0/1 = masih ada; 2 = VM yang melapor.
pub extern "C" fn iter_ada(slot: i64) -> i32 {
    let (vm, basis) = konteks();
    vm.iter_ada(basis + slot as usize, None).map_or(2, |b| b as i32)
}

/// `IterMaju`: indeks iterator di posisi `slot` (bulat milik VM) maju satu; 1 = bukan bulat (VM yang melapor).
pub extern "C" fn maju(slot: i64) -> i32 {
    let (vm, basis) = konteks();
    match vm.stack.get_mut(basis + slot as usize) {
        Some(VmValue::Int(n)) if *n < i128::MAX => { *n += 1; 0 }
        _ => 1,
    }
}

/// `SlideDown`: posisi `dari` (milik VM) pindah ke `ke`; yang tertinggal di `dari` jadi `nil`.
pub extern "C" fn pindah(dari: i64, ke: i64) {
    let (vm, basis) = konteks();
    let v = std::mem::replace(sediakan(vm, basis, dari as usize), VmValue::Nil);
    *sediakan(vm, basis, ke as usize) = v;
}

/// `LoadGlobal`: posisi `ke` = global `nama` (isi sel `var` modul); 1 = tidak ada (VM yang melapor).
pub extern "C" fn muat_global(nama: *const String, ke: i64) -> i32 {
    let (vm, basis) = konteks();
    let v = match vm.globals.get(unsafe { &*nama }) {
        Some(VmValue::Cell(sel)) => sel.lock().unwrap_or_else(|p| p.into_inner()).clone(),
        Some(v) => v.clone(),
        None => return 1,
    };
    *sediakan(vm, basis, ke as usize) = v;
    0
}

/// `LoadUpval`: posisi `ke` = isi sel tangkapan ke-`idx` closure bingkai terluar; 1 = tidak ada (VM yang melapor).
pub extern "C" fn muat_upval(idx: i64, ke: i64) -> i32 {
    let (vm, basis) = konteks();
    let Ok(sel) = vm.upval_at(idx as u32, None) else { return 1 };
    let v = sel.lock().unwrap_or_else(|p| p.into_inner()).clone();
    *sediakan(vm, basis, ke as usize) = v;
    0
}

/// `StoreUpval`: sel tangkapan ke-`idx` = nilai posisi `dari` berjenis `jenis` (kode `simpan`); 1 = tidak ada.
pub extern "C" fn simpan_upval(idx: i64, dari: i64, nilai: i64, jenis: i32) -> i32 {
    let (vm, basis) = konteks();
    let Ok(sel) = vm.upval_at(idx as u32, None) else { return 1 };
    let v = match skalar_kode(jenis, nilai) {
        Some(v) => v,
        None => sediakan(vm, basis, dari as usize).clone(),
    };
    *sel.lock().unwrap_or_else(|p| p.into_inner()) = v;
    0
}

/// `StoreGlobal`: sel `var` modul `nama` = nilai posisi `dari` berjenis `jenis` (kode `simpan`);
/// 1 = bukan `var` yang sudah didefinisikan (VM yang melapor), belum ada yang berubah.
pub extern "C" fn simpan_global(nama: *const String, dari: i64, nilai: i64, jenis: i32) -> i32 {
    let (vm, basis) = konteks();
    let v = match skalar_kode(jenis, nilai) {
        Some(v) => v,
        None => sediakan(vm, basis, dari as usize).clone(),
    };
    match vm.globals.get(unsafe { &*nama }) {
        Some(VmValue::Cell(sel)) => { *sel.lock().unwrap_or_else(|p| p.into_inner()) = v; 0 }
        _ => 1,
    }
}

/// `GetIndex`: posisi `p` (wadah milik VM) diganti elemennya; indeksnya bulat/bool/pecahan (`jenis`,
/// kode `simpan`) atau milik VM di `p + 1`. 1 = galat, belum ada yang berubah (VM yang melapor).
pub extern "C" fn indeks(p: i64, nilai: i64, jenis: i32) -> i32 {
    let (vm, basis) = konteks();
    let i = basis + p as usize;
    let idx = match skalar_kode(jenis, nilai) {
        Some(v) => v,
        None => match vm.stack.get(i + 1) { Some(v) => v.clone(), None => return 1 },
    };
    let Some(wadah) = vm.stack.get(i) else { return 1 };
    match vm.get_index(wadah, &idx, None) {
        Ok(v) => { vm.stack[i] = v; 0 }
        Err(_) => 1,
    }
}

/// `GetField`/`GetFieldPrivat` (`privat` 1): posisi `p` (milik VM) diganti field `nama`-nya. 1 = galat
/// atau method `math::` yang ditulis tanpa `()` (VM yang menjalankannya), belum ada yang berubah.
pub extern "C" fn field(p: i64, nama: *const String, privat: i32) -> i32 {
    let (vm, basis) = konteks();
    let (i, nama) = (basis + p as usize, unsafe { &*nama });
    let Some(obj) = vm.stack.get(i) else { return 1 };
    if privat != 0 && vm.periksa_visibilitas(obj, nama, None).is_err() { return 1; }
    match vm.get_field(obj, nama, None) {
        Ok(VmValue::NativeFn(_)) if matches!(obj, VmValue::Float(_) | VmValue::Int(_)) => 1,
        Ok(v) => { vm.stack[i] = v; 0 }
        Err(_) => 1,
    }
}

/// `Dup` atas posisi `dari` milik VM: posisi `ke` = salinannya.
pub extern "C" fn salin(dari: i64, ke: i64) {
    let (vm, basis) = konteks();
    let v = sediakan(vm, basis, dari as usize).clone();
    *sediakan(vm, basis, ke as usize) = v;
}

/// `Swap` atas dua posisi milik VM.
pub extern "C" fn tukar(x: i64, y: i64) {
    let (vm, basis) = konteks();
    sediakan(vm, basis, x.max(y) as usize);
    vm.stack.swap(basis + x as usize, basis + y as usize);
}

/// Jaga tembolok method: penerima di posisi `p` berlabel `label`; lolos = posisinya dilepas (dipakai
/// `GetMethod`) dan galat method tertunda di atasnya dibuang, seperti `op_get_method` yang berhasil.
/// 1 = label lain, belum ada yang berubah.
pub extern "C" fn cek_metode(p: i64, label: *const String) -> i32 {
    let (vm, basis) = konteks();
    let v = sediakan(vm, basis, p as usize);
    if v.nama_tipe() != Some(unsafe { (*label).as_str() }) { return 1; }
    *v = VmValue::Nil;
    let slot = (basis + p as usize).saturating_sub(1);
    if !vm.galat_metode.is_empty() { vm.galat_metode.retain(|(i, _)| *i < slot); }
    0
}

/// `Dup; GetMethod; Swap` atas penerima milik VM di posisi `p` dengan method yang sudah diketahui: label
/// dijaga di tempat, penerimanya pindah ke `p + 1` (seperti `cek_metode` lalu `Swap`). 1 = label lain:
/// salinan `Dup` ditaruh di `p + 1` dan deopt di `GetMethod`.
pub extern "C" fn metode_tukar(p: i64, label: *const String) -> i32 {
    let (vm, basis) = konteks();
    let i = basis + p as usize;
    let v = sediakan(vm, basis, p as usize);
    if v.nama_tipe() != Some(unsafe { (*label).as_str() }) {
        let salinan = v.clone();
        *sediakan(vm, basis, p as usize + 1) = salinan;
        return 1;
    }
    let v = std::mem::replace(v, VmValue::Nil);
    *sediakan(vm, basis, p as usize + 1) = v;
    if !vm.galat_metode.is_empty() { vm.galat_metode.retain(|(j, _)| *j < i); }
    0
}

/// `LoadLocal`/`AmbilLokal` lalu `metode_tukar`: lokal `slot` (dibaca, atau diambil kalau `ambil`) langsung
/// ke `p + 1`. 1 = label lain: lokalnya di `p`, salinan `Dup` di `p + 1`, deopt di `GetMethod`.
pub extern "C" fn metode_lokal(slot: i64, p: i64, label: *const String, ambil: i32) -> i32 {
    let (vm, basis) = konteks();
    let i = basis + slot as usize;
    let v = if ambil != 0 { vm.ambil_lokal(i) } else { vm.baca_lokal(i) }.unwrap_or(VmValue::Nil);
    if v.nama_tipe() != Some(unsafe { (*label).as_str() }) {
        let salinan = v.clone();
        *sediakan(vm, basis, p as usize) = v;
        *sediakan(vm, basis, p as usize + 1) = salinan;
        return 1;
    }
    *sediakan(vm, basis, p as usize) = VmValue::Nil;
    *sediakan(vm, basis, p as usize + 1) = v;
    let j = basis + p as usize;
    if !vm.galat_metode.is_empty() { vm.galat_metode.retain(|(x, _)| *x < j); }
    0
}

/// Tepi ke titik temu yang dilebarkan: angka native di posisi `p` jadi nilai VM (kode jenis `simpan`).
pub extern "C" fn tulis_vm(p: i64, nilai: i64, jenis: i32) {
    let (vm, basis) = konteks();
    // Tepi cuma menulis posisi native (`Jenis::kode` selain 4).
    let v = skalar_kode(jenis, nilai).expect("tepi menulis angka native");
    *sediakan(vm, basis, p as usize) = v;
}

/// `Jenis::Lebar` di posisi `p` ke stack VM: bulat i128 dari dua belahan 64 bit.
pub extern "C" fn tulis_lebar(p: i64, lo: i64, hi: i64) {
    let (vm, basis) = konteks();
    *sediakan(vm, basis, p as usize) = VmValue::Int(((hi as i128) << 64) | (lo as u64 as i128));
}

/// Posisi `p` (milik VM) sebagai bulat i128: `keluar[0]` belahan bawah, `keluar[1]` atas; 1 = bukan bulat.
pub extern "C" fn buka_lebar(p: i64, keluar: *mut i64) -> i32 {
    let (vm, basis) = konteks();
    match vm.stack.get(basis + p as usize) {
        Some(VmValue::Int(n)) => {
            unsafe { *keluar = *n as i64; *keluar.add(1) = (*n >> 64) as i64; }
            0
        }
        _ => 1,
    }
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
        (Some(VmValue::Rune(c)), 7) => Some(*c as i64),
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
#[inline(always)]
fn norm_indeks(i: i64, n: usize) -> Option<usize> {
    // Panjang larik selalu di bawah `i64::MAX`, jadi `i + n` untuk `i` negatif tidak meluap.
    let j = if i < 0 { i + n as i64 } else { i };
    ((j as u64) < n as u64).then_some(j as usize)
}

/// `IndeksLokal` atas larik berelemen bulat 64 bit: 0 = `*keluar` terisi.
pub extern "C" fn baca(rel: i64, i: i64, keluar: *mut i64) -> i32 {
    let bulat = |g: &[VmValue]| match g.get(norm_indeks(i, g.len())?)? {
        VmValue::Int(n) => i64::try_from(*n).ok(),
        _ => None,
    };
    baca_dengan(rel, keluar, bulat)
}

/// `IndeksLokal` atas larik berelemen pecahan (umpan VM): bit `f64`-nya di `*keluar`.
pub extern "C" fn baca_pecahan(rel: i64, i: i64, keluar: *mut i64) -> i32 {
    let pecahan = |g: &[VmValue]| match g.get(norm_indeks(i, g.len())?)? {
        VmValue::Float(x) => Some(x.to_bits() as i64),
        _ => None,
    };
    baca_dengan(rel, keluar, pecahan)
}

#[inline(always)]
fn baca_dengan(rel: i64, keluar: *mut i64, bulat: impl Fn(&[VmValue]) -> Option<i64>) -> i32 {
    let v = atas_larik(rel, |t| {
        // Milik tunggal: tanpa kunci (aturan `IndeksLokal` VM).
        if let Some(g) = crate::vm::larik_milik_tunggal(t) { return bulat(g); }
        let VmValue::Array(a) = t else { return None };
        let g = a.lock().unwrap_or_else(|p| p.into_inner());
        bulat(&g)
    });
    match v {
        Some(n) => { unsafe { *keluar = n }; 0 }
        None => 1,
    }
}

/// `TulisIndexLokal`: elemen ke-`i` = nilai berjenis `jenis` (1 bulat, 2 bool, 5 pecahan); 0 = tertulis.
pub extern "C" fn tulis(rel: i64, i: i64, nilai: i64, jenis: i32) -> i32 {
    let Some(v) = skalar_kode(jenis, nilai) else { return 1 };
    let ok = atas_larik(rel, |t| {
        if let Some(g) = crate::vm::larik_milik_tunggal(t) {
            let k = norm_indeks(i, g.len())?;
            g[k] = v;
            return Some(());
        }
        // Aturan `PinjamLokal`: isi dipisahkan dari pemegang lain sebelum ditulis (tidak terlihat bila gagal).
        t.pisahkan();
        let VmValue::Array(a) = t else { return None };
        let mut g = a.lock().unwrap_or_else(|p| p.into_inner());
        let k = norm_indeks(i, g.len())?;
        g[k] = v;
        Some(())
    });
    if ok.is_some() { 0 } else { 1 }
}
