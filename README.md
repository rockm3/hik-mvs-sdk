<div align="center">

# hik-mvs-sdk

**Safe, idiomatic Rust bindings for Hikrobot MVS cameras**

[简体中文](README.zh-CN.md) · English

[![Crates.io](https://img.shields.io/crates/v/hik-mvs-sdk?style=flat-square)](https://crates.io/crates/hik-mvs-sdk)
[![Documentation](https://img.shields.io/docsrs/hik-mvs-sdk?style=flat-square)](https://docs.rs/hik-mvs-sdk)
[![CI](https://img.shields.io/github/actions/workflow/status/rockm3/hik-mvs-sdk/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/rockm3/hik-mvs-sdk/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/hik-mvs-sdk?style=flat-square)](#license)
[![MSRV](https://img.shields.io/badge/MSRV-1.74-dea584?style=flat-square)](https://www.rust-lang.org/)

</div>

`hik-mvs-sdk` wraps the Hikrobot MVS SDK in a small, stable C ABI and exposes an ergonomic Rust API. It follows the capabilities of the official MVS examples without redistributing proprietary SDK files or forcing CMake on downstream projects.

> [!IMPORTANT]
> This is an independent open-source project, not an official Hikrobot product. Install and license the Hikrobot MVS SDK separately.

## Highlights

- Safe RAII lifecycle for the SDK, cameras, streams, frames, callbacks, and recordings
- GigE and USB device enumeration, open-by-serial, acquisition, and software triggering
- Generic GenICam parameters, feature files, user sets, and reconnect workflows
- Image, exception, and camera-event callbacks delivered through Rust channels
- GigE Force IP, network statistics, and Action Command broadcasting
- Pixel conversion, BMP/JPEG/PNG/TIFF encoding, rotation, flipping, chunk data, and AVI recording
- Structured library errors with `thiserror`; contextual examples with `anyhow`
- No CMake dependency and no bundled Hikrobot headers, libraries, drivers, or documentation

## Installation

```bash
cargo add hik-mvs-sdk
```

Or add the dependency manually:

```toml
[dependencies]
hik-mvs-sdk = "0.1"
```

For unreleased changes:

```toml
[dependencies]
hik-mvs-sdk = { git = "https://github.com/rockm3/hik-mvs-sdk" }
```

## Prerequisites

Install the **Hikrobot MVS Development SDK** for your target platform. The installation must provide:

- `MvCameraControl.h`
- the target platform's link library
- `MvCameraControl.dll` or `libMvCameraControl.so` at runtime

The build script discovers the SDK in this order:

| Priority | Location |
|---:|---|
| 1 | `HIK_MVS_SDK_DIR` — recommended explicit override |
| 2 | `MVS_SDK_DIR` — compatible generic override |
| 3 | `MVCAM_COMMON_RUNENV` — normally set by the MVS installer |
| 4 | common installation directories for the target platform |

Windows PowerShell:

```powershell
$env:HIK_MVS_SDK_DIR = 'D:\Program Files\MVS\Development'
cargo run --example enumerate
```

Linux:

```bash
export HIK_MVS_SDK_DIR=/opt/MVS
cargo run --example enumerate
```

The MVS installer normally configures the runtime library path. For a custom Windows runtime location:

```powershell
$env:PATH = 'C:\my-mvs-runtime;' + $env:PATH
```

## Quick start

Enumerate all connected GigE and USB cameras:

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

Run the included examples using the serial number printed by `enumerate`:

```powershell
cargo run --example enumerate
cargo run --example grab_one -- <serial>
cargo run --example continuous -- <serial> 100
cargo run --example software_trigger -- <serial>
cargo run --example image_callback -- <serial> 30
cargo run --example network_stats -- <serial> 60
```

See the complete [`examples`](examples) directory for parameters, feature files, callbacks, recording, image transforms, chunk data, multi-camera acquisition, and reconnect workflows.

## Safety-sensitive operations

Commands that broadcast packets or modify device configuration require an explicit `--apply` flag:

```powershell
# Broadcast a GigE Action Command
cargo run --example action_command -- --apply <serial> 255.255.255.255

# Change a GigE camera's network configuration
cargo run --example force_ip -- --apply <serial> 169.254.183.48 255.255.0.0 0.0.0.0
```

Verify the target device, broadcast address, IP address, subnet mask, and gateway before running these commands.

## API layout

| Module | Responsibility |
|---|---|
| [`sdk`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/sdk/) | SDK lifecycle, enumeration, Force IP, and Action Command |
| [`camera`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/camera/) | Camera handles, acquisition, parameters, and feature files |
| [`frame`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/frame/) | Borrowed and owned frames, chunks, and output formats |
| [`imaging`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/imaging/) | Image encoding, recording, rotation, and flipping |
| [`events`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/events/) | Callbacks and network statistics |
| [`params`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/params/) | Typed GenICam integer and floating-point constraints |
| [`error`](https://docs.rs/hik-mvs-sdk/latest/hik_mvs_sdk/error/) | Matchable `Error` and `Result` types |

Module imports and compatible flat re-exports are both supported:

```rust
use hik_mvs_sdk::error::Result;
use hik_mvs_sdk::frame::OutputFormat;
use hik_mvs_sdk::sdk::Sdk;
```

## OpenCV integration

The core crate deliberately does not depend on OpenCV. Request `OutputFormat::Bgr8`, then construct an OpenCV matrix from the frame width, height, stride, and byte slice in your application. This keeps the binding independent of a particular OpenCV version and build system.

## Platform support

| Rust target | Status |
|---|---|
| `x86_64-pc-windows-msvc` | Tested with the MVS x64 SDK |
| `x86_64-unknown-linux-gnu` | Designed; native validation requires a licensed SDK runner |
| `x86_64-pc-windows-gnu` | Requires a GNU-compatible MVS import library |
| macOS / ARM | Unsupported until a matching vendor SDK is available |

The MSRV is Rust 1.74. Stable and newer toolchains are supported. See [`COMPATIBILITY.md`](COMPATIBILITY.md) for the ABI and target policy.

## Design boundary

The native bridge exposes a fixed-width C ABI. Hikrobot structures, C++ STL types, exceptions, `cv::Mat`, and Rust-owned values never cross that boundary. Unsafe Rust is isolated in the low-level FFI layer, while the public API owns and releases native resources through RAII.

## Releases

Releases use [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing/) with short-lived GitHub OIDC credentials. No long-lived crates.io API token is stored in the repository. See [`RELEASING.md`](RELEASING.md) for the release checklist.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

This license covers only this wrapper. The Hikrobot MVS SDK and related materials remain subject to Hikrobot's own license terms.
