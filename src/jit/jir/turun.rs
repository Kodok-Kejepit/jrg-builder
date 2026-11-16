//! JIR → CLIF. Konvensi sama dengan penurun langsung (`super::super::turun`): tanda tangan umum atau register
//! (`tanda_tangan_bulat`), kedalaman dijaga di pintu masuk (`ULANG`), tiap jaga yang gagal mencatat keadaan
//! bingkainya lewat `deopt::catat` lalu kembali `DEOPT`, hasil `Return` dengan status menurut jenisnya.

use cranelift_codegen::ir::{condcodes::{FloatCC, IntCC}, types, Block, FuncRef, InstBuilder, MemFlagsData, StackSlot, StackSlotData, StackSlotKind, Type, Value};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Module};

use super::{Akhir, Banding, Biner, Fungsi, Hasil, Inst, Jaga, Keadaan, Nilai, Tipe, Unar};
use crate::jit::turun::{Impor, BERHASIL, BERHASIL_BOOL, BERHASIL_PECAHAN, DEOPT, GAGAL_SPEK, KEDALAMAN_MAKS, ULANG};

struct Penurun<'a, 'b> {
    b: FunctionBuilder<'b>,
    j: &'a Fungsi,
    nilai: Vec<Option<Value>>,
    slot: StackSlot,
    catat: FuncRef,
    undi: Option<FuncRef>,
    fid: i64,
    ptr: Type,
    /// Penunjuk hasil (`tanda_tangan` umum).
    keluar: Value,
    register: bool,
    /// Callee bulat per nomor `Rencana::callee` (yang dipanggil native).
    callee: Vec<Option<FuncRef>>,
    /// Kedalaman untuk callee (kedalaman bingkai ini + 1).
    kedalaman1: Value,
}

fn tipe_clif(t: Tipe) -> Type { if t == Tipe::I { types::I64 } else { types::F64 } }

pub fn turunkan(
    m: &mut impl Module, fungsi: &mut cranelift_codegen::ir::Function, fctx: &mut FunctionBuilderContext,
    j: &Fungsi, impor: &Impor, id_callee: &[Option<FuncId>],
) {
    let ptr = m.target_config().pointer_type();
    let register = fungsi.signature.returns.len() == 2;
    let mut b = FunctionBuilder::new(fungsi, fctx);
    let masuk = b.create_block();
    b.append_block_params_for_function_params(masuk);
    b.switch_to_block(masuk);
    let parameter = b.block_params(masuk).to_vec();
    let (args, kedalaman, keluar) = if register {
        let nol = b.ins().iconst(ptr, 0);
        (nol, parameter[parameter.len() - 1], nol)
    } else { (parameter[0], parameter[1], parameter[2]) };
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, (j.maks * 8) as u32, 3));
    let catat = m.declare_func_in_func(impor.catat, b.func);
    let undi = impor.undi.map(|u| m.declare_func_in_func(u, b.func));
    let blok: Vec<Block> = j.blok.iter().map(|_| b.create_block()).collect();
    for (i, d) in j.blok.iter().enumerate().skip(1) {
        for p in &d.params { b.append_block_param(blok[i], tipe_clif(j.tipe(*p))); }
    }
    let callee: Vec<Option<FuncRef>> = id_callee.iter().map(|i| i.map(|i| m.declare_func_in_func(i, b.func))).collect();
    let kedalaman1 = b.ins().iadd_imm_s(kedalaman, 1);
    let mut p = Penurun {
        b, j, nilai: vec![None; j.tipe.len()], slot, catat, undi, fid: impor.fid as i64, ptr, keluar, register, callee, kedalaman1,
    };
    // Kedalaman native dijaga di pintu masuk; parameter dimuat dari register atau penyangga argumen.
    let dalam = p.b.create_block();
    p.b.set_cold_block(dalam);
    let terlalu = p.b.ins().icmp_imm_s(IntCC::SignedGreaterThan, kedalaman, KEDALAMAN_MAKS);
    p.b.ins().brif(terlalu, dalam, &[], blok[0], &[]);
    p.b.switch_to_block(dalam);
    p.pulang_status(ULANG);
    p.b.switch_to_block(blok[0]);
    let fl = MemFlagsData::trusted();
    for (k, pos) in j.posisi_masuk.iter().enumerate() {
        let v = if register { parameter[*pos] } else { p.b.ins().load(types::I64, fl, args, (*pos * 8) as i32) };
        let par = j.blok[0].params[k];
        let v = if j.tipe(par) == Tipe::F { p.b.ins().bitcast(types::F64, MemFlagsData::new(), v) } else { v };
        p.nilai[par.0 as usize] = Some(v);
    }
    for (i, d) in j.blok.iter().enumerate() {
        if i > 0 {
            p.b.switch_to_block(blok[i]);
            let ps = p.b.block_params(blok[i]).to_vec();
            for (n, v) in d.params.iter().zip(ps) { p.nilai[n.0 as usize] = Some(v); }
        }
        for ins in &d.inst { p.inst(ins); }
        match d.akhir.as_ref().expect("diperiksa periksa") {
            Akhir::Lompat { ke, args } => {
                let a = p.args(args);
                p.b.ins().jump(blok[ke.0 as usize], &a);
            }
            Akhir::Cabang { kondisi, ya, ya_args, tidak, tidak_args } => {
                let c = p.v(*kondisi);
                let (ya_a, tidak_a) = (p.args(ya_args), p.args(tidak_args));
                p.b.ins().brif(c, blok[ya.0 as usize], &ya_a, blok[tidak.0 as usize], &tidak_a);
            }
            Akhir::Kembali { nilai, hasil } => p.kembali(*nilai, *hasil),
            Akhir::Keluar { keadaan } => p.deopt(keadaan, 0),
        }
    }
    p.b.seal_all_blocks();
    p.b.finalize(m.target_config());
}

