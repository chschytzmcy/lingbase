use std::env;
use std::path::PathBuf;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    let lib_dir = PathBuf::from("lib");

    let (arch_lib_dir, link_libs) = match target_arch.as_str() {
        "x86_64" => {
            let dir = lib_dir.join("x86_64");
            // Core libraries - ggml will dynamically load appropriate CPU variant
            let libs = vec!["llama", "ggml", "ggml-base", "llama-common"];
            (dir, libs)
        }
        "aarch64" => {
            let dir = lib_dir.join("arm64");
            let libs = vec!["llama", "ggml", "ggml-base", "llama-common"];
            (dir, libs)
        }
        _ => {
            eprintln!("Unsupported architecture: {}", target_arch);
            std::process::exit(1);
        }
    };

    if !arch_lib_dir.exists() {
        eprintln!("Warning: Library directory {} does not exist", arch_lib_dir.display());
        eprintln!("Please place prebuilt llama.cpp libraries in lib/<arch>/");
    }

    // Add link search path
    println!("cargo:rustc-link-search=native={}", arch_lib_dir.display());

    // Dynamic linking
    for lib in &link_libs {
        println!("cargo:rustc-link-lib=dylib={}", lib);
    }

    // Rerun if libraries change
    println!("cargo:rerun-if-changed=lib/{}", target_arch);
    println!("cargo:rerun-if-changed=include/llama.h");
}