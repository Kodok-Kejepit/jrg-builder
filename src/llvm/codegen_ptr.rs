// src/llvm/codegen_ptr.rs — `ptr::` di backend LLVM tanpa jembatan: sel bersama di `korutin.c`,
// isinya nilai bertag yang disalin sampai ke dalam saat masuk dan keluar, jadi jalan juga di `--bebas`.

use crate::lexer::Span;
use crate::parser::Expr;

use super::codegen::{Codegen, Val};
use super::types::{JrgType, StructLayout};

/// Label struct pegangan `ptr::`: tampil `Ptr`, tapi bukan `Ptr` milik `jawa::` atau program.
pub(crate) const LABEL_PTR: &str = "Ptr\u{E000}";

impl Codegen {
    pub(crate) fn tipe_ptr_sel(&mut self) -> JrgType {
        if !self.types.structs.contains_key(LABEL_PTR) {
            self.types.register_struct(StructLayout {
                name: LABEL_PTR.to_string(),
                fields: vec![("id".to_string(), JrgType::I64)],
            });
        }
        JrgType::Struct(LABEL_PTR.to_string())
    }

    /// Layani `ptr::…`; `None` kalau namanya bukan milik `ptr::` atau jumlah argumennya lain.
    pub(crate) fn coba_builtin_ptr(&mut self, nama: &str, args: &[Expr], span: Span) -> Option<Result<Val, String>> {
        let item = nama.strip_prefix("ptr::")?;
        let (item, n) = match item {
            "new" if args.len() <= 1 => ("new", args.len()),
            "null" => ("null", 0),
            "is_null" | "addr" | "get" | "free" | "inc" | "dec" => (item, 1),
            "deref" => ("get", 1),
            "set" | "update" => (item, 2),
            _ => return None,
        };
        if args.len() != n { return None; }
        Some(self.ptr(item, args, span))
    }

    fn ptr(&mut self, item: &str, args: &[Expr], span: Span) -> Result<Val, String> {
        let fname = format!("ptr::{}", item);
        match item {
            "new" => {
                let v = match args.first() {
                    Some(a) => self.codegen_expr(a, None)?,
                    None => Val { text: "zeroinitializer".into(), ty: JrgType::Any },
                };
                let k = self.kotak_any(&v, span)?;
                let slot = self.spill_to_place(&k);
                self.ensure_extern("declare i64 @jrg_ptr_baru(ptr)", "jrg_ptr_baru");
                let id = self.fresh("ptr.");
                self.emit(format!("{} = call i64 @jrg_ptr_baru(ptr {})", id, slot));
                Ok(self.pegangan_ptr(&id))
            }
            "null" => Ok(self.pegangan_ptr("0")),
            "update" => {
                let p = self.codegen_expr(&args[0], None)?;
                let f = self.turunkan_fungsi_arg(&args[1], &[JrgType::Any], span)?;
                let id = self.id_ptr(&p, &fname, span)?;
                self.buka_ptr(&id, &fname);
                self.ensure_extern("declare void @jrg_ptr_pegang(i64)", "jrg_ptr_pegang");
                self.emit(format!("call void @jrg_ptr_pegang(i64 {})", id));
                let lama = self.baca_ptr(&id);
                let r = match f {
                    Ok(fv) if matches!(fv.ty, JrgType::Fn(..)) => self.panggil_val(&fv, &[lama])?,
                    Ok(fv) => {
                        self.panik_statis(&crate::interpreter::pesan_bukan_fungsi(&fv.ty.nama_typeof()));
                        Val { text: "zeroinitializer".into(), ty: JrgType::Any }
                    }
                    Err(pesan) => {
                        self.panik_statis(&pesan);
                        Val { text: "zeroinitializer".into(), ty: JrgType::Any }
                    }
                };
                let r = if matches!(r.ty, JrgType::Unit | JrgType::Never) { Val::unit() } else { r };
                let k = self.kotak_any(&r, span)?;
                let slot = self.spill_to_place(&k);
                self.ensure_extern("declare void @jrg_ptr_tulis(i64, ptr)", "jrg_ptr_tulis");
                self.ensure_extern("declare void @jrg_ptr_lepas(i64)", "jrg_ptr_lepas");
                self.emit(format!("call void @jrg_ptr_tulis(i64 {}, ptr {})", id, slot));
                self.emit(format!("call void @jrg_ptr_lepas(i64 {})", id));
                Ok(self.load_place(&slot, &JrgType::Any))
            }
            "set" => {
                let p = self.codegen_expr(&args[0], None)?;
                let v = self.codegen_expr(&args[1], None)?;
                let id = self.id_ptr(&p, &fname, span)?;
                self.buka_ptr(&id, &fname);
                let k = self.kotak_any(&v, span)?;
                let slot = self.spill_to_place(&k);
                self.ensure_extern("declare void @jrg_ptr_tulis(i64, ptr)", "jrg_ptr_tulis");
                self.emit(format!("call void @jrg_ptr_tulis(i64 {}, ptr {})", id, slot));
                Ok(Val::unit())
            }
            _ => {
                let p = self.codegen_expr(&args[0], None)?;
                let id = self.id_ptr(&p, &fname, span)?;
                match item {
                    "is_null" | "addr" => {
                        if item == "addr" { return Ok(Val { text: id, ty: JrgType::I64 }); }
                        let (c, b) = (self.fresh("ptr."), self.fresh("ptr."));
                        self.emit(format!("{} = icmp eq i64 {}, 0", c, id));
                        self.emit(format!("{} = zext i1 {} to i8", b, c));
                        Ok(Val { text: b, ty: JrgType::Bool })
                    }
                    "free" => {
                        self.ensure_extern("declare void @jrg_ptr_free(i64)", "jrg_ptr_free");
                        self.emit(format!("call void @jrg_ptr_free(i64 {})", id));
                        Ok(Val::unit())
                    }
                    "get" => {
                        self.buka_ptr(&id, &fname);
                        Ok(self.baca_ptr(&id))
                    }
                    _ => self.langkah_ptr(&id, &fname, if item == "inc" { 1 } else { -1 }),
                }
            }
        }
    }

