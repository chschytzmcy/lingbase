//! rknn3-sys build script
//!
//! 查找 librknn3_api.so 预编译库

fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    // 只支持 aarch64
    if target_arch != "aarch64" {
        println!("cargo:warning=rknn3-sys 仅支持 aarch64 架构，当前: {}", target_arch);
        return;
    }

    let lib_dir = std::path::PathBuf::from("lib/rknn3-aarch64");

    // 检查库是否存在
    if !lib_dir.exists() {
        println!("cargo:warning=RKNN3 库目录不存在: {}", lib_dir.display());
        println!("cargo:warning=请将 librknn3_api.so 放入 lib/rknn3-aarch64/ 目录");
    }

    // 添加链接搜索路径
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // 动态链接库
    println!("cargo:rustc-link-lib=dylib=rknn3_api");

    // 设置 RPATH 以便运行时能找到库
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib/rknn3-aarch64");

    // 依赖头文件变化时重新构建
    println!("cargo:rerun-if-changed=lib/rknn3-aarch64/");
}