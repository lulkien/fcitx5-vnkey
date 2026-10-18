use std::process::Command;

fn pkg_config(args: &[&str]) -> Option<String> {
    let output = Command::new("pkg-config").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn main() {
    println!("cargo:rerun-if-changed=src/bridge.rs");
    println!("cargo:rerun-if-changed=cpp/facade.cpp");
    println!("cargo:rerun-if-changed=cpp/facade.h");
    println!("cargo:rerun-if-env-changed=FCITX5_ADDON_SKIP_FACADE");

    if std::env::var_os("CARGO_FEATURE_FFI").is_none() {
        return;
    }

    // The facade needs fcitx5 headers. Without them we still build the Rust
    // side of the bridge, so `cargo check --features ffi` works anywhere.
    let facade_enabled = std::env::var_os("FCITX5_ADDON_SKIP_FACADE").is_none()
        && pkg_config(&["--exists", "Fcitx5Core"]).is_some();

    let mut build = cxx_build::bridge("src/bridge.rs");
    build.std("c++20").warnings(false);

    for package in ["Fcitx5Core", "Fcitx5Config", "Fcitx5Utils"] {
        if let Some(cflags) = pkg_config(&["--cflags", package]) {
            for token in cflags.split_whitespace() {
                if let Some(dir) = token.strip_prefix("-I") {
                    build.include(dir);
                }
            }
        }
        if let Some(libs) = pkg_config(&["--libs", package]) {
            for token in libs.split_whitespace() {
                if let Some(dir) = token.strip_prefix("-L") {
                    println!("cargo:rustc-link-search=native={dir}");
                } else if let Some(name) = token.strip_prefix("-l") {
                    if package == "Fcitx5Core" {
                        println!("cargo:rustc-link-lib=dylib={name}");
                    }
                }
            }
        }
    }

    if facade_enabled {
        build.file("cpp/facade.cpp").include("cpp");
    } else {
        println!(
            "cargo:warning=fcitx5 headers not found; built the Rust bridge only. \
             Install libfcitx5core-dev (or the equivalent package) to build the addon."
        );
    }

    build.compile("vnkey_bridge");
}
