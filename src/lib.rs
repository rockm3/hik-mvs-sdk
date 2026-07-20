//! Safe Rust API for Hikrobot MVS cameras.
//! Hikrobot MVS 相机的安全 Rust API。
//!
//! The crate keeps Hikrobot's C/C++ types behind a small C ABI. Camera handles and frame
//! 本 crate 通过精简的 C ABI 隔离 Hikrobot 的 C/C++ 类型。相机句柄与帧
//! buffers are released through RAII, while callback data is copied into Rust-owned memory
//! 缓冲区通过 RAII 释放，回调数据则复制到 Rust 自有内存中，
//! before it leaves the SDK callback thread.
//! 确保数据离开 SDK 回调线程前已经完成复制。
//!
//! # Typical workflow
//! # 典型工作流程
//!
//! 1. Create one [`Sdk`] guard with [`Sdk::initialize`].
//! 1. 使用 `Sdk::initialize` 创建一个 `Sdk` 生命周期守卫。
//! 2. Discover devices with [`Sdk::enumerate`] and open one by serial number.
//! 2. 使用 `Sdk::enumerate` 发现设备，并按序列号打开相机。
//! 3. Configure GenICam nodes, select a trigger mode, then call [`Camera::start`].
//! 3. 配置 GenICam 节点并选择触发模式，然后调用 `Camera::start`。
//! 4. Acquire frames with [`Camera::grab`] or [`Camera::frame_channel`].
//! 4. 使用 `Camera::grab` 或 `Camera::frame_channel` 获取帧。
//!
//! Dropping [`Frame`], [`Camera`], and [`Sdk`] releases resources in the required order.
//! `Frame`、`Camera` 和 `Sdk` 析构时会按要求的顺序释放资源。
#![warn(missing_docs)]

mod sys;

/// Camera handles, acquisition control, and general operations.
/// 相机句柄、采集控制与通用操作。
pub mod camera;
/// Error and result types.
/// 错误和结果类型。
pub mod error;
/// Callbacks, asynchronous events, and network statistics.
/// 回调、异步事件和网络统计。
pub mod events;
/// Frame buffers, Chunk metadata, and output formats.
/// 帧缓冲、Chunk 元数据与输出格式。
pub mod frame;
/// Image encoding and geometric transformation types.
/// 图片编码和几何变换类型。
pub mod imaging;
/// GenICam parameter values and constraints.
/// GenICam 参数值及约束类型。
pub mod params;
/// SDK lifetime, discovery, and network-level operations.
/// SDK 生命周期、设备发现与网络级操作。
pub mod sdk;

use std::{
    ffi::{CStr, CString},
    marker::PhantomData,
    ptr::NonNull,
    slice,
    sync::mpsc,
};

/// Result type returned by all fallible high-level operations in this crate.
/// 该箱中所有可出错的高层操作返回的结果类型。
pub type Result<T> = std::result::Result<T, Error>;

/// Error produced by the safe wrapper before or during an MVS operation.
/// 安全封装层在MVS操作前或运行期间产生的错误。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error code returned by the wrapper or the MVS SDK.
    /// 封装器或MVS SDK返回的错误代码。
    #[error("{message} (MVS/wrapper code 0x{code:08X})", code = *code as u32)]
    Sdk {
        /// Raw signed wrapper or MVS status code.
        /// 封装层或 MVS 返回的原始有符号状态码。
        code: i32,
        /// Operation-specific diagnostic copied from the native thread-local error buffer.
        /// 操作特定诊断从原生线程局部错误缓冲区复制。
        message: String,
    },
    /// A Rust string could not be passed through the C ABI because it contains `\0`.
    /// Rust字符串无法通过C ABI，因为它包含“\0”。
    #[error("{field} contains an interior NUL byte")]
    InteriorNul {
        /// Logical argument name containing the invalid byte.
        /// 逻辑参数名称包含无效字节。
        field: &'static str,
    },
}

