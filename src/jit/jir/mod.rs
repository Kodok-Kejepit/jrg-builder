//! JIR — IR SSA milik JRGJIT (`dokumentasi/jit/jir-rencana.md`): dibangun dari bytecode dan `layak::Rencana`,
//! diperiksa (`periksa`), diturunkan ke CLIF (`turun`) dengan status, deopt dan jaga yang sama dengan penurun
//! langsung (`super::turun`), yang tetap menurunkan unit di luar cakupannya.

pub mod bangun;
pub mod periksa;
pub mod turun;

use std::fmt;

/// Satu nilai SSA (didefinisikan tepat sekali: parameter blok atau hasil instruksi).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Nilai(pub u32);

/// Satu blok dasar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Blok(pub u32);

/// Tipe nilai: bulat 64 bit (juga `bool` sebagai 0/1) atau pecahan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipe { I, F }

/// Perbandingan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Banding { Lt, Le, Gt, Ge, Eq, Ne }

/// Operasi dua operand. `I*` atas bulat, `F*` atas pecahan; perbandingan menghasilkan bulat 0/1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Biner {
    /// Tambah, kurang, kali bulat dengan luapan 64 bit dijaga.
    ITambah, IKurang, IKali,
    /// Bagi dan sisa bulat; pembagi 0 dan -1 dijaga (kalimat galat, `MIN / -1`, aturan VM).
    IBagi, ISisa,
    IDan, IAtau, IXor, IDanBukan,
    /// Geser kiri: geseran di atas 63 dan bit yang hilang dijaga.
    IGeserKiri,
    /// Geser kanan aritmetika: geseran 128 ke atas dijaga, 64–127 sama dengan 63.
    IGeserKanan,
    ICmp(Banding),
    FTambah, FKurang, FKali, FBagi,
    FCmp(Banding),
}

/// Operasi satu operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unar {
    /// Negasi bulat; `MIN` dijaga.
    INeg,
    FNeg,
    /// `!` atas `bool` (0/1).
    Bukan,
    /// `~` bulat.
    IBukanBit,
    /// Kebenaran bulat/`bool`: bukan nol.
    KeBool,
    /// Bulat ke pecahan, seperti `i128 as f64` acuan untuk nilai 64 bit.
    KeF,
}

/// Keadaan bingkai VM di titik deopt: instruksi tempat VM melanjutkan dan isi native tiap posisi stack (`None` =
/// posisi tanpa isi native — `nil`, `()` — yang dibangun `deopt::pulihkan` dari jenisnya).
#[derive(Clone, Debug, PartialEq)]
pub struct Keadaan {
    pub ip: usize,
    pub isi: Vec<Option<Nilai>>,
}

/// Jaga yang gagal = deopt di `keadaan`. `spek`: kegagalannya dicatat (`GAGAL_SPEK`), kompilasi berikutnya tidak
/// berspekulasi lagi di sana.
#[derive(Clone, Debug, PartialEq)]
pub struct Jaga {
    pub keadaan: Keadaan,
    pub spek: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Inst {
    KonstI { hasil: Nilai, nilai: i64 },
    KonstF { hasil: Nilai, nilai: f64 },
    /// `jaga` = kegagalan operasinya sendiri (luapan, pembagi, geseran); `None` = terbukti tidak gagal.
    Biner { hasil: Nilai, op: Biner, a: Nilai, b: Nilai, jaga: Option<Jaga> },
    Unar { hasil: Nilai, op: Unar, a: Nilai, jaga: Option<Jaga> },
    /// `jika ? ya : tidak`.
    Pilih { hasil: Nilai, jika: Nilai, ya: Nilai, tidak: Nilai },
    /// `kondisi` bukan nol = deopt.
    JagaJika { kondisi: Nilai, jaga: Jaga },
    /// Undian deopt acak (`JRG_JIT_STRES=deopt-acak`): 1 kira-kira sekali tiap 16.
    Undi { hasil: Nilai },
    /// Panggilan langsung ke callee bulat `Rencana::callee[callee]` (`tanda_tangan_bulat`). Callee yang belum mulai
    /// (`ULANG`) = deopt di `di` (VM yang memanggil); callee yang deopt = deopt di `sesudah` (VM melanjutkan callee-nya).
    Panggil { hasil: Nilai, callee: usize, args: Vec<Nilai>, di: Keadaan, sesudah: Keadaan },
}

/// Status `Return` native (`super::turun::BERHASIL*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hasil { Bulat, Bool, Pecahan }

#[derive(Clone, Debug, PartialEq)]
pub enum Akhir {
    Lompat { ke: Blok, args: Vec<Nilai> },
    Cabang { kondisi: Nilai, ya: Blok, ya_args: Vec<Nilai>, tidak: Blok, tidak_args: Vec<Nilai> },
    Kembali { nilai: Nilai, hasil: Hasil },
    /// Titik keluar: VM menjalankan instruksi `keadaan.ip`.
    Keluar { keadaan: Keadaan },
}

#[derive(Clone, Debug)]
pub struct DataBlok {
    /// Instruksi bytecode tempat blok ini mulai.
    pub ip: usize,
    pub params: Vec<Nilai>,
    pub inst: Vec<Inst>,
    pub akhir: Option<Akhir>,
}

/// Satu titik masuk yang dikompilasi.
#[derive(Clone, Debug)]
pub struct Fungsi {
    pub blok: Vec<DataBlok>,
    /// Blok pertama: parameternya isi native posisi stack titik masuk (`Rencana::jenis_masuk`) yang skalar.
    pub masuk: Blok,
    /// Tipe tiap nilai, menurut nomornya.
    pub tipe: Vec<Tipe>,
    /// Posisi stack titik masuk yang menjadi parameter blok masuk, urut.
    pub posisi_masuk: Vec<usize>,
    /// Panjang stack terbesar di titik deopt mana pun (ukuran slot rekaman).
    pub maks: usize,
}

impl Fungsi {
    pub fn nilai_baru(&mut self, t: Tipe) -> Nilai {
        self.tipe.push(t);
        Nilai(self.tipe.len() as u32 - 1)
    }

