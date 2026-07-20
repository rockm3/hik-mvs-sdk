use anyhow::{Context, Result};
use hik_sdk::Sdk;
use std::{env, time::Duration};

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let seconds: u64 = env::args()
        .nth(2)
        .as_deref()
        .unwrap_or("10")
        .parse()
        .context("秒数不是整数")?;
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    let events = camera.event_channel().context("注册全部相机事件")?;
    println!("监听相机事件 {seconds} 秒（相机需在 MVS 中启用对应 EventNotification）");
    let deadline = std::time::Instant::now() + Duration::from_secs(seconds);
    while let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) {
        match events.recv_timeout(remaining) {
            Ok(event) => println!(
                "event={} id={} channel={} block={} timestamp={}",
                event.name, event.event_id, event.stream_channel, event.block_id, event.timestamp
            ),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
            Err(error) => return Err(error).context("事件 channel 已断开"),
        }
    }
    Ok(())
}
