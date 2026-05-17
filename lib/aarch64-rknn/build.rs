//! aarch64-rknn build script
//!
//! Link librknn3_api.so from rknn3/lib

fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    // only support aarch64
    if target_arch != "aarch64" {
        println!("cargo:warning=aarch64-rknn only supports aarch64, current: {}", target_arch);
        return;
    }

    // lib dir: lib/aarch64-rknn/{rknn,rknn3,rkllm}/lib
    let lib_dir = std::path::PathBuf::from("lib/rknn3/lib");

    if !lib_dir.exists() {
        println!("cargo:warning=RKNN lib dir not found: {}", lib_dir.display());
        println!("cargo:warning=put librknn3_api.so in lib/aarch64-rknn/rknn3/lib/");
    }

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=rknn3_api");
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../../../lib/rknn3/lib");
    println!("cargo:rerun-if-changed=lib/aarch64-rknn/");
}