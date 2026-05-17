//! Safe wrappers for RKNN3 Runtime API.
//!
//! All `unsafe` FFI calls are encapsulated here. Consumers of `rknn3-sys`
//! should use these types and functions without needing `unsafe`.

use std::ffi::CString;
use std::path::Path;
use std::ptr;

use crate::error::Error;
use crate::ffi::raw;

// ---------------------------------------------------------------------------
// Safe wrapper types (replacing raw types in public API)
// ---------------------------------------------------------------------------

/// Memory allocation flags for NPU tensor memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemAllocFlags {
    /// Cacheable memory allocation.
    #[default]
    Cacheable,
    /// Non-cacheable memory allocation.
    NonCacheable,
}

impl From<MemAllocFlags> for raw::rknn3_mem_alloc_flags {
    fn from(f: MemAllocFlags) -> Self {
        match f {
            MemAllocFlags::Cacheable => raw::_rknn3_mem_alloc_flags_RKNN3_FLAG_MEMORY_CACHEABLE,
            MemAllocFlags::NonCacheable => {
                raw::_rknn3_mem_alloc_flags_RKNN3_FLAG_MEMORY_NON_CACHEABLE
            }
        }
    }
}

/// Memory synchronization mode for CPU-device transfers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemSyncMode {
    /// Synchronize from host (CPU) to device (NPU).
    ToDevice,
    /// Synchronize from device (NPU) to host (CPU).
    FromDevice,
    /// Bidirectional synchronization.
    Bidirectional,
}

impl From<MemSyncMode> for raw::rknn3_mem_sync_mode {
    fn from(m: MemSyncMode) -> Self {
        match m {
            MemSyncMode::ToDevice => raw::_rknn3_mem_sync_mode_RKNN3_MEMORY_SYNC_TO_DEVICE,
            MemSyncMode::FromDevice => raw::_rknn3_mem_sync_mode_RKNN3_MEMORY_SYNC_FROM_DEVICE,
            MemSyncMode::Bidirectional => raw::_rknn3_mem_sync_mode_RKNN3_MEMORY_SYNC_BIDIRECTIONAL,
        }
    }
}

/// Safe wrapper for NPU tensor memory.
///
/// Wraps `rknn3_tensor_mem` (`_rknn3_tensor_memory`). Created via [`Context::create_mem`].
pub struct TensorMem {
    /// Pointer to the underlying raw tensor memory (owned by the C runtime).
    ptr: *mut raw::rknn3_tensor_mem,
}

// SAFETY: rknn3_tensor_mem is a C struct of plain data and pointers;
// access is mediated through Context methods which take &self.
unsafe impl Send for TensorMem {}

impl TensorMem {
    /// Virtual address of the tensor buffer.
    pub fn virt_addr(&self) -> *mut std::ffi::c_void {
        unsafe { (*self.ptr).virt_addr }
    }

    /// Physical address of the tensor buffer.
    pub fn phys_addr(&self) -> u64 {
        unsafe { (*self.ptr).phys_addr }
    }

    /// File descriptor of the tensor buffer.
    pub fn fd(&self) -> i32 {
        unsafe { (*self.ptr).fd }
    }

    /// Size of the tensor buffer in bytes.
    pub fn size(&self) -> u64 {
        unsafe { (*self.ptr).size }
    }

    /// Offset of the memory.
    pub fn offset(&self) -> u64 {
        unsafe { (*self.ptr).offset }
    }

    /// NPU core ID.
    pub fn core_id(&self) -> i32 {
        unsafe { (*self.ptr).core_id }
    }

    /// Return the raw pointer for internal use.
    pub(crate) fn as_ptr(&self) -> *mut raw::rknn3_tensor_mem {
        self.ptr
    }
}

/// Safe wrapper for a tensor used in RKNN3 inference.
///
/// Wraps `rknn3_tensor` which contains a memory pointer and attribute pointer.
#[derive(Debug, Clone, Copy)]
pub struct Tensor {
    mem: Option<*mut raw::rknn3_tensor_mem>,
    attr: Option<*mut raw::rknn3_tensor_attr>,
}

impl Tensor {
    /// Create a new empty tensor (both memory and attribute are null).
    pub fn new() -> Self {
        Self {
            mem: None,
            attr: None,
        }
    }

    /// Create a tensor wrapping a [`TensorMem`].
    pub fn with_mem(mem: &TensorMem) -> Self {
        Self {
            mem: Some(mem.as_ptr()),
            attr: None,
        }
    }

    /// Convert to raw `rknn3_tensor`.
    pub(crate) fn to_raw(self) -> raw::rknn3_tensor {
        raw::rknn3_tensor {
            mem: self.mem.unwrap_or(ptr::null_mut()),
            attr: self.attr.unwrap_or(ptr::null_mut()),
        }
    }
}

impl Default for Tensor {
    fn default() -> Self {
        Self::new()
    }
}

/// Image format for image memory operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// RGB888 (3 bytes per pixel).
    Rgb888,
    /// BGR888 (3 bytes per pixel).
    Bgr888,
    /// 8-bit grayscale.
    Gray8,
    /// YCbCr 4:2:0 semi-planar (NV21).
    YCbCr420Sp,
    /// YCrCb 4:2:0 semi-planar (NV12).
    YCrCb420Sp,
    /// YCbCr 4:2:2 semi-planar.
    YCbCr422Sp,
    /// YCrCb 4:2:2 semi-planar.
    YCrCb422Sp,
    /// Unknown format.
    Unknown,
}

impl From<ImageFormat> for raw::rknn3_im_fmt {
    fn from(f: ImageFormat) -> Self {
        match f {
            ImageFormat::Rgb888 => raw::_rknn3_im_fmt_RKNN3_IM_FMT_RGB888,
            ImageFormat::Bgr888 => raw::_rknn3_im_fmt_RKNN3_IM_FMT_BGR888,
            ImageFormat::Gray8 => raw::_rknn3_im_fmt_RKNN3_IM_FMT_GRAY8,
            ImageFormat::YCbCr420Sp => raw::_rknn3_im_fmt_RKNN3_IM_FMT_YCbCr_420_SP,
            ImageFormat::YCrCb420Sp => raw::_rknn3_im_fmt_RKNN3_IM_FMT_YCrCb_420_SP,
            ImageFormat::YCbCr422Sp => raw::_rknn3_im_fmt_RKNN3_IM_FMT_YCbCr_422_SP,
            ImageFormat::YCrCb422Sp => raw::_rknn3_im_fmt_RKNN3_IM_FMT_YCrCb_422_SP,
            ImageFormat::Unknown => raw::_rknn3_im_fmt_RKNN3_IM_FMT_UNKNOWN,
        }
    }
}

/// Query command for `rknn3_query`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryCmd {
    /// Query input/output tensor count.
    InOutNum,
    /// Query input tensor attributes.
    InputAttr,
    /// Query output tensor attributes.
    OutputAttr,
    /// Query SDK version.
    SdkVersion,
    /// Query NPU core memory size.
    CoreMemSize,
    /// Query native input tensor attributes.
    NativeInputAttr,
    /// Query native output tensor attributes.
    NativeOutputAttr,
    /// Query device memory info.
    DeviceMemInfo,
    /// Query NPU core count.
    CoreNumber,
    /// Query allocation info.
    AllocationInfo,
    /// Query dynamic shape configuration.
    DynamicShapeConfig,
    /// Query dynamic shape info.
    DynamicShapeInfo,
    /// Query LLM configuration.
    LlmConfig,
    /// Query post-process input/output tensor count.
    PostProcessInOutNum,
    /// Query post-process output tensor attributes.
    PostProcessOutputAttr,
    /// Query post-process dynamic shape info.
    PostProcessDynamicShapeInfo,
}