impl Penurun<'_, '_> {
    fn v(&self, n: Nilai) -> Value { self.nilai[n.0 as usize].expect("didefinisikan lebih dulu (periksa)") }

    fn tetapkan(&mut self, n: Nilai, v: Value) { self.nilai[n.0 as usize] = Some(v); }

    fn args(&self, a: &[Nilai]) -> Vec<cranelift_codegen::ir::BlockArg> { a.iter().map(|n| self.v(*n).into()).collect() }

    fn pulang_status(&mut self, st: i64) {
        let s = self.b.ins().iconst(types::I32, st);
        if self.register { let nol = self.b.ins().iconst(types::I64, 0); self.b.ins().return_(&[s, nol]); } else { self.b.ins().return_(&[s]); }
    }

    /// Isi native keadaan ke slot rekaman, `deopt::catat`, lalu kembali `DEOPT`.
    fn deopt(&mut self, k: &Keadaan, bendera: i64) {
        for (i, isi) in k.isi.iter().enumerate() {
            let Some(n) = isi else { continue };
            let v = self.v(*n);
            let v = if self.j.tipe(*n) == Tipe::F { self.b.ins().bitcast(types::I64, MemFlagsData::new(), v) } else { v };
            self.b.ins().stack_store(self.ptr, v, self.slot, (i * 8) as i32);
        }
        let pn = self.b.ins().stack_addr(self.ptr, self.slot, 0);
        let fid = self.b.ins().iconst(types::I32, self.fid);
        let ipv = self.b.ins().iconst(types::I32, k.ip as i64 | bendera);
        let n = self.b.ins().iconst(types::I32, k.isi.len() as i64);
        self.b.ins().call(self.catat, &[fid, ipv, pn, n]);
        self.pulang_status(DEOPT);
    }

    /// `cond` bukan nol = deopt; sesudahnya builder di blok lanjutan.
    fn jaga(&mut self, cond: Value, j: &Jaga) {
        let (d, terus) = (self.b.create_block(), self.b.create_block());
        self.b.set_cold_block(d);
        self.b.ins().brif(cond, d, &[], terus, &[]);
        self.b.switch_to_block(d);
        self.deopt(&j.keadaan, if j.spek { GAGAL_SPEK } else { 0 });
        self.b.switch_to_block(terus);
    }

