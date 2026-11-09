//! Satu `JITModule` per proses (memori kode W^X diurus cranelift-jit: ditulis, lalu jadi RX saat
//! finalisasi). Fungsi dan semua callee-nya dianalisis DULU; batch dibatalkan kalau satu tidak layak.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

use super::{deopt, layak, turun, KodeJit};
use crate::vm::value::VmFunction;

struct ModulJit {
    m: JITModule,
    /// Alamat `VmFunction` → fungsi yang sudah didefinisikan (Arc dipegang supaya alamatnya tak dipakai ulang).
    jadi: HashMap<usize, (FuncId, Arc<VmFunction>)>,
    /// Per nomor bingkai (`fid`): fungsi dan rencananya, dibaca `deopt::pulihkan`.
    info: Vec<(Arc<VmFunction>, Arc<layak::Rencana>)>,
    catat: FuncId,
    undi: Option<FuncId>,
    fctx: FunctionBuilderContext,
    urut: u32,
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
            Some(ModulJit { m, jadi: HashMap::new(), info: Vec::new(), catat, undi, fctx: FunctionBuilderContext::new(), urut: 0 })
        }))
    })
}

/// Nama fungsi yang berhasil dikompilasi, untuk `JRG_JIT_LOG`.
pub fn terkompilasi() -> Vec<String> {
    let g = modul().lock().unwrap_or_else(|p| p.into_inner());
    let mut v: Vec<String> = g.as_ref().map(|m| m.jadi.values().map(|(_, f)| f.name.clone()).collect()).unwrap_or_default();
    v.sort();
    v
}

fn kunci(f: &Arc<VmFunction>) -> usize { Arc::as_ptr(f) as usize }

/// Fungsi dan rencana bingkai bernomor `fid` (nomor itu cuma lahir dari kompilasi yang berhasil).
pub fn info(fid: u32) -> (Arc<VmFunction>, Arc<layak::Rencana>) {
    let g = modul().lock().unwrap_or_else(|p| p.into_inner());
    let (f, r) = &g.as_ref().expect("modul JIT hidup selama kodenya").info[fid as usize];
    (Arc::clone(f), Arc::clone(r))
}

pub fn kompilasi(f: &Arc<VmFunction>, cari: &dyn Fn(&str) -> Option<Arc<VmFunction>>) -> Option<KodeJit> {
    let mut g = modul().lock().unwrap_or_else(|p| p.into_inner());
    let mj = g.as_mut()?;
    if let Some((id, _)) = mj.jadi.get(&kunci(f)) {
        return Some(KodeJit { masuk: unsafe { std::mem::transmute(mj.m.get_finalized_function(*id)) } });
    }
    // Kumpulkan f dan semua callee yang belum jadi; satu yang tak layak = batal semuanya.
    let mut baru: Vec<(Arc<VmFunction>, Arc<layak::Rencana>)> = Vec::new();
    let mut antre = vec![Arc::clone(f)];
    while let Some(h) = antre.pop() {
        if mj.jadi.contains_key(&kunci(&h)) || baru.iter().any(|(b, _)| Arc::ptr_eq(b, &h)) { continue; }
        let r = layak::analisis(&h, cari)?;
        antre.extend(r.callee.iter().cloned());
        baru.push((h, Arc::new(r)));
    }
    let cc = mj.m.isa().default_call_conv();
    let sig = turun::tanda_tangan(&mj.m, cc);
    let mut id_baru = Vec::new();
    for (h, _) in &baru {
        mj.urut += 1;
        let nama = format!("jrg{}_{}", mj.urut, h.name.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>());
        let id = mj.m.declare_function(&nama, Linkage::Local, &sig).ok()?;
        id_baru.push(id);
    }
    let id_dari = |c: &Arc<VmFunction>, mj: &ModulJit| -> FuncId {
        match baru.iter().position(|(b, _)| Arc::ptr_eq(b, c)) {
            Some(i) => id_baru[i],
            None => mj.jadi[&kunci(c)].0,
        }
    };
    let mut ctx = mj.m.make_context();
    let mut utuh = true;
    let fid0 = mj.info.len() as u32;
    for (i, (h, r)) in baru.iter().enumerate() {
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
    for (i, (h, r)) in baru.into_iter().enumerate() {
        mj.info.push((Arc::clone(&h), r));
        mj.jadi.insert(kunci(&h), (id_baru[i], h));
    }
    let id = mj.jadi[&kunci(f)].0;
    Some(KodeJit { masuk: unsafe { std::mem::transmute(mj.m.get_finalized_function(id)) } })
}
