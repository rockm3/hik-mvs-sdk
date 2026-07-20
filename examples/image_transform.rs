use anyhow::{Context, Result};
use hik_mvs_sdk::{FlipDirection, ImageFormat, OutputFormat, Rotation, Sdk};
use std::env;

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    camera.set_enum("TriggerMode", "Off")?;
    camera.start()?;
    let source = camera.grab(3000, OutputFormat::Bgr8).context("抓取 BGR8")?;
    let rotated = camera
        .rotate(&source, Rotation::Degrees90)
        .context("旋转 90 度")?;
    camera
        .save_image(&rotated, ImageFormat::Png, 0, "rotated.png")
        .context("保存旋转图")?;
    let flipped = camera
        .flip(&source, FlipDirection::Horizontal)
        .context("水平翻转")?;
    camera
        .save_image(&flipped, ImageFormat::Png, 0, "flipped.png")
        .context("保存翻转图")?;
    println!(
        "source={}x{} rotated={}x{}",
        source.width(),
        source.height(),
        rotated.width(),
        rotated.height()
    );
    Ok(())
}
