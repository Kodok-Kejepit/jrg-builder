// src/dok/crypto.rs — crypto:: — hashing (MD5/SHA-1/SHA-256/SHA-512), HMAC-SHA256, CRC32,
// Base64/Hex, random aman (OS RNG), dan AES-256-CBC. Semuanya ditulis

use std::sync::atomic::{AtomicU64, Ordering};

use crate::interpreter::{err_value, ok_value, Value};
pub use crate::kripto_inti::*;

// ─── Helpers Value <-> bytes/str ────────────────────────────────────────

/// Isi yang diolah sebagai byte: teks (byte UTF-8-nya) atau array byte.
fn isi_byte(v: &Value, fname: &str) -> Result<Vec<u8>, String> {
    match v {
        Value::Array(a) => array_ke_byte(a, fname),
        lain => Ok(as_str(lain, fname)?.as_bytes().to_vec()),
    }
}

fn as_str<'a>(v: &'a Value, fname: &str) -> Result<&'a str, String> {
    match v { Value::Str(s) => Ok(s.as_str()), other => Err(format!("{}: butuh string, dapat `{}`", fname, other.type_name())) }
}

// ============================================================================
//  HASHING
// ============================================================================

const MD5_S: [u32; 64] = [
    7,12,17,22, 7,12,17,22, 7,12,17,22, 7,12,17,22,
    5, 9,14,20, 5, 9,14,20, 5, 9,14,20, 5, 9,14,20,
    4,11,16,23, 4,11,16,23, 4,11,16,23, 4,11,16,23,
    6,10,15,21, 6,10,15,21, 6,10,15,21, 6,10,15,21,
];
const MD5_K: [u32; 64] = [
    0xd76aa478,0xe8c7b756,0x242070db,0xc1bdceee,0xf57c0faf,0x4787c62a,0xa8304613,0xfd469501,
    0x698098d8,0x8b44f7af,0xffff5bb1,0x895cd7be,0x6b901122,0xfd987193,0xa679438e,0x49b40821,
    0xf61e2562,0xc040b340,0x265e5a51,0xe9b6c7aa,0xd62f105d,0x02441453,0xd8a1e681,0xe7d3fbc8,
    0x21e1cde6,0xc33707d6,0xf4d50d87,0x455a14ed,0xa9e3e905,0xfcefa3f8,0x676f02d9,0x8d2a4c8a,
    0xfffa3942,0x8771f681,0x6d9d6122,0xfde5380c,0xa4beea44,0x4bdecfa9,0xf6bb4b60,0xbebfbc70,
    0x289b7ec6,0xeaa127fa,0xd4ef3085,0x04881d05,0xd9d4d039,0xe6db99e5,0x1fa27cf8,0xc4ac5665,
    0xf4292244,0x432aff97,0xab9423a7,0xfc93a039,0x655b59c3,0x8f0ccc92,0xffeff47d,0x85845dd1,
    0x6fa87e4f,0xfe2ce6e0,0xa3014314,0x4e0811a1,0xf7537e82,0xbd3af235,0x2ad7d2bb,0xeb86d391,
];

pub fn md5(input: &[u8]) -> [u8; 16] {
    let (mut a0, mut b0, mut c0, mut d0): (u32, u32, u32, u32) = (0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476);
    let mut msg = input.to_vec();
    let bitlen = (input.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 { msg.push(0); }
    msg.extend_from_slice(&bitlen.to_le_bytes());

    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for i in 0..16 { m[i] = u32::from_le_bytes([chunk[i*4], chunk[i*4+1], chunk[i*4+2], chunk[i*4+3]]); }
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = if i < 16 { ((b & c) | (!b & d), i) }
                else if i < 32 { ((d & b) | (!d & c), (5*i + 1) % 16) }
                else if i < 48 { (b ^ c ^ d, (3*i + 5) % 16) }
                else { (c ^ (b | !d), (7*i) % 16) };
            let f = f.wrapping_add(a).wrapping_add(MD5_K[i]).wrapping_add(m[g]);
            a = d; d = c; c = b; b = b.wrapping_add(f.rotate_left(MD5_S[i]));
        }
        a0 = a0.wrapping_add(a); b0 = b0.wrapping_add(b); c0 = c0.wrapping_add(c); d0 = d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a0.to_le_bytes()); out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes()); out[12..16].copy_from_slice(&d0.to_le_bytes());
    out
}

