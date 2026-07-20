use anyhow::{Context, Result};
use hik_mvs_sdk::{OutputFormat, Sdk};
use std::{env, fs::File, io::Write};

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    camera
        .set_enum("TriggerMode", "Off")
        .context("关闭触发模式")?;
    camera.start().context("启动连续采集")?;
    let frame = camera
        .grab(3000, OutputFormat::Bgr8)
        .context("抓取并转换 BGR8 图像")?;

    let path = "frame.ppm";
    let mut file = File::create(path)?;
    write!(file, "P6\n{} {}\n255\n", frame.width(), frame.height())?;
    // PPM requires RGB, while MVS and OpenCV commonly use BGR.
    // PPM 要求 RGB，而 MVS 和 OpenCV 通常使用 BGR。
    for bgr in frame.data().chunks_exact(3) {
        file.write_all(&[bgr[2], bgr[1], bgr[0]])?;
    }
    println!(
        "帧 {}: {}x{}, {} bytes -> {path}",
        frame.frame_number(),
        frame.width(),
        frame.height(),
        frame.data().len()
    );
    Ok(())
}
