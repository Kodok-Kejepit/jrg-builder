//! Image disk yang bisa di-boot mesin UEFI (`jrgc --bebas --img`): MBR dengan satu partisi Sistem EFI (0xEF)
//! berisi FAT16 yang memuat `EFI/BOOT/BOOTX64.EFI`. Ditulis sendiri (tanpa mtools/root), jadi isinya pasti.

const SEKTOR: usize = 512;
const PER_KLASTER: usize = 4;                 // klaster 2 KiB
const MULAI_PARTISI: usize = 2048;            // rata 1 MiB, seperti alat partisi umum
const AKAR_ENTRI: usize = 512;

fn le16(b: &mut [u8], i: usize, v: u16) { b[i..i + 2].copy_from_slice(&v.to_le_bytes()); }
fn le32(b: &mut [u8], i: usize, v: u32) { b[i..i + 4].copy_from_slice(&v.to_le_bytes()); }

/// Entri direktori 8.3: `nama` sudah huruf besar, ≤ 8 + 3.
fn entri(nama: &str, ext: &str, dir: bool, klaster: u16, ukuran: u32) -> [u8; 32] {
    let mut e = [b' '; 32];
    e[..nama.len()].copy_from_slice(nama.as_bytes());
    e[8..8 + ext.len()].copy_from_slice(ext.as_bytes());
    for x in &mut e[11..] { *x = 0; }
    e[11] = if dir { 0x10 } else { 0x20 };
    le16(&mut e, 26, klaster);
    le32(&mut e, 28, ukuran);
    e
}

/// `efi` = isi `BOOTX64.EFI`. Ukuran volume mengikuti isinya (paling kecil 64 MiB, supaya tetap FAT16 sah).
pub fn buat(efi: &[u8]) -> Vec<u8> {
    let klaster_b = SEKTOR * PER_KLASTER;
    let perlu = efi.len().div_ceil(klaster_b) + 8;
    let klaster_n = perlu.max(32 * 1024).min(65_000);
    let fat_sektor = ((klaster_n + 2) * 2).div_ceil(SEKTOR);
    let cadangan = 4;
    let akar_sektor = AKAR_ENTRI * 32 / SEKTOR;
    let vol = cadangan + 2 * fat_sektor + akar_sektor + klaster_n * PER_KLASTER;
    let mut d = vec![0u8; (MULAI_PARTISI + vol) * SEKTOR];

    // MBR: satu partisi tipe 0xEF (Sistem EFI), CHS diisi "pakai LBA".
    {
        let m = &mut d[..SEKTOR];
        let p = 446;
        m[p] = 0x80;
        m[p + 1..p + 4].copy_from_slice(&[0xFE, 0xFF, 0xFF]);
        m[p + 4] = 0xEF;
        m[p + 5..p + 8].copy_from_slice(&[0xFE, 0xFF, 0xFF]);
        le32(m, p + 8, MULAI_PARTISI as u32);
        le32(m, p + 12, vol as u32);
        m[510] = 0x55; m[511] = 0xAA;
    }
    let v0 = MULAI_PARTISI * SEKTOR;
    // Sektor boot FAT16 (BPB).
    {
        let b = &mut d[v0..v0 + SEKTOR];
        b[0..3].copy_from_slice(&[0xEB, 0x3C, 0x90]);
        b[3..11].copy_from_slice(b"JRG-OS  ");
        le16(b, 11, SEKTOR as u16);
        b[13] = PER_KLASTER as u8;
        le16(b, 14, cadangan as u16);
        b[16] = 2;
        le16(b, 17, AKAR_ENTRI as u16);
        if vol < 65536 { le16(b, 19, vol as u16); } else { le32(b, 32, vol as u32); }
        b[21] = 0xF8;
        le16(b, 22, fat_sektor as u16);
        le16(b, 24, 32);
        le16(b, 26, 64);
        le32(b, 28, MULAI_PARTISI as u32);
        b[36] = 0x80;
        b[38] = 0x29;
        le32(b, 39, 0x4A52_4721);
        b[43..54].copy_from_slice(b"JRG-OS     ");
        b[54..62].copy_from_slice(b"FAT16   ");
        b[510] = 0x55; b[511] = 0xAA;
    }
    // Klaster: 2 = /EFI, 3 = /EFI/BOOT, 4.. = BOOTX64.EFI berurutan.
    let fat0 = v0 + cadangan * SEKTOR;
    let akar = fat0 + 2 * fat_sektor * SEKTOR;
    let data = akar + akar_sektor * SEKTOR;
    let efi_klaster = efi.len().div_ceil(klaster_b).max(1);
    let mut fat = vec![0u8; fat_sektor * SEKTOR];
    let mut tulis_fat = |k: usize, v: u16| le16(&mut fat, k * 2, v);
    tulis_fat(0, 0xFFF8);
    tulis_fat(1, 0xFFFF);
    tulis_fat(2, 0xFFFF);
    tulis_fat(3, 0xFFFF);
    for i in 0..efi_klaster {
        let k = 4 + i;
        tulis_fat(k, if i + 1 == efi_klaster { 0xFFFF } else { (k + 1) as u16 });
    }
    for salinan in 0..2 {
        let a = fat0 + salinan * fat_sektor * SEKTOR;
        d[a..a + fat.len()].copy_from_slice(&fat);
    }
    let di_klaster = |k: usize| data + (k - 2) * klaster_b;
    let mut label = entri("JRG-OS", "", false, 0, 0);
    label[11] = 0x08;
    d[akar..akar + 32].copy_from_slice(&label);
    d[akar + 32..akar + 64].copy_from_slice(&entri("EFI", "", true, 2, 0));
    let efi_dir = di_klaster(2);
    d[efi_dir..efi_dir + 32].copy_from_slice(&entri(".", "", true, 2, 0));
    d[efi_dir + 32..efi_dir + 64].copy_from_slice(&entri("..", "", true, 0, 0));
    d[efi_dir + 64..efi_dir + 96].copy_from_slice(&entri("BOOT", "", true, 3, 0));
    let boot_dir = di_klaster(3);
    d[boot_dir..boot_dir + 32].copy_from_slice(&entri(".", "", true, 3, 0));
    d[boot_dir + 32..boot_dir + 64].copy_from_slice(&entri("..", "", true, 2, 0));
    d[boot_dir + 64..boot_dir + 96].copy_from_slice(&entri("BOOTX64", "EFI", false, 4, efi.len() as u32));
    let isi = di_klaster(4);
    d[isi..isi + efi.len()].copy_from_slice(efi);
    d
}
