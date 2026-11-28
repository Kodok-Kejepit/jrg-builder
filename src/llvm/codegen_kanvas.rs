// src/llvm/codegen_kanvas.rs — `Canvas::` di backend LLVM tanpa jembatan: pegangan = nomor di registry runtime
// (`runtime/src/canvas/`), tiap argumen menyeberang sebagai kotak bertag + nama tipe acuannya + tampilannya, dan
// aturan serta kalimat tiap method milik `canvas::layan` yang sama dengan interpreter dan VM.

use crate::lexer::Span;
use crate::parser::{Expr, FieldInit, NamedType, TypeExpr};

use super::codegen::{Codegen, Val};
use super::types::{JrgType, StructLayout};

/// Label struct pegangan kanvas: tampil `Canvas`, tidak bertabrakan dengan struct program bernama sama.
pub(crate) const LABEL_KANVAS: &str = "Canvas\u{E000}";
/// Hasil `get_mouse_pos`/`sentuhan`: struct `MousePos { x, y }` acuan.
pub(crate) const LABEL_POSISI: &str = "MousePos\u{E000}";
/// `Warna` pengganti kalau program tidak punya `struct Warna` (berkas ber-`import` tanpa `std/mesin/warna`).
const LABEL_WARNA: &str = "Warna\u{E000}";

#[derive(Clone, Copy, PartialEq)]
enum Hasil { Unit, Bool, Int, Float, Warna, Posisi }

fn hasil_metode(m: &str) -> Hasil {
    match m {
        "is_open" | "update" | "key_pressed" | "key_held" | "get_mouse_button" | "save_ppm" | "save_bmp"
        | "save_png" | "blit" | "blit_region" => Hasil::Bool,
        "width" | "height" | "frame" | "jumlah_sentuhan" => Hasil::Int,
        "fps" | "delta_time" => Hasil::Float,
        "get_pixel" => Hasil::Warna,
        "get_mouse_pos" | "sentuhan" => Hasil::Posisi,
        _ => Hasil::Unit,
    }
}

/// Method kanvas yang dilayani runtime (tanpa yang tidak butuh pegangan).
fn metode_kanvas(m: &str) -> bool {
    !matches!(m, "punya_jendela" | "layar_sentuh" | "nama_tombol")
        && crate::interpreter::calon_method("canvas").iter().any(|c| c == m)
}

fn nol() -> Span { Span { lo: crate::lexer::BytePos(0), hi: crate::lexer::BytePos(0), file_id: 0 } }

impl Codegen {
    pub(crate) fn tipe_kanvas(&mut self) -> JrgType {
        if !self.types.structs.contains_key(LABEL_KANVAS) {
            self.types.register_struct(StructLayout { name: LABEL_KANVAS.to_string(), fields: vec![("id".to_string(), JrgType::I64)] });
        }
        JrgType::Struct(LABEL_KANVAS.to_string())
    }

    fn tipe_posisi(&mut self) -> JrgType {
        if !self.types.structs.contains_key(LABEL_POSISI) {
            self.types.register_struct(StructLayout { name: LABEL_POSISI.to_string(),
                fields: vec![("x".to_string(), JrgType::F64), ("y".to_string(), JrgType::F64)] });
        }
        JrgType::Struct(LABEL_POSISI.to_string())
    }

    /// `Canvas::…`, `canvas::…`; `None` untuk nama yang bukan miliknya.
    pub(crate) fn coba_builtin_kanvas(&mut self, nama: &str, args: &[Expr], span: Span) -> Option<Result<Val, String>> {
        match nama {
            "Canvas::new" => Some(self.kanvas_baru(nama, args, span)),
            "Canvas::load_png" => Some(self.kanvas_muat_png(nama, args, span)),
            "canvas::punya_jendela" | "canvas::layar_sentuh" if args.is_empty() => Some(Ok(self.kanvas_layar(nama))),
            "canvas::nama_tombol" if args.len() == 1 => Some(self.kanvas_nama_tombol(args, span)),
            _ => {
                let m = nama.strip_prefix("canvas::")?;
                if !metode_kanvas(m) { return None; }
                Some(self.kanvas_namespace(nama, m, args, span))
            }
        }
    }

