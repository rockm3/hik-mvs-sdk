# Releasing / 发布

## CI without the vendor SDK / 无厂商 SDK 的 CI

GitHub-hosted runners do not contain the proprietary Hikrobot MVS Development SDK. The CI
workflow sets `HIK_MVS_SKIP_NATIVE=1` and performs Rust type checking, formatting, Clippy,
documentation, MSRV, and package-content checks without linking the native SDK.

GitHub 托管运行器不包含专有的海康机器人 MVS Development SDK。CI 工作流设置
`HIK_MVS_SKIP_NATIVE=1`，在不链接原生 SDK 的情况下执行 Rust 类型检查、格式检查、
Clippy、文档、MSRV 和发布包内容检查。

A real native build still requires MVS on the target machine. Before a release, run the complete
test suite on a machine with MVS installed.

真实的原生构建仍要求目标机器安装 MVS。发布前应在安装了 MVS 的机器上运行完整测试。

```powershell
$env:HIK_MVS_SDK_DIR = 'D:\Program Files\MVS\Development'
cargo test --all-targets
```

## First crates.io release / 首次发布

Trusted Publishing can only be configured after the crate has been published once. Sign in to
crates.io, create a narrowly scoped token for `hik-mvs-sdk`, and publish the first version locally.

Trusted Publishing 只能在 crate 首次发布后配置。登录 crates.io，为 `hik-mvs-sdk` 创建
最小权限 Token，然后在本机发布首个版本。

```powershell
cargo login
cargo publish --locked
```

Publishing is permanent. Confirm that the crate name, version, package contents, and license are
correct before running the command.

发布不可撤销。执行命令前必须确认 crate 名称、版本、发布包内容和许可证均正确。

## Trusted Publishing / 可信发布

After the first release, open the `hik-mvs-sdk` settings on crates.io and add a GitHub Trusted
Publisher with these values:

首次发布后，在 crates.io 的 `hik-mvs-sdk` 设置中添加 GitHub Trusted Publisher：

```text
Repository owner: rockm3
Repository name:  hik-mvs-sdk
Workflow file:    publish.yml
Environment:      release
```

For later releases, update `version` in `Cargo.toml`, commit it, and push a matching tag:

后续发布时，更新 `Cargo.toml` 中的 `version`，提交后推送匹配的标签：

```powershell
git tag v0.1.0
git push origin main --tags
```

The workflow verifies that the tag matches the Cargo version and obtains a short-lived crates.io
token through GitHub OIDC. No long-lived crates.io token is stored in the repository.

工作流会验证标签与 Cargo 版本一致，并通过 GitHub OIDC 获取短期 crates.io Token；仓库
中不保存长期 crates.io Token。
