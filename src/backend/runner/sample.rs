//! 采样函数模块
//!
//! 提供 top-k, top-p, temperature 等采样算法的公共实现。

/// 根据配置采样一个 token
pub fn sample_token(logits: &[f32], temperature: f32, top_p: f32, top_k: usize, n_vocab: usize) -> i32 {
    // Find top-k candidates first
    let mut indices: Vec<usize> = (0..n_vocab).collect();

    if top_k > 0 {
        indices.sort_by(|&a, &b| logits[b].partial_cmp(&logits[a]).unwrap());
        indices.truncate(top_k);
    } else {
        indices.sort_by(|&a, &b| logits[b].partial_cmp(&logits[a]).unwrap());
    }

    // Apply temperature and compute probabilities
    let mut probs: Vec<f32> = indices.iter()
        .map(|&i| {
            let logit = logits[i];
            if temperature > 0.0 && temperature != 1.0 {
                logit / temperature
            } else {
                logit
            }
        })
        .collect();

    // Compute softmax
    let max_logit = probs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp_sum: f32 = probs.iter().map(|&l| (l - max_logit).exp()).sum();

    if top_p < 1.0 && !probs.is_empty() {
        // Top-p (nucleus) sampling
        let mut cumsum = 0.0f32;
        for (i, prob) in probs.iter_mut().enumerate() {
            let p = (*prob - max_logit).exp() / exp_sum;
            cumsum += p;
            if cumsum > top_p {
                // Zero out remaining probabilities
                for j in i..probs.len() {
                    probs[j] = f32::NEG_INFINITY;
                }
                break;
            }
        }
    }

    // Find the selected index - probs corresponds to top-k candidates in indices
    let mut max_idx = 0;
    let mut max_val = f32::NEG_INFINITY;
    for (i, &_idx) in indices.iter().enumerate() {
        if probs[i] > max_val {
            max_val = probs[i];
            max_idx = i;
        }
    }

    // Return the actual token ID from the sorted indices array
    indices[max_idx] as i32
}