    /// `c.m(..)` atas pegangan kanvas.
    pub(crate) fn metode_kanvas(&mut self, penerima: &Val, method: &str, args: &[Expr], span: Span) -> Result<Val, String> {
        let nilai = args.iter().map(|a| self.codegen_expr(a, None)).collect::<Result<Vec<_>, _>>()?;
        let ns = if method == "to_string" { "*" } else { "canvas" };
        if let Some(p) = crate::interpreter::pesan_jumlah_arg_metode(ns, method, nilai.len()) {
            self.panik_statis(&p);
            return Ok(self.kosong_hasil(method));
        }
        if method == "to_string" { return self.val_to_str(penerima); }
        if !metode_kanvas(method) {
            let calon = crate::interpreter::calon_method("canvas");
            let saran = crate::diag::saran_nama_terdekat(method, calon.iter().map(|s| s.as_str()));
            self.panik_statis(&crate::interpreter::pesan_method_tak_ada("canvas", method, "", saran.as_deref()));
            return Ok(Val::unit());
        }
        let id = self.id_kanvas(penerima);
        self.kanvas_panggil(&id, method, &nilai, span)
    }

    /// `canvas::m(c, ..)`: jumlah argumen di pintu native, lalu penerimanya, seperti `stdlib/kanvas.rs`.
    fn kanvas_namespace(&mut self, nama: &str, m: &str, args: &[Expr], span: Span) -> Result<Val, String> {
        let nilai = args.iter().map(|a| self.codegen_expr(a, None)).collect::<Result<Vec<_>, _>>()?;
        if let Some(p) = Self::aritas_kanvas(nama, nilai.len()) {
            self.panik_statis(&p);
            return Ok(self.kosong_hasil(m));
        }
        let kanvas = self.tipe_kanvas();
        let Some(pertama) = nilai.first() else {
            self.panik_statis(&format!("canvas::{}: argumen pertamanya harus sebuah Canvas, dapat `tidak ada`", m));
            return Ok(self.kosong_hasil(m));
        };
        let penerima = if pertama.ty == kanvas {
            pertama.clone()
        } else if pertama.ty == JrgType::Any {
            let kata = self.kata_buram(&kanvas);
            let slot = self.spill_to_place(pertama);
            let (w, c) = (self.fresh("kv."), self.fresh("kv."));
            self.emit(format!("{} = load i32, ptr {}", w, slot));
            self.emit(format!("{} = icmp ne i32 {}, {}", c, w, kata));
            let kotak = pertama.clone();
            let f = m.to_string();
            self.gagal_bila(&c, |cg| cg.panik_tipe_any(&kotak, |t| format!(
                "canvas::{}: argumen pertamanya harus sebuah Canvas, dapat `{}`", f, t)));
            self.buka_any(pertama, &kanvas, span)?
        } else {
            let t = crate::ruang_tipe::bersih(&pertama.ty.nama_typeof()).into_owned();
            self.panik_statis(&format!("canvas::{}: argumen pertamanya harus sebuah Canvas, dapat `{}`", m, t));
            return Ok(self.kosong_hasil(m));
        };
        let id = self.id_kanvas(&penerima);
        self.kanvas_panggil(&id, m, &nilai[1..], span)
    }

    /// Kalimat pintu native untuk jumlah argumen builtin (`interpreter::arity`), gagal saat jalan.
    fn aritas_kanvas(nama: &str, n: usize) -> Option<String> {
        use crate::interpreter::arity;
        let maks = arity::maks(nama);
        if let Some(m) = maks.filter(|m| n > *m) { return Some(arity::pesan_kebanyakan(nama, m, n)); }
        let min = arity::minimal(nama).filter(|m| n < *m)?;
        Some(arity::pesan_kurang(nama, min, maks, n, arity::tanda_tangan(nama)))
    }

    fn kosong_hasil(&mut self, m: &str) -> Val {
        match hasil_metode(m) {
            Hasil::Unit => Val::unit(),
            Hasil::Bool => Val { text: "0".into(), ty: JrgType::Bool },
            Hasil::Int => Val { text: "0".into(), ty: JrgType::I64 },
            Hasil::Float => Val { text: "0.0".into(), ty: JrgType::F64 },
            Hasil::Warna => { let t = self.tipe_warna(); Val { text: "zeroinitializer".into(), ty: t } }
            Hasil::Posisi => { let t = self.tipe_posisi(); Val { text: "zeroinitializer".into(), ty: t } }
        }
    }

    fn tipe_warna(&mut self) -> JrgType {
        if self.types.structs.contains_key("Warna") { return JrgType::Struct("Warna".into()); }
        if !self.types.structs.contains_key(LABEL_WARNA) {
            let f = ["r", "g", "b", "a"].iter().map(|n| (n.to_string(), JrgType::I64)).collect();
            self.types.register_struct(StructLayout { name: LABEL_WARNA.to_string(), fields: f });
        }
        JrgType::Struct(LABEL_WARNA.to_string())
    }

