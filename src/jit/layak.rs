//! Kelayakan JIT (`dokumentasi/jit/j4-rencana.md`): bytecode ditelusuri dengan stack ABSTRAK dari satu
//! titik masuk. Instruksi yang belum bisa dibuktikan jadi TITIK KELUAR: kode native deopt di sana dan VM
//! melanjutkan bingkainya, jadi fungsi apa pun bisa dikompilasi sebagian.

use std::sync::Arc;

use crate::vm::opcode::OpCode;
use crate::vm::value::{VmFunction, VmValue};

/// Jenis satu posisi stack di kode yang dikompilasi.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Jenis {
    Bulat,
    Bool,
    /// `f64`; bitnya disimpan di variabel `i64`.
    Pecahan,
    /// Rujukan ke `Rencana::callee[i]` (hasil `LoadGlobal`, cuma boleh dipanggil).
    Fungsi(usize),
    /// `nil` (juga lubang yang ditinggal `AmbilLokal`).
    Kosong,
    /// Nilai lain, milik bingkai VM: tetap di stack VM di indeks yang sama, cuma disentuh lewat `lambat`.
    Nilai,
    /// `Nilai` yang pasti fungsi bawaan (`Const`), jadi `Call` atasnya tidak pernah mendorong bingkai.
    Bawaan,
}

impl Jenis {
    /// Isinya ada di stack VM, bukan di variabel native.
    pub fn di_vm(self) -> bool { matches!(self, Jenis::Nilai | Jenis::Bawaan) }
}

/// Cara satu instruksi dijalankan kode native (`Rencana::cara`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cara {
    Asli,
    /// Posisi `Nilai` ini dibuka jadi bulat/pecahan (jaga), lalu instruksinya asli.
    Buka(Vec<(usize, Jenis)>),
    /// VM menjalankan instruksinya (`lambat::langkah`): `ambil` operand teratas diganti `taruh` nilai.
    Langkah { ambil: usize, taruh: usize },
    /// `LoadLocal` atas lokal `Nilai` (`lambat::muat`).
    Muat,
    /// `Const` bukan angka dan `Unit` (`lambat::taruh`).
    Konstanta,
    /// `AmbilLokal` atas lokal `Nilai` (`lambat::ambil`).
    Ambil,
    /// `StoreLocal`/`SimpanBuang` yang melibatkan nilai milik VM (`lambat::simpan`).
    Simpan,
    /// Truthiness `Nilai` di puncak (`lambat::benar`), untuk lompat bersyarat dan `KeBool`.
    Benar,
    /// `Pop` atas `Nilai`: nilainya dilepas (`lambat::buang`).
    Buang,
    /// `Return` atas `Nilai` (masuk dari VM): VM mengambilnya dari stack-nya.
    KembaliNilai,
    /// `ReturnNil` (masuk dari VM).
    KembaliUnit,
}

pub const KEMBALI_TIDAK: u8 = 0;
pub const KEMBALI_BULAT: u8 = 1;
pub const KEMBALI_SKALAR: u8 = 2;

/// Jenis nilai VM di titik masuk.
pub fn jenis_nilai(v: &VmValue) -> Jenis {
    match v {
        VmValue::Int(n) if i64::try_from(*n).is_ok() => Jenis::Bulat,
        VmValue::Bool(_) => Jenis::Bool,
        VmValue::Float(_) => Jenis::Pecahan,
        VmValue::Nil => Jenis::Kosong,
        _ => Jenis::Nilai,
    }
}

#[derive(Debug)]
pub struct Rencana {
    /// Instruksi tempat kode native mulai (0 = panggilan, selain itu kepala loop lewat OSR).
    pub masuk: usize,
    pub jenis_masuk: Vec<Jenis>,
    /// `Return` native: `KEMBALI_TIDAK` (OSR; VM yang mengembalikan), `KEMBALI_BULAT` (callee native),
    /// `KEMBALI_SKALAR` (masuk dari VM: bulat, bool atau pecahan).
    pub kembali: u8,
    /// Stack abstrak SEBELUM tiap instruksi; `None` = tak terjangkau.
    pub keadaan: Vec<Option<Vec<Jenis>>>,
    /// Instruksi yang dijalankan VM: kode native deopt tepat sebelumnya.
    pub keluar: Vec<bool>,
    /// Fungsi yang dipanggil (dirinya sendiri juga, kalau rekursif).
    pub callee: Vec<Arc<VmFunction>>,
    /// Instruksi yang memulai blok dasar.
    pub pemimpin: Vec<bool>,
    /// Kedalaman stack terbesar (jumlah variabel yang dibutuhkan).
    pub maks: usize,
    /// Nilai `Const` bulat (atau bit pecahan) per instruksi.
    pub konstanta: Vec<Option<i64>>,
    /// Cara tiap instruksi yang bukan titik keluar dijalankan.
    pub cara: Vec<Cara>,
}

