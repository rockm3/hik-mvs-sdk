use anyhow::{bail, Context, Result};
use hik_sdk::{OutputFormat, Sdk};
use std::env;

fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() < 2 || args[0] != "--apply" {
        bail!("Action Command 会广播触发设备。用法：action_command --apply <serial> [broadcast]");
    }
    let serial = &args[1];
    let broadcast = args.get(2).map(String::as_str).unwrap_or("255.255.255.255");
    let device_key = 1;
    let group_key = 1;
    let group_mask = u32::MAX;
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let mut camera = sdk
        .open(serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    camera.set_int("ActionDeviceKey", device_key)?;
    camera.set_int("ActionGroupKey", group_key)?;
    camera.set_int("ActionGroupMask", group_mask as i64)?;
    camera.set_enum("TriggerMode", "On")?;
    camera.set_enum("TriggerSource", "Action1")?;
    camera.start()?;
    let results = sdk.action_command(
        device_key as u32,
        group_key as u32,
        group_mask,
        broadcast,
        500,
    )?;
    for result in results {
        println!(
            "device={} status=0x{:04X}",
            result.device_address, result.status
        );
    }
    let frame = camera
        .grab(5000, OutputFormat::Raw)
        .context("等待 Action Command 触发帧")?;
    println!(
        "Action frame={} {}x{}",
        frame.frame_number(),
        frame.width(),
        frame.height()
    );
    Ok(())
}
