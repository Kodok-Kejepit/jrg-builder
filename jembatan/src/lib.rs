// jembatan/src/lib.rs — pintu C dari biner `--emit-llvm` ke builtin milik `jrgc`.
// Nilai menyeberang sebagai kotak bertag (`%JrgAny`).

use jrgc::vm::value::{Label, VmValue};
use std::collections::HashMap;

/// `%JrgAny`: tag (8 bit bawah) + lukisan isi (24 bit atas), lalu dua kata muatan.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KotakAny { tag: i32, sela: i32, a: u64, b: u64 }

#[repr(C)]
struct Dua { ptr: *mut u8, len: i64 }

/// Primitif milik biner pemanggil: pemesan memori, penyisip dict, dan pengubah isi kotak.
#[repr(C)]
pub struct Kait {
    alloc: extern "C" fn(i64, i64) -> *mut u8,
    dict_tempat: unsafe extern "C" fn(*mut Dua, i64, i64, *const u8, i64) -> *mut u8,
    any_ubah: unsafe extern "C" fn(*const KotakAny, i32, *mut KotakAny, *const u8, i64),
    /// Struct/tuple milik program (tag 10) jadi dict bertanda; kosong kalau biner tidak memancarkannya.
    buram: Option<unsafe extern "C" fn(*const KotakAny, *mut KotakAny)>,
    /// Beri goroutine (korutin) lain di biner itu satu giliran.
    serah: extern "C" fn(),
    /// Jumlah goroutine yang belum selesai.
    lain: extern "C" fn() -> i64,
    /// `fn(arg)` jadi korutin baru di biner (`jrg_ko_spawn`).
    luncur: extern "C" fn(extern "C" fn(*mut u8), *mut u8),
}

const LANGKAH: i64 = 40;
const KATA_ARRAY: i32 = 6 | 1 << 8;
const KATA_DICT: i32 = 7 | 1 << 8;
/// Nilai milik `jrgc` yang tidak punya bentuk di biner LLVM (handle, struct, tuple): dititipkan utuh.
const TAG_TITIPAN: i32 = 9;
/// Nilai fungsi milik program: penunjuk ke `{ %JrgFn, adaptor }` (`llvm::codegen_jembatan`).
const TAG_FUNGSI: i32 = 11;
/// Penanda struct dan tuple milik program, sama dengan `llvm::codegen_jembatan::TANDA_*`.
const TANDA_STRUCT: &str = "\u{1}struct";
const TANDA_TUPLE: &str = "\u{1}tuple";
const TANDA_ENUM: &str = "\u{1}enum";
const TANDA_VARIAN: &str = "\u{1}varian";
const TANDA_ISI: &str = "\u{1}isi";
const TAG_BURAM: i32 = 10;

const PESAN_MILIK_PROGRAM: &str = "nilai milik program (struct, enum, tuple, atau fungsi) tidak bisa diserahkan ke builtin ini dari backend `--emit-llvm` — ini celah backend, bukan salah programmu\n  bantuan: serahkan isinya sebagai dict, array, atau teks";

fn kotak(tag: i32, a: u64, b: u64) -> KotakAny { KotakAny { tag, sela: 0, a, b } }

fn bocorkan(s: &str) -> (u64, u64) {
    if s.is_empty() { return (1, 0); }
    let b: &'static mut [u8] = Box::leak(s.as_bytes().to_vec().into_boxed_slice());
    (b.as_ptr() as u64, b.len() as u64)
}

unsafe fn teks<'a>(p: u64, n: u64) -> &'a str {
    if n == 0 { return ""; }
    std::str::from_utf8(std::slice::from_raw_parts(p as *const u8, n as usize)).unwrap_or("")
}