/// Sifat fungsi yang membuatnya bisa dipanggil langsung dari kode native (bingkainya disusun ulang
/// `deopt::pulihkan` tanpa closure dan dengan argumen tepat sebanyak aritasnya).
pub fn bentuk_layak(f: &VmFunction) -> bool {
    !f.is_closure && !f.variadic && f.arity_min == f.arity && f.upvals.is_empty()
        && f.chunk.tipe_self.is_none()
}

/// Rencana dari `masuk` dengan stack `jenis_masuk`, atau alasan tidak ada yang bisa dikompilasi.
pub fn analisis(
    f: &VmFunction, cari: &dyn Fn(&str) -> Option<Arc<VmFunction>>,
    masuk: usize, jenis_masuk: Vec<Jenis>, kembali: u8,
) -> Result<Rencana, String> {
    let kode = &f.chunk.code;
    let n = kode.len();
    if masuk >= n { return Err("kode kosong".into()); }
    let mut r = Rencana {
        masuk, jenis_masuk: jenis_masuk.clone(), kembali,
        keadaan: vec![None; n], keluar: vec![false; n], callee: Vec::new(),
        pemimpin: vec![false; n], maks: jenis_masuk.len(), konstanta: vec![None; n],
        cara: vec![Cara::Asli; n],
    };
    r.keadaan[masuk] = Some(jenis_masuk);
    let mut kerja = vec![masuk];
    while let Some(ip) = kerja.pop() {
        let s = r.keadaan[ip].clone().expect("diisi saat masuk antrean");
        // Penerus yang keadaannya bentrok dengan kedatangan sebelumnya: instruksi ini dijalankan VM.
        let lanjut = langkah(f, cari, &mut r, ip, s)
            .filter(|ps| ps.iter().all(|(t, st)| *t < n && r.keadaan[*t].as_ref().is_none_or(|a| a == st)));
        let Some(ps) = lanjut else { r.keluar[ip] = true; continue };
        for (t, st) in ps {
            r.maks = r.maks.max(st.len());
            if r.keadaan[t].is_none() {
                r.keadaan[t] = Some(st);
                kerja.push(t);
            }
        }
    }
    if r.keluar[masuk] {
        return Err(format!("{:?} di ip {masuk} belum diturunkan", kode[masuk].op));
    }
    // Kode yang pasti keluar sebelum mengulang loop atau kembali cuma menambah ongkos masuk-keluar.
    let berguna = (0..n).any(|ip| r.keadaan[ip].is_some() && !r.keluar[ip] && match kode[ip].op {
        OpCode::Jump => (kode[ip].arg as usize) <= ip,
        OpCode::Return | OpCode::ReturnNil => true,
        _ => false,
    });
    if !berguna {
        let ip = (0..n).find(|&i| r.keluar[i]).unwrap_or(masuk);
        return Err(format!("keluar di {:?} (ip {ip}) sebelum loop atau kembali", kode[ip].op));
    }
    // Satu langkah VM dari kode native berongkos ±10 instruksi VM biasa (masuk loop VM, menyalin
    // operand); kode yang kebanyakan langkah lebih lambat dari VM sendiri.
    let jalan = |ip: usize| r.keadaan[ip].is_some() && !r.keluar[ip];
    let asli = (0..n).filter(|&ip| jalan(ip) && matches!(r.cara[ip], Cara::Asli | Cara::Buka(_))).count();
    let langkah_vm = (0..n).filter(|&ip| jalan(ip) && matches!(r.cara[ip], Cara::Langkah { .. })).count();
    if langkah_vm * 4 > asli {
        return Err(format!("{langkah_vm} instruksi dijalankan VM, cuma {asli} native"));
    }
    // Blok dasar: titik masuk, sasaran lompat, dan instruksi yang tidak didatangi dari pendahulunya.
    r.pemimpin[masuk] = true;
    for ip in 0..n {
        if r.keadaan[ip].is_none() { continue; }
        let ins = kode[ip];
        if matches!(ins.op, OpCode::Jump | OpCode::JumpFalse | OpCode::JumpTrue
            | OpCode::JumpFalseKeep | OpCode::JumpTrueKeep) && !r.keluar[ip] {
            r.pemimpin[ins.arg as usize] = true;
        }
        let jatuh = ip > 0 && r.keadaan[ip - 1].is_some() && !r.keluar[ip - 1]
            && !matches!(kode[ip - 1].op, OpCode::Jump | OpCode::Return | OpCode::ReturnNil | OpCode::JumpFalse
                | OpCode::JumpTrue | OpCode::JumpFalseKeep | OpCode::JumpTrueKeep);
        if !jatuh { r.pemimpin[ip] = true; }
    }
    // `JumpFalse` dkk. bercabang ke instruksi sesudahnya juga.
    for ip in 0..n.saturating_sub(1) {
        if r.keadaan[ip].is_some() && !r.keluar[ip] && matches!(kode[ip].op,
            OpCode::JumpFalse | OpCode::JumpTrue | OpCode::JumpFalseKeep | OpCode::JumpTrueKeep) {
            r.pemimpin[ip + 1] = true;
        }
    }
    r.maks += 1;
    Ok(r)
}

