//! Raw FFI declarations for the C++ wrapper.
//! C++ 封装层的原始 FFI 声明。
//!
//! This private module mirrors the C header; `lib.rs` enforces pointer, string, and lifetime rules.
//! 本私有模块严格对应 C 头文件；指针、C 字符串和生命周期约束由 `lib.rs` 维护。

use std::ffi::{c_char, c_int, c_void};

/// Success status shared by the wrapper and MVS SDK.
/// C++ 封装层和 MVS SDK 共用的成功状态码。
pub const HIK_OK: c_int = 0;

/// Enumerated device; layout matches the C structure exactly.
/// 设备枚举结果；布局必须与 C 结构体 `hik_device_info_t` 完全一致。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HikDeviceInfo {
    /// Transport: `1` for GigE, `2` for USB3.
    /// 传输层类型：`1` 为 GigE，`2` 为 USB3。
    pub transport: u32,
    /// Current GigE IPv4 address in network byte order.
    /// GigE 相机当前 IPv4 地址，使用网络字节序。
    pub ip: u32,
    /// NUL-terminated camera model name.
    /// 相机型号，NUL 结尾。
    pub model: [c_char; 64],
    /// NUL-terminated manufacturer serial number.
    /// 厂商序列号，NUL 结尾。
    pub serial: [c_char; 64],
    /// NUL-terminated user-defined device name.
    /// 相机中保存的用户自定义名称。
    pub user_name: [c_char; 64],
}

/// Frame descriptor passed across the C ABI.
/// 跨 C ABI 传递的帧描述。
///
/// `data` ownership is contextual: owned frames must be released; callback frames must be copied.
/// `data` 的所有权取决于场景：拥有的帧必须释放，回调帧必须立即复制。
#[repr(C)]
pub struct HikFrame {
    /// Image width in pixels.
    /// 图像宽度，单位为像素。
    pub width: u32,
    /// Image height in pixels.
    /// 图像高度，单位为像素。
    pub height: u32,
    /// Byte distance between adjacent rows.
    /// 相邻两行起始地址之间的字节数。
    pub stride: u32,
    /// Raw Hikrobot pixel-type value.
    /// 海康 `MvGvspPixelType` 的原始数值。
    pub pixel_format: u32,
    /// Device frame sequence number.
    /// 设备帧序号。
    pub frame_number: u64,
    /// Combined device timestamp.
    /// 合并后的设备时间戳。
    pub timestamp: u64,
    /// Per-frame gain watermark.
    /// 此帧携带的增益水印。
    pub gain: f32,
    /// Per-frame exposure time, normally in microseconds.
    /// 此帧携带的曝光时间，通常为微秒。
    pub exposure_time: f32,
    /// Device-computed average brightness.
    /// 设备计算的平均亮度。
    pub average_brightness: u32,
    /// Packets lost while receiving this frame.
    /// 接收此帧时检测到的丢包数。
    pub lost_packets: u32,
    /// Image width reported by Chunk data.
    /// Chunk 中报告的图像宽度。
    pub chunk_width: u32,
    /// Image height reported by Chunk data.
    /// Chunk 中报告的图像高度。
    pub chunk_height: u32,
    /// Number of Chunk entries not parsed by MVS.
    /// SDK 未解析的 Chunk 条目数量。
    pub unparsed_chunk_count: u32,
    /// Image buffer address; ownership depends on the calling context.
    /// 图像数据首地址。
    pub data: *mut u8,
    /// Number of valid bytes at `data`.
    /// `data` 指向的有效字节数。
    pub data_len: usize,
}

/// Integer GenICam value and constraints.
/// 整数型 GenICam 节点的当前值和约束。
#[repr(C)]
pub struct HikIntValue {
    /// Current value.
    /// 当前值。
    pub current: i64,
    /// Minimum allowed value.
    /// 最小允许值。
    pub minimum: i64,
    /// Maximum allowed value.
    /// 最大允许值。
    pub maximum: i64,
    /// Required increment between valid values.
    /// 有效值的步进。
    pub increment: i64,
}

/// Floating-point GenICam value and constraints.
/// 浮点型 GenICam 节点的当前值和约束。
#[repr(C)]
pub struct HikFloatValue {
    /// Current value.
    /// 当前值。
    pub current: f32,
    /// Minimum allowed value.
    /// 最小允许值。
    pub minimum: f32,
    /// Maximum allowed value.
    /// 最大允许值。
    pub maximum: f32,
}

/// Camera-event data copied by the C++ trampoline.
/// 相机事件回调数据。
#[repr(C)]
pub struct HikEvent {
    /// NUL-terminated GenICam event name.
    /// GenICam 事件名，NUL 结尾。
    pub name: [c_char; 128],
    /// Numeric event identifier.
    /// 事件编号。
    pub event_id: u16,
    /// Stream channel that emitted the event.
    /// 产生事件的流通道。
    pub stream_channel: u16,
    /// Associated block/frame identifier.
    /// 关联的 Block/帧编号。
    pub block_id: u64,
    /// Device event timestamp.
    /// 设备事件时间戳。
    pub timestamp: u64,
}

