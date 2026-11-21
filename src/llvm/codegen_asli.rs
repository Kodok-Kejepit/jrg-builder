// src/llvm/codegen_asli.rs — `asli::`, akses perangkat keras untuk kernel `--bebas`: port I/O, memori volatil,
// register kendali dan MSR. Semuanya IR sebaris (asm/`load volatile`), bukan panggilan runtime.

use crate::lexer::Span;
use crate::parser::*;

use super::codegen::{Codegen, Val};
use super::types::*;

/// `(nama, jumlah argumen)` — satu daftar untuk codegen, pesan penolakan dan dokumentasi.
pub const NAMA_ASLI: &[(&str, usize)] = &[
    ("outb", 2), ("outw", 2), ("outl", 2), ("inb", 1), ("inw", 1), ("inl", 1),
    ("baca8", 1), ("baca16", 1), ("baca32", 1), ("baca64", 1),
    ("tulis8", 2), ("tulis16", 2), ("tulis32", 2), ("tulis64", 2),
    ("hlt", 0), ("cli", 0), ("sti", 0), ("pause", 0), ("rdtsc", 0),
    ("rdmsr", 1), ("wrmsr", 2), ("cpuid", 2), ("cr0", 0), ("cr2", 0), ("cr3", 0), ("cr4", 0),
    ("tulis_cr3", 1), ("invlpg", 1), ("interupsi", 2),
];

/// Kalimat untuk `asli::` di luar kernel `--bebas` — dipakai juga interpreter dan VM.
pub fn pesan_bukan_bebas(nama: &str) -> String {
    format!("`asli::{}` menyentuh perangkat keras langsung, jadi cuma ada di kernel `jrgc --bebas`; program ini \
             jalan di atas sistem operasi", nama)
}

const PEMBERSIH: &str = "~{dirflag},~{fpsr},~{flags}";

impl Codegen {
    fn asli_bulat(&mut self, nama: &str, e: &Expr) -> Result<Val, String> {
        let v = self.codegen_expr(e, Some(&JrgType::I64))?;
        if !v.ty.is_int() {
            return Err(super::galat::tandai(e.span(), format!(
                "`asli::{}` butuh bilangan bulat, dapat `{}`", nama, v.ty.nama_typeof())));
        }
        Ok(self.ke_i64_terperiksa(&v, &format!("asli::{}", nama)))
    }

    /// `v` di luar `0..=maks` gagal saat jalan menyebut nilainya — tidak pernah dipotong diam-diam.
    fn asli_rentang(&mut self, nama: &str, apa: &str, v: &Val, maks: u64) {
        let c = self.fresh("asr.");
        self.emit(format!("{} = icmp ugt i64 {}, {}", c, v.text, maks));
        let pesan = format!("`asli::{}`: {} {} di luar rentang 0..={}, tidak dipotong", nama, apa, super::codegen_konversi::PENANDA, maks);
        let vv = v.clone();
        self.gagal_bila(&c, |cg| cg.panik_sisip(&pesan, &super::codegen_konversi::Cetak::Bulat(vv)));
    }

    fn asli_potong(&mut self, v: &Val, lebar: u32) -> String {
        if lebar == 64 { return v.text.clone(); }
        let t = self.fresh("asp.");
        self.emit(format!("{} = trunc i64 {} to i{}", t, v.text, lebar));
        t
    }

    fn asli_lebar(&mut self, s: &str, lebar: u32) -> Val {
        if lebar == 64 { return Val { text: s.to_string(), ty: JrgType::I64 }; }
        let t = self.fresh("asz.");
        self.emit(format!("{} = zext i{} {} to i64", t, lebar, s));
        Val { text: t, ty: JrgType::I64 }
    }