pub fn sha1(input: &[u8]) -> [u8; 20] {
    let (mut h0, mut h1, mut h2, mut h3, mut h4): (u32,u32,u32,u32,u32) =
        (0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0);
    let mut msg = input.to_vec();
    let ml = (input.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 { msg.push(0); }
    msg.extend_from_slice(&ml.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 { w[i] = u32::from_be_bytes([chunk[i*4], chunk[i*4+1], chunk[i*4+2], chunk[i*4+3]]); }
        for i in 16..80 { w[i] = (w[i-3] ^ w[i-8] ^ w[i-14] ^ w[i-16]).rotate_left(1); }
        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = if i < 20 { ((b & c) | ((!b) & d), 0x5A827999u32) }
                else if i < 40 { (b ^ c ^ d, 0x6ED9EBA1u32) }
                else if i < 60 { ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32) }
                else { (b ^ c ^ d, 0xCA62C1D6u32) };
            let temp = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(wi);
            e = d; d = c; c = b.rotate_left(30); b = a; a = temp;
        }
        h0 = h0.wrapping_add(a); h1 = h1.wrapping_add(b); h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d); h4 = h4.wrapping_add(e);
    }
    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes()); out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes()); out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}




pub use crate::kompres::crc32;


// ─── Wrapper native (Value-level) untuk hashing ─────────────────────────

pub fn f_md5(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(std::sync::Arc::new(hex_encode(&md5(&isi_byte(args.get(0).unwrap_or(&Value::Nil), "crypto::md5")?)))))
}
pub fn f_sha1(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(std::sync::Arc::new(hex_encode(&sha1(&isi_byte(args.get(0).unwrap_or(&Value::Nil), "crypto::sha1")?)))))
}
pub fn f_sha256(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(std::sync::Arc::new(hex_encode(&sha256(&isi_byte(args.get(0).unwrap_or(&Value::Nil), "crypto::sha256")?)))))
}
pub fn f_sha512(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(std::sync::Arc::new(hex_encode(&sha512(&isi_byte(args.get(0).unwrap_or(&Value::Nil), "crypto::sha512")?)))))
}
pub fn f_hmac_sha256(args: &[Value]) -> Result<Value, String> {
    let key = isi_byte(args.get(0).unwrap_or(&Value::Nil), "crypto::hmac_sha256")?;
    let msg = isi_byte(args.get(1).unwrap_or(&Value::Nil), "crypto::hmac_sha256")?;
    Ok(Value::Str(std::sync::Arc::new(hex_encode(&hmac_sha256(&key, &msg)))))
}
/// `crypto::compare_digest(a, b)`: padanan `hmac.compare_digest` Python, untuk tanda tangan dan token.
pub fn f_compare_digest(args: &[Value]) -> Result<Value, String> {
    let a = isi_byte(args.get(0).unwrap_or(&Value::Nil), "crypto::compare_digest")?;
    let b = isi_byte(args.get(1).unwrap_or(&Value::Nil), "crypto::compare_digest")?;
    Ok(Value::Bool(sama_waktu_tetap(&a, &b)))
}
pub fn f_crc32(args: &[Value]) -> Result<Value, String> {
    let s = isi_byte(args.get(0).unwrap_or(&Value::Nil), "crypto::crc32")?;
    Ok(Value::Int(crc32(&s) as i128))
}

// ============================================================================
//  ENCODING: Base64 / Hex
// ============================================================================


