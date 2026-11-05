// web/build.rs — stack untuk interpreter tree-walking. Bawaan wasm-ld 1 MiB,
// dan rekursi JRG 500 lapis sudah melewatinya (diukur).
fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.starts_with("wasm32") {
        println!("cargo:rustc-link-arg=-zstack-size={}", 64 * 1024 * 1024);
    }
}
