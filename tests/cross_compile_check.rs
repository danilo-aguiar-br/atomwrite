// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cross-compile gate for Windows targets.
//!
//! This test verifies that the project compiles for Windows targets,
//! guarding against regressions of GAP 14 (E0433 + E0308 in cfg(windows)
//! blocks that never compile on Linux CI).
//!
//! Run with: `cargo test --test cross_compile_check -- --ignored`
//!
//! Skipped by default because the Windows target may not be installed
//! on every host. Install with:
//! - `rustup target add x86_64-pc-windows-gnu` (cross-compile from Linux)
//! - `rustup target add x86_64-pc-windows-msvc` (requires MSVC toolchain)

use std::process::Command;

const WINDOWS_GNU_TARGET: &str = "x86_64-pc-windows-gnu";
const WINDOWS_MSVC_TARGET: &str = "x86_64-pc-windows-msvc";
const WINDOWS_GNU_I686_TARGET: &str = "i686-pc-windows-gnu";

fn cargo_check(target: &str) -> std::process::Output {
    Command::new("cargo")
        .args(["check", "--target", target, "--lib"])
        .env_remove("RUSTC_WRAPPER")
        .env("CARGO_BUILD_RUSTC_WRAPPER", "")
        .output()
        .expect("failed to spawn cargo check")
}

fn target_installed(target: &str) -> bool {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|line| line.trim() == target)
        })
        .unwrap_or(false)
}

fn assert_no_gap14_errors(target: &str, output: &std::process::Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "cargo check --target {target} failed:\n{stderr}"
    );
    // GAP 14 regression guard: forbid the specific E0433 / E0308 errors
    assert!(
        !stderr.contains("E0433") && !stderr.contains("E0308"),
        "GAP 14 regression detected in {target}: raw pointer / missing import errors:\n{stderr}"
    );
}

#[test]
#[ignore = "requires x86_64-pc-windows-gnu target installed; see module docs"]
fn cross_compile_windows_gnu_x64_succeeds() {
    if !target_installed(WINDOWS_GNU_TARGET) {
        eprintln!(
            "skipping: target {WINDOWS_GNU_TARGET} not installed; \
             run `rustup target add {WINDOWS_GNU_TARGET}`"
        );
        return;
    }
    let output = cargo_check(WINDOWS_GNU_TARGET);
    assert_no_gap14_errors(WINDOWS_GNU_TARGET, &output);
}

#[test]
#[ignore = "requires i686-pc-windows-gnu target installed and mingw32 toolchain"]
fn cross_compile_windows_gnu_i686_succeeds() {
    if !target_installed(WINDOWS_GNU_I686_TARGET) {
        eprintln!(
            "skipping: target {WINDOWS_GNU_I686_TARGET} not installed; \
             run `rustup target add {WINDOWS_GNU_I686_TARGET}`"
        );
        return;
    }
    let output = cargo_check(WINDOWS_GNU_I686_TARGET);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // 32-bit mingw requires i686-w64-mingw32-gcc. Skip gracefully if missing.
    if !output.status.success() && stderr.contains("i686-w64-mingw32-gcc") {
        eprintln!(
            "skipping: i686 mingw32 compiler not installed; install \
             mingw32-gcc to enable this check"
        );
        return;
    }
    assert_no_gap14_errors(WINDOWS_GNU_I686_TARGET, &output);
}

#[test]
#[ignore = "requires x86_64-pc-windows-msvc target installed and MSVC toolchain"]
fn cross_compile_windows_msvc_succeeds() {
    if !target_installed(WINDOWS_MSVC_TARGET) {
        eprintln!(
            "skipping: target {WINDOWS_MSVC_TARGET} not installed; \
             run `rustup target add {WINDOWS_MSVC_TARGET}`"
        );
        return;
    }
    let output = cargo_check(WINDOWS_MSVC_TARGET);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // MSVC requires Visual Studio Build Tools (lib.exe). Skip gracefully
    // if the linker is missing rather than reporting a hard failure.
    if !output.status.success() && stderr.contains("lib.exe") {
        eprintln!(
            "skipping: MSVC linker (lib.exe) not available; install \
             Visual Studio Build Tools to enable this check"
        );
        return;
    }
    assert_no_gap14_errors(WINDOWS_MSVC_TARGET, &output);
}
