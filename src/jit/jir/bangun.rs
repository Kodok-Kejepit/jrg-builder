//! Bytecode + `layak::Rencana` → JIR. Cakupan JIR-1: semua instruksi terjangkau `Cara::Asli` atas bulat, pecahan
//! dan `bool` (plus `nil`/`()` tanpa isi), tanpa panggilan, tanpa posisi milik VM, tanpa tepi yang mengubah jenis.
//! Di luar itu `Err(alasan)`, dan unitnya diturunkan penurun langsung.

use super::{Akhir, Banding, Biner, Blok, DataBlok, Fungsi, Hasil, Inst, Jaga, Keadaan, Nilai, Tipe, Unar};
use crate::jit::layak::{Cara, Jenis, Rencana, KEMBALI_BULAT};
use crate::vm::opcode::OpCode;
use crate::vm::value::VmFunction;

/// Isi native jenis `j`: tipenya, atau `None` kalau posisinya tidak punya isi (`nil`, `()`).
fn tipe_jenis(j: Jenis) -> Result<Option<Tipe>, String> {
    match j {
        Jenis::Bulat | Jenis::Bool => Ok(Some(Tipe::I)),
        Jenis::Pecahan => Ok(Some(Tipe::F)),
        // Callee dan bawaan maya: nilainya tetap, dibangun `deopt::pulihkan` dari jenisnya.
        Jenis::Kosong | Jenis::Satuan | Jenis::Fungsi(_) | Jenis::Bawaan(_) | Jenis::Bungkus(..) => Ok(None),
        lain => Err(format!("jenis {lain:?} di luar JIR-1")),
    }
}

/// Opcode yang dicakup JIR-1.
fn opcode_dicakup(op: OpCode) -> bool {
    use OpCode::*;
    matches!(op, LoadLocal | LokalHitung | AmbilLokal | IntImm | Const | True | False | Nil | Unit
        | Add | Sub | Mul | Div | Mod | Lt | Le | Gt | Ge | Eq | Ne | BitAnd | BitOr | BitXor | AndNot | BitNot
        | Shl | Shr | Neg | Not | KeBool | Dup | Swap | SlideDown | Pop | StoreLocal | SimpanBuang
        | Jump | JumpFalse | JumpTrue | JumpFalseKeep | JumpTrueKeep
        | MakeIterRentang | IterAda | IterMaju | IterVar | Return | Call | LoadGlobal)
}

struct Pembangun<'a> {
    f: &'a VmFunction,
    r: &'a Rencana,
    j: Fungsi,
    /// Blok per instruksi pemimpin yang terjangkau.
    blok_ip: Vec<Option<Blok>>,
    acak: bool,
}

/// JIR titik masuk `r` fungsi `f`; `acak` = deopt acak di tiap instruksi (`JRG_JIT_STRES=deopt-acak`).
pub fn bangun(f: &VmFunction, r: &Rencana, acak: bool) -> Result<Fungsi, String> {
    periksa_cakupan(f, r)?;
    let kode = &f.chunk.code;
    let mut p = Pembangun {
        f, r, acak,
        j: Fungsi { blok: Vec::new(), masuk: Blok(0), tipe: Vec::new(), posisi_masuk: Vec::new(), maks: 1 },
        blok_ip: vec![None; kode.len()],
    };
    // Blok masuk lebih dulu: parameternya argumen fungsi (atau isi bingkai OSR).
    let mut params = Vec::new();
    for (i, j) in r.jenis_masuk.iter().enumerate() {
        if let Some(t) = tipe_jenis(*j)? {
            params.push(p.j.nilai_baru(t));
            p.j.posisi_masuk.push(i);
        }
    }
    p.j.blok.push(DataBlok { ip: r.masuk, params: params.clone(), inst: Vec::new(), akhir: None });
    for ip in 0..kode.len() {
        if r.keadaan[ip].is_some() && r.pemimpin[ip] {
            let b = p.blok_baru(ip)?;
            p.blok_ip[ip] = Some(b);
        }
    }
    let ke = p.blok_ip[r.masuk].ok_or("titik masuk bukan pemimpin")?;
    let args = p.args_ke(r.masuk, &p.isi_masuk(&params)?)?;
    p.j.blok[0].akhir = Some(Akhir::Lompat { ke, args });
    p.badan()?;
    p.j.maks = r.keadaan.iter().flatten().map(|s| s.len()).max().unwrap_or(1).max(1);
    super::periksa::periksa(&p.j)?;
    Ok(p.j)
}