pub fn f_base64url_encode(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(std::sync::Arc::new(base64url_encode_bytes(&isi_byte(args.get(0).unwrap_or(&Value::Nil), "crypto::base64url_encode")?))))
}
pub fn f_base64url_decode(args: &[Value]) -> Result<Value, String> {
    let s = as_str(args.get(0).unwrap_or(&Value::Nil), "crypto::base64url_decode")?;
    match base64url_decode_str(s) {
        Ok(bytes) => Ok(teks_atau_err(bytes, "crypto::base64url_decode", "pakai `crypto::base64url_decode_bytes` untuk mengambil byte-nya")),
        Err(e) => Ok(err_value(e)),
    }
}
pub fn f_base64url_decode_bytes(args: &[Value]) -> Result<Value, String> {
    let s = as_str(args.get(0).unwrap_or(&Value::Nil), "crypto::base64url_decode_bytes")?;
    Ok(match base64url_decode_str(s) { Ok(b) => ok_value(byte_ke_array(b)), Err(e) => err_value(e) })
}


pub fn f_base64_encode(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(std::sync::Arc::new(base64_encode_bytes(&isi_byte(args.get(0).unwrap_or(&Value::Nil), "crypto::base64_encode")?))))
}
pub fn f_base64_decode(args: &[Value]) -> Result<Value, String> {
    let s = as_str(args.get(0).unwrap_or(&Value::Nil), "crypto::base64_decode")?;
    match base64_decode_str(s) {
        Ok(bytes) => Ok(teks_atau_err(bytes, "crypto::base64_decode", "pakai `crypto::base64_decode_bytes` untuk mengambil byte-nya")),
        Err(e) => Ok(err_value(e)),
    }
}

/// Byte hasil dekode sebagai teks. Yang bukan UTF-8 sah dijawab `Err` yang menyebut jalan
/// keluarnya; dulu byte rusaknya diganti U+FFFD tanpa suara.
fn teks_atau_err(bytes: Vec<u8>, nama: &str, jalan: &str) -> Value {
    let n = bytes.len();
    match String::from_utf8(bytes) {
        Ok(s) => ok_value(Value::Str(std::sync::Arc::new(s))),
        Err(_) => err_value(format!("{}: hasilnya ({} byte) bukan teks UTF-8 yang sah; {}", nama, n, jalan)),
    }
}

/// Kebalikan `byte_ke_array`: tiap elemen harus bilangan 0 sampai 255.
pub(crate) fn array_ke_byte(isi: &[Value], fname: &str) -> Result<Vec<u8>, String> {
    isi.iter().enumerate().map(|(i, e)| match e {
        Value::Int(n) if (0..=255).contains(n) => Ok(*n as u8),
        Value::Int(n) => Err(format!("{}: elemen ke-{} array byte harus 0 sampai 255, dapat {}", fname, i, n)),
        lain => Err(format!("{}: elemen ke-{} array byte harus bilangan 0 sampai 255, dapat `{}`",
                            fname, i, lain.type_name())),
    }).collect()
}

pub(crate) fn byte_ke_array(bytes: Vec<u8>) -> Value {
    Value::Array(bytes.into_iter().map(|b| Value::Int(b as i128)).collect::<Vec<_>>().into())
}

/// `crypto::base64_decode_bytes`: byte apa pun, sebagai array bilangan 0 sampai 255.
pub fn f_base64_decode_bytes(args: &[Value]) -> Result<Value, String> {
    let s = as_str(args.get(0).unwrap_or(&Value::Nil), "crypto::base64_decode_bytes")?;
    Ok(match base64_decode_str(s) { Ok(b) => ok_value(byte_ke_array(b)), Err(e) => err_value(e) })
}

/// `crypto::hex_decode_bytes`: padanan `base64_decode_bytes` untuk heksadesimal.
pub fn f_hex_decode_bytes(args: &[Value]) -> Result<Value, String> {
    let s = as_str(args.get(0).unwrap_or(&Value::Nil), "crypto::hex_decode_bytes")?;
    Ok(match hex_decode_str(s) { Ok(b) => ok_value(byte_ke_array(b)), Err(e) => err_value(e) })
}
pub fn f_hex_encode(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(std::sync::Arc::new(hex_encode(&isi_byte(args.get(0).unwrap_or(&Value::Nil), "crypto::hex_encode")?))))
}
pub fn f_hex_decode(args: &[Value]) -> Result<Value, String> {
    let s = as_str(args.get(0).unwrap_or(&Value::Nil), "crypto::hex_decode")?;
    match hex_decode_str(s) {
        Ok(bytes) => Ok(teks_atau_err(bytes, "crypto::hex_decode", "pakai `crypto::hex_decode_bytes` untuk mengambil byte-nya")),
        Err(e) => Ok(err_value(e)),
    }
}

