//! Satu `JITModule` per proses (memori kode W^X diurus cranelift-jit: ditulis, lalu jadi RX saat
//! finalisasi). Fungsi dan semua callee-nya dianalisis DULU; batch dibatalkan kalau satu tidak layak.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

use super::layak::Jenis;
use super::{deopt, layak, turun, KodeJit};
use crate::vm::value::VmFunction;

struct ModulJit {
    m: JITModule,
    /// Titik masuk → fungsi yang sudah didefinisikan (Arc dipegang supaya alamatnya tak dipakai ulang).
    jadi: HashMap<Kunci, (FuncId, Arc<VmFunction>)>,
    /// Per nomor bingkai (`fid`): fungsi dan rencananya, dibaca `deopt::pulihkan`.
    info: Vec<(Arc<VmFunction>, Arc<layak::Rencana>)>,
    catat: FuncId,
    undi: Option<FuncId>,
    fctx: FunctionBuilderContext,
    urut: u32,
    /// Fungsi yang ditolak dan alasannya, untuk `JRG_JIT_LOG`.
    tolak: Vec<(String, String)>,
}

// `JITModule` memegang penunjuk mentah; semua aksesnya lewat `Mutex` di bawah, satu per satu.
unsafe impl Send for ModulJit {}

fn modul() -> &'static Mutex<Option<ModulJit>> {
    static M: OnceLock<Mutex<Option<ModulJit>>> = OnceLock::new();
    M.get_or_init(|| {
        let mut fb = settings::builder();
        let _ = fb.set("opt_level", "speed");
        let _ = fb.set("is_pic", "false");
        let _ = fb.set("use_colocated_libcalls", "false");
        // Verifier IR memeriksa `turun.rs` di build debug dan mode stres; di jalan biasa cuma biaya kompilasi.
        let periksa = cfg!(debug_assertions) || std::env::var_os("JRG_JIT_STRES").is_some();
        let _ = fb.set("enable_verifier", if periksa { "true" } else { "false" });
        let isa = cranelift_native::builder().ok()
            .and_then(|ib| ib.finish(settings::Flags::new(fb)).ok());
        Mutex::new(isa.and_then(|isa| {
            let mut jb = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
            jb.symbol("jrg_jit_catat", deopt::catat as *const u8);
            jb.symbol("jrg_jit_undi", deopt::undi as *const u8);
            let mut m = JITModule::new(jb);
            let cc = m.isa().default_call_conv();
            let catat = m.declare_function("jrg_jit_catat", Linkage::Import, &turun::tanda_tangan_catat(&m, cc)).ok()?;
            let undi = if deopt::acak_aktif() {
                Some(m.declare_function("jrg_jit_undi", Linkage::Import, &turun::tanda_tangan_undi(cc)).ok()?)
            } else { None };
            Some(ModulJit { m, jadi: HashMap::new(), info: Vec::new(), catat, undi, fctx: FunctionBuilderContext::new(), urut: 0, tolak: Vec::new() })
        }))
    })
}

/// Nama fungsi yang berhasil dikompilasi, untuk `JRG_JIT_LOG`.
pub fn terkompilasi() -> Vec<String> {
    let g = modul().lock().unwrap_or_else(|p| p.into_inner());
    let mut v: Vec<String> = g.as_ref().map(|m| m.jadi.values().map(|(_, f)| f.name.clone()).collect()).unwrap_or_default();
    v.sort();
    v.dedup();
    v
}

/// Fungsi yang ditolak beserta alasannya, untuk `JRG_JIT_LOG`.
pub fn ditolak() -> Vec<(String, String)> {
    let g = modul().lock().unwrap_or_else(|p| p.into_inner());
    g.as_ref().map(|m| m.tolak.clone()).unwrap_or_default()
}

/// Fungsi, instruksi masuk, boleh `Return` native, jenis stack di titik masuk.
type Kunci = (usize, usize, u8, Vec<Jenis>);

fn kunci(f: &Arc<VmFunction>, masuk: usize, kembali: u8, jenis: &[Jenis]) -> Kunci {
    (Arc::as_ptr(f) as usize, masuk, kembali, jenis.to_vec())
}

/// Titik masuk callee yang dipanggil kode native: awal fungsi, semua argumen bulat.
fn kunci_callee(f: &Arc<VmFunction>) -> Kunci {
    kunci(f, 0, layak::KEMBALI_BULAT, &vec![Jenis::Bulat; f.arity as usize])
}

