// src/terjemah/mesin.rs — pola "teks {} teks" dicari DI DALAM pesan (pesan dirakit dari potongan) dan diganti.
// Yang sudah diterjemahkan dibekukan; isi tangkapan jadi teks baru. Rinciannya di dokumentasi/src/terjemah/mod.md.

use std::sync::OnceLock;

pub(crate) struct Pola {
    /// Potongan teks Indonesia di antara `{}`; selalu diawali dan diakhiri teks (dijaga tes).
    pub(crate) seg: Vec<&'static str>,
    /// Potongan padanan Inggris, dan nomor tangkapan yang diletakkan sesudah tiap potongan.
    pub(crate) en_seg: Vec<&'static str>,
    pub(crate) en_lubang: Vec<usize>,
    bobot: usize,
}

fn urai_en(en: &'static str, n_lubang: usize) -> (Vec<&'static str>, Vec<usize>) {
    // `{}` atau `{N}` adalah isian; kurung kurawal lain (contoh kode JRG di kalimat) tetap teks.
    let mut seg = Vec::new();
    let mut lubang = Vec::new();
    let (mut mulai, mut i, mut urut) = (0, 0, 0);
    while let Some(rel) = en[i..].find('{') {
        let a = i + rel;
        if let Some(rb) = en[a..].find('}') {
            let isi = &en[a + 1..a + rb];
            let nomor = if isi.is_empty() { Some(urut) } else { isi.parse::<usize>().ok() };
            if let Some(n) = nomor.filter(|n| *n < n_lubang) {
                if isi.is_empty() { urut += 1; }
                seg.push(&en[mulai..a]);
                lubang.push(n);
                mulai = a + rb + 1;
                i = mulai;
                continue;
            }
        }
        i = a + 1;
    }
    seg.push(&en[mulai..]);
    (seg, lubang)
}

pub(crate) fn pola() -> &'static [Pola] {
    static P: OnceLock<Vec<Pola>> = OnceLock::new();
    P.get_or_init(|| {
        let mut v: Vec<Pola> = super::en::TABEL.iter().map(|(id, en)| {
            let seg: Vec<&'static str> = id.split("{}").collect();
            let (en_seg, en_lubang) = urai_en(en, seg.len() - 1);
            let bobot = seg.iter().map(|s| s.len()).sum();
            Pola { seg, en_seg, en_lubang, bobot }
        }).collect();
        // Yang paling panjang lebih dulu: kalimat utuh menang atas potongan pendek di dalamnya.
        v.sort_by(|a, b| b.bobot.cmp(&a.bobot));
        v
    })
}

/// Satu potongan keluaran: sudah diterjemahkan (tidak disentuh lagi), atau masih teks asli. `lubang` = isi
/// tangkapan sebuah pola; cuma itu yang boleh diganti utuh lewat `en::KATA` (`kunci` di kolom perintah tetap).
enum Potong { Jadi(String), Asli(String, bool) }

fn huruf(c: Option<char>) -> bool { c.is_some_and(|c| c.is_alphanumeric() || c == '_') }

/// Cari `p` di `teks`; kembalikan (awal, akhir, tangkapan).
fn cocok<'a>(p: &Pola, teks: &'a str) -> Option<(usize, usize, Vec<&'a str>)> {
    let mut mulai_cari = 0;
    while let Some(rel) = teks[mulai_cari..].find(p.seg[0]) {
        let awal = mulai_cari + rel;
        let mut pos = awal + p.seg[0].len();
        let mut tangkap = Vec::new();
        let mut gagal = false;
        for s in &p.seg[1..] {
            // Tangkapan tidak melewati baris baru dan tidak lebih dari 400 byte.
            let mut batas = teks[pos..].find('\n').map(|i| pos + i).unwrap_or(teks.len()).min(pos + 400 + s.len());
            while !teks.is_char_boundary(batas) { batas -= 1; }
            match teks[pos..batas].find(s) {
                Some(i) => {
                    tangkap.push(&teks[pos..pos + i]);
                    pos += i + s.len();
                }
                _ => { gagal = true; break }
            }
        }
        // Pola yang diawali/diakhiri huruf cuma cocok dengan kata utuh: `argumen` tidak di dalam `argument`.
        let tepi_kiri = !huruf(p.seg[0].chars().next()) || !huruf(teks[..awal].chars().next_back());
        let tepi_kanan = !huruf(p.seg.last().and_then(|s| s.chars().next_back())) || !huruf(teks[pos..].chars().next());
        if !gagal && tepi_kiri && tepi_kanan { return Some((awal, pos, tangkap)); }
        mulai_cari = awal + p.seg[0].len().max(1);
        while mulai_cari < teks.len() && !teks.is_char_boundary(mulai_cari) { mulai_cari += 1; }
        if mulai_cari >= teks.len() { break; }
    }
    None
}