impl From<QueryCmd> for raw::rknn3_query_cmd {
    fn from(c: QueryCmd) -> Self {
        match c {
            QueryCmd::InOutNum => raw::_rknn3_query_cmd_RKNN3_QUERY_IN_OUT_NUM,
            QueryCmd::InputAttr => raw::_rknn3_query_cmd_RKNN3_QUERY_INPUT_ATTR,
            QueryCmd::OutputAttr => raw::_rknn3_query_cmd_RKNN3_QUERY_OUTPUT_ATTR,
            QueryCmd::SdkVersion => raw::_rknn3_query_cmd_RKNN3_QUERY_SDK_VERSION,
            QueryCmd::CoreMemSize => raw::_rknn3_query_cmd_RKNN3_QUERY_CORE_MEM_SIZE,
            QueryCmd::NativeInputAttr => raw::_rknn3_query_cmd_RKNN3_QUERY_NATIVE_INPUT_ATTR,
            QueryCmd::NativeOutputAttr => raw::_rknn3_query_cmd_RKNN3_QUERY_NATIVE_OUTPUT_ATTR,
            QueryCmd::DeviceMemInfo => raw::_rknn3_query_cmd_RKNN3_QUERY_DEVICE_MEM_INFO,
            QueryCmd::CoreNumber => raw::_rknn3_query_cmd_RKNN3_QUERY_CORE_NUMBER,
            QueryCmd::AllocationInfo => raw::_rknn3_query_cmd_RKNN3_QUERY_ALLOCATION_INFO,
            QueryCmd::DynamicShapeConfig => raw::_rknn3_query_cmd_RKNN3_QUERY_DYNAMIC_SHAPE_CONFIG,
            QueryCmd::DynamicShapeInfo => raw::_rknn3_query_cmd_RKNN3_QUERY_DYNAMIC_SHAPE_INFO,
            QueryCmd::LlmConfig => raw::_rknn3_query_cmd_RKNN3_QUERY_LLM_CONFIG,
            QueryCmd::PostProcessInOutNum => {
                raw::_rknn3_query_cmd_RKNN3_QUERY_POSTPROCESS_IN_OUT_NUM
            }
            QueryCmd::PostProcessOutputAttr => {
                raw::_rknn3_query_cmd_RKNN3_QUERY_POSTPROCESS_OUTPUT_ATTR
            }
            QueryCmd::PostProcessDynamicShapeInfo => {
                raw::_rknn3_query_cmd_RKNN3_QUERY_POSTPROCESS_DYNAMIC_SHAPE_INFO
            }
        }
    }
}

/// Information about an available NPU device.
#[derive(Debug, Clone)]
pub struct Device {
    /// Device ID string.
    pub id: String,
    /// Device type string.
    pub device_type: String,
}

/// KV cache policy for LLM sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KvCachePolicy {
    /// Recurrent KV cache policy.
    #[default]
    Recurrent,
    /// Normal KV cache policy.
    Normal,
}

impl From<KvCachePolicy> for raw::rknn3_kvcache_policy {
    fn from(p: KvCachePolicy) -> Self {
        match p {
            KvCachePolicy::Recurrent => raw::rknn3_kvcache_policy_RKNN3_KVCACHE_POLICY_RECURRENT,
            KvCachePolicy::Normal => raw::rknn3_kvcache_policy_RKNN3_KVCACHE_POLICY_NORMAL,
        }
    }
}

/// Parameters for recurrent KV cache policy.
#[derive(Debug, Clone, Copy)]
pub struct KvCachePolicyParam {
    /// Number of cache entries to keep.
    pub n_keep: i64,
    /// Aligned number of cache entries to keep.
    pub n_keep_aligned: i64,
}

impl KvCachePolicyParam {
    /// Create new policy parameters.
    pub fn new(n_keep: i64, n_keep_aligned: i64) -> Self {
        Self {
            n_keep,
            n_keep_aligned,
        }
    }

    /// Convert to raw `rknn3_kvcache_policy_param`.
    #[allow(clippy::wrong_self_convention)]
    fn to_raw(&self) -> raw::rknn3_kvcache_policy_param {
        raw::rknn3_kvcache_policy_param {
            recurrent: raw::_rknn3_kvcache_policy_param__bindgen_ty_1 {
                n_keep: self.n_keep,
                n_keep_aligned: self.n_keep_aligned,
            },
            reserved: [0u8; 64],
        }
    }
}

/// KV cache clear policy for LLM sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KvCacheClearPolicy {
    /// Clear all KV cache entries.
    #[default]
    All,
    /// Keep the system prompt in KV cache.
    KeepSystemPrompt,
}

impl From<KvCacheClearPolicy> for raw::rknn3_kvcache_clear_policy {
    fn from(p: KvCacheClearPolicy) -> Self {
        match p {
            KvCacheClearPolicy::All => raw::rknn3_kvcache_clear_policy_RKNN3_KVCACHE_CLEAR_ALL,
            KvCacheClearPolicy::KeepSystemPrompt => {
                raw::rknn3_kvcache_clear_policy_RKNN3_KVCACHE_KEEP_SYSTEM_PROMPT
            }
        }
    }
}

/// LoRA adapter information.
#[derive(Debug, Clone)]
pub struct Lora {
    /// Name of the LoRA adapter.
    pub name: String,
    /// Scaling factor.
    pub scale: f32,
}

impl Lora {
    /// Create a new LoRA adapter description.
    pub fn new(name: impl Into<String>, scale: f32) -> Self {
        Self {
            name: name.into(),
            scale,
        }
    }

    /// Convert to raw `rknn3_lora`.
    fn to_raw(&self) -> raw::rknn3_lora {
        let mut lora_name: [std::os::raw::c_char; 256] = [0; 256];
        let name_bytes = self.name.as_bytes();
        let len = name_bytes.len().min(255);
        let name_cstr: Vec<std::os::raw::c_char> = name_bytes[..len]
            .iter()
            .map(|&b| b as std::os::raw::c_char)
            .collect();
        lora_name[..len].copy_from_slice(&name_cstr);
        raw::rknn3_lora {
            lora_name,
            scale: self.scale,
        }
    }

    /// Convert from raw `rknn3_lora`.
    fn from_raw(raw: &raw::rknn3_lora) -> Self {
        Self {
            name: c_chars_to_string(&raw.lora_name),
            scale: raw.scale,
        }
    }
}

/// LLM session run state.
#[derive(Debug, Clone)]
pub struct RunState {
    /// Total number of tokens processed.
    pub n_total_tokens: u64,
    /// Maximum number of tokens that can be processed.
    pub n_max_tokens: u64,
    /// Number of decode tokens generated.
    pub n_decode_tokens: u64,
    /// Number of prefill tokens processed.
    pub n_prefill_tokens: u64,
    /// Number of LoRA adapters enabled.
    pub n_loras_enabled: i32,
}

impl RunState {
    /// Convert from raw `RKLLMRunState`.
    fn from_raw(raw: &raw::RKLLMRunState) -> Self {
        Self {
            n_total_tokens: raw.n_total_tokens,
            n_max_tokens: raw.n_max_tokens,
            n_decode_tokens: raw.n_decode_tokens,
            n_prefill_tokens: raw.n_prefill_tokens,
            n_loras_enabled: raw.n_loras_enabled,
        }
    }
}