/// Seluruh instruksi terjangkau harus berada di cakupan JIR-1.
fn periksa_cakupan(f: &VmFunction, r: &Rencana) -> Result<(), String> {
    if r.tepi.iter().any(|((_, ke), asal)| r.keadaan[*ke].as_ref() != Some(asal)) { return Err("tepi yang mengubah jenis".into()); }
    if r.ada_defer { return Err("fungsi ber-defer".into()); }
    for j in &r.jenis_masuk { tipe_jenis(*j)?; }
    for (ip, ins) in f.chunk.code.iter().enumerate() {
        let Some(s) = &r.keadaan[ip] else { continue };
        for j in s { tipe_jenis(*j)?; }
        if !r.wujud[ip].is_empty() { return Err("posisi maya diwujudkan".into()); }
        if r.keluar[ip] { continue; }
        // Fungsi bawaan sebagai konstanta (`Const`): maya, tanpa isi.
        let bawaan = r.cara[ip] == Cara::Konstanta && ins.op == OpCode::Const
            && matches!(f.chunk.constants.get(ins.arg as usize), Some(crate::vm::value::VmValue::NativeFn(_)));
        if r.cara[ip] != Cara::Asli && !bawaan { return Err(format!("{:?} di ip {ip}", r.cara[ip])); }
        if !opcode_dicakup(ins.op) { return Err(format!("{:?} di ip {ip}", ins.op)); }
        // `Return` cuma untuk titik masuk yang boleh kembali native.
        if ins.op == OpCode::Return && r.kembali == crate::jit::layak::KEMBALI_TIDAK { return Err("Return OSR".into()); }
    }
    Ok(())
}