std::thread_local! {
    /// Kedalaman sarang terdalam yang dilihat `dari_any` terakhir di thread ini.
    static DALAM: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
/// Argumen sedalam ini dijalankan di thread pembantu ber-stack besar: penjelajah nilai `jrgc`
/// cuma bisa pindah stack sendiri di thread yang puncaknya tercatat (AGI-20261118-04).
const SARANG_DALAM: usize = 1000;

enum Rakit { Arr(Vec<VmValue>, *const KotakAny, usize, usize), Map(Peta, *const u8, usize, usize, String) }
#[derive(Default)]
struct Peta { m: HashMap<String, VmValue>, nama_struct: Option<String>, nama_enum: Option<String>, varian: Option<String>, tuple: bool }

impl Peta {
    fn isi(&mut self, nama: String, nilai: VmValue) {
        if nama == TANDA_STRUCT {
            if let VmValue::Str(t) = &nilai { self.nama_struct = Some(t.to_string()); return; }
        }
        if nama == TANDA_TUPLE { self.tuple = true; return; }
        if nama == TANDA_ENUM || nama == TANDA_VARIAN {
            if let VmValue::Str(t) = &nilai {
                if nama == TANDA_ENUM { self.nama_enum = Some(t.to_string()) } else { self.varian = Some(t.to_string()) }
                return;
            }
        }
        self.m.insert(nama, nilai);
    }
    fn jadi(mut self) -> VmValue {
        if let (Some(e), Some(v)) = (&self.nama_enum, &self.varian) {
            return VmValue::varian(e, v, self.m.remove(TANDA_ISI));
        }
        if self.tuple {
            return VmValue::tuple((0..self.m.len()).map(|i| self.m.remove(&i.to_string()).unwrap_or(VmValue::Nil)).collect());
        }
        match self.nama_struct { Some(t) => VmValue::struktur(&t, self.m), None => VmValue::map(self.m) }
    }
}

/// Daun langsung jadi nilai; array/dict mendorong bingkainya dan menjawab `None`.
unsafe fn turun(a: &KotakAny, k: &Kait, tumpuk: &mut Vec<Rakit>, simpan: &mut Vec<Box<KotakAny>>) -> Result<Option<VmValue>, String> {
    let pola = "nilai bertipe `\u{1}` tidak bisa diserahkan ke builtin ini dari backend `--emit-llvm`";
    Ok(Some(match a.tag & 255 {
        0 => VmValue::Nil,
        8 => VmValue::Unit,
        1 => VmValue::Bool(a.a & 255 != 0),
        2 => VmValue::Int(a.a as i64 as i128),
        3 => VmValue::Float(f64::from_bits(a.a)),
        4 => char::from_u32(a.a as u32).map(VmValue::Rune).unwrap_or(VmValue::Nil),
        5 => VmValue::str(teks(a.a, a.b)),
        6 => {
            let mut s = *a;
            if a.tag != KATA_ARRAY { (k.any_ubah)(a, KATA_ARRAY, &mut s, pola.as_ptr(), pola.len() as i64); }
            tumpuk.push(Rakit::Arr(Vec::with_capacity(s.b as usize), s.a as *const KotakAny, s.b as usize, 0));
            return Ok(None);
        }
        7 => {
            let mut s = *a;
            if a.tag != KATA_DICT { (k.any_ubah)(a, KATA_DICT, &mut s, pola.as_ptr(), pola.len() as i64); }
            tumpuk.push(Rakit::Map(Peta::default(), s.a as *const u8, s.b as usize, 0, String::new()));
            return Ok(None);
        }
        TAG_TITIPAN => (*(a.a as *const VmValue)).clone(),
        TAG_FUNGSI => fungsi_program(a.a as *const u8, k),
        TAG_BURAM if k.buram.is_some() => {
            let mut o = Box::new(kotak(0, 0, 0));
            (k.buram.unwrap())(a, &mut *o);
            if o.tag & 255 == TAG_BURAM { return Err(PESAN_MILIK_PROGRAM.to_string()); }
            let r = turun(&o, k, tumpuk, simpan);
            simpan.push(o);
            return r;
        }
        // Struct/tuple/enum/fungsi milik program: tidak punya bentuk di sisi `jrgc`.
        _ => return Err(PESAN_MILIK_PROGRAM.to_string()),
    }))
}

/// Iteratif: nilai bersarang berapa pun dalam tidak menghabiskan stack korutin biner (AGI-20261118-04).
unsafe fn dari_any(akar: &KotakAny, k: &Kait) -> Result<VmValue, String> {
    let (mut tumpuk, mut simpan) = (Vec::new(), Vec::new());
    let mut naik = turun(akar, k, &mut tumpuk, &mut simpan)?;
    let mut dalam = tumpuk.len();
    loop {
        dalam = dalam.max(tumpuk.len());
        if let Some(v) = naik.take() {
            match tumpuk.last_mut() {
                None => { DALAM.with(|d| d.set(d.get().max(dalam))); return Ok(v); }
                Some(Rakit::Arr(isi, ..)) => isi.push(v),
                Some(Rakit::Map(peta, _, _, _, nama)) => peta.isi(std::mem::take(nama), v),
            }
        }
        let anak: Option<*const KotakAny> = match tumpuk.last_mut() {
            Some(Rakit::Arr(_, p, n, i)) if *i < *n => { *i += 1; Some(p.add(*i - 1)) }
            Some(Rakit::Map(_, p, n, i, nama)) if *i < *n => {
                let pas = p.add(*i * LANGKAH as usize);
                *i += 1;
                let kunci = &*(pas as *const Dua);
                *nama = teks(kunci.ptr as u64, kunci.len as u64).to_string();
                Some(pas.add(16) as *const KotakAny)
            }
            _ => None,
        };
        naik = match anak {
            Some(c) => turun(&*c, k, &mut tumpuk, &mut simpan)?,
            None => match tumpuk.pop() {
                Some(Rakit::Arr(isi, ..)) => Some(VmValue::array(isi)),
                Some(Rakit::Map(peta, ..)) => Some(peta.jadi()),
                None => unreachable!(),
            },
        };
    }
}

type Adaptor = unsafe extern "C" fn(*const u8, *const KotakAny, i64, *mut KotakAny) -> i32;

/// Penunjuk mentah yang hidup sepanjang proses (kotak fungsi dan tabel kait milik biner).
#[derive(Clone, Copy)]
struct Mentah(*const u8, *const Kait);
unsafe impl Send for Mentah {}
unsafe impl Sync for Mentah {}

/// Nilai yang dipindah utuh antar-thread; `VmValue` tidak `Send` karena isinya bisa sel bersama.
struct Kirim<T>(T);
unsafe impl<T> Send for Kirim<T> {}

type Tugas = Box<dyn FnOnce() + Send>;
type Hasil = std::thread::Result<Option<Result<VmValue, String>>>;

/// Panggilan fungsi milik biner yang dititipkan thread lain, dijalankan thread biner selama ia menunggu (`layani`).
/// `bool`: dari thread pembantu builtin pemblokir (callback-nya, dijalankan di tempat); `false`: dari thread koneksi
/// `dok::serve`, jadi korutin sendiri. `AKTIF` menghitung penunggu yang sedang melayani.
static ANTRE: std::sync::OnceLock<(std::sync::mpsc::Sender<Kirim<(bool, Tugas)>>, std::sync::Mutex<std::sync::mpsc::Receiver<Kirim<(bool, Tugas)>>>)> =
    std::sync::OnceLock::new();
static AKTIF: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

std::thread_local! { static MELAYANI: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

fn antre() -> &'static (std::sync::mpsc::Sender<Kirim<(bool, Tugas)>>, std::sync::Mutex<std::sync::mpsc::Receiver<Kirim<(bool, Tugas)>>>) {
    ANTRE.get_or_init(|| { let (tx, rx) = std::sync::mpsc::channel(); (tx, std::sync::Mutex::new(rx)) })
}

/// Builtin yang memblokir: dijalankan di thread pembantu supaya korutin lain tetap mendapat
/// giliran, dan (untuk `dok::serve`) supaya handler dari thread koneksi bisa dititipkan.
fn pemblokir(nama: &str) -> bool {
    matches!(nama, "dok::serve" | "dok::tcp_accept" | "dok::tcp_recv" | "dok::tcp_recv_line" | "dok::tcp_recv_bytes"
        | "dok::tcp_recv_all" | "dok::tcp_connect" | "dok::udp_recv_from" | "dok::udp_recv_bytes_from" | "dok::ws_recv")
}

unsafe fn jalankan_fungsi(m: Mentah, args: &[VmValue]) -> Result<VmValue, String> {
    let k = &*m.1;
    let masuk: Vec<KotakAny> = args.iter().map(|x| ke_any(x, k)).collect();
    let mut out = kotak(0, 0, 0);
    let adaptor: Adaptor = std::mem::transmute(*(m.0.add(16) as *const usize));
    let r = adaptor(m.0, masuk.as_ptr(), masuk.len() as i64, &mut out);
    if r < 0 { return Err(teks(out.a, out.b).to_string()); }
    dari_any(&out, k)
}

/// Fungsi milik biner LLVM sebagai nilai VM: argumennya dikotak, adaptornya yang memanggil.
/// Biner itu satu thread: panggilan dari thread lain dititipkan ke thread biner kalau ia sedang
/// menunggu builtin pemblokir (`layani`), selain itu DITOLAK.
unsafe fn fungsi_program(kotak_fn: *const u8, k: &Kait) -> VmValue {
    let m = Mentah(kotak_fn, k as *const Kait);
    let pemilik = std::thread::current().id();
    let panggil = move |args: &[VmValue]| -> Result<VmValue, String> {
        // Utuh, bukan per field: penunjuknya cuma boleh menyeberang lewat `Mentah`.
        let m = &m;
        if std::thread::current().id() == pemilik { return jalankan_fungsi(*m, args); }
        if AKTIF.load(std::sync::atomic::Ordering::SeqCst) == 0 {
            return Err("fungsi milik biner `--emit-llvm` dipanggil dari thread lain — biner itu satu thread, \
                        jadi pemanggilan ini ditolak; ini celah backend, bukan salah programmu".to_string());
        }
        let (rtx, rrx) = std::sync::mpsc::channel::<Kirim<Result<VmValue, String>>>();
        let (m, args) = (*m, Kirim(args.to_vec()));
        let tugas: Tugas = Box::new(move || { let a = args; let _ = rtx.send(Kirim(jalankan_fungsi(m, &a.0))); });
        if antre().0.send(Kirim((MELAYANI.with(|m| m.get()), tugas))).is_err() {
            return Err("thread biner `--emit-llvm` sudah berhenti melayani".to_string());
        }
        rrx.recv().map(|r| r.0).unwrap_or_else(|_| Err("handler di thread biner `--emit-llvm` tidak menjawab".to_string()))
    };
    VmValue::Asing(jrgc::vm::ffi_asing::Asing::Fungsi(std::sync::Arc::new(
        jrgc::vm::ffi_asing::FungsiLuar { panggil: Box::new(panggil) })))
}

/// `nama` di thread pembantu; selama menunggu, thread ini menjalankan panggilan yang dititipkan
/// padanya dan memberi korutin lain giliran.
/// Thread pembantu yang menganggur, dipakai ulang: membuat thread per panggilan memakan ±50 µs.
static NGANGGUR: std::sync::Mutex<Vec<std::sync::mpsc::Sender<Tugas>>> = std::sync::Mutex::new(Vec::new());

fn kerjakan(t: Tugas) {
    let siap = NGANGGUR.lock().unwrap_or_else(|p| p.into_inner()).pop();
    let t = match siap { Some(tx) => match tx.send(t) { Ok(()) => return, Err(e) => e.0 }, None => t };
    let (tx, rx) = std::sync::mpsc::channel::<Tugas>();
    let _ = tx.send(t);
    // Ber-stack `jrgc` dan tercatat: penjelajah nilai di builtinnya bisa pindah stack sendiri.
    let _ = std::thread::Builder::new().stack_size(jrgc::interpreter::Interpreter::INTERPRETER_STACK_SIZE).spawn(move || {
        jrgc::interpreter::catat_puncak_stack();
        while let Ok(t) = rx.recv() {
            t();
            NGANGGUR.lock().unwrap_or_else(|p| p.into_inner()).push(tx.clone());
        }
    });
}

extern "C" fn jalan_tugas(p: *mut u8) {
    let t = unsafe { Box::from_raw(p as *mut Tugas) };
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(t));
}

