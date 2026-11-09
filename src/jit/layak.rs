//! Kelayakan J2 (`dokumentasi/jit/j2-rencana.md`): fungsi bulat murni. Bytecode ditelusuri dengan
//! stack ABSTRAK; yang tidak bisa dibuktikan ditolak, dan fungsi itu tetap jalan di VM.

use std::sync::Arc;

use crate::vm::opcode::OpCode;
use crate::vm::value::{VmFunction, VmValue};

/// Jenis satu posisi stack di kode yang dikompilasi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Jenis {
    Bulat,
    Bool,
    /// Rujukan ke `Rencana::callee[i]` (hasil `LoadGlobal`, cuma boleh dipanggil).
    Fungsi(usize),
}

#[derive(Debug)]
pub struct Rencana {
    /// Stack abstrak SEBELUM tiap instruksi; `None` = tak terjangkau.
    pub keadaan: Vec<Option<Vec<Jenis>>>,
    /// Fungsi yang dipanggil (dirinya sendiri juga, kalau rekursif).
    pub callee: Vec<Arc<VmFunction>>,
    /// Instruksi yang memulai blok dasar.
    pub pemimpin: Vec<bool>,
    /// Kedalaman stack terbesar (jumlah variabel yang dibutuhkan).
    pub maks: usize,
    /// Nilai `Const` bulat per instruksi.
    pub konstanta: Vec<Option<i64>>,
}

/// Sifat fungsi itu sendiri (di luar isi kodenya) yang membuatnya bisa diulang dari awal di VM.
pub fn bentuk_layak(f: &VmFunction) -> bool {
    !f.is_closure && !f.variadic && f.arity_min == f.arity && f.upvals.is_empty()
        && f.chunk.tipe_self.is_none()
}

pub fn analisis(f: &VmFunction, cari: &dyn Fn(&str) -> Option<Arc<VmFunction>>) -> Option<Rencana> {
    if !bentuk_layak(f) { return None; }
    let kode = &f.chunk.code;
    let n = kode.len();
    let mut keadaan: Vec<Option<Vec<Jenis>>> = vec![None; n];
    let mut pemimpin = vec![false; n];
    let mut konstanta = vec![None; n];
    let mut callee: Vec<Arc<VmFunction>> = Vec::new();
    let mut maks = f.arity as usize;
    if n == 0 { return None; }
    pemimpin[0] = true;
    let mut kerja = vec![(0usize, vec![Jenis::Bulat; f.arity as usize])];

    // Satukan keadaan di `ip`; beda jenis atau tinggi stack di titik pertemuan = tidak layak.
    fn tiba(keadaan: &mut [Option<Vec<Jenis>>], kerja: &mut Vec<(usize, Vec<Jenis>)>, ip: usize, s: Vec<Jenis>) -> Option<()> {
        match keadaan.get(ip)? {
            Some(ada) => if *ada != s { return None },
            None => kerja.push((ip, s)),
        }
        Some(())
    }

    while let Some((ip, mut s)) = kerja.pop() {
        if keadaan[ip].is_some() {
            if keadaan[ip].as_ref() != Some(&s) { return None; }
            continue;
        }
        keadaan[ip] = Some(s.clone());
        let ins = kode[ip];
        let a = ins.arg as usize;
        let lanjut = ip + 1;
        let bulat2 = |s: &mut Vec<Jenis>| -> Option<()> {
            let y = s.pop()?; let x = s.pop()?;
            (x == Jenis::Bulat && y == Jenis::Bulat).then_some(())
        };
        match ins.op {
            OpCode::LoadLocal | OpCode::LokalHitung => {
                let j = *s.get(a)?;
                if j != Jenis::Bulat { return None; }
                s.push(j);
            }
            OpCode::IntImm => s.push(Jenis::Bulat),
            OpCode::Const => match f.chunk.constants.get(a)? {
                VmValue::Int(v) => { konstanta[ip] = Some(i64::try_from(*v).ok()?); s.push(Jenis::Bulat) }
                _ => return None,
            },
            OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Mod => { bulat2(&mut s)?; s.push(Jenis::Bulat) }
            OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge | OpCode::Eq | OpCode::Ne => {
                bulat2(&mut s)?; s.push(Jenis::Bool)
            }
            OpCode::Pop => { s.pop()?; }
            OpCode::StoreLocal => {
                if *s.last()? != Jenis::Bulat || *s.get(a)? != Jenis::Bulat || a + 1 >= s.len() { return None; }
            }
            OpCode::SimpanBuang => {
                if s.pop()? != Jenis::Bulat || *s.get(a)? != Jenis::Bulat { return None; }
            }
            OpCode::Jump => {
                *pemimpin.get_mut(a)? = true;
                tiba(&mut keadaan, &mut kerja, a, s)?;
                if lanjut < n { pemimpin[lanjut] = true; }
                continue;
            }
            OpCode::JumpFalse => {
                if s.pop()? != Jenis::Bool { return None; }
                *pemimpin.get_mut(a)? = true;
                if lanjut >= n { return None; }
                pemimpin[lanjut] = true;
                tiba(&mut keadaan, &mut kerja, a, s.clone())?;
                tiba(&mut keadaan, &mut kerja, lanjut, s)?;
                continue;
            }
            OpCode::Return => {
                if s.pop()? != Jenis::Bulat { return None; }
                if lanjut < n { pemimpin[lanjut] = true; }
                continue;
            }
            OpCode::LoadGlobal => {
                let nama = f.chunk.names.get(a)?;
                let g = cari(nama)?;
                if !bentuk_layak(&g) { return None; }
                let i = match callee.iter().position(|c| Arc::ptr_eq(c, &g)) {
                    Some(i) => i,
                    None => { callee.push(g); callee.len() - 1 }
                };
                s.push(Jenis::Fungsi(i));
            }
            OpCode::Call => {
                if s.len() < a + 1 { return None; }
                let Jenis::Fungsi(i) = s[s.len() - a - 1] else { return None };
                if callee[i].arity as usize != a { return None; }
                for _ in 0..a { if s.pop()? != Jenis::Bulat { return None; } }
                s.pop();
                s.push(Jenis::Bulat);
            }
            // `ReturnNil` yang terjangkau, I/O, wadah, closure, ...: bukan fungsi bulat murni.
            _ => return None,
        }
        maks = maks.max(s.len());
        if lanjut >= n { return None; }
        tiba(&mut keadaan, &mut kerja, lanjut, s)?;
    }
    Some(Rencana { keadaan, callee, pemimpin, maks, konstanta })
}
