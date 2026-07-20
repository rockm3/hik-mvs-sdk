use anyhow::{Context, Result};
use hik_mvs_sdk::{OutputFormat, Sdk};
use std::env;

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    match camera.set_bool("ChunkModeActive", true) {
        Ok(()) => println!("ChunkModeActive=On"),
        Err(error) => eprintln!("相机不支持或当前不能启用 ChunkModeActive：{error}"),
    }
    camera.set_enum("TriggerMode", "Off")?;
    camera.start()?;
    let frame = camera
        .grab(3000, OutputFormat::Raw)
        .context("抓取 Chunk 帧")?;
    let chunk = frame.chunk();
    println!(
        "frame={} gain={} exposure={} brightness={} lost_packets={} chunk={}x{} unparsed={}",
        frame.frame_number(),
        chunk.gain,
        chunk.exposure_time,
        chunk.average_brightness,
        chunk.lost_packets,
        chunk.width,
        chunk.height,
        chunk.unparsed_count
    );
    drop(frame);
    camera.stop()?;
    camera
        .set_bool("ChunkModeActive", false)
        .context("恢复 ChunkModeActive=Off")?;
    Ok(())
}
