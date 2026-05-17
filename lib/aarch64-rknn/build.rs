//! aarch64-rknn build script
//!
//! 生成 FFI bindings 并链接 librknn3_api.so

use std::env;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").expect("TARGET not set");
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));

    // 只支持 aarch64
    if !target.contains("aarch64") {
        println!("cargo:warning=aarch64-rknn only supports aarch64, current: {}", target);
        return;
    }

    // lib/rknn3/{lib,include}
    let rknn3_lib = manifest_dir.join("lib/rknn3/lib");
    let rknn3_include = manifest_dir.join("lib/rknn3/include");
    let header_path = rknn3_include.join("rknn3_api.h");

    if !header_path.exists() {
        println!("cargo:warning=Header not found: {}, generating empty bindings", header_path.display());
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let _ = std::fs::write(out_dir.join("bindings.rs"), "");
        return;
    }

    // 设置链接搜索路径
    println!("cargo:rustc-link-search=native={}", rknn3_lib.display());

    // 设置 RPATH
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", rknn3_lib.display());

    // 链接库
    println!("cargo:rustc-link-lib=dylib=rknn3_api");

    // 复制 .so 文件到 OUT_DIR
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let so_file = rknn3_lib.join("librknn3_api.so");
    if so_file.exists() {
        let _ = std::fs::copy(&so_file, out_dir.join("librknn3_api.so"));
        println!("cargo:rerun-if-changed={}", so_file.display());
    }

    // 生成 bindgen bindings
    let mut builder = bindgen::Builder::default()
        .header(header_path.to_str().unwrap())
        .clang_arg(format!("-I{}", rknn3_include.display()));

    // allowlist 需要的函数和类型
    builder = builder.allowlist_function("rknn3_.*");
    builder = builder.allowlist_function("RKNN3_.*");
    builder = builder.allowlist_type("rknn3_.*");
    builder = builder.allowlist_type("RKNN3_.*");
    builder = builder.allowlist_var("RKNN3_.*");

    let bindings = builder
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rerun-if-changed={}", header_path.display());
}