/// Penerus instruksi `ip` dengan stack sebelum `s` (dan `r.cara[ip]`); `None` = titik keluar.
fn langkah(
    f: &VmFunction, cari: &dyn Fn(&str) -> Option<Arc<VmFunction>>,
    r: &mut Rencana, ip: usize, s: Vec<Jenis>,
) -> Option<Vec<(usize, Vec<Jenis>)>> {
    // `langkah_asli` boleh memilih cara lain (`ReturnNil`).
    r.cara[ip] = Cara::Asli;
    if let Some(p) = langkah_asli(f, cari, r, ip, s.clone()) { return Some(p); }
    // Nilai milik VM cuma ada di bingkai terluar (`lambat` memakai basis bingkai itu).
    if r.kembali == KEMBALI_BULAT { return None; }
    let (cara, p) = langkah_lambat(f, cari, r, ip, s)?;
    r.cara[ip] = cara;
    Some(p)
}

/// Jumlah operand teratas yang dibaca instruksi yang boleh dibuka (`Cara::Buka`).
fn operand_angka(op: OpCode, arg: usize) -> usize {
    match op {
        OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod | OpCode::Lt | OpCode::Le
        | OpCode::Gt | OpCode::Ge | OpCode::Eq | OpCode::Ne | OpCode::MakeIterRentang => 2,
        OpCode::Neg | OpCode::IndeksLokal => 1,
        // Cuma indeksnya; nilai yang ditulis boleh apa saja (`Cara::Langkah`).
        OpCode::TulisIndexLokal => 1,
        OpCode::Call => arg,
        _ => 0,
    }
}