    fn kembali(&mut self, n: Nilai, h: Hasil) {
        let v = self.v(n);
        let v = if self.j.tipe(n) == Tipe::F { self.b.ins().bitcast(types::I64, MemFlagsData::new(), v) } else { v };
        let st = match h { Hasil::Bool => BERHASIL_BOOL, Hasil::Pecahan => BERHASIL_PECAHAN, Hasil::Bulat => BERHASIL };
        let st = self.b.ins().iconst(types::I32, st);
        if self.register { self.b.ins().return_(&[st, v]); } else {
            self.b.ins().store(MemFlagsData::trusted(), v, self.keluar, 0);
            self.b.ins().return_(&[st]);
        }
    }

    fn inst(&mut self, ins: &Inst) {
        match ins {
            Inst::KonstI { hasil, nilai } => { let v = self.b.ins().iconst(types::I64, *nilai); self.tetapkan(*hasil, v); }
            Inst::KonstF { hasil, nilai } => { let v = self.b.ins().f64const(*nilai); self.tetapkan(*hasil, v); }
            Inst::Biner { hasil, op, a, b: y, jaga } => { let v = self.biner(*op, *a, *y, jaga.as_ref()); self.tetapkan(*hasil, v); }
            Inst::Unar { hasil, op, a, jaga } => { let v = self.unar(*op, *a, jaga.as_ref()); self.tetapkan(*hasil, v); }
            Inst::Pilih { hasil, jika, ya, tidak } => {
                let (c, y, t) = (self.v(*jika), self.v(*ya), self.v(*tidak));
                let v = self.b.ins().select(c, y, t);
                self.tetapkan(*hasil, v);
            }
            Inst::JagaJika { kondisi, jaga } => { let c = self.v(*kondisi); self.jaga(c, jaga); }
            Inst::Panggil { hasil, callee, args, di, sesudah } => {
                let fref = self.callee[*callee].expect("callee bulat dikompilasi");
                let mut argv: Vec<Value> = args.iter().map(|a| self.v(*a)).collect();
                argv.push(self.kedalaman1);
                let c = self.b.ins().call(fref, &argv);
                let (st, h) = (self.b.inst_results(c)[0], self.b.inst_results(c)[1]);
                let (terus, gagal, ulang, deopt) = (self.b.create_block(), self.b.create_block(), self.b.create_block(), self.b.create_block());
                for bl in [gagal, ulang, deopt] { self.b.set_cold_block(bl); }
                self.b.ins().brif(st, gagal, &[], terus, &[]);
                self.b.switch_to_block(gagal);
                let u = self.b.ins().icmp_imm_s(IntCC::Equal, st, ULANG);
                self.b.ins().brif(u, ulang, &[], deopt, &[]);
                self.b.switch_to_block(ulang);
                self.deopt(di, 0);
                self.b.switch_to_block(deopt);
                self.deopt(sesudah, 0);
                self.b.switch_to_block(terus);
                self.tetapkan(*hasil, h);
            }
            Inst::Undi { hasil } => {
                let u = self.undi.expect("undi cuma dibangun di deopt acak");
                let c = self.b.ins().call(u, &[]);
                let c = self.b.inst_results(c)[0];
                let v = self.b.ins().uextend(types::I64, c);
                self.tetapkan(*hasil, v);
            }
        }
    }

    fn banding_i(c: Banding) -> IntCC {
        match c {
            Banding::Lt => IntCC::SignedLessThan, Banding::Le => IntCC::SignedLessThanOrEqual,
            Banding::Gt => IntCC::SignedGreaterThan, Banding::Ge => IntCC::SignedGreaterThanOrEqual,
            Banding::Eq => IntCC::Equal, Banding::Ne => IntCC::NotEqual,
        }
    }

    /// Urutan atas NaN selalu `false`; `!=` benar untuk NaN (`!vm_eq`).
    fn banding_f(c: Banding) -> FloatCC {
        match c {
            Banding::Lt => FloatCC::LessThan, Banding::Le => FloatCC::LessThanOrEqual,
            Banding::Gt => FloatCC::GreaterThan, Banding::Ge => FloatCC::GreaterThanOrEqual,
            Banding::Eq => FloatCC::Equal, Banding::Ne => FloatCC::NotEqual,
        }
    }