fn check(code: i32) -> Result<()> {
    if code == sys::HIK_OK {
        return Ok(());
    }
    // SAFETY: the wrapper returns a thread-local NUL-terminated string whose storage remains
    // 安全：包装器返回一个线程本地的 NUL 终止字符串，其存储仍保留
    // valid until the next wrapper error on this thread. We copy it before returning.
    // 有效，直到本线程下一次封装错误。我们会在返回前复制它。
    let message = unsafe {
        let ptr = sys::hik_last_error();
        if ptr.is_null() {
            "unknown error".into()
        } else {
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    };
    Err(Error::Sdk { code, message })
}

fn cstring(value: &str, field: &'static str) -> Result<CString> {
    CString::new(value).map_err(|_| Error::InteriorNul { field })
}

fn text(chars: &[std::ffi::c_char]) -> String {
    // SAFETY: every fixed-size native string passed here is zero-initialized by the wrapper and
    // 安全性：这里传递的每个固定大小的本地字符串都被包装器初始化为零，且
    // explicitly NUL-terminated before crossing the ABI.
    // 明确在跨越 ABI之前被NUL 结尾。
    unsafe {
        CStr::from_ptr(chars.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}

/// Owned camera description produced by device enumeration.
/// 自有相机描述由设备枚举生成。
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Physical transport used by the device.
    /// 设备使用的物理传输方式。
    pub transport: Transport,
    /// Current IPv4 address for GigE cameras; `None` for transports without an IP address.
    /// GigE相机当前IPv4地址;没有IP地址的传输则显示“无”。
    pub ip: Option<std::net::Ipv4Addr>,
    /// Manufacturer model name, for example `MV-CS016-10GC`.
    /// 制造商型号名称，例如“MV-CS016-10GC”。
    pub model: String,
    /// Stable manufacturer serial number used by [`Sdk::open`].
    /// 稳定的制造商序列号由Sdk：：open使用。
    pub serial: String,
    /// User-defined device name stored in the camera.
    /// 存储在相机中的用户自定义设备名称。
    pub user_name: String,
}

/// Camera transport reported by the MVS enumeration API.
/// MVS枚举API报告的相机传输。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// GigE Vision camera.
    /// GigE Vision相机。
    GigE,
    /// USB3 Vision camera.
    /// USB3 Vision 相机。
    Usb3,
    /// Transport not yet represented by this crate.
    /// 该 crate 尚未表示此传输类型。
    Unknown,
}

/// Process-level MVS SDK lifetime guard.
/// 进程级MVS SDK生命周期守护。
///
/// Keep this value alive for at least as long as every [`Camera`] opened from it. The borrow
/// 此值必须至少与从中打开的每个 `Camera` 存活同样长；
/// carried by `Camera<'sdk>` enforces that ordering for safe Rust code.
/// `Camera<'sdk>` 携带的借用关系会在安全 Rust 代码中强制保证此析构顺序。
pub struct Sdk;

impl Sdk {
    /// Initializes the process-wide MVS runtime.
    /// 初始化进程范围的MVS运行时。
    ///
    /// # Errors
    /// # 错误
    /// Returns [`Error::Sdk`] when `MV_CC_Initialize` fails.
    /// 当“MV_CC_Initialize”失败时返回错误：：sdk。
    pub fn initialize() -> Result<Self> {
        check(unsafe { sys::hik_initialize() })?;
        Ok(Self)
    }

    /// Returns the version of the currently loaded MVS runtime.
    /// 返回当前加载的MVS运行时版本。
    pub fn version(&self) -> SdkVersion {
        SdkVersion::from_raw(unsafe { sys::hik_sdk_version() })
    }

    /// Enumerates GigE Vision and USB3 Vision cameras and returns owned descriptions.
    /// 列举GigE Vision和USB3 Vision相机，并返回自有描述。
    ///
    /// # Errors
    /// # 错误
    /// Returns an SDK error when device discovery fails.
    /// 设备发现失败时会返回SDK错误。
    pub fn enumerate(&self) -> Result<Vec<DeviceInfo>> {
        let mut count = 0;
        check(unsafe { sys::hik_enumerate(std::ptr::null_mut(), 0, &mut count) })?;
        let blank = sys::HikDeviceInfo {
            transport: 0,
            ip: 0,
            model: [0; 64],
            serial: [0; 64],
            user_name: [0; 64],
        };
        let mut raw = vec![blank; count];
        check(unsafe { sys::hik_enumerate(raw.as_mut_ptr(), raw.len(), &mut count) })?;
        raw.truncate(count.min(raw.len()));
        Ok(raw
            .into_iter()
            .map(|d| DeviceInfo {
                transport: match d.transport {
                    1 => Transport::GigE,
                    2 => Transport::Usb3,
                    _ => Transport::Unknown,
                },
                ip: (d.ip != 0).then(|| std::net::Ipv4Addr::from(d.ip)),
                model: text(&d.model),
                serial: text(&d.serial),
                user_name: text(&d.user_name),
            })
            .collect())
    }

    /// Opens a camera by stable manufacturer serial number using exclusive access.
    /// 通过独占访问，按稳定厂商序列号打开相机。
    ///
    /// The returned camera cannot outlive this SDK guard. GigE packet size optimization is
    /// 退回的相机无法比这个SDK守卫活得久。GigE数据包大小优化为
    /// applied automatically when the driver can determine an optimal value.
    /// 当驱动程序能够确定最优值时，会自动应用。
    ///
    /// # Errors
    /// # 错误
    /// Returns an error for an unknown serial, an inaccessible device, or invalid input text.
    /// 返回未知串行、无法访问设备或无效输入文本的错误。
    pub fn open(&'_ self, serial: &str) -> Result<Camera<'_>> {
        let serial = cstring(serial, "serial")?;
        let mut camera = std::ptr::null_mut();
        check(unsafe { sys::hik_camera_open(serial.as_ptr(), &mut camera) })?;
        Ok(Camera {
            inner: NonNull::new(camera).expect("native returned null camera"),
            started: false,
            recording: false,
            frame_callback: None,
            exception_callback: None,
            event_callback: None,
            _sdk: PhantomData,
        })
    }

    /// Forces IPv4 settings on a GigE camera without opening its stream.
    /// 强制在不打开GigE相机流的情况下设置IPv4。
    ///
    /// A successful call can make the device unreachable at its previous address.
    /// 调用成功时，设备在之前的地址可能无法访问。
    ///
    /// # Errors
    /// # 错误
    /// Returns an error for non-GigE devices, unknown serials, or rejected settings.
    /// 对于非 GigE 设备、未知序列号或拒绝设置，返回错误。
    pub fn force_ip(
        &self,
        serial: &str,
        ip: std::net::Ipv4Addr,
        subnet: std::net::Ipv4Addr,
        gateway: std::net::Ipv4Addr,
    ) -> Result<()> {
        let serial = cstring(serial, "serial")?;
        check(unsafe {
            sys::hik_force_ip(
                serial.as_ptr(),
                u32::from(ip),
                u32::from(subnet),
                u32::from(gateway),
            )
        })
    }

    /// Broadcasts a GigE Vision Action Command and returns device acknowledgements.
    /// 广播GigEVision Action Command并返回设备确认。
    ///
    /// The keys and mask must match the Action Control nodes configured in each target camera.
    /// 按键和掩码必须与每个目标相机配置的动作控制节点匹配。
    /// This is memory-safe but can trigger multiple physical devices simultaneously.
    /// 这对内存安全，但可以同时触发多个物理设备。
    ///
    /// # Errors
    /// # 错误
    /// Returns an error if the broadcast address is invalid for the C ABI or the SDK rejects
    /// 如果广播地址对C ABI无效或SDK拒绝，则返回错误
    /// the command.
    /// 命令。
    pub fn action_command(
        &self,
        device_key: u32,
        group_key: u32,
        group_mask: u32,
        broadcast_address: &str,
        timeout_ms: u32,
    ) -> Result<Vec<ActionResult>> {
        let address = cstring(broadcast_address, "broadcast address")?;
        let blank = sys::HikActionResult {
            device_address: [0; 16],
            status: 0,
        };
        let mut results = vec![blank; 256];
        let mut count = 0;
        check(unsafe {
            sys::hik_action_command(
                device_key,
                group_key,
                group_mask,
                address.as_ptr(),
                timeout_ms,
                results.as_mut_ptr(),
                results.len(),
                &mut count,
            )
        })?;
        results.truncate(count.min(results.len()));
        Ok(results
            .into_iter()
            .map(|r| ActionResult {
                device_address: text(&r.device_address),
                status: r.status,
            })
            .collect())
    }
}

impl Drop for Sdk {
    fn drop(&mut self) {
        unsafe {
            sys::hik_finalize();
        }
    }
}

/// Exclusive camera handle tied to the lifetime of an initialized [`Sdk`].
/// 独占的相机句柄，绑定于初始化的Sdk的生命周期。
///
/// The handle closes automatically on drop. A camera may be moved to another thread, but its
/// 句柄在析构时会自动关闭。相机可以被移动到另一个线程，但其
/// methods require exclusive or shared access and therefore cannot race through safe Rust.
/// 方法需要独占或共享访问，因此无法通过安全的Rust进行产生数据竞争。
pub struct Camera<'sdk> {
    inner: NonNull<std::ffi::c_void>,
    started: bool,
    recording: bool,
    frame_callback: Option<Box<FrameCallbackContext>>,
    exception_callback: Option<Box<ExceptionCallbackContext>>,
    event_callback: Option<Box<EventCallbackContext>>,
    _sdk: PhantomData<&'sdk Sdk>,
}
// SAFETY: the native handle is only accessed through methods borrowing `self`; no method exposes
// 安全性：原生句柄仅通过借用“self”的方法访问;没有方法会暴露
// the pointer, and callback contexts use thread-safe channels. The type deliberately is not Sync.
// 指针和回调上下文使用线程安全通道。这种类型故意不是同步。
unsafe impl Send for Camera<'_> {}

impl Camera<'_> {
    /// Starts image acquisition using the camera's current trigger configuration.
    /// 开始使用相机当前触发配置进行图像采集。
    ///
    /// This method does not modify `TriggerMode`; configure it explicitly before starting.
    /// 该方法不修改“触发模式”;开始前要明确配置。
    ///
    /// # Errors
    /// # 错误
    /// Returns an SDK error when acquisition cannot be started.
    /// 当无法启动采集时返回 SDK 错误。
    pub fn start(&mut self) -> Result<()> {
        check(unsafe { sys::hik_camera_start(self.inner.as_ptr()) })?;
        self.started = true;
        Ok(())
    }

    /// Stops image acquisition. Calling it while already stopped is accepted by the wrapper.
    /// 停止图像采集。在已经停止时调用它会被包装器接受。
    ///
    /// # Errors
    /// # 错误
    /// Returns an SDK error when the stream cannot be stopped cleanly.
    /// 当流无法干净地停止时，会返回SDK错误。
    pub fn stop(&mut self) -> Result<()> {
        check(unsafe { sys::hik_camera_stop(self.inner.as_ptr()) })?;
        self.started = false;
        Ok(())
    }

    /// Waits for one frame and returns an owned image buffer.
    /// 等待一帧后返回一个拥有的图像缓冲区。
    ///
    /// `timeout_ms` is the maximum SDK wait in milliseconds. Converted formats are produced by
    /// “timeout_ms”是SDK的最大等待时间，单位是毫秒。转换格式由以下公司制作
    /// the MVS pixel converter. Do not combine active polling with [`Self::frame_channel`].
    /// MVS像素转换器。请勿将主动轮询与Self：：frame_channel结合使用。
    ///
    /// # Errors
    /// # 错误
    /// Returns an error on timeout, transport failure, allocation failure, or unsupported pixel
    /// 超时、传输失败、分配失败或不支持像素时返回错误
    /// conversion.
    /// 转换。
    pub fn grab(&mut self, timeout_ms: u32, format: OutputFormat) -> Result<Frame> {
        let mut raw = sys::HikFrame {
            width: 0,
            height: 0,
            stride: 0,
            pixel_format: 0,
            frame_number: 0,
            timestamp: 0,
            gain: 0.0,
            exposure_time: 0.0,
            average_brightness: 0,
            lost_packets: 0,
            chunk_width: 0,
            chunk_height: 0,
            unparsed_chunk_count: 0,
            data: std::ptr::null_mut(),
            data_len: 0,
        };
        check(unsafe {
            sys::hik_camera_grab(self.inner.as_ptr(), timeout_ms, format as i32, &mut raw)
        })?;
        Ok(Frame { raw })
    }

    /// Queries whether the SDK currently considers the physical device connected.
    /// 查询SDK当前是否将物理设备视为已连接。
    pub fn is_connected(&self) -> Result<bool> {
        let mut value = 0;
        check(unsafe { sys::hik_camera_is_connected(self.inner.as_ptr(), &mut value) })?;
        Ok(value != 0)
    }

    /// Sets the number of frame-buffer nodes maintained by the MVS stream engine.
    /// 设定MVS流引擎维护的帧缓冲节点数量。
    ///
    /// Must normally be called before [`Self::start`]. `count` must be greater than zero.
    /// 通常必须在Self：：start之前调用。“计数”必须大于零。
    pub fn set_image_node_count(&mut self, count: u32) -> Result<()> {
        check(unsafe { sys::hik_camera_set_image_node_count(self.inner.as_ptr(), count) })
    }

    /// Registers the official MVS image callback and returns a Rust receiver.
    /// 注册官方MVS图像回调并返回Rust接收器。
    ///
    /// Every callback frame is copied into [`OwnedFrame`] before the SDK callback returns. This
    /// 每个回调帧都会被复制到 OwnedFrame，然后 SDK 回调才返回。就是这样
    /// makes receiving safe but allocates and copies once per frame. Register before starting.
    /// 使接收安全，但每帧分配和复制一次。开始前请注册。
    /// Dropping the receiver discards later frames without panicking across FFI.
    /// 丢弃接收器可以丢弃后续帧，不会在FFI中panic。
    pub fn frame_channel(&mut self) -> Result<mpsc::Receiver<OwnedFrame>> {
        let (sender, receiver) = mpsc::channel();
        let mut context = Box::new(FrameCallbackContext { sender });
        check(unsafe {
            sys::hik_camera_register_frame_callback(
                self.inner.as_ptr(),
                rust_frame_callback,
                (&mut *context as *mut FrameCallbackContext).cast(),
            )
        })?;
        self.frame_callback = Some(context);
        Ok(receiver)
    }

    /// Registers the device exception callback and returns its event receiver.
    /// 注册设备异常回调并返回其事件接收器。
    ///
    /// The most important event is [`ExceptionEvent::Disconnected`]. Register after opening and
    /// 最重要的事件是 `ExceptionEvent::Disconnected`。请在打开相机后注册，
    /// keep the camera alive while consuming the channel.
    /// 并在接收事件期间保持相机存活。
    pub fn exception_channel(&mut self) -> Result<mpsc::Receiver<ExceptionEvent>> {
        let (sender, receiver) = mpsc::channel();
        let mut context = Box::new(ExceptionCallbackContext { sender });
        check(unsafe {
            sys::hik_camera_register_exception_callback(
                self.inner.as_ptr(),
                rust_exception_callback,
                (&mut *context as *mut ExceptionCallbackContext).cast(),
            )
        })?;
        self.exception_callback = Some(context);
        Ok(receiver)
    }

    /// Registers the camera event callback and returns owned event records through a channel.
    /// 注册相机事件回调，并通过通道返回已拥有的事件记录。
    ///
    /// Event notifications must also be enabled in the camera's GenICam nodes.
    /// 事件通知也必须在相机的 GenICam 节点中启用。
    pub fn event_channel(&mut self) -> Result<mpsc::Receiver<CameraEvent>> {
        let (sender, receiver) = mpsc::channel();
        let mut context = Box::new(EventCallbackContext { sender });
        check(unsafe {
            sys::hik_camera_register_event_callback(
                self.inner.as_ptr(),
                rust_event_callback,
                (&mut *context as *mut EventCallbackContext).cast(),
            )
        })?;
        self.event_callback = Some(context);
        Ok(receiver)
    }

    /// Reads cumulative GigE transport statistics for the current open/stream interval.
    /// 读取当前打开/取流期间的累计GigE传输统计数据。
    ///
    /// # Errors
    /// # 错误
    /// USB cameras and unsupported transports return an SDK error.
    /// USB相机和不支持的传输会返回SDK错误。
    pub fn network_stats(&self) -> Result<NetworkStats> {
        let mut stats = sys::HikNetworkStats {
            received_bytes: 0,
            lost_packets: 0,
            lost_frames: 0,
            received_frames: 0,
            requested_resend_packets: 0,
            resent_packets: 0,
        };
        check(unsafe { sys::hik_camera_network_stats(self.inner.as_ptr(), &mut stats) })?;
        Ok(NetworkStats {
            received_bytes: stats.received_bytes,
            lost_packets: stats.lost_packets,
            lost_frames: stats.lost_frames,
            received_frames: stats.received_frames,
            requested_resend_packets: stats.requested_resend_packets,
            resent_packets: stats.resent_packets,
        })
    }

    /// Encodes a frame to an image file using the MVS image encoder.
    /// 使用MVS图像编码器将帧编码为图像文件。
    ///
    /// `quality` is used for JPEG and should be in `(50, 99]`; it is ignored by other formats.
    /// “质量”用于JPEG，应在“（50， 99]'中;其他格式会忽略它。
    /// Paths pass through the SDK's UTF-8/ANSI C interface and are subject to its platform limit.
    /// 路径通过SDK的UTF-8/ANSI C接口，并受其平台限制。
    ///
    /// # Errors
    /// # 错误
    /// Returns an SDK error for unsupported source formats, invalid quality, or file I/O failure.
    /// 返回SDK错误，原因是不支持的源格式、质量无效或文件I/O失败。
    pub fn save_image(
        &self,
        frame: &Frame,
        format: ImageFormat,
        quality: u32,
        path: &str,
    ) -> Result<()> {
        let path = cstring(path, "image path")?;
        check(unsafe {
            sys::hik_camera_save_image(
                self.inner.as_ptr(),
                &frame.raw,
                format as i32,
                quality,
                path.as_ptr(),
            )
        })
    }

    /// Starts the MVS AVI recorder using metadata from `first_frame`.
    /// 使用“first_frame”中的元数据启动MVS AVI录制器。
    ///
    /// `frame_rate` is frames per second and `bit_rate_kbps` is kilobits per second. Subsequent
    /// “frame_rate”是每秒帧数，“bit_rate_kbps”是千比特每秒。后续
    /// frames passed to [`Self::record_input`] must have compatible dimensions and pixel format.
    /// 传递给Self：：record_input的帧必须具有兼容的尺寸和像素格式。
    ///
    /// # Errors
    /// # 错误
    /// Returns an error for invalid recording parameters or an unavailable encoder.
    /// 因录制参数无效或编码器不可用而返回错误。
    pub fn record_start(
        &mut self,
        first_frame: &Frame,
        frame_rate: f32,
        bit_rate_kbps: u32,
        path: &str,
    ) -> Result<()> {
        let path = cstring(path, "record path")?;
        check(unsafe {
            sys::hik_camera_record_start(
                self.inner.as_ptr(),
                &first_frame.raw,
                frame_rate,
                bit_rate_kbps,
                path.as_ptr(),
            )
        })?;
        self.recording = true;
        Ok(())
    }

    /// Submits one acquired frame to an active recorder.
    /// 将一帧获得的帧提交给活动录像器。
    ///
    /// # Errors
    /// # 错误
    /// Returns an error if recording is inactive or the frame is incompatible.
    /// 如果录制处于非激活状态或帧不兼容，会返回错误。
    pub fn record_input(&mut self, frame: &Frame) -> Result<()> {
        check(unsafe { sys::hik_camera_record_input(self.inner.as_ptr(), &frame.raw) })
    }

    /// Finalizes the current AVI file. It is also attempted automatically on camera drop.
    /// 最终确定当前的AVI文件。相机掉落时也会自动尝试。
    pub fn record_stop(&mut self) -> Result<()> {
        check(unsafe { sys::hik_camera_record_stop(self.inner.as_ptr()) })?;
        self.recording = false;
        Ok(())
    }

    /// Rotates a Mono8 or RGB/BGR packed frame through the MVS image processor.
    /// 通过MVS图像处理器旋转一个Mono8或RGB/BGR填充的帧。
    ///
    /// The returned frame owns a newly allocated buffer.
    /// 返回的帧拥有一个新分配的缓冲区。
    pub fn rotate(&self, source: &Frame, angle: Rotation) -> Result<Frame> {
        let mut output = empty_frame();
        check(unsafe {
            sys::hik_camera_rotate(self.inner.as_ptr(), &source.raw, angle as u32, &mut output)
        })?;
        Ok(Frame { raw: output })
    }

    /// Flips a Mono8 or RGB/BGR packed frame and returns a newly owned buffer.
    /// 翻转一个Mono8或RGB/BGR填充的帧，返回一个新拥有的缓冲区。
    pub fn flip(&self, source: &Frame, direction: FlipDirection) -> Result<Frame> {
        let mut output = empty_frame();
        check(unsafe {
            sys::hik_camera_flip(
                self.inner.as_ptr(),
                &source.raw,
                direction as u32,
                &mut output,
            )
        })?;
        Ok(Frame { raw: output })
    }

    /// Writes a floating-point GenICam node such as `ExposureTime` or `Gain`.
    /// 写入浮点GenICam节点，如“ExposureTime”或“Gain”。
    pub fn set_float(&mut self, name: &str, value: f32) -> Result<()> {
        let name = cstring(name, "parameter name")?;
        check(unsafe { sys::hik_camera_set_float(self.inner.as_ptr(), name.as_ptr(), value) })
    }

    /// Reads a floating-point GenICam node together with its allowed range.
    /// 读取浮点 GenICam 节点及其允许范围。
    pub fn get_float(&self, name: &str) -> Result<FloatValue> {
        let name = cstring(name, "parameter name")?;
        let mut v = sys::HikFloatValue {
            current: 0.0,
            minimum: 0.0,
            maximum: 0.0,
        };
        check(unsafe { sys::hik_camera_get_float(self.inner.as_ptr(), name.as_ptr(), &mut v) })?;
        Ok(FloatValue {
            current: v.current,
            minimum: v.minimum,
            maximum: v.maximum,
        })
    }

    /// Writes an integer GenICam node such as `Width`, `Height`, or an Action key.
    /// 写入整数GenICam节点，如“Width”、“Height”或Action键。
    pub fn set_int(&mut self, name: &str, value: i64) -> Result<()> {
        let name = cstring(name, "parameter name")?;
        check(unsafe { sys::hik_camera_set_int(self.inner.as_ptr(), name.as_ptr(), value) })
    }

    /// Reads an integer GenICam node and its minimum, maximum, and increment.
    /// 读取整数GenICam节点及其最小值、最大值和增量。
    pub fn get_int(&self, name: &str) -> Result<IntValue> {
        let name = cstring(name, "parameter name")?;
        let mut v = sys::HikIntValue {
            current: 0,
            minimum: 0,
            maximum: 0,
            increment: 0,
        };
        check(unsafe { sys::hik_camera_get_int(self.inner.as_ptr(), name.as_ptr(), &mut v) })?;
        Ok(IntValue {
            current: v.current,
            minimum: v.minimum,
            maximum: v.maximum,
            increment: v.increment,
        })
    }

    /// Selects an enumeration node by symbolic name, for example `TriggerMode = On`.
    /// 通过符号名称选择枚举节点，例如“TriggerMode = On”。
    pub fn set_enum(&mut self, name: &str, value: &str) -> Result<()> {
        let name = cstring(name, "parameter name")?;
        let value = cstring(value, "parameter value")?;
        check(unsafe {
            sys::hik_camera_set_enum(self.inner.as_ptr(), name.as_ptr(), value.as_ptr())
        })
    }

    /// Returns the symbolic name of the current enumeration value.
    /// 返回当前枚举值的符号名称。
    pub fn get_enum(&self, name: &str) -> Result<String> {
        let name = cstring(name, "parameter name")?;
        let mut required = 0;
        check(unsafe {
            sys::hik_camera_get_enum(
                self.inner.as_ptr(),
                name.as_ptr(),
                std::ptr::null_mut(),
                0,
                &mut required,
            )
        })?;
        let mut value = vec![0i8; required];
        check(unsafe {
            sys::hik_camera_get_enum(
                self.inner.as_ptr(),
                name.as_ptr(),
                value.as_mut_ptr(),
                value.len(),
                &mut required,
            )
        })?;
        Ok(unsafe { CStr::from_ptr(value.as_ptr()) }
            .to_string_lossy()
            .into_owned())
    }

    /// Writes a Boolean GenICam node such as `ChunkModeActive`.
    /// 写入布尔 GenICam 节点，如 'ChunkModeActive'。
    pub fn set_bool(&mut self, name: &str, value: bool) -> Result<()> {
        let name = cstring(name, "parameter name")?;
        check(unsafe {
            sys::hik_camera_set_bool(self.inner.as_ptr(), name.as_ptr(), u8::from(value))
        })
    }

    /// Reads a Boolean GenICam node.
    /// 读取一个布尔GenICam节点。
    pub fn get_bool(&self, name: &str) -> Result<bool> {
        let name = cstring(name, "parameter name")?;
        let mut value = 0;
        check(unsafe { sys::hik_camera_get_bool(self.inner.as_ptr(), name.as_ptr(), &mut value) })?;
        Ok(value != 0)
    }

    /// Writes a string GenICam node such as `DeviceUserID`.
    /// 写入字符串GenICam节点，如“DeviceUserID”。
    pub fn set_string(&mut self, name: &str, value: &str) -> Result<()> {
        let name = cstring(name, "parameter name")?;
        let value = cstring(value, "parameter value")?;
        check(unsafe {
            sys::hik_camera_set_string(self.inner.as_ptr(), name.as_ptr(), value.as_ptr())
        })
    }

    /// Reads a string GenICam node into an owned UTF-8-lossy Rust string.
    /// 将字符串GenICam节点读取到拥有的UTF-8有损Rust字符串中。
    pub fn get_string(&self, name: &str) -> Result<String> {
        let name = cstring(name, "parameter name")?;
        let mut required = 0;
        check(unsafe {
            sys::hik_camera_get_string(
                self.inner.as_ptr(),
                name.as_ptr(),
                std::ptr::null_mut(),
                0,
                &mut required,
            )
        })?;
        let mut value = vec![0i8; required];
        check(unsafe {
            sys::hik_camera_get_string(
                self.inner.as_ptr(),
                name.as_ptr(),
                value.as_mut_ptr(),
                value.len(),
                &mut required,
            )
        })?;
        Ok(unsafe { CStr::from_ptr(value.as_ptr()) }
            .to_string_lossy()
            .into_owned())
    }

    /// Executes a command node such as `TriggerSoftware` or `UserSetSave` once.
    /// 执行一次命令节点，如“TriggerSoftware”或“UserSetSave”。
    pub fn command(&mut self, name: &str) -> Result<()> {
        let name = cstring(name, "command")?;
        check(unsafe { sys::hik_camera_command(self.inner.as_ptr(), name.as_ptr()) })
    }

    /// Saves the camera's GenICam feature configuration to a host file.
    /// 将相机的GenICamFeature 配置保存到主机文件中。
    pub fn feature_save(&self, path: &str) -> Result<()> {
        let path = cstring(path, "feature path")?;
        check(unsafe { sys::hik_camera_feature_save(self.inner.as_ptr(), path.as_ptr()) })
    }

    /// Loads GenICam features from a host file and applies them to the camera.
    /// 从主机文件加载GenICam的功能并应用到相机上。
    ///
    /// This can change acquisition, trigger, network, and image-format settings.
    /// 这可以改变采集、触发、网络和图像格式的设置。
    pub fn feature_load(&mut self, path: &str) -> Result<()> {
        let path = cstring(path, "feature path")?;
        check(unsafe { sys::hik_camera_feature_load(self.inner.as_ptr(), path.as_ptr()) })
    }

    /// Saves current settings into a camera-resident UserSet slot.
    /// 将当前设置保存到相机驻留的UserSet插槽中。
    pub fn user_set_save(&mut self, user_set: &str) -> Result<()> {
        self.set_enum("UserSetSelector", user_set)?;
        self.command("UserSetSave")
    }

    /// Loads a camera-resident UserSet slot immediately.
    /// 立即加载一个相机内置的 UserSet 插槽。
    pub fn user_set_load(&mut self, user_set: &str) -> Result<()> {
        self.set_enum("UserSetSelector", user_set)?;
        self.command("UserSetLoad")
    }

    /// Selects the UserSet loaded by the camera on its next startup.
    /// 在相机下一次启动时选择加载的用户集。
    pub fn user_set_default(&mut self, user_set: &str) -> Result<()> {
        self.set_enum("UserSetDefault", user_set)
    }
}