    fn pegangan_ptr(&mut self, id: &str) -> Val {
        let ty = self.tipe_ptr_sel();
        let a = self.fresh("ptr.");
        self.emit(format!("{} = insertvalue {} undef, i64 {}, 0", a, ty.llvm_ty(), id));
        Val { text: a, ty }
    }

    fn baca_ptr(&mut self, id: &str) -> Val {
        let slot = self.alloca_typed("ptrb.", &JrgType::Any);
        self.ensure_extern("declare void @jrg_ptr_baca(i64, ptr)", "jrg_ptr_baca");
        self.emit(format!("call void @jrg_ptr_baca(i64 {}, ptr {})", id, slot));
        self.load_place(&slot, &JrgType::Any)
    }

    /// Tunggu sel lepas dari `ptr::update` aktor lain; null, di-free dan diperbarui-aktor-ini gagal dengan kalimat acuan.
    fn buka_ptr(&mut self, id: &str, fname: &str) {
        self.ensure_extern("declare i32 @jrg_ptr_buka(i64)", "jrg_ptr_buka");
        let s = self.fresh("ptr.");
        self.emit(format!("{} = call i32 @jrg_ptr_buka(i64 {})", s, id));
        for (kode, pesan) in [
            (1, format!("{}: null pointer dereference", fname)),
            (3, crate::dok::ptr::pesan_sibuk(fname)),
            (2, format!("{}: pointer tidak valid (sudah di-free?)", fname)),
        ] {
            let c = self.fresh("ptr.");
            self.emit(format!("{} = icmp eq i32 {}, {}", c, s, kode));
            self.gagal_bila(&c, |cg| cg.panik(&pesan));
        }
    }

