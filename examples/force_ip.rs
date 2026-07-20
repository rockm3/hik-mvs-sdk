use anyhow::{bail, Context, Result};
use hik_mvs_sdk::Sdk;
use std::{env, net::Ipv4Addr};

fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 5 || args[0] != "--apply" {
        bail!("此操作会修改相机网络。用法：force_ip --apply <serial> <ip> <subnet> <gateway>");
    }
    let serial = &args[1];
    let ip: Ipv4Addr = args[2].parse().context("无效 IP")?;
    let subnet: Ipv4Addr = args[3].parse().context("无效子网掩码")?;
    let gateway: Ipv4Addr = args[4].parse().context("无效网关")?;
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    sdk.force_ip(serial, ip, subnet, gateway)
        .with_context(|| format!("修改相机 {serial} 的 IP"))?;
    println!("Force IP 成功：{serial} -> {ip}/{subnet}, gateway={gateway}");
    Ok(())
}