/// Convert a null-terminated C char array to a String.
fn c_chars_to_string(chars: &[std::os::raw::c_char]) -> String {
    let bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(chars.as_ptr() as *const u8, chars.len()) };
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

// ---------------------------------------------------------------------------
// LlmConfig — LLM model configuration (queried via rknn3_query)
// ---------------------------------------------------------------------------

/// KV Cache data type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum KvCacheDtype {
    Undefined = 0,
    Int4ToF16 = 1,
    Int4ToF8 = 2,
    Int8ToF16 = 3,
    Float4ToF16 = 4,
    Float4ToF8 = 5,
    Float8ToF16 = 6,
    Float8ToF8 = 7,
    Float16 = 8,
}

/// KV Cache store method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum KvCacheStoreMethod {
    Undefined = 0,
    Normal = 1,
    GroupQuant = 2,
}

/// LLM task type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LlmTaskType {
    Generate = 0,
    Embedding = 1,
}

/// LLM configuration reported by the loaded model.
///
/// Obtained via `Context::query_llm_config()` after `model_init()`.
#[derive(Debug)]
pub struct LlmConfig {
    pub vocab_size: u32,
    pub embedding_dim: u32,
    pub max_ctx_len: u32,
    pub max_position_embeddings: u32,
    pub kvcache_store_method: KvCacheStoreMethod,
    pub kvcache_dtype: KvCacheDtype,
    pub kvcache_group_size: u32,
    pub kvcache_residual_depth: u32,
    pub model_type: Option<String>,
    pub task_type: LlmTaskType,
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

/// RAII wrapper around `rknn3_context`.
///
/// Created via [`Context::new()`], destroyed automatically on drop.
pub struct Context {
    ctx: raw::rknn3_context,
}

impl Context {
    /// Initialize the RKNN3 runtime.
    pub fn new() -> Result<Self, Error> {
        let mut ctx: raw::rknn3_context = 0;
        // SAFETY: rknn3_init initializes the context; null extend is documented as valid.
        let ret = unsafe { raw::rknn3_init(&mut ctx, ptr::null_mut()) };
        Error::check("rknn3_init", ret)?;
        Ok(Self { ctx })
    }

    /// Initialize with device ID extension.
    pub fn with_device_id(device_id: &str) -> Result<Self, Error> {
        let mut ctx: raw::rknn3_context = 0;
        let device_cstr = CString::new(device_id).map_err(|_| Error::nul_byte("device_id"))?;
        let mut extend = raw::rknn3_init_extend {
            device_id: device_cstr.as_ptr() as *mut _,
            reserved: [0u8; 128],
        };
        // SAFETY: extend is stack-allocated and valid for the call duration.
        // device_cstr is kept alive for the duration of this call.
        let ret = unsafe { raw::rknn3_init(&mut ctx, &mut extend) };
        Error::check("rknn3_init", ret)?;
        Ok(Self { ctx })
    }

    /// Load a model from file paths.
    pub fn load_model<P: AsRef<Path>>(&self, model_path: P, weight_path: P) -> Result<(), Error> {
        let model_cstr = to_cstr(model_path.as_ref(), "model_path")?;
        let weight_cstr = to_cstr(weight_path.as_ref(), "weight_path")?;
        // SAFETY: ctx is valid (from new()), paths are valid CStrings.
        let ret = unsafe {
            raw::rknn3_load_model_from_path(self.ctx, model_cstr.as_ptr(), weight_cstr.as_ptr())
        };
        Error::check("rknn3_load_model_from_path", ret)
    }

    /// Load a model from memory buffers.
    pub fn load_model_from_data(&self, model_data: &[u8], weight_data: &[u8]) -> Result<(), Error> {
        // SAFETY: ctx is valid, data pointers and sizes come from valid slices.
        let ret = unsafe {
            raw::rknn3_load_model_from_data(
                self.ctx,
                model_data.as_ptr() as *const std::ffi::c_void,
                model_data.len() as u64,
                weight_data.as_ptr() as *const std::ffi::c_void,
                weight_data.len() as u64,
            )
        };
        Error::check("rknn3_load_model_from_data", ret)
    }

    /// Initialize the loaded model with configuration.
    pub fn model_init(&self, config: &mut ModelConfig) -> Result<(), Error> {
        let mut ffi_config = config.to_ffi();
        // SAFETY: ctx is valid, ffi_config is a valid mutable reference.
        let ret = unsafe { raw::rknn3_model_init(self.ctx, &mut ffi_config) };
        Error::check("rknn3_model_init", ret)?;
        Ok(())
    }

    /// Duplicate this context.
    pub fn dup_context(&self) -> Result<Self, Error> {
        let mut new_ctx: raw::rknn3_context = 0;
        // SAFETY: ctx is valid, new_ctx is a valid output pointer.
        let ret = unsafe { raw::rknn3_dup_context(self.ctx, &mut new_ctx) };
        Error::check("rknn3_dup_context", ret)?;
        Ok(Self { ctx: new_ctx })
    }

    /// Run synchronous inference.
    pub fn run(&self, inputs: &[Tensor], outputs: &mut [Tensor]) -> Result<(), Error> {
        let raw_inputs: Vec<raw::rknn3_tensor> =
            inputs.iter().map(|t| Tensor::to_raw(*t)).collect();
        let mut raw_outputs: Vec<raw::rknn3_tensor> =
            outputs.iter_mut().map(|t| Tensor::to_raw(*t)).collect();
        // SAFETY: ctx is valid, input/output arrays are valid for their lengths.
        let ret = unsafe {
            raw::rknn3_run(
                self.ctx,
                raw_inputs.as_ptr(),
                raw_inputs.len() as u32,
                raw_outputs.as_mut_ptr(),
                raw_outputs.len() as u32,
            )
        };
        Error::check("rknn3_run", ret)
    }

    /// Run asynchronous inference.
    pub fn run_async(&self, inputs: &[Tensor], outputs: &mut [Tensor]) -> Result<(), Error> {
        let raw_inputs: Vec<raw::rknn3_tensor> =
            inputs.iter().map(|t| Tensor::to_raw(*t)).collect();
        let mut raw_outputs: Vec<raw::rknn3_tensor> =
            outputs.iter_mut().map(|t| Tensor::to_raw(*t)).collect();
        // SAFETY: ctx is valid, input/output arrays are valid.
        let ret = unsafe {
            raw::rknn3_run_async(
                self.ctx,
                raw_inputs.as_ptr(),
                raw_inputs.len() as u32,
                raw_outputs.as_mut_ptr(),
                raw_outputs.len() as u32,
            )
        };
        Error::check("rknn3_run_async", ret)
    }

    /// Wait for asynchronous inference to complete.
    pub fn wait(&self) -> Result<(), Error> {
        // SAFETY: ctx is valid.
        let ret = unsafe { raw::rknn3_wait(self.ctx) };
        Error::check("rknn3_wait", ret)
    }

    /// Query runtime/model information.
    pub fn query<T>(&self, cmd: QueryCmd, info: &mut T) -> Result<(), Error> {
        let raw_cmd: raw::rknn3_query_cmd = cmd.into();
        // SAFETY: ctx is valid, info is a valid mutable pointer of correct size.
        let ret = unsafe {
            raw::rknn3_query(
                self.ctx,
                raw_cmd,
                info as *mut T as *mut std::ffi::c_void,
                std::mem::size_of::<T>() as u64,
            )
        };
        Error::check("rknn3_query", ret)
    }

