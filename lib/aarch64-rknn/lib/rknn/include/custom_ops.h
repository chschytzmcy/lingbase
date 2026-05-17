#ifndef CUSTOM_OPS_H
#define CUSTOM_OPS_H

#ifdef __cplusplus
extern "C" {
#endif

#include "rknn_custom_op.h"

// =============== Custom Operator Registration Functions ===============

// Register CumSum custom operator
// ctx: rknn context
// return: 0 succeed, others fail
int register_cumsum_custom_op(rknn_context ctx);

// =============== Custom Operator Callback Functions ===============

// CumSum operator callbacks
int cumsum_init_callback(rknn_custom_op_context* op_ctx,
                         rknn_custom_op_tensor* inputs, uint32_t n_inputs,
                         rknn_custom_op_tensor* outputs, uint32_t n_outputs);

int cumsum_compute_callback(rknn_custom_op_context* op_ctx,
                            rknn_custom_op_tensor* inputs, uint32_t n_inputs,
                            rknn_custom_op_tensor* outputs, uint32_t n_outputs);

int cumsum_destroy_callback(rknn_custom_op_context* op_ctx);

#ifdef __cplusplus
}
#endif

#endif // CUSTOM_OPS_H