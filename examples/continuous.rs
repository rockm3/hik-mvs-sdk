use anyhow::{Context, Result};
use hik_mvs_sdk::{OutputFormat, Sdk};
use std::{env, time::Instant};

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let count: usize = env::args()
        .nth(2)
        .as_deref()
        .unwrap_or("100")
        .parse()
        .context("帧数不是整数")?;
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    camera
        .set_enum("TriggerMode", "Off")
        .context("关闭触发模式")?;
    camera
        .set_image_node_count(8)
        .context("设置 SDK 图像缓存节点")?;
    camera.start().context("开始取流")?;

    let started = Instant::now();
    let mut bytes = 0usize;
    for index in 0..count {
        let frame = camera
            .grab(3000, OutputFormat::Raw)
            .with_context(|| format!("抓取第 {} 帧", index + 1))?;
        bytes += frame.data().len();
        if index % 10 == 0 {
            println!(
                "frame={} {}x{} format=0x{:08X}",
                frame.frame_number(),
                frame.width(),
                frame.height(),
                frame.pixel_format()
            );
        }
    }
    let elapsed = started.elapsed().as_secs_f64();
    println!(
        "{count} 帧，{elapsed:.3}s，{:.2} FPS，{:.2} MiB/s",
        count as f64 / elapsed,
        bytes as f64 / elapsed / 1_048_576.0
    );
    Ok(())
}
