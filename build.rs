use std::{env, path::PathBuf};

fn first_existing(candidates: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|path| path.exists())
}

fn main() {
    println!("cargo:rerun-if-env-changed=HIK_MVS_SKIP_NATIVE");

    // Hosted documentation and source-only CI do not have the proprietary MVS SDK installed.
    // 托管文档和仅检查源码的 CI 环境没有安装专有的 MVS SDK。
    if env::var_os("DOCS_RS").is_some() || env::var_os("HIK_MVS_SKIP_NATIVE").is_some() {
        return;
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Cargo did not set target OS");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("Cargo did not set target arch");
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_arch != "x86_64" {
        panic!("hik-mvs-sdk currently supports x86_64 MVS SDKs; target was {target_arch}");
    }

    let mvs = ["HIK_MVS_SDK_DIR", "MVS_SDK_DIR", "MVCAM_COMMON_RUNENV"]
        .into_iter()
        .find_map(|name| env::var_os(name).map(PathBuf::from))
        .or_else(|| match target_os.as_str() {
            "windows" => first_existing([
                PathBuf::from(r"D:\Program Files\MVS\Development"),
                PathBuf::from(r"C:\Program Files (x86)\MVS\Development"),
                PathBuf::from(r"C:\Program Files\MVS\Development"),
            ]),
            "linux" => first_existing([
                PathBuf::from("/opt/MVS"),
                PathBuf::from("/opt/MVS/Development"),
            ]),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!("MVS SDK not found for {target_os}; set HIK_MVS_SDK_DIR to its development root")
        });

    let include_dir = first_existing([mvs.join("Includes"), mvs.join("include")])
        .unwrap_or_else(|| panic!("MVS headers not found below {}", mvs.display()));
    let library_dir = match target_os.as_str() {
        "windows" => first_existing([mvs.join("Libraries/win64"), mvs.join("lib/win64")]),
        "linux" => first_existing([
            mvs.join("lib/64"),
            mvs.join("lib/x86_64"),
            mvs.join("Libraries/64"),
            mvs.join("lib"),
        ]),
        _ => panic!("MVS does not provide a supported SDK for target OS {target_os}"),
    }
    .unwrap_or_else(|| panic!("MVS library directory not found below {}", mvs.display()));

    if target_os == "windows" && target_env == "gnu" {
        println!("cargo:warning=Windows GNU needs a GNU-compatible MvCameraControl import library. MSVC is the vendor-tested Windows target.");
    }

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .include(include_dir)
        .include("native/include")
        .file("native/src/hik_mvs_sdk.cpp")
        .compile("hik_mvs_sdk_native");

    println!("cargo:rustc-link-search=native={}", library_dir.display());
    println!("cargo:rustc-link-lib=dylib=MvCameraControl");
    println!("cargo:rerun-if-env-changed=HIK_MVS_SDK_DIR");
    println!("cargo:rerun-if-env-changed=MVS_SDK_DIR");
    println!("cargo:rerun-if-env-changed=MVCAM_COMMON_RUNENV");
    println!("cargo:rerun-if-changed=native/include/hik_mvs_sdk.h");
    println!("cargo:rerun-if-changed=native/src/hik_mvs_sdk.cpp");
}
