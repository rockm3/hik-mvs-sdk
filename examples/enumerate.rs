use anyhow::{Context, Result};
use hik_mvs_sdk::Sdk;

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    println!("MVS SDK {}", sdk.version());
    let devices = sdk.enumerate().context("枚举相机")?;
    println!("发现 {} 台相机", devices.len());
    for (index, device) in devices.iter().enumerate() {
        println!(
            "[{index}] {:?} {}  serial={}  ip={}",
            device.transport,
            device.model,
            device.serial,
            device.ip.map_or_else(|| "-".into(), |ip| ip.to_string())
        );
    }
    Ok(())
}
