use anyhow::{Context, Result};
use hik_mvs_sdk::{OutputFormat, Sdk};
use std::env;

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let count: usize = env::args()
        .nth(2)
        .as_deref()
        .unwrap_or("60")
        .parse()
        .context("帧数不是整数")?;
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    camera
        .set_enum("TriggerMode", "Off")
        .context("关闭触发模式")?;
    camera.start().context("开始取流")?;
    for index in 0..count {
        camera
            .grab(3000, OutputFormat::Raw)
            .with_context(|| format!("抓取第 {} 帧", index + 1))?;
    }
    let stats = camera.network_stats().context("读取 GigE 网络统计")?;
    println!("received_bytes={} received_frames={} lost_packets={} lost_frames={} requested_resend={} resent={}",
        stats.received_bytes, stats.received_frames, stats.lost_packets, stats.lost_frames,
        stats.requested_resend_packets, stats.resent_packets);
    Ok(())
}