    pub(crate) fn codegen_asli(&mut self, item: &str, args: &[Expr], span: Span) -> Result<Val, String> {
        let Some(&(_, n)) = NAMA_ASLI.iter().find(|(m, _)| *m == item) else {
            return Err(super::galat::tandai(span, format!("`asli::{}` tidak ada — yang ada: {}.", item,
                NAMA_ASLI.iter().map(|(m, _)| format!("`{}`", m)).collect::<Vec<_>>().join(", "))));
        };
        if args.len() != n {
            return Err(super::galat::tandai(span, format!("`asli::{}` butuh {} argumen, dapat {} argumen", item, n, args.len())));
        }
        if !self.options.bebas {
            return Err(super::galat::tandai(span, pesan_bukan_bebas(item)));
        }
        // `asli::interupsi(vektor, f)`: `f(vektor)` dipanggil di stack IST tiap interupsi itu datang.
        if item == "interupsi" {
            let v = self.asli_bulat(item, &args[0])?;
            self.asli_rentang(item, "vektor", &v, 255);
            let ty = JrgType::Fn(vec![JrgType::I64], Box::new(JrgType::Unit));
            let f = self.codegen_expr(&args[1], Some(&ty))?;
            if !matches!(f.ty, JrgType::Fn(..)) {
                return Err(super::galat::tandai(args[1].span(), format!(
                    "`asli::interupsi` butuh fungsi `fn(int)` sebagai penangan, dapat `{}`", f.ty.nama_typeof())));
            }
            let f = self.coerce(&f, &ty);
            let (k, e) = (self.fresh("asi."), self.fresh("asi."));
            self.emit(format!("{} = extractvalue %JrgFn {}, 0", k, f.text));
            self.emit(format!("{} = extractvalue %JrgFn {}, 1", e, f.text));
            self.ensure_extern("declare void @jrg_bebas_pasang_penangan(i64, ptr, ptr)", "jrg_bebas_pasang_penangan");
            self.emit(format!("call void @jrg_bebas_pasang_penangan(i64 {}, ptr {}, ptr {})", v.text, k, e));
            return Ok(Val::unit());
        }
        let mut a = Vec::new();
        for e in args { a.push(self.asli_bulat(item, e)?); }
        let lebar_port = |m: &str| match &m[m.len() - 1..] { "b" => 8u32, "w" => 16, _ => 32 };
        let unit = Val::unit();
        match item {
            "outb" | "outw" | "outl" => {
                let w = lebar_port(item);
                self.asli_rentang(item, "port", &a[0], 0xFFFF);
                self.asli_rentang(item, "nilai", &a[1], (1u64 << w) - 1);
                let (p, v) = (self.asli_potong(&a[0], 16), self.asli_potong(&a[1], w));
                let r = match w { 8 => "al", 16 => "ax", _ => "eax" };
                self.emit(format!("call void asm sideeffect \"out{} $0, $1\", \"{{{}}},{{dx}},{}\"(i{} {}, i16 {})",
                    &item[3..], r, PEMBERSIH, w, v, p));
                Ok(unit)
            }
            "inb" | "inw" | "inl" => {
                let w = lebar_port(item);
                self.asli_rentang(item, "port", &a[0], 0xFFFF);
                let p = self.asli_potong(&a[0], 16);
                let r = match w { 8 => "al", 16 => "ax", _ => "eax" };
                let t = self.fresh("asin.");
                self.emit(format!("{} = call i{} asm sideeffect \"in{} $1, $0\", \"={{{}}},{{dx}},{}\"(i16 {})",
                    t, w, &item[2..], r, PEMBERSIH, p));
                Ok(self.asli_lebar(&t, w))
            }
            _ if item.starts_with("baca") => {
                let w: u32 = item[4..].parse().unwrap_or(64);
                let (p, t) = (self.fresh("asa."), self.fresh("asb."));
                self.emit(format!("{} = inttoptr i64 {} to ptr", p, a[0].text));
                self.emit(format!("{} = load volatile i{}, ptr {}, align 1", t, w, p));
                Ok(self.asli_lebar(&t, w))
            }
            _ if item.starts_with("tulis") && item != "tulis_cr3" => {
                let w: u32 = item[5..].parse().unwrap_or(64);
                if w < 64 { self.asli_rentang(item, "nilai", &a[1], (1u64 << w) - 1); }
                let v = self.asli_potong(&a[1], w);
                let p = self.fresh("asa.");
                self.emit(format!("{} = inttoptr i64 {} to ptr", p, a[0].text));
                self.emit(format!("store volatile i{} {}, ptr {}, align 1", w, v, p));
                Ok(unit)
            }
            "hlt" | "cli" | "sti" | "pause" => {
                self.emit(format!("call void asm sideeffect \"{}\", \"~{{memory}},{}\"()", item, PEMBERSIH));
                Ok(unit)
            }
            "rdtsc" => {
                let t = self.fresh("ast.");
                self.emit(format!("{} = call i64 @llvm.readcyclecounter()", t));
                self.ensure_extern("declare i64 @llvm.readcyclecounter()", "llvm.readcyclecounter");
                Ok(Val { text: t, ty: JrgType::I64 })
            }
            "rdmsr" => {
                self.asli_rentang(item, "MSR", &a[0], 0xFFFF_FFFF);
                let m = self.asli_potong(&a[0], 32);
                let lo = self.fresh("asm.");
                self.emit(format!("{} = call {{ i32, i32 }} asm sideeffect \"rdmsr\", \"={{eax}},={{edx}},{{ecx}},{}\"(i32 {})",
                    lo, PEMBERSIH, m));
                let (a0, d0) = (self.fresh("asm."), self.fresh("asm."));
                self.emit(format!("{} = extractvalue {{ i32, i32 }} {}, 0", a0, lo));
                self.emit(format!("{} = extractvalue {{ i32, i32 }} {}, 1", d0, lo));
                let (az, dz, ds, g) = (self.fresh("asm."), self.fresh("asm."), self.fresh("asm."), self.fresh("asm."));
                self.emit(format!("{} = zext i32 {} to i64", az, a0));
                self.emit(format!("{} = zext i32 {} to i64", dz, d0));
                self.emit(format!("{} = shl i64 {}, 32", ds, dz));
                self.emit(format!("{} = or i64 {}, {}", g, ds, az));
                Ok(Val { text: g, ty: JrgType::I64 })
            }
            "wrmsr" => {
                self.asli_rentang(item, "MSR", &a[0], 0xFFFF_FFFF);
                let m = self.asli_potong(&a[0], 32);
                let (lo, hs, hi) = (self.fresh("asw."), self.fresh("asw."), self.fresh("asw."));
                self.emit(format!("{} = trunc i64 {} to i32", lo, a[1].text));
                self.emit(format!("{} = lshr i64 {}, 32", hs, a[1].text));
                self.emit(format!("{} = trunc i64 {} to i32", hi, hs));
                self.emit(format!("call void asm sideeffect \"wrmsr\", \"{{ecx}},{{eax}},{{edx}},{}\"(i32 {}, i32 {}, i32 {})",
                    PEMBERSIH, m, lo, hi));
                Ok(unit)
            }
            "cpuid" => {
                self.asli_rentang(item, "leaf", &a[0], 0xFFFF_FFFF);
                self.asli_rentang(item, "subleaf", &a[1], 0xFFFF_FFFF);
                let (l, s) = (self.asli_potong(&a[0], 32), self.asli_potong(&a[1], 32));
                let r = self.fresh("asc.");
                self.emit(format!("{} = call {{ i32, i32, i32, i32 }} asm sideeffect \"cpuid\", \
                    \"={{eax}},={{ebx}},={{ecx}},={{edx}},{{eax}},{{ecx}},{}\"(i32 {}, i32 {})", r, PEMBERSIH, l, s));
                let ty = JrgType::Tuple(vec![JrgType::I64; 4]);
                let mut kini = "undef".to_string();
                for i in 0..4 {
                    let e = self.fresh("asc.");
                    self.emit(format!("{} = extractvalue {{ i32, i32, i32, i32 }} {}, {}", e, r, i));
                    let z = self.asli_lebar(&e, 32);
                    let n = self.fresh("asc.");
                    self.emit(format!("{} = insertvalue {} {}, i64 {}, {}", n, ty.llvm_ty(), kini, z.text, i));
                    kini = n;
                }
                Ok(Val { text: kini, ty })
            }
            "cr0" | "cr2" | "cr3" | "cr4" => {
                let t = self.fresh("asr.");
                self.emit(format!("{} = call i64 asm sideeffect \"mov %{}, $0\", \"=r,{}\"()", t, item, PEMBERSIH));
                Ok(Val { text: t, ty: JrgType::I64 })
            }
            "tulis_cr3" => {
                self.emit(format!("call void asm sideeffect \"mov $0, %cr3\", \"r,~{{memory}},{}\"(i64 {})", PEMBERSIH, a[0].text));
                Ok(unit)
            }
            "invlpg" => {
                let p = self.fresh("asa.");
                self.emit(format!("{} = inttoptr i64 {} to ptr", p, a[0].text));
                self.emit(format!("call void asm sideeffect \"invlpg ($0)\", \"r,~{{memory}},{}\"(ptr {})", PEMBERSIH, p));
                Ok(unit)
            }
            _ => unreachable!("NAMA_ASLI dan codegen_asli tidak sepakat soal `{}`", item),
        }
    }
}