impl Drop for Camera<'_> {
    fn drop(&mut self) {
        unsafe {
            if self.recording {
                sys::hik_camera_record_stop(self.inner.as_ptr());
            }
            if self.started {
                sys::hik_camera_stop(self.inner.as_ptr());
            }
            sys::hik_camera_close(self.inner.as_ptr());
        }
    }
}

/// Integer GenICam node value and constraints.
/// 整数GenICam节点值与约束。
#[derive(Debug, Clone, Copy)]
pub struct IntValue {
    /// Current node value.
    /// 当前节点值。
    pub current: i64,
    /// Smallest permitted value.
    /// 最小允许值。
    pub minimum: i64,
    /// Largest permitted value.
    /// 最大允许值。
    pub maximum: i64,
    /// Required step between valid values; zero if the device does not report one.
    /// 有效值之间的必要步进;如果设备没有报告，则为零。
    pub increment: i64,
}
/// Floating-point GenICam node value and constraints.
/// 浮点 GenICam 节点值与约束。
#[derive(Debug, Clone, Copy)]
pub struct FloatValue {
    /// Current node value.
    /// 当前节点值。
    pub current: f32,
    /// Smallest permitted value.
    /// 最小允许值。
    pub minimum: f32,
    /// Largest permitted value.
    /// 最大允许值。
    pub maximum: f32,
}

