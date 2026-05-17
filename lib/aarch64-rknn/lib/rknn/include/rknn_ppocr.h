#ifndef RKNN_PPOCR_H
#define RKNN_PPOCR_H

#ifdef __cplusplus
extern "C" {
#endif

#ifdef _WIN32
    #ifdef RKNN_PPOCR_EXPORTS
        #define RKNN_PPOCR_API __declspec(dllimport)
    #else
        #define RKNN_PPOCR_API __declspec(dllimport)
    #endif
#else
    #define RKNN_PPOCR_API __attribute__((visibility("default")))
#endif

// 句柄类型
typedef void* rknn_ppocr_handle_t;

// 文本识别结果
typedef struct {
    float left_top_x;     // 左上角x坐标
    float left_top_y;     // 左上角y坐标
    float right_top_x;    // 右上角x坐标
    float right_top_y;    // 右上角y坐标
    float right_bottom_x; // 右下角x坐标
    float right_bottom_y; // 右下角y坐标
    float left_bottom_x;  // 左下角x坐标
    float left_bottom_y;  // 左下角y坐标
    float score;          // 置信度
    char text[512];       // 识别的文本
    float text_score;     // 文本置信度
} rknn_ppocr_text_result_t;

// 检测参数
typedef struct {
    float threshold;        // 检测阈值
    float box_threshold;    // 框阈值
    int use_dilate;         // 是否使用膨胀
    char* db_score_mode;    // DB分数模式
    char* db_box_type;      // DB框类型
    float db_unclip_ratio;  // DB解压缩比例
} rknn_ppocr_det_params_t;

// 初始化参数
typedef struct {
    const char* det_model_path;    // 检测模型路径
    const char* rec_model_path;    // 识别模型路径
    rknn_ppocr_det_params_t det_params; // 检测参数
} rknn_ppocr_init_params_t;

/**
 * @brief 初始化PPOCR引擎
 * 
 * @param params 初始化参数
 * @return rkppocr_handle_t 成功返回句柄，失败返回NULL
 */
RKNN_PPOCR_API rknn_ppocr_handle_t rknn_ppocr_init(const rknn_ppocr_init_params_t* params);

/**
 * @brief 执行OCR推理
 * 
 * @param handle PPOCR句柄
 * @param image_data 图像数据 (RGB格式)
 * @param width 图像宽度
 * @param height 图像高度
 * @param results 输出结果数组指针，函数会动态分配内存
 * @return int 成功返回结果数量，失败返回-1
 */
RKNN_PPOCR_API int rknn_ppocr_inference(rknn_ppocr_handle_t handle, 
                                  const unsigned char* image_data, 
                                  int width, int height,
                                  rknn_ppocr_text_result_t** results);

/**
 * @brief 释放推理结果内存
 * 
 * @param results 推理结果数组
 */
RKNN_PPOCR_API void rknn_ppocr_free_results(rknn_ppocr_text_result_t* results);

/**
 * @brief 复制PPOCR上下文
 * 
 * @param handle_in 输入PPOCR句柄
 * @param handle_out 输出PPOCR句柄指针
 * @return int 成功返回0，失败返回-1
 */
RKNN_PPOCR_API int rknn_ppocr_dup_context(rknn_ppocr_handle_t handle_in, rknn_ppocr_handle_t* handle_out);

/**
 * @brief 释放PPOCR引擎
 * 
 * @param handle PPOCR句柄
 */
RKNN_PPOCR_API void rknn_ppocr_destroy(rknn_ppocr_handle_t handle);

#ifdef __cplusplus
}
#endif

#endif // RKNN_PPOCR_H