# RKNN3 库管理

本目录用于存放 RKNN3 Runtime 预编译库，结构参考 llama.cpp 库管理方式。

## 目录结构

```
lib/
├── rknn3-aarch64/        # Linux ARM64 RKNN3 库
│   ├── librknn3_api.so   # RKNN3 API 库
│   └── README.md         # 本文件
└── ...
```

## 库文件说明

| 文件 | 说明 | 来源 |
|------|------|------|
| `librknn3_api.so` | RKNN3 Runtime 主库 | Rockchip RKNN3 SDK |

## 获取 RKNN3 SDK

RKNN3 库来自 Rockchip 官方 SDK，需要从 RK3588 设备或 SDK 包获取：

```bash
# 方法1：从设备复制（推荐）
scp root@192.168.x.x:/usr/lib/librknn3_api.so lib/rknn3-aarch64/

# 方法2：从 Rockchip SDK 获取
# 下载 RKNN3 SDK 后从以下路径复制：
# external/rknn3_api/lib/librknn3_api.so
```

## 与 llama.cpp 库管理的区别

| 维度 | llama.cpp | rknn3 |
|------|-----------|-------|
| 库目录 | `lib/x86_64-cpu/`, `lib/aarch64-rk-cpu/` 等 | `lib/rknn3-aarch64/` |
| 链接方式 | build.rs 动态指定 `cargo:rustc-link-lib` | rknn3-sys build.rs 使用 PrebuiltLibBuilder |
| 依赖数量 | 多个 .so (llama, ggml, ggml-base 等) | 仅 librknn3_api.so |

## 构建说明

启用 rknn3 feature 时：

```toml
# Cargo.toml
[features]
default = []
rknn3 = ["rknn3-sys"]
```

rknn3-sys 的 build.rs 会通过 PrebuiltLibBuilder 查找 `lib/rknn3-aarch64/librknn3_api.so`。

## 运行时

```bash
export LD_LIBRARY_PATH=lib/rknn3-aarch64:$LD_LIBRARY_PATH
```