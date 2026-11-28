// runtime/src/canvas/mod.rs — `Canvas::` di biner `--emit-llvm`, WebAssembly dan kernel `--bebas`, tanpa jembatan.
// Isinya cuma lem: registry pegangan, nilai bertag ↔ `NilaiK`, jam dan berkas lewat `jrg_os`. Gambar, kalimat
// galat dan aturan tiap method ada di berkas bersama (`inti`, `layan`, ...), salinan persis milik `jrgc`.
#![allow(dead_code)]

pub mod matek;
pub mod inti;
pub mod layan;
pub mod lanjut;
pub mod pena;
pub mod png;
pub mod png_baca;
pub use inti::*;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::time::Duration;

use crate::json_llvm::{bocorkan, gagal, seragamkan, Dua, KaitBuram, KotakAny, KATA_ARRAY, KATA_DICT, LANGKAH};
use crate::{borrow_str, emit_string};
use layan::{Kanvas, NilaiK, Sajian, Sumber};

/// Hasil struct dari `panggil`, dibaca codegen: `Warna` (`a` = r | g<<8 | b<<16 | a<<24) dan `MousePos`
/// (`a`, `b` = bit pecahan x, y).
pub const TAG_HASIL_WARNA: i32 = 101;
pub const TAG_HASIL_POSISI: i32 = 102;

#[repr(C)]
pub struct Teks { ptr: *const u8, len: i64 }

/// Masukan satu bingkai dari penyaji jembatan (`jembatan/src/lib.rs`, `SajiC`).
#[repr(C)]
struct SajiC {
    ditekan_p: *const u8, ditekan_l: i64,
    ditahan_p: *const u8, ditahan_l: i64,
    ada_tetikus: i32, x: f32, y: f32,
    ada_tombol: i32, tombol: [u8; 3],
    ada_jari: i32, jari_p: *const f32, jari_n: i64,
    galat_p: *const u8, galat_l: i64,
}

extern "C" {
    fn jrg_os(op: i32, ap: *const u8, an: i64, bp: *const u8, bn: i64, i: i64, out: *mut Teks) -> i64;
    fn jrg_dict_rapikan(p: *mut u8, len: i64, langkah: i64);
    // `primitif.c`: pembungkus simbol lemah penyaji jembatan.
    fn jrg_kanvas_lemah_buka(id: *mut i64, mp: *const u8, ml: i64, jp: *const u8, jl: i64, w: i64, h: i64,
                             gp: *mut *const u8, gl: *mut i64) -> i32;
    fn jrg_kanvas_lemah_saji(id: i64, p: *const u32, n: i64, out: *mut SajiC) -> i32;
    fn jrg_kanvas_lemah_judul(id: i64, p: *const u8, l: i64);
    fn jrg_kanvas_lemah_tutup(id: i64);
    fn jrg_kanvas_lemah_layar() -> i32;
}

/// Pegangan `Canvas::` = nomor di sini, mulai 1, berikut nomor penyajinya (0 = tanpa tampilan). Satu thread
/// (korutin bergiliran), jadi tanpa kunci.
static mut DAFTAR: Vec<Option<(Kanvas, i64)>> = Vec::new();

fn daftar() -> &'static mut Vec<Option<(Kanvas, i64)>> {
    unsafe { &mut *core::ptr::addr_of_mut!(DAFTAR) }
}

fn os(op: i32, a: &[u8], b: &[u8], i: i64) -> (i64, String) {
    let mut t = Teks { ptr: core::ptr::null(), len: 0 };
    let r = unsafe { jrg_os(op, a.as_ptr(), a.len() as i64, b.as_ptr(), b.len() as i64, i, &mut t) };
    (r, unsafe { borrow_str(t.ptr, t.len) }.to_string())
}

fn sekarang() -> Duration {
    let (us, _) = os(33, b"", b"", 0);
    Duration::from_micros(us.max(0) as u64)
}

fn tulis(jalur: &str, isi: &[u8]) -> Result<(), String> {
    let (r, _) = os(2, jalur.as_bytes(), isi, 0);
    if r < 0 { Err(os(32, b"", b"", -r).1) } else { Ok(()) }
}

struct TuanRt { sendiri: i64, saji: i64 }

