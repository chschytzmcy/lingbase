use std::env;
use std::path::PathBuf;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    // 库目录
    let lib_dir = PathBuf::from("lib");

    let (arch_lib_dir, link_libs) = match target_arch.as_str() {
        "x86_64" => {
            let dir = lib_dir.join("x86_64");
            let libs = vec!["llama", "ggml", "ggml-cpu-x64", "ggml-base", "llama-common", "mtmd"];
            (dir, libs)
        }
        "aarch64" => {
            let dir = lib_dir.join("arm64");
            let libs = vec!["llama", "ggml", "ggml-cpu", "ggml-base", "llama-common", "mtmd"];
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

    // 添加链接搜索路径
    println!("cargo:rustc-link-search=native={}", arch_lib_dir.display());

    // 动态链接库
    for lib in &link_libs {
        println!("cargo:rustc-link-lib=dylib={}", lib);
    }

    // 头文件路径
    let include_dir = PathBuf::from("include");
    println!("cargo:rustc-link-search=native={}", include_dir.display());

    // 追踪依赖
    println!("cargo:rerun-if-changed=lib/{}", target_arch);
    println!("cargo:rerun-if-changed=include/llama.h");
}