    fn langkah_ptr(&mut self, id: &str, fname: &str, d: i64) -> Result<Val, String> {
        self.buka_ptr(id, fname);
        let slot = self.alloca_typed("ptrl.", &JrgType::Any);
        self.ensure_extern("declare i32 @jrg_ptr_langkah(i64, i64, ptr)", "jrg_ptr_langkah");
        let s = self.fresh("ptr.");
        self.emit(format!("{} = call i32 @jrg_ptr_langkah(i64 {}, i64 {}, ptr {})", s, id, d, slot));
        let isi = self.load_place(&slot, &JrgType::Any);
        let c = self.fresh("ptr.");
        self.emit(format!("{} = icmp eq i32 {}, 4", c, s));
        let kotak = isi.clone();
        let f = fname.to_string();
        self.gagal_bila(&c, |cg| cg.panik_tipe_any(&kotak, |t| crate::dok::ptr::pesan_bukan_angka(&f, t)));
        let c = self.fresh("ptr.");
        self.emit(format!("{} = icmp eq i32 {}, 5", c, s));
        let kotak = isi.clone();
        let awal = format!("{}: ", fname);
        self.gagal_bila(&c, |cg| {
            let n = cg.buka_any(&kotak, &JrgType::I64, Span { lo: crate::lexer::BytePos(0), hi: crate::lexer::BytePos(0), file_id: 0 }).map(|v| v.text).unwrap_or_else(|_| "0".into());
            cg.ensure_extern("declare void @jrg_panic_luber_awal(ptr, i64, i64, i64, i32, i32, i32) cold noreturn",
                "jrg_panic_luber_awal");
            let a = cg.intern_string(&awal);
            let (ap, al) = cg.split_str(&a);
            let (b, op) = if d > 0 { (1, 0) } else { (1, 1) };
            cg.emit(format!("call void @jrg_panic_luber_awal(ptr {}, i64 {}, i64 {}, i64 {}, i32 {}, i32 64, i32 1)",
                ap, al, n, b, op));
            cg.terminate("unreachable".to_string());
        });
        Ok(isi)
    }

    /// Id sebuah pegangan `ptr::`; yang lain gagal SAAT JALAN dengan kalimat acuan.
    fn id_ptr(&mut self, v: &Val, fname: &str, span: Span) -> Result<String, String> {
        let ty = self.tipe_ptr_sel();
        if v.ty == ty {
            let r = self.fresh("ptrid.");
            self.emit(format!("{} = extractvalue {} {}, 0", r, ty.llvm_ty(), v.text));
            return Ok(r);
        }
        // `Ptr` milik `jawa::` (berfield `kind`), bukan struct program bernama sama.
        let jawa = self.types.structs.get("Ptr").and_then(|l| l.field_index("kind")) == Some(1);
        if v.ty == JrgType::Any {
            let kata = self.kata_buram(&ty);
            let slot = self.alloca_typed("ptrk.", &JrgType::Any);
            self.emit(format!("store %JrgAny {}, ptr {}", v.text, slot));
            let (w, c) = (self.fresh("ptrk."), self.fresh("ptrk."));
            self.emit(format!("{} = load i32, ptr {}", w, slot));
            self.emit(format!("{} = icmp ne i32 {}, {}", c, w, kata));
            let kotak = v.clone();
            let f = fname.to_string();
            let kata_jawa = jawa.then(|| self.kata_buram(&JrgType::Struct("Ptr".into())));
            self.gagal_bila(&c, |cg| {
                if let Some(kj) = kata_jawa {
                    let cj = cg.fresh("ptrk.");
                    cg.emit(format!("{} = icmp eq i32 {}, {}", cj, w, kj));
                    let p = crate::dok::ptr::pesan_handle_jawa(&f);
                    cg.gagal_bila(&cj, |cg| cg.panik(&p));
                }
                cg.panik_tipe_any(&kotak, |t| crate::dok::ptr::pesan_bukan_ptr(&f, t))
            });
            let buka = self.buka_any(v, &ty, span)?;
            return self.id_ptr(&buka, fname, span);
        }
        if jawa && v.ty == JrgType::Struct("Ptr".into()) {
            self.panik_statis(&crate::dok::ptr::pesan_handle_jawa(fname));
        } else {
            let t = crate::ruang_tipe::bersih(&v.ty.nama_typeof()).into_owned();
            self.panik_statis(&crate::dok::ptr::pesan_bukan_ptr(fname, &t));
        }
        Ok("0".into())
    }
}
