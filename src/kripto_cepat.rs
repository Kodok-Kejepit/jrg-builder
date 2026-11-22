// src/kripto_cepat.rs — jalur perangkat keras untuk `kripto_inti`: SHA-NI dan AES-NI, dipilih saat jalan.
// Runtime LLVM punya padanan yang selalu menjawab "tidak ada", jadi `kripto_inti` sama persis di dua tempat.

/// Kompres satu blok lewat SHA-NI; `false` kalau prosesornya tidak punya (pemanggil memakai jalur skalar).
pub fn sha256(h: &mut [u32; 8], blok: &[u8]) -> bool {
    #[cfg(target_arch = "x86_64")]
    if shani::ada() { shani::kompres(h, blok); return true; }
    let _ = (h, blok);
    false
}
pub fn aes_ada() -> bool { ni::ada() }
pub fn aes_sandi(b: &[u8; 16], rk: &[[u8; 16]; 15]) -> [u8; 16] { ni::sandi(b, rk) }
pub fn aes_buka(b: &[u8; 16], rk: &[[u8; 16]; 15]) -> [u8; 16] { ni::buka(b, rk) }

#[cfg(target_arch = "x86_64")]
mod shani {
    use std::arch::x86_64::*;
    pub fn ada() -> bool {
        std::arch::is_x86_feature_detected!("sha") && std::arch::is_x86_feature_detected!("sse4.1")
            && std::arch::is_x86_feature_detected!("ssse3")
    }
    pub fn kompres(h: &mut [u32; 8], blok: &[u8]) { assert!(blok.len() >= 64); unsafe { kompres_ni(h, blok) } }
    #[target_feature(enable = "sha,sse2,ssse3,sse4.1")]
    unsafe fn kompres_ni(h: &mut [u32; 8], blok: &[u8]) {
        let k = &crate::kripto_inti::SHA256_K;
        let mask = _mm_set_epi64x(0x0C0D_0E0F_0809_0A0Bu64 as i64, 0x0405_0607_0001_0203u64 as i64);
        let p = h.as_ptr() as *const __m128i;
        let (dcba, efgh) = unsafe { (_mm_loadu_si128(p), _mm_loadu_si128(p.add(1))) };
        let cdab = _mm_shuffle_epi32(dcba, 0xB1);
        let efgh = _mm_shuffle_epi32(efgh, 0x1B);
        let mut abef = _mm_alignr_epi8(cdab, efgh, 8);
        let mut cdgh = _mm_blend_epi16(efgh, cdab, 0xF0);
        let (abef0, cdgh0) = (abef, cdgh);
        let d = blok.as_ptr() as *const __m128i;
        let mut w = [_mm_setzero_si128(); 5];
        for (i, x) in w.iter_mut().take(4).enumerate() { *x = _mm_shuffle_epi8(unsafe { _mm_loadu_si128(d.add(i)) }, mask); }
        for i in 0..16 {
            if i >= 4 {
                let j = i - 4;
                let t1 = _mm_sha256msg1_epu32(w[j % 5], w[(j + 1) % 5]);
                let t2 = _mm_alignr_epi8(w[(j + 3) % 5], w[(j + 2) % 5], 4);
                w[(j + 4) % 5] = _mm_sha256msg2_epu32(_mm_add_epi32(t1, t2), w[(j + 3) % 5]);
            }
            let kv = _mm_set_epi32(k[4*i+3] as i32, k[4*i+2] as i32, k[4*i+1] as i32, k[4*i] as i32);
            let t1 = _mm_add_epi32(w[i % 5], kv);
            cdgh = _mm_sha256rnds2_epu32(cdgh, abef, t1);
            abef = _mm_sha256rnds2_epu32(abef, cdgh, _mm_shuffle_epi32(t1, 0x0E));
        }
        abef = _mm_add_epi32(abef, abef0);
        cdgh = _mm_add_epi32(cdgh, cdgh0);
        let feba = _mm_shuffle_epi32(abef, 0x1B);
        let dchg = _mm_shuffle_epi32(cdgh, 0xB1);
        let q = h.as_mut_ptr() as *mut __m128i;
        unsafe {
            _mm_storeu_si128(q, _mm_blend_epi16(feba, dchg, 0xF0));
            _mm_storeu_si128(q.add(1), _mm_alignr_epi8(dchg, feba, 8));
        }
    }
}

#[cfg(target_arch = "x86_64")]
mod ni {
    use std::arch::x86_64::*;
    pub fn ada() -> bool { std::arch::is_x86_feature_detected!("aes") && std::arch::is_x86_feature_detected!("sse2") }
    pub fn sandi(b: &[u8; 16], rk: &[[u8; 16]; 15]) -> [u8; 16] { unsafe { sandi_ni(b, rk) } }
    pub fn buka(b: &[u8; 16], rk: &[[u8; 16]; 15]) -> [u8; 16] { unsafe { buka_ni(b, rk) } }
    #[target_feature(enable = "aes,sse2")]
    unsafe fn sandi_ni(b: &[u8; 16], rk: &[[u8; 16]; 15]) -> [u8; 16] {
        let k = |i: usize| unsafe { _mm_loadu_si128(rk[i].as_ptr() as *const __m128i) };
        let mut s = unsafe { _mm_xor_si128(_mm_loadu_si128(b.as_ptr() as *const __m128i), k(0)) };
        for i in 1..14 { s = _mm_aesenc_si128(s, k(i)); }
        s = _mm_aesenclast_si128(s, k(14));
        let mut o = [0u8; 16];
        unsafe { _mm_storeu_si128(o.as_mut_ptr() as *mut __m128i, s) };
        o
    }
    #[target_feature(enable = "aes,sse2")]
    unsafe fn buka_ni(b: &[u8; 16], rk: &[[u8; 16]; 15]) -> [u8; 16] {
        let k = |i: usize| unsafe { _mm_loadu_si128(rk[i].as_ptr() as *const __m128i) };
        let mut s = unsafe { _mm_xor_si128(_mm_loadu_si128(b.as_ptr() as *const __m128i), k(14)) };
        for i in (1..14).rev() { s = _mm_aesdec_si128(s, _mm_aesimc_si128(k(i))); }
        s = _mm_aesdeclast_si128(s, k(0));
        let mut o = [0u8; 16];
        unsafe { _mm_storeu_si128(o.as_mut_ptr() as *mut __m128i, s) };
        o
    }
}

#[cfg(not(target_arch = "x86_64"))]
mod ni {
    pub fn ada() -> bool { false }
    pub fn sandi(_: &[u8; 16], _: &[[u8; 16]; 15]) -> [u8; 16] { unreachable!("tanpa AES-NI") }
    pub fn buka(_: &[u8; 16], _: &[[u8; 16]; 15]) -> [u8; 16] { unreachable!("tanpa AES-NI") }
}