fn layani(nama: &str, masuk: Vec<VmValue>, k: &Kait) -> Hasil {
    let (tx, rx) = std::sync::mpsc::channel::<Kirim<Hasil>>();
    let (nama, masuk) = (nama.to_string(), Kirim(masuk));
    AKTIF.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    kerjakan(Box::new(move || {
        let m = masuk;
        MELAYANI.with(|x| x.set(true));
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| jrgc::vm::panggil_bawaan(&nama, &m.0)));
        MELAYANI.with(|x| x.set(false));
        let _ = tx.send(Kirim(r));
    }));
    let hasil = loop {
        // Kuncinya dilepas sebelum tugasnya jalan: tugas itu boleh masuk `layani` lagi.
        let tugas = antre().1.lock().unwrap_or_else(|p| p.into_inner()).try_recv().ok();
        // Handler dari thread koneksi jadi korutin sendiri: yang menunggu (`ws_recv`, `sleep`, channel)
        // memberi giliran ke handler lain, bukan menumpuk `layani` di atas stack-nya (AGI-20261118-03).
        if let Some(Kirim((milik, t))) = tugas {
            if milik { t(); } else { (k.luncur)(jalan_tugas, Box::into_raw(Box::new(t)) as *mut u8); }
            continue;
        }
        (k.serah)();
        match rx.recv_timeout(std::time::Duration::from_micros(300)) {
            Ok(r) => break r.0,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => break Ok(Some(Err("thread pembantu jembatan berhenti tanpa jawaban".to_string()))),
        }
    };
    AKTIF.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    hasil
}