/// Four-component version number reported by `MV_CC_GetSDKVersion`.
/// 四组件版本号由“MV_CC_GetSDKVersion”报告。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SdkVersion {
    /// Major version component.
    /// 主版本号。
    pub major: u8,
    /// Minor version component.
    /// 小版本组件。
    pub minor: u8,
    /// Patch version component.
    /// 补丁版本组件。
    pub patch: u8,
    /// Build or revision component.
    /// 构建或修订组件。
    pub build: u8,
    /// Original packed 32-bit SDK value.
    /// 原始打包的32位SDK值。
    pub raw: u32,
}

/// Acknowledgement returned by one device after an Action Command broadcast.
/// 在Action Command广播后，一台设备返回了确认。
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// Device IPv4 address formatted by the MVS SDK.
    /// 设备IPv4地址由MVS SDK格式化。
    pub device_address: String,
    /// GigE Vision acknowledgement status; zero indicates success.
    /// GigE Vision确认状态;零表示成功。
    pub status: i32,
}
impl SdkVersion {
    fn from_raw(raw: u32) -> Self {
        Self {
            major: (raw >> 24) as u8,
            minor: (raw >> 16) as u8,
            patch: (raw >> 8) as u8,
            build: raw as u8,
            raw,
        }
    }
}
impl std::fmt::Display for SdkVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.major, self.minor, self.patch, self.build
        )
    }
}

