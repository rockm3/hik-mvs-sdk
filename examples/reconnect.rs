use anyhow::{Context, Result};
use hik_mvs_sdk::Sdk;
use std::{env, thread, time::Duration};

fn main() -> Result<()> {
    let sdk = Sdk::initialize().context("初始化 MVS SDK")?;
    let serial = env::args().nth(1).unwrap_or_else(|| "DB1856739".into());
    let checks: usize = env::args()
        .nth(2)
        .as_deref()
        .unwrap_or("30")
        .parse()
        .context("检查次数不是整数")?;
    let mut camera = sdk
        .open(&serial)
        .with_context(|| format!("打开相机 {serial}"))?;
    for index in 0..checks {
        if camera.is_connected().context("检查连接")? {
            println!("[{index}] connected");
        } else {
            eprintln!("[{index}] disconnected，尝试重新枚举并打开");
            drop(camera);
            loop {
                match sdk.open(&serial) {
                    Ok(new_camera) => {
                        camera = new_camera;
                        println!("reconnected");
                        break;
                    }
                    Err(error) => {
                        eprintln!("retry: {error:#}");
                        thread::sleep(Duration::from_secs(1));
                    }
                }
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}
