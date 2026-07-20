use anyhow::{Context, Result};
use hik_mvs_sdk::{OutputFormat, Sdk};

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let devices = sdk.enumerate().context("枚举相机")?;
    anyhow::ensure!(!devices.is_empty(), "没有发现相机");

    for device in devices {
        let mut camera = sdk
            .open(&device.serial)
            .with_context(|| format!("打开 {} ({})", device.model, device.serial))?;
        camera
            .set_enum("TriggerMode", "Off")
            .with_context(|| format!("设置 {} 连续模式", device.serial))?;
        camera
            .start()
            .with_context(|| format!("启动 {}", device.serial))?;
        let frame = camera
            .grab(3000, OutputFormat::Raw)
            .with_context(|| format!("从 {} 抓图", device.serial))?;
        println!(
            "{} {}: frame={} {}x{} {} bytes",
            device.model,
            device.serial,
            frame.frame_number(),
            frame.width(),
            frame.height(),
            frame.data().len()
        );
    }
    Ok(())
}
