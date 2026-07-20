use anyhow::{bail, Context, Result};
use hik_sdk::Sdk;
use std::env;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let operation = args.next().unwrap_or_else(|| "save".into());
    let serial = args.next().unwrap_or_else(|| "DB1856739".into());
    let path = args.next().unwrap_or_else(|| "camera_features.ini".into());
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    match operation.as_str() {
        "save" => camera
            .feature_save(&path)
            .with_context(|| format!("保存配置到 {path}"))?,
        "load" => camera
            .feature_load(&path)
            .with_context(|| format!("从 {path} 加载配置"))?,
        _ => bail!("用法：feature_file <save|load> [serial] [path]"),
    }
    println!("{operation} 完成：{path}");
    Ok(())
}