/// Pixel representation requested from [`Camera::grab`].
/// 像素表示请求来自Camera：：grab。
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Preserve the camera's native pixel format and encoded layout.
    /// 保持相机的原始像素格式和编码布局。
    Raw = 0,
    /// Convert to one unsigned 8-bit luminance channel per pixel.
    /// 转换为每个像素一个无符号的8位亮度通道。
    Mono8 = 1,
    /// Convert to packed blue-green-red bytes, three bytes per pixel.
    /// 转换为打包蓝绿红字节，每个像素三字节。
    Bgr8 = 2,
}

/// File encoding used by [`Camera::save_image`].
/// 文件编码由Camera：：save_image使用。
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// Uncompressed Windows bitmap.
    /// 未压缩的Windows位图。
    Bmp = 1,
    /// Lossy JPEG encoding controlled by the quality argument.
    /// 由质量参数控制的有损JPEG编码。
    Jpeg = 2,
    /// Lossless PNG encoding.
    /// 无损PNG编码。
    Png = 3,
    /// TIFF encoding.
    /// TIFF编码。
    Tiff = 4,
}

/// Clockwise rotation angle for [`Camera::rotate`].
/// 顺时针旋转角度 Camera：：rotate。
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// Rotate clockwise by 90 degrees.
    /// 顺时针旋转90度。
    Degrees90 = 1,
    /// Rotate by 180 degrees.
    /// 旋转180度。
    Degrees180 = 2,
    /// Rotate clockwise by 270 degrees.
    /// 顺时针旋转270度。
    Degrees270 = 3,
}

