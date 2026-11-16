//! Pemeriksa JIR: tiap nilai didefinisikan sekali, tiap pemakaian didahului definisinya di bloknya sendiri (nilai
//! antarblok cuma lewat parameter), tipe operand cocok, lompatan membawa argumen sebanyak dan setipe parameter
//! tujuannya, dan tiap blok punya akhir. Dijalankan sesudah pembangun dan sesudah tiap pass.

use std::collections::HashSet;

use super::{Akhir, Biner, Blok, Fungsi, Inst, Jaga, Nilai, Tipe, Unar};

pub fn periksa(f: &Fungsi) -> Result<(), String> {
    let mut sekali: HashSet<Nilai> = HashSet::new();
    for (bi, b) in f.blok.iter().enumerate() {
        let mut ada: HashSet<Nilai> = HashSet::new();
        let mut definisi = |v: Nilai, ada: &mut HashSet<Nilai>| -> Result<(), String> {
            if v.0 as usize >= f.tipe.len() { return Err(format!("b{bi}: {v} tanpa tipe")); }
            if !sekali.insert(v) { return Err(format!("b{bi}: {v} didefinisikan dua kali")); }
            ada.insert(v);
            Ok(())
        };
        for p in &b.params { definisi(*p, &mut ada)?; }
        let pakai = |v: Nilai, ada: &HashSet<Nilai>| -> Result<(), String> {
            if ada.contains(&v) { Ok(()) } else { Err(format!("b{bi}: {v} dipakai sebelum didefinisikan di bloknya")) }
        };
        let tipe = |v: Nilai, t: Tipe| -> Result<(), String> {
            if f.tipe(v) == t { Ok(()) } else { Err(format!("b{bi}: {v} bertipe {:?}, diharapkan {t:?}", f.tipe(v))) }
        };
        let jaga = |j: &Jaga, ada: &HashSet<Nilai>| -> Result<(), String> {
            for v in j.keadaan.isi.iter().flatten() { pakai(*v, ada)?; }
            Ok(())
        };
        for ins in &b.inst {
            match ins {
                Inst::KonstI { hasil, .. } => { definisi(*hasil, &mut ada)?; tipe(*hasil, Tipe::I)?; }
                Inst::KonstF { hasil, .. } => { definisi(*hasil, &mut ada)?; tipe(*hasil, Tipe::F)?; }
                Inst::Biner { hasil, op, a, b: y, jaga: j } => {
                    pakai(*a, &ada)?;
                    pakai(*y, &ada)?;
                    if let Some(j) = j { jaga(j, &ada)?; }
                    let (masuk, keluar) = match op {
                        Biner::FTambah | Biner::FKurang | Biner::FKali | Biner::FBagi => (Tipe::F, Tipe::F),
                        Biner::FCmp(_) => (Tipe::F, Tipe::I),
                        _ => (Tipe::I, Tipe::I),
                    };
                    tipe(*a, masuk)?;
                    tipe(*y, masuk)?;
                    definisi(*hasil, &mut ada)?;
                    tipe(*hasil, keluar)?;
                }
                Inst::Unar { hasil, op, a, jaga: j } => {
                    pakai(*a, &ada)?;
                    if let Some(j) = j { jaga(j, &ada)?; }
                    let (masuk, keluar) = match op {
                        Unar::FNeg => (Tipe::F, Tipe::F),
                        Unar::KeF => (Tipe::I, Tipe::F),
                        _ => (Tipe::I, Tipe::I),
                    };
                    tipe(*a, masuk)?;
                    definisi(*hasil, &mut ada)?;
                    tipe(*hasil, keluar)?;
                }
                Inst::Pilih { hasil, jika, ya, tidak } => {
                    for v in [jika, ya, tidak] { pakai(*v, &ada)?; }
                    tipe(*jika, Tipe::I)?;
                    if f.tipe(*ya) != f.tipe(*tidak) { return Err(format!("b{bi}: pilih atas dua tipe")); }
                    definisi(*hasil, &mut ada)?;
                    tipe(*hasil, f.tipe(*ya))?;
                }
                Inst::JagaJika { kondisi, jaga: j } => { pakai(*kondisi, &ada)?; tipe(*kondisi, Tipe::I)?; jaga(j, &ada)?; }
                Inst::Undi { hasil } => { definisi(*hasil, &mut ada)?; tipe(*hasil, Tipe::I)?; }
                Inst::Panggil { hasil, args, di, sesudah, .. } => {
                    for a in args { pakai(*a, &ada)?; tipe(*a, Tipe::I)?; }
                    for v in di.isi.iter().chain(&sesudah.isi).flatten() { pakai(*v, &ada)?; }
                    definisi(*hasil, &mut ada)?;
                    tipe(*hasil, Tipe::I)?;
                }
            }
        }
        let lompat = |ke: Blok, args: &[Nilai], ada: &HashSet<Nilai>| -> Result<(), String> {
            let t = f.blok.get(ke.0 as usize).ok_or_else(|| format!("b{bi}: lompat ke {ke} yang tidak ada"))?;
            if t.params.len() != args.len() {
                return Err(format!("b{bi}: {ke} butuh {} argumen, dapat {}", t.params.len(), args.len()));
            }
            for (a, p) in args.iter().zip(&t.params) {
                pakai(*a, ada)?;
                if f.tipe(*a) != f.tipe(*p) { return Err(format!("b{bi}: argumen {a} ke {ke} beda tipe")); }
            }
            if ke == f.masuk { return Err(format!("b{bi}: lompat ke blok masuk")); }
            Ok(())
        };
        match &b.akhir {
            Some(Akhir::Lompat { ke, args }) => lompat(*ke, args, &ada)?,
            Some(Akhir::Cabang { kondisi, ya, ya_args, tidak, tidak_args }) => {
                pakai(*kondisi, &ada)?;
                tipe(*kondisi, Tipe::I)?;
                lompat(*ya, ya_args, &ada)?;
                lompat(*tidak, tidak_args, &ada)?;
            }
            Some(Akhir::Kembali { nilai, .. }) => pakai(*nilai, &ada)?,
            Some(Akhir::Keluar { keadaan }) => for v in keadaan.isi.iter().flatten() { pakai(*v, &ada)?; },
            None => return Err(format!("b{bi} tanpa akhir")),
        }
    }
    Ok(())
}