    fn id_kanvas(&mut self, v: &Val) -> String {
        let ty = self.tipe_kanvas();
        let r = self.fresh("kvid.");
        self.emit(format!("{} = extractvalue {} {}, 0", r, ty.llvm_ty(), v.text));
        r
    }

    fn pegangan_kanvas(&mut self, id: &str) -> Val {
        let ty = self.tipe_kanvas();
        let a = self.fresh("kv.");
        self.emit(format!("{} = insertvalue {} undef, i64 {}, 0", a, ty.llvm_ty(), id));
        Val { text: a, ty }
    }

    /// Hasil struct dibaca dari muatan kotaknya (`TAG_HASIL_*` di `runtime/src/canvas/mod.rs`).
    /// Tiga larik sejajar untuk runtime: kotak bertag, nama tipe acuan, dan tampilan di tempat teks `teks`.
    fn argumen_kanvas(&mut self, nilai: &[Val], teks: Option<usize>, span: Span) -> Result<(String, String, String, String), String> {
        let n = nilai.len().max(1);
        let kotak = self.alloca_raw("kvk.", &format!("[{} x %JrgAny]", n), None);
        let tipe = self.alloca_raw("kvt.", &format!("[{} x %JrgStr]", n), None);
        let tampil = self.alloca_raw("kvs.", &format!("[{} x %JrgStr]", n), None);
        let kanvas = self.tipe_kanvas();
        let mut butuh_kait = false;
        for (i, v) in nilai.iter().enumerate() {
            let k = self.kotak_any(v, span)?;
            let t = if v.ty == JrgType::Any { self.typeof_any(v)? }
                    else if v.ty == kanvas { self.intern_string("canvas") }
                    else { let n = v.ty.nama_typeof(); self.intern_string(&n) };
            let s = if teks == Some(i) { self.val_to_str(v)? } else { self.intern_string("") };
            butuh_kait |= v.ty == JrgType::Any || (matches!(v.ty, JrgType::Struct(_)) && v.ty != kanvas)
                || super::codegen_jembatan::mengandung_buram(&v.ty);
            for (tempat, jenis, isi) in [(&kotak, "%JrgAny", &k), (&tipe, "%JrgStr", &t), (&tampil, "%JrgStr", &s)] {
                let p = self.fresh("kvp.");
                self.emit(format!("{} = getelementptr [{} x {}], ptr {}, i64 0, i64 {}", p, n, jenis, tempat, i));
                self.emit(format!("store {} {}, ptr {}", jenis, isi.text, p));
            }
        }
        let kait = if butuh_kait { self.pakai_json_buram = true; "@jrg_buram_ke_json" } else { "null" };
        Ok((kotak, tipe, tampil, kait.to_string()))
    }