impl layan::Tuan for TuanRt {
    fn sekarang(&self) -> Duration { sekarang() }
    fn tulis_berkas(&mut self, jalur: &str, isi: &[u8]) -> Result<(), String> { tulis(jalur, isi) }
    fn galat_baris(&mut self, baris: &str) {
        let mut s = String::from(baris);
        s.push('\n');
        crate::write_all(2, s.as_bytes());
    }
    fn sajikan(&mut self, fb: &Framebuffer) -> Result<Sajian, String> {
        if self.saji == 0 { return Ok(Sajian::Tanpa); }
        let mut o: SajiC = unsafe { core::mem::zeroed() };
        let r = unsafe { jrg_kanvas_lemah_saji(self.saji, fb.pixels.as_ptr(), fb.pixels.len() as i64, &mut o) };
        let teks = |p: *const u8, l: i64| unsafe { borrow_str(p, l) };
        let daftar = |p, l| -> Vec<String> { teks(p, l).split('\n').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect() };
        Ok(match r {
            1 => Sajian::Tertutup,
            2 => Sajian::Masukan(layan::Masukan {
                ditekan: daftar(o.ditekan_p, o.ditekan_l),
                ditahan: daftar(o.ditahan_p, o.ditahan_l).into_iter().collect(),
                tetikus: (o.ada_tetikus != 0).then_some((o.x, o.y)),
                tombol: (o.ada_tombol != 0).then(|| o.tombol.map(|b| b != 0)),
                jari: (o.ada_jari != 0).then(|| (0..o.jari_n.max(0) as usize)
                    .map(|i| unsafe { (*o.jari_p.add(2 * i), *o.jari_p.add(2 * i + 1)) }).collect()),
            }),
            3 => return Err(teks(o.galat_p, o.galat_l).to_string()),
            _ => Sajian::Tanpa,
        })
    }
    fn ganti_judul(&mut self, judul: &str) {
        if self.saji != 0 { unsafe { jrg_kanvas_lemah_judul(self.saji, judul.as_ptr(), judul.len() as i64) } }
    }
    fn tutup(&mut self) {
        if self.saji != 0 { unsafe { jrg_kanvas_lemah_tutup(self.saji) } }
    }
    fn sumber(&mut self, no: usize) -> Result<Sumber, String> {
        if no as i64 == self.sendiri { return Ok(Sumber::Diri); }
        match daftar().get(no.wrapping_sub(1)).and_then(|k| k.as_ref()).map(|(k, _)| k) {
            Some(k) => Ok(Sumber::Lain { lebar: k.width, tinggi: k.height, piksel: k.fb.pixels.clone() }),
            None => Err(format!("canvas: pegangan {} tidak dikenal", no)),
        }
    }
    // Nama method sudah diperiksa codegen saat menyusun; yang sampai sini cuma lewat nilai bertag.
    fn method_tak_ada(&self, nama: &str) -> String { format!("canvas: tidak punya method `{}`", nama) }
}

/// Mode `"tab"`/`"web"` lewat penyaji jembatan; tanpa jembatan (kernel `--bebas`) tidak ada layarnya.
struct PembukaRt { saji: i64 }

impl layan::Pembuka for PembukaRt {
    fn sekarang(&self) -> Duration { sekarang() }
    fn buka(&mut self, mode: &str, judul: &str, lebar: usize, tinggi: usize) -> Result<(), String> {
        let (mut gp, mut gl) = (core::ptr::null(), 0i64);
        let r = unsafe { jrg_kanvas_lemah_buka(&mut self.saji, mode.as_ptr(), mode.len() as i64, judul.as_ptr(),
                                               judul.len() as i64, lebar as i64, tinggi as i64, &mut gp, &mut gl) };
        match r {
            0 => Ok(()),
            -1 => Err(format!("Canvas::new: mode `\"{}\"` butuh layar, dan biner ini tidak punya — pakai mode \
                               `\"png\"` (berkas tiap bingkai) atau `\"polos\"`", mode)),
            _ => Err(unsafe { borrow_str(gp, gl) }.to_string()),
        }
    }
}

/// `canvas::punya_jendela` (bit 0) dan `canvas::layar_sentuh` (bit 1) menurut penyajinya; tanpa penyaji, tidak ada.
#[no_mangle]
pub extern "C" fn jrg_kanvas_layar() -> i32 { unsafe { jrg_kanvas_lemah_layar() } }

fn nama_tag(tag: i32) -> &'static str {
    match tag & 255 {
        0 => "nil", 1 => "bool", 2 => "int", 3 => "float", 4 => "rune", 5 => "string",
        6 => "array", 7 => "dict", 8 => "unit", 11 => "function", _ => "nilai",
    }
}

