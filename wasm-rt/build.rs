// Sumber runtime yang disertakan memegang alokator dan penanganan panik sendiri; di sini std yang memegangnya.
fn main() {
    println!("cargo:rustc-cfg=jrg_gabung");
}