    fn kanvas_panggil(&mut self, id: &str, m: &str, nilai: &[Val], span: Span) -> Result<Val, String> {
        let (kotak, tipe, tampil, kait) = self.argumen_kanvas(nilai, crate::canvas::layan::tempat_teks(m), span)?;
        let kanvas = self.tipe_kanvas();
        let kata = self.kata_buram(&kanvas);
        self.ensure_extern("declare void @jrg_kanvas_panggil(i64, ptr, i64, ptr, ptr, ptr, i64, ptr, ptr, i32)",
            "jrg_kanvas_panggil");
        let nm = self.intern_string(m);
        let (mp, ml) = self.split_str(&nm);
        let out = self.alloca_typed("kvo.", &JrgType::Any);
        self.emit(format!("call void @jrg_kanvas_panggil(i64 {}, ptr {}, i64 {}, ptr {}, ptr {}, ptr {}, i64 {}, ptr {}, ptr {}, i32 {})",
            id, mp, ml, kotak, tipe, tampil, nilai.len(), out, kait, kata));
        let muatan = |cg: &mut Codegen, i: u32| {
            let p = cg.fresh("kvm.");
            cg.emit(format!("{} = getelementptr %JrgAny, ptr {}, i32 0, i32 1, i32 {}", p, out, i));
            let v = cg.fresh("kvm.");
            cg.emit(format!("{} = load i64, ptr {}", v, p));
            v
        };
        Ok(match hasil_metode(m) {
            Hasil::Unit => Val::unit(),
            Hasil::Bool => {
                let a = muatan(self, 0);
                let b = self.fresh("kvb.");
                self.emit(format!("{} = trunc i64 {} to i8", b, a));
                Val { text: b, ty: JrgType::Bool }
            }
            Hasil::Int => Val { text: muatan(self, 0), ty: JrgType::I64 },
            Hasil::Float => {
                let a = muatan(self, 0);
                let f = self.fresh("kvf.");
                self.emit(format!("{} = bitcast i64 {} to double", f, a));
                Val { text: f, ty: JrgType::F64 }
            }
            Hasil::Posisi => {
                let (a, b) = (muatan(self, 0), muatan(self, 1));
                let ty = self.tipe_posisi();
                let mut acc = "undef".to_string();
                for (i, bit) in [a, b].iter().enumerate() {
                    let f = self.fresh("kvf.");
                    self.emit(format!("{} = bitcast i64 {} to double", f, bit));
                    let s = self.fresh("kvs.");
                    self.emit(format!("{} = insertvalue {} {}, double {}, {}", s, ty.llvm_ty(), acc, f, i));
                    acc = s;
                }
                Val { text: acc, ty }
            }
            Hasil::Warna => {
                let a = muatan(self, 0);
                let mut kolom = Vec::new();
                for (i, nama) in ["r", "g", "b", "a"].iter().enumerate() {
                    let (g, d) = (self.fresh("kvw."), self.fresh("kvw."));
                    self.emit(format!("{} = lshr i64 {}, {}", g, a, i * 8));
                    self.emit(format!("{} = and i64 {}, 255", d, g));
                    let tempat = self.spill_to_place(&Val { text: d, ty: JrgType::I64 });
                    let e = self.lokal_sementara(tempat, JrgType::I64, span);
                    kolom.push(FieldInit { name: nama.to_string(), value: e, span: nol() });
                }
                let ty = self.tipe_warna();
                let JrgType::Struct(nama) = &ty else { unreachable!() };
                let lit = Expr::Struct {
                    ty: TypeExpr::Named(NamedType { package: None, modul: None, name: nama.clone(), args: vec![], span: nol() }),
                    fields: kolom, spread: None, span: nol(),
                };
                self.codegen_expr(&lit, Some(&ty))?
            }
        })
    }

    /// Jawaban penyaji jembatan (`jrg_kanvas_layar`); tanpa jembatan (kernel `--bebas`) memang tidak ada layar.
    fn kanvas_layar(&mut self, nama: &str) -> Val {
        Self::butuh_penyaji();
        self.ensure_extern("declare i32 @jrg_kanvas_layar()", "jrg_kanvas_layar");
        let (r, b, c) = (self.fresh("kvl."), self.fresh("kvl."), self.fresh("kvl."));
        self.emit(format!("{} = call i32 @jrg_kanvas_layar()", r));
        self.emit(format!("{} = and i32 {}, {}", b, r, if nama.ends_with("punya_jendela") { 1 } else { 2 }));
        self.emit(format!("{} = icmp ne i32 {}, 0", c, b));
        let d = self.fresh("kvl.");
        self.emit(format!("{} = zext i1 {} to i8", d, c));
        Val { text: d, ty: JrgType::Bool }
    }