enum Tulis { Arr(Vec<VmValue>, *mut KotakAny, usize), Dict(Vec<(String, VmValue)>, Box<Dua>, usize) }

/// Iteratif seperti `dari_any`: hasil `py::eval` bersarang dalam tidak menghabiskan stack.
unsafe fn ke_any(akar: &VmValue, k: &Kait) -> KotakAny {
    let daun = |v: &VmValue, tumpuk: &mut Vec<Tulis>| -> Option<KotakAny> {
        Some(match v {
            VmValue::Nil => kotak(0, 0, 0),
            VmValue::Unit => kotak(8, 0, 0),
            VmValue::Bool(b) => kotak(1, *b as u64, 0),
            VmValue::Int(n) => match i64::try_from(*n) {
                Ok(x) => kotak(2, x as u64, 0),
                Err(_) => titip(v),
            },
            VmValue::Float(f) => kotak(3, f.to_bits(), 0),
            VmValue::Rune(c) => kotak(4, *c as u64, 0),
            VmValue::Str(s) => { let (p, n) = bocorkan(s); kotak(5, p, n) }
            VmValue::Array(a) => {
                let isi = a.lock().unwrap_or_else(|p| p.into_inner()).clone();
                let buf = (k.alloc)((isi.len().max(1) * 24) as i64, 8) as *mut KotakAny;
                tumpuk.push(Tulis::Arr(isi, buf, 0));
                return None;
            }
            VmValue::Map(m) if matches!(m.label, Label::Dict) => {
                let isi: Vec<(String, VmValue)> = m.lock().unwrap_or_else(|p| p.into_inner())
                    .iter().map(|(kunci, x)| (kunci.to_string(), x.clone())).collect();
                tumpuk.push(Tulis::Dict(isi, Box::new(Dua { ptr: std::ptr::null_mut(), len: 0 }), 0));
                return None;
            }
            lain => titip(lain),
        })
    };
    let mut tumpuk: Vec<Tulis> = Vec::new();
    let mut naik = daun(akar, &mut tumpuk);
    loop {
        if let Some(x) = naik.take() {
            match tumpuk.last_mut() {
                None => return x,
                Some(Tulis::Arr(_, buf, i)) => { buf.add(*i - 1).write(x); }
                Some(Tulis::Dict(isi, d, i)) => {
                    let (kp, kl) = bocorkan(&isi[*i - 1].0);
                    let pas = (k.dict_tempat)(&mut **d, LANGKAH, 8, kp as *const u8, kl as i64);
                    (pas.add(16) as *mut KotakAny).write(x);
                }
            }
        }
        let anak: Option<VmValue> = match tumpuk.last_mut() {
            Some(Tulis::Arr(isi, _, i)) if *i < isi.len() => { *i += 1; Some(isi[*i - 1].clone()) }
            Some(Tulis::Dict(isi, _, i)) if *i < isi.len() => { *i += 1; Some(isi[*i - 1].1.clone()) }
            _ => None,
        };
        naik = match anak {
            Some(v) => daun(&v, &mut tumpuk),
            None => match tumpuk.pop() {
                Some(Tulis::Arr(isi, buf, _)) => Some(kotak(KATA_ARRAY, buf as u64, isi.len() as u64)),
                Some(Tulis::Dict(_, d, _)) => Some(kotak(KATA_DICT, d.ptr as u64, d.len as u64)),
                None => unreachable!(),
            },
        };
    }
}

