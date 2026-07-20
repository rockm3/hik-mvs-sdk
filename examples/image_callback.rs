use anyhow::{Context, Result};
use hik_mvs_sdk::Sdk;
use std::{env, time::Instant};

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let count: usize = env::args()
        .nth(2)
        .as_deref()
        .unwrap_or("30")
        .parse()
        .context("帧数不是整数")?;
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    camera
        .set_enum("TriggerMode", "Off")
        .context("关闭触发模式")?;
    camera.set_image_node_count(8).context("设置缓存节点")?;
    let frames = camera.frame_channel().context("注册图像回调")?;
    camera.start().context("开始回调取流")?;

    let started = Instant::now();
    for index in 0..count {
        let frame = frames
            .recv_timeout(std::time::Duration::from_secs(3))
            .with_context(|| format!("等待第 {} 个回调帧", index + 1))?;
        if index % 10 == 0 {
            println!(
                "callback frame={} {}x{} format=0x{:08X} bytes={}",
                frame.frame_number,
                frame.width,
                frame.height,
                frame.pixel_format,
                frame.data.len()
            );
        }
    }
    println!(
        "回调收到 {count} 帧，{:.2} FPS",
        count as f64 / started.elapsed().as_secs_f64()
    );
    Ok(())
}
