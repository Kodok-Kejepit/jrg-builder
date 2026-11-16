//! Bytecode + `layak::Rencana` → JIR. Cakupan JIR-1: semua instruksi terjangkau `Cara::Asli` atas bulat, pecahan
//! dan `bool` (plus `nil`/`()` tanpa isi), tanpa panggilan, tanpa posisi milik VM, tanpa tepi yang mengubah jenis.
//! Di luar itu `Err(alasan)`, dan unitnya diturunkan penurun langsung.

use super::{OpL, Akhir, Arg, Banding, Biner, Blok, DataBlok, Fungsi, Hasil, Inst, Jaga, Keadaan, Nilai, Pb, Tipe, Uji, Unar};
use crate::jit::layak::{Cara, Jenis, Rencana, KEMBALI_BULAT};
use crate::vm::opcode::OpCode;
use crate::vm::value::VmFunction;

/// Isi native jenis `j`: tipenya, atau `None` kalau posisinya tidak punya isi (`nil`, `()`).
fn tipe_jenis(j: Jenis) -> Result<Option<Tipe>, String> {
    match j {
        // Rune: kode karakter di register; jenisnya dibawa keadaan (`Jenis::kode` 7).
        Jenis::Bulat | Jenis::Bool | Jenis::Rune => Ok(Some(Tipe::I)),
        Jenis::Pecahan => Ok(Some(Tipe::F)),
        Jenis::Lebar => Ok(Some(Tipe::L)),
        // Tanpa isi native: callee/bawaan/konstanta maya, nilai milik VM, salinan lokal (diwujudkan lewat
        // `Rencana::wujud`); `deopt::pulihkan` membangun atau membiarkannya menurut jenisnya.
        Jenis::Kosong | Jenis::Satuan | Jenis::Fungsi(_) | Jenis::Bawaan(_) | Jenis::Bungkus(..) | Jenis::Nilai
        | Jenis::Salinan(_) | Jenis::Konst(_) => Ok(None),
    }
}

/// Opcode yang dicakup JIR-1.
fn opcode_dicakup(op: OpCode) -> bool {
    use OpCode::*;
    matches!(op, LoadLocal | LokalHitung | AmbilLokal | IntImm | Const | True | False | Nil | Unit
        | Add | Sub | Mul | Div | Mod | Lt | Le | Gt | Ge | Eq | Ne | BitAnd | BitOr | BitXor | AndNot | BitNot
        | Shl | Shr | Neg | Not | KeBool | Dup | Swap | SlideDown | Pop | StoreLocal | SimpanBuang
        | Jump | JumpFalse | JumpTrue | JumpFalseKeep | JumpTrueKeep
        | MakeIterRentang | IterAda | IterMaju | IterVar | Return | Call | LoadGlobal | IndeksLokal | GetIndex | TulisIndexLokal)
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
    let args = p.args_ke(Blok(0), r.masuk, &p.isi_masuk(&params)?)?;
    p.j.blok[0].akhir = Some(Akhir::Lompat { ke, args });
    p.badan()?;
    p.j.maks = r.keadaan.iter().flatten().map(|s| s.len()).max().unwrap_or(1).max(1);
    super::periksa::periksa(&p.j)?;
    Ok(p.j)
}