    /// Query LLM configuration (must be called after `model_init`).
    pub fn query_llm_config(&self) -> Result<LlmConfig, Error> {
        let mut raw_cfg: raw::rknn3_llm_config = unsafe { std::mem::zeroed() };
        self.query(QueryCmd::LlmConfig, &mut raw_cfg)?;

        let model_type = if raw_cfg.model_type.is_null() {
            None
        } else {
            // SAFETY: model_type is a valid C string returned by the API.
            let s = unsafe { std::ffi::CStr::from_ptr(raw_cfg.model_type) };
            Some(s.to_string_lossy().into_owned())
        };

        Ok(LlmConfig {
            vocab_size: raw_cfg.vocab_size,
            embedding_dim: raw_cfg.embedding_dim,
            max_ctx_len: raw_cfg.max_ctx_len,
            max_position_embeddings: raw_cfg.max_position_embeddings,
            kvcache_store_method: unsafe {
                std::mem::transmute::<u32, KvCacheStoreMethod>(raw_cfg.kvcache_store_method)
            },
            kvcache_dtype: unsafe {
                std::mem::transmute::<u32, KvCacheDtype>(raw_cfg.kvcache_dtype)
            },
            kvcache_group_size: raw_cfg.kvcache_group_size,
            kvcache_residual_depth: raw_cfg.kvcache_residual_depth,
            model_type,
            task_type: unsafe { std::mem::transmute::<u32, LlmTaskType>(raw_cfg.task_type) },
        })
    }

    /// Allocate NPU tensor memory.
    pub fn create_mem(
        &self,
        size: u64,
        core_id: i32,
        flags: MemAllocFlags,
    ) -> Result<TensorMem, Error> {
        let raw_flags: raw::rknn3_mem_alloc_flags = flags.into();
        // SAFETY: ctx is valid, returns pointer or null on failure.
        let mem = unsafe { raw::rknn3_create_mem(self.ctx, size, core_id, raw_flags) };
        if mem.is_null() {
            return Err(Error::NullHandle {
                context: "rknn3_create_mem",
            });
        }
        Ok(TensorMem { ptr: mem })
    }

    /// Destroy NPU tensor memory.
    pub fn destroy_mem(&self, mem: &TensorMem) -> Result<(), Error> {
        // SAFETY: ctx and mem are valid.
        let ret = unsafe { raw::rknn3_destroy_mem(self.ctx, mem.as_ptr()) };
        Error::check("rknn3_destroy_mem", ret)
    }

    /// Synchronize CPU and device memory.
    pub fn mem_sync(&self, mem: &TensorMem, mode: MemSyncMode) -> Result<(), Error> {
        let raw_mode: raw::rknn3_mem_sync_mode = mode.into();
        // SAFETY: ctx and mem are valid.
        let ret = unsafe { raw::rknn3_mem_sync(self.ctx, mem.as_ptr(), raw_mode) };
        Error::check("rknn3_mem_sync", ret)
    }

    /// Set dynamic input shape by shape ID.
    pub fn set_shape(&self, shape_id: i32) -> Result<(), Error> {
        // SAFETY: ctx is valid, shape_id is a valid ID.
        let ret = unsafe { raw::rknn3_set_shape(self.ctx, shape_id) };
        Error::check("rknn3_set_shape", ret)
    }

    /// Find available NPU devices.
    pub fn find_devices() -> Result<Vec<Device>, Error> {
        let mut devices: raw::rknn3_devices = unsafe { std::mem::zeroed() };
        // SAFETY: devices is stack-allocated with sufficient capacity.
        let ret = unsafe { raw::rknn3_find_devices(&mut devices) };
        Error::check("rknn3_find_devices", ret)?;
        let count = devices.n_devices as usize;
        Ok(devices.devices[..count]
            .iter()
            .map(|d| Device {
                id: c_chars_to_string(&d.id),
                device_type: c_chars_to_string(&d.type_),
            })
            .collect())
    }

    /// Dump layer features to .npy files for debugging.
    pub fn dump_features(
        &self,
        inputs: &[Tensor],
        outputs: &mut [Tensor],
        dump_dir: &str,
    ) -> Result<(), Error> {
        let cstr = CString::new(dump_dir).map_err(|_| Error::nul_byte("dump_dir"))?;
        let raw_inputs: Vec<raw::rknn3_tensor> =
            inputs.iter().map(|t| Tensor::to_raw(*t)).collect();
        let mut raw_outputs: Vec<raw::rknn3_tensor> =
            outputs.iter_mut().map(|t| Tensor::to_raw(*t)).collect();
        // SAFETY: ctx is valid, tensors and dump_dir are valid.
        let ret = unsafe {
            raw::rknn3_dump_features(
                self.ctx,
                raw_inputs.as_ptr(),
                raw_inputs.len() as u32,
                raw_outputs.as_mut_ptr(),
                raw_outputs.len() as u32,
                cstr.as_ptr(),
            )
        };
        Error::check("rknn3_dump_features", ret)
    }

    /// Profile operator-level timing.
    pub fn profile_ops(
        &self,
        inputs: &[Tensor],
        outputs: &mut [Tensor],
        log_level: u32,
    ) -> Result<(), Error> {
        let raw_inputs: Vec<raw::rknn3_tensor> =
            inputs.iter().map(|t| Tensor::to_raw(*t)).collect();
        let mut raw_outputs: Vec<raw::rknn3_tensor> =
            outputs.iter_mut().map(|t| Tensor::to_raw(*t)).collect();
        // SAFETY: ctx is valid, tensors are valid.
        let ret = unsafe {
            raw::rknn3_profile_ops(
                self.ctx,
                raw_inputs.as_ptr(),
                raw_inputs.len() as u32,
                raw_outputs.as_mut_ptr(),
                raw_outputs.len() as u32,
                log_level,
            )
        };
        Error::check("rknn3_profile_ops", ret)
    }

    /// Profile memory usage.
    pub fn profile_mem(&self) -> Result<(), Error> {
        // SAFETY: ctx is valid.
        let ret = unsafe { raw::rknn3_profile_mem(self.ctx) };
        Error::check("rknn3_profile_mem", ret)
    }

    /// Register custom operator plugins from a shared library.
    pub fn register_custom_ops_plugins(&self, plugin_path: &str) -> Result<(), Error> {
        let cstr = CString::new(plugin_path).map_err(|_| Error::nul_byte("plugin_path"))?;
        // SAFETY: ctx is valid, plugin_path is a valid CString.
        let ret = unsafe { raw::rknn3_register_custom_ops_plugins(self.ctx, cstr.as_ptr(), 0) };
        Error::check("rknn3_register_custom_ops_plugins", ret)
    }

    /// Return the raw context handle for internal crate use.
    pub(crate) fn ctx_handle(&self) -> raw::rknn3_context {
        self.ctx
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if self.ctx != 0 {
            // SAFETY: ctx is valid (non-zero) and owned by this instance.
            let ret = unsafe { raw::rknn3_destroy(self.ctx) };
            if ret != 0 {
                tracing::warn!("rknn3_destroy failed with code {}", ret);
            }
        }
    }
}

// Context 不是 Sync — 单个 Context 不能在多线程间共享（C API 未声明线程安全）。
// 多线程场景需通过 dup_context() 为每个线程创建独立副本。
// SAFETY: rknn3_context 是一个 u64 句柄值，可以在线程间移动（Send）。
unsafe impl Send for Context {}

// ---------------------------------------------------------------------------
// LLM Callback State
// ---------------------------------------------------------------------------

/// Safe enum for LLM call state (mirrors raw `LLMCallState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmCallState {
    Normal,
    Waiting,
    Finish,
    Stop,
    MaxNewTokensReached,
    Error,
}

