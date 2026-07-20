//! Callback events and network statistics.
//! 回调事件与网络统计。
//!
//! Native callbacks copy their data before returning and deliver it through Rust channels.
//! 原生回调在返回前复制数据，再通过 Rust channel 交付。

pub use crate::{CameraEvent, ExceptionEvent, NetworkStats};
