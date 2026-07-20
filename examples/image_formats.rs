use anyhow::{Context, Result};
use hik_sdk::{ImageFormat, OutputFormat, Sdk};
use std::env;

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    camera
        .set_enum("TriggerMode", "Off")
        .context("关闭触发模式")?;
    camera.start().context("开始取流")?;
    // Preserve BayerRG8 so MVS performs Bayer interpolation and image encoding itself.
    // 保留 BayerRG8 原始格式，让 MVS 自行完成 Bayer 插值和图片编码。
    let frame = camera.grab(3000, OutputFormat::Raw).context("抓取原始帧")?;
    for (format, quality, path) in [
        (ImageFormat::Bmp, 0, "frame.bmp"),
        (ImageFormat::Jpeg, 95, "frame.jpg"),
        (ImageFormat::Png, 0, "frame.png"),
        (ImageFormat::Tiff, 0, "frame.tiff"),
    ] {
        camera
            .save_image(&frame, format, quality, path)
            .with_context(|| format!("MVS 编码 {path}"))?;
        println!("saved {path}");
    }
    Ok(())
}