/// GigE transport statistics matching `MV_MATCH_INFO_NET_DETECT`.
/// GigE 传输统计。
#[repr(C)]
pub struct HikNetworkStats {
    /// Total bytes received.
    /// 累计接收字节数。
    pub received_bytes: i64,
    /// Number of lost network packets.
    /// 丢失网络包数量。
    pub lost_packets: i64,
    /// Number of incomplete frames discarded.
    /// 丢弃帧数。
    pub lost_frames: u32,
    /// Frames received by the network layer.
    /// 网络层接收帧数。
    pub received_frames: u32,
    /// Number of resend packets requested.
    /// 请求重发包数。
    pub requested_resend_packets: i64,
    /// Number of resent packets received.
    /// 实际收到的重发包数。
    pub resent_packets: i64,
}

/// Device acknowledgement for an Action Command.
/// 设备对 GigE Action Command 的应答。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HikActionResult {
    /// NUL-terminated device IPv4 address.
    /// 设备 IPv4 地址，NUL 结尾。
    pub device_address: [c_char; 16],
    /// Device status; `0` means success.
    /// 设备返回状态，`0` 表示成功。
    pub status: i32,
}

// SAFETY: declarations mirror the C header exactly; all structures use `#[repr(C)]`.
// 安全性：以下声明逐项对应 C 头文件，结构体均使用 `#[repr(C)]` 固定布局。
extern "C" {
    /// Initializes the process-wide MVS SDK.
    /// 初始化 MVS SDK。
    pub fn hik_initialize() -> c_int;
    /// Finalizes MVS after all cameras are closed.
    /// 反初始化 MVS SDK。
    pub fn hik_finalize() -> c_int;
    /// Returns the packed MVS SDK version.
    /// 获取 SDK 版本。
    pub fn hik_sdk_version() -> u32;
    /// Enumerates cameras with a two-pass buffer query.
    /// 两阶段枚举相机。
    pub fn hik_enumerate(devices: *mut HikDeviceInfo, capacity: usize, count: *mut usize) -> c_int;
    /// Broadcasts an Action Command and copies acknowledgements.
    /// 广播 Action Command。
    pub fn hik_action_command(
        device_key: u32,
        group_key: u32,
        group_mask: u32,
        broadcast_address: *const c_char,
        timeout_ms: u32,
        results: *mut HikActionResult,
        capacity: usize,
        count: *mut usize,
    ) -> c_int;
    /// Opens a camera by serial and returns an opaque handle.
    /// 按序列号打开相机。
    pub fn hik_camera_open(serial: *const c_char, camera: *mut *mut c_void) -> c_int;
    /// Starts acquisition without changing trigger settings.
    /// 启动采集且不修改触发配置。
    pub fn hik_camera_start(camera: *mut c_void) -> c_int;
    /// Stops acquisition.
    /// 停止采集。
    pub fn hik_camera_stop(camera: *mut c_void) -> c_int;
    /// Waits for one owned frame.
    /// 主动等待一帧。
    pub fn hik_camera_grab(
        camera: *mut c_void,
        timeout_ms: u32,
        output: c_int,
        frame: *mut HikFrame,
    ) -> c_int;
    /// Releases an owned frame and clears its descriptor.
    /// 释放帧并清零。
    pub fn hik_frame_release(frame: *mut HikFrame);
    /// Writes a floating-point GenICam node.
    /// 写入浮点节点。
    pub fn hik_camera_set_float(camera: *mut c_void, name: *const c_char, value: f32) -> c_int;
    /// Reads a floating-point node and range.
    /// 读取浮点节点及范围。
    pub fn hik_camera_get_float(
        camera: *mut c_void,
        name: *const c_char,
        value: *mut HikFloatValue,
    ) -> c_int;
    /// Writes an integer GenICam node.
    /// 写入整数节点。
    pub fn hik_camera_set_int(camera: *mut c_void, name: *const c_char, value: i64) -> c_int;
    /// Reads an integer node and constraints.
    /// 读取整数节点及约束。
    pub fn hik_camera_get_int(
        camera: *mut c_void,
        name: *const c_char,
        value: *mut HikIntValue,
    ) -> c_int;
    /// Writes an enum node by symbolic name.
    /// 写入枚举符号。
    pub fn hik_camera_set_enum(
        camera: *mut c_void,
        name: *const c_char,
        value: *const c_char,
    ) -> c_int;
    /// Reads an enum symbol using a size query.
    /// 两阶段读取枚举符号。
    pub fn hik_camera_get_enum(
        camera: *mut c_void,
        name: *const c_char,
        value: *mut c_char,
        capacity: usize,
        required: *mut usize,
    ) -> c_int;
    /// Writes a Boolean node using ABI-stable `0/1`.
    /// 写入布尔节点。
    pub fn hik_camera_set_bool(camera: *mut c_void, name: *const c_char, value: u8) -> c_int;
    /// Reads a Boolean node as `0/1`.
    /// 读取布尔节点。
    pub fn hik_camera_get_bool(camera: *mut c_void, name: *const c_char, value: *mut u8) -> c_int;
    /// Writes a string GenICam node.
    /// 写入字符串节点。
    pub fn hik_camera_set_string(
        camera: *mut c_void,
        name: *const c_char,
        value: *const c_char,
    ) -> c_int;
    /// Reads a string node using a size query.
    /// 两阶段读取字符串。
    pub fn hik_camera_get_string(
        camera: *mut c_void,
        name: *const c_char,
        value: *mut c_char,
        capacity: usize,
        required: *mut usize,
    ) -> c_int;
    /// Queries the SDK connection state.
    /// 查询设备连接。
    pub fn hik_camera_is_connected(camera: *mut c_void, connected: *mut u8) -> c_int;
    /// Sets the SDK image-buffer node count.
    /// 设置图像缓存节点数。
    pub fn hik_camera_set_image_node_count(camera: *mut c_void, count: u32) -> c_int;
    /// Registers an image callback with callback-scoped frame memory.
    /// 注册图像回调。
    pub fn hik_camera_register_frame_callback(
        camera: *mut c_void,
        callback: unsafe extern "C" fn(*const HikFrame, *mut c_void),
        user: *mut c_void,
    ) -> c_int;
    /// Registers a device-exception callback.
    /// 注册异常回调。
    pub fn hik_camera_register_exception_callback(
        camera: *mut c_void,
        callback: unsafe extern "C" fn(u32, *mut c_void),
        user: *mut c_void,
    ) -> c_int;
    /// Registers the all-events callback.
    /// 注册全部事件。
    pub fn hik_camera_register_event_callback(
        camera: *mut c_void,
        callback: unsafe extern "C" fn(*const HikEvent, *mut c_void),
        user: *mut c_void,
    ) -> c_int;
    /// Reads GigE receive, loss, and resend counters.
    /// 读取 GigE 网络统计。
    pub fn hik_camera_network_stats(camera: *mut c_void, stats: *mut HikNetworkStats) -> c_int;
    /// Encodes a frame as BMP/JPEG/PNG/TIFF.
    /// 使用 MVS 保存图片。
    pub fn hik_camera_save_image(
        camera: *mut c_void,
        frame: *const HikFrame,
        format: c_int,
        quality: u32,
        path: *const c_char,
    ) -> c_int;
    /// Starts AVI recording from first-frame metadata.
    /// 启动 AVI 录像。
    pub fn hik_camera_record_start(
        camera: *mut c_void,
        first_frame: *const HikFrame,
        frame_rate: f32,
        bit_rate_kbps: u32,
        path: *const c_char,
    ) -> c_int;
    /// Submits a frame to the active recorder.
    /// 提交录像帧。
    pub fn hik_camera_record_input(camera: *mut c_void, frame: *const HikFrame) -> c_int;
    /// Finalizes and closes the AVI file.
    /// 完成 AVI 文件。
    pub fn hik_camera_record_stop(camera: *mut c_void) -> c_int;
    /// Rotates Mono8/RGB/BGR into an owned frame.
    /// 旋转图像。
    pub fn hik_camera_rotate(
        camera: *mut c_void,
        source: *const HikFrame,
        angle: u32,
        output: *mut HikFrame,
    ) -> c_int;
    /// Flips Mono8/RGB/BGR vertically or horizontally.
    /// 翻转图像。
    pub fn hik_camera_flip(
        camera: *mut c_void,
        source: *const HikFrame,
        direction: u32,
        output: *mut HikFrame,
    ) -> c_int;
    /// Saves camera features to a host file.
    /// 保存 Feature 配置。
    pub fn hik_camera_feature_save(camera: *mut c_void, path: *const c_char) -> c_int;
    /// Loads and applies features from a host file.
    /// 加载 Feature 配置。
    pub fn hik_camera_feature_load(camera: *mut c_void, path: *const c_char) -> c_int;
    /// Forces GigE IPv4 settings by serial number.
    /// 强制修改 GigE IPv4。
    pub fn hik_force_ip(serial: *const c_char, ip: u32, subnet: u32, gateway: u32) -> c_int;
    /// Executes a GenICam command node once.
    /// 执行 Command 节点。
    pub fn hik_camera_command(camera: *mut c_void, name: *const c_char) -> c_int;
    /// Stops, closes, and destroys the camera handle.
    /// 关闭并销毁相机。
    pub fn hik_camera_close(camera: *mut c_void);
    /// Returns a borrowed thread-local error string.
    /// 获取线程局部错误。
    pub fn hik_last_error() -> *const c_char;
}
