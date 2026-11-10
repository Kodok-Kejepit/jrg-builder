//! Bytecode → Cranelift IR (`dokumentasi/jit/j2-rencana.md`, `j4-rencana.md`). Tiap posisi stack abstrak
//! = satu variabel `i64`. Jaga yang gagal dan titik keluar mencatat keadaan bingkai lewat `deopt::catat`
//! lalu keluar; status 0 berhasil, 1 deopt, 2 panggilan belum dimulai.

use cranelift_codegen::ir::{condcodes::{FloatCC, IntCC}, types, AbiParam, Block, FuncRef, InstBuilder, MemFlagsData, Signature, StackSlot, StackSlotData, StackSlotKind, Type, Value};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId, Module};

use super::layak::{Jenis, Rencana};
use crate::vm::opcode::OpCode;
use crate::vm::value::VmFunction;

pub const BERHASIL: i64 = 0;
/// Bingkai native sudah dicatat (yang paling dalam lebih dulu); VM melanjutkannya.
pub const DEOPT: i64 = 1;
/// Callee keluar sebelum mulai (terlalu dalam): pemanggilnya deopt DI instruksi `Call`, VM yang memanggil.
pub const ULANG: i64 = 2;
/// Berhasil, hasilnya pecahan (bitnya di `keluar`).
pub const BERHASIL_PECAHAN: i64 = 4;
/// Berhasil, hasilnya `bool` (cuma kode yang dimasuki dari VM, `KEMBALI_SKALAR`).
pub const BERHASIL_BOOL: i64 = 3;
/// Kedalaman panggilan native paling banyak; `panggil::coba` menurunkannya mendekati batas bingkai VM.
pub const KEDALAMAN_MAKS: i64 = 4096;

/// `fn(args: *const i64, kedalaman: i32, keluar: *mut i64) -> i32`, satu bentuk untuk semua.
pub fn tanda_tangan(m: &impl Module, cc: CallConv) -> Signature {
    let ptr = m.target_config().pointer_type();
    let mut sig = Signature::new(cc);
    sig.params.push(AbiParam::new(ptr));
    sig.params.push(AbiParam::new(types::I32));
    sig.params.push(AbiParam::new(ptr));
    sig.returns.push(AbiParam::new(types::I32));
    sig
}

/// `deopt::catat(fid, ip, nilai, n)`.
pub fn tanda_tangan_catat(m: &impl Module, cc: CallConv) -> Signature {
    let ptr = m.target_config().pointer_type();
    let mut sig = Signature::new(cc);
    for t in [types::I32, types::I32, ptr, types::I32] { sig.params.push(AbiParam::new(t)); }
    sig
}

/// `deopt::undi() -> i32`.
pub fn tanda_tangan_undi(cc: CallConv) -> Signature {
    let mut sig = Signature::new(cc);
    sig.returns.push(AbiParam::new(types::I32));
    sig
}

/// `lambat::baca(rel, i, keluar) -> i32`.
pub fn tanda_tangan_baca(m: &impl Module, cc: CallConv) -> Signature {
    let mut sig = Signature::new(cc);
    for t in [types::I64, types::I64, m.target_config().pointer_type()] { sig.params.push(AbiParam::new(t)); }
    sig.returns.push(AbiParam::new(types::I32));
    sig
}

/// `lambat::tulis(rel, i, nilai, jenis) -> i32`.
pub fn tanda_tangan_tulis(cc: CallConv) -> Signature {
    let mut sig = Signature::new(cc);
    for t in [types::I64, types::I64, types::I64, types::I32] { sig.params.push(AbiParam::new(t)); }
    sig.returns.push(AbiParam::new(types::I32));
    sig
}

/// Fungsi impor dan identitas bingkai untuk blok deopt.
pub struct Impor { pub catat: FuncId, pub undi: Option<FuncId>, pub baca: FuncId, pub tulis: FuncId, pub fid: u32 }

struct Keluar { catat: FuncRef, fid: i64, ptr: Type, slot: StackSlot }

