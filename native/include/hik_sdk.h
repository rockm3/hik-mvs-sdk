#ifndef HIK_SDK_H
#define HIK_SDK_H

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32) && defined(HIK_SDK_BUILD_DLL)
#define HIK_API __declspec(dllexport)
#elif defined(_WIN32) && defined(HIK_SDK_USE_DLL)
#define HIK_API __declspec(dllimport)
#elif defined(__GNUC__) || defined(__clang__)
#define HIK_API __attribute__((visibility("default")))
#else
#define HIK_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

enum {
    HIK_OK = 0,
    HIK_ERROR_INVALID_ARGUMENT = -1,
    HIK_ERROR_NOT_FOUND = -2,
    HIK_ERROR_OUT_OF_MEMORY = -3,
    HIK_ERROR_UNSUPPORTED_FORMAT = -4,
    HIK_ERROR_EXCEPTION = -5
};

typedef enum hik_transport {
    HIK_TRANSPORT_UNKNOWN = 0,
    HIK_TRANSPORT_GIGE = 1,
    HIK_TRANSPORT_USB3 = 2
} hik_transport_t;

typedef enum hik_output_format {
    HIK_OUTPUT_RAW = 0,
    HIK_OUTPUT_MONO8 = 1,
    HIK_OUTPUT_BGR8 = 2
} hik_output_format_t;

typedef enum hik_image_format {
    HIK_IMAGE_BMP = 1,
    HIK_IMAGE_JPEG = 2,
    HIK_IMAGE_PNG = 3,
    HIK_IMAGE_TIFF = 4
} hik_image_format_t;

typedef struct hik_camera hik_camera_t;

typedef struct hik_device_info {
    uint32_t transport;
    uint32_t ip;
    char model[64];
    char serial[64];
    char user_name[64];
} hik_device_info_t;

typedef struct hik_frame {
    uint32_t width;
    uint32_t height;
    uint32_t stride;
    uint32_t pixel_format;
    uint64_t frame_number;
    uint64_t timestamp;
    float gain;
    float exposure_time;
    uint32_t average_brightness;
    uint32_t lost_packets;
    uint32_t chunk_width;
    uint32_t chunk_height;
    uint32_t unparsed_chunk_count;
    uint8_t* data;
    size_t data_len;
} hik_frame_t;

typedef void (*hik_frame_callback_t)(const hik_frame_t* frame, void* user);
typedef void (*hik_exception_callback_t)(uint32_t message_type, void* user);

typedef struct hik_event {
    char name[128];
    uint16_t event_id;
    uint16_t stream_channel;
    uint64_t block_id;
    uint64_t timestamp;
} hik_event_t;
typedef void (*hik_event_callback_t)(const hik_event_t* event, void* user);

typedef struct hik_network_stats {
    int64_t received_bytes;
    int64_t lost_packets;
    uint32_t lost_frames;
    uint32_t received_frames;
    int64_t requested_resend_packets;
    int64_t resent_packets;
} hik_network_stats_t;

typedef struct hik_action_result {
    char device_address[16];
    int32_t status;
} hik_action_result_t;

typedef struct hik_int_value {
    int64_t current;
    int64_t minimum;
    int64_t maximum;
    int64_t increment;
} hik_int_value_t;

typedef struct hik_float_value {
    float current;
    float minimum;
    float maximum;
} hik_float_value_t;

HIK_API int32_t hik_initialize(void);
HIK_API int32_t hik_finalize(void);
HIK_API uint32_t hik_sdk_version(void);
HIK_API int32_t hik_enumerate(hik_device_info_t* devices, size_t capacity, size_t* count);
HIK_API int32_t hik_action_command(uint32_t device_key, uint32_t group_key, uint32_t group_mask,
                                   const char* broadcast_address, uint32_t timeout_ms,
                                   hik_action_result_t* results, size_t capacity, size_t* count);
HIK_API int32_t hik_camera_open(const char* serial, hik_camera_t** camera);
HIK_API int32_t hik_camera_start(hik_camera_t* camera);
HIK_API int32_t hik_camera_stop(hik_camera_t* camera);
HIK_API int32_t hik_camera_grab(hik_camera_t* camera, uint32_t timeout_ms,
                                hik_output_format_t output, hik_frame_t* frame);
HIK_API void hik_frame_release(hik_frame_t* frame);
HIK_API int32_t hik_camera_set_float(hik_camera_t* camera, const char* name, float value);
HIK_API int32_t hik_camera_get_float(hik_camera_t* camera, const char* name, hik_float_value_t* value);
HIK_API int32_t hik_camera_set_int(hik_camera_t* camera, const char* name, int64_t value);
HIK_API int32_t hik_camera_get_int(hik_camera_t* camera, const char* name, hik_int_value_t* value);
HIK_API int32_t hik_camera_set_enum(hik_camera_t* camera, const char* name, const char* value);
HIK_API int32_t hik_camera_get_enum(hik_camera_t* camera, const char* name, char* value,
                                    size_t capacity, size_t* required);
HIK_API int32_t hik_camera_set_bool(hik_camera_t* camera, const char* name, uint8_t value);
HIK_API int32_t hik_camera_get_bool(hik_camera_t* camera, const char* name, uint8_t* value);
HIK_API int32_t hik_camera_set_string(hik_camera_t* camera, const char* name, const char* value);
HIK_API int32_t hik_camera_get_string(hik_camera_t* camera, const char* name, char* value,
                                      size_t capacity, size_t* required);
HIK_API int32_t hik_camera_is_connected(hik_camera_t* camera, uint8_t* connected);
HIK_API int32_t hik_camera_set_image_node_count(hik_camera_t* camera, uint32_t count);
HIK_API int32_t hik_camera_register_frame_callback(hik_camera_t* camera,
                                                   hik_frame_callback_t callback, void* user);
HIK_API int32_t hik_camera_register_exception_callback(hik_camera_t* camera,
                                                       hik_exception_callback_t callback, void* user);
HIK_API int32_t hik_camera_register_event_callback(hik_camera_t* camera,
                                                   hik_event_callback_t callback, void* user);
HIK_API int32_t hik_camera_network_stats(hik_camera_t* camera, hik_network_stats_t* stats);
HIK_API int32_t hik_camera_save_image(hik_camera_t* camera, const hik_frame_t* frame,
                                      hik_image_format_t format, uint32_t quality, const char* path);
HIK_API int32_t hik_camera_record_start(hik_camera_t* camera, const hik_frame_t* first_frame,
                                        float frame_rate, uint32_t bit_rate_kbps, const char* path);
HIK_API int32_t hik_camera_record_input(hik_camera_t* camera, const hik_frame_t* frame);
HIK_API int32_t hik_camera_record_stop(hik_camera_t* camera);
HIK_API int32_t hik_camera_rotate(hik_camera_t* camera, const hik_frame_t* source,
                                  uint32_t angle, hik_frame_t* output);
HIK_API int32_t hik_camera_flip(hik_camera_t* camera, const hik_frame_t* source,
                                uint32_t direction, hik_frame_t* output);
HIK_API int32_t hik_camera_feature_save(hik_camera_t* camera, const char* path);
HIK_API int32_t hik_camera_feature_load(hik_camera_t* camera, const char* path);
HIK_API int32_t hik_force_ip(const char* serial, uint32_t ip, uint32_t subnet, uint32_t gateway);
HIK_API int32_t hik_camera_command(hik_camera_t* camera, const char* name);
HIK_API void hik_camera_close(hik_camera_t* camera);
HIK_API const char* hik_last_error(void);

#ifdef __cplusplus
}
#endif
#endif