/// Seluruh instruksi terjangkau harus berada di cakupan JIR-1.
fn periksa_cakupan(f: &VmFunction, r: &Rencana) -> Result<(), String> {
    if r.ada_defer { return Err("fungsi ber-defer".into()); }
    for j in &r.jenis_masuk { tipe_jenis(*j)?; }
    for (ip, ins) in f.chunk.code.iter().enumerate() {
        let Some(s) = &r.keadaan[ip] else { continue };
        for j in s { tipe_jenis(*j)?; }
        if r.keluar[ip] { continue; }
        match &r.cara[ip] {
            Cara::Asli | Cara::Buka(_) => if !opcode_dicakup(ins.op) { return Err(format!("{:?} di ip {ip}", ins.op)); },
            Cara::Konstanta | Cara::Muat | Cara::Ambil | Cara::Simpan | Cara::Buang | Cara::Benar | Cara::KembaliNilai
            | Cara::KembaliUnit | Cara::Langkah { .. } | Cara::Bawaan | Cara::Biner | Cara::MuatUpval | Cara::SimpanUpval
            | Cara::TeksBulat | Cara::Geser | Cara::Cabang { .. } | Cara::Bangun(_) | Cara::Salinan | Cara::Salin | Cara::Tukar
            | Cara::Metode(_) | Cara::IterAda | Cara::Maju | Cara::Langsung(_) | Cara::Panjang | Cara::PanjangLokal(_)
            | Cara::Field | Cara::Pola | Cara::Payload | Cara::MuatGlobal | Cara::SimpanGlobal | Cara::Indeks
            | Cara::TipeLipat(_) | Cara::TipeSama(_) | Cara::PayloadSama(..) | Cara::PayloadPanjang(_) => {}
            c => return Err(format!("{c:?} di ip {ip}")),
        }
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
    /// Bulat yang bertemu `Lebar` di titik temu dilebarkan di blok asal `dari`.
    fn args_ke(&mut self, dari: Blok, ke: usize, isi: &[Option<Nilai>]) -> Result<Vec<Nilai>, String> {
        let s = self.keadaan(ke).clone();
        if s.len() > isi.len() { return Err(format!("stack ke ip {ke} lebih panjang dari asalnya")); }
        let mut args = Vec::new();
        for (i, j) in s.iter().enumerate() {
            match (tipe_jenis(*j)?, isi[i]) {
                (Some(t), Some(v)) if self.j.tipe(v) == t => args.push(v),
                (Some(Tipe::L), Some(v)) if self.j.tipe(v) == Tipe::I => {
                    let h = self.j.nilai_baru(Tipe::L);
                    self.dorong(dari, Inst::Unar { hasil: h, op: Unar::KeL, a: v, jaga: None });
                    args.push(h);
                }
                (None, _) => {}
                _ => return Err(format!("isi posisi {i} ke ip {ke} tidak cocok")),
            }
        }
        Ok(args)
    }

    /// Keadaan deopt di `ip`: isi native cuma untuk posisi yang skalar menurut keadaan `ip` (nilai milik VM yang baru
    /// dibuka tetap utuh di stack VM, jadi VM membacanya dari sana).
    fn keadaan_di(&self, ip: usize, isi: &[Option<Nilai>]) -> Keadaan {
        let s = self.keadaan(ip);
        Keadaan { ip, isi: isi.iter().enumerate().map(|(i, v)| if s.get(i).is_some_and(|j| j.skalar() || *j == Jenis::Lebar) { *v } else { None }).collect() }
    }

    fn badan(&mut self) -> Result<(), String> {
        let kode = &self.f.chunk.code;
        let mut kini: Option<(Blok, Vec<Option<Nilai>>)> = None;
        for ip in 0..kode.len() {
            if self.r.keadaan[ip].is_none() { kini = None; continue; }
            if let Some(b) = self.blok_ip[ip] {
                // Jatuh ke pemimpin: lompat ke bloknya.
                if let Some((asal, isi)) = kini.take() {
                    let (ke, args) = self.tepi(asal, ip - 1, ip, &isi)?;
                    self.akhiri(asal, Akhir::Lompat { ke, args });
                }
                let params = self.j.blok[b.0 as usize].params.clone();
                kini = Some((b, self.isi_dari_params(ip, &params)?));
            }
            let Some((b, mut isi)) = kini.take() else { continue };
            if isi.len() != self.keadaan(ip).len() { return Err(format!("panjang stack di ip {ip} tidak cocok")); }
            // Posisi maya yang tidak boleh melewati instruksi ini diwujudkan ke stack VM dulu.
            for (p, j) in self.r.wujud[ip].clone() {
                match j {
                    Jenis::Salinan(l) => self.dorong(b, Inst::Muat { lokal: l as usize, ke: p, pindah: false }),
                    j => {
                        let k = j.konstanta().ok_or("wujud bukan salinan atau konstanta")? as usize;
                        self.dorong(b, Inst::Taruh { pos: p, alamat: &self.f.chunk.constants[k] as *const _ as i64 });
                    }
                }
                isi[p] = None;
            }
            if self.acak {
                let u = self.j.nilai_baru(Tipe::I);
                self.dorong(b, Inst::Undi { hasil: u });
                let jaga = Jaga { keadaan: self.keadaan_di(ip, &isi), spek: false, puncak_vm: false };
                self.dorong(b, Inst::JagaJika { kondisi: u, jaga });
            }
            if self.r.keluar[ip] {
                self.akhiri(b, Akhir::Keluar { keadaan: self.keadaan_di(ip, &isi) });
                continue;
            }
            match self.cara(b, ip, &mut isi)? {
                Some(akhir) => self.akhiri(b, akhir),
                None => {
                    // Keadaan yang dibawa tepinya (sebelum titik temu melebarkannya: `isi_tepi` yang menulis).
                    let lanjut = self.r.tepi.get(&(ip, ip + 1)).or_else(|| self.r.keadaan.get(ip + 1).and_then(|s| s.as_ref()))
                        .ok_or_else(|| format!("ip {} tak terjangkau sesudah ip {ip}", ip + 1))?.clone();
                    if lanjut.len() != isi.len() { return Err(format!("panjang stack sesudah ip {ip}: {} lawan {}", isi.len(), lanjut.len())); }
                    // Posisi tanpa isi native di keadaan berikutnya (lubang `AmbilLokal`, milik VM) tidak membawa nilai.
                    for (i, j) in lanjut.iter().enumerate() {
                        match (tipe_jenis(*j)?, isi[i]) {
                            (None, _) => isi[i] = None,
                            (Some(t), Some(v)) if self.j.tipe(v) == t => {}
                            _ => return Err(format!("posisi {i} sesudah ip {ip} tanpa isi native yang cocok")),
                        }
                    }
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
        Some(Jaga { keadaan: self.keadaan_di(ip, isi), spek, puncak_vm: false })
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

    /// Instruksi `ip` menurut `Rencana::cara`-nya; `Some` = pengakhir blok.
    fn cara(&mut self, b: Blok, ip: usize, isi: &mut Vec<Option<Nilai>>) -> Result<Option<Akhir>, String> {
        let ins = self.f.chunk.code[ip];
        let a = ins.arg as usize;
        let t = isi.len();
        let s = self.keadaan(ip).clone();
        match self.r.cara[ip].clone() {
            Cara::Asli => return self.instruksi(b, ip, isi, &s),
            Cara::Buka(daftar) => {
                // Nilai milik VM yang dipakai sebagai angka: dibuka berjaga (spekulasi), lalu instruksinya asli.
                let mut s2 = s.clone();
                for (p, j) in daftar {
                    let tipe = tipe_jenis(j)?.ok_or("buka ke jenis tanpa isi")?;
                    let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: true, puncak_vm: false };
                    let h = self.j.nilai_baru(tipe);
                    if tipe == Tipe::L { self.dorong(b, Inst::BukaLebar { hasil: h, pos: p, jaga }); }
                    else { self.dorong(b, Inst::Buka { hasil: h, pos: p, kode: j.kode() as i32, jaga }); }
                    isi[p] = Some(h);
                    s2[p] = j;
                }
                return self.instruksi(b, ip, isi, &s2);
            }
            Cara::Konstanta => {
                let alamat = match (ins.op, self.f.chunk.constants.get(a)) {
                    (OpCode::Const, Some(crate::vm::value::VmValue::NativeFn(_))) => { isi.push(None); return Ok(None); }
                    (OpCode::Const, Some(k)) => k as *const _ as i64,
                    _ => 0,
                };
                self.dorong(b, Inst::Taruh { pos: t, alamat });
                isi.push(None);
            }
            Cara::Muat => {
                // Lokal yang isinya salinan maya: lokal asalnya yang dibaca (disalin, tidak pernah dipindah).
                let (lokal, pindah) = match s[a] {
                    Jenis::Salinan(l) => (l as usize, false),
                    _ => (a, crate::jit::turun::mati_sesudah(self.f, self.r, ip, a)),
                };
                self.dorong(b, Inst::Muat { lokal, ke: t, pindah });
                isi.push(None);
            }
            Cara::Ambil => { self.dorong(b, Inst::Muat { lokal: a, ke: t, pindah: true }); isi.push(None); }
            Cara::Simpan => {
                let j = s[t - 1];
                self.lebar_ke_vm(b, t - 1, j, isi)?;
                let nilai = if j.skalar() { Some(Self::ambil(isi, t - 1)?) } else { None };
                let buang = ins.op == OpCode::SimpanBuang;
                self.dorong(b, Inst::Simpan { ke: a, dari: t - 1, nilai, kode: j.kode() as i32, buang });
                isi[a] = None;
                if buang { isi.pop(); }
            }
            Cara::Buang => { self.dorong(b, Inst::Buang { pos: t - 1 }); isi.pop(); }
            Cara::Benar => {
                let tetap = matches!(ins.op, OpCode::JumpFalseKeep | OpCode::JumpTrueKeep);
                let h = self.j.nilai_baru(Tipe::I);
                self.dorong(b, Inst::Benar { hasil: h, pos: t - 1, lepas: !tetap });
                if ins.op == OpCode::KeBool { isi[t - 1] = Some(h); return Ok(None); }
                let sesudah: Vec<Option<Nilai>> = if tetap { isi.clone() } else { isi[..t - 1].to_vec() };
                return self.cabang(b, ip, a, h, &sesudah, matches!(ins.op, OpCode::JumpFalse | OpCode::JumpFalseKeep)).map(Some);
            }
            Cara::KembaliNilai => {
                // Bulat 128 bit di register: ditulis ke stack VM dulu, lalu kembali sebagai milik VM.
                if let Some(Some(v)) = isi.get(t - 1) {
                    if self.j.tipe(*v) != Tipe::L { return Err("KembaliNilai atas isi native 64 bit".into()); }
                    self.dorong(b, Inst::TulisLebar { pos: t - 1, nilai: *v });
                }
                return Ok(Some(Akhir::KembaliNilai { pos: t - 1 }));
            }
            Cara::KembaliUnit => return Ok(Some(Akhir::KembaliUnit)),
            // `Typeof; Const; Eq` (dan `AmbilPayload` di depannya, atau `Len` sesudah `AmbilPayload`) jadi satu: hasilnya
            // di puncak, lanjut di `ip + lebar` (seperti penurun langsung).
            Cara::TipeLipat(ya) => {
                let v = self.konst_i(b, ya as i64);
                isi[t - 1] = Some(v);
                let (ke, args) = self.tepi(b, ip, ip + 3, isi)?;
                return Ok(Some(Akhir::Lompat { ke, args }));
            }
            Cara::TipeSama(ci) => {
                let alamat = &self.f.chunk.constants[ci] as *const _ as i64;
                let h = self.j.nilai_baru(Tipe::I);
                self.dorong(b, Inst::Pembantu { pb: Pb::JenisSama, args: vec![Arg::I64(t as i64 - 1), Arg::I64(alamat)], hasil: Some(h), gagal: None, keluar: None });
                isi[t - 1] = Some(h);
                let (ke, args) = self.tepi(b, ip, ip + 3, isi)?;
                return Ok(Some(Akhir::Lompat { ke, args }));
            }
            Cara::PayloadSama(l, ci) => {
                // 2 = bukan enum berpayload: deopt.
                let alamat = &self.f.chunk.constants[ci] as *const _ as i64;
                let h = self.j.nilai_baru(Tipe::I);
                let gagal = Some((Uji::Sama(2), Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false }));
                self.dorong(b, Inst::Pembantu { pb: Pb::PayloadJenisSama, args: vec![Arg::I64(l as i64), Arg::I64(alamat)], hasil: Some(h), gagal, keluar: None });
                isi[t - 1] = Some(h);
                let (ke, args) = self.tepi(b, ip, ip + 4, isi)?;
                return Ok(Some(Akhir::Lompat { ke, args }));
            }
            Cara::PayloadPanjang(l) => {
                let k = self.j.nilai_baru(Tipe::I);
                let gagal = Some((Uji::BukanNol, Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false }));
                self.dorong(b, Inst::Pembantu { pb: Pb::PayloadPanjang, args: vec![Arg::I64(l as i64), Arg::Wadah], hasil: None, gagal, keluar: Some(k) });
                isi[t - 1] = Some(k);
                let (ke, args) = self.tepi(b, ip, ip + 2, isi)?;
                return Ok(Some(Akhir::Lompat { ke, args }));
            }
            Cara::Langkah { ambil, taruh, tumpah } => {
                let (isi_n, taruh_n) = self.tumpahan(&s, t - tumpah..t, isi)?;
                let jenis = self.jenis_ip(ip);
                self.dorong(b, Inst::Langkah { ip, jenis, t, tumpah, sesudah: t - ambil + taruh, isi: isi_n, taruh: taruh_n });
                isi.truncate(t - ambil);
                for _ in 0..taruh { isi.push(None); }
            }
            Cara::Bawaan => {
                let fungsi = match self.f.chunk.constants.get(s[t - a - 1].konstanta().ok_or("callee bawaan bukan konstanta")? as usize) {
                    Some(crate::vm::value::VmValue::NativeFn(g)) => *g as usize as i64,
                    _ => return Err("callee bawaan bukan fungsi bawaan".into()),
                };
                let polos = if crate::jit::lambat::bawaan_polos(fungsi, a) { crate::jit::lambat::BAWAAN_POLOS } else { 0 };
                let (isi_n, taruh_n) = self.tumpahan(&s, t - a..t, isi)?;
                let jenis = self.jenis_ip(ip);
                self.dorong(b, Inst::Bawaan { ip, jenis, t, argc: a as i64 | polos, fungsi, isi: isi_n, taruh: taruh_n });
                isi.truncate(t - a - 1);
                isi.push(None);
                self.buka_hasil(b, ip, isi)?;
            }
            Cara::Biner => {
                let (isi_n, taruh_n) = self.tumpahan(&s, t - 2..t, isi)?;
                let banding = matches!(ins.op, OpCode::Eq | OpCode::Ne | OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge);
                let hasil = banding.then(|| self.j.nilai_baru(Tipe::I));
                let jenis = self.jenis_ip(ip);
                self.dorong(b, Inst::BinerVm { hasil, tambah: ins.op == OpCode::Add, ip, jenis, t, isi: isi_n, taruh: taruh_n });
                isi.truncate(t - 2);
                isi.push(hasil);
            }
            Cara::MuatUpval => {
                let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false };
                self.dorong(b, Inst::MuatUpval { idx: a, ke: t, jaga });
                isi.push(None);
                self.buka_hasil(b, ip, isi)?;
            }
            Cara::Bangun(n) => {
                use crate::jit::lambat::SitusBangun;
                use crate::vm::value::{Label, VmValue};
                let kode = &self.f.chunk.code;
                let situs = SitusBangun::bocor(match ins.op {
                    OpCode::BuildStruct => match self.f.chunk.constants.get(a) {
                        // Langsung dikembalikan: lokal bingkai ini yang sebentuk dan milik tunggal boleh dipakai ulang.
                        Some(VmValue::Map(p)) if kode.get(ip + 1).is_some_and(|i| i.op == OpCode::Return)
                            && matches!(self.r.cara.get(ip + 1), Some(Cara::KembaliNilai)) =>
                            SitusBangun::StrukturUlang(p.label.clone(), p.bentuk().ok_or("prototipe tanpa bentuk")?),
                        Some(VmValue::Map(p)) => SitusBangun::Struktur(p.label.clone(), p.bentuk().ok_or("prototipe tanpa bentuk")?),
                        _ => return Err("prototipe struct bukan peta".into()),
                    },
                    OpCode::BuildTuple => SitusBangun::Tuple,
                    OpCode::BuildEnum => match self.f.chunk.constants.get(a) {
                        Some(VmValue::Map(p)) => match &p.label {
                            Label::Enum { nama, .. } => SitusBangun::Varian(nama),
                            _ => return Err("prototipe varian bukan enum".into()),
                        },
                        _ => return Err("prototipe varian bukan peta".into()),
                    },
                    _ => SitusBangun::Larik,
                });
                let (isi_n, taruh_n) = self.tumpahan(&s, t - n..t, isi)?;
                let jenis = self.jenis_ip(ip);
                self.dorong(b, Inst::Bangun { situs: situs as *const _ as i64, jenis, t, n, isi: isi_n, taruh: taruh_n });
                isi.truncate(t - n);
                isi.push(None);
            }
            Cara::Salinan => match ins.op {
                // Salinan atau konstanta maya: cuma jenisnya yang berpindah.
                OpCode::LoadLocal | OpCode::LokalHitung | OpCode::Const => isi.push(None),
                OpCode::StoreLocal => isi[a] = None,
                OpCode::SimpanBuang => { isi[a] = None; isi.pop(); }
                op => return Err(format!("Cara::Salinan atas {op:?}")),
            },
            Cara::Langsung(n) => {
                let (dari, fungsi) = match s[t - a - 1] {
                    Jenis::Fungsi(c) => (t - a, &self.r.callee_nilai[c] as *const _ as i64),
                    _ => (t - a - 1, 0),
                };
                let (isi_n, taruh_n) = self.tumpahan(&s, dari..t, isi)?;
                let jenis_arg = self.r.langsung[n].1.clone();
                let mut args = Vec::new();
                for (k, j) in jenis_arg.iter().enumerate() {
                    if j.skalar() { args.push((k, Self::ambil(isi, t - a + k)?)); }
                }
                let mut sesudah: Vec<Option<Nilai>> = isi[..t - a - 1].to_vec();
                sesudah.push(None);
                let mut basi = self.keadaan_di(ip + 1, &sesudah);
                if let Some(p) = basi.isi.last_mut() { *p = None; }
                let hasil = match self.r.spek[ip] {
                    None => None,
                    Some(j @ (Jenis::Bulat | Jenis::Pecahan)) => {
                        let tipe = tipe_jenis(j)?.ok_or("spekulasi tanpa isi")?;
                        let h = self.j.nilai_baru(tipe);
                        Some((h, j.kode() as i32, Jaga { keadaan: basi.clone(), spek: true, puncak_vm: true }))
                    }
                    Some(j) => return Err(format!("hasil langsung berspekulasi {j:?}")),
                };
                let jenis = self.jenis_ip(ip);
                self.dorong(b, Inst::Langsung { n, ip, jenis, t, a, fungsi, isi: isi_n, taruh: taruh_n, args, basi, hasil: hasil.clone() });
                isi.truncate(t - a - 1);
                isi.push(hasil.map(|(h, ..)| h));
            }
            Cara::Panjang | Cara::PanjangLokal(_) => {
                let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false };
                let h = self.j.nilai_baru(Tipe::I);
                let (pos, lokal, hapus) = match self.r.cara[ip] {
                    // `len(lokal)`: posisi callee `len` dan salinannya diganti panjangnya.
                    Cara::PanjangLokal(l) => (l as usize, true, 2),
                    _ => (t - 1, false, 1),
                };
                self.dorong(b, Inst::Panjang { hasil: h, pos, lokal, jaga });
                isi.truncate(t - hapus);
                isi.push(Some(h));
            }
            Cara::Field => {
                let args = vec![Arg::I64(t as i64 - 1), Arg::I64(&self.f.chunk.names[a] as *const String as i64),
                    Arg::I32((ins.op == OpCode::GetFieldPrivat) as i32)];
                self.pembantu(b, ip, isi, Pb::Field, args, false, Some(Uji::BukanNol));
                self.buka_hasil(b, ip, isi)?;
            }
            Cara::Pola => {
                let situs = crate::jit::lambat::SitusPola::bocor(&self.f.chunk.constants[a]) as *const _ as i64;
                let (pb, args) = match s[t - 1] {
                    Jenis::Salinan(l) => (Pb::PolaLokal, vec![Arg::I64(l as i64), Arg::I64(situs)]),
                    _ => (Pb::Pola, vec![Arg::I64(t as i64 - 1), Arg::I64(situs)]),
                };
                // 0/1 = cocok tidaknya; 2 = jumlah sub-pola salah, VM yang melaporkannya.
                let h = self.pembantu(b, ip, isi, pb, args, true, Some(Uji::Sama(2)));
                isi[t - 1] = h;
            }
            Cara::Payload => {
                let (pb, args) = match s[t - 1] {
                    Jenis::Salinan(l) => (Pb::PayloadLokal, vec![Arg::I64(l as i64), Arg::I64(t as i64 - 1)]),
                    _ => (Pb::Payload, vec![Arg::I64(t as i64 - 1)]),
                };
                self.pembantu(b, ip, isi, pb, args, false, Some(Uji::BukanNol));
                isi[t - 1] = None;
                self.buka_hasil(b, ip, isi)?;
            }
            Cara::MuatGlobal => {
                let args = vec![Arg::I64(&self.f.chunk.names[a] as *const String as i64), Arg::I64(t as i64)];
                self.pembantu(b, ip, isi, Pb::MuatGlobal, args, false, Some(Uji::BukanNol));
                isi.push(None);
                self.buka_hasil(b, ip, isi)?;
            }
            Cara::SimpanGlobal => {
                let j = s[t - 1];
                self.lebar_ke_vm(b, t - 1, j, isi)?;
                let v = if j.skalar() { Arg::Isi(Self::ambil(isi, t - 1)?) } else { Arg::I64(0) };
                let args = vec![Arg::I64(&self.f.chunk.names[a] as *const String as i64), Arg::I64(t as i64 - 1), v, Arg::I32(j.kode() as i32)];
                self.pembantu(b, ip, isi, Pb::SimpanGlobal, args, false, Some(Uji::BukanNol));
            }
            Cara::Indeks => {
                let j = s[t - 1];
                self.lebar_ke_vm(b, t - 1, j, isi)?;
                let v = if j.skalar() { Arg::Isi(Self::ambil(isi, t - 1)?) } else { Arg::I64(0) };
                let args = vec![Arg::I64(t as i64 - 2), v, Arg::I32(j.kode() as i32)];
                self.pembantu(b, ip, isi, Pb::Indeks, args, false, Some(Uji::BukanNol));
                if j.di_vm() { self.dorong(b, Inst::Buang { pos: t - 1 }); }
                isi.truncate(t - 1);
                isi[t - 2] = None;
                self.buka_hasil(b, ip, isi)?;
            }
            Cara::Salin => { self.dorong(b, Inst::Salin { dari: t - 1, ke: t }); isi.push(None); }
            Cara::Tukar => {
                let (x, y) = (t - 2, t - 1);
                match (s[x].di_vm(), s[y].di_vm()) {
                    (true, true) => self.dorong(b, Inst::Tukar { x, y }),
                    (true, false) => self.dorong(b, Inst::Pindah { dari: x, ke: y }),
                    _ => self.dorong(b, Inst::Pindah { dari: y, ke: x }),
                }
                isi.swap(x, y);
            }
            Cara::Metode(n) => {
                let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: true, puncak_vm: false };
                self.dorong(b, Inst::CekMetode { pos: t - 1, label: &self.r.metode[n] as *const String as i64, jaga });
                isi[t - 1] = None;
            }
            Cara::IterAda => {
                let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false };
                let h = self.j.nilai_baru(Tipe::I);
                self.dorong(b, Inst::IterAda { hasil: h, slot: a, jaga });
                isi.push(Some(h));
            }
            Cara::Maju => {
                let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false };
                self.dorong(b, Inst::Maju { slot: a, jaga });
            }
            Cara::TeksBulat => {
                let n = Self::ambil(isi, t - 1)?;
                self.dorong(b, Inst::TeksBulat { pos: t - 2, n });
                isi.truncate(t - 2);
                isi.push(None);
            }
            Cara::Geser => {
                // `SlideDown` dengan nilai milik VM: yang tertimpa dilepas, puncaknya dipindah (atau isinya disalin).
                let d0 = t - a - 1;
                for p in d0..t - 1 { if s[p].di_vm() { self.dorong(b, Inst::Buang { pos: p }); } }
                if s[t - 1].di_vm() { self.dorong(b, Inst::Pindah { dari: t - 1, ke: d0 }); }
                isi[d0] = isi[t - 1];
                isi.truncate(d0 + 1);
            }
            Cara::Cabang { tumpah, hasil_bool, tujuan } => {
                let (isi_n, taruh_n) = self.tumpahan(&s, t - tumpah..t, isi)?;
                let jenis = self.jenis_ip(ip);
                // Posisi yang disentuh VM tidak punya isi native di tujuan mana pun (dijamin layak: hasilnya milik VM).
                let mut tj = Vec::new();
                for ke in tujuan {
                    let st = self.keadaan(ke).clone();
                    let mut sesudah: Vec<Option<Nilai>> = isi.iter().take(st.len()).copied().collect();
                    for (i, j) in st.iter().enumerate() {
                        if i >= t - tumpah && (j.skalar() || *j == Jenis::Lebar) { return Err("cabang VM ke posisi native yang disentuhnya".into()); }
                        if i >= sesudah.len() { sesudah.push(None); }
                    }
                    let (blok, args) = self.tepi(b, ip, ke, &sesudah)?;
                    tj.push((blok, args, ke));
                }
                let tujuan = [tj[0].clone(), tj[1].clone()];
                return Ok(Some(Akhir::CabangVm { ip, jenis, t, tumpah, hasil_bool, isi: isi_n, taruh: taruh_n, tujuan }));
            }
            Cara::SimpanUpval => {
                let j = s[t - 1];
                self.lebar_ke_vm(b, t - 1, j, isi)?;
                let nilai = if j.skalar() { Some(Self::ambil(isi, t - 1)?) } else { None };
                let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false };
                self.dorong(b, Inst::SimpanUpval { idx: a, dari: t - 1, nilai, kode: j.kode() as i32, jaga });
            }
            c => return Err(format!("{c:?} belum diturunkan JIR")),
        }
        Ok(None)
    }

    /// `Inst::Pembantu` dengan jaga di `ip` (keadaan sebelum instruksinya); `hasil` = statusnya jadi nilai bulat.
    #[allow(clippy::too_many_arguments)]
    fn pembantu(&mut self, b: Blok, ip: usize, isi: &[Option<Nilai>], pb: Pb, args: Vec<Arg>, hasil: bool, uji: Option<Uji>) -> Option<Nilai> {
        let gagal = uji.map(|u| (u, Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false }));
        let h = hasil.then(|| self.j.nilai_baru(Tipe::I));
        self.dorong(b, Inst::Pembantu { pb, args, hasil: h, gagal, keluar: None });
        h
    }

    /// Alamat `Rencana::keadaan[ip]` (dibaca `lambat::langkah`/`bawaan`; rencana hidup selama kodenya).
    fn jenis_ip(&self, ip: usize) -> i64 { self.keadaan(ip).as_ptr() as i64 }

    /// Posisi `rentang` yang dibaca pembantu dari stack VM: isi native ke slot rekaman, callee dan bawaan maya
    /// ditaruh dari alamatnya (seperti `tumpahkan` penurun langsung).
    #[allow(clippy::type_complexity)]
    fn tumpahan(&self, s: &[Jenis], rentang: std::ops::Range<usize>, isi: &[Option<Nilai>]) -> Result<(Vec<(usize, Nilai)>, Vec<(usize, i64)>), String> {
        let (mut isi_n, mut taruh_n) = (Vec::new(), Vec::new());
        for i in rentang {
            match s[i] {
                Jenis::Fungsi(c) => taruh_n.push((i, &self.r.callee_nilai[c] as *const _ as i64)),
                j if j.konstanta().is_some() => {
                    let k = j.konstanta().expect("baru diperiksa") as usize;
                    taruh_n.push((i, &self.f.chunk.constants[k] as *const _ as i64));
                }
                j if j.skalar() || j == Jenis::Lebar => isi_n.push((i, Self::ambil(isi, i)?)),
                _ => {}
            }
        }
        Ok((isi_n, taruh_n))
    }

    /// Hasil milik VM di puncak yang dispekulasikan angka (`Rencana::spek`): dibuka berjaga di `ip + 1` dengan puncak
    /// milik VM; gagal = deopt spekulasi.
    fn buka_hasil(&mut self, b: Blok, ip: usize, isi: &mut [Option<Nilai>]) -> Result<(), String> {
        let Some(j) = self.r.spek[ip] else { return Ok(()) };
        let tipe = tipe_jenis(j)?.ok_or("spekulasi hasil tanpa isi")?;
        let t = isi.len();
        let mut k = self.keadaan_di(ip + 1, isi);
        if let Some(p) = k.isi.last_mut() { *p = None; }
        let h = self.j.nilai_baru(tipe);
        let jaga = Jaga { keadaan: k, spek: true, puncak_vm: true };
        if tipe == Tipe::L { self.dorong(b, Inst::BukaLebar { hasil: h, pos: t - 1, jaga }); }
        else { self.dorong(b, Inst::Buka { hasil: h, pos: t - 1, kode: j.kode() as i32, jaga }); }
        isi[t - 1] = Some(h);
        Ok(())
    }

    /// Tujuan tepi `dari → ke` dengan stack `isi`. Kalau titik temu `ke` dilebarkan (`Rencana::tepi`: posisi native
    /// jadi milik VM), tepinya lewat blok stub yang menulis isinya ke stack VM dulu (seperti `isi_tepi`).
    fn tepi(&mut self, asal_b: Blok, dari: usize, ke: usize, isi: &[Option<Nilai>]) -> Result<(Blok, Vec<Nilai>), String> {
        let sasaran = self.blok_ip[ke].ok_or_else(|| format!("tujuan ip {ke} bukan pemimpin"))?;
        let tuju = self.keadaan(ke).clone();
        let asal = self.r.tepi.get(&(dari, ke)).cloned().unwrap_or_else(|| tuju.clone());
        let perlu: Vec<usize> = (0..tuju.len().min(asal.len())).filter(|&i| tuju[i].di_vm() && !asal[i].di_vm()).collect();
        if perlu.is_empty() { return Ok((sasaran, self.args_ke(asal_b, ke, isi)?)); }
        // Stub: parameternya isi native yang dibawa, nilai lain tidak boleh melintas blok.
        let (mut params, mut args) = (Vec::new(), Vec::new());
        let mut isi_s: Vec<Option<Nilai>> = Vec::new();
        for v in isi.iter().take(tuju.len()) {
            isi_s.push(match v {
                Some(v) => { let p = self.j.nilai_baru(self.j.tipe(*v)); params.push(p); args.push(*v); Some(p) }
                None => None,
            });
        }
        self.j.blok.push(super::DataBlok { ip: ke, params, inst: Vec::new(), akhir: None });
        let stub = Blok(self.j.blok.len() as u32 - 1);
        for i in perlu {
            match asal[i] {
                Jenis::Fungsi(c) => self.dorong(stub, Inst::Taruh { pos: i, alamat: &self.r.callee_nilai[c] as *const _ as i64 }),
                j if j.konstanta().is_some() => {
                    let k = j.konstanta().expect("baru diperiksa") as usize;
                    self.dorong(stub, Inst::Taruh { pos: i, alamat: &self.f.chunk.constants[k] as *const _ as i64 });
                }
                Jenis::Salinan(l) => self.dorong(stub, Inst::Muat { lokal: l as usize, ke: i, pindah: false }),
                Jenis::Lebar => {
                    let n = isi_s[i].ok_or("isi tepi lebar hilang")?;
                    self.dorong(stub, Inst::TulisLebar { pos: i, nilai: n });
                }
                j => {
                    let nilai = if j.skalar() { Some(isi_s[i].ok_or("isi tepi native hilang")?) } else { None };
                    self.dorong(stub, Inst::TulisVm { pos: i, nilai, kode: j.kode() as i32 });
                }
            }
            isi_s[i] = None;
        }
        let args_s = self.args_ke(stub, ke, &isi_s)?;
        self.akhiri(stub, Akhir::Lompat { ke: sasaran, args: args_s });
        Ok((stub, args))
    }

    /// Cabang atas kondisi `c` ke `ip + 1` dan `sasaran`, dengan stack `sesudah`.
    fn cabang(&mut self, b: Blok, ip: usize, sasaran: usize, c: Nilai, sesudah: &[Option<Nilai>], salah: bool) -> Result<Akhir, String> {
        let (b_lanjut, args_lanjut) = self.tepi(b, ip, ip + 1, sesudah)?;
        let (b_sasar, args_sasar) = self.tepi(b, ip, sasaran, sesudah)?;
        let (ya, ya_args, tidak, tidak_args) = if salah {
            (b_lanjut, args_lanjut, b_sasar, args_sasar)
        } else { (b_sasar, args_sasar, b_lanjut, args_lanjut) };
        Ok(Akhir::Cabang { kondisi: c, ya, ya_args, tidak, tidak_args })
    }

    /// Satu instruksi asli atas stack `isi` berjenis `s` (sesudah bukaan); `Some` = pengakhir blok.
    fn instruksi(&mut self, b: Blok, ip: usize, isi: &mut Vec<Option<Nilai>>, s: &[Jenis]) -> Result<Option<Akhir>, String> {
        let ins = self.f.chunk.code[ip];
        let a = ins.arg as usize;
        let s = s.to_vec();
        let t = isi.len();
        let pecahan = |k: usize| s.get(t.wrapping_sub(k)) == Some(&Jenis::Pecahan);
        if let Some(akhir) = self.lebar(b, ip, isi, &s)? { return Ok(akhir); }
        match ins.op {
            OpCode::LoadLocal | OpCode::LokalHitung | OpCode::AmbilLokal => {
                let v = *isi.get(a).ok_or("lokal di luar stack")?;
                isi.push(v);
            }
            OpCode::IntImm => { let v = self.konst_i(b, ins.arg as i32 as i64); isi.push(Some(v)); }
            OpCode::Const if matches!(self.f.chunk.constants.get(a), Some(crate::vm::value::VmValue::NativeFn(_))) => isi.push(None),
            OpCode::LoadGlobal => isi.push(None),
            OpCode::Call => return self.panggil(b, ip, isi, &s).map(|_| None),
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
                let (ke, args) = self.tepi(b, ip, a, isi)?;
                return Ok(Some(Akhir::Lompat { ke, args }));
            }
            OpCode::JumpFalse | OpCode::JumpTrue | OpCode::JumpFalseKeep | OpCode::JumpTrueKeep => {
                let c = Self::ambil(isi, t - 1)?;
                let tetap = matches!(ins.op, OpCode::JumpFalseKeep | OpCode::JumpTrueKeep);
                let sesudah: Vec<Option<Nilai>> = if tetap { isi.clone() } else { isi[..t - 1].to_vec() };
                return self.cabang(b, ip, a, c, &sesudah, matches!(ins.op, OpCode::JumpFalse | OpCode::JumpFalseKeep)).map(Some);
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
                let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false };
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
            OpCode::IndeksLokal | OpCode::GetIndex => {
                // `a[i]` atas larik lokal (atau salinan maya lokal): elemen bulat/pecahan dari `lambat::baca*`;
                // jenis lain = deopt spekulasi.
                let (lokal, tujuan) = match s.get(t.wrapping_sub(2)) {
                    Some(Jenis::Salinan(l)) if ins.op == OpCode::GetIndex => (*l as usize, t - 2),
                    _ if ins.op == OpCode::IndeksLokal => (a, t - 1),
                    _ => return Err("GetIndex asli bukan atas salinan".into()),
                };
                let i = Self::ambil(isi, t - 1)?;
                let pecahan = self.r.spek[ip] == Some(Jenis::Pecahan);
                let k = self.j.nilai_baru(if pecahan { Tipe::F } else { Tipe::I });
                let gagal = Some((Uji::BukanNol, Jaga { keadaan: self.keadaan_di(ip, isi), spek: true, puncak_vm: false }));
                let pb = if pecahan { Pb::BacaPecahan } else { Pb::Baca };
                self.dorong(b, Inst::Pembantu { pb, args: vec![Arg::I64(lokal as i64), Arg::Isi(i), Arg::Wadah], hasil: None, gagal, keluar: Some(k) });
                isi.truncate(tujuan);
                isi.push(Some(k));
            }
            OpCode::TulisIndexLokal => {
                let (v, i) = (Self::ambil(isi, t - 2)?, Self::ambil(isi, t - 1)?);
                let gagal = Some((Uji::BukanNol, Jaga { keadaan: self.keadaan_di(ip, isi), spek: true, puncak_vm: false }));
                let args = vec![Arg::I64(a as i64), Arg::Isi(i), Arg::Isi(v), Arg::I32(s[t - 2].kode() as i32)];
                self.dorong(b, Inst::Pembantu { pb: Pb::Tulis, args, hasil: None, gagal, keluar: None });
                isi.truncate(t - 2);
            }
            OpCode::Return => {
                let v = Self::ambil(isi, t - 1)?;
                if self.j.tipe(v) == Tipe::L || s[t - 1] == Jenis::Rune { return Err(format!("Return atas {:?}", s[t - 1])); }
                let hasil = match s[t - 1] { Jenis::Bool => Hasil::Bool, Jenis::Pecahan => Hasil::Pecahan, _ => Hasil::Bulat };
                return Ok(Some(Akhir::Kembali { nilai: v, hasil }));
            }
            op => return Err(format!("{op:?} belum diturunkan JIR")),
        }
        Ok(None)
    }

    /// Instruksi atas bulat 128 bit (`Jenis::Lebar`), aturan `turun::turun_lebar`: `Some` = sudah diturunkan.
    #[allow(clippy::type_complexity)]
    fn lebar(&mut self, b: Blok, ip: usize, isi: &mut Vec<Option<Nilai>>, s: &[Jenis]) -> Result<Option<Option<Akhir>>, String> {
        use Jenis::{Bulat, Lebar};
        let ins = self.f.chunk.code[ip];
        let t = isi.len();
        let dua = t >= 2 && matches!((s[t - 2], s[t - 1]), (Lebar, Bulat | Lebar) | (Bulat, Lebar));
        let op = match ins.op {
            OpCode::Const if self.r.konstanta[ip].is_none() => {
                let Some(v) = self.r.konstanta128[ip] else { return Ok(None) };
                let h = self.j.nilai_baru(Tipe::L);
                self.dorong(b, Inst::KonstL { hasil: h, nilai: v });
                isi.push(Some(h));
                return Ok(Some(None));
            }
            OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge | OpCode::Eq | OpCode::Ne if dua => OpL::Cmp(match ins.op {
                OpCode::Lt => Banding::Lt, OpCode::Le => Banding::Le, OpCode::Gt => Banding::Gt,
                OpCode::Ge => Banding::Ge, OpCode::Eq => Banding::Eq, _ => Banding::Ne,
            }),
            OpCode::Add | OpCode::Sub | OpCode::Mul if self.r.tepi.get(&(ip, ip + 1)).and_then(|st| st.last()) == Some(&Lebar) =>
                match ins.op { OpCode::Add => OpL::Tambah, OpCode::Sub => OpL::Kurang, _ => OpL::Kali },
            OpCode::Mod | OpCode::Div if dua => if ins.op == OpCode::Mod { OpL::Sisa } else { OpL::Bagi },
            OpCode::BitAnd | OpCode::BitOr | OpCode::BitXor | OpCode::AndNot if dua => match ins.op {
                OpCode::BitAnd => OpL::Dan, OpCode::BitOr => OpL::Atau, OpCode::BitXor => OpL::Xor, _ => OpL::DanBukan,
            },
            OpCode::Shl | OpCode::Shr if t >= 2 && s[t - 2] == Lebar && s[t - 1] == Bulat =>
                if ins.op == OpCode::Shl { OpL::GeserKiri } else { OpL::GeserKanan },
            OpCode::BitNot | OpCode::KeBool if t >= 1 && s[t - 1] == Lebar => {
                let x = Self::ambil(isi, t - 1)?;
                let (op, tipe) = if ins.op == OpCode::BitNot { (Unar::BukanBitL, Tipe::L) } else { (Unar::KeBoolL, Tipe::I) };
                let h = self.j.nilai_baru(tipe);
                self.dorong(b, Inst::Unar { hasil: h, op, a: x, jaga: None });
                isi[t - 1] = Some(h);
                return Ok(Some(None));
            }
            OpCode::JumpFalse | OpCode::JumpTrue | OpCode::JumpFalseKeep | OpCode::JumpTrueKeep if t >= 1 && s[t - 1] == Lebar => {
                let x = Self::ambil(isi, t - 1)?;
                let c = self.j.nilai_baru(Tipe::I);
                self.dorong(b, Inst::Unar { hasil: c, op: Unar::KeBoolL, a: x, jaga: None });
                let tetap = matches!(ins.op, OpCode::JumpFalseKeep | OpCode::JumpTrueKeep);
                let sesudah: Vec<Option<Nilai>> = if tetap { isi.clone() } else { isi[..t - 1].to_vec() };
                let salah = matches!(ins.op, OpCode::JumpFalse | OpCode::JumpFalseKeep);
                return self.cabang(b, ip, ins.arg as usize, c, &sesudah, salah).map(|a| Some(Some(a)));
            }
            _ => return Ok(None),
        };
        let (x, y) = (Self::ambil(isi, t - 2)?, Self::ambil(isi, t - 1)?);
        let geser = matches!(op, OpL::GeserKiri | OpL::GeserKanan);
        let x = self.ke_l(b, x);
        let y = if geser { y } else { self.ke_l(b, y) };
        let (jaga, jaga_muat) = match op {
            OpL::Tambah | OpL::Kurang | OpL::GeserKiri | OpL::GeserKanan => (self.jaga(ip, isi, false), None),
            OpL::Bagi | OpL::Sisa => (self.jaga(ip, isi, false), self.jaga(ip, isi, true)),
            _ => (None, None),
        };
        let tipe = if matches!(op, OpL::Cmp(_) | OpL::Bagi | OpL::Sisa) { Tipe::I } else { Tipe::L };
        let h = self.j.nilai_baru(tipe);
        self.dorong(b, Inst::BinerL { hasil: h, op, a: x, b: y, jaga, jaga_muat });
        isi.truncate(t - 2);
        isi.push(Some(h));
        Ok(Some(None))
    }

    /// Bulat 128 bit di posisi `pos` ditulis ke stack VM: pembantu yang menerima kode `Lebar` (6) membacanya dari sana.
    fn lebar_ke_vm(&mut self, b: Blok, pos: usize, j: Jenis, isi: &[Option<Nilai>]) -> Result<(), String> {
        if j == Jenis::Lebar { let v = Self::ambil(isi, pos)?; self.dorong(b, Inst::TulisLebar { pos, nilai: v }); }
        Ok(())
    }

    fn ke_l(&mut self, b: Blok, v: Nilai) -> Nilai {
        if self.j.tipe(v) == Tipe::L { return v; }
        let h = self.j.nilai_baru(Tipe::L);
        self.dorong(b, Inst::Unar { hasil: h, op: Unar::KeL, a: v, jaga: None });
        h
    }

    /// `Call`: callee bulat yang dikompilasi native, atau intrinsik `int`/`uint`/`float` atas bulat.
    fn panggil(&mut self, b: Blok, ip: usize, isi: &mut Vec<Option<Nilai>>, s: &[Jenis]) -> Result<(), String> {
        use crate::jit::layak::{BUNGKUS_KE_BULAT, BUNGKUS_KE_PECAHAN};
        let a = self.f.chunk.code[ip].arg as usize;
        let t = isi.len();
        if a + 1 > t { return Err("argumen panggilan di luar stack".into()); }
        if a == 2 && s[t - 3].wrap() {
            // `math::wrap_*`: modulo 2^64 atas 64 bit bawah, hasil tak bertanda di 128 bit.
            let Jenis::Bungkus(jenis, _) = s[t - 3] else { unreachable!() };
            let bawah = |me: &mut Self, i: usize| -> Result<Nilai, String> {
                let v = Self::ambil(isi, i)?;
                if me.j.tipe(v) != Tipe::L { return Ok(v); }
                let h = me.j.nilai_baru(Tipe::I);
                me.dorong(b, Inst::Unar { hasil: h, op: Unar::Rendah, a: v, jaga: None });
                Ok(h)
            };
            let (x, y) = (bawah(self, t - 2)?, bawah(self, t - 1)?);
            let op = match jenis {
                crate::jit::layak::BUNGKUS_TAMBAH => Biner::ITambah,
                crate::jit::layak::BUNGKUS_KURANG => Biner::IKurang,
                _ => Biner::IKali,
            };
            let w = self.biner(b, op, x, y, None);
            let h = self.j.nilai_baru(Tipe::L);
            self.dorong(b, Inst::Unar { hasil: h, op: Unar::KeLU, a: w, jaga: None });
            isi.truncate(t - 3);
            isi.push(Some(h));
            return Ok(());
        }
        if a == 2 && s[t - 3].bungkus(crate::jit::layak::BUNGKUS_CHAR_AT) && s[t - 1] == Jenis::Bulat {
            // `str::char_at` atas teks lokal (salinan maya) atau milik VM: kode karakternya; −1 (indeks negatif, di luar
            // batas, bukan teks) = deopt, VM yang menjawab atau melapor.
            let i = Self::ambil(isi, t - 1)?;
            let (pb, asal) = match s[t - 2] {
                Jenis::Salinan(l) => (Pb::KarakterLokal, l as i64),
                Jenis::Nilai => (Pb::Karakter, t as i64 - 2),
                lain => return Err(format!("char_at atas {lain:?}")),
            };
            let h = self.j.nilai_baru(Tipe::I);
            let gagal = Some((Uji::Negatif, Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false }));
            self.dorong(b, Inst::Pembantu { pb, args: vec![Arg::I64(asal), Arg::Isi(i)], hasil: Some(h), gagal, keluar: None });
            isi.truncate(t - 3);
            isi.push(Some(h));
            return Ok(());
        }
        if a == 1 && s[t - 1] == Jenis::Rune && matches!(s[t - 2], Jenis::Bungkus(BUNGKUS_KE_BULAT, _)) {
            // `int`/`uint` atas rune: kodenya sendiri.
            let v = Self::ambil(isi, t - 1)?;
            isi.truncate(t - 2);
            isi.push(Some(v));
            return Ok(());
        }
        match s[t - a - 1] {
            Jenis::Bungkus(BUNGKUS_KE_BULAT, k) if a == 1 && s[t - 1] == Jenis::Lebar => {
                let x = Self::ambil(isi, t - 1)?;
                let nama = match self.f.chunk.constants.get(k as usize) {
                    Some(crate::vm::value::VmValue::NativeFn(g)) => crate::vm::info_native::info(*g).map(|i| i.kanonik.clone()),
                    _ => None,
                };
                if nama.as_deref() == Some("uint") {
                    let nol = self.j.nilai_baru(Tipe::L);
                    self.dorong(b, Inst::KonstL { hasil: nol, nilai: 0 });
                    let neg = self.j.nilai_baru(Tipe::I);
                    self.dorong(b, Inst::BinerL { hasil: neg, op: OpL::Cmp(Banding::Lt), a: x, b: nol, jaga: None, jaga_muat: None });
                    let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false };
                    self.dorong(b, Inst::JagaJika { kondisi: neg, jaga });
                }
                let h = if self.keadaan(ip + 1).get(t - 2) == Some(&Jenis::Bulat) {
                    let h = self.j.nilai_baru(Tipe::I);
                    let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: true, puncak_vm: false };
                    self.dorong(b, Inst::Sempit { hasil: h, a: x, jaga });
                    h
                } else { x };
                isi.truncate(t - 2);
                isi.push(Some(h));
            }
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
                    let jaga = Jaga { keadaan: self.keadaan_di(ip, isi), spek: false, puncak_vm: false };
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