/// Blok saat ini menjadi keluar deopt: nilai stack `jenis` dicatat untuk dilanjutkan VM di `ip`.
fn keluar_deopt(b: &mut FunctionBuilder, k: &Keluar, var: &[Variable], ip: usize, jenis: &[Jenis]) {
    let slot = k.slot;
    for (i, j) in jenis.iter().enumerate() {
        // Fungsi, `nil` dan nilai milik VM tidak punya isi native; `deopt::pulihkan` membangunnya.
        if !matches!(j, Jenis::Bulat | Jenis::Bool | Jenis::Pecahan) { continue; }
        let v = b.use_var(var[i]);
        b.ins().stack_store(k.ptr, v, slot, (i * 8) as i32);
    }
    let pn = b.ins().stack_addr(k.ptr, slot, 0);
    let fid = b.ins().iconst(types::I32, k.fid);
    let ipv = b.ins().iconst(types::I32, ip as i64);
    let n = b.ins().iconst(types::I32, jenis.len() as i64);
    b.ins().call(k.catat, &[fid, ipv, pn, n]);
    let st = b.ins().iconst(types::I32, DEOPT);
    b.ins().return_(&[st]);
}

/// `cond` benar = deopt di `ip` dengan stack `jenis`; sesudahnya builder ada di blok lanjutan.
fn jaga(b: &mut FunctionBuilder, k: &Keluar, var: &[Variable], cond: Value, ip: usize, jenis: &[Jenis]) {
    let d = b.create_block();
    b.set_cold_block(d);
    let terus = b.create_block();
    b.ins().brif(cond, d, &[], terus, &[]);
    b.switch_to_block(d);
    keluar_deopt(b, k, var, ip, jenis);
    b.switch_to_block(terus);
}

/// Nilai angka jadi `f64`: pecahan dibaca bitnya, bulat dikonversi seperti `i128 as f64` acuan.
fn ke_f64(b: &mut FunctionBuilder, v: Value, j: Jenis) -> Value {
    if j == Jenis::Pecahan { b.ins().bitcast(types::F64, MemFlagsData::new(), v) }
    else { b.ins().fcvt_from_sint(types::F64, v) }
}

fn dari_f64(b: &mut FunctionBuilder, f: Value) -> Value {
    b.ins().bitcast(types::I64, MemFlagsData::new(), f)
}

