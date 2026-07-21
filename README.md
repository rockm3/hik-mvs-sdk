# hik-mvs-sdk

[![Crates.io](https://img.shields.io/crates/v/hik-mvs-sdk.svg)](https://crates.io/crates/hik-mvs-sdk)
[![Documentation](https://docs.rs/hik-mvs-sdk/badge.svg)](https://docs.rs/hik-mvs-sdk)
[![CI](https://github.com/rockm3/hik-mvs-sdk/actions/workflows/ci.yml/badge.svg)](https://github.com/rockm3/hik-mvs-sdk/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/hik-mvs-sdk.svg)](#license--许可证)

Safe Rust bindings and a small C++ RAII bridge for Hikrobot MVS cameras.

海康机器人 MVS 相机的安全 Rust 绑定与轻量 C++ RAII 桥接层。

The crate follows the capabilities and workflows of the official MVS examples while exposing an idiomatic Rust API.

本 crate 尽可能覆盖 MVS 官方范例的能力与流程，同时提供符合 Rust 习惯的 API。

> This project is an independent open-source wrapper and is not an official Hikrobot product.
>
> 本项目是独立开源封装，并非海康机器人的官方产品。

## Features / 功能

- Device enumeration, camera lifecycle management, acquisition, and software triggering.
- 设备枚举、相机生命周期管理、图像采集与软触发。
- Generic GenICam parameter access, feature files, user sets, and reconnect support.
- 通用 GenICam 参数访问、Feature 文件、用户参数组与断线重连。
- Safe channels for image, exception, and camera-event callbacks.
- 使用安全 channel 接收图像、异常和相机事件回调。
- GigE Force IP, network statistics, and Action Command broadcasting.
- GigE Force IP、网络统计和 Action Command 广播。
- Pixel conversion, native image encoding, rotation, flipping, chunk data, and AVI recording.
- 像素转换、原生图像编码、旋转、翻转、Chunk 数据与 AVI 录像。
- No CMake requirement and no vendor SDK files redistributed by this crate.
- 无需 CMake，且本 crate 不重新分发厂商 SDK 文件。

## Installation / 安装

Add the published crate to your project:

在项目中添加已发布的 crate：

```bash
cargo add hik-mvs-sdk
```

Or edit `Cargo.toml`:

也可以直接编辑 `Cargo.toml`：

```toml
[dependencies]
hik-mvs-sdk = "0.1"
```

To track unreleased changes, use the Git repository explicitly:

如需跟踪尚未发布的改动，可显式使用 Git 仓库：

```toml
[dependencies]
hik-mvs-sdk = { git = "https://github.com/rockm3/hik-mvs-sdk" }
```

## Prerequisites / 前置要求

Install the Hikrobot MVS Development SDK for the target platform before building.

构建前需要安装目标平台对应的海康机器人 MVS Development SDK。

The SDK installation must provide `MvCameraControl.h`, the link library, and the runtime shared library.

SDK 安装目录必须包含 `MvCameraControl.h`、链接库以及运行时动态库。

The crate contains only its own bridge source and never bundles Hikrobot headers, libraries, drivers, or documentation.

本 crate 仅包含自身桥接层源码，不内置海康机器人的头文件、库、驱动或文档。

## SDK discovery / SDK 自动发现

`build.rs` searches for the MVS Development SDK in the following order:

`build.rs` 按以下优先级查找 MVS Development SDK：

| Priority / 优先级 | Location / 位置 |
|---|---|
| 1 | `HIK_MVS_SDK_DIR`, the recommended explicit override / 本 crate 推荐的显式配置 |
| 2 | `MVS_SDK_DIR`, a compatible generic override / 兼容的通用配置名称 |
| 3 | `MVCAM_COMMON_RUNENV`, normally created by the MVS installer / 通常由 MVS 安装程序写入 |
| 4 | Common installation directories for the target platform / 目标平台的常见安装目录 |

Windows PowerShell example:

Windows PowerShell 示例：

```powershell
$env:HIK_MVS_SDK_DIR = 'D:\Program Files\MVS\Development'
cargo run --example enumerate
```

Linux example:

Linux 示例：

```bash
export HIK_MVS_SDK_DIR=/opt/MVS
cargo run --example enumerate
```

At runtime, `MvCameraControl.dll` or `libMvCameraControl.so` must be visible through the operating system's dynamic-library search path.

运行时必须确保操作系统可通过动态库搜索路径找到 `MvCameraControl.dll` 或 `libMvCameraControl.so`。

The Windows installer normally configures `PATH`; for a custom runtime location, add it manually:

Windows 安装程序通常会配置 `PATH`；使用自定义运行时位置时需要手动添加：

```powershell
$env:PATH = 'C:\my-mvs-runtime;' + $env:PATH
```

## Quick start / 快速开始

The following program initializes MVS and lists all GigE and USB cameras:

以下程序初始化 MVS，并列出所有 GigE 与 USB 相机：

```rust
use hik_mvs_sdk::{Result, Sdk};

fn main() -> Result<()> {
    let sdk = Sdk::initialize()?;
    println!("MVS SDK {}", sdk.version());

    for device in sdk.enumerate()? {
        println!(
            "{:?}: model={}, serial={}, ip={:?}",
            device.transport, device.model, device.serial, device.ip
        );
    }

    Ok(())
}
```

Run the included enumeration example:

运行仓库内置的枚举范例：

```powershell
cargo run --example enumerate
```

## Module layout / 模块结构

| Module / 模块 | Responsibility / 职责 |
|---|---|
| `hik_mvs_sdk::sdk` | SDK lifecycle, enumeration, Force IP, and Action Command / SDK 生命周期、枚举、Force IP 与 Action Command |
| `hik_mvs_sdk::camera` | Camera handles, acquisition, parameters, and feature files / 相机句柄、采集、参数与 Feature 文件 |
| `hik_mvs_sdk::frame` | Borrowed and owned frames, chunks, and output formats / 借用帧与所有权帧、Chunk 和输出格式 |
| `hik_mvs_sdk::imaging` | Image encoding, recording, rotation, and flipping / 图像编码、录像、旋转与翻转 |
| `hik_mvs_sdk::events` | Image, exception, event callbacks, and network statistics / 图像、异常、事件回调与网络统计 |
| `hik_mvs_sdk::params` | Typed GenICam integer and floating-point constraints / 类型化 GenICam 整数与浮点约束 |
| `hik_mvs_sdk::error` | Matchable `Error` and `Result` types based on `thiserror` / 基于 `thiserror` 的可匹配 `Error` 与 `Result` 类型 |

New code may import from individual modules; compatible flat re-exports are also available.

新代码可以按模块导入，同时仍提供兼容的扁平路径重导出。

```rust
use hik_mvs_sdk::error::Result;
use hik_mvs_sdk::frame::OutputFormat;
use hik_mvs_sdk::sdk::Sdk;
```

## Examples / 范例

Replace `<serial>` with the serial number printed by the `enumerate` example.

请将 `<serial>` 替换为 `enumerate` 范例输出的相机序列号。

| Example / 范例 | Capability / 能力 |
|---|---|
| `enumerate` | SDK initialization and GigE/USB enumeration / SDK 初始化与 GigE/USB 枚举 |
| `grab_one` | Acquire one frame, convert to BGR8, and save PPM / 获取单帧、转换 BGR8 并保存 PPM |
| `continuous` | Continuous acquisition and throughput/FPS statistics / 连续采集与吞吐量、FPS 统计 |
| `software_trigger` | Trigger mode, trigger source, and software trigger / 触发模式、触发源与软触发 |
| `parameters` | Generic integer and floating-point GenICam parameters / 通用整数与浮点 GenICam 参数 |
| `multi_camera` | Open and acquire from multiple cameras / 打开多台相机并采图 |
| `feature_file` | Save and load camera feature files / 保存与加载相机 Feature 文件 |
| `reconnect` | Detect disconnects and reopen by serial number / 检测断线并按序列号重新连接 |
| `image_callback` | Receive copied `OwnedFrame` values through a channel / 通过 channel 接收复制后的 `OwnedFrame` |
| `exception_callback` | Receive device exceptions through a channel / 通过 channel 接收设备异常 |
| `event_callback` | Receive exposure and acquisition events / 接收曝光与采集事件 |
| `network_stats` | GigE receive, loss, drop, and resend statistics / GigE 接收、丢包、丢帧与重发统计 |
| `user_set` | Query and optionally update camera user sets / 查询并按需更新相机用户参数组 |
| `image_formats` | Native BMP, JPEG, PNG, and TIFF encoding / 原生 BMP、JPEG、PNG 与 TIFF 编码 |
| `recording` | Native AVI recording / 原生 AVI 录像 |
| `image_transform` | BGR8 rotation and flipping / BGR8 旋转与翻转 |
| `chunk_data` | Chunk and watermark frame metadata / Chunk 与水印帧元数据 |
| `action_command` | GigE Action Command broadcast triggering / GigE Action Command 广播触发 |
| `force_ip` | GigE Force IP network configuration / GigE Force IP 网络配置 |

Common commands:

常用命令：

```powershell
cargo run --example grab_one -- <serial>
cargo run --example continuous -- <serial> 100
cargo run --example software_trigger -- <serial>
cargo run --example parameters -- <serial>
cargo run --example image_callback -- <serial> 30
cargo run --example network_stats -- <serial> 60
```

Operations that broadcast commands or change persistent device configuration require an explicit `--apply` flag.

广播命令或修改设备持久配置的操作必须显式提供 `--apply` 参数。

```powershell
cargo run --example action_command -- --apply <serial> 255.255.255.255
cargo run --example force_ip -- --apply <serial> 169.254.183.48 255.255.0.0 0.0.0.0
```

Review the target device and network parameters carefully before running these commands.

运行这些命令前，请仔细核对目标设备及网络参数。

All examples use `anyhow` to attach application-level context; the library uses `thiserror` for structured errors.

所有范例使用 `anyhow` 添加应用层上下文；库本身使用 `thiserror` 提供结构化错误。

## OpenCV integration / OpenCV 集成

The core crate does not depend on OpenCV. Request `OutputFormat::Bgr8`, then construct an OpenCV matrix from the frame width, height, stride, and byte slice in the application layer.

核心 crate 不依赖 OpenCV。上层应用可请求 `OutputFormat::Bgr8`，再通过帧的宽、高、步长和字节切片构造 OpenCV 矩阵。

Keeping OpenCV outside the binding avoids imposing a specific OpenCV version or build system on downstream Rust projects.

将 OpenCV 保留在绑定层之外，可以避免强制下游 Rust 项目使用特定 OpenCV 版本或构建系统。

## Compatibility / 兼容性

| Rust target / Rust 目标 | Status / 状态 |
|---|---|
| `x86_64-pc-windows-msvc` | Tested with the MVS x64 SDK / 已使用 MVS x64 SDK 测试 |
| `x86_64-unknown-linux-gnu` | Designed; native SDK validation still requires a licensed runner / 已设计支持；原生 SDK 验证仍需具备许可的 runner |
| `x86_64-pc-windows-gnu` | Requires a GNU-compatible MVS import library / 需要 GNU 兼容的 MVS 导入库 |
| macOS and ARM / macOS 与 ARM | Unsupported until a matching vendor SDK is available / 在厂商提供匹配 SDK 前暂不支持 |

The MSRV is Rust 1.74. Stable and newer toolchains are supported, and CMake is not part of the Cargo build.

最低支持 Rust 版本为 1.74，同时支持 stable 与更新工具链；Cargo 构建过程不依赖 CMake。

The stable interoperability boundary is a fixed-width C ABI. Vendor structs, C++ STL types, exceptions, `cv::Mat`, and Rust-owned values never cross that boundary.

稳定互操作边界采用固定宽度类型的 C ABI；厂商结构体、C++ STL 类型、异常、`cv::Mat` 与 Rust 所有权值均不会跨越该边界。

See [`COMPATIBILITY.md`](COMPATIBILITY.md) for the detailed target and ABI policy.

详细目标平台与 ABI 策略请参阅 [`COMPATIBILITY.md`](COMPATIBILITY.md)。

## Releases / 发布

Releases are published to crates.io by GitHub Actions using crates.io Trusted Publishing and short-lived OIDC credentials.

版本通过 GitHub Actions、crates.io Trusted Publishing 与短期 OIDC 凭据发布到 crates.io。

No long-lived crates.io API token is stored in the repository. A `vX.Y.Z` tag must match the version in `Cargo.toml` before publication proceeds.

仓库中不保存长期 crates.io API Token；发布前 `vX.Y.Z` 标签必须与 `Cargo.toml` 中的版本一致。

See [`RELEASING.md`](RELEASING.md) for the release checklist.

发布检查清单请参阅 [`RELEASING.md`](RELEASING.md)。

## License / 许可证

Licensed under either the Apache License, Version 2.0 or the MIT license, at your option.

本项目采用 `MIT OR Apache-2.0` 双许可证，使用者可任选其一。

This license covers only this wrapper. The Hikrobot MVS SDK and related materials remain subject to Hikrobot's own license terms.

许可证仅覆盖本封装项目；海康机器人 MVS SDK 及相关材料仍受海康机器人自身许可条款约束。
