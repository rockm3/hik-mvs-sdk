#include "hik_sdk.h"
#include "MvCameraControl.h"

#include <algorithm>
#include <cstdio>
#include <cstring>
#include <memory>
#include <new>
#include <string>

struct hik_camera {
    void* handle = nullptr;
    bool grabbing = false;
    bool recording = false;
    hik_frame_callback_t frame_callback = nullptr;
    void* frame_user = nullptr;
    hik_exception_callback_t exception_callback = nullptr;
    void* exception_user = nullptr;
    hik_event_callback_t event_callback = nullptr;
    void* event_user = nullptr;
};

namespace {
thread_local std::string g_last_error;

int fail(int code, const char* operation, int sdk_code = 0) {
    char text[160]{};
    if (sdk_code != 0) {
        std::snprintf(text, sizeof(text), "%s failed: MVS error 0x%08X", operation,
                      static_cast<unsigned int>(sdk_code));
    } else {
        std::snprintf(text, sizeof(text), "%s", operation);
    }
    g_last_error = text;
    return code;
}

template <size_t N>
void copy_text(char (&dst)[N], const unsigned char* src, size_t src_size) {
    const auto length = std::min(N - 1, strnlen(reinterpret_cast<const char*>(src), src_size));
    std::memcpy(dst, src, length);
    dst[length] = '\0';
}

void fill_device(const MV_CC_DEVICE_INFO* src, hik_device_info_t* dst) {
    std::memset(dst, 0, sizeof(*dst));
    if (src->nTLayerType == MV_GIGE_DEVICE) {
        const auto& info = src->SpecialInfo.stGigEInfo;
        dst->transport = HIK_TRANSPORT_GIGE;
        dst->ip = info.nCurrentIp;
        copy_text(dst->model, info.chModelName, sizeof(info.chModelName));
        copy_text(dst->serial, info.chSerialNumber, sizeof(info.chSerialNumber));
        copy_text(dst->user_name, info.chUserDefinedName, sizeof(info.chUserDefinedName));
    } else if (src->nTLayerType == MV_USB_DEVICE) {
        const auto& info = src->SpecialInfo.stUsb3VInfo;
        dst->transport = HIK_TRANSPORT_USB3;
        copy_text(dst->model, info.chModelName, sizeof(info.chModelName));
        copy_text(dst->serial, info.chSerialNumber, sizeof(info.chSerialNumber));
        copy_text(dst->user_name, info.chUserDefinedName, sizeof(info.chUserDefinedName));
    }
}

std::string serial_of(const MV_CC_DEVICE_INFO* src) {
    if (src->nTLayerType == MV_GIGE_DEVICE)
        return reinterpret_cast<const char*>(src->SpecialInfo.stGigEInfo.chSerialNumber);
    if (src->nTLayerType == MV_USB_DEVICE)
        return reinterpret_cast<const char*>(src->SpecialInfo.stUsb3VInfo.chSerialNumber);
    return {};
}

int enumerate_sdk(MV_CC_DEVICE_INFO_LIST& list) {
    std::memset(&list, 0, sizeof(list));
    return MV_CC_EnumDevices(MV_GIGE_DEVICE | MV_USB_DEVICE, &list);
}

void __stdcall native_frame_callback(unsigned char* data, MV_FRAME_OUT_INFO_EX* info, void* user) {
    auto* camera = static_cast<hik_camera*>(user);
    if (!camera || !camera->frame_callback || !data || !info) return;
    hik_frame_t frame{};
    frame.width = info->nWidth;
    frame.height = info->nHeight;
    frame.stride = info->nHeight ? info->nFrameLen / info->nHeight : 0;
    frame.pixel_format = static_cast<uint32_t>(info->enPixelType);
    frame.frame_number = info->nFrameNum;
    frame.timestamp = (static_cast<uint64_t>(info->nDevTimeStampHigh) << 32) | info->nDevTimeStampLow;
    frame.gain = info->fGain;
    frame.exposure_time = info->fExposureTime;
    frame.average_brightness = info->nAverageBrightness;
    frame.lost_packets = info->nLostPacket;
    frame.chunk_width = info->nChunkWidth;
    frame.chunk_height = info->nChunkHeight;
    frame.unparsed_chunk_count = info->nUnparsedChunkNum;
    frame.data = data;
    frame.data_len = info->nFrameLen;
    camera->frame_callback(&frame, camera->frame_user);
}

void __stdcall native_exception_callback(unsigned int message_type, void* user) {
    auto* camera = static_cast<hik_camera*>(user);
    if (camera && camera->exception_callback)
        camera->exception_callback(message_type, camera->exception_user);
}

void __stdcall native_event_callback(MV_EVENT_OUT_INFO* info, void* user) {
    auto* camera = static_cast<hik_camera*>(user);
    if (!camera || !camera->event_callback || !info) return;
    hik_event_t event{};
    const size_t length = std::min(sizeof(event.name) - 1,
                                   strnlen(info->EventName, sizeof(info->EventName)));
    std::memcpy(event.name, info->EventName, length);
    event.name[length] = '\0';
    event.event_id = info->nEventID;
    event.stream_channel = info->nStreamChannel;
    event.block_id = (static_cast<uint64_t>(info->nBlockIdHigh) << 32) | info->nBlockIdLow;
    event.timestamp = (static_cast<uint64_t>(info->nTimestampHigh) << 32) | info->nTimestampLow;
    camera->event_callback(&event, camera->event_user);
}
}  // namespace

