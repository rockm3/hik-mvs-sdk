use anyhow::{Context, Result};
use hik_sdk::{OutputFormat, Sdk};
use std::env;

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    camera
        .set_enum("TriggerMode", "On")
        .context("启用触发模式")?;
    camera
        .set_enum("TriggerSource", "Software")
        .context("选择软件触发源")?;
    camera.start().context("开始取流")?;

    camera
        .command("TriggerSoftware")
        .context("发送软件触发命令")?;
    let frame = camera
        .grab(5000, OutputFormat::Bgr8)
        .context("等待触发帧")?;
    println!(
        "软件触发成功：frame={} {}x{} bytes={}",
        frame.frame_number(),
        frame.width(),
        frame.height(),
        frame.data().len()
    );
    Ok(())
}
