// src/diag/lapor.rs — pelaporan runtime/VM/LLVM dan saran nama

use super::*;

/// Laporkan sebuah error runtime dengan format PENUH: sapaan, kode, lokasi,
/// cuplikan sumber, caret, dan baris `bantuan:`/`catatan:`.
pub fn laporkan_runtime(map: &mut SourceMap, e: &crate::interpreter::RuntimeError) {
    // Pesan runtime sering sudah membawa baris "bantuan:"/"catatan:" sendiri
    // (dibangun di interpreter, di mana konteks nilainya ada).
    let mut lines = e.message.lines();
    let head = lines.next().unwrap_or_default().to_string();
    // Error runtime bisa berasal dari dalam modul yang di-`import`.
    // Sumbernya baru dibaca saat program berjalan, jadi didaftarkan di
    // sini supaya cuplikannya diambil dari berkas yang benar.
    if let Some(sp) = e.span {
        map.ensure_module(sp.file_id);
    }
    // `saat_jalan` menandai fase, dan itu satu-satunya hal yang
    // membedakan nada omelan runtime dari nada omelan kompilasi.
    let mut d = Diagnostic::error(head).with_span_opt(e.span).saat_jalan();
    d = tambah_baris_sisa(d, lines);
    // Modul yang muncul di jejak belum tentu yang sama dengan modul tempat
    // galatnya mendarat, jadi tiap berkasnya didaftarkan juga — kalau tidak …
    for fr in &e.jejak {
        if let Some(sp) = fr.span { map.ensure_module(sp.file_id); }
    }
    d = d.with_jejak(e.jejak.clone(), e.jejak_sisa);
    d.emit(map);
}

/// Laporkan error dari backend VM — dengan lokasi PERSIS.
pub fn laporkan_vm(map: &mut SourceMap, e: &crate::vm::vm::VmError) {
    if let Some(sp) = e.span {
        map.ensure_module(sp.file_id);
    }
    let mut lines = e.msg.lines();
    let head = lines.next().unwrap_or_default().to_string();
    let mut d = Diagnostic::error(head).with_span_opt(e.span).saat_jalan();
    d = tambah_baris_sisa(d, lines);
    for fr in &e.jejak {
        if let Some(sp) = fr.span { map.ensure_module(sp.file_id); }
    }
    d = d.with_jejak(e.jejak.clone(), e.jejak_sisa);
    d.emit(map);
}

/// Laporkan kumpulan error backend LLVM.
pub fn laporkan_llvm(map: &mut SourceMap, galat: &[crate::llvm::galat::GalatLlvm]) {
    for (i, g) in galat.iter().enumerate() {
        if let Some(sp) = g.span {
            map.ensure_module(sp.file_id);
        }
        let mut lines = g.msg.lines();
        let head = lines.next().unwrap_or_default().to_string();
        let mut d = Diagnostic::error(head).with_span_opt(g.span);
        d = tambah_baris_sisa(d, lines);
        if i == 0 {
            d.emit(map);
        } else {
            d.emit_tanpa_sapaan(map);
        }
    }
    if galat.len() > 1 {
        crate::host::galat_baris(&ringkas_jumlah(galat.len()));
    }
}

/// Versi untuk error yang cuma berupa teks — tidak ada `Span` yang bisa
/// dipakai, jadi tidak ada cuplikan sumber yang bisa ditunjukkan, tapi …
pub fn laporkan_pesan(map: &SourceMap, pesan: &str, saat_jalan: bool) {
    let mut lines = pesan.lines();
    let head = lines.next().unwrap_or_default().to_string();
    let mut d = Diagnostic::error(head);
    if saat_jalan { d = d.saat_jalan(); }
    d = tambah_baris_sisa(d, lines);
    d.emit(map);
}

/// Baris sesudah baris pertama sebuah pesan jadi `bantuan:`/`catatan:` milik
/// diagnostiknya — satu definisi untuk semua pintu, termasuk galat lex/parse
/// yang dulu mencetaknya SEBELUM lokasinya, seolah milik galat di atasnya.
pub fn tambah_baris_sisa<'a>(mut d: Diagnostic, lines: impl Iterator<Item = &'a str>) -> Diagnostic {
    for rest in lines {
        let t = rest.trim();
        if let Some(x) = t.strip_prefix("bantuan:") {
            d = d.with_help(x.trim());
        } else if let Some(x) = t.strip_prefix("catatan:") {
            d = d.with_note(x.trim());
        } else if !t.is_empty() {
            d = d.with_note(t);
        }
    }
    d
}

// ─── Saran nama ────────────────────────────────────────────────────────────

pub use crate::saran_inti::{saran_nama_terdekat, jarak_sunting};
pub(crate) use crate::saran_inti::urutan_pilih;

/// Nama yang SAMA PERSIS di namespace lain.
pub fn nama_di_namespace_lain<'a>(
    path: &str, calon: impl Iterator<Item = &'a str> + Clone,
) -> Vec<String> {
    let Some((ns, ekor)) = path.rsplit_once("::") else { return Vec::new() };
    let mut ada: Vec<&str> = calon
        .filter(|t| *t != path)
        .filter(|t| match t.rsplit_once("::") {
            Some((n2, e2)) => e2 == ekor && n2 != ns,
            None => false,
        })
        .collect();
    // Diurutkan menurut JARAKNYA ke path aslinya, bukan abjad: yang terdekat
    // hampir selalu yang dimaksud, dan itu juga yang membuat pemanggilnya
    // bisa memutuskan "ini cukup dekat untuk disebut `maksudnya X?`".
    ada.sort_by(|a, b| jarak_sunting(path, a).cmp(&jarak_sunting(path, b))
        .then_with(|| urutan_pilih(a, b)));
    ada.dedup();
    ada.into_iter().take(3).map(|s| s.to_string()).collect()
}

/// Bentuk siap-tempel: `" — maksudnya `X`?"`, atau kosong.
pub fn saran_nama<'a>(nama: &str, calon: impl Iterator<Item = &'a str> + Clone) -> String {
    saran_nama_terdekat(nama, calon)
        .map(|t| format!(" — maksudnya `{}`?", t))
        .unwrap_or_default()
}

