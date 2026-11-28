// src/saran_inti.rs — saran "maksudnya `X`?" atas nama yang salah eja. Tanpa `std`: SALINAN PERSISNYA di
// `runtime/src/saran_inti.rs`, dijaga tes, karena `Canvas::` di biner LLVM menyarankan nama tombol dan mode
// saat jalan dengan aturan yang sama.
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Nama terdekat dengan `nama` di antara `calon`, kalau ada yang cukup dekat.
pub fn saran_nama_terdekat<'a>(
    nama: &str, calon: impl Iterator<Item = &'a str> + Clone,
) -> Option<String> {
    // 1. Beda huruf besar-kecil: kepastian, bukan tebakan.
    let mut sama_huruf: Vec<&str> =
        calon.clone().filter(|t| *t != nama && t.eq_ignore_ascii_case(nama)).collect();
    if !sama_huruf.is_empty() {
        sama_huruf.sort_by(urutan_pilih);
        return Some(sama_huruf[0].to_string());
    }

    // 2. Kumpulkan calon per jarak, sampai batas yang boleh untuk panjang ini.
    let batas = batas_jarak(nama);
    let mut d1: Vec<&str> = Vec::new();
    let mut d2: Vec<&str> = Vec::new();
    for t in calon {
        if t == nama { continue; }
        match jarak_sunting(nama, t) {
            1 => d1.push(t),
            2 if batas >= 2 => d2.push(t),
            _ => {}
        }
    }

    // 3. Satu suntingan: sinyal kuat, jadi seri pun tetap disarankan.
    if !d1.is_empty() {
        d1.sort_by(urutan_pilih);
        return Some(d1[0].to_string());
    }

    // 4.
    if d2.len() == 1 { return Some(d2[0].to_string()); }
    None
}

/// Berapa suntingan yang boleh dimaafkan untuk nama sepanjang ini.
pub fn batas_jarak(nama: &str) -> usize {
    (nama.chars().count() / 5).clamp(1, 2)
}

/// Urutan memilih di antara calon yang sejarak: yang **terpendek** dulu, lalu
/// abjad.
pub fn urutan_pilih(a: &&str, b: &&str) -> core::cmp::Ordering {
    a.chars().count().cmp(&b.chars().count()).then_with(|| a.cmp(b))
}

/// Jarak sunting (Levenshtein) per karakter Unicode.
pub fn jarak_sunting(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (la, lb) = (a.len(), b.len());
    // Beda panjang lebih dari ambang tidak mungkin lolos; keluar lebih awal
    // supaya 500-an calon tidak semuanya mengisi matriks.
    if la.abs_diff(lb) > 1 { return usize::MAX; }
    let mut baris: Vec<usize> = (0..=lb).collect();
    for i in 1..=la {
        let mut diag = baris[0];
        baris[0] = i;
        for j in 1..=lb {
            let simpan = baris[j];
            baris[j] = if a[i - 1] == b[j - 1] { diag }
                       else { 1 + baris[j].min(baris[j - 1]).min(diag) };
            diag = simpan;
        }
    }
    baris[lb]
}
