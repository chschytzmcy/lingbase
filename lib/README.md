# Llama.cpp 库管理

本项目使用预编译的 llama.cpp 库，存放在 `lib/` 目录。

## 目录结构

```
lib/
├── x86_64/              # Linux x86_64 CPU 库
├── cuda/                # Linux x86_64 NVIDIA CUDA GPU 库
├── aarch64/             # Linux ARM64 CPU 库（glibc 2.35 编译，含 ARMv8.2+ 优化变体）
└── aarch64-glibc-227/  # Linux ARM64 CPU 库（glibc 2.27 编译，firefly 本地编译）
```

每个目录包含完整的 llama.cpp 动态库集合（`libllama.so`, `libggml*.so` 等）。

## 库文件说明

- `libllama.so` - 主推理库
- `libggml.so` - 张量计算库
- `libggml-base.so` - GGML 基础库
- `libggml-cpu.so` - CPU 后端实现（符号链接，指向具体变体）
- `libllama-common.so` - LLAMA 公共组件
- `libmtmd.so` - 多线程/调度库

## glibc 版本兼容性

| 目录 | 最低 glibc | 兼容系统 | 编译环境 |
|------|-----------|----------|----------|
| `x86_64/` | 2.27 | Ubuntu 18.04+ | - |
| `cuda/` | 2.27 | Ubuntu 18.04+ | - |
| `aarch64/` | 2.35 | Ubuntu 22.04+ | Ubuntu 22.04 (glibc 2.35) |
| `aarch64-glibc-227/` | 2.27 | Ubuntu 18.04+ | firefly Ubuntu 22.04 (glibc 2.35) 本地编译 |

## aarch64 vs aarch64-glibc-227

### aarch64/ - ARMv8.2+ 优化版本

- 针对特定 ARM 核心架构优化的多版本库
- 最低 glibc 2.35
- 编译环境: Ubuntu 22.04 (glibc 2.35)
- 适用于边缘设备、物联网芯片

包含的优化库：

| 库文件 | 对应 ARM 架构 |
|--------|--------------|
| `libggml-cpu-armv8.0_1.so` | ARMv8.0 |
| `libggml-cpu-armv8.2_1.so` | ARMv8.2 |
| `libggml-cpu-armv8.6_1.so` | ARMv8.6 |
| `libggml-cpu-armv9.2_1.so` | ARMv9.2 |

运行时 llama.cpp 会根据 CPU 特性自动选择最优的库文件。

### aarch64-glibc-227/ - 通用版本

- 只有一个 `libggml-cpu.so`
- 最低 glibc 2.27，兼容 Ubuntu 18.04+
- 在 firefly Ubuntu 22.04 (glibc 2.35) 本地编译
- 适用于大多数 ARM64 服务器

## 部署配置

部署 ARM64 服务器时，在 `config/deploy.toml` 中指定 `lib_dir` 选择库版本：

```toml
[remote.arm-server]
arch = "aarch64"
lib_dir = "aarch64-glibc-227"  # 使用通用版本 (glibc 2.27)
# 或
lib_dir = "aarch64"             # 使用优化版本 (glibc 2.35)
```

## 获取产物

### 1. x86_64 (CPU) - 从 Release 下载

```bash
mkdir -p lib/x86_64
wget https://github.com/ggml-org/llama.cpp/releases/download/b9129/llama-b9129-bin-ubuntu-x64.tar.gz
mkdir -p /tmp/x64_extract && tar -xzf llama-b9129-bin-ubuntu-x64.tar.gz -C /tmp/x64_extract
cp /tmp/x64_extract/llama-b9129/lib*.so* lib/x86_64/
```

### 2. aarch64 (ARM64) - 远程编译

在 ARM 服务器 (192.168.0.124) 上编译后复制：

```bash
# 在 ARM 服务器上编译 llama.cpp
cd ~ && mkdir -p build && cd build
git clone https://github.com/ggml-org/llama.cpp.git
cd llama.cpp && git checkout b9174
cmake -B build -DGGML_CPU=ON -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel

# 复制产物到本项目
scp firefly@192.168.0.124:/home/firefly/llama.cpp/build/bin/lib*.so* lib/aarch64/
```

### 3. cuda (GPU) - 远程编译

在 GPU 服务器 (67.0.0.5) 上执行：

```bash
# 编译
cd ~ && mkdir -p build && cd build
git clone https://github.com/ggml-org/llama.cpp.git
cd llama.cpp && git checkout b9129
cmake -B build -DGGML_CUDA=ON -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel

# 复制产物到本项目
scp etsme@67.0.0.5:/home/etsme/llama.cpp/build/bin/lib*.so* lib/cuda/
```

## 运行时

```bash
# GPU 模式
export LD_LIBRARY_PATH=lib/cuda:$LD_LIBRARY_PATH

# CPU 模式 (x86_64)
export LD_LIBRARY_PATH=lib/x86_64:$LD_LIBRARY_PATH

# CPU 模式 (ARM64 通用版)
export LD_LIBRARY_PATH=lib/aarch64-glibc-227:$LD_LIBRARY_PATH

# CPU 模式 (ARM64 优化版)
export LD_LIBRARY_PATH=lib/aarch64:$LD_LIBRARY_PATH
```