// ============================================================================
//  RANDOM
// ============================================================================

static FALLBACK_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Coba `/dev/urandom` (aman secara kriptografis) dulu; kalau sungguhan tidak
/// tersedia (platform non-Unix), baru jatuh ke PRNG splitmix64 yang di-seed …
/// Untuk kripto (kunci, IV, garam, token): sumber OS atau GAGAL — cadangan berbenih jam bisa ditebak.
pub fn acak_aman(n: usize, siapa: &str) -> Result<Vec<u8>, String> {
    use std::io::Read;
    if let Some(dari_host) = crate::host::acak(n) { return Ok(dari_host); }
    let mut buf = vec![0u8; n];
    match std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut buf)) {
        Ok(()) => Ok(buf),
        Err(e) => Err(format!("{}: sumber acak yang aman (`/dev/urandom`) tidak bisa dibaca: {} — angka dari jam \
            bisa ditebak, jadi tidak dipakai untuk kripto", siapa, crate::host::galat_io(&e))),
    }
}

pub fn secure_random_bytes(n: usize) -> Vec<u8> {
    use std::io::Read;
    if let Some(dari_host) = crate::host::acak(n) { return dari_host; }
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        let mut buf = vec![0u8; n];
        if f.read_exact(&mut buf).is_ok() { return buf; }
    }
    let counter = FALLBACK_COUNTER.fetch_add(1, Ordering::SeqCst);
    let nanos = crate::host::sejak_epoch().map(|d| d.as_nanos() as u64).unwrap_or(0);
    let mut state = nanos ^ counter.wrapping_mul(0x9E3779B97F4A7C15) ^ (std::process::id() as u64);
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        state = state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^= z >> 31;
        out.extend_from_slice(&z.to_le_bytes());
    }
    out.truncate(n);
    out
}

pub fn f_random_hex(args: &[Value]) -> Result<Value, String> {
    let n = match args.get(0) { Some(Value::Int(n)) if *n >= 0 => *n, _ => return Err("crypto::random_hex(n)".to_string()) };
    // Batasnya diperiksa SEBELUM jadi `usize`: `as usize` memotong 2^64+5 jadi 5.
    crate::interpreter::cek_batas_elemen("crypto::random_hex", n)?;
    Ok(Value::Str(std::sync::Arc::new(hex_encode(&acak_aman(n as usize, "crypto::random_hex")?))))
}

pub fn f_random_int(args: &[Value]) -> Result<Value, String> {
    let lo = match args.get(0) { Some(Value::Int(n)) => *n, _ => return Err("crypto::random_int(min, max)".to_string()) };
    let hi = match args.get(1) { Some(Value::Int(n)) => *n, _ => return Err("crypto::random_int(min, max)".to_string()) };
    if hi < lo { return Err("crypto::random_int: max harus >= min".to_string()); }
    // Tanpa bias modulo: `n % m` cuma seragam kalau `n` di bawah kelipatan `m` terbesar, sisanya ditarik ulang
    // (dulu rentang selebar mendekati 2^127 memberi separuh nilainya peluang dua kali lipat).
    let m = (hi.wrapping_sub(lo) as u128).wrapping_add(1);
    let ambang = if m == 0 { 0 } else { m.wrapping_neg() % m };
    loop {
        let mut n: u128 = 0;
        for b in acak_aman(16, "crypto::random_int")? { n = (n << 8) | b as u128; }
        if n >= ambang { return Ok(Value::Int(crate::interpreter::acak_dalam_rentang(lo, hi, n))); }
    }
}