/// Satu kotak sebagai `NilaiK`; `tipe` = nama tipe acuan yang dihitung codegen (kosong = dari tagnya).
unsafe fn ke_nilai(a: &KotakAny, tipe: &str, tampil: &str, render: bool, kait: KaitBuram, kata_kanvas: i32) -> NilaiK {
    let tipe = if tipe.is_empty() { nama_tag(a.tag) } else { tipe };
    let skalar = matches!(a.tag & 255, 1 | 2 | 3 | 5 | 8);
    if render && !skalar { return NilaiK::Lain { tipe: tipe.to_string(), tampil: tampil.to_string() }; }
    match a.tag & 255 {
        1 => NilaiK::Bool(a.a & 255 != 0),
        2 => NilaiK::Int(a.a as i64 as i128),
        3 => NilaiK::Float(f64::from_bits(a.a)),
        5 => NilaiK::Str(borrow_str(a.a as *const u8, a.b as i64).to_string()),
        8 => NilaiK::Unit,
        6 => {
            let s = seragamkan(a, KATA_ARRAY);
            let p = s.a as *const KotakAny;
            let isi = (0..s.b as usize).map(|i| ke_nilai(&*p.add(i), "", "", false, kait, kata_kanvas)).collect();
            NilaiK::Larik { isi, tampil: String::new() }
        }
        7 => NilaiK::Struktur { nama: "dict".to_string(), field: isi_dict(a, kait, kata_kanvas), tampil: String::new() },
        10 if a.tag == kata_kanvas => NilaiK::Kanvas(*(a.a as *const i64) as usize),
        10 => match (tipe.strip_prefix("struct:"), kait) {
            (Some(nama), Some(f)) => {
                let mut d = KotakAny { tag: 0, sela: 0, a: 0, b: 0 };
                f(a, &mut d);
                if d.tag & 255 != 7 { return NilaiK::Lain { tipe: tipe.to_string(), tampil: String::new() }; }
                NilaiK::Struktur { nama: nama.to_string(), field: isi_dict(&d, kait, kata_kanvas), tampil: String::new() }
            }
            _ => NilaiK::Lain { tipe: tipe.to_string(), tampil: String::new() },
        },
        _ => NilaiK::Lain { tipe: tipe.to_string(), tampil: String::new() },
    }
}

unsafe fn isi_dict(a: &KotakAny, kait: KaitBuram, kata_kanvas: i32) -> Vec<(String, NilaiK)> {
    let s = seragamkan(a, KATA_DICT);
    let p = s.a as *mut u8;
    jrg_dict_rapikan(p, s.b as i64, LANGKAH);
    (0..s.b as usize).map(|i| {
        let pas = p.add(i * LANGKAH as usize);
        let d = &*(pas as *const Dua);
        let k = borrow_str(d.ptr, d.len).to_string();
        (k, ke_nilai(&*(pas.add(16) as *const KotakAny), "", "", false, kait, kata_kanvas))
    }).collect()
}

fn ke_kotak(n: NilaiK) -> KotakAny {
    let k = |tag: i32, a: u64, b: u64| KotakAny { tag, sela: 0, a, b };
    match n {
        NilaiK::Unit => k(8, 0, 0),
        NilaiK::Bool(b) => k(1, b as u64, 0),
        NilaiK::Int(i) => k(2, i as i64 as u64, 0),
        NilaiK::Float(f) => k(3, f.to_bits(), 0),
        NilaiK::Str(s) => { let (p, l) = bocorkan(s); k(5, p, l) }
        NilaiK::Struktur { nama, field, .. } => {
            let bulat = |f: &str| field.iter().find(|(n, _)| n == f).map_or(0, |(_, v)| match v { NilaiK::Int(i) => *i as u64 & 255, _ => 0 });
            let pecahan = |f: &str| field.iter().find(|(n, _)| n == f).map_or(0, |(_, v)| match v { NilaiK::Float(x) => x.to_bits(), _ => 0 });
            if nama == "Warna" {
                k(TAG_HASIL_WARNA, bulat("r") | bulat("g") << 8 | bulat("b") << 16 | bulat("a") << 24, 0)
            } else {
                k(TAG_HASIL_POSISI, pecahan("x"), pecahan("y"))
            }
        }
        NilaiK::Larik { .. } | NilaiK::Kanvas(_) | NilaiK::Lain { .. } => k(8, 0, 0),
    }
}

unsafe fn argumen(kotak: *const KotakAny, tipe: *const Teks, tampil: *const Teks, n: i64, teks: Option<usize>,
                  kait: KaitBuram, kata_kanvas: i32) -> Vec<NilaiK> {
    (0..n.max(0) as usize).map(|i| {
        let t = &*tipe.add(i);
        let s = &*tampil.add(i);
        ke_nilai(&*kotak.add(i), borrow_str(t.ptr, t.len), borrow_str(s.ptr, s.len), teks == Some(i), kait, kata_kanvas)
    }).collect()
}