fn titip(v: &VmValue) -> KotakAny {
    kotak(TAG_TITIPAN, Box::into_raw(Box::new(v.clone())) as u64, 0)
}

/// Bebaskan satu kotak titipan (handle FFI yang tidak keluar dari bloknya); handle asingnya dilepas worker.
/// # Safety
/// `a` menunjuk kotak sah yang tidak dipakai lagi oleh siapa pun.
#[no_mangle]
pub unsafe extern "C" fn jrg_jembatan_lepas_titipan(a: *const KotakAny) {
    let k = &*a;
    if k.tag & 0xff == TAG_TITIPAN && k.a != 0 { drop(Box::from_raw(k.a as *mut VmValue)); }
}

/// Panggil builtin `nama`: 0 nilai biasa, 1 `Some`/`Ok`, 2 `None`, 3 `Err`, -1 gagal (teksnya di `out`).
/// # Safety
/// `args` menunjuk `n` kotak sah, `out` satu kotak yang bisa ditulis, `kait` primitif pemanggil.
#[no_mangle]
pub unsafe extern "C" fn jrg_jembatan_panggil(
    nama: *const u8, nl: i64, args: *const KotakAny, n: i64, out: *mut KotakAny, kait: *const Kait,
) -> i32 {
    let k = &*kait;
    #[cfg(target_os = "wasi")]
    let _jaga = JagaWasi::masuk();
    let nama = teks(nama as u64, nl as u64);
    let gagal = |pesan: String| { let (p, l) = bocorkan(&jrgc::terjemah::t(&pesan)); kotak(5, p, l) };
    // `::tampil` dan `::typeof`: tampilan dan nama tipe sebuah nilai titipan.
    // `::field`: `x.f` atas nilai titipan (struct milik `jrgc`, mis. `MousePos`).
    if nama == "::field" {
        let v = match dari_any(&*args, k) { Ok(v) => v, Err(e) => { out.write(gagal(e)); return -1; } };
        let f = match dari_any(&*args.add(1), k) { Ok(VmValue::Str(f)) => f.to_string(), _ => String::new() };
        return match &v {
            VmValue::Map(m) if !matches!(m.label, Label::Dict) => match m.lock().unwrap_or_else(|p| p.into_inner()).get(&f) {
                Some(x) => { out.write(ke_any(x, k)); 0 }
                None => { out.write(gagal(format!("`{}` tidak punya field `{}`", v.type_name_penuh(), f))); -1 }
            },
            // Dict dari builtin (`zip::list`): `d.k` membaca kuncinya, kalimat kunci hilangnya milik `--vm`.
            VmValue::Map(_) => match jrgc::vm::baca_field(&v, &f) {
                Ok(x) => { out.write(ke_any(&x, k)); 0 }
                Err(e) => { out.write(gagal(e)); -1 }
            },
            // Tuple (`t.0`) dan jenis lain: pintu field `--vm` sendiri, dengan kalimatnya.
            _ => match jrgc::vm::baca_field(&v, &f) {
                Ok(x) => { out.write(ke_any(&x, k)); 0 }
                Err(e) => { out.write(gagal(e)); -1 }
            },
        };
    }
    // `::field_tulis`: `x.f = v` atas nilai titipan; nilai milik program (bertag 10) tidak lewat sini.
    if nama == "::field_tulis" {
        if (*args).tag & 255 == TAG_BURAM {
            out.write(gagal("menulis field atas struct milik program yang bertag belum didukung backend `--emit-llvm` \
                             — ini celah backend, bukan salah programmu".to_string()));
            return -1;
        }
        let ambil = |i: usize| dari_any(&*args.add(i), k);
        let r = match (ambil(0), ambil(1), ambil(2)) {
            (Ok(v), Ok(VmValue::Str(f)), Ok(x)) => jrgc::vm::tulis_field(&v, &f, x),
            (Err(e), _, _) | (_, _, Err(e)) => Err(e),
            _ => Err("nama field bukan teks".to_string()),
        };
        return match r { Ok(v) => { out.write(ke_any(&v, k)); 0 } Err(e) => { out.write(gagal(e)); -1 } };
    }
    if nama == "::tampil" || nama == "::typeof" {
        DALAM.with(|d| d.set(0));
        let v = match dari_any(&*args, k) { Ok(v) => v, Err(e) => { out.write(gagal(e)); return -1; } };
        let lega = DALAM.with(|d| d.get()) > SARANG_DALAM;
        let t = if nama == "::tampil" { if lega { tampil_lega(&v) } else { v.to_string() } } else {
            // Lewat builtin `typeof` yang sama dengan `--vm`, bukan nama tipe internalnya.
            match jrgc::vm::panggil_bawaan("typeof", &[v.clone()]) {
                Some(Ok(VmValue::Str(t))) => t.to_string(),
                _ => v.type_name().to_string(),
            }
        };
        let (p, l) = bocorkan(&t);
        out.write(kotak(5, p, l));
        return 0;
    }
    DALAM.with(|d| d.set(0));
    let masuk: Vec<VmValue> = match (0..n as usize).map(|i| dari_any(&*args.add(i), k)).collect() {
        Ok(m) => m,
        Err(e) => { out.write(gagal(e)); return -1; }
    };
    // FFI (`py::` dan kawan-kawan) sudah semahal IPC ke worker-nya, dan hasilnya bisa bersarang dalam: selalu
    // di thread pembantu ber-stack `jrgc`, seperti thread interpreter di mesin acuan.
    let ffi = ["py::", "js::", "php::", "lua::"].iter().any(|p| nama.starts_with(p));
    let dalam = ffi || DALAM.with(|d| d.get()) > SARANG_DALAM;
    // Selama ada goroutine lain atau yang melayani (server), panggilan apa pun bisa menunggu
    // sesuatu yang harus jalan di korutin lain: `proses::`, `http::`, `dok::http_get` ke server sendiri.
    let ramai = AKTIF.load(std::sync::atomic::Ordering::SeqCst) > 0 || (k.lain)() > 0;
    // WASI tanpa thread (`--emit-wasm`): semuanya di thread biner; yang memblokir ditolak builtinnya sendiri.
    if !cfg!(target_os = "wasi") && (pemblokir(nama) || dalam || (ramai && !nama.starts_with("::"))) {
        let hasil = layani(nama, masuk, k);
        return tulis_hasil(nama, hasil, out, k);
    }
    // Jalur langsung = thread utama biner: batas stack-nya dipasang supaya penjelajah nilai `jrgc`
    // (hasil `py::eval` bersarang dalam, JSON) pindah stack sebelum habis (AGI-20261118-04).
    let lama = stack_utama().map(|(p, u)| jrgc::interpreter::pasang_stack_luar(p, u));
    let _pulih = Pulih(lama);
    // `::extern`: argumen pertamanya deskripsi `extern fn`-nya, sisanya argumen panggilannya.
    // `::metode`: argumen pertamanya nama method, lalu penerima dan argumennya.
    let hasil = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match (nama, masuk.first()) {
        ("::extern", Some(VmValue::Str(d))) => Some(jrgc::vm::panggil_extern(d, &masuk[1..])),
        ("::metode", Some(VmValue::Str(m))) => Some(jrgc::vm::panggil_metode(m, &masuk[1..])),
        // Registry `jawa::` milik biner itu sendiri: hitungannya datang dari sana, bentuknya dari `jrgc`.
        ("::jawa_stats", _) => {
            let n: Vec<i128> = masuk.iter().map(|v| if let VmValue::Int(x) = v { *x } else { 0 }).collect();
            Some(Ok(jrgc::stdlib::jawa::heap_stats(n[0], n[1], n[2], n[3], n[4])))
        }
        _ => jrgc::vm::panggil_bawaan(nama, &masuk),
    }));
    tulis_hasil(nama, hasil, out, k)
}