// ============================================================================
//  AES-256 (FIPS-197) — block cipher murni dari nol, mode CBC + padding PKCS7
// ============================================================================




// ─── Wrapper native (Value-level) untuk AES ─────────────────────────────


pub fn f_aes_gen_key(_args: &[Value]) -> Result<Value, String> { Ok(Value::Str(std::sync::Arc::new(hex_encode(&acak_aman(32, "crypto::aes_gen_key")?)))) }
pub fn f_aes_gen_iv(_args: &[Value]) -> Result<Value, String> { Ok(Value::Str(std::sync::Arc::new(hex_encode(&acak_aman(16, "crypto::aes_gen_iv")?)))) }

pub fn f_aes_encrypt(args: &[Value]) -> Result<Value, String> {
    let plaintext = isi_byte(args.get(0).unwrap_or(&Value::Nil), "crypto::aes_encrypt")?;
    let key_hex = as_str(args.get(1).unwrap_or(&Value::Nil), "crypto::aes_encrypt")?;
    let iv_hex = as_str(args.get(2).unwrap_or(&Value::Nil), "crypto::aes_encrypt")?;
    let key = match parse_key32(key_hex, "crypto::aes_encrypt") { Ok(k) => k, Err(e) => return Ok(err_value(e)) };
    let iv = match parse_iv16(iv_hex, "crypto::aes_encrypt") { Ok(v) => v, Err(e) => return Ok(err_value(e)) };
    let ct = aes256_cbc_encrypt(&plaintext, &key, &iv);
    Ok(ok_value(Value::Str(std::sync::Arc::new(base64_encode_bytes(&ct)))))
}

pub fn f_aes_decrypt(args: &[Value]) -> Result<Value, String> {
    Ok(match aes_buka(args, "crypto::aes_decrypt")? {
        Ok(bytes) => teks_atau_err(bytes, "crypto::aes_decrypt",
            "kunci atau IV-nya mungkin bukan yang dipakai menyandikan; isi biner diambil `crypto::aes_decrypt_bytes`"),
        Err(e) => err_value(e),
    })
}

// ─── seal/unseal: AES-256-CBC + HMAC-SHA256 (enkripsi lalu MAC) ──────────
// `aes_encrypt` tanpa MAC: sandi yang diubah tetap dibuka, dan bedanya galat padding dengan
// berhasil adalah oracle (kelas CVE-2016-2107). Kotak: IV 16 || sandi || tanda 32, base64.


/// `crypto::seal(isi, kunci)` → `Result<string>`; IV acak dari OS, di dalam kotaknya.
pub fn f_seal(args: &[Value]) -> Result<Value, String> {
    const N: &str = "crypto::seal";
    let isi = isi_byte(args.get(0).unwrap_or(&Value::Nil), N)?;
    let key_hex = as_str(args.get(1).unwrap_or(&Value::Nil), N)?;
    let (ke, km) = match kunci_kotak(key_hex, N) { Ok(k) => k, Err(e) => return Ok(err_value(e)) };
    let mut iv = [0u8; 16];
    iv.copy_from_slice(&acak_aman(16, N)?);
    let mut kotak = iv.to_vec();
    kotak.extend(aes256_cbc_encrypt(&isi, &ke, &iv));
    let tanda = hmac_sha256(&km, &kotak);
    kotak.extend_from_slice(&tanda);
    Ok(ok_value(Value::Str(std::sync::Arc::new(base64_encode_bytes(&kotak)))))
}

