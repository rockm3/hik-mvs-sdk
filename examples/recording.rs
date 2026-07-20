use anyhow::{Context, Result};
use hik_sdk::{OutputFormat, Sdk};
use std::env;

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let count: usize = env::args()
        .nth(2)
        .as_deref()
        .unwrap_or("120")
        .parse()
        .context("帧数不是整数")?;
    let fps: f32 = env::args()
        .nth(3)
        .as_deref()
        .unwrap_or("60")
        .parse()
        .context("FPS 不是数字")?;
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    camera
        .set_enum("TriggerMode", "Off")
        .context("关闭触发模式")?;
    camera.start().context("开始取流")?;
    let first = camera
        .grab(3000, OutputFormat::Raw)
        .context("抓取录像首帧")?;
    camera
        .record_start(&first, fps, 8_000, "capture.avi")
        .context("启动 MVS AVI 录像")?;
    camera.record_input(&first).context("写入录像首帧")?;
    for index in 1..count {
        let frame = camera
            .grab(3000, OutputFormat::Raw)
            .with_context(|| format!("抓取录像帧 {}", index + 1))?;
        camera
            .record_input(&frame)
            .with_context(|| format!("写入录像帧 {}", index + 1))?;
    }
    camera.record_stop().context("结束录像")?;
    println!("已写入 capture.avi：{count} 帧 @ {fps} FPS");
    Ok(())
}
