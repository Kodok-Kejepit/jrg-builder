//! `var x: T` tanpa nilai awal untuk T tanpa nilai nol: terbukti DIISI sebelum disebut di sisa bloknya?
//! Kalau ya, tempatnya tidak pernah terbaca kosong dan LLVM boleh menerimanya; selain itu tetap ditolak.

use std::collections::HashSet;
use crate::interpreter::capture::{kumpul_nama_blok, names_mentioned};
use crate::parser::{AssignOp, Block, Expr, IfStmt, Stmt};

#[derive(PartialEq, Clone, Copy)]
enum Hasil { Diisi, TakDisebut, Disebut }

fn kosong() -> crate::lexer::Span {
    crate::lexer::Span { lo: crate::lexer::BytePos(0), hi: crate::lexer::BytePos(0), file_id: 0 }
}

fn stmt_menyebut(st: &Stmt, nama: &str) -> bool {
    let b = Block { stmts: vec![st.clone()], tail: None, span: kosong() };
    let mut out = HashSet::new();
    kumpul_nama_blok(&b, &mut out);
    out.contains(nama)
}

fn ekspr_menyebut(e: &Expr, nama: &str) -> bool { names_mentioned(e).contains(nama) }

fn blok(b: &Block, nama: &str) -> Hasil { urutan(&b.stmts, b.tail.as_deref(), nama) }

fn urutan(stmts: &[Stmt], ekor: Option<&Expr>, nama: &str) -> Hasil {
    for st in stmts {
        match satu(st, nama) {
            Hasil::TakDisebut => {}
            h => return h,
        }
    }
    match ekor { Some(e) if ekspr_menyebut(e, nama) => Hasil::Disebut, _ => Hasil::TakDisebut }
}

fn satu(st: &Stmt, nama: &str) -> Hasil {
    match st {
        Stmt::Assign(a) if matches!(a.op, AssignOp::Eq) && a.targets.len() == 1 && a.values.len() == 1
            && matches!(&a.targets[0], Expr::Ident(n, _) if n == nama)
            && !ekspr_menyebut(&a.values[0], nama) => Hasil::Diisi,
        Stmt::If(s) => jika(st, s, nama),
        Stmt::Expr(e) => match &**e { Expr::If(s) => jika(st, s, nama), _ => umum(st, nama) },
        _ => umum(st, nama),
    }
}

fn umum(st: &Stmt, nama: &str) -> Hasil {
    if stmt_menyebut(st, nama) { Hasil::Disebut } else { Hasil::TakDisebut }
}

/// Semua cabang `if` (dengan `else`) mengisinya, dan syaratnya tidak menyebutnya.
fn jika(st: &Stmt, s: &IfStmt, nama: &str) -> Hasil {
    let bisa = s.init.is_none() && s.else_.is_some() && !ekspr_menyebut(&s.cond, nama)
        && s.else_ifs.iter().all(|(i, c, _)| i.is_none() && !ekspr_menyebut(c, nama));
    let semua = bisa && std::iter::once(&s.then).chain(s.else_ifs.iter().map(|(_, _, b)| b))
        .chain(s.else_.iter()).all(|b| blok(b, nama) == Hasil::Diisi);
    if semua { Hasil::Diisi } else { umum(st, nama) }
}

/// `sesudah`: pernyataan sesudah `var nama: T` di bloknya, lalu ekor blok.
pub fn terbukti(sesudah: &[Stmt], ekor: Option<&Expr>, nama: &str) -> bool {
    urutan(sesudah, ekor, nama) != Hasil::Disebut
}
