//! Pemeriksa JIR: tiap nilai didefinisikan sekali, tiap pemakaian didahului definisinya di bloknya sendiri (nilai
//! antarblok cuma lewat parameter), tipe operand cocok, lompatan membawa argumen sebanyak dan setipe parameter
//! tujuannya, dan tiap blok punya akhir. Dijalankan sesudah pembangun dan sesudah tiap pass.

use std::collections::HashSet;

use super::{Akhir, Biner, Blok, Fungsi, Inst, Jaga, Nilai, OpL, Tipe, Unar};

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
        // Pembantu dan stack VM cuma menerima kata 64 bit; `Lebar` lewat `TulisLebar`/tumpahan.
        let bukan_l = |v: Nilai| -> Result<(), String> {
            if f.tipe(v) == Tipe::L { Err(format!("b{bi}: {v} bertipe 128 bit di tempat 64 bit")) } else { Ok(()) }
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
                        Unar::KeL | Unar::KeLU => (Tipe::I, Tipe::L),
                        Unar::BukanBitL => (Tipe::L, Tipe::L),
                        Unar::KeBoolL | Unar::Rendah => (Tipe::L, Tipe::I),
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
                Inst::Muat { .. } | Inst::Buang { .. } | Inst::Taruh { .. } => {}
                Inst::Simpan { nilai, .. } => if let Some(v) = nilai { pakai(*v, &ada)?; bukan_l(*v)?; },
                Inst::Benar { hasil, .. } => { definisi(*hasil, &mut ada)?; tipe(*hasil, Tipe::I)?; }
                Inst::Langkah { isi, .. } => for (_, v) in isi { pakai(*v, &ada)?; },
                Inst::Buka { hasil, jaga: j, .. } => { jaga(j, &ada)?; definisi(*hasil, &mut ada)?; }
                Inst::Bawaan { isi, .. } => for (_, v) in isi { pakai(*v, &ada)?; },
                Inst::BinerVm { hasil, isi, .. } => {
                    for (_, v) in isi { pakai(*v, &ada)?; }
                    if let Some(h) = hasil { definisi(*h, &mut ada)?; tipe(*h, Tipe::I)?; }
                }
                Inst::MuatUpval { jaga: j, .. } => jaga(j, &ada)?,
                Inst::TeksBulat { n, .. } => { pakai(*n, &ada)?; tipe(*n, Tipe::I)?; }
                Inst::Pindah { .. } => {}
                Inst::Bangun { isi, .. } => for (_, v) in isi { pakai(*v, &ada)?; },
                Inst::Salin { .. } | Inst::Tukar { .. } => {}
                Inst::CekMetode { jaga: j, .. } | Inst::Maju { jaga: j, .. } => jaga(j, &ada)?,
                Inst::KonstL { hasil, .. } => { definisi(*hasil, &mut ada)?; tipe(*hasil, Tipe::L)?; }
                Inst::BinerL { hasil, op, a, b: y, jaga: j, jaga_muat } => {
                    pakai(*a, &ada)?;
                    pakai(*y, &ada)?;
                    for g in [j, jaga_muat].into_iter().flatten() { jaga(g, &ada)?; }
                    tipe(*a, Tipe::L)?;
                    tipe(*y, if matches!(op, OpL::GeserKiri | OpL::GeserKanan) { Tipe::I } else { Tipe::L })?;
                    definisi(*hasil, &mut ada)?;
                    tipe(*hasil, if matches!(op, OpL::Cmp(_) | OpL::Bagi | OpL::Sisa) { Tipe::I } else { Tipe::L })?;
                }
                Inst::Sempit { hasil, a, jaga: j } => { pakai(*a, &ada)?; tipe(*a, Tipe::L)?; jaga(j, &ada)?; definisi(*hasil, &mut ada)?; tipe(*hasil, Tipe::I)?; }
                Inst::TulisLebar { nilai, .. } => { pakai(*nilai, &ada)?; tipe(*nilai, Tipe::L)?; }
                Inst::BukaLebar { hasil, jaga: j, .. } => { jaga(j, &ada)?; definisi(*hasil, &mut ada)?; tipe(*hasil, Tipe::L)?; }
                Inst::Pembantu { args, hasil, gagal, keluar, .. } => {
                    for a in args { if let super::Arg::Isi(v) = a { pakai(*v, &ada)?; bukan_l(*v)?; } }
                    if let Some((_, j)) = gagal { jaga(j, &ada)?; }
                    if let Some(h) = hasil { definisi(*h, &mut ada)?; tipe(*h, Tipe::I)?; }
                    if let Some(k) = keluar { definisi(*k, &mut ada)?; }
                }
                Inst::Panjang { hasil, jaga: j, .. } => { jaga(j, &ada)?; definisi(*hasil, &mut ada)?; tipe(*hasil, Tipe::I)?; }
                Inst::Langsung { isi, args, basi, hasil, .. } => {
                    for (_, v) in isi.iter().chain(args) { pakai(*v, &ada)?; }
                    for (_, v) in args { bukan_l(*v)?; }
                    for v in basi.isi.iter().flatten() { pakai(*v, &ada)?; }
                    if let Some((h, _, j)) = hasil { jaga(j, &ada)?; definisi(*h, &mut ada)?; }
                }
                Inst::IterAda { hasil, jaga: j, .. } => { jaga(j, &ada)?; definisi(*hasil, &mut ada)?; tipe(*hasil, Tipe::I)?; }
                Inst::TulisVm { nilai, .. } => if let Some(v) = nilai { pakai(*v, &ada)?; bukan_l(*v)?; },
                Inst::SimpanUpval { nilai, jaga: j, .. } => { if let Some(v) = nilai { pakai(*v, &ada)?; bukan_l(*v)?; } jaga(j, &ada)?; }
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
            Some(Akhir::KembaliNilai { .. } | Akhir::KembaliUnit) => {}
            Some(Akhir::CabangVm { isi, tujuan, .. }) => {
                for (_, v) in isi { pakai(*v, &ada)?; }
                for (ke, args, _) in tujuan { lompat(*ke, args, &ada)?; }
            }
            Some(Akhir::Keluar { keadaan }) => for v in keadaan.isi.iter().flatten() { pakai(*v, &ada)?; },
            None => return Err(format!("b{bi} tanpa akhir")),
        }
    }
    Ok(())
}