    pub fn tipe(&self, v: Nilai) -> Tipe { self.tipe[v.0 as usize] }
}

impl fmt::Display for Nilai {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "v{}", self.0) }
}

impl fmt::Display for Blok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "b{}", self.0) }
}

fn daftar(v: &[Nilai]) -> String { v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ") }

impl fmt::Display for Keadaan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let isi: Vec<String> = self.isi.iter().map(|x| x.map_or("_".into(), |v| v.to_string())).collect();
        write!(f, "@{} [{}]", self.ip, isi.join(" "))
    }
}

impl fmt::Display for Fungsi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, b) in self.blok.iter().enumerate() {
            let params: Vec<String> = b.params.iter().map(|p| format!("{p}:{:?}", self.tipe(*p))).collect();
            writeln!(f, "b{i} (ip {}) ({}):", b.ip, params.join(", "))?;
            for ins in &b.inst {
                let jaga = |j: &Option<Jaga>| j.as_ref().map_or(String::new(), |j| format!(" jaga{} {}", if j.spek { "-spek" } else { "" }, j.keadaan));
                match ins {
                    Inst::KonstI { hasil, nilai } => writeln!(f, "  {hasil} = {nilai}")?,
                    Inst::KonstF { hasil, nilai } => writeln!(f, "  {hasil} = {nilai:?}")?,
                    Inst::Biner { hasil, op, a, b, jaga: j } => writeln!(f, "  {hasil} = {op:?} {a}, {b}{}", jaga(j))?,
                    Inst::Unar { hasil, op, a, jaga: j } => writeln!(f, "  {hasil} = {op:?} {a}{}", jaga(j))?,
                    Inst::Pilih { hasil, jika, ya, tidak } => writeln!(f, "  {hasil} = pilih {jika} ? {ya} : {tidak}")?,
                    Inst::JagaJika { kondisi, jaga: j } => writeln!(f, "  jaga {kondisi}{}", jaga(&Some(j.clone())))?,
                    Inst::Undi { hasil } => writeln!(f, "  {hasil} = undi")?,
                    Inst::Panggil { hasil, callee, args, di, sesudah } =>
                        writeln!(f, "  {hasil} = panggil #{callee}({}) ulang {di} deopt {sesudah}", daftar(args))?,
                }
            }
            match &b.akhir {
                Some(Akhir::Lompat { ke, args }) => writeln!(f, "  lompat {ke}({})", daftar(args))?,
                Some(Akhir::Cabang { kondisi, ya, ya_args, tidak, tidak_args }) =>
                    writeln!(f, "  cabang {kondisi} ? {ya}({}) : {tidak}({})", daftar(ya_args), daftar(tidak_args))?,
                Some(Akhir::Kembali { nilai, hasil }) => writeln!(f, "  kembali {nilai} ({hasil:?})")?,
                Some(Akhir::Keluar { keadaan }) => writeln!(f, "  keluar {keadaan}")?,
                None => writeln!(f, "  <tanpa akhir>")?,
            }
        }
        Ok(())
    }
}

/// `JRG_JIT_JIR=paksa`: unit yang dicakup JIR diturunkan lewat JIR (fase JIR-1; penurun langsung tetap bawaan).
pub fn aktif() -> bool {
    static A: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *A.get_or_init(|| std::env::var("JRG_JIT_JIR").is_ok_and(|v| v == "paksa"))
}

/// Unit yang diturunkan lewat JIR di proses ini (untuk `JRG_JIT_LOG=ringkas` dan tes).
pub static UNIT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
