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
    /// Rujukan ke `Rencana::callee[i]` (hasil `LoadGlobal`, cuma boleh dipanggil).
    Fungsi(usize),
    /// `nil` (juga lubang yang ditinggal `AmbilLokal`).
    Kosong,
    /// Nilai lain milik bingkai VM di titik masuk; tetap di stack VM, tidak pernah disentuh kode native.
    Nilai,
}

pub const KEMBALI_TIDAK: u8 = 0;
pub const KEMBALI_BULAT: u8 = 1;
pub const KEMBALI_SKALAR: u8 = 2;

/// Jenis nilai VM di titik masuk.
pub fn jenis_nilai(v: &VmValue) -> Jenis {
    match v {
        VmValue::Int(n) if i64::try_from(*n).is_ok() => Jenis::Bulat,
        VmValue::Bool(_) => Jenis::Bool,
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
    /// `KEMBALI_SKALAR` (masuk dari VM: bulat atau bool).
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
    /// Nilai `Const` bulat per instruksi.
    pub konstanta: Vec<Option<i64>>,
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
        OpCode::Return => true,
        _ => false,
    });
    if !berguna {
        let ip = (0..n).find(|&i| r.keluar[i]).unwrap_or(masuk);
        return Err(format!("keluar di {:?} (ip {ip}) sebelum loop atau kembali", kode[ip].op));
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
            && !matches!(kode[ip - 1].op, OpCode::Jump | OpCode::Return | OpCode::JumpFalse
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

/// Penerus instruksi `ip` dengan stack sebelum `s`; `None` = titik keluar.
fn langkah(
    f: &VmFunction, cari: &dyn Fn(&str) -> Option<Arc<VmFunction>>,
    r: &mut Rencana, ip: usize, mut s: Vec<Jenis>,
) -> Option<Vec<(usize, Vec<Jenis>)>> {
    use Jenis::*;
    let ins = f.chunk.code[ip];
    let a = ins.arg as usize;
    let skalar = |j: Jenis| matches!(j, Bulat | Bool | Kosong);
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
        OpCode::IntImm => s.push(Bulat),
        OpCode::True | OpCode::False => s.push(Bool),
        OpCode::Nil => s.push(Kosong),
        OpCode::Const => match f.chunk.constants.get(a)? {
            VmValue::Int(v) => { r.konstanta[ip] = Some(i64::try_from(*v).ok()?); s.push(Bulat) }
            _ => return None,
        },
        OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Mod => { bulat2(&mut s)?; s.push(Bulat) }
        OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge | OpCode::Eq | OpCode::Ne => {
            bulat2(&mut s)?; s.push(Bool)
        }
        OpCode::Neg => if *s.last()? != Bulat { return None },
        OpCode::Not => if *s.last()? != Bool { return None },
        OpCode::KeBool => {
            if !matches!(s.pop()?, Bulat | Bool) { return None; }
            s.push(Bool);
        }
        // Nilai yang dibuang tetap di stack VM sampai bingkainya dipulihkan atau selesai.
        OpCode::Pop => { s.pop()?; }
        OpCode::Dup => {
            let j = *s.last()?;
            if j == Nilai { return None; }
            s.push(j);
        }
        OpCode::Swap => {
            let t = s.len();
            if t < 2 || s[t - 1] == Nilai || s[t - 2] == Nilai { return None; }
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
                KEMBALI_SKALAR => matches!(*s.last()?, Bulat | Bool),
                _ => false,
            };
            if !ok { return None; }
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