/// Fungsi dan rencana bingkai bernomor `fid` (nomor itu cuma lahir dari kompilasi yang berhasil).
pub fn info(fid: u32) -> (Arc<VmFunction>, Arc<layak::Rencana>) {
    let g = modul().lock().unwrap_or_else(|p| p.into_inner());
    let (f, r) = &g.as_ref().expect("modul JIT hidup selama kodenya").info[fid as usize];
    (Arc::clone(f), Arc::clone(r))
}

/// Kompilasi titik masuk `masuk` fungsi `f` (stack bingkai berjenis `jenis`) beserta callee-nya.
pub fn kompilasi(
    f: &Arc<VmFunction>, masuk: usize, jenis: &[Jenis], kembali: u8,
    cari: &dyn Fn(&str) -> Option<Arc<VmFunction>>,
) -> Option<KodeJit> {
    let mut g = modul().lock().unwrap_or_else(|p| p.into_inner());
    let mj = g.as_mut()?;
    let k0 = kunci(f, masuk, kembali, jenis);
    if let Some((id, _)) = mj.jadi.get(&k0) {
        return Some(KodeJit { masuk: unsafe { std::mem::transmute(mj.m.get_finalized_function(*id)) } });
    }
    // Kumpulkan titik masuk ini dan semua callee yang belum jadi. Callee yang tak layak dilarang dan
    // analisisnya diulang: panggilan ke sana jadi titik keluar.
    let mut larang: Vec<usize> = Vec::new();
    let baru = 'ulang: loop {
        let cari_boleh = |nama: &str| cari(nama).filter(|g| !larang.contains(&(Arc::as_ptr(g) as usize)));
        let mut baru: Vec<(Kunci, Arc<VmFunction>, Arc<layak::Rencana>)> = Vec::new();
        let mut antre = vec![(k0.clone(), Arc::clone(f))];
        while let Some((k, h)) = antre.pop() {
            if mj.jadi.contains_key(&k) || baru.iter().any(|(b, _, _)| *b == k) { continue; }
            let utama = k == k0;
            match layak::analisis(&h, &cari_boleh, k.1, k.3.clone(), k.2) {
                Ok(r) => {
                    antre.extend(r.callee.iter().map(|c| (kunci_callee(c), Arc::clone(c))));
                    baru.push((k, h, Arc::new(r)));
                }
                Err(_) if !utama => {
                    larang.push(Arc::as_ptr(&h) as usize);
                    continue 'ulang;
                }
                Err(alasan) => {
                    let catatan = (f.name.clone(), alasan);
                    if !mj.tolak.contains(&catatan) { mj.tolak.push(catatan); }
                    return None;
                }
            }
        }
        break baru;
    };
    let cc = mj.m.isa().default_call_conv();
    let sig = turun::tanda_tangan(&mj.m, cc);
    let mut id_baru = Vec::new();
    for (_, h, _) in &baru {
        mj.urut += 1;
        let nama = format!("jrg{}_{}", mj.urut, h.name.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>());
        let id = mj.m.declare_function(&nama, Linkage::Local, &sig).ok()?;
        id_baru.push(id);
    }
    let id_dari = |c: &Arc<VmFunction>, mj: &ModulJit| -> FuncId {
        let k = kunci_callee(c);
        match baru.iter().position(|(b, _, _)| *b == k) {
            Some(i) => id_baru[i],
            None => mj.jadi[&k].0,
        }
    };
    let mut ctx = mj.m.make_context();
    let mut utuh = true;
    let fid0 = mj.info.len() as u32;
    for (i, (_, h, r)) in baru.iter().enumerate() {
        let callee: Vec<FuncId> = r.callee.iter().map(|c| id_dari(c, mj)).collect();
        ctx.func.signature = sig.clone();
        let impor = turun::Impor { catat: mj.catat, undi: mj.undi, fid: fid0 + i as u32 };
        turun::turunkan(&mut mj.m, &mut ctx.func, &mut mj.fctx, h, r, &callee, &impor);
        utuh = mj.m.define_function(id_baru[i], &mut ctx).is_ok();
        mj.m.clear_context(&mut ctx);
        if !utuh { break; }
    }
    utuh = utuh && mj.m.finalize_definitions().is_ok();
    if !utuh {
        // Deklarasi yang tak terdefinisi menggagalkan finalisasi berikutnya; proses ini berhenti memakai JIT.
        *g = None;
        return None;
    }
    for (i, (k, h, r)) in baru.into_iter().enumerate() {
        mj.info.push((Arc::clone(&h), r));
        mj.jadi.insert(k, (id_baru[i], h));
    }
    let id = mj.jadi[&k0].0;
    Some(KodeJit { masuk: unsafe { std::mem::transmute(mj.m.get_finalized_function(id)) } })
}
