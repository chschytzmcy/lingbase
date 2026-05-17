# Llama.cpp 库管理

本项目使用预编译的 llama.cpp 库，存放在 `lib/` 目录。

## 目录结构

```
lib/
├── x86_64-cpu/       # Linux x86_64 CPU 库 (x86_64 软链接指向此目录)
├── x86_64-cuda/      # Linux x86_64 NVIDIA CUDA GPU 库
├── aarch64/          # Linux ARM64 CPU 库（软链接，指向 aarch64-rk-cpu）
├── aarch64-raw/      # Linux ARM64 CPU 库（含 ARMv8.x 多架构优化变体）
└── aarch64-rk-cpu/   # Linux ARM64 CPU 库（RK3588 设备本地编译版本）
```

## 软链接机制

`aarch64/` 是软链接，部署时通过 `deploy.toml` 的 `lib_dir` 指向实际目录：

```bash
# 当前配置（RK3588 兼容）
ln -sfn aarch64-rk-cpu lib/aarch64

# 切换到原始优化版（需要 glibc 2.38+）
ln -sfn aarch64-raw lib/aarch64
```

## glibc 版本兼容性

| 目录 | glibc 要求 | 适用设备 |
|------|-----------|----------|
| `x86_64-cpu/` | ≥ 2.27 | x86_64 服务器 |
| `x86_64-cuda/` | ≥ 2.27 | NVIDIA GPU 服务器 |
| `aarch64-raw/` | ≥ 2.38 | 高版本 glibc 的 ARM64 设备 |
| `aarch64-rk-cpu/` | ≥ 2.35 | RK3588 (Ubuntu 22.04) |

**注意**：RK3588 设备 (arm-server) 系统为 Ubuntu 22.04，glibc 2.35，只能使用 `aarch64-rk-cpu/` 目录的库。

## 库文件说明

| 文件 | 说明 |
|------|------|
| `libllama.so` | 主推理库 |
| `libggml.so` | 张量计算库 |
| `libggml-base.so` | GGML 基础库 |
| `libggml-cpu.so` | CPU 后端（符号链接，指向具体变体） |
| `libllama-common.so` | LLAMA 公共组件 |
| `libmtmd.so` | 多线程/调度库 |

## aarch64-raw vs aarch64-rk-cpu

### aarch64-raw/ - 多架构优化版

包含多个 ARM 核心架构优化库，运行时自动选择最优：

| 库文件 | 对应 ARM 架构 |
|--------|--------------|
| `libggml-cpu-armv8.0_1.so` | ARMv8.0 |
| `libggml-cpu-armv8.2_1.so` | ARMv8.2 |
| `libggml-cpu-armv8.6_1.so` | ARMv8.6 |
| `libggml-cpu-armv9.2_1.so` | ARMv9.2 |

**要求**：glibc ≥ 2.38

### aarch64-rk-cpu/ - RK3588 兼容版

从 RK3588 设备本地编译，只有一个 `libggml-cpu.so`。

**要求**：glibc ≥ 2.35

## 部署配置

在 `config/deploy.toml` 中指定 `lib_dir`：

```toml
[remote.arm-server]
arch = "aarch64"
lib_dir = "aarch64"        # 通过软链接使用（当前指向 aarch64-rk-cpu）
```

## 获取产物

### 1. x86_64-cpu - 从 Release 下载

```bash
mkdir -p lib/x86_64-cpu
wget https://github.com/ggml-org/llama.cpp/releases/download/b9129/llama-b9129-bin-ubuntu-x64.tar.gz
mkdir -p /tmp/x64_extract && tar -xzf llama-b9129-bin-ubuntu-x64.tar.gz -C /tmp/x64_extract
cp /tmp/x64_extract/llama-b9129/lib*.so* lib/x86_64-cpu/
```

### 2. aarch64 (ARM64) - 远程编译

```bash
# 在 ARM 服务器上编译 llama.cpp
cd ~ && mkdir -p build && cd build
git clone https://github.com/ggml-org/llama.cpp.git
cd llama.cpp && git checkout b9174
cmake -B build -DGGML_CPU=ON -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel

# 复制到本项目
scp firefly@192.168.0.124:/home/firefly/llama.cpp/build/bin/lib*.so* lib/aarch64-raw/
```

### 3. x86_64-cuda (GPU) - 远程编译

```bash
# 在 GPU 服务器上编译
cd ~ && mkdir -p build && cd build
git clone https://github.com/ggml-org/llama.cpp.git
cd llama.cpp && git checkout b9129
cmake -B build -DGGML_CUDA=ON -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel

# 复制到本项目
scp etsme@67.0.0.5:/home/etsme/llama.cpp/build/bin/lib*.so* lib/x86_64-cuda/
```

## 运行时

```bash
# GPU 模式
export LD_LIBRARY_PATH=lib/x86_64-cuda:$LD_LIBRARY_PATH

# CPU 模式 (x86_64)
export LD_LIBRARY_PATH=lib/x86_64-cpu:$LD_LIBRARY_PATH

# CPU 模式 (ARM64)
export LD_LIBRARY_PATH=lib/aarch64:$LD_LIBRARY_PATH
```