/// Bentuk lambat: membuka `Nilai` jadi angka, atau menyerahkan instruksinya ke VM.
fn langkah_lambat(
    f: &VmFunction, cari: &dyn Fn(&str) -> Option<Arc<VmFunction>>,
    r: &mut Rencana, ip: usize, mut s: Vec<Jenis>,
) -> Option<(Cara, Vec<(usize, Vec<Jenis>)>)> {
    use Jenis::*;
    let kode = &f.chunk.code;
    let ins = kode[ip];
    let a = ins.arg as usize;
    let t = s.len();
    let lanjut = ip + 1;
    // Operand `Nilai` di samping angka dibuka ke jenis angkanya (pecahan kalau ada pecahan).
    let k = operand_angka(ins.op, a).min(t);
    let atas = &s[t - k..];
    let indeks = matches!(ins.op, OpCode::IndeksLokal | OpCode::TulisIndexLokal);
    if atas.contains(&Nilai) && (indeks || atas.iter().any(|j| matches!(j, Bulat | Pecahan))) {
        let sasaran = if atas.contains(&Pecahan) && ins.op != OpCode::Call { Pecahan } else { Bulat };
        let buka: Vec<(usize, Jenis)> = (t - k..t).filter(|&i| s[i] == Nilai).map(|i| (i, sasaran)).collect();
        let mut s2 = s.clone();
        for (i, j) in &buka { s2[*i] = *j; }
        if let Some(p) = langkah_asli(f, cari, r, ip, s2) { return Some((Cara::Buka(buka), p)); }
    }
    let puncak = *s.last()?;
    match ins.op {
        OpCode::LoadLocal | OpCode::LokalHitung if s.get(a)?.di_vm() => {
            s.push(Nilai);
            return Some((Cara::Muat, vec![(lanjut, s)]));
        }
        OpCode::Pop if puncak.di_vm() => {
            s.pop();
            return Some((Cara::Buang, vec![(lanjut, s)]));
        }
        OpCode::KeBool if puncak.di_vm() => {
            s.pop();
            s.push(Bool);
            return Some((Cara::Benar, vec![(lanjut, s)]));
        }
        OpCode::JumpFalse | OpCode::JumpTrue if puncak.di_vm() => {
            s.pop();
            return Some((Cara::Benar, vec![(a, s.clone()), (lanjut, s)]));
        }
        OpCode::JumpFalseKeep | OpCode::JumpTrueKeep if puncak.di_vm() => {
            return Some((Cara::Benar, vec![(a, s.clone()), (lanjut, s)]));
        }
        OpCode::Return if r.kembali == KEMBALI_SKALAR && puncak.di_vm() => return Some((Cara::KembaliNilai, vec![])),
        OpCode::Const | OpCode::Unit => {
            let bawaan = ins.op == OpCode::Const && matches!(f.chunk.constants.get(a)?, VmValue::NativeFn(_));
            s.push(if bawaan { Bawaan } else { Nilai });
            return Some((Cara::Konstanta, vec![(lanjut, s)]));
        }
        OpCode::AmbilLokal if s.get(a)?.di_vm() => {
            s[a] = Nilai;
            s.push(Nilai);
            return Some((Cara::Ambil, vec![(lanjut, s)]));
        }
        // Lokal milik VM bisa sel tangkapan: penulisannya menembus sel (`Vm::simpan_lokal`).
        OpCode::StoreLocal | OpCode::SimpanBuang
            if a + 1 < t && (s[a].di_vm() || puncak.di_vm()) && !matches!(puncak, Fungsi(_)) => {
            s[a] = Nilai;
            if ins.op == OpCode::SimpanBuang { s.pop(); }
            return Some((Cara::Simpan, vec![(lanjut, s)]));
        }
        _ => {}
    }
    // Sisanya dijalankan VM. Operand fungsi milik kode native tidak punya padanan di stack VM.
    let (ambil, taruh, lokal) = efek_langkah(kode, ip, &s)?;
    if ambil > t || s[t - ambil..].iter().any(|j| matches!(j, Fungsi(_))) { return None; }
    s.truncate(t - ambil);
    s.extend(std::iter::repeat_n(Nilai, taruh));
    if let Some(l) = lokal { *s.get_mut(l)? = Nilai; }
    Some((Cara::Langkah { ambil, taruh }, vec![(lanjut, s)]))
}

