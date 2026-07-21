<div align="center">

# hik-mvs-sdk

**面向海康机器人 MVS 相机的安全、易用 Rust 绑定**

简体中文 · [English](README.md)

[![Crates.io](https://img.shields.io/crates/v/hik-mvs-sdk?style=flat-square)](https://crates.io/crates/hik-mvs-sdk)
[![Documentation](https://img.shields.io/docsrs/hik-mvs-sdk?style=flat-square)](https://docs.rs/hik-mvs-sdk)
[![CI](https://img.shields.io/github/actions/workflow/status/rockm3/hik-mvs-sdk/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/rockm3/hik-mvs-sdk/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/hik-mvs-sdk?style=flat-square)](#许可证)
[![MSRV](https://img.shields.io/badge/MSRV-1.74-dea584?style=flat-square)](https://www.rust-lang.org/)

</div>

`hik-mvs-sdk` 通过轻量、稳定的 C ABI 封装海康机器人 MVS SDK，并对外提供符合 Rust 习惯的 API。本项目尽可能覆盖 MVS 官方范例的能力，但不会重新分发专有 SDK 文件，也不要求下游项目使用 CMake。

> [!IMPORTANT]
> 本项目是独立开源封装，并非海康机器人的官方产品。请单独安装 MVS SDK，并遵守厂商许可条款。

## 主要能力

- 使用 RAII 安全管理 SDK、相机、取流、帧、回调与录像生命周期
- 枚举 GigE 与 USB 设备、按序列号打开相机、采集图像和软触发
- 通用 GenICam 参数、Feature 文件、用户参数组与断线重连
- 通过 Rust channel 安全接收图像、异常和相机事件回调
- GigE Force IP、网络统计与 Action Command 广播
- 像素转换、BMP/JPEG/PNG/TIFF 编码、旋转、翻转、Chunk 数据与 AVI 录像
- 库使用 `thiserror` 提供结构化错误，范例使用 `anyhow` 补充操作上下文
- 无 CMake 依赖，不内置海康机器人的头文件、库、驱动或文档

## 安装

```bash
cargo add hik-mvs-sdk
```

也可以手动添加依赖：

```toml
[dependencies]
hik-mvs-sdk = "0.1"
```

如需跟踪尚未发布的改动：

```toml
[dependencies]
hik-mvs-sdk = { git = "https://github.com/rockm3/hik-mvs-sdk" }
```

## 前置要求

请安装目标平台对应的 **海康机器人 MVS Development SDK**。安装目录必须提供：

- `MvCameraControl.h`
- 目标平台的链接库
- 运行时所需的 `MvCameraControl.dll` 或 `libMvCameraControl.so`

构建脚本按照以下顺序查找 SDK：

| 优先级 | 位置 |
|---:|---|
| 1 | `HIK_MVS_SDK_DIR`，推荐使用的显式配置 |
| 2 | `MVS_SDK_DIR`，兼容的通用配置名称 |
| 3 | `MVCAM_COMMON_RUNENV`，通常由 MVS 安装程序设置 |
| 4 | 目标平台的常见安装目录 |

Windows PowerShell：

```powershell
$env:HIK_MVS_SDK_DIR = 'D:\Program Files\MVS\Development'
cargo run --example enumerate
```

Linux：

```bash
export HIK_MVS_SDK_DIR=/opt/MVS
cargo run --example enumerate
```

MVS 安装程序通常会配置运行时动态库路径。使用自定义 Windows 运行时目录时，可手动添加：

```powershell
$env:PATH = 'C:\my-mvs-runtime;' + $env:PATH
```

## 快速开始

枚举所有已连接的 GigE 与 USB 相机：

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

使用 `enumerate` 输出的序列号运行内置范例：

```powershell
cargo run --example enumerate
cargo run --example grab_one -- <serial>
cargo run --example continuous -- <serial> 100
cargo run --example software_trigger -- <serial>
cargo run --example image_callback -- <serial> 30
cargo run --example network_stats -- <serial> 60
```

完整的 [`examples`](examples) 目录还包括参数控制、Feature 文件、回调、录像、图像变换、Chunk 数据、多相机采集与断线重连范例。

## 高风险操作

广播数据包或修改设备配置的命令必须显式提供 `--apply` 参数：

```powershell
# 广播 GigE Action Command
cargo run --example action_command -- --apply <serial> 255.255.255.255

# 修改 GigE 相机网络配置
cargo run --example force_ip -- --apply <serial> 169.254.183.48 255.255.0.0 0.0.0.0
```

运行前请仔细核对目标设备、广播地址、IP 地址、子网掩码与网关。

## API 模块

| 模块 | 职责 |
|---|---|
| [`sdk`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/sdk/) | SDK 生命周期、枚举、Force IP 与 Action Command |
| [`camera`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/camera/) | 相机句柄、采集、参数与 Feature 文件 |
| [`frame`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/frame/) | 借用帧与所有权帧、Chunk 和输出格式 |
| [`imaging`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/imaging/) | 图像编码、录像、旋转与翻转 |
| [`events`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/events/) | 回调与网络统计 |
| [`params`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/params/) | 类型化 GenICam 整数与浮点约束 |
| [`error`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/error/) | 可匹配的 `Error` 与 `Result` 类型 |

既可以按模块导入，也可以使用兼容的扁平路径重导出：

```rust
use hik_mvs_sdk::error::Result;
use hik_mvs_sdk::frame::OutputFormat;
use hik_mvs_sdk::sdk::Sdk;
```

## OpenCV 集成

核心 crate 特意不依赖 OpenCV。应用层可以请求 `OutputFormat::Bgr8`，再使用帧的宽、高、步长与字节切片构造 OpenCV 矩阵。这样可以避免绑定层强制下游项目使用特定的 OpenCV 版本或构建系统。

## 平台支持

| Rust 目标 | 状态 |
|---|---|
| `x86_64-pc-windows-msvc` | 已使用 MVS x64 SDK 测试 |
| `x86_64-unknown-linux-gnu` | 已设计支持；原生验证需要具备许可 SDK 的 runner |
| `x86_64-pc-windows-gnu` | 需要 GNU 兼容的 MVS 导入库 |
| macOS / ARM | 在厂商提供匹配 SDK 前暂不支持 |

最低支持 Rust 版本为 1.74，同时支持 stable 与更新工具链。ABI 与目标平台策略详见 [`COMPATIBILITY.md`](COMPATIBILITY.md)。

## 设计边界

原生桥接层暴露使用固定宽度类型的 C ABI。海康结构体、C++ STL 类型、异常、`cv::Mat` 与 Rust 所有权值都不会跨越该边界。非安全 Rust 代码被限制在底层 FFI 中，公开 API 则通过 RAII 管理原生资源的所有权与释放。

## 发布

版本发布使用 [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing/) 与短期 GitHub OIDC 凭据。仓库中不保存长期 crates.io API Token。发布检查清单请参阅 [`RELEASING.md`](RELEASING.md)。

## 许可证

使用者可以任选以下许可证：

- Apache License 2.0（[LICENSE-APACHE](LICENSE-APACHE)）
- MIT License（[LICENSE-MIT](LICENSE-MIT)）

许可证仅覆盖本封装项目。海康机器人 MVS SDK 及相关材料仍受海康机器人自身许可条款约束。