/// Tanda diperiksa (waktu tetap) SEBELUM didekripsi: kotak yang diubah satu bit tidak pernah dibuka.
fn buka_kotak(args: &[Value], nama: &str) -> Result<Result<Vec<u8>, String>, String> {
    let kotak_b64 = as_str(args.get(0).unwrap_or(&Value::Nil), nama)?;
    let key_hex = as_str(args.get(1).unwrap_or(&Value::Nil), nama)?;
    Ok((|| {
        let (ke, km) = kunci_kotak(key_hex, nama)?;
        let kotak = base64_decode_str(kotak_b64).map_err(|e| format!("{}: {}", nama, e))?;
        if kotak.len() < 16 + 16 + 32 {
            return Err(format!("{}: kotaknya terlalu pendek ({} byte) — bukan hasil `crypto::seal`", nama, kotak.len()));
        }
        let (isi, tanda) = kotak.split_at(kotak.len() - 32);
        let harus = hmac_sha256(&km, isi);
        if !sama_waktu_tetap(&harus, tanda) {
            return Err(format!("{}: tanda tangan tidak cocok — kuncinya lain, atau kotaknya diubah", nama));
        }
        let mut iv = [0u8; 16];
        iv.copy_from_slice(&isi[..16]);
        aes256_cbc_decrypt(&isi[16..], &ke, &iv).map_err(|e| format!("{}: {}", nama, e))
    })())
}

pub fn f_unseal(args: &[Value]) -> Result<Value, String> {
    Ok(match buka_kotak(args, "crypto::unseal")? {
        Ok(b) => teks_atau_err(b, "crypto::unseal", "isi biner diambil `crypto::unseal_bytes`"),
        Err(e) => err_value(e),
    })
}

pub fn f_unseal_bytes(args: &[Value]) -> Result<Value, String> {
    Ok(match buka_kotak(args, "crypto::unseal_bytes")? { Ok(b) => ok_value(byte_ke_array(b)), Err(e) => err_value(e) })
}

/// `crypto::aes_decrypt_bytes`: isi yang dibuka sebagai array byte, teks atau bukan.
pub fn f_aes_decrypt_bytes(args: &[Value]) -> Result<Value, String> {
    Ok(match aes_buka(args, "crypto::aes_decrypt_bytes")? { Ok(b) => ok_value(byte_ke_array(b)), Err(e) => err_value(e) })
}

/// Galat argumen di luar, galat data (kunci, IV, sandi) di dalam: yang dalam jadi `Err`.
fn aes_buka(args: &[Value], nama: &str) -> Result<Result<Vec<u8>, String>, String> {
    let ct_b64 = as_str(args.get(0).unwrap_or(&Value::Nil), nama)?;
    let key_hex = as_str(args.get(1).unwrap_or(&Value::Nil), nama)?;
    let iv_hex = as_str(args.get(2).unwrap_or(&Value::Nil), nama)?;
    Ok((|| {
        let key = parse_key32(key_hex, nama)?;
        let iv = parse_iv16(iv_hex, nama)?;
        aes256_cbc_decrypt(&base64_decode_str(ct_b64)?, &key, &iv)
    })())
}


#[cfg(test)]
mod uji_acak {
    /// Rentang 3·2^126: dengan bias modulo sepertiga pertama keluar separuh waktu, tanpa bias sepertiga.
    #[test]
    fn random_int_tanpa_bias_modulo() {
        use crate::interpreter::Value;
        // `i128::MIN ..= 2^126 - 1` lebarnya 3·2^126; sepertiga pertamanya di bawah `-2^126`.
        let (lo, hi, batas) = (i128::MIN, (1i128 << 126) - 1, -(1i128 << 126));
        let (mut bawah, n) = (0, 3000);
        for _ in 0..n {
            match super::f_random_int(&[Value::Int(lo), Value::Int(hi)]).unwrap() {
                Value::Int(x) => { assert!(lo <= x && x <= hi); if x < batas { bawah += 1 } }
                lain => panic!("{:?}", lain),
            }
        }
        let f = bawah as f64 / n as f64;
        assert!((0.28..0.39).contains(&f), "pecahan sepertiga pertama {} (bias modulo memberi 0,5)", f);
        for (lo, hi) in [(5, 5), (i128::MIN, i128::MAX), (-3, 3)] {
            let Value::Int(x) = super::f_random_int(&[Value::Int(lo), Value::Int(hi)]).unwrap() else { panic!() };
            assert!(lo <= x && x <= hi);
        }
    }
}


