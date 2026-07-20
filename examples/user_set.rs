use anyhow::{bail, Context, Result};
use hik_mvs_sdk::Sdk;
use std::env;

fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let serial = args.first().map(String::as_str).unwrap_or("DB1856739");
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let mut camera = sdk
        .open(serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    println!(
        "current selector={} default={}",
        camera.get_enum("UserSetSelector")?,
        camera.get_enum("UserSetDefault")?
    );
    if args.len() == 1 {
        return Ok(());
    }
    if args.len() != 4 || args[1] != "--apply" {
        bail!("只读：user_set [serial]\n写入：user_set <serial> --apply <save|load|default> <UserSet1|UserSet2|...>");
    }
    match args[2].as_str() {
        "save" => camera.user_set_save(&args[3]).context("保存 UserSet")?,
        "load" => camera.user_set_load(&args[3]).context("加载 UserSet")?,
        "default" => camera
            .user_set_default(&args[3])
            .context("设置默认 UserSet")?,
        _ => bail!("操作必须是 save、load 或 default"),
    }
    println!("{} {} 完成", args[2], args[3]);
    Ok(())
}
