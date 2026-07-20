//! SDK lifetime and device discovery.
//! SDK 生命周期与设备发现。
//!
//! This module groups process-level operations that do not require an already opened camera.
//! 本模块汇总不依赖某个已打开相机的进程级操作。

pub use crate::{ActionResult, DeviceInfo, Sdk, SdkVersion, Transport};
