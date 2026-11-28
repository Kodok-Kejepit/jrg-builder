// runtime/src/canvas/matek.rs — `floor`, `sin` dan kawannya untuk berkas bersama `Canvas::` di runtime `no_std`.
// Lewat libm yang sama dengan yang dipanggil std di `jrgc`, jadi pikselnya sama sampai bit terakhir; di `jrgc`
// berkas ini kosong karena method-nya bawaan std.
extern "C" {
    fn floorf(x: f32) -> f32;
    fn ceilf(x: f32) -> f32;
    fn roundf(x: f32) -> f32;
    fn truncf(x: f32) -> f32;
    fn sqrtf(x: f32) -> f32;
    fn sinf(x: f32) -> f32;
    fn cosf(x: f32) -> f32;
    fn floor(x: f64) -> f64;
    fn ceil(x: f64) -> f64;
    fn round(x: f64) -> f64;
    fn trunc(x: f64) -> f64;
    fn sqrt(x: f64) -> f64;
    fn sin(x: f64) -> f64;
    fn cos(x: f64) -> f64;
}

pub trait Matek: Copy {
    fn floor(self) -> Self;
    fn ceil(self) -> Self;
    fn round(self) -> Self;
    fn trunc(self) -> Self;
    fn fract(self) -> Self;
    fn sqrt(self) -> Self;
    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn powi(self, n: i32) -> Self;
}

impl Matek for f32 {
    fn floor(self) -> f32 { unsafe { floorf(self) } }
    fn ceil(self) -> f32 { unsafe { ceilf(self) } }
    fn round(self) -> f32 { unsafe { roundf(self) } }
    fn trunc(self) -> f32 { unsafe { truncf(self) } }
    fn fract(self) -> f32 { self - unsafe { truncf(self) } }
    fn sqrt(self) -> f32 { unsafe { sqrtf(self) } }
    fn sin(self) -> f32 { unsafe { sinf(self) } }
    fn cos(self) -> f32 { unsafe { cosf(self) } }
    // Algoritme `__powisf2` compiler-rt, yang dipanggil std untuk `f32::powi`.
    fn powi(self, n: i32) -> f32 {
        let (mut a, mut b, mut r) = (self, n, 1.0f32);
        loop {
            if b & 1 != 0 { r *= a; }
            b /= 2;
            if b == 0 { break; }
            a *= a;
        }
        if n < 0 { 1.0 / r } else { r }
    }
}

impl Matek for f64 {
    fn floor(self) -> f64 { unsafe { floor(self) } }
    fn ceil(self) -> f64 { unsafe { ceil(self) } }
    fn round(self) -> f64 { unsafe { round(self) } }
    fn trunc(self) -> f64 { unsafe { trunc(self) } }
    fn fract(self) -> f64 { self - unsafe { trunc(self) } }
    fn sqrt(self) -> f64 { unsafe { sqrt(self) } }
    fn sin(self) -> f64 { unsafe { sin(self) } }
    fn cos(self) -> f64 { unsafe { cos(self) } }
    fn powi(self, n: i32) -> f64 {
        let (mut a, mut b, mut r) = (self, n, 1.0f64);
        loop {
            if b & 1 != 0 { r *= a; }
            b /= 2;
            if b == 0 { break; }
            a *= a;
        }
        if n < 0 { 1.0 / r } else { r }
    }
}
