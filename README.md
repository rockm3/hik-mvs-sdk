# hik-sdk

Safe Rust bindings and a C++ resource-management layer for the Hikrobot MVS SDK.

海康机器人 MVS SDK 的安全 Rust 绑定与 C++ 资源管理层。

Rust 项目可以直接把本仓库作为
Cargo 依赖，不需要 CMake，也不需要先构建本项目的 DLL。

```toml
[dependencies]
hik-sdk = { git = "https://github.com/your-org/hik-sdk" }
```

发布 crates.io 后可改为版本依赖：

```toml
[dependencies]
hik-sdk = "0.1"
```

## License / 开源许可证

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

本项目采用 `MIT OR Apache-2.0` 双许可证，使用者可任选其一。MIT 简洁宽松；
Apache-2.0 额外提供明确的专利授权，更适合 SDK、FFI 和商业集成场景。

This license applies only to this wrapper. The Hikrobot MVS SDK, headers, libraries,
drivers, and documentation remain subject to Hikrobot's own license terms and are not
redistributed by this crate.

许可证仅覆盖本封装项目。海康机器人 MVS SDK、头文件、动态库、驱动和文档仍受海康机器人
自身许可条款约束，本 crate 不重新分发这些文件。

## MVS 自动发现

构建脚本按以下优先级寻找 MVS Development SDK：

1. `HIK_MVS_SDK_DIR`（本 crate 推荐的显式配置）
2. `MVS_SDK_DIR`（通用兼容名称）
3. MVS 安装器写入的 `MVCAM_COMMON_RUNENV`
4. 当前平台的常见安装目录

例如：

```powershell
$env:HIK_MVS_SDK_DIR = 'D:\Program Files\MVS\Development'
cargo run --example enumerate
```

Linux：

```bash
export HIK_MVS_SDK_DIR=/opt/MVS
cargo run --example enumerate
```

Development SDK 必须包含 `MvCameraControl.h` 和目标平台的链接库。运行时，
`MvCameraControl.dll`/`libMvCameraControl.so` 必须位于操作系统动态库搜索路径中。
MVS Windows 安装器通常已经把 Runtime 目录加入 `PATH`；自定义位置时需要自行加入：

```powershell
$env:PATH = 'C:\my-mvs-runtime;' + $env:PATH
```

## 模块结构 / Module layout

```text
hik_sdk::sdk       SDK 生命周期、枚举、Force IP、Action Command
hik_sdk::camera    相机句柄与采集控制
hik_sdk::frame     Frame、OwnedFrame、Chunk 和输出格式
hik_sdk::imaging   图片编码、旋转和翻转
hik_sdk::events    异常、相机事件和网络统计
hik_sdk::params    GenICam 整数/浮点约束
hik_sdk::error     Error 与 Result
```

推荐新代码按模块导入；原有扁平路径继续兼容：

```rust
use hik_sdk::sdk::Sdk;
use hik_sdk::frame::OutputFormat;
use hik_sdk::error::Result;
```

## 范例

```powershell
cargo run --example enumerate
cargo run --example grab_one -- DB1856739
cargo run --example continuous -- DB1856739 100
cargo run --example software_trigger -- DB1856739
cargo run --example parameters -- DB1856739
cargo run --example multi_camera
cargo run --example feature_file -- save DB1856739 camera_features.ini
cargo run --example reconnect -- DB1856739 30
cargo run --example image_callback -- DB1856739 30
cargo run --example exception_callback -- DB1856739 30
cargo run --example event_callback -- DB1856739 10
cargo run --example network_stats -- DB1856739 60
cargo run --example user_set -- DB1856739
cargo run --example image_formats -- DB1856739
cargo run --example recording -- DB1856739 120 60
cargo run --example image_transform -- DB1856739
cargo run --example chunk_data -- DB1856739
# 广播触发，仅确认配置后运行：
cargo run --example action_command -- --apply DB1856739 255.255.255.255
# 危险操作，仅在确认网络参数后运行：
cargo run --example force_ip -- --apply DB1856739 169.254.183.48 255.255.0.0 0.0.0.0
```

| 范例 | 对应官方能力 |
|---|---|
| `enumerate` | 初始化、GigE/USB 枚举和设备信息 |
| `grab_one` | 连续模式、取一帧、Bayer/Mono 转 BGR8、保存图像 |
| `continuous` | SDK 缓存节点、主动循环取流、吞吐/FPS 统计 |
| `software_trigger` | TriggerMode、TriggerSource、TriggerSoftware |
| `parameters` | Int/Float 通用 GenICam 参数与连接状态 |
| `multi_camera` | 枚举并依次打开多台相机取图 |
| `feature_file` | 相机 Feature 参数保存与加载 |
| `reconnect` | 连接轮询、断线后按序列号重新枚举打开 |
| `force_ip` | GigE Force IP（要求显式 `--apply`） |
| `image_callback` | 官方图像回调，复制为 Rust `OwnedFrame` channel |
| `exception_callback` | 设备异常/断线回调 channel |
| `event_callback` | 曝光、采集等相机事件的安全 channel |
| `network_stats` | GigE 接收、丢包、丢帧与重发统计 |
| `user_set` | UserSet 查询；写操作要求显式 `--apply` |
| `image_formats` | MVS 原生 BMP/JPEG/PNG/TIFF 编码 |
| `recording` | MVS AVI 录像编码 |
| `image_transform` | MVS BGR8 旋转和翻转 |
| `chunk_data` | Chunk/水印帧元数据 |
| `action_command` | GigE Action Command 广播触发（显式 `--apply`） |

所有范例使用 `anyhow` 添加操作上下文；库本身使用 `thiserror` 暴露可匹配的错误类型。

抓图成功后生成 `frame.ppm`。核心封装不依赖 OpenCV；MVS 将 Bayer/Mono 转换为
BGR8，上层可以用帧的宽、高、步长和字节切片构造 OpenCV Mat。

## 设计边界

公开边界是固定的 C ABI，不暴露 MVS 结构体、C++ STL、异常、`cv::Mat` 或 Rust 内部类型。
Rust edition 为 2021，MSRV 为 1.74。平台支持与 ABI 规则见 `COMPATIBILITY.md`。