impl From<raw::LLMCallState> for LlmCallState {
    fn from(state: raw::LLMCallState) -> Self {
        match state {
            raw::LLMCallState_RKLLM_RUN_NORMAL => Self::Normal,
            raw::LLMCallState_RKLLM_RUN_WAITING => Self::Waiting,
            raw::LLMCallState_RKLLM_RUN_FINISH => Self::Finish,
            raw::LLMCallState_RKLLM_RUN_STOP => Self::Stop,
            raw::LLMCallState_RKLLM_RUN_MAX_NEW_TOKEN_REACHED => Self::MaxNewTokensReached,
            raw::LLMCallState_RKLLM_RUN_ERROR => Self::Error,
            _ => Self::Error,
        }
    }
}

// ---------------------------------------------------------------------------
// LLM Callbacks
// ---------------------------------------------------------------------------

/// Error type for LLM callback operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackError {
    TokenizeFailed,
    EmbedFailed,
    InvalidInput,
}

impl std::fmt::Display for CallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TokenizeFailed => write!(f, "tokenization failed"),
            Self::EmbedFailed => write!(f, "embedding lookup failed"),
            Self::InvalidInput => write!(f, "invalid callback input"),
        }
    }
}

impl std::error::Error for CallbackError {}

/// Trait for LLM session callbacks.
///
/// Implement this to handle tokenizer, embedding, and result callbacks.
pub trait LlmCallbacks {
    /// Tokenize text. Write token IDs into `buf` and return the count written.
    fn tokenize(&self, text: &str, buf: &mut [i32]) -> Result<usize, CallbackError>;

    /// Look up embeddings for token IDs. Write f16 bytes into `buf`.
    fn embed(&self, tokens: &[i32], buf: &mut [u8]) -> Result<(), CallbackError>;

    /// Handle inference result tokens.
    fn on_result(&mut self, token_ids: &[i32], state: LlmCallState);
}

// ---------------------------------------------------------------------------
// LLM Parameters
// ---------------------------------------------------------------------------

/// Safe wrapper for `rknn3_llm_param` — configuration for creating an LLM session.
pub struct LlmParams {
    logits_name: Option<CString>,
    max_context_len: i32,
    top_k: i32,
    top_p: f32,
    temperature: f32,
    repeat_penalty: f32,
    frequency_penalty: f32,
    presence_penalty: f32,
    vocab_size: i32,
    special_bos_id: [i32; 64],
    n_special_bos_id: i32,
    special_eos_id: [i32; 64],
    n_special_eos_id: i32,
    linefeed_id: i32,
    skip_special_token: bool,
    ignore_eos_token: bool,
}

impl LlmParams {
    pub fn new(logits_name: &str, vocab_size: i32) -> Result<Self, Error> {
        let logits_name = CString::new(logits_name).map_err(|_| Error::nul_byte("logits_name"))?;
        Ok(Self {
            logits_name: Some(logits_name),
            max_context_len: 4096,
            top_k: 1,
            top_p: 0.9,
            temperature: 1.0,
            repeat_penalty: 1.2,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            vocab_size,
            special_bos_id: [0; 64],
            n_special_bos_id: 0,
            special_eos_id: [0; 64],
            n_special_eos_id: 0,
            linefeed_id: 0,
            skip_special_token: true,
            ignore_eos_token: false,
        })
    }

    pub fn max_context_len(mut self, len: i32) -> Self {
        self.max_context_len = len;
        self
    }
    pub fn top_k(mut self, k: i32) -> Self {
        self.top_k = k;
        self
    }
    pub fn top_p(mut self, p: f32) -> Self {
        self.top_p = p;
        self
    }
    pub fn temperature(mut self, t: f32) -> Self {
        self.temperature = t;
        self
    }
    pub fn repeat_penalty(mut self, p: f32) -> Self {
        self.repeat_penalty = p;
        self
    }
    pub fn frequency_penalty(mut self, p: f32) -> Self {
        self.frequency_penalty = p;
        self
    }
    pub fn presence_penalty(mut self, p: f32) -> Self {
        self.presence_penalty = p;
        self
    }
    pub fn linefeed_id(mut self, id: i32) -> Self {
        self.linefeed_id = id;
        self
    }
    pub fn skip_special_token(mut self, skip: bool) -> Self {
        self.skip_special_token = skip;
        self
    }
    pub fn ignore_eos_token(mut self, ignore: bool) -> Self {
        self.ignore_eos_token = ignore;
        self
    }

    pub fn special_bos_id(mut self, ids: &[i32]) -> Self {
        let len = ids.len().min(64);
        self.special_bos_id[..len].copy_from_slice(&ids[..len]);
        self.n_special_bos_id = len as i32;
        self
    }

    pub fn special_eos_id(mut self, ids: &[i32]) -> Self {
        let len = ids.len().min(64);
        self.special_eos_id[..len].copy_from_slice(&ids[..len]);
        self.n_special_eos_id = len as i32;
        self
    }