/// `Canvas::new(..)` atas `n` argumen; menjawab nomor pegangannya.
#[no_mangle]
pub unsafe extern "C" fn jrg_kanvas_baru(kotak: *const KotakAny, tipe: *const Teks, tampil: *const Teks, n: i64,
                                         kait: KaitBuram, kata_kanvas: i32) -> i64 {
    let args = argumen(kotak, tipe, tampil, n, Some(2), kait, kata_kanvas);
    let mut p = PembukaRt { saji: 0 };
    match layan::baru(&args, &mut p) {
        Ok(k) => { daftar().push(Some((k, p.saji))); daftar().len() as i64 }
        Err(e) => gagal(e),
    }
}

/// `canvas::<metode>(kanvas, ..)`: hasilnya kotak bertag, atau `TAG_HASIL_*` untuk struct.
#[no_mangle]
pub unsafe extern "C" fn jrg_kanvas_panggil(id: i64, mp: *const u8, ml: i64, kotak: *const KotakAny, tipe: *const Teks,
                                            tampil: *const Teks, n: i64, out: *mut KotakAny, kait: KaitBuram,
                                            kata_kanvas: i32) {
    let metode = borrow_str(mp, ml);
    let args = argumen(kotak, tipe, tampil, n, layan::tempat_teks(metode), kait, kata_kanvas);
    let slot = id.wrapping_sub(1) as usize;
    // Diambil dari daftar selama method jalan, jadi `blit` bisa membaca kanvas lain; dikembalikan SEBELUM galat.
    let Some((mut k, saji)) = daftar().get_mut(slot).and_then(|k| k.take()) else {
        gagal(format!("canvas::{}: pegangan kanvas {} tidak dikenal", metode, id));
    };
    let hasil = layan::panggil(&mut k, &mut TuanRt { sendiri: id, saji }, metode, &args);
    daftar()[slot] = Some((k, saji));
    match hasil {
        Ok(v) => out.write(ke_kotak(v)),
        Err(e) => gagal(e),
    }
}

/// `Canvas::load_png(path)`: kalimat galatnya sama dengan mesin acuan (`stdlib/kanvas.rs`).
#[no_mangle]
pub unsafe extern "C" fn jrg_kanvas_muat_png(pp: *const u8, pl: i64) -> i64 {
    let path = borrow_str(pp, pl);
    let mut t = Teks { ptr: core::ptr::null(), len: 0 };
    let r = jrg_os(1, pp, pl, b"".as_ptr(), 0, 0, &mut t);
    if r < 0 { gagal(format!("Canvas::load_png '{}': {}", path, os(32, b"", b"", -r).1)); }
    let isi = if t.len > 0 { core::slice::from_raw_parts(t.ptr, t.len as usize) } else { &[] };
    let (w, h, piksel) = match png_baca::baca(isi) {
        Ok(x) => x,
        Err(e) => gagal(format!("Canvas::load_png '{}': {}", path, e)),
    };
    let mut k = Kanvas::new(w, h, path, sekarang());
    k.fb.pixels = piksel;
    daftar().push(Some((k, 0)));
    daftar().len() as i64
}

/// Tampilan pegangan, sama dengan `Display` acuan: `<Canvas 64x48 'judul'>`.
#[no_mangle]
pub unsafe extern "C" fn jrg_kanvas_tampil(id: i64, out_ptr: *mut *const u8, out_len: *mut i64) {
    let s = match daftar().get(id.wrapping_sub(1) as usize).and_then(|k| k.as_ref()).map(|(k, _)| k) {
        Some(k) => format!("<Canvas {}x{} '{}'>", k.width, k.height, k.title),
        None => "<Canvas [busy]>".to_string(),
    };
    emit_string(s, out_ptr, out_len);
}

/// `canvas::nama_tombol(s)`: nama kanonik tombol; yang asing ditolak dengan kalimat `layan::kenali_tombol`.
#[no_mangle]
pub unsafe extern "C" fn jrg_kanvas_nama_tombol(p: *const u8, l: i64, out_ptr: *mut *const u8, out_len: *mut i64) {
    match layan::kenali_tombol(borrow_str(p, l)) {
        Ok(t) => emit_string(t.to_string(), out_ptr, out_len),
        Err(e) => gagal(e),
    }
}
