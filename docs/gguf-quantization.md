# GGUF 量化模型差异分析

本文档记录 llama.cpp GGUF 格式的量化模型差异，帮助选择合适的量化级别。

## 量化命名规则

### 基本格式
- **Q{n}**: 量化位数 (4/5/6/8)，数字越大精度越高、模型越大
- **_K**: K-quants 系列量化算法
- **_0**: legacy 旧版量化格式

### K-quants 系列 (推荐)
llama.cpp 新一代量化算法，比 legacy 更高效：

| 类型 | 说明 |
|------|------|
| `_K_M` | Medium - 平衡质量与大小 |
| `_K_S` | Small - 更小体积，略损质量 |
| `_K_L` | Large - 更高质量，略大体积 |

### Legacy 系列
旧版量化格式，兼容性好但效率略低：

| 类型 | 说明 |
|------|------|
| `_0` | 基础量化，无特殊优化 |

## Qwen3-4B GGUF 模型对比

| 文件名 | 大小 | 量化类型 | 精度等级 | 推理速度 | 内存占用 | 适用场景 |
|--------|------|----------|----------|----------|----------|----------|
| Qwen3-4B-Q4_K_M.gguf | 2.4G | 4-bit K-quants Medium | 较低 | 最快 | ~2.5G | 边缘设备、资源受限环境 |
| Qwen3-4B-Q5_0.gguf | 2.7G | 5-bit legacy | 中低 | 快 | ~3G | 平衡速度与质量 |
| Qwen3-4B-Q5_K_M.gguf | 2.7G | 5-bit K-quants Medium | 中 | 较快 | ~3G | **推荐：最佳平衡点** |
| Qwen3-4B-Q6_K.gguf | 3.1G | 6-bit K-quants | 中高 | 中等 | ~3.5G | 高质量推理 |
| Qwen3-4B-Q8_0.gguf | 4.0G | 8-bit legacy | 高 | 较慢 | ~4.5G | 接近原始 FP16 精度 |

## 精度损失对比

相对于原始 FP16 模型的精度损失：

| 量化级别 | 平均精度损失 | PPL (Perplexity) 增加 |
|----------|--------------|----------------------|
| Q4_K_M | ~1-2% | +0.1-0.2 |
| Q5_K_M | ~0.5-1% | +0.05-0.1 |
| Q6_K | ~0.2-0.5% | +0.02-0.05 |
| Q8_0 | ~0.1% | +0.01-0.02 |

## 内存计算公式

量化模型的内存占用估算：

```
内存占用 ≈ 模型文件大小 + KV Cache + Context

KV Cache ≈ n_ctx × n_layers × 2 × (hidden_size / 8)
```

示例 (Qwen3-4B, n_ctx=4096):
- Q4_K_M: ~2.5G + ~0.5G KV Cache = ~3G
- Q8_0: ~4.5G + ~0.5G KV Cache = ~5G

## 选择建议

### 按使用场景

| 场景 | 推荐量化 | 原因 |
|------|----------|------|
| 手机/边缘设备 | Q4_K_M | 内存受限，速度优先 |
| 个人电脑 (8G RAM) | Q5_K_M | 平衡内存与质量 |
| 个人电脑 (16G+ RAM) | Q6_K 或 Q8_0 | 追求高质量输出 |
| 服务器部署 | Q8_0 或 FP16 | 资源充足，精度优先 |

### 按任务类型

| 任务类型 | 推荐量化 | 原因 |
|----------|----------|------|
| 简单问答 | Q4_K_M | 任务简单，低精度足够 |
| 代码生成 | Q5_K_M+ | 需要较高精度避免语法错误 |
| 数学推理 | Q6_K+ | 精度敏感，避免计算错误 |
| 文本创作 | Q5_K_M | 平衡创意质量与速度 |
| 翻译任务 | Q6_K+ | 语言理解需要较高精度 |

## 性能测试建议

切换量化模型后建议测试：

1. **首 Token 延迟**: 预填充速度
2. **Token/s**: 生成速度
3. **输出质量**: 手动评估关键任务
4. **内存峰值**: 实际运行时内存占用

## 配置切换

修改 `config/environment.toml`:

```toml
[model]
# 高精度配置
test_model_path = "/path/to/Qwen3-4B-Q8_0.gguf"

# 平衡配置 (推荐)
test_model_path = "/path/to/Qwen3-4B-Q5_K_M.gguf"

# 低资源配置
test_model_path = "/path/to/Qwen3-4B-Q4_K_M.gguf"
```

## 参考

- [llama.cpp 量化说明](https://github.com/ggerganov/llama.cpp/blob/master/examples/quantize/README.md)
- [GGUF 格式规范](https://github.com/ggerganov/ggml/blob/master/docs/gguf.md)