    fn to_raw(&self) -> raw::rknn3_llm_param {
        raw::rknn3_llm_param {
            logits_name: self
                .logits_name
                .as_ref()
                .map_or(ptr::null_mut(), |s| s.as_ptr() as *mut _),
            max_context_len: self.max_context_len,
            sampling_param: raw::rknn3_sampling_params {
                top_k: self.top_k,
                top_p: self.top_p,
                temperature: self.temperature,
                repeat_penalty: self.repeat_penalty,
                frequency_penalty: self.frequency_penalty,
                presence_penalty: self.presence_penalty,
            },
            vocab_info: raw::rknn3_vocab_info {
                vocab_size: self.vocab_size,
                special_bos_id: self.special_bos_id,
                special_eos_id: self.special_eos_id,
                n_special_bos_id: self.n_special_bos_id,
                n_special_eos_id: self.n_special_eos_id,
                linefeed_id: self.linefeed_id,
                skip_special_token: self.skip_special_token,
                ignore_eos_token: self.ignore_eos_token,
                reserved: [0u8; 64],
            },
            extend_param: raw::rknn3_llm_extend_param {
                reserved: [0u8; 128],
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Inference Parameters
// ---------------------------------------------------------------------------

/// Safe wrapper for `rknn3_llm_infer_param`.
#[derive(Debug)]
pub struct InferParams {
    pub keep_history: bool,
    pub max_new_tokens: i32,
}

impl Default for InferParams {
    fn default() -> Self {
        Self {
            keep_history: false,
            max_new_tokens: 512,
        }
    }
}

impl InferParams {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn keep_history(mut self, keep: bool) -> Self {
        self.keep_history = keep;
        self
    }
    pub fn max_new_tokens(mut self, n: i32) -> Self {
        self.max_new_tokens = n;
        self
    }

    fn to_raw(&self) -> raw::rknn3_llm_infer_param {
        raw::rknn3_llm_infer_param {
            keep_history: self.keep_history as i32,
            max_new_tokens: self.max_new_tokens,
            reserved: [0u8; 128],
        }
    }
}

// ---------------------------------------------------------------------------
// LLM Input
// ---------------------------------------------------------------------------

/// Safe wrapper for `rknn3_llm_input` — input data for LLM inference.
pub struct LlmInput {
    role: Option<CString>,
    #[allow(dead_code)]
    prompt_cstr: Option<CString>, // held for lifetime
    owned_tokens: Vec<i32>,
    input_type: raw::rknn3_llm_input_type,
    tensor: raw::rknn3_llm_tensor,
}

// SAFETY: LlmInput owns all its data (CString, Vec<i32>). The raw pointers in
// `tensor` point into owned data and are only used by the C API during the
// `session.run()` call. The struct is safe to send across threads because all
// data is owned and immutable after construction.
unsafe impl Send for LlmInput {}

impl LlmInput {
    /// Create a prompt input.
    pub fn prompt(prompt: &str) -> Result<Self, Error> {
        let prompt_cstr = CString::new(prompt).map_err(|_| Error::nul_byte("prompt"))?;
        let prompt_ptr = prompt_cstr.as_ptr();
        Ok(Self {
            role: None,
            prompt_cstr: Some(prompt_cstr),
            owned_tokens: Vec::new(),
            input_type: raw::rknn3_llm_input_type_RKNN3_LLM_INPUT_PROMPT,
            tensor: raw::rknn3_llm_tensor {
                name: ptr::null(),
                prompt: prompt_ptr,
                embed: ptr::null_mut(),
                tokens: ptr::null_mut(),
                n_tokens: 0,
                enable_thinking: false,
            },
        })
    }

    /// Create a token input (owns the token data).
    pub fn tokens(tokens: Vec<i32>) -> Self {
        let n_tokens = tokens.len() as u64;
        Self {
            role: None,
            prompt_cstr: None,
            owned_tokens: tokens,
            input_type: raw::rknn3_llm_input_type_RKNN3_LLM_INPUT_TOKEN,
            tensor: raw::rknn3_llm_tensor {
                name: ptr::null(),
                prompt: ptr::null(),
                embed: ptr::null_mut(),
                tokens: ptr::null_mut(),
                n_tokens,
                enable_thinking: false,
            },
        }
    }

    /// Set the message role (e.g., "user", "tool").
    pub fn role(mut self, role: &str) -> Result<Self, Error> {
        self.role = Some(CString::new(role).map_err(|_| Error::nul_byte("role"))?);
        Ok(self)
    }

    /// Enable thinking mode.
    pub fn enable_thinking(mut self, enable: bool) -> Self {
        self.tensor.enable_thinking = enable;
        self
    }

    #[allow(clippy::wrong_self_convention)]
    fn to_raw(&mut self) -> raw::rknn3_llm_input {
        if self.input_type == raw::rknn3_llm_input_type_RKNN3_LLM_INPUT_TOKEN {
            self.tensor.tokens = self.owned_tokens.as_mut_ptr();
        }
        raw::rknn3_llm_input {
            role: self.role.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
            input_type: self.input_type,
            __bindgen_anon_1: raw::rknn3_llm_input__bindgen_ty_1 {
                llm_input: self.tensor,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// C callback trampolines (unsafe, internal only)
// ---------------------------------------------------------------------------

/// Heap-allocated callback holder, independent of Session.
///
/// This is stored separately from Session so that trampoline callbacks can
/// access the LlmCallbacks trait object without creating an aliasing `&mut Session`
/// while `session.run(&mut self)` is active on the Rust side.
struct CallbackBox {
    callbacks: Option<Box<dyn LlmCallbacks>>,
}

unsafe extern "C" fn trampoline_tokenizer(
    userdata: *mut std::ffi::c_void,
    text: *const std::ffi::c_char,
    text_len: i32,
    tokens: *mut i32,
    n_tokens_max: i32,
) -> i32 {
    if userdata.is_null() || text.is_null() || tokens.is_null() {
        return -1;
    }
    // SAFETY: userdata was set to a valid CallbackBox pointer in set_callback().
    // C API guarantees no re-entrant calls to the same callback.
    unsafe {
        let cb_box = &*(userdata as *const CallbackBox);
        let cb = match &cb_box.callbacks {
            Some(cb) => cb,
            None => return -1,
        };
        let text_slice = std::slice::from_raw_parts(text as *const u8, text_len as usize);
        let text_str = match std::str::from_utf8(text_slice) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let tokens_buf = std::slice::from_raw_parts_mut(tokens, n_tokens_max as usize);
        match cb.tokenize(text_str, tokens_buf) {
            Ok(n) => n as i32,
            Err(_) => -1,
        }
    }
}

unsafe extern "C" fn trampoline_embed(
    userdata: *mut std::ffi::c_void,
    tokens: *mut i32,
    num_tokens: u64,
    embed: *mut std::ffi::c_void,
    len: u64,
) -> i32 {
    if userdata.is_null() || tokens.is_null() || embed.is_null() {
        return -1;
    }
    // SAFETY: userdata was set to a valid CallbackBox pointer in set_callback().
    unsafe {
        let cb_box = &*(userdata as *const CallbackBox);
        let cb = match &cb_box.callbacks {
            Some(cb) => cb,
            None => return -1,
        };
        let tokens_slice = std::slice::from_raw_parts(tokens, num_tokens as usize);
        let embed_buf = std::slice::from_raw_parts_mut(embed as *mut u8, len as usize);
        match cb.embed(tokens_slice, embed_buf) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }
}

unsafe extern "C" fn trampoline_result(
    userdata: *mut std::ffi::c_void,
    result: *mut raw::RKLLMResult,
    state: raw::LLMCallState,
) -> i32 {
    if userdata.is_null() {
        return 0;
    }
    // SAFETY: userdata was set to a valid CallbackBox pointer in set_callback().
    // C API guarantees no re-entrant calls to the same callback.
    // Using *mut to get &mut access to the Box<dyn LlmCallbacks> for on_result(&mut self).
    unsafe {
        let cb_box = &mut *(userdata as *mut CallbackBox);
        let cb = match &mut cb_box.callbacks {
            Some(cb) => cb,
            None => return 0,
        };
        if result.is_null() {
            cb.on_result(&[], state.into());
            return 0;
        }
        let r = &*result;
        let token_ids = if r.token_ids.is_null() || r.num_tokens <= 0 {
            &[]
        } else {
            std::slice::from_raw_parts(r.token_ids, r.num_tokens as usize)
        };
        cb.on_result(token_ids, state.into());
    }
    0
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

/// RAII wrapper around `rknn3_session` for LLM inference.
pub struct Session {
    session: *mut raw::rknn3_session,
    /// Callbacks are stored in a separate heap allocation so that
    /// C trampolines can access them via raw pointer without aliasing `&mut Session`.
    callback_box: Option<Box<CallbackBox>>,
    raw_callback: raw::RKLLMCallback,
}

impl Session {
    /// Create a new LLM session.
    pub fn new(ctx: &Context, params: &mut [LlmParams]) -> Result<Self, Error> {
        let raw_params: Vec<raw::rknn3_llm_param> = params.iter().map(|p| p.to_raw()).collect();
        let mut raw_params = raw_params;
        let session = unsafe {
            raw::rknn3_session_init(
                ctx.ctx_handle(),
                raw_params.as_mut_ptr(),
                raw_params.len() as i32,
            )
        };
        if session.is_null() {
            return Err(Error::NullHandle {
                context: "rknn3_session_init",
            });
        }
        Ok(Self {
            session,
            callback_box: None,
            raw_callback: unsafe { std::mem::zeroed() },
        })
    }

    /// Set callback handlers for the session.
    ///
    /// The `Box<dyn LlmCallbacks>` is owned by the session and dropped with it.
    /// Callbacks are stored in a separate `CallbackBox` so that C trampolines
    /// can access them without creating aliasing `&mut Session` references
    /// (which would be UB when `session.run(&mut self)` is active).
    pub fn set_callback(&mut self, callbacks: Box<dyn LlmCallbacks>) -> Result<(), Error> {
        self.raw_callback.result_callback = Some(trampoline_result);
        self.raw_callback.tokenizer_callback = Some(trampoline_tokenizer);
        self.raw_callback.embed_callback = Some(trampoline_embed);

        let cb_box = Box::new(CallbackBox {
            callbacks: Some(callbacks),
        });
        let cb_ptr = Box::into_raw(cb_box) as *mut std::ffi::c_void;
        self.raw_callback.result_userdata = cb_ptr;
        self.raw_callback.tokenizer_userdata = cb_ptr;
        self.raw_callback.embed_userdata = cb_ptr;

        // SAFETY: session is valid (from new()), raw_callback is a valid mutable reference.
        let ret = unsafe { raw::rknn3_session_set_callback(self.session, &mut self.raw_callback) };
        Error::check("rknn3_session_set_callback", ret)?;

        // Store the raw pointer so we can reconstruct and drop the Box in Drop.
        // SAFETY: cb_ptr was just created from Box::into_raw above and is valid.
        self.callback_box = Some(unsafe { Box::from_raw(cb_ptr as *mut CallbackBox) });

        Ok(())
    }

    /// Run synchronous LLM inference.
    pub fn run(&mut self, inputs: &mut [LlmInput], param: &mut InferParams) -> Result<(), Error> {
        let mut raw_param = param.to_raw();
        let raw_inputs: Vec<raw::rknn3_llm_input> = inputs.iter_mut().map(|i| i.to_raw()).collect();
        let mut raw_inputs = raw_inputs;
        // SAFETY: session is valid, inputs and param are valid mutable references.
        let ret = unsafe {
            raw::rknn3_session_run(
                self.session,
                raw_inputs.as_mut_ptr(),
                raw_inputs.len() as u32,
                &mut raw_param,
            )
        };
        Error::check("rknn3_session_run", ret)
    }

    /// Run asynchronous LLM inference.
    pub fn run_async(
        &mut self,
        inputs: &mut [LlmInput],
        param: &mut InferParams,
    ) -> Result<(), Error> {
        let mut raw_param = param.to_raw();
        let raw_inputs: Vec<raw::rknn3_llm_input> = inputs.iter_mut().map(|i| i.to_raw()).collect();
        let mut raw_inputs = raw_inputs;
        // SAFETY: session is valid, inputs and param are valid mutable references.
        let ret = unsafe {
            raw::rknn3_session_run_async(
                self.session,
                raw_inputs.as_mut_ptr(),
                raw_inputs.len() as u32,
                &mut raw_param,
            )
        };
        Error::check("rknn3_session_run_async", ret)
    }

    /// Stop an ongoing inference.
    pub fn stop(&self) -> Result<(), Error> {
        // SAFETY: session is valid.
        let ret = unsafe { raw::rknn3_session_stop(self.session) };
        Error::check("rknn3_session_stop", ret)
    }

    /// Create a [`StopHandle`] that can stop inference from another thread.
    ///
    /// The handle remains valid as long as this `Session` is alive.
    pub fn stop_handle(&self) -> StopHandle {
        StopHandle {
            session: self.session,
        }
    }

    /// Set chat template strings.
    pub fn set_chat_template(
        &self,
        system_prompt: Option<&str>,
        prompt_prefix: Option<&str>,
        prompt_postfix: Option<&str>,
    ) -> Result<(), Error> {
        let sp = system_prompt
            .map(CString::new)
            .transpose()
            .map_err(|_| Error::nul_byte("system_prompt"))?;
        let pp = prompt_prefix
            .map(CString::new)
            .transpose()
            .map_err(|_| Error::nul_byte("prompt_prefix"))?;
        let pf = prompt_postfix
            .map(CString::new)
            .transpose()
            .map_err(|_| Error::nul_byte("prompt_postfix"))?;
        // SAFETY: session is valid, all CStrings are valid (or null).
        let ret = unsafe {
            raw::rknn3_session_set_chat_template(
                self.session,
                sp.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
                pp.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
                pf.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
            )
        };
        Error::check("rknn3_session_set_chat_template", ret)
    }

    /// Update LLM parameters.
    pub fn set_llm_param(&self, params: &mut [LlmParams]) -> Result<(), Error> {
        let mut raw_params: Vec<raw::rknn3_llm_param> = params.iter().map(|p| p.to_raw()).collect();
        // SAFETY: session is valid, params is a valid mutable slice.
        let ret = unsafe {
            raw::rknn3_session_set_llm_param(
                self.session,
                raw_params.as_mut_ptr(),
                raw_params.len() as i32,
            )
        };
        Error::check("rknn3_session_set_llm_param", ret)
    }

    /// Set function tools for the session.
    pub fn set_function_tools(&self, tools_json: &str) -> Result<(), Error> {
        let cstr = CString::new(tools_json).map_err(|_| Error::nul_byte("function_tools_json"))?;
        // SAFETY: session is valid, tools_json is a valid CString.
        let ret = unsafe { raw::rknn3_session_set_function_tools(self.session, cstr.as_ptr()) };
        Error::check("rknn3_session_set_function_tools", ret)
    }

    /// Set KV cache policy.
    pub fn set_kvcache_policy(
        &self,
        policy: KvCachePolicy,
        param: &KvCachePolicyParam,
    ) -> Result<(), Error> {
        let raw_policy: raw::rknn3_kvcache_policy = policy.into();
        let mut raw_param = param.to_raw();
        // SAFETY: session is valid, policy and param are valid.
        let ret = unsafe {
            raw::rknn3_session_set_kvcache_policy(self.session, raw_policy, &mut raw_param)
        };
        Error::check("rknn3_session_set_kvcache_policy", ret)
    }

    /// Clear KV cache.
    pub fn clear_kvcache(&self, policy: KvCacheClearPolicy) -> Result<(), Error> {
        let raw_policy: raw::rknn3_kvcache_clear_policy = policy.into();
        // SAFETY: session is valid.
        let ret = unsafe { raw::rknn3_session_clear_kvcache(self.session, raw_policy) };
        Error::check("rknn3_session_clear_kvcache", ret)
    }

    /// Load KV cache from file.
    pub fn load_kvcache(&self, kvcache_path: &str) -> Result<(), Error> {
        let cstr = CString::new(kvcache_path).map_err(|_| Error::nul_byte("kvcache_path"))?;
        let path_len = kvcache_path.len() as i64;
        // SAFETY: session is valid, path is a valid CString, path_len is correct.
        let ret = unsafe { raw::rknn3_session_load_kvcache(self.session, cstr.as_ptr(), path_len) };
        Error::check("rknn3_session_load_kvcache", ret)
    }

    /// Save KV cache to file.
    pub fn save_kvcache(&self, kvcache_path: &str) -> Result<(), Error> {
        let cstr = CString::new(kvcache_path).map_err(|_| Error::nul_byte("kvcache_path"))?;
        // SAFETY: session is valid, path is a valid CString.
        let ret = unsafe {
            raw::rknn3_session_save_kvcache(
                self.session,
                cstr.as_ptr() as *mut std::os::raw::c_char,
            )
        };
        Error::check("rknn3_session_save_kvcache", ret)
    }

    /// Query session state.
    pub fn query_state(&self) -> Result<RunState, Error> {
        let mut state: raw::RKLLMRunState = unsafe { std::mem::zeroed() };
        // SAFETY: session is valid, state is a valid output pointer.
        let ret = unsafe { raw::rknn3_session_query_state(self.session, &mut state) };
        Error::check("rknn3_session_query_state", ret)?;
        Ok(RunState::from_raw(&state))
    }

    /// Enable a LoRA adapter.
    pub fn enable_lora(&self, lora: &Lora) -> Result<(), Error> {
        let mut raw_lora = lora.to_raw();
        // SAFETY: session is valid, lora is a valid mutable reference.
        let ret = unsafe { raw::rknn3_session_enable_lora(self.session, &mut raw_lora) };
        Error::check("rknn3_session_enable_lora", ret)
    }

    /// Disable a LoRA adapter.
    pub fn disable_lora(&self, lora: &Lora) -> Result<(), Error> {
        let mut raw_lora = lora.to_raw();
        // SAFETY: session is valid, lora is a valid mutable reference.
        let ret = unsafe { raw::rknn3_session_disable_lora(self.session, &mut raw_lora) };
        Error::check("rknn3_session_disable_lora", ret)
    }

    /// Query LoRA adapter info.
    pub fn query_lora(&self) -> Result<Vec<Lora>, Error> {
        let mut lora_ptr: *mut raw::rknn3_lora = ptr::null_mut();
        let mut n_lora: std::os::raw::c_int = 0;
        // SAFETY: session is valid, output pointers are valid.
        let ret =
            unsafe { raw::rknn3_session_query_lora(self.session, &mut lora_ptr, &mut n_lora) };
        Error::check("rknn3_session_query_lora", ret)?;
        if lora_ptr.is_null() || n_lora <= 0 {
            return Ok(Vec::new());
        }
        let count = n_lora as usize;
        // SAFETY: lora_ptr points to n_lora valid rknn3_lora elements.
        let loras = unsafe { std::slice::from_raw_parts(lora_ptr, count) }
            .iter()
            .map(Lora::from_raw)
            .collect();
        Ok(loras)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.session.is_null() {
            // SAFETY: session is valid (non-null) and owned by this instance.
            let ret = unsafe { raw::rknn3_session_destroy(self.session) };
            if ret != 0 {
                tracing::warn!("rknn3_session_destroy failed with code {}", ret);
            }
        }
    }
}

/// Handle to stop an ongoing inference from another thread.
///
/// Obtained via `Session::stop_handle()`. The underlying session pointer remains
/// valid as long as the `Session` that created this handle is alive.
pub struct StopHandle {
    session: *mut raw::rknn3_session,
}

// SAFETY: rknn3_session_stop is thread-safe by API contract.
unsafe impl Send for StopHandle {}
unsafe impl Sync for StopHandle {}

impl StopHandle {
    /// Stop the ongoing inference.
    pub fn stop(&self) -> Result<(), Error> {
        // SAFETY: session pointer is valid as long as the originating Session is alive.
        let ret = unsafe { raw::rknn3_session_stop(self.session) };
        Error::check("rknn3_session_stop", ret)
    }
}

// Session 不是 Sync — 单个 Session 不能在多线程间共享。
// SAFETY: rknn3_session 是不透明指针，可以在线程间移动（Send）。
unsafe impl Send for Session {}

// ---------------------------------------------------------------------------
// ModelConfig
// ---------------------------------------------------------------------------

/// Configuration for `rknn3_model_init`.
pub struct ModelConfig {
    pub priority: i32,
    pub run_timeout: u32,
    pub core_mask: u32,
    pub user_mem_weight: bool,
    pub user_mem_internal: bool,
    pub user_sram: bool,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            priority: 0,
            run_timeout: 0,
            core_mask: raw::_rknn3_core_mask_RKNN3_NPU_CORE_AUTO,
            user_mem_weight: false,
            user_mem_internal: false,
            user_sram: false,
        }
    }
}

impl ModelConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn run_timeout(mut self, timeout_ms: u32) -> Self {
        self.run_timeout = timeout_ms;
        self
    }

    pub fn core_mask(mut self, mask: u32) -> Self {
        self.core_mask = mask;
        self
    }

    pub fn user_mem_weight(mut self, enabled: bool) -> Self {
        self.user_mem_weight = enabled;
        self
    }

    pub fn user_mem_internal(mut self, enabled: bool) -> Self {
        self.user_mem_internal = enabled;
        self
    }

    pub fn user_sram(mut self, enabled: bool) -> Self {
        self.user_sram = enabled;
        self
    }

    fn to_ffi(&self) -> raw::rknn3_config {
        raw::rknn3_config {
            priority: self.priority,
            run_timeout: self.run_timeout,
            run_core_mask: self.core_mask,
            user_mem_weight: self.user_mem_weight as u8,
            user_mem_internal: self.user_mem_internal as u8,
            user_sram: self.user_sram as u8,
            reserved: [0u8; 128],
        }
    }
}

// ---------------------------------------------------------------------------
// Image memory helpers
// ---------------------------------------------------------------------------

/// Safe wrapper for image memory.
///
/// Wraps `rknn3_im_mem` (`_rknn3_im_mem`). Created via [`im_mem_create`].
pub struct ImageMem {
    /// Underlying raw image memory.
    raw: raw::rknn3_im_mem,
}

impl ImageMem {
    /// Width of the image buffer.
    pub fn width(&self) -> i32 {
        self.raw.width
    }
    /// Height of the image buffer.
    pub fn height(&self) -> i32 {
        self.raw.height
    }
    /// Stride of the image buffer.
    pub fn stride(&self) -> i32 {
        self.raw.stride
    }
    /// Whether sync to host is enabled.
    pub fn sync_to_host(&self) -> bool {
        self.raw.sync_to_host
    }
    /// Return mutable reference to the underlying raw image memory.
    pub(crate) fn as_mut_raw(&mut self) -> &mut raw::rknn3_im_mem {
        &mut self.raw
    }
}

/// Create image memory for processing.
pub fn im_mem_create(
    ctx: &Context,
    width: i32,
    height: i32,
    fmt: ImageFormat,
    size: i32,
    core_id: i32,
    flags: MemAllocFlags,
) -> Result<ImageMem, Error> {
    let raw_fmt: raw::rknn3_im_fmt = fmt.into();
    let raw_flags: raw::rknn3_mem_alloc_flags = flags.into();
    let mut im_mem: raw::rknn3_im_mem = unsafe { std::mem::zeroed() };
    // SAFETY: ctx is valid, im_mem is a valid output pointer.
    let ret = unsafe {
        raw::rknn3_im_mem_create(
            ctx.ctx_handle(),
            width,
            height,
            raw_fmt,
            size,
            core_id,
            raw_flags,
            &mut im_mem,
        )
    };
    Error::check("rknn3_im_mem_create", ret)?;
    Ok(ImageMem { raw: im_mem })
}

/// Destroy image memory.
pub fn im_mem_destroy(ctx: &Context, im_mem: &mut ImageMem) -> Result<(), Error> {
    // SAFETY: ctx and im_mem are valid.
    let ret = unsafe { raw::rknn3_im_mem_destroy(ctx.ctx_handle(), im_mem.as_mut_raw()) };
    Error::check("rknn3_im_mem_destroy", ret)
}

/// Convert image color space.
pub fn im_cvt_color(ctx: &Context, src: &mut ImageMem, dst: &mut ImageMem) -> Result<(), Error> {
    // SAFETY: ctx, src, and dst are valid.
    let ret =
        unsafe { raw::rknn3_im_cvt_color(ctx.ctx_handle(), src.as_mut_raw(), dst.as_mut_raw()) };
    Error::check("rknn3_im_cvt_color", ret)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn to_cstr(path: &Path, context: &'static str) -> Result<CString, Error> {
    let s = path
        .to_str()
        .ok_or_else(|| Error::invalid_config(context, "path contains invalid UTF-8"))?;
    CString::new(s).map_err(|_| Error::nul_byte(context))
}