/// Instruksi yang aman dijalankan VM satu per satu dari dalam kode native: `(ambil, taruh, lokal
/// yang ditulis)`. Tidak pernah mendorong bingkai, melompat, atau bergabung dengan instruksi sesudahnya.
fn efek_langkah(kode: &[crate::vm::opcode::Instruction], ip: usize, s: &[Jenis]) -> Option<(usize, usize, Option<usize>)> {
    let ins = kode[ip];
    let a = ins.arg as usize;
    let t = s.len();
    let ada_vm = |k: usize| k <= t && s[t - k..].iter().any(|j| j.di_vm());
    // `int_cepat` VM menggabungkan banding dua bulat dengan `JumpFalse` sesudahnya.
    let tanpa_lompat = kode.get(ip + 1).is_none_or(|i| i.op != OpCode::JumpFalse);
    Some(match ins.op {
        OpCode::LoadFn => (0, 1, None),
        OpCode::PinjamLokal if s.get(a)?.di_vm() => (0, 1, Some(a)),
        OpCode::Dup if ada_vm(1) => (1, 2, None),
        OpCode::Swap if ada_vm(2) => (2, 2, None),
        OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod | OpCode::Pow
        | OpCode::BitAnd | OpCode::BitOr | OpCode::BitXor | OpCode::Shl | OpCode::Shr | OpCode::AndNot
        | OpCode::Concat if ada_vm(2) => (2, 1, None),
        OpCode::Eq | OpCode::Ne | OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge
            if ada_vm(2) && tanpa_lompat => (2, 1, None),
        OpCode::Neg | OpCode::Not | OpCode::BitNot | OpCode::Len | OpCode::Typeof | OpCode::CopyVal
            if ada_vm(1) => (1, 1, None),
        OpCode::GetIndex if ada_vm(2) => (2, 1, None),
        OpCode::IndeksLokal if s.get(a)?.di_vm() => (1, 1, None),
        OpCode::TulisIndexLokal if s.get(a)?.di_vm() => (2, 0, Some(a)),
        OpCode::SetIndex if ada_vm(3) => (3, 0, None),
        OpCode::Slice if ada_vm(3) => (3, 1, None),
        OpCode::GetField | OpCode::GetFieldPrivat if ada_vm(1) => (1, 1, None),
        OpCode::BuildArray | OpCode::BuildTuple => (a, 1, None),
        // Fungsi bawaan tidak pernah mendorong bingkai (`Vm::call_value`, cabang `NativeFn`).
        OpCode::Call if a < t && s[t - a - 1] == Jenis::Bawaan => (a + 1, 1, None),
        _ => return None,
    })
}

