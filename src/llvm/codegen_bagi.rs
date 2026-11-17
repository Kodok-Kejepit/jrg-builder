// src/llvm/codegen_bagi.rs — salin-saat-tulis untuk lokal polos (antrean B butir 4): `let b = a` atas slice datar
// berbagi penyangga; tiap lokal itu membawa bendera "milik sendiri", dan tulisan pertama menyalin. Rancangan di
// dokumentasi/src/llvm/cow-rancangan.md.

use crate::parser::*;

use super::codegen::Codegen;
use super::types::JrgType;

impl Codegen {
    /// Slice yang elemennya tidak memuat penyangga lain: salinan dangkalnya sudah milik sendiri.
    fn slice_datar(&self, t: &JrgType) -> bool {
        matches!(t, JrgType::Slice(e) if !self.butuh_salin(e))
    }

    /// Dipanggil `alloca_local`: lokal calon berbagi (`capture::calon_bagi`) yang bertipe slice datar mendapat
    /// bendera, mulai 1 (nilai yang diikat kepadanya milik sendiri, kecuali `bagi_lokal` mengubahnya).
    pub(crate) fn pasang_bendera_milik(&mut self, nama: &str, slot: &str, ty: &JrgType) {
        if !self.calon_bagi.contains(nama) || self.tangkapan.contains(nama) || !self.slice_datar(ty) { return; }
        let bendera = self.alloca_raw("milik.", "i1", Some(1));
        self.emit(format!("store i1 1, ptr {}", bendera));
        self.milik_lokal.insert(slot.to_string(), bendera);
    }

    fn bendera_nama(&self, nama: &str) -> Option<(String, String, JrgType)> {
        let (slot, ty) = self.lookup_local(nama)?;
        let b = self.milik_lokal.get(&slot)?.clone();
        Some((slot, b, ty))
    }

    /// `e` adalah lokal berbendera bertipe `ty` (atau tipe apa pun kalau `None`): bendera sumbernya.
    pub(crate) fn sumber_bagi(&self, e: &Expr, ty: Option<&JrgType>) -> Option<String> {
        let Expr::Ident(n, _) = e else { return None };
        if self.pinjaman.contains(n) { return None; }
        let (_, b, t) = self.bendera_nama(n)?;
        (ty.map_or(true, |x| *x == t)).then_some(b)
    }

    /// Apakah lokal baru `nama` bertipe `ty` akan mendapat bendera (syarat yang sama dengan `pasang_bendera_milik`)?
    pub(crate) fn akan_berbendera(&self, nama: &str, ty: &JrgType) -> bool {
        self.calon_bagi.contains(nama) && !self.tangkapan.contains(nama) && self.slice_datar(ty)
    }

    /// Dua nama sekarang memegang penyangga yang sama: dua-duanya bukan pemilik tunggal.
    pub(crate) fn tandai_berbagi(&mut self, nama_baru: &str, bendera_sumber: &str) {
        self.emit(format!("store i1 0, ptr {}", bendera_sumber));
        if let Some((_, b, _)) = self.bendera_nama(nama_baru) { self.emit(format!("store i1 0, ptr {}", b)); }
    }

    /// Sebelum menulis ke (atau memindahkan) lokal berbendera: salin penyangganya kalau masih dibagi.
    pub(crate) fn pastikan_milik(&mut self, nama: &str) {
        let Some((slot, bendera, ty)) = self.bendera_nama(nama) else { return };
        let f = self.fresh("milik.f.");
        self.emit(format!("{} = load i1, ptr {}", f, bendera));
        let (salin, usai) = (self.fresh_label("milik.salin."), self.fresh_label("milik.usai."));
        self.terminate(format!("br i1 {}, label %{}, label %{}, !prof !1", f, usai, salin));
        self.start_block(salin);
        let v = self.load_place(&slot, &ty);
        let baru = self.salin_slice(&v);
        self.emit(format!("store %JrgSlice {}, ptr {}", baru.text, slot));
        self.emit(format!("store i1 1, ptr {}", bendera));
        self.terminate(format!("br label %{}", usai));
        self.start_block(usai);
    }

    /// Awal `codegen_assign`: `x = e` tanpa `x` di kanan mengganti nilainya; tulisan lain dan bentuk pindah (yang
    /// menyebut `x` di kanan) menyalin dulu `x` yang masih dibagi. Jawabannya: boleh diikat tanpa salinan.
    pub(crate) fn milik_sebelum_tugas(&mut self, a: &AssignStmt) -> bool {
        if self.milik_lokal.is_empty() { return false; }
        let disebut = |n: &str| a.values.iter().any(|v| crate::interpreter::capture::names_mentioned(v).contains(n));
        let mut tanpa_salin = false;
        for t in &a.targets {
            let Some(akar) = akar_tulis(t) else { continue };
            let Some((_, bendera, ty)) = self.bendera_nama(akar) else { continue };
            let utuh = a.targets.len() == 1 && matches!(t, Expr::Ident(..)) && matches!(a.op, AssignOp::Eq) && !disebut(akar);
            if !utuh { self.pastikan_milik(akar); continue; }
            match a.values.as_slice() {
                [v] => match self.sumber_bagi(v, Some(&ty)) {
                    Some(sumber) => {
                        self.emit(format!("store i1 0, ptr {}", sumber));
                        self.emit(format!("store i1 0, ptr {}", bendera));
                        tanpa_salin = true;
                    }
                    None => self.emit(format!("store i1 1, ptr {}", bendera)),
                },
                _ => self.emit(format!("store i1 1, ptr {}", bendera)),
            }
        }
        tanpa_salin
    }
}

fn akar_tulis(e: &Expr) -> Option<&str> {
    match e {
        Expr::Ident(n, _) => Some(n.as_str()),
        Expr::Field { obj, .. } | Expr::Index { obj, .. } => akar_tulis(obj),
        _ => None,
    }
}