/// Tampilan nilai bersarang dalam: di thread ber-stack `jrgc` yang tercatat, bukan di stack korutin biner.
fn tampil_lega(v: &VmValue) -> String {
    if cfg!(target_os = "wasi") { return v.to_string(); }
    let v = Kirim(v.clone());
    std::thread::scope(|s| {
        let h = std::thread::Builder::new().stack_size(jrgc::interpreter::Interpreter::INTERPRETER_STACK_SIZE)
            .spawn_scoped(s, move || { let v = v; jrgc::interpreter::catat_puncak_stack(); Kirim(v.0.to_string()) });
        match h { Ok(h) => h.join().map(|t| t.0).unwrap_or_default(), Err(_) => String::new() }
    })
}

struct Pulih(Option<(usize, usize)>);
impl Drop for Pulih {
    fn drop(&mut self) { if let Some((p, u)) = self.0 { jrgc::interpreter::pasang_stack_luar(p, u); } }
}

/// Puncak dan ukuran stack thread ini (thread utama biner), dengan sisa 64 KiB untuk bingkai biner sendiri.
fn stack_utama() -> Option<(usize, usize)> {
    std::thread_local! { static BATAS: std::cell::OnceCell<Option<(usize, usize)>> = const { std::cell::OnceCell::new() }; }
    BATAS.with(|b| *b.get_or_init(|| batas_os().map(|(p, u)| (p, u.saturating_sub(64 * 1024)))))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn batas_os() -> Option<(usize, usize)> {
    unsafe {
        let mut attr: libc::pthread_attr_t = std::mem::zeroed();
        if libc::pthread_getattr_np(libc::pthread_self(), &mut attr) != 0 { return None; }
        let (mut alamat, mut ukuran): (*mut libc::c_void, usize) = (std::ptr::null_mut(), 0);
        let r = libc::pthread_attr_getstack(&attr, &mut alamat, &mut ukuran);
        libc::pthread_attr_destroy(&mut attr);
        if r != 0 || ukuran == 0 { return None; }
        Some((alamat as usize + ukuran, ukuran))
    }
}

#[cfg(target_os = "macos")]
fn batas_os() -> Option<(usize, usize)> {
    unsafe {
        let t = libc::pthread_self();
        Some((libc::pthread_get_stackaddr_np(t) as usize, libc::pthread_get_stacksize_np(t)))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
fn batas_os() -> Option<(usize, usize)> { None }

unsafe fn tulis_hasil(nama: &str, hasil: Hasil, out: *mut KotakAny, k: &Kait) -> i32 {
    let gagal = |pesan: String| { let (p, l) = bocorkan(&jrgc::terjemah::t(&pesan)); kotak(5, p, l) };
    match hasil {
        Err(_) => { out.write(gagal(format!("`{}` panik di dalam jrgc — ini bug di jrgc, bukan di programmu", nama))); -1 }
        Ok(None) => { out.write(gagal(format!("`{}` tidak ada di pustaka standar", nama))); -1 }
        Ok(Some(Err(e))) => { out.write(gagal(e)); -1 }
        Ok(Some(Ok(v))) => match v.bagian_enum() {
            Some(("Option", "Some", Some(p))) | Some(("Result", "Ok", Some(p))) => { out.write(ke_any(p, k)); 1 }
            Some(("Result", "Ok", None)) => { out.write(kotak(8, 0, 0)); 1 }
            Some(("Option", "None", _)) => { out.write(kotak(0, 0, 0)); 2 }
            Some(("Result", "Err", p)) => { out.write(p.map(|x| ke_any(x, k)).unwrap_or(kotak(0, 0, 0))); 3 }
            _ => { out.write(ke_any(&v, k)); 0 }
        },
    }
}

/// Dipanggil pengantar APK (`src/llvm/android.c`) sebelum `main` program jalan.
/// # Safety
/// `t` menunjuk tabel yang hidup selama prosesnya.
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn jrg_android_pasang(t: *const jrgc::canvas::jendela::TabelAndroid) {
    jrgc::canvas::jendela::pasang(t)
}

/// WASI (`--emit-wasm`): kedalaman panggilan jembatan. Bingkai `jrgc` tidak diinstrumentasi Asyncify, jadi korutin
/// yang berganti di bawahnya ditolak keras (`korutin_wasm.c`, `jrg_wasm_cek_jembatan`).
#[cfg(target_os = "wasi")]
#[no_mangle]
#[allow(non_upper_case_globals)]
pub static mut jrg_wasm_di_jembatan: i32 = 0;

#[cfg(target_os = "wasi")]
struct JagaWasi;

#[cfg(target_os = "wasi")]
impl JagaWasi {
    fn masuk() -> Self { unsafe { jrg_wasm_di_jembatan += 1 }; JagaWasi }
}

#[cfg(target_os = "wasi")]
impl Drop for JagaWasi {
    fn drop(&mut self) { unsafe { jrg_wasm_di_jembatan -= 1 }; }
}

