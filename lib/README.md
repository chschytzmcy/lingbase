# Llama.cpp 库管理

本项目使用预编译的 llama.cpp 库，存放在 `lib/` 目录。

## 目录结构

```
lib/
├── x86_64/           # Linux x86_64 CPU 库
├── cuda/             # Linux x86_64 NVIDIA CUDA GPU 库
└── arm64/            # Linux ARM64 CPU 库
```

每个目录包含完整的 llama.cpp 动态库集合（`libllama.so`, `libggml*.so` 等）。

## 构建要求

**每次发布/构建必须产出全部 3 个架构的产物：**

| 架构 | 平台 | 产物目录 | 构建方式 |
|------|------|----------|----------|
| x86_64 | Linux Intel/AMD CPU | `lib/x86_64/` | 官方 Release 或源码编译 |
| cuda | Linux NVIDIA GPU | `lib/cuda/` | 源码编译 (需 GPU 服务器) |
| arm64 | Linux ARM CPU | `lib/arm64/` | 官方 Release 或源码编译 |

## 获取产物

### 1. x86_64 (CPU) - 从 Release 下载

```bash
mkdir -p lib/x86_64
wget https://github.com/ggml-org/llama.cpp/releases/download/b9129/llama-b9129-bin-ubuntu-x64.tar.gz
mkdir -p /tmp/x64_extract && tar -xzf llama-b9129-bin-ubuntu-x64.tar.gz -C /tmp/x64_extract
cp /tmp/x64_extract/llama-b9129/lib*.so* lib/x86_64/
```

### 2. arm64 (CPU) - 从 Release 下载

```bash
mkdir -p lib/arm64
wget https://github.com/ggml-org/llama.cpp/releases/download/b9129/llama-b9129-bin-ubuntu-arm64.tar.gz
mkdir -p /tmp/arm64_extract && tar -xzf llama-b9129-bin-ubuntu-arm64.tar.gz -C /tmp/arm64_extract
cp /tmp/arm64_extract/llama-b9129/lib*.so* lib/arm64/
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
scp -r /home/etsme/llama.cpp/build/bin/*.so* user@local:/path/to/lingbase/lib/cuda/
```

或从远程拉取：

```bash
mkdir -p lib/cuda
sshpass -p 'l7OiD3QuWNI#sC' scp etsme@67.0.0.5:/home/etsme/llama.cpp/build/bin/*.so* lib/cuda/
```

## 全量构建脚本

每次版本更新或首次构建时，执行以下脚本产出全部 3 个架构：

```bash
#!/bin/bash
set -e

TAG="${1:-b9129}"

echo "=== Building llama.cpp $TAG for all platforms ==="

# 1. x86_64
echo ">>> Building x86_64..."
mkdir -p lib/x86_64
[ ! -f "llama-b9129-bin-ubuntu-x64.tar.gz" ] && \
    wget -q https://github.com/ggml-org/llama.cpp/releases/download/${TAG}/llama-b9129-bin-ubuntu-x64.tar.gz
mkdir -p /tmp/x64 && tar -xzf llama-b9129-bin-ubuntu-x64.tar.gz -C /tmp/x64
cp /tmp/x64/llama-b9129/lib*.so* lib/x86_64/

# 2. arm64
echo ">>> Building arm64..."
mkdir -p lib/arm64
[ ! -f "llama-b9129-bin-ubuntu-arm64.tar.gz" ] && \
    wget -q https://github.com/ggml-org/llama.cpp/releases/download/${TAG}/llama-b9129-bin-ubuntu-arm64.tar.gz
mkdir -p /tmp/arm64 && tar -xzf llama-b9129-bin-ubuntu-arm64.tar.gz -C /tmp/arm64
cp /tmp/arm64/llama-b9129/lib*.so* lib/arm64/

# 3. cuda (从远程服务器)
echo ">>> Fetching cuda from remote server..."
mkdir -p lib/cuda
sshpass -p 'l7OiD3QuWNI#sC' scp etsme@67.0.0.5:/home/etsme/llama.cpp/build/bin/*.so* lib/cuda/

echo "=== Done ==="
ls -la lib/x86_64/ lib/arm64/ lib/cuda/
```

## 头文件

```bash
curl -sL https://raw.githubusercontent.com/ggml-org/llama.cpp/b9129/include/llama.h -o include/llama.h
```

## 验证

```bash
# 检查所有架构库是否完整
ls lib/x86_64/libllama.so && \
ls lib/arm64/libllama.so && \
ls lib/cuda/libllama.so && \
ls lib/cuda/libggml-cuda.so

# 检查 CUDA 库依赖
ldd lib/cuda/libllama.so | grep cuda
```

## 运行时

```bash
# GPU 模式
export LD_LIBRARY_PATH=/path/to/lingbase/lib/cuda:$LD_LIBRARY_PATH

# CPU 模式 (x86_64)
export LD_LIBRARY_PATH=/path/to/lingbase/lib/x86_64:$LD_LIBRARY_PATH

# CPU 模式 (ARM64)
export LD_LIBRARY_PATH=/path/to/lingbase/lib/arm64:$LD_LIBRARY_PATH
```