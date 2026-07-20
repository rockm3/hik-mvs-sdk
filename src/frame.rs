//! Image frames and Chunk data.
//! 图像帧与 Chunk 数据。
//!
//! Synchronous acquisition returns [`Frame`], while callbacks deliver Rust-owned [`OwnedFrame`] values.
//! 同步抓图返回 [`Frame`]，回调抓图返回完全由 Rust 拥有的 [`OwnedFrame`]。

pub use crate::{ChunkMetadata, Frame, OutputFormat, OwnedFrame};