impl Pembangun<'_> {
    fn keadaan(&self, ip: usize) -> &Vec<Jenis> { self.r.keadaan[ip].as_ref().expect("ip terjangkau") }

    /// Blok untuk pemimpin `ip`: satu parameter per posisi skalar keadaannya.
    fn blok_baru(&mut self, ip: usize) -> Result<Blok, String> {
        let mut params = Vec::new();
        for j in self.keadaan(ip).clone() {
            if let Some(t) = tipe_jenis(j)? { params.push(self.j.nilai_baru(t)); }
        }
        self.j.blok.push(DataBlok { ip, params, inst: Vec::new(), akhir: None });
        Ok(Blok(self.j.blok.len() as u32 - 1))
    }

    /// Stack abstrak dari parameter blok (urut posisi skalar keadaan `ip`).
    fn isi_dari_params(&self, ip: usize, params: &[Nilai]) -> Result<Vec<Option<Nilai>>, String> {
        let mut it = params.iter();
        self.keadaan(ip).iter().map(|j| Ok(match tipe_jenis(*j)? { Some(_) => Some(*it.next().ok_or("parameter kurang")?), None => None }))
            .collect()
    }

    fn isi_masuk(&self, params: &[Nilai]) -> Result<Vec<Option<Nilai>>, String> {
        let mut it = params.iter();
        self.r.jenis_masuk.iter().map(|j| Ok(match tipe_jenis(*j)? { Some(_) => Some(*it.next().ok_or("parameter kurang")?), None => None }))
            .collect()
    }

    /// Argumen lompatan ke pemimpin `ke` dari stack `isi`: posisi skalar keadaan tujuan, yang jenisnya harus
    /// sama dengan isinya (tepi pengubah jenis sudah ditolak).
    fn args_ke(&self, ke: usize, isi: &[Option<Nilai>]) -> Result<Vec<Nilai>, String> {
        let s = self.keadaan(ke);
        if s.len() > isi.len() { return Err(format!("stack ke ip {ke} lebih panjang dari asalnya")); }
        let mut args = Vec::new();
        for (i, j) in s.iter().enumerate() {
            match (tipe_jenis(*j)?, isi[i]) {
                (Some(t), Some(v)) if self.j.tipe(v) == t => args.push(v),
                (None, _) => {}
                _ => return Err(format!("isi posisi {i} ke ip {ke} tidak cocok")),
            }
        }
        Ok(args)
    }

    fn keadaan_di(&self, ip: usize, isi: &[Option<Nilai>]) -> Keadaan {
        Keadaan { ip, isi: isi.to_vec() }
    }

    fn badan(&mut self) -> Result<(), String> {
        let kode = &self.f.chunk.code;
        let mut kini: Option<(Blok, Vec<Option<Nilai>>)> = None;
        for ip in 0..kode.len() {
            if self.r.keadaan[ip].is_none() { kini = None; continue; }
            if let Some(b) = self.blok_ip[ip] {
                // Jatuh ke pemimpin: lompat ke bloknya.
                if let Some((asal, isi)) = kini.take() {
                    let args = self.args_ke(ip, &isi)?;
                    self.akhiri(asal, Akhir::Lompat { ke: b, args });
                }
                let params = self.j.blok[b.0 as usize].params.clone();
                kini = Some((b, self.isi_dari_params(ip, &params)?));
            }
            let Some((b, mut isi)) = kini.take() else { continue };
            if isi.len() != self.keadaan(ip).len() { return Err(format!("panjang stack di ip {ip} tidak cocok")); }
            if self.acak {
                let u = self.j.nilai_baru(Tipe::I);
                self.dorong(b, Inst::Undi { hasil: u });
                let jaga = Jaga { keadaan: self.keadaan_di(ip, &isi), spek: false };
                self.dorong(b, Inst::JagaJika { kondisi: u, jaga });
            }
            if self.r.keluar[ip] {
                self.akhiri(b, Akhir::Keluar { keadaan: self.keadaan_di(ip, &isi) });
                continue;
            }
            match self.instruksi(b, ip, &mut isi)? {
                Some(akhir) => self.akhiri(b, akhir),
                None => {
                    let lanjut = self.r.keadaan.get(ip + 1).and_then(|s| s.as_ref())
                        .ok_or_else(|| format!("ip {} tak terjangkau sesudah ip {ip}", ip + 1))?;
                    if lanjut.len() != isi.len() { return Err(format!("panjang stack sesudah ip {ip}: {} lawan {}", isi.len(), lanjut.len())); }
                    // Posisi tanpa isi native di keadaan berikutnya (lubang `AmbilLokal`) tidak membawa nilai.
                    for (i, j) in lanjut.iter().enumerate() { if tipe_jenis(*j)?.is_none() { isi[i] = None; } }
                    kini = Some((b, isi));
                }
            }
        }
        if kini.is_some() { return Err("kode habis tanpa akhir".into()); }
        Ok(())
    }

    fn dorong(&mut self, b: Blok, i: Inst) { self.j.blok[b.0 as usize].inst.push(i); }

    fn akhiri(&mut self, b: Blok, a: Akhir) { self.j.blok[b.0 as usize].akhir = Some(a); }

    fn jaga(&self, ip: usize, isi: &[Option<Nilai>], spek: bool) -> Option<Jaga> {
        Some(Jaga { keadaan: self.keadaan_di(ip, isi), spek })
    }

    /// Nilai di posisi `i` stack yang wajib skalar.
    fn ambil(isi: &[Option<Nilai>], i: usize) -> Result<Nilai, String> {
        isi.get(i).copied().flatten().ok_or_else(|| format!("posisi {i} tanpa isi native"))
    }

    /// Pecahan dari nilai bulat atau pecahan (operand campur, aturan VM).
    fn ke_f(&mut self, b: Blok, v: Nilai) -> Nilai {
        if self.j.tipe(v) == Tipe::F { return v; }
        let h = self.j.nilai_baru(Tipe::F);
        self.dorong(b, Inst::Unar { hasil: h, op: Unar::KeF, a: v, jaga: None });
        h
    }

    fn konst_i(&mut self, b: Blok, n: i64) -> Nilai {
        let h = self.j.nilai_baru(Tipe::I);
        self.dorong(b, Inst::KonstI { hasil: h, nilai: n });
        h
    }

    fn biner(&mut self, b: Blok, op: Biner, x: Nilai, y: Nilai, jaga: Option<Jaga>) -> Nilai {
        let t = match op { Biner::FTambah | Biner::FKurang | Biner::FKali | Biner::FBagi => Tipe::F, _ => Tipe::I };
        let h = self.j.nilai_baru(t);
        self.dorong(b, Inst::Biner { hasil: h, op, a: x, b: y, jaga });
        h
    }

    /// Satu instruksi atas stack `isi`; `Some` = instruksi pengakhir blok.
    fn instruksi(&mut self, b: Blok, ip: usize, isi: &mut Vec<Option<Nilai>>) -> Result<Option<Akhir>, String> {
        let ins = self.f.chunk.code[ip];
        let a = ins.arg as usize;
        let s = self.keadaan(ip).clone();
        let t = isi.len();
        let pecahan = |k: usize| s.get(t.wrapping_sub(k)) == Some(&Jenis::Pecahan);
        match ins.op {
            OpCode::LoadLocal | OpCode::LokalHitung | OpCode::AmbilLokal => {
                let v = *isi.get(a).ok_or("lokal di luar stack")?;
                isi.push(v);
            }
            OpCode::IntImm => { let v = self.konst_i(b, ins.arg as i32 as i64); isi.push(Some(v)); }
            OpCode::Const if matches!(self.f.chunk.constants.get(a), Some(crate::vm::value::VmValue::NativeFn(_))) => isi.push(None),
            OpCode::LoadGlobal => isi.push(None),
            OpCode::Call => return self.panggil(b, ip, isi).map(|_| None),
            OpCode::Const => {
                let k = self.r.konstanta[ip].ok_or("konstanta di luar 64 bit")?;
                let lanjut = self.keadaan(ip + 1);
                let v = if lanjut.last() == Some(&Jenis::Pecahan) {
                    let h = self.j.nilai_baru(Tipe::F);
                    self.dorong(b, Inst::KonstF { hasil: h, nilai: f64::from_bits(k as u64) });
                    h
                } else { self.konst_i(b, k) };
                isi.push(Some(v));
            }
            OpCode::True | OpCode::False => { let v = self.konst_i(b, (ins.op == OpCode::True) as i64); isi.push(Some(v)); }
            OpCode::Nil | OpCode::Unit => isi.push(None),
            OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div if pecahan(1) || pecahan(2) => {
                let (x, y) = (Self::ambil(isi, t - 2)?, Self::ambil(isi, t - 1)?);
                let (x, y) = (self.ke_f(b, x), self.ke_f(b, y));
                let op = match ins.op { OpCode::Add => Biner::FTambah, OpCode::Sub => Biner::FKurang, OpCode::Mul => Biner::FKali, _ => Biner::FBagi };
                let h = self.biner(b, op, x, y, None);
                isi.truncate(t - 2);
                isi.push(Some(h));
            }
            OpCode::Add | OpCode::Sub | OpCode::Mul => {
                let (x, y) = (Self::ambil(isi, t - 2)?, Self::ambil(isi, t - 1)?);
                let op = match ins.op { OpCode::Add => Biner::ITambah, OpCode::Sub => Biner::IKurang, _ => Biner::IKali };
                // Luapan di bingkai ber-stack VM dicatat: kompilasi berikutnya menghitung di 128 bit.
                let jaga = self.jaga(ip, isi, self.r.kembali != KEMBALI_BULAT);
                let h = self.biner(b, op, x, y, jaga);
                isi.truncate(t - 2);
                isi.push(Some(h));
            }
            OpCode::Mod | OpCode::Div => {
                let (x, y) = (Self::ambil(isi, t - 2)?, Self::ambil(isi, t - 1)?);
                let op = if ins.op == OpCode::Mod { Biner::ISisa } else { Biner::IBagi };
                let jaga = self.jaga(ip, isi, false);
                let h = self.biner(b, op, x, y, jaga);
                isi.truncate(t - 2);
                isi.push(Some(h));
            }
            OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge | OpCode::Eq | OpCode::Ne => {
                let c = match ins.op {
                    OpCode::Lt => Banding::Lt, OpCode::Le => Banding::Le, OpCode::Gt => Banding::Gt,
                    OpCode::Ge => Banding::Ge, OpCode::Eq => Banding::Eq, _ => Banding::Ne,
                };
                let (x, y) = (Self::ambil(isi, t - 2)?, Self::ambil(isi, t - 1)?);
                let h = if pecahan(1) || pecahan(2) {
                    let (x, y) = (self.ke_f(b, x), self.ke_f(b, y));
                    self.biner(b, Biner::FCmp(c), x, y, None)
                } else { self.biner(b, Biner::ICmp(c), x, y, None) };
                isi.truncate(t - 2);
                isi.push(Some(h));
            }
            OpCode::BitAnd | OpCode::BitOr | OpCode::BitXor | OpCode::AndNot | OpCode::Shl | OpCode::Shr => {
                let (x, y) = (Self::ambil(isi, t - 2)?, Self::ambil(isi, t - 1)?);
                let (op, jaga) = match ins.op {
                    OpCode::BitAnd => (Biner::IDan, None),
                    OpCode::BitOr => (Biner::IAtau, None),
                    OpCode::BitXor => (Biner::IXor, None),
                    OpCode::AndNot => (Biner::IDanBukan, None),
                    OpCode::Shl => (Biner::IGeserKiri, self.jaga(ip, isi, false)),
                    _ => (Biner::IGeserKanan, self.jaga(ip, isi, false)),
                };
                let h = self.biner(b, op, x, y, jaga);
                isi.truncate(t - 2);
                isi.push(Some(h));
            }
            OpCode::BitNot | OpCode::Neg | OpCode::Not | OpCode::KeBool => {
                let x = Self::ambil(isi, t - 1)?;
                let (op, tipe, jaga) = match ins.op {
                    OpCode::BitNot => (Unar::IBukanBit, Tipe::I, None),
                    OpCode::Neg if pecahan(1) => (Unar::FNeg, Tipe::F, None),
                    OpCode::Neg => (Unar::INeg, Tipe::I, self.jaga(ip, isi, false)),
                    OpCode::Not => (Unar::Bukan, Tipe::I, None),
                    _ => (Unar::KeBool, Tipe::I, None),
                };
                let h = self.j.nilai_baru(tipe);
                self.dorong(b, Inst::Unar { hasil: h, op, a: x, jaga });
                isi[t - 1] = Some(h);
            }
            OpCode::Dup => { let v = *isi.last().ok_or("stack kosong")?; isi.push(v); }
            OpCode::Swap => isi.swap(t - 2, t - 1),
            OpCode::SlideDown => { let v = isi[t - 1]; isi[t - a - 1] = v; isi.truncate(t - a); }
            OpCode::Pop => { isi.pop(); }
            OpCode::StoreLocal => { isi[a] = isi[t - 1]; }
            OpCode::SimpanBuang => { isi[a] = isi[t - 1]; isi.pop(); }
            OpCode::Jump => {
                let ke = self.blok_ip[a].ok_or("sasaran lompat bukan pemimpin")?;
                return Ok(Some(Akhir::Lompat { ke, args: self.args_ke(a, isi)? }));
            }
            OpCode::JumpFalse | OpCode::JumpTrue | OpCode::JumpFalseKeep | OpCode::JumpTrueKeep => {
                let c = Self::ambil(isi, t - 1)?;
                let tetap = matches!(ins.op, OpCode::JumpFalseKeep | OpCode::JumpTrueKeep);
                let sesudah: Vec<Option<Nilai>> = if tetap { isi.clone() } else { isi[..t - 1].to_vec() };
                let (b_lanjut, b_sasar) = (self.blok_ip[ip + 1].ok_or("penerus cabang bukan pemimpin")?,
                    self.blok_ip[a].ok_or("sasaran cabang bukan pemimpin")?);
                let (args_lanjut, args_sasar) = (self.args_ke(ip + 1, &sesudah)?, self.args_ke(a, &sesudah)?);
                let salah = matches!(ins.op, OpCode::JumpFalse | OpCode::JumpFalseKeep);
                let (ya, ya_args, tidak, tidak_args) = if salah {
                    (b_lanjut, args_lanjut, b_sasar, args_sasar)
                } else { (b_sasar, args_sasar, b_lanjut, args_lanjut) };
                return Ok(Some(Akhir::Cabang { kondisi: c, ya, ya_args, tidak, tidak_args }));
            }
            OpCode::MakeIterRentang => {
                let (lo, hi) = (Self::ambil(isi, t - 2)?, Self::ambil(isi, t - 1)?);
                let d = self.biner(b, Biner::IKurang, hi, lo, self.jaga(ip, isi, false));
                let (jumlah, ada) = if a == 1 {
                    let satu = self.konst_i(b, 1);
                    let d1 = self.biner(b, Biner::ITambah, d, satu, self.jaga(ip, isi, false));
                    (d1, self.biner(b, Biner::ICmp(Banding::Le), lo, hi, None))
                } else { (d, self.biner(b, Biner::ICmp(Banding::Lt), lo, hi, None)) };
                let nol = self.konst_i(b, 0);
                let h = self.j.nilai_baru(Tipe::I);
                self.dorong(b, Inst::Pilih { hasil: h, jika: ada, ya: jumlah, tidak: nol });
                isi[t - 2] = Some(h);
                isi[t - 1] = Some(lo);
                isi.push(Some(nol));
            }
            OpCode::IterAda => {
                let (jumlah, idx) = (Self::ambil(isi, a)?, Self::ambil(isi, a + 2)?);
                let h = self.biner(b, Biner::ICmp(Banding::Lt), idx, jumlah, None);
                isi.push(Some(h));
            }
            OpCode::IterMaju => {
                let x = Self::ambil(isi, a)?;
                let maks = self.konst_i(b, i64::MAX);
                let c = self.biner(b, Biner::ICmp(Banding::Eq), x, maks, None);
                let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: false };
                self.dorong(b, Inst::JagaJika { kondisi: c, jaga });
                let satu = self.konst_i(b, 1);
                // Luapan sudah dijaga di atas.
                let h = self.biner(b, Biner::ITambah, x, satu, None);
                isi[a] = Some(h);
            }
            OpCode::IterVar => {
                let kk = a / 2;
                let idx = Self::ambil(isi, kk + 2)?;
                let v = if a % 2 == 0 { idx } else {
                    let awal = Self::ambil(isi, kk + 1)?;
                    self.biner(b, Biner::ITambah, awal, idx, self.jaga(ip, isi, false))
                };
                isi.push(Some(v));
            }
            OpCode::Return => {
                let v = Self::ambil(isi, t - 1)?;
                let hasil = match s[t - 1] { Jenis::Bool => Hasil::Bool, Jenis::Pecahan => Hasil::Pecahan, _ => Hasil::Bulat };
                return Ok(Some(Akhir::Kembali { nilai: v, hasil }));
            }
            op => return Err(format!("{op:?} belum diturunkan JIR")),
        }
        Ok(None)
    }

    /// `Call`: callee bulat yang dikompilasi native, atau intrinsik `int`/`uint`/`float` atas bulat.
    fn panggil(&mut self, b: Blok, ip: usize, isi: &mut Vec<Option<Nilai>>) -> Result<(), String> {
        use crate::jit::layak::{BUNGKUS_KE_BULAT, BUNGKUS_KE_PECAHAN};
        let a = self.f.chunk.code[ip].arg as usize;
        let s = self.keadaan(ip).clone();
        let t = isi.len();
        if a + 1 > t { return Err("argumen panggilan di luar stack".into()); }
        match s[t - a - 1] {
            Jenis::Bungkus(BUNGKUS_KE_BULAT, k) if a == 1 && s[t - 1] == Jenis::Bulat => {
                let v = Self::ambil(isi, t - 1)?;
                // `uint` atas bulat negatif: VM yang melaporkannya.
                let nama = match self.f.chunk.constants.get(k as usize) {
                    Some(crate::vm::value::VmValue::NativeFn(g)) => crate::vm::info_native::info(*g).map(|i| i.kanonik.clone()),
                    _ => None,
                };
                if nama.as_deref() == Some("uint") {
                    let nol = self.konst_i(b, 0);
                    let neg = self.biner(b, Biner::ICmp(Banding::Lt), v, nol, None);
                    let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: false };
                    self.dorong(b, Inst::JagaJika { kondisi: neg, jaga });
                }
                isi.truncate(t - 2);
                isi.push(Some(v));
            }
            Jenis::Bungkus(BUNGKUS_KE_PECAHAN, _) if a == 1 && matches!(s[t - 1], Jenis::Bulat | Jenis::Pecahan) => {
                let v = Self::ambil(isi, t - 1)?;
                let h = self.ke_f(b, v);
                isi.truncate(t - 2);
                isi.push(Some(h));
            }
            Jenis::Fungsi(c) if self.r.callee_asli().contains(&c) => {
                let args: Vec<Nilai> = (t - a..t).map(|i| Self::ambil(isi, i)).collect::<Result<_, _>>()?;
                if args.iter().any(|v| self.j.tipe(*v) != Tipe::I) { return Err("argumen callee bulat bukan bulat".into()); }
                let di = self.keadaan_di(ip, isi);
                let sesudah = self.keadaan_di(ip + 1, &isi[..t - a - 1]);
                let h = self.j.nilai_baru(Tipe::I);
                self.dorong(b, Inst::Panggil { hasil: h, callee: c, args, di, sesudah });
                isi.truncate(t - a - 1);
                isi.push(Some(h));
            }
            j => return Err(format!("panggilan atas {j:?} belum diturunkan JIR")),
        }
        Ok(())
    }
}