    fn biner(&mut self, op: Biner, a: Nilai, y: Nilai, jaga: Option<&Jaga>) -> Value {
        let (x, y) = (self.v(a), self.v(y));
        match op {
            Biner::ITambah | Biner::IKurang | Biner::IKali => match jaga {
                Some(j) => {
                    let (h, luap) = match op {
                        Biner::ITambah => self.b.ins().sadd_overflow(x, y),
                        Biner::IKurang => self.b.ins().ssub_overflow(x, y),
                        _ => self.b.ins().smul_overflow(x, y),
                    };
                    self.jaga(luap, j);
                    h
                }
                None => match op {
                    Biner::ITambah => self.b.ins().iadd(x, y),
                    Biner::IKurang => self.b.ins().isub(x, y),
                    _ => self.b.ins().imul(x, y),
                },
            },
            Biner::IBagi | Biner::ISisa => {
                // Pembagi 0 dan -1 jatuh ke VM: kalimat galatnya, luapan `MIN / -1`, dan aturan `int_biner`.
                if let Some(j) = jaga {
                    let nol = self.b.ins().icmp_imm_s(IntCC::Equal, y, 0);
                    let min1 = self.b.ins().icmp_imm_s(IntCC::Equal, y, -1);
                    let tolak = self.b.ins().bor(nol, min1);
                    self.jaga(tolak, j);
                }
                if op == Biner::ISisa { self.b.ins().srem(x, y) } else { self.b.ins().sdiv(x, y) }
            }
            Biner::IDan => self.b.ins().band(x, y),
            Biner::IAtau => self.b.ins().bor(x, y),
            Biner::IXor => self.b.ins().bxor(x, y),
            Biner::IDanBukan => self.b.ins().band_not(x, y),
            Biner::IGeserKiri => {
                let j = jaga.expect("geser kiri selalu dijaga");
                let jauh = self.b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, y, 63);
                self.jaga(jauh, j);
                let h = self.b.ins().ishl(x, y);
                let balik = self.b.ins().sshr(h, y);
                let hilang = self.b.ins().icmp(IntCC::NotEqual, balik, x);
                self.jaga(hilang, j);
                h
            }
            Biner::IGeserKanan => {
                let j = jaga.expect("geser kanan selalu dijaga");
                let salah = self.b.ins().icmp_imm_u(IntCC::UnsignedGreaterThanOrEqual, y, 128);
                self.jaga(salah, j);
                let enam3 = self.b.ins().iconst(types::I64, 63);
                let besar = self.b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, y, 63);
                let k = self.b.ins().select(besar, enam3, y);
                self.b.ins().sshr(x, k)
            }
            Biner::ICmp(c) => { let h = self.b.ins().icmp(Self::banding_i(c), x, y); self.b.ins().uextend(types::I64, h) }
            Biner::FCmp(c) => { let h = self.b.ins().fcmp(Self::banding_f(c), x, y); self.b.ins().uextend(types::I64, h) }
            Biner::FTambah => self.b.ins().fadd(x, y),
            Biner::FKurang => self.b.ins().fsub(x, y),
            Biner::FKali => self.b.ins().fmul(x, y),
            Biner::FBagi => self.b.ins().fdiv(x, y),
        }
    }

    fn unar(&mut self, op: Unar, a: Nilai, jaga: Option<&Jaga>) -> Value {
        let x = self.v(a);
        match op {
            Unar::INeg => {
                if let Some(j) = jaga {
                    let luap = self.b.ins().icmp_imm_s(IntCC::Equal, x, i64::MIN);
                    self.jaga(luap, j);
                }
                self.b.ins().ineg(x)
            }
            Unar::FNeg => self.b.ins().fneg(x),
            Unar::Bukan => self.b.ins().bxor_imm_s(x, 1),
            Unar::IBukanBit => self.b.ins().bnot(x),
            Unar::KeBool => { let c = self.b.ins().icmp_imm_s(IntCC::NotEqual, x, 0); self.b.ins().uextend(types::I64, c) }
            Unar::KeF => self.b.ins().fcvt_from_sint(types::F64, x),
        }
    }
}
