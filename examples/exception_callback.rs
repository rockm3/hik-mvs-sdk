use anyhow::{Context, Result};
use hik_mvs_sdk::Sdk;
use std::{env, time::Duration};

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let seconds: u64 = env::args()
        .nth(2)
        .as_deref()
        .unwrap_or("30")
        .parse()
        .context("秒数不是整数")?;
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    let exceptions = camera.exception_channel().context("注册异常回调")?;
    println!("监听异常 {seconds} 秒；可在此期间拔掉相机网线测试断线事件");
    match exceptions.recv_timeout(Duration::from_secs(seconds)) {
        Ok(event) => println!("收到异常：{event:?}"),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => println!("监听期内没有异常"),
        Err(error) => return Err(error).context("异常回调 channel 已断开"),
    }
    Ok(())
}
