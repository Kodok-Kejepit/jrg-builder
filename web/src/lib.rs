// web/src/lib.rs — pintu JavaScript ke interpreter JRG yang sama dengan `jrgc`.
// Keluaran program mengalir lewat impor `jrg.tulis` (lihat `src/host.rs`);
// yang dikembalikan di sini cuma kode keluarnya.

use jrgc::diag;
use jrgc::host;
use jrgc::lexer::SourceFile;
use jrgc::pipa;

/// Tempat `n` byte untuk sumber program; JS menulisnya lalu memanggil `jrg_jalankan`.
#[no_mangle]
pub extern "C" fn jrg_alokasi(n: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; n].into_boxed_slice()) as *mut u8
}

/// Jalankan sumber di `p` (`n` byte UTF-8) sebagai `praktik.jrg`, lewat
/// pipeline yang sama dengan `jrgc berkas.jrg`. 0 berhasil, 1 gagal.
#[no_mangle]
pub extern "C" fn jrg_jalankan(p: *mut u8, n: usize) -> i32 {
    // Panik di wasm tidak bisa ditangkap; kalimatnya dikirim sebelum trap.
    std::panic::set_hook(Box::new(|info| {
        let pesan = info.payload().downcast_ref::<&str>().map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        host::galat_baris(&format!(
            "interpreter berhenti tak wajar di peramban: {}. Biasanya karena programnya \
             memakai sesuatu yang tidak ada di peramban (thread, berkas, jaringan); kalau \
             bukan, ini bug di jrgc", pesan));
    }));
    let isi = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(p, n)) };
    let Ok(src) = String::from_utf8(isi.into_vec()) else {
        host::galat_baris("sumber program bukan UTF-8 yang sah");
        return 1;
    };
    let sf = SourceFile { id: 0, name: "praktik.jrg".to_string(), src };
    let mut peta = diag::SourceMap::with_file(sf.clone());
    let Some(tokens) = pipa::lex_dilaporkan(&sf, &peta) else { return 1 };
    let (ast, galat) = jrgc::parser::parse(tokens, false);
    if !galat.is_empty() {
        pipa::laporkan_galat_parse(&galat, false, &peta);
        return 1;
    }
    match jrgc::interpreter::interpret(&ast) {
        Ok(()) => 0,
        Err(e) => { diag::laporkan_runtime(&mut peta, &e); 1 }
    }
}