extern "C" {
int32_t hik_initialize(void) {
    const int rc = MV_CC_Initialize();
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_Initialize", rc);
}

int32_t hik_finalize(void) {
    const int rc = MV_CC_Finalize();
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_Finalize", rc);
}

uint32_t hik_sdk_version(void) { return MV_CC_GetSDKVersion(); }

int32_t hik_enumerate(hik_device_info_t* devices, size_t capacity, size_t* count) {
    if (!count) return fail(HIK_ERROR_INVALID_ARGUMENT, "count is null");
    MV_CC_DEVICE_INFO_LIST list{};
    const int rc = enumerate_sdk(list);
    if (rc != MV_OK) return fail(rc, "MV_CC_EnumDevices", rc);
    *count = list.nDeviceNum;
    if (!devices || capacity == 0) return HIK_OK;
    for (size_t i = 0; i < std::min(capacity, static_cast<size_t>(list.nDeviceNum)); ++i)
        fill_device(list.pDeviceInfo[i], &devices[i]);
    return HIK_OK;
}

int32_t hik_action_command(uint32_t device_key, uint32_t group_key, uint32_t group_mask,
                           const char* broadcast_address, uint32_t timeout_ms,
                           hik_action_result_t* results, size_t capacity, size_t* count) {
    if (!broadcast_address || !count)
        return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid action command argument");
    MV_ACTION_CMD_INFO info{};
    info.nDeviceKey = device_key;
    info.nGroupKey = group_key;
    info.nGroupMask = group_mask;
    info.pBroadcastAddress = broadcast_address;
    info.nTimeOut = timeout_ms;
    MV_ACTION_CMD_RESULT_LIST native{};
    const int rc = MV_GIGE_IssueActionCommand(&info, &native);
    if (rc != MV_OK) return fail(rc, "MV_GIGE_IssueActionCommand", rc);
    *count = native.nNumResults;
    if (results) {
        for (size_t i = 0; i < std::min(capacity, static_cast<size_t>(native.nNumResults)); ++i) {
            std::memset(&results[i], 0, sizeof(results[i]));
            const size_t length = std::min(sizeof(results[i].device_address) - 1,
                strnlen(reinterpret_cast<const char*>(native.pResults[i].strDeviceAddress),
                        sizeof(native.pResults[i].strDeviceAddress)));
            std::memcpy(results[i].device_address, native.pResults[i].strDeviceAddress, length);
            results[i].status = native.pResults[i].nStatus;
        }
    }
    return HIK_OK;
}

int32_t hik_camera_open(const char* serial, hik_camera_t** output) {
    if (!serial || !output) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid open argument");
    *output = nullptr;
    MV_CC_DEVICE_INFO_LIST list{};
    int rc = enumerate_sdk(list);
    if (rc != MV_OK) return fail(rc, "MV_CC_EnumDevices", rc);
    MV_CC_DEVICE_INFO* selected = nullptr;
    for (unsigned int i = 0; i < list.nDeviceNum; ++i) {
        if (serial_of(list.pDeviceInfo[i]) == serial) {
            selected = list.pDeviceInfo[i];
            break;
        }
    }
    if (!selected) return fail(HIK_ERROR_NOT_FOUND, "camera serial not found");

    auto camera = std::unique_ptr<hik_camera>(new (std::nothrow) hik_camera{});
    if (!camera) return fail(HIK_ERROR_OUT_OF_MEMORY, "camera allocation failed");
    rc = MV_CC_CreateHandle(&camera->handle, selected);
    if (rc != MV_OK) return fail(rc, "MV_CC_CreateHandle", rc);
    rc = MV_CC_OpenDevice(camera->handle);
    if (rc != MV_OK) {
        MV_CC_DestroyHandle(camera->handle);
        return fail(rc, "MV_CC_OpenDevice", rc);
    }
    if (selected->nTLayerType == MV_GIGE_DEVICE) {
        const int packet = MV_CC_GetOptimalPacketSize(camera->handle);
        if (packet > 0) MV_CC_SetIntValueEx(camera->handle, "GevSCPSPacketSize", packet);
    }
    *output = camera.release();
    return HIK_OK;
}

int32_t hik_camera_start(hik_camera_t* camera) {
    if (!camera) return fail(HIK_ERROR_INVALID_ARGUMENT, "camera is null");
    const int rc = MV_CC_StartGrabbing(camera->handle);
    if (rc == MV_OK) camera->grabbing = true;
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_StartGrabbing", rc);
}

int32_t hik_camera_stop(hik_camera_t* camera) {
    if (!camera) return fail(HIK_ERROR_INVALID_ARGUMENT, "camera is null");
    if (!camera->grabbing) return HIK_OK;
    const int rc = MV_CC_StopGrabbing(camera->handle);
    if (rc == MV_OK) camera->grabbing = false;
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_StopGrabbing", rc);
}

int32_t hik_camera_grab(hik_camera_t* camera, uint32_t timeout_ms,
                        hik_output_format_t output, hik_frame_t* frame) {
    if (!camera || !frame) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid grab argument");
    std::memset(frame, 0, sizeof(*frame));
    MV_FRAME_OUT source{};
    int rc = MV_CC_GetImageBuffer(camera->handle, &source, timeout_ms);
    if (rc != MV_OK) return fail(rc, "MV_CC_GetImageBuffer", rc);

    const auto release = [&] { MV_CC_FreeImageBuffer(camera->handle, &source); };
    uint32_t stride = 0;
    size_t length = 0;
    enum MvGvspPixelType target = source.stFrameInfo.enPixelType;
    if (output == HIK_OUTPUT_RAW) {
        length = source.stFrameInfo.nFrameLen;
        stride = source.stFrameInfo.nHeight ? static_cast<uint32_t>(length / source.stFrameInfo.nHeight) : 0;
    } else {
        target = output == HIK_OUTPUT_MONO8 ? PixelType_Gvsp_Mono8 : PixelType_Gvsp_BGR8_Packed;
        stride = source.stFrameInfo.nWidth * (output == HIK_OUTPUT_MONO8 ? 1u : 3u);
        length = static_cast<size_t>(stride) * source.stFrameInfo.nHeight;
    }

    auto data = new (std::nothrow) uint8_t[length];
    if (!data) {
        release();
        return fail(HIK_ERROR_OUT_OF_MEMORY, "frame allocation failed");
    }
    if (output == HIK_OUTPUT_RAW) {
        std::memcpy(data, source.pBufAddr, length);
    } else {
        MV_CC_PIXEL_CONVERT_PARAM_EX convert{};
        convert.nWidth = source.stFrameInfo.nWidth;
        convert.nHeight = source.stFrameInfo.nHeight;
        convert.enSrcPixelType = source.stFrameInfo.enPixelType;
        convert.pSrcData = source.pBufAddr;
        convert.nSrcDataLen = source.stFrameInfo.nFrameLen;
        convert.enDstPixelType = target;
        convert.pDstBuffer = data;
        convert.nDstBufferSize = static_cast<unsigned int>(length);
        rc = MV_CC_ConvertPixelTypeEx(camera->handle, &convert);
        if (rc != MV_OK) {
            delete[] data;
            release();
            return fail(rc, "MV_CC_ConvertPixelTypeEx", rc);
        }
        length = convert.nDstLen;
    }
    frame->width = source.stFrameInfo.nWidth;
    frame->height = source.stFrameInfo.nHeight;
    frame->stride = stride;
    frame->pixel_format = static_cast<uint32_t>(target);
    frame->frame_number = source.stFrameInfo.nFrameNum;
    frame->timestamp = (static_cast<uint64_t>(source.stFrameInfo.nDevTimeStampHigh) << 32) |
                       source.stFrameInfo.nDevTimeStampLow;
    frame->gain = source.stFrameInfo.fGain;
    frame->exposure_time = source.stFrameInfo.fExposureTime;
    frame->average_brightness = source.stFrameInfo.nAverageBrightness;
    frame->lost_packets = source.stFrameInfo.nLostPacket;
    frame->chunk_width = source.stFrameInfo.nChunkWidth;
    frame->chunk_height = source.stFrameInfo.nChunkHeight;
    frame->unparsed_chunk_count = source.stFrameInfo.nUnparsedChunkNum;
    frame->data = data;
    frame->data_len = length;
    release();
    return HIK_OK;
}

void hik_frame_release(hik_frame_t* frame) {
    if (!frame) return;
    delete[] frame->data;
    std::memset(frame, 0, sizeof(*frame));
}

int32_t hik_camera_set_float(hik_camera_t* camera, const char* name, float value) {
    if (!camera || !name) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid float parameter");
    const int rc = MV_CC_SetFloatValue(camera->handle, name, value);
    return rc == MV_OK ? HIK_OK : fail(rc, name, rc);
}

int32_t hik_camera_get_float(hik_camera_t* camera, const char* name, hik_float_value_t* value) {
    if (!camera || !name || !value) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid float argument");
    MVCC_FLOATVALUE native{};
    const int rc = MV_CC_GetFloatValue(camera->handle, name, &native);
    if (rc != MV_OK) return fail(rc, name, rc);
    value->current = native.fCurValue;
    value->minimum = native.fMin;
    value->maximum = native.fMax;
    return HIK_OK;
}

int32_t hik_camera_set_int(hik_camera_t* camera, const char* name, int64_t value) {
    if (!camera || !name) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid integer parameter");
    const int rc = MV_CC_SetIntValueEx(camera->handle, name, value);
    return rc == MV_OK ? HIK_OK : fail(rc, name, rc);
}

int32_t hik_camera_get_int(hik_camera_t* camera, const char* name, hik_int_value_t* value) {
    if (!camera || !name || !value) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid integer argument");
    MVCC_INTVALUE_EX native{};
    const int rc = MV_CC_GetIntValueEx(camera->handle, name, &native);
    if (rc != MV_OK) return fail(rc, name, rc);
    value->current = native.nCurValue;
    value->minimum = native.nMin;
    value->maximum = native.nMax;
    value->increment = native.nInc;
    return HIK_OK;
}

int32_t hik_camera_set_enum(hik_camera_t* camera, const char* name, const char* value) {
    if (!camera || !name || !value) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid enum parameter");
    const int rc = MV_CC_SetEnumValueByString(camera->handle, name, value);
    return rc == MV_OK ? HIK_OK : fail(rc, name, rc);
}

int32_t hik_camera_get_enum(hik_camera_t* camera, const char* name, char* value,
                            size_t capacity, size_t* required) {
    if (!camera || !name || !required) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid enum argument");
    MVCC_ENUMVALUE_EX current{};
    int rc = MV_CC_GetEnumValueEx(camera->handle, name, &current);
    if (rc != MV_OK) return fail(rc, name, rc);
    MVCC_ENUMENTRY entry{};
    entry.nValue = current.nCurValue;
    rc = MV_CC_GetEnumEntrySymbolic(camera->handle, name, &entry);
    if (rc != MV_OK) return fail(rc, name, rc);
    const size_t length = strnlen(entry.chSymbolic, sizeof(entry.chSymbolic));
    *required = length + 1;
    if (!value || capacity < length + 1) return HIK_OK;
    std::memcpy(value, entry.chSymbolic, length);
    value[length] = '\0';
    return HIK_OK;
}

int32_t hik_camera_set_bool(hik_camera_t* camera, const char* name, uint8_t value) {
    if (!camera || !name) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid boolean parameter");
    const int rc = MV_CC_SetBoolValue(camera->handle, name, value != 0);
    return rc == MV_OK ? HIK_OK : fail(rc, name, rc);
}

int32_t hik_camera_get_bool(hik_camera_t* camera, const char* name, uint8_t* value) {
    if (!camera || !name || !value) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid boolean argument");
    bool native = false;
    const int rc = MV_CC_GetBoolValue(camera->handle, name, &native);
    if (rc != MV_OK) return fail(rc, name, rc);
    *value = native ? 1 : 0;
    return HIK_OK;
}

int32_t hik_camera_set_string(hik_camera_t* camera, const char* name, const char* value) {
    if (!camera || !name || !value) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid string parameter");
    const int rc = MV_CC_SetStringValue(camera->handle, name, value);
    return rc == MV_OK ? HIK_OK : fail(rc, name, rc);
}

int32_t hik_camera_get_string(hik_camera_t* camera, const char* name, char* value,
                              size_t capacity, size_t* required) {
    if (!camera || !name || !required) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid string argument");
    MVCC_STRINGVALUE native{};
    const int rc = MV_CC_GetStringValue(camera->handle, name, &native);
    if (rc != MV_OK) return fail(rc, name, rc);
    const size_t length = strnlen(native.chCurValue, sizeof(native.chCurValue));
    *required = length + 1;
    if (!value || capacity < length + 1) return HIK_OK;
    std::memcpy(value, native.chCurValue, length);
    value[length] = '\0';
    return HIK_OK;
}

int32_t hik_camera_is_connected(hik_camera_t* camera, uint8_t* connected) {
    if (!camera || !connected) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid connected argument");
    *connected = MV_CC_IsDeviceConnected(camera->handle) ? 1 : 0;
    return HIK_OK;
}

int32_t hik_camera_set_image_node_count(hik_camera_t* camera, uint32_t count) {
    if (!camera || count == 0) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid image node count");
    const int rc = MV_CC_SetImageNodeNum(camera->handle, count);
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_SetImageNodeNum", rc);
}

int32_t hik_camera_register_frame_callback(hik_camera_t* camera,
                                           hik_frame_callback_t callback, void* user) {
    if (!camera || !callback) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid frame callback");
    camera->frame_callback = callback;
    camera->frame_user = user;
    const int rc = MV_CC_RegisterImageCallBackEx(camera->handle, native_frame_callback, camera);
    if (rc != MV_OK) { camera->frame_callback = nullptr; camera->frame_user = nullptr; }
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_RegisterImageCallBackEx", rc);
}

int32_t hik_camera_register_exception_callback(hik_camera_t* camera,
                                               hik_exception_callback_t callback, void* user) {
    if (!camera || !callback) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid exception callback");
    camera->exception_callback = callback;
    camera->exception_user = user;
    const int rc = MV_CC_RegisterExceptionCallBack(camera->handle, native_exception_callback, camera);
    if (rc != MV_OK) { camera->exception_callback = nullptr; camera->exception_user = nullptr; }
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_RegisterExceptionCallBack", rc);
}

int32_t hik_camera_register_event_callback(hik_camera_t* camera,
                                           hik_event_callback_t callback, void* user) {
    if (!camera || !callback) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid event callback");
    camera->event_callback = callback;
    camera->event_user = user;
    const int rc = MV_CC_RegisterAllEventCallBack(camera->handle, native_event_callback, camera);
    if (rc != MV_OK) { camera->event_callback = nullptr; camera->event_user = nullptr; }
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_RegisterAllEventCallBack", rc);
}

int32_t hik_camera_network_stats(hik_camera_t* camera, hik_network_stats_t* stats) {
    if (!camera || !stats) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid network stats argument");
    MV_MATCH_INFO_NET_DETECT native{};
    MV_ALL_MATCH_INFO match{};
    match.nType = MV_MATCH_TYPE_NET_DETECT;
    match.pInfo = &native;
    match.nInfoSize = sizeof(native);
    const int rc = MV_CC_GetAllMatchInfo(camera->handle, &match);
    if (rc != MV_OK) return fail(rc, "MV_CC_GetAllMatchInfo", rc);
    stats->received_bytes = native.nReceiveDataSize;
    stats->lost_packets = native.nLostPacketCount;
    stats->lost_frames = native.nLostFrameCount;
    stats->received_frames = native.nNetRecvFrameCount;
    stats->requested_resend_packets = native.nRequestResendPacketCount;
    stats->resent_packets = native.nResendPacketCount;
    return HIK_OK;
}

int32_t hik_camera_save_image(hik_camera_t* camera, const hik_frame_t* frame,
                              hik_image_format_t format, uint32_t quality, const char* path) {
    if (!camera || !frame || !frame->data || !path)
        return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid save image argument");
    MV_CC_IMAGE image{};
    image.nWidth = frame->width;
    image.nHeight = frame->height;
    image.enPixelType = static_cast<MvGvspPixelType>(frame->pixel_format);
    image.pImageBuf = frame->data;
    image.nImageBufSize = frame->data_len;
    image.nImageLen = frame->data_len;
    MV_CC_SAVE_IMAGE_PARAM save{};
    save.enImageType = static_cast<MV_SAVE_IAMGE_TYPE>(format);
    save.nQuality = quality;
    save.iMethodValue = 1;
    const int rc = MV_CC_SaveImageToFileEx2(camera->handle, &image, &save, path);
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_SaveImageToFileEx2", rc);
}

int32_t hik_camera_record_start(hik_camera_t* camera, const hik_frame_t* first_frame,
                                float frame_rate, uint32_t bit_rate_kbps, const char* path) {
    if (!camera || !first_frame || !path || frame_rate <= 0)
        return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid record start argument");
    MV_CC_RECORD_PARAM record{};
    record.enPixelType = static_cast<MvGvspPixelType>(first_frame->pixel_format);
    record.nWidth = static_cast<unsigned short>(first_frame->width);
    record.nHeight = static_cast<unsigned short>(first_frame->height);
    record.fFrameRate = frame_rate;
    record.nBitRate = bit_rate_kbps;
    record.enRecordFmtType = MV_FormatType_AVI;
    record.strFilePath = const_cast<char*>(path);
    const int rc = MV_CC_StartRecord(camera->handle, &record);
    if (rc == MV_OK) camera->recording = true;
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_StartRecord", rc);
}

int32_t hik_camera_record_input(hik_camera_t* camera, const hik_frame_t* frame) {
    if (!camera || !frame || !frame->data || !camera->recording)
        return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid record frame");
    MV_CC_INPUT_FRAME_INFO_EX input{};
    input.enPixelType = static_cast<MvGvspPixelType>(frame->pixel_format);
    input.nWidth = frame->width;
    input.nHeight = frame->height;
    input.pData = frame->data;
    input.nDataLen = frame->data_len;
    const int rc = MV_CC_InputOneFrameEx(camera->handle, &input);
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_InputOneFrameEx", rc);
}

int32_t hik_camera_record_stop(hik_camera_t* camera) {
    if (!camera) return fail(HIK_ERROR_INVALID_ARGUMENT, "camera is null");
    if (!camera->recording) return HIK_OK;
    const int rc = MV_CC_StopRecord(camera->handle);
    if (rc == MV_OK) camera->recording = false;
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_StopRecord", rc);
}

int32_t hik_camera_rotate(hik_camera_t* camera, const hik_frame_t* source,
                          uint32_t angle, hik_frame_t* output) {
    if (!camera || !source || !source->data || !output || angle < 1 || angle > 3)
        return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid rotate argument");
    std::memset(output, 0, sizeof(*output));
    const size_t capacity = source->data_len;
    auto data = new (std::nothrow) uint8_t[capacity];
    if (!data) return fail(HIK_ERROR_OUT_OF_MEMORY, "rotate allocation failed");
    MV_CC_ROTATE_IMAGE_PARAM param{};
    param.enPixelType = static_cast<MvGvspPixelType>(source->pixel_format);
    param.nWidth = source->width; param.nHeight = source->height;
    param.pSrcData = source->data; param.nSrcDataLen = static_cast<unsigned int>(source->data_len);
    param.pDstBuf = data; param.nDstBufSize = static_cast<unsigned int>(capacity);
    param.enRotationAngle = static_cast<MV_IMG_ROTATION_ANGLE>(angle);
    const int rc = MV_CC_RotateImage(camera->handle, &param);
    if (rc != MV_OK) { delete[] data; return fail(rc, "MV_CC_RotateImage", rc); }
    *output = *source;
    output->width = param.nWidth; output->height = param.nHeight;
    output->stride = output->height ? param.nDstBufLen / output->height : 0;
    output->data = data; output->data_len = param.nDstBufLen;
    return HIK_OK;
}

int32_t hik_camera_flip(hik_camera_t* camera, const hik_frame_t* source,
                        uint32_t direction, hik_frame_t* output) {
    if (!camera || !source || !source->data || !output || direction < 1 || direction > 2)
        return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid flip argument");
    std::memset(output, 0, sizeof(*output));
    const size_t capacity = source->data_len;
    auto data = new (std::nothrow) uint8_t[capacity];
    if (!data) return fail(HIK_ERROR_OUT_OF_MEMORY, "flip allocation failed");
    MV_CC_FLIP_IMAGE_PARAM param{};
    param.enPixelType = static_cast<MvGvspPixelType>(source->pixel_format);
    param.nWidth = source->width; param.nHeight = source->height;
    param.pSrcData = source->data; param.nSrcDataLen = static_cast<unsigned int>(source->data_len);
    param.pDstBuf = data; param.nDstBufSize = static_cast<unsigned int>(capacity);
    param.enFlipType = static_cast<MV_IMG_FLIP_TYPE>(direction);
    const int rc = MV_CC_FlipImage(camera->handle, &param);
    if (rc != MV_OK) { delete[] data; return fail(rc, "MV_CC_FlipImage", rc); }
    *output = *source;
    output->data = data; output->data_len = param.nDstBufLen;
    return HIK_OK;
}

int32_t hik_camera_feature_save(hik_camera_t* camera, const char* path) {
    if (!camera || !path) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid feature save argument");
    const int rc = MV_CC_FeatureSave(camera->handle, path);
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_FeatureSave", rc);
}

int32_t hik_camera_feature_load(hik_camera_t* camera, const char* path) {
    if (!camera || !path) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid feature load argument");
    const int rc = MV_CC_FeatureLoad(camera->handle, path);
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_CC_FeatureLoad", rc);
}

int32_t hik_force_ip(const char* serial, uint32_t ip, uint32_t subnet, uint32_t gateway) {
    if (!serial) return fail(HIK_ERROR_INVALID_ARGUMENT, "serial is null");
    MV_CC_DEVICE_INFO_LIST list{};
    int rc = enumerate_sdk(list);
    if (rc != MV_OK) return fail(rc, "MV_CC_EnumDevices", rc);
    MV_CC_DEVICE_INFO* selected = nullptr;
    for (unsigned int i = 0; i < list.nDeviceNum; ++i) {
        if (serial_of(list.pDeviceInfo[i]) == serial) { selected = list.pDeviceInfo[i]; break; }
    }
    if (!selected) return fail(HIK_ERROR_NOT_FOUND, "camera serial not found");
    if (selected->nTLayerType != MV_GIGE_DEVICE)
        return fail(HIK_ERROR_INVALID_ARGUMENT, "Force IP requires a GigE camera");
    void* handle = nullptr;
    rc = MV_CC_CreateHandle(&handle, selected);
    if (rc != MV_OK) return fail(rc, "MV_CC_CreateHandle", rc);
    rc = MV_GIGE_ForceIpEx(handle, ip, subnet, gateway);
    MV_CC_DestroyHandle(handle);
    return rc == MV_OK ? HIK_OK : fail(rc, "MV_GIGE_ForceIpEx", rc);
}

int32_t hik_camera_command(hik_camera_t* camera, const char* name) {
    if (!camera || !name) return fail(HIK_ERROR_INVALID_ARGUMENT, "invalid command parameter");
    const int rc = MV_CC_SetCommandValue(camera->handle, name);
    return rc == MV_OK ? HIK_OK : fail(rc, name, rc);
}

void hik_camera_close(hik_camera_t* camera) {
    if (!camera) return;
    if (camera->recording) MV_CC_StopRecord(camera->handle);
    if (camera->grabbing) MV_CC_StopGrabbing(camera->handle);
    if (camera->handle) {
        MV_CC_CloseDevice(camera->handle);
        MV_CC_DestroyHandle(camera->handle);
    }
    delete camera;
}

const char* hik_last_error(void) { return g_last_error.c_str(); }
}
