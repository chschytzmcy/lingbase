# RKNN 预构建库

本目录包含 RKNN 相关的预构建动态链接库，用于 `etsllm` 的 `rkemb`、`rkocr` 等 feature 的交叉编译。

## 交叉编译环境

- **编译器**: gcc-linaro-6.3.1-2017.05-x86_64_aarch64-linux-gnu
- **目标架构**: ARM aarch64 (GNU/Linux)
- **源码**: [rklibs](https://gitlab.etsme.com/ai/rklibs) `2297fa3`

## 库文件说明

### librknnrt.so

RKNN 运行时库，由 Rockchip 提供，来自 `rknn_model_zoo/3rdparty/rknpu2/Linux/aarch64/`。

**动态依赖**: 仅系统库（libc、libpthread、libdl、libm、libstdc++）

### librknncstop.so

RKNN 自定义算子库（cumsum 等），用于 `etsllm` 的 `rkemb`（嵌入模型）等 feature。

**动态依赖**: librknnrt.so、libstdc++.so.6、libm.so.6、libgcc_s.so.1、libc.so.6

### librknnppocr.so

PPOCR（OCR）推理库，用于 `etsllm` 的 `rkocr` feature。

**动态依赖**: librknnrt.so、libdl.so.2、libpthread.so.0、librt.so.1、libstdc++.so.6、libm.so.6、libgcc_s.so.1、libc.so.6

## 重新构建

使用 aarch64 交叉编译器（gcc-linaro-6.3.1-2017.05-x86_64_aarch64-linux-gnu）重新构建：

```bash
cd rklibs/rknn_cstop
GCC_COMPILER=/path/to/gcc-linaro-6.3.1-2017.05-x86_64_aarch64-linux-gnu/bin/aarch64-linux-gnu ./build.sh

cd rklibs/rknn_ppocr
GCC_COMPILER=/path/to/gcc-linaro-6.3.1-2017.05-x86_64_aarch64-linux-gnu/bin/aarch64-linux-gnu ./build.sh
```

构建完成后，将产物复制到本目录：

```bash
cp rklibs/rknn_model_zoo/3rdparty/rknpu2/Linux/aarch64/librknnrt.so lib/linux-aarch64/rknn/lib/
cp rklibs/rknn_cstop/build/librknncstop.so lib/linux-aarch64/rknn/lib/
cp rklibs/rknn_ppocr/build/librknnppocr.so lib/linux-aarch64/rknn/lib/
```