pub fn turunkan(
    m: &mut impl Module,
    fungsi: &mut cranelift_codegen::ir::Function,
    fctx: &mut FunctionBuilderContext,
    f: &VmFunction,
    r: &Rencana,
    id_callee: &[FuncId],
    impor: &Impor,
) {
    let ptr = m.target_config().pointer_type();
    let mut b = FunctionBuilder::new(fungsi, fctx);
    let var: Vec<Variable> = (0..r.maks.max(1)).map(|_| b.declare_var(types::I64)).collect();
    // Penyangga argumen dan hasil milik pemanggil: selaras, tidak pernah menjebak.
    let fl = MemFlagsData::trusted();
    let kode = &f.chunk.code;
    let blok: Vec<Option<Block>> = (0..kode.len())
        .map(|i| (r.pemimpin[i] && r.keadaan[i].is_some()).then(|| b.create_block())).collect();

    let masuk = b.create_block();
    b.append_block_params_for_function_params(masuk);
    b.switch_to_block(masuk);
    let (args, kedalaman, keluar) = {
        let p = b.block_params(masuk);
        (p[0], p[1], p[2])
    };
    // Satu slot rekaman untuk semua titik deopt fungsi ini (cuma satu yang pernah dipakai per panggilan).
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, (r.maks.max(1) * 8) as u32, 3));
    let k = Keluar { catat: m.declare_func_in_func(impor.catat, b.func), fid: impor.fid as i64, ptr, slot };
    let undi = impor.undi.map(|u| m.declare_func_in_func(u, b.func));
    let (baca, tulis) = (m.declare_func_in_func(impor.baca, b.func), m.declare_func_in_func(impor.tulis, b.func));
    let wadah = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let dalam = b.create_block();
    b.set_cold_block(dalam);
    // Kedalaman native dijaga di pintu masuk; parameter dimuat dari penyangga argumen.
    let terlalu = b.ins().icmp_imm_s(IntCC::SignedGreaterThan, kedalaman, KEDALAMAN_MAKS);
    let mulai = b.create_block();
    b.ins().brif(terlalu, dalam, &[], mulai, &[]);
    b.switch_to_block(mulai);
    // Isi bingkai di titik masuk; posisi tanpa isi native diberi nol supaya tiap variabel terdefinisi.
    let nol = b.ins().iconst(types::I64, 0);
    for v in &var { b.def_var(*v, nol); }
    for (i, j) in r.jenis_masuk.iter().enumerate() {
        if !matches!(j, Jenis::Bulat | Jenis::Bool | Jenis::Pecahan) { continue; }
        let v = b.ins().load(types::I64, fl, args, (i * 8) as i32);
        b.def_var(var[i], v);
    }
    let kedalaman1 = b.ins().iadd_imm_s(kedalaman, 1);
    b.ins().jump(blok[r.masuk].expect("titik masuk selalu pemimpin"), &[]);

    let mut terbuka = false;
    for ip in 0..kode.len() {
        let Some(s) = &r.keadaan[ip] else { terbuka = false; continue };
        if let Some(bl) = blok[ip] {
            if terbuka { b.ins().jump(bl, &[]); }
            b.switch_to_block(bl);
            terbuka = true;
        }
        if let Some(u) = undi {
            let c = b.ins().call(u, &[]);
            let c = b.inst_results(c)[0];
            jaga(&mut b, &k, &var, c, ip, s);
        }
        let t = s.len();
        if r.keluar[ip] {
            keluar_deopt(&mut b, &k, &var, ip, s);
            terbuka = false;
            continue;
        }
        let ins = kode[ip];
        let a = ins.arg as usize;
        let atas = |b: &mut FunctionBuilder, k: usize| b.use_var(var[t - k]);
        match ins.op {
            OpCode::LoadLocal | OpCode::LokalHitung => { let v = b.use_var(var[a]); b.def_var(var[t], v); }
            OpCode::IntImm => { let v = b.ins().iconst(types::I64, ins.arg as i32 as i64); b.def_var(var[t], v); }
            OpCode::Const => { let v = b.ins().iconst(types::I64, r.konstanta[ip].unwrap_or(0)); b.def_var(var[t], v); }
            OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div if s[t - 1] == Jenis::Pecahan || s[t - 2] == Jenis::Pecahan => {
                let x = atas(&mut b, 2);
                let x = ke_f64(&mut b, x, s[t - 2]);
                let y = atas(&mut b, 1);
                let y = ke_f64(&mut b, y, s[t - 1]);
                let h = match ins.op {
                    OpCode::Add => b.ins().fadd(x, y),
                    OpCode::Sub => b.ins().fsub(x, y),
                    OpCode::Mul => b.ins().fmul(x, y),
                    _ => b.ins().fdiv(x, y),
                };
                let h = dari_f64(&mut b, h);
                b.def_var(var[t - 2], h);
            }
            OpCode::Add | OpCode::Sub | OpCode::Mul => {
                let (x, y) = (atas(&mut b, 2), atas(&mut b, 1));
                let (h, luap) = match ins.op {
                    OpCode::Add => b.ins().sadd_overflow(x, y),
                    OpCode::Sub => b.ins().ssub_overflow(x, y),
                    _ => b.ins().smul_overflow(x, y),
                };
                jaga(&mut b, &k, &var, luap, ip, s);
                b.def_var(var[t - 2], h);
            }
            OpCode::Mod | OpCode::Div => {
                let (x, y) = (atas(&mut b, 2), atas(&mut b, 1));
                // Pembagi 0 dan -1 jatuh ke VM: kalimat galatnya, luapan `MIN / -1`, dan aturan `int_biner`.
                let nol = b.ins().icmp_imm_s(IntCC::Equal, y, 0);
                let min1 = b.ins().icmp_imm_s(IntCC::Equal, y, -1);
                let tolak = b.ins().bor(nol, min1);
                jaga(&mut b, &k, &var, tolak, ip, s);
                let h = if ins.op == OpCode::Mod { b.ins().srem(x, y) } else { b.ins().sdiv(x, y) };
                b.def_var(var[t - 2], h);
            }
            OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge | OpCode::Eq | OpCode::Ne
                if s[t - 1] == Jenis::Pecahan || s[t - 2] == Jenis::Pecahan => {
                // Urutan: NaN membuat keempatnya `false`; `!=` benar untuk NaN (`!vm_eq`).
                let cc = match ins.op {
                    OpCode::Lt => FloatCC::LessThan,
                    OpCode::Le => FloatCC::LessThanOrEqual,
                    OpCode::Gt => FloatCC::GreaterThan,
                    OpCode::Ge => FloatCC::GreaterThanOrEqual,
                    OpCode::Eq => FloatCC::Equal,
                    _ => FloatCC::NotEqual,
                };
                let x = atas(&mut b, 2);
                let x = ke_f64(&mut b, x, s[t - 2]);
                let y = atas(&mut b, 1);
                let y = ke_f64(&mut b, y, s[t - 1]);
                let c = b.ins().fcmp(cc, x, y);
                let v = b.ins().uextend(types::I64, c);
                b.def_var(var[t - 2], v);
            }
            OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge | OpCode::Eq | OpCode::Ne => {
                let cc = match ins.op {
                    OpCode::Lt => IntCC::SignedLessThan,
                    OpCode::Le => IntCC::SignedLessThanOrEqual,
                    OpCode::Gt => IntCC::SignedGreaterThan,
                    OpCode::Ge => IntCC::SignedGreaterThanOrEqual,
                    OpCode::Eq => IntCC::Equal,
                    _ => IntCC::NotEqual,
                };
                let (x, y) = (atas(&mut b, 2), atas(&mut b, 1));
                let c = b.ins().icmp(cc, x, y);
                let v = b.ins().uextend(types::I64, c);
                b.def_var(var[t - 2], v);
            }
            OpCode::AmbilLokal => { let v = b.use_var(var[a]); b.def_var(var[t], v); }
            OpCode::IndeksLokal => {
                let (rel, i) = (b.ins().iconst(types::I64, a as i64), atas(&mut b, 1));
                let pk = b.ins().stack_addr(ptr, wadah, 0);
                let c = b.ins().call(baca, &[rel, i, pk]);
                let st = b.inst_results(c)[0];
                jaga(&mut b, &k, &var, st, ip, s);
                let v = b.ins().stack_load(ptr, types::I64, wadah, 0);
                b.def_var(var[t - 1], v);
            }
            OpCode::TulisIndexLokal => {
                let (v, i) = (atas(&mut b, 2), atas(&mut b, 1));
                let rel = b.ins().iconst(types::I64, a as i64);
                let jenis = b.ins().iconst(types::I32, match s[t - 2] { Jenis::Bool => 2, Jenis::Pecahan => 5, _ => 1 });
                let c = b.ins().call(tulis, &[rel, i, v, jenis]);
                let st = b.inst_results(c)[0];
                jaga(&mut b, &k, &var, st, ip, s);
            }
            OpCode::True | OpCode::False => {
                let v = b.ins().iconst(types::I64, (ins.op == OpCode::True) as i64);
                b.def_var(var[t], v);
            }
            OpCode::Nil => {}
            OpCode::Neg if s[t - 1] == Jenis::Pecahan => {
                let x = atas(&mut b, 1);
                let x = ke_f64(&mut b, x, Jenis::Pecahan);
                let h = b.ins().fneg(x);
                let h = dari_f64(&mut b, h);
                b.def_var(var[t - 1], h);
            }
            OpCode::Neg => {
                let x = atas(&mut b, 1);
                // -MIN tidak muat 64 bit: VM menghitungnya di 128 bit.
                let luap = b.ins().icmp_imm_s(IntCC::Equal, x, i64::MIN);
                jaga(&mut b, &k, &var, luap, ip, s);
                let h = b.ins().ineg(x);
                b.def_var(var[t - 1], h);
            }
            OpCode::Not => { let x = atas(&mut b, 1); let h = b.ins().bxor_imm_s(x, 1); b.def_var(var[t - 1], h); }
            OpCode::KeBool => {
                let x = atas(&mut b, 1);
                let c = b.ins().icmp_imm_s(IntCC::NotEqual, x, 0);
                let h = b.ins().uextend(types::I64, c);
                b.def_var(var[t - 1], h);
            }
            OpCode::Dup => { let v = atas(&mut b, 1); b.def_var(var[t], v); }
            OpCode::Swap => {
                let (x, y) = (atas(&mut b, 2), atas(&mut b, 1));
                b.def_var(var[t - 2], y);
                b.def_var(var[t - 1], x);
            }
            OpCode::Pop | OpCode::LoadGlobal => {}
            OpCode::StoreLocal | OpCode::SimpanBuang => { let v = atas(&mut b, 1); b.def_var(var[a], v); }
            OpCode::Jump => { b.ins().jump(blok[a].expect("sasaran lompat terjangkau"), &[]); terbuka = false; }
            OpCode::JumpFalse | OpCode::JumpTrue | OpCode::JumpFalseKeep | OpCode::JumpTrueKeep => {
                let c = atas(&mut b, 1);
                let (sesudah, sasaran) = (blok[ip + 1].expect("sesudah cabang"), blok[a].expect("sasaran cabang"));
                let (ya, tidak) = if matches!(ins.op, OpCode::JumpFalse | OpCode::JumpFalseKeep) {
                    (sesudah, sasaran)
                } else { (sasaran, sesudah) };
                b.ins().brif(c, ya, &[], tidak, &[]);
                terbuka = false;
            }
            OpCode::MakeIterRentang => {
                let (lo, hi) = (atas(&mut b, 2), atas(&mut b, 1));
                // Aturan `range_hitung` untuk langkah 1; selisih yang tak muat 64 bit dihitung VM.
                let (d, luap) = b.ins().ssub_overflow(hi, lo);
                jaga(&mut b, &k, &var, luap, ip, s);
                let (jumlah, ada) = if a == 1 {
                    let satu = b.ins().iconst(types::I64, 1);
                    let (d1, luap) = b.ins().sadd_overflow(d, satu);
                    jaga(&mut b, &k, &var, luap, ip, s);
                    (d1, b.ins().icmp(IntCC::SignedLessThanOrEqual, lo, hi))
                } else { (d, b.ins().icmp(IntCC::SignedLessThan, lo, hi)) };
                let nol = b.ins().iconst(types::I64, 0);
                let jumlah = b.ins().select(ada, jumlah, nol);
                b.def_var(var[t - 2], jumlah);
                b.def_var(var[t - 1], lo);
                b.def_var(var[t], nol);
            }
            OpCode::IterAda => {
                let (jumlah, idx) = (b.use_var(var[a]), b.use_var(var[a + 2]));
                let c = b.ins().icmp(IntCC::SignedLessThan, idx, jumlah);
                let v = b.ins().uextend(types::I64, c);
                b.def_var(var[t], v);
            }
            OpCode::IterMaju => {
                let x = b.use_var(var[a]);
                let luap = b.ins().icmp_imm_s(IntCC::Equal, x, i64::MAX);
                jaga(&mut b, &k, &var, luap, ip, s);
                let h = b.ins().iadd_imm_s(x, 1);
                b.def_var(var[a], h);
            }
            OpCode::IterVar => {
                let kk = a / 2;
                let idx = b.use_var(var[kk + 2]);
                let v = if a % 2 == 0 { idx } else {
                    let awal = b.use_var(var[kk + 1]);
                    let (h, luap) = b.ins().sadd_overflow(awal, idx);
                    jaga(&mut b, &k, &var, luap, ip, s);
                    h
                };
                b.def_var(var[t], v);
            }
            OpCode::Return => {
                let v = atas(&mut b, 1);
                b.ins().store(fl, v, keluar, 0);
                let st = match s[t - 1] { Jenis::Bool => BERHASIL_BOOL, Jenis::Pecahan => BERHASIL_PECAHAN, _ => BERHASIL };
                let st = b.ins().iconst(types::I32, st);
                b.ins().return_(&[st]);
                terbuka = false;
            }
            OpCode::Call => {
                let Jenis::Fungsi(i) = s[t - a - 1] else { unreachable!("dijamin layak.rs") };
                let fref = m.declare_func_in_func(id_callee[i], b.func);
                let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, (a.max(1) * 8) as u32, 3));
                let hasil = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
                for k in 0..a {
                    let v = b.use_var(var[t - a + k]);
                    b.ins().stack_store(ptr, v, slot, (k * 8) as i32);
                }
                let pa = b.ins().stack_addr(ptr, slot, 0);
                let ph = b.ins().stack_addr(ptr, hasil, 0);
                let panggil = b.ins().call(fref, &[pa, kedalaman1, ph]);
                let st = b.inst_results(panggil)[0];
                let terus = b.create_block();
                let gagal = b.create_block();
                b.set_cold_block(gagal);
                b.ins().brif(st, gagal, &[], terus, &[]);
                b.switch_to_block(gagal);
                // Callee belum mulai: VM menjalankan `Call` ini sendiri. Callee sudah tercatat: lanjut sesudahnya.
                let ulang = b.ins().icmp_imm_s(IntCC::Equal, st, ULANG);
                let (bu, bb) = (b.create_block(), b.create_block());
                b.set_cold_block(bu);
                b.set_cold_block(bb);
                b.ins().brif(ulang, bu, &[], bb, &[]);
                b.switch_to_block(bu);
                keluar_deopt(&mut b, &k, &var, ip, s);
                b.switch_to_block(bb);
                keluar_deopt(&mut b, &k, &var, ip + 1, &s[..t - a - 1]);
                b.switch_to_block(terus);
                let v = b.ins().stack_load(ptr, types::I64, hasil, 0);
                b.def_var(var[t - a - 1], v);
            }
            _ => unreachable!("dijamin layak.rs"),
        }
    }
    b.switch_to_block(dalam);
    let st = b.ins().iconst(types::I32, ULANG);
    b.ins().return_(&[st]);
    b.seal_all_blocks();
    b.finalize(m.target_config());
}
