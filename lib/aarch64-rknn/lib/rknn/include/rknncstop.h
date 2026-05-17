#ifndef RKNNCSTOP_H
#define RKNNCSTOP_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include "rknn_api.h"

// =============== Context Interface ===============

// Create an rknn context
// ctx: pointer to rknn context
// model: pointer to model buffer in memory
// size: size of model buffer
// flag: extended flag for initialization
// extend: extended parameter for initialization
// return: 0 succeed, others fail
int rknncstop_init(rknn_context* ctx, void* model, uint32_t size, uint32_t flag, rknn_init_extend* extend);

// Duplicate an rknn context
// ctx_in: pointer to source rknn context
// ctx_out: pointer to destination rknn context
// return: 0 succeed, others fail
int rknncstop_dup_context(rknn_context* ctx_in, rknn_context* ctx_out);

// Destroy rknn context
// ctx: rknn context
// return: 0 succeed, others fail
int rknncstop_destroy(rknn_context ctx);

// =============== Model Interface ===============

// There is no separate load/unload functions in RKNN API, model is loaded during init

// =============== Query Interface ===============

// Query information of rknn model
// ctx: rknn context
// cmd: query command
// info: pointer to input information
// size: size of input information
// return: 0 succeed, others fail
int rknncstop_query(rknn_context ctx, rknn_query_cmd cmd, void* info, uint32_t size);

// =============== Input/Ouput Interface ===============

// Set input shape
// ctx: rknn context
// attr: pointer to input tensor attributes
// return: 0 succeed, others fail
int rknncstop_set_input_shape(rknn_context ctx, rknn_tensor_attr* attr);

// Set input shapes for multiple inputs
// ctx: rknn context
// n_inputs: number of inputs
// attr: pointer to array of input tensor attributes
// return: 0 succeed, others fail
int rknncstop_set_input_shapes(rknn_context ctx, uint32_t n_inputs, rknn_tensor_attr* attr);

// Set input data
// ctx: rknn context
// n_inputs: number of inputs
// inputs: pointer to array of input structures
// return: 0 succeed, others fail
int rknncstop_inputs_set(rknn_context ctx, uint32_t n_inputs, rknn_input inputs[]);

// Get output data
// ctx: rknn context
// n_outputs: number of outputs
// outputs: pointer to array of output structures
// extend: pointer to extended parameters (optional)
// return: 0 succeed, others fail
int rknncstop_outputs_get(rknn_context ctx, uint32_t n_outputs, rknn_output outputs[], rknn_output_extend* extend);

// Release output buffer
// ctx: rknn context
// n_outputs: number of outputs
// outputs: pointer to array of output structures
// return: 0 succeed, others fail
int rknncstop_outputs_release(rknn_context ctx, uint32_t n_outputs, rknn_output outputs[]);

// =============== Memory Interface ===============

// Create memory
// ctx: rknn context
// size: memory size
// data: pointer to data buffer
// return: pointer to created memory structure
rknn_tensor_mem* rknncstop_create_mem(rknn_context ctx, uint32_t size, void* data);

// Destroy memory
// ctx: rknn context
// mem: pointer to memory structure
// return: 0 succeed, others fail
int rknncstop_destroy_mem(rknn_context ctx, rknn_tensor_mem* mem);

// Set input memory
// ctx: rknn context
// mem: pointer to memory structure
// attr: pointer to tensor attributes
// return: 0 succeed, others fail
int rknncstop_set_input_mem(rknn_context ctx, rknn_tensor_mem* mem, rknn_tensor_attr* attr);

// Set output memory
// ctx: rknn context
// mem: pointer to memory structure
// attr: pointer to tensor attributes
// return: 0 succeed, others fail
int rknncstop_set_output_mem(rknn_context ctx, rknn_tensor_mem* mem, rknn_tensor_attr* attr);

// =============== Run Interface ===============

// Run model
// ctx: rknn context
// extend: pointer to extended parameters (optional)
// return: 0 succeed, others fail
int rknncstop_run(rknn_context ctx, rknn_run_extend* extend);

// =============== Custom Operator Interface ===============

// Register custom operators by names
// ctx: rknn context
// op_names: comma-separated list of custom operator names to register, e.g., "cumsum,relu"
//            or "cumsum" or empty string for none
// return: 0 succeed, others fail
int rknncstop_register_custom_ops_by_name(rknn_context ctx, const char* op_names);

#ifdef __cplusplus
}
#endif

#endif // RKNNCSTOP_H