/// Axis used by [`Camera::flip`].
/// 轴线由Camera：：flip使用。
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlipDirection {
    /// Mirror top to bottom.
    /// 从上到下镜像。
    Vertical = 1,
    /// Mirror left to right.
    /// 从左到右镜像。
    Horizontal = 2,
}

/// Owned frame returned by synchronous acquisition and image transforms.
/// 拥有帧通过同步采集和图像转换返回。
///
/// The backing allocation is owned by the native wrapper and is released on drop. Use
/// 备份分配归原生封装者所有，并直接释放。用途
/// [`Self::data`] to borrow it; the pointer is never exposed by the safe API.
/// Self：:d ata借用;该指针从未被安全API暴露。
pub struct Frame {
    raw: sys::HikFrame,
}
impl Frame {
    /// Image width in pixels.
    /// 图像宽度以像素为单位。
    pub fn width(&self) -> u32 {
        self.raw.width
    }
    /// Image height in pixels.
    /// 图像高度（像素单位）。
    pub fn height(&self) -> u32 {
        self.raw.height
    }
    /// Number of bytes between adjacent image rows.
    /// 相邻图像行之间的字节数。
    pub fn stride(&self) -> u32 {
        self.raw.stride
    }
    /// Raw numeric value from Hikrobot's `MvGvspPixelType` enumeration.
    /// 来自Hikrobot的“MvGvspPixelType”枚举中的原始数值。
    pub fn pixel_format(&self) -> u32 {
        self.raw.pixel_format
    }
    /// Monotonic device frame sequence number.
    /// 单调递增的设备帧序号。
    pub fn frame_number(&self) -> u64 {
        self.raw.frame_number
    }
    /// Device timestamp assembled from the SDK's high and low words.
    /// 设备时间戳由SDK的高低字组成。
    pub fn timestamp(&self) -> u64 {
        self.raw.timestamp
    }
    /// Returns parsed watermark and Chunk fields reported with this frame.
    /// 返回时，解析水印和块字段报告，使用此框架。
    pub fn chunk(&self) -> ChunkMetadata {
        ChunkMetadata {
            gain: self.raw.gain,
            exposure_time: self.raw.exposure_time,
            average_brightness: self.raw.average_brightness,
            lost_packets: self.raw.lost_packets,
            width: self.raw.chunk_width,
            height: self.raw.chunk_height,
            unparsed_count: self.raw.unparsed_chunk_count,
        }
    }
    /// Borrows the image bytes for the lifetime of this frame.
    /// 借用该帧寿命内的图像字节。
    pub fn data(&self) -> &[u8] {
        // SAFETY: successful native frame creation guarantees a live allocation of `data_len`
        // 安全性：成功的原生帧创建保证了“data_len”的实时分配
        // bytes. `Frame` owns that allocation exclusively and releases it only in `Drop`.
        // 字节。“Frame”独家拥有该分配，仅在“Drop”中释放。
        unsafe { slice::from_raw_parts(self.raw.data, self.raw.data_len) }
    }
}

fn empty_frame() -> sys::HikFrame {
    sys::HikFrame {
        width: 0,
        height: 0,
        stride: 0,
        pixel_format: 0,
        frame_number: 0,
        timestamp: 0,
        gain: 0.0,
        exposure_time: 0.0,
        average_brightness: 0,
        lost_packets: 0,
        chunk_width: 0,
        chunk_height: 0,
        unparsed_chunk_count: 0,
        data: std::ptr::null_mut(),
        data_len: 0,
    }
}

/// Frame watermark and Chunk fields parsed by the MVS SDK.
/// 帧水印和块字段由 MVS SDK 解析。
///
/// A zero value may mean either that the camera reported zero or that the corresponding Chunk
/// 零值可能表示相机报告零值，或对应的块
/// entry was not enabled. [`Self::unparsed_count`] reports vendor-specific chunks not decoded
/// 进入未被启用。self：：unparsed_count 报告供应商特定Chunk未解码
/// into the standard fields.
/// 进入标准字段。
#[derive(Debug, Clone, Copy, Default)]
pub struct ChunkMetadata {
    /// Gain reported for this frame, in the device-defined unit.
    /// 该帧在设备定义单元内报告增益。
    pub gain: f32,
    /// Exposure time reported for this frame, normally microseconds.
    /// 本帧的曝光时间通常为微秒。
    pub exposure_time: f32,
    /// Device-computed average brightness.
    /// 设备计算的平均亮度。
    pub average_brightness: u32,
    /// Packets lost while receiving this individual frame.
    /// 在接收该单独帧时丢失的数据包。
    pub lost_packets: u32,
    /// Width reported by the Chunk payload.
    /// 块状载荷报告的宽度。
    pub width: u32,
    /// Height reported by the Chunk payload.
    /// Chunk有效载荷报告的高度。
    pub height: u32,
    /// Number of Chunk entries not parsed by the SDK.
    /// SDK 未解析的Chunk条目数量。
    pub unparsed_count: u32,
}
impl Drop for Frame {
    fn drop(&mut self) {
        // SAFETY: `raw` was initialized by the wrapper and has not been released elsewhere; the
        // 安全性：“RAW”由包装初始化，未在其他地方释放;该
        // safe API never exposes a mutable pointer or implements Clone for `Frame`.
        // safe API 从不暴露可变指针，也不会为“Frame”实现克隆。
        unsafe {
            sys::hik_frame_release(&mut self.raw);
        }
    }
}

