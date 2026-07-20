//! Camera handles and control.
//! 相机句柄与控制。
//!
//! [`Camera`] is tied to its [`crate::Sdk`] and shuts down recording, acquisition, and the device in order when dropped.
//! [`Camera`] 通过生命周期绑定到创建它的 [`crate::Sdk`]，并在析构时按顺序停止录像、停止采集和关闭设备。

pub use crate::Camera;