/// Terjemahkan `teks` sejauh tabel mengenalnya. Pola dengan rentang PALING PANJANG menang (seri: teks polanya
/// lebih panjang), supaya kalimat luar menang atas potongan pendek di dalamnya.
pub fn terjemah_ke_en(teks: &str) -> String {
    if !teks.bytes().any(|b| b.is_ascii_lowercase()) { return teks.to_string(); }
    let mut potong = vec![Potong::Asli(teks.to_string(), false)];
    loop {
        let mut berubah = false;
        let mut baru = Vec::with_capacity(potong.len() + 4);
        for pt in potong {
            let Potong::Asli(s, lubang) = pt else { baru.push(pt); continue };
            let terbaik = pola().iter().filter_map(|p| cocok(p, &s).map(|(a, b, t)| (p, a, b, t)))
                .max_by_key(|(p, a, b, _)| (b - a, p.bobot, std::cmp::Reverse(*a)));
            let Some((p, a, b, tangkap)) = terbaik else { baru.push(Potong::Asli(s, lubang)); continue };
            berubah = true;
            if a > 0 { baru.push(Potong::Asli(s[..a].to_string(), false)); }
            for (i, seg) in p.en_seg.iter().enumerate() {
                if !seg.is_empty() { baru.push(Potong::Jadi(seg.to_string())); }
                if let Some(&n) = p.en_lubang.get(i) {
                    if !tangkap[n].is_empty() { baru.push(Potong::Asli(tangkap[n].to_string(), true)); }
                }
            }
            if b < s.len() { baru.push(Potong::Asli(s[b..].to_string(), false)); }
        }
        potong = baru;
        if !berubah { break; }
    }
    let hasil: String = potong.into_iter().map(|p| match p {
        Potong::Jadi(s) => s,
        Potong::Asli(s, true) => kata_utuh(&s).unwrap_or(s),
        Potong::Asli(s, false) => s,
    }).collect();
    if hasil == teks { hasil } else { tunggal(&hasil) }
}

/// Bahasa Indonesia tidak menandai jamak, jadi "{} berkas" jadi "{} files"; "1 files" dibetulkan jadi "1 file".
fn tunggal(s: &str) -> String {
    const JAMAK: [(&str, &str); 14] = [("tests", "test"), ("files", "file"), ("errors", "error"),
        ("warnings", "warning"), ("elements", "element"), ("arguments", "argument"), ("calls", "call"),
        ("names", "name"), ("functions", "function"), ("characters", "character"), ("items", "item"),
        ("parameters", "parameter"), ("values", "value"), ("bytes", "byte")];
    let mut keluar = String::with_capacity(s.len());
    let mut sisa = s;
    while let Some(i) = sisa.find("1 ") {
        let sebelum = sisa[..i].chars().next_back();
        let lepas = !sebelum.is_some_and(|c| c.is_ascii_digit() || c == '.' || c == '-');
        let ganti = lepas.then(|| JAMAK.iter().find(|(j, _)| {
            sisa[i + 2..].starts_with(j) && !huruf(sisa[i + 2 + j.len()..].chars().next())
        })).flatten();
        match ganti {
            Some((j, t)) => { keluar.push_str(&sisa[..i + 2]); keluar.push_str(t); sisa = &sisa[i + 2 + j.len()..]; }
            None => { keluar.push_str(&sisa[..i + 2]); sisa = &sisa[i + 2..]; }
        }
    }
    keluar.push_str(sisa);
    keluar
}

/// Sisipan yang SELURUHNYA satu kata dari `en::KATA` (spasi di tepi dipertahankan).
fn kata_utuh(s: &str) -> Option<String> {
    let inti = s.trim();
    let (_, en) = super::en::KATA.iter().find(|(id, _)| *id == inti)?;
    let awal = &s[..s.len() - s.trim_start().len()];
    let akhir = &s[s.trim_end().len()..];
    Some(format!("{awal}{en}{akhir}"))
}

#[cfg(test)]
pub(crate) fn urai_en_uji(en: &'static str, n: usize) -> (Vec<&'static str>, Vec<usize>) { urai_en(en, n) }

#[cfg(test)]
mod tests {
    use super::terjemah_ke_en;

    #[test]
    fn pola_ditemukan_di_dalam_pesan_dan_isinya_ikut() {
        assert_eq!(terjemah_ke_en("`f` dipanggil di a.jrg:3:1"), "`f` called at a.jrg:3:1");
        assert_eq!(terjemah_ke_en("… dan 4 panggilan lagi"), "… and 4 more calls");
        assert_eq!(terjemah_ke_en("teks yang tidak dikenal tabel"), "teks yang tidak dikenal tabel");
        assert_eq!(terjemah_ke_en(""), "");
        assert_eq!(terjemah_ke_en("[jrgm] 1 test di 1 berkas"), "[jrgm] 1 test in 1 file");
        assert_eq!(terjemah_ke_en("[jrgm] 11 test di 21 berkas"), "[jrgm] 11 tests in 21 files");
    }
}