    /// Jendela `"tab"` dan peramban `"web"` milik `jrgc`: jembatan ikut ditautkan, tapi sebagai CADANGAN, jadi
    /// kernel `--bebas` tetap menerima programnya (dan menjawab "butuh layar" saat mode itu diminta).
    fn butuh_penyaji() {
        super::JEMBATAN_CADANGAN.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// `Canvas::new(lebar, tinggi, judul?, mode?)`.
    fn kanvas_baru(&mut self, nama: &str, args: &[Expr], span: Span) -> Result<Val, String> {
        let tanpa_layar = |e: &Expr| matches!(e, Expr::Literal(crate::parser::LitKind::Str(s), _)
            if matches!(s.as_str(), "polos" | "default" | "png"));
        if args.get(3).is_some_and(|m| !tanpa_layar(m)) { Self::butuh_penyaji(); }
        let nilai = args.iter().map(|a| self.codegen_expr(a, None)).collect::<Result<Vec<_>, _>>()?;
        if let Some(p) = Self::aritas_kanvas(nama, nilai.len()) {
            self.panik_statis(&p);
            return Ok(self.pegangan_kanvas("0"));
        }
        let (kotak, tipe, tampil, kait) = self.argumen_kanvas(&nilai, Some(2), span)?;
        let kanvas = self.tipe_kanvas();
        let kata = self.kata_buram(&kanvas);
        self.ensure_extern("declare i64 @jrg_kanvas_baru(ptr, ptr, ptr, i64, ptr, i32)", "jrg_kanvas_baru");
        let id = self.fresh("kvid.");
        self.emit(format!("{} = call i64 @jrg_kanvas_baru(ptr {}, ptr {}, ptr {}, i64 {}, ptr {}, i32 {})",
            id, kotak, tipe, tampil, nilai.len(), kait, kata));
        Ok(self.pegangan_kanvas(&id))
    }

    /// `Canvas::load_png(path)`: path wajib teks, kalimatnya milik `stdlib/kanvas.rs`.
    fn kanvas_muat_png(&mut self, nama: &str, args: &[Expr], span: Span) -> Result<Val, String> {
        let nilai = args.iter().map(|a| self.codegen_expr(a, None)).collect::<Result<Vec<_>, _>>()?;
        if let Some(p) = Self::aritas_kanvas(nama, nilai.len()) {
            self.panik_statis(&p);
            return Ok(self.pegangan_kanvas("0"));
        }
        let pesan = |t: &str| format!("Canvas::load_png butuh 1 arg: (path), dapat `{}`", t);
        let path = match nilai[0].ty {
            JrgType::Str => nilai[0].clone(),
            JrgType::Any => {
                let tag = self.tag_any(&nilai[0]);
                let c = self.fresh("kv.");
                self.emit(format!("{} = icmp ne i32 {}, {}", c, tag.text, super::codegen_any::TAG_STR));
                let kotak = nilai[0].clone();
                self.gagal_bila(&c, |cg| cg.panik_tipe_any(&kotak, pesan));
                self.buka_any(&nilai[0], &JrgType::Str, span)?
            }
            ref lain => {
                let t = crate::ruang_tipe::bersih(&lain.nama_typeof()).into_owned();
                self.panik_statis(&pesan(&t));
                return Ok(self.pegangan_kanvas("0"));
            }
        };
        self.ensure_extern("declare i64 @jrg_kanvas_muat_png(ptr, i64)", "jrg_kanvas_muat_png");
        let (p, l) = self.split_str(&path);
        let id = self.fresh("kvid.");
        self.emit(format!("{} = call i64 @jrg_kanvas_muat_png(ptr {}, i64 {})", id, p, l));
        Ok(self.pegangan_kanvas(&id))
    }

    /// `canvas::nama_tombol(s)`: nama kanonik, yang asing ditolak dengan kalimat `layan::kenali_tombol`.
    fn kanvas_nama_tombol(&mut self, args: &[Expr], span: Span) -> Result<Val, String> {
        let v = self.codegen_expr(&args[0], None)?;
        let pesan = |t: &str| format!("canvas::nama_tombol: argumen ke-1 harus teks, dapat `{}`", t);
        let s = match v.ty {
            JrgType::Str => v,
            JrgType::Any => {
                let tag = self.tag_any(&v);
                let c = self.fresh("kv.");
                self.emit(format!("{} = icmp ne i32 {}, {}", c, tag.text, super::codegen_any::TAG_STR));
                let kotak = v.clone();
                self.gagal_bila(&c, |cg| cg.panik_tipe_any(&kotak, pesan));
                self.buka_any(&v, &JrgType::Str, span)?
            }
            ref lain => {
                let t = crate::ruang_tipe::bersih(&lain.nama_typeof()).into_owned();
                self.panik_statis(&pesan(&t));
                return Ok(self.intern_string(""));
            }
        };
        self.ensure_extern("declare void @jrg_kanvas_nama_tombol(ptr, i64, ptr, ptr)", "jrg_kanvas_nama_tombol");
        let (p, l) = self.split_str(&s);
        let outp = self.alloca_raw("kvnp.", "ptr", None);
        let outl = self.alloca_raw("kvnl.", "i64", None);
        self.emit(format!("call void @jrg_kanvas_nama_tombol(ptr {}, i64 {}, ptr {}, ptr {})", p, l, outp, outl));
        self.join_str(&outp, &outl)
    }

    /// Tampilan pegangan kanvas, sama dengan `Display` acuan (`<Canvas 64x48 'judul'>`).
    pub(crate) fn tampil_kanvas(&mut self, v: &Val) -> Result<Val, String> {
        let id = self.id_kanvas(v);
        self.ensure_extern("declare void @jrg_kanvas_tampil(i64, ptr, ptr)", "jrg_kanvas_tampil");
        let outp = self.alloca_raw("kvtp.", "ptr", None);
        let outl = self.alloca_raw("kvtl.", "i64", None);
        self.emit(format!("call void @jrg_kanvas_tampil(i64 {}, ptr {}, ptr {})", id, outp, outl));
        self.join_str(&outp, &outl)
    }
}
