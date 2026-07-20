# Compatibility policy

## Stable boundaries

The public interoperability boundary is `native/include/hik_sdk.h`. It uses a C ABI and
fixed-width integer types. MVS structs, C++ classes, STL containers, exceptions, `cv::Mat`,
and Rust-owned types must never cross this boundary.

Adding a function or appending fields to a versioned structure is compatible. Reordering or
removing fields, changing calling conventions, or changing ownership rules requires a major
version. Buffers returned by `hik_camera_grab` are owned by the wrapper and must be released
with `hik_frame_release` from the same library.

## Supported build targets

| Target | Status | Vendor dependency |
|---|---|---|
| `x86_64-pc-windows-msvc` | Tested | MVS x64 `.lib` and runtime DLLs |
| `x86_64-unknown-linux-gnu` | Designed; CI needs licensed SDK runner | MVS x86_64 `.so` |
| `x86_64-pc-windows-gnu` | Conditional | GNU-compatible import library required |
| macOS / ARM | Unsupported until Hikrobot ships a matching SDK | None currently known |

The Rust MSRV is 1.74 and the lockfile remains at format version 3. Both stable and newer
toolchains are supported. Unsafe code stays isolated in `src/sys.rs` and the small RAII bridge
in `src/lib.rs`.

## Release checks

Before release, test each supported target with its native MVS SDK:

```text
cargo +1.74.0 check --all-targets
cargo +stable test --all-targets
cargo +nightly check --all-targets
cargo run --example enumerate
cargo run --example grab_one -- <serial>
```

Never use a Windows host library while cross-compiling for Linux; `build.rs` selects paths from
Cargo's target metadata. CMake is deliberately not part of the Rust package workflow.