/// Penerus instruksi `ip` kalau semua operandnya milik kode native; `None` = bukan bentuk asli.
fn langkah_asli(
    f: &VmFunction, cari: &dyn Fn(&str) -> Option<Arc<VmFunction>>,
    r: &mut Rencana, ip: usize, mut s: Vec<Jenis>,
) -> Option<Vec<(usize, Vec<Jenis>)>> {
    use Jenis::*;
    let ins = f.chunk.code[ip];
    let a = ins.arg as usize;
    let skalar = |j: Jenis| matches!(j, Bulat | Bool | Pecahan | Kosong);
    let angka = |j: Jenis| matches!(j, Bulat | Pecahan);
    // Dua angka: bulat dengan bulat tetap bulat, selain itu pecahan (aturan `VmValue::add` dkk.).
    let angka2 = |s: &mut Vec<Jenis>| -> Option<Jenis> {
        let y = s.pop()?; let x = s.pop()?;
        if !angka(x) || !angka(y) { return None; }
        Some(if x == Bulat && y == Bulat { Bulat } else { Pecahan })
    };
    let bulat2 = |s: &mut Vec<Jenis>| -> Option<()> {
        let y = s.pop()?; let x = s.pop()?;
        (x == Bulat && y == Bulat).then_some(())
    };
    let lanjut = ip + 1;
    match ins.op {
        OpCode::LoadLocal | OpCode::LokalHitung => {
            let j = *s.get(a)?;
            if !skalar(j) { return None; }
            s.push(j);
        }
        OpCode::AmbilLokal => {
            let j = *s.get(a)?;
            if !skalar(j) { return None; }
            s[a] = Kosong;
            s.push(j);
        }
        // Larik milik VM di lokal `a`, lewat `lambat::baca`/`tulis` (bukan larik bulat = titik keluar saat jalan).
        OpCode::IndeksLokal => {
            if s.pop()? != Bulat || *s.get(a)? != Nilai { return None; }
            s.push(Bulat);
        }
        OpCode::TulisIndexLokal => {
            let (i, v) = (s.pop()?, s.pop()?);
            if i != Bulat || !matches!(v, Bulat | Bool | Pecahan) || *s.get(a)? != Nilai { return None; }
        }
        OpCode::IntImm => s.push(Bulat),
        OpCode::True | OpCode::False => s.push(Bool),
        OpCode::Nil => s.push(Kosong),
        OpCode::Const => match f.chunk.constants.get(a)? {
            VmValue::Int(v) => { r.konstanta[ip] = Some(i64::try_from(*v).ok()?); s.push(Bulat) }
            VmValue::Float(x) => { r.konstanta[ip] = Some(x.to_bits() as i64); s.push(Pecahan) }
            _ => return None,
        },
        OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div => { let j = angka2(&mut s)?; s.push(j) }
        // `%` pecahan adalah `fmod`, yang tidak punya instruksi mesin: VM yang menghitungnya.
        OpCode::Mod => { bulat2(&mut s)?; s.push(Bulat) }
        OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge => { angka2(&mut s)?; s.push(Bool) }
        OpCode::Eq | OpCode::Ne => {
            let t = s.len();
            if t >= 2 && s[t - 1] == Bool && s[t - 2] == Bool { s.truncate(t - 2); } else { angka2(&mut s)?; }
            s.push(Bool)
        }
        OpCode::Neg => if !angka(*s.last()?) { return None },
        OpCode::Not => if *s.last()? != Bool { return None },
        OpCode::KeBool => {
            if !matches!(s.pop()?, Bulat | Bool) { return None; }
            s.push(Bool);
        }
        // Nilai milik VM yang dibuang dilepas lewat `Cara::Buang`.
        OpCode::Pop => if s.pop()?.di_vm() { return None },
        OpCode::Dup => {
            let j = *s.last()?;
            if j.di_vm() { return None; }
            s.push(j);
        }
        OpCode::Swap => {
            let t = s.len();
            if t < 2 || s[t - 1].di_vm() || s[t - 2].di_vm() { return None; }
            s.swap(t - 1, t - 2);
        }
        OpCode::StoreLocal | OpCode::SimpanBuang => {
            let v = *s.last()?;
            // Lokal `Nilai` bisa berupa sel tangkapan closure: penulisannya milik VM.
            if !skalar(v) || !skalar(*s.get(a)?) || a + 1 >= s.len() { return None; }
            s[a] = v;
            if ins.op == OpCode::SimpanBuang { s.pop(); }
        }
        OpCode::Jump => return Some(vec![(a, s)]),
        OpCode::JumpFalse | OpCode::JumpTrue => {
            if !matches!(s.pop()?, Bulat | Bool) { return None; }
            return Some(vec![(a, s.clone()), (lanjut, s)]);
        }
        OpCode::JumpFalseKeep | OpCode::JumpTrueKeep => {
            if !matches!(*s.last()?, Bulat | Bool) { return None; }
            return Some(vec![(a, s.clone()), (lanjut, s)]);
        }
        OpCode::Return => {
            let ok = match r.kembali {
                KEMBALI_BULAT => *s.last()? == Bulat,
                KEMBALI_SKALAR => matches!(*s.last()?, Bulat | Bool | Pecahan),
                _ => false,
            };
            if !ok { return None; }
            return Some(vec![]);
        }
        OpCode::ReturnNil if r.kembali == KEMBALI_SKALAR => {
            r.cara[ip] = Cara::KembaliUnit;
            return Some(vec![]);
        }
        OpCode::LoadGlobal => {
            let g = cari(f.chunk.names.get(a)?)?;
            if !bentuk_layak(&g) { return None; }
            let i = match r.callee.iter().position(|c| Arc::ptr_eq(c, &g)) {
                Some(i) => i,
                None => { r.callee.push(g); r.callee.len() - 1 }
            };
            s.push(Fungsi(i));
        }
        OpCode::Call => {
            if s.len() < a + 1 { return None; }
            let Fungsi(i) = s[s.len() - a - 1] else { return None };
            if r.callee[i].arity as usize != a { return None; }
            for _ in 0..a { if s.pop()? != Bulat { return None; } }
            s.pop();
            s.push(Bulat);
        }
        // `for` atas rentang bulat: [jumlah, awal, indeks] (`MakeIterRentang` VM).
        OpCode::MakeIterRentang => {
            bulat2(&mut s)?;
            s.extend([Bulat, Bulat, Bulat]);
        }
        OpCode::IterAda => {
            if *s.get(a)? != Bulat || *s.get(a + 1)? != Bulat || *s.get(a + 2)? != Bulat { return None; }
            s.push(Bool);
        }
        OpCode::IterMaju => if *s.get(a)? != Bulat { return None },
        OpCode::IterVar => {
            let k = a / 2;
            if *s.get(k)? != Bulat || *s.get(k + 1)? != Bulat || *s.get(k + 2)? != Bulat { return None; }
            s.push(Bulat);
        }
        _ => return None,
    }
    Some(vec![(lanjut, s)])
}
