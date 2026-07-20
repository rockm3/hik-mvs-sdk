use anyhow::{Context, Result};
use hik_mvs_sdk::Sdk;
use std::env;

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;

    let width = camera.get_int("Width").context("读取 Width")?;
    let height = camera.get_int("Height").context("读取 Height")?;
    let exposure = camera
        .get_float("ExposureTime")
        .context("读取 ExposureTime")?;
    let gain = camera.get_float("Gain").context("读取 Gain")?;
    let pixel_format = camera.get_enum("PixelFormat").context("读取 PixelFormat")?;
    println!(
        "Width  = {}  [{}..{}, step={}]",
        width.current, width.minimum, width.maximum, width.increment
    );
    println!(
        "Height = {}  [{}..{}, step={}]",
        height.current, height.minimum, height.maximum, height.increment
    );
    println!(
        "ExposureTime = {:.2}  [{:.2}..{:.2}]",
        exposure.current, exposure.minimum, exposure.maximum
    );
    println!(
        "Gain = {:.2}  [{:.2}..{:.2}]",
        gain.current, gain.minimum, gain.maximum
    );
    println!("PixelFormat = {pixel_format}");

    // Demonstrate parameter writes while preserving the user's current camera configuration.
    // 演示参数写入，同时保持当前值，不改变用户的相机配置。
    camera
        .set_float("ExposureTime", exposure.current)
        .context("回写当前曝光值")?;
    println!(
        "connected={}",
        camera.is_connected().context("检查连接状态")?
    );
    Ok(())
}