/// Rust-owned frame delivered by [`Camera::frame_channel`].
/// 由 `Camera::frame_channel` 交付、所有权属于 Rust 的帧。
///
/// Unlike [`Frame`], all bytes live in a normal [`Vec`] and may be retained after the camera is
/// 与[“Frame”]不同，所有字节都存在于正常的Vec中，并且可以在相机
/// stopped or dropped.
/// 停下来或掉落。
#[derive(Debug, Clone)]
pub struct OwnedFrame {
    /// Image width in pixels.
    /// 图像宽度以像素为单位。
    pub width: u32,
    /// Image height in pixels.
    /// 图像高度（像素单位）。
    pub height: u32,
    /// Bytes between adjacent rows.
    /// 相邻行之间的字节。
    pub stride: u32,
    /// Raw `MvGvspPixelType` numeric value.
    /// 原始数值为“MvGvspPixelType”。
    pub pixel_format: u32,
    /// Device frame sequence number.
    /// 设备帧序列号。
    pub frame_number: u64,
    /// Device timestamp.
    /// 设备时间戳。
    pub timestamp: u64,
    /// Owned image bytes copied inside the callback.
    /// 回调内复制的图像字节。
    pub data: Vec<u8>,
}

/// Asynchronous device exception delivered by [`Camera::exception_channel`].
/// 异步设备异常由Camera：：exception_channel提供。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionEvent {
    /// The camera disconnected (`MV_EXCEPTION_DEV_DISCONNECT`).
    /// 相机断开（“MV_EXCEPTION_DEV_DISCONNECT”）。
    Disconnected,
    /// Another vendor exception identified by its raw message type.
    /// 另一个厂商异常，因其原始消息类型被识别。
    Other(u32),
}

/// Owned camera event delivered by [`Camera::event_channel`].
/// 由Camera：：event_channel提供自有相机事件。
#[derive(Debug, Clone)]
pub struct CameraEvent {
    /// GenICam event name, for example an exposure-end event.
    /// GenICam 事件名称，例如曝光结束事件。
    pub name: String,
    /// Numeric event identifier.
    /// 数字事件标识符。
    pub event_id: u16,
    /// Stream channel that produced the event.
    /// 产生该事件的流通道。
    pub stream_channel: u16,
    /// Associated block/frame identifier when supplied by firmware.
    /// 固件提供时，关联的块/帧标识符。
    pub block_id: u64,
    /// Device event timestamp.
    /// 设备事件时间戳。
    pub timestamp: u64,
}

/// Cumulative GigE stream health counters returned by [`Camera::network_stats`].
/// 累计GigE流的健康状态计数器由Camera：：network_stats返回。
#[derive(Debug, Clone, Copy, Default)]
pub struct NetworkStats {
    /// Total payload bytes received during the current stream interval.
    /// 当前流间隔内接收的总有效载荷字节数。
    pub received_bytes: i64,
    /// Number of missing network packets detected.
    /// 检测到的网络数据包缺失数量。
    pub lost_packets: i64,
    /// Number of incomplete frames discarded.
    /// 丢弃的未完成帧数。
    pub lost_frames: u32,
    /// Number of frames received by the network layer.
    /// 网络层接收的帧数。
    pub received_frames: u32,
    /// Number of packet retransmissions requested by the receiver.
    /// 接收方请求的数据包重传次数。
    pub requested_resend_packets: i64,
    /// Number of retransmitted packets actually received.
    /// 实际接收的重传数据包数量。
    pub resent_packets: i64,
}

struct FrameCallbackContext {
    sender: mpsc::Sender<OwnedFrame>,
}
struct ExceptionCallbackContext {
    sender: mpsc::Sender<ExceptionEvent>,
}
struct EventCallbackContext {
    sender: mpsc::Sender<CameraEvent>,
}

unsafe extern "C" fn rust_frame_callback(frame: *const sys::HikFrame, user: *mut std::ffi::c_void) {
    if frame.is_null() || user.is_null() {
        return;
    }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: the C++ trampoline supplies both pointers and keeps them valid for this call.
        // 安全性：C++ 中转函数提供这两个指针，并保持它们在本次调用中的有效性。
        let frame = unsafe { &*frame };
        if frame.data.is_null() {
            return;
        }
        // SAFETY: `user` points to the boxed context retained by `Camera` until after stream stop.
        // 安全：'用户'指向由'相机'保留的装箱的上下文，直到直播停止后。
        let context = unsafe { &*(user as *const FrameCallbackContext) };
        let owned = OwnedFrame {
            width: frame.width,
            height: frame.height,
            stride: frame.stride,
            pixel_format: frame.pixel_format,
            frame_number: frame.frame_number,
            timestamp: frame.timestamp,
            // SAFETY: MVS guarantees the callback buffer for the duration of this callback. The
            // 安全性：MVS保证回调缓冲区在回调期间的存在。该
            // immediate copy ensures no borrowed SDK memory reaches the receiver.
            // 即时复制确保借用的SDK内存不会到达接收端。
            data: unsafe { slice::from_raw_parts(frame.data, frame.data_len) }.to_vec(),
        };
        let _ = context.sender.send(owned);
    }));
}

unsafe extern "C" fn rust_exception_callback(message_type: u32, user: *mut std::ffi::c_void) {
    if user.is_null() {
        return;
    }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: `user` is the boxed context stored by `Camera` until native close completes.
        // 安全：“用户”是“Camera”存储的装箱的上下文，直到原生关闭完成。
        let context = unsafe { &*(user as *const ExceptionCallbackContext) };
        let event = if message_type == 0x0000_8001 {
            ExceptionEvent::Disconnected
        } else {
            ExceptionEvent::Other(message_type)
        };
        let _ = context.sender.send(event);
    }));
}

unsafe extern "C" fn rust_event_callback(event: *const sys::HikEvent, user: *mut std::ffi::c_void) {
    if event.is_null() || user.is_null() {
        return;
    }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: both pointers originate from the C++ callback trampoline and remain valid for
        // 安全：这两个指针都来自C++回调中转函数，并且在
        // this invocation; all event fields are copied before returning.
        // 本次调用;所有事件字段在返回前都会被复制。
        let event = unsafe { &*event };
        // SAFETY: the boxed context is retained by `Camera` until native close completes.
        // 安全：框内上下文由“相机”保留，直到原生关闭完成。
        let context = unsafe { &*(user as *const EventCallbackContext) };
        let owned = CameraEvent {
            name: text(&event.name),
            event_id: event.event_id,
            stream_channel: event.stream_channel,
            block_id: event.block_id,
            timestamp: event.timestamp,
        };
        let _ = context.sender.send(owned);
    }));
}
