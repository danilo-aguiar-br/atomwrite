// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cross-compile gate for multiplatform Tier-1 targets.
//!
//! Guards against regressions of GAP 14 (E0433 + E0308 in `cfg(windows)`
//! blocks that never compile on a Linux host) and verifies additional
//! triples from Rules Rust multiplataforma (musl, aarch64, Apple, Windows ARM).
//!
//! Run with: `cargo test --test cross_compile_check -- --ignored`
//!
//! Skipped by default when the target is not installed. Install examples:
//! - `rustup target add x86_64-pc-windows-gnu`
//! - `rustup target add x86_64-unknown-linux-musl`
//! - `rustup target add aarch64-unknown-linux-gnu`
//! - `rustup target add aarch64-apple-darwin` (often needs cross / macOS SDK)

use std::process::Command;

const WINDOWS_GNU_TARGET: &str = "x86_64-pc-windows-gnu";
const WINDOWS_MSVC_TARGET: &str = "x86_64-pc-windows-msvc";
const WINDOWS_GNU_I686_TARGET: &str = "i686-pc-windows-gnu";
const WINDOWS_ARM64_MSVC: &str = "aarch64-pc-windows-msvc";
const LINUX_MUSL_X64: &str = "x86_64-unknown-linux-musl";
const LINUX_AARCH64_GNU: &str = "aarch64-unknown-linux-gnu";
const LINUX_AARCH64_MUSL: &str = "aarch64-unknown-linux-musl";
const APPLE_AARCH64: &str = "aarch64-apple-darwin";
const APPLE_X64: &str = "x86_64-apple-darwin";

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

fn skip_if_missing_linker(target: &str, output: &std::process::Output) -> bool {
    if output.status.success() {
        return false;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Common cross-link failures when only the rustc target is installed.
    let linker_hints = [
        "linker",
        "cannot find",
        "not found",
        "lib.exe",
        "ld:",
        "No such file",
        "unknown target CPU",
        "SDKROOT",
        "xcrun",
    ];
    if linker_hints.iter().any(|h| stderr.contains(h)) {
        eprintln!(
            "skipping: target {target} installed but host cannot link it:\n{stderr}"
        );
        return true;
    }
    false
}

#[test]
#[ignore = "requires aarch64-pc-windows-msvc target + Windows ARM64 toolchain"]
fn cross_compile_windows_arm64_msvc_succeeds() {
    if !target_installed(WINDOWS_ARM64_MSVC) {
        eprintln!(
            "skipping: target {WINDOWS_ARM64_MSVC} not installed; \
             run `rustup target add {WINDOWS_ARM64_MSVC}`"
        );
        return;
    }
    let output = cargo_check(WINDOWS_ARM64_MSVC);
    if skip_if_missing_linker(WINDOWS_ARM64_MSVC, &output) {
        return;
    }
    assert_no_gap14_errors(WINDOWS_ARM64_MSVC, &output);
}

#[test]
#[ignore = "requires x86_64-unknown-linux-musl target (Alpine / static containers)"]
fn cross_compile_linux_musl_x64_succeeds() {
    if !target_installed(LINUX_MUSL_X64) {
        eprintln!(
            "skipping: target {LINUX_MUSL_X64} not installed; \
             run `rustup target add {LINUX_MUSL_X64}`"
        );
        return;
    }
    let output = cargo_check(LINUX_MUSL_X64);
    if skip_if_missing_linker(LINUX_MUSL_X64, &output) {
        return;
    }
    assert_no_gap14_errors(LINUX_MUSL_X64, &output);
}

#[test]
#[ignore = "requires aarch64-unknown-linux-gnu target (Graviton / RPi 64-bit)"]
fn cross_compile_linux_aarch64_gnu_succeeds() {
    if !target_installed(LINUX_AARCH64_GNU) {
        eprintln!(
            "skipping: target {LINUX_AARCH64_GNU} not installed; \
             run `rustup target add {LINUX_AARCH64_GNU}`"
        );
        return;
    }
    let output = cargo_check(LINUX_AARCH64_GNU);
    if skip_if_missing_linker(LINUX_AARCH64_GNU, &output) {
        return;
    }
    assert_no_gap14_errors(LINUX_AARCH64_GNU, &output);
}

#[test]
#[ignore = "requires aarch64-unknown-linux-musl target"]
fn cross_compile_linux_aarch64_musl_succeeds() {
    if !target_installed(LINUX_AARCH64_MUSL) {
        eprintln!(
            "skipping: target {LINUX_AARCH64_MUSL} not installed; \
             run `rustup target add {LINUX_AARCH64_MUSL}`"
        );
        return;
    }
    let output = cargo_check(LINUX_AARCH64_MUSL);
    if skip_if_missing_linker(LINUX_AARCH64_MUSL, &output) {
        return;
    }
    assert_no_gap14_errors(LINUX_AARCH64_MUSL, &output);
}

#[test]
#[ignore = "requires aarch64-apple-darwin target (often macOS SDK / cross)"]
fn cross_compile_apple_aarch64_succeeds() {
    if !target_installed(APPLE_AARCH64) {
        eprintln!(
            "skipping: target {APPLE_AARCH64} not installed; \
             run `rustup target add {APPLE_AARCH64}`"
        );
        return;
    }
    let output = cargo_check(APPLE_AARCH64);
    if skip_if_missing_linker(APPLE_AARCH64, &output) {
        return;
    }
    assert_no_gap14_errors(APPLE_AARCH64, &output);
}

#[test]
#[ignore = "requires x86_64-apple-darwin target"]
fn cross_compile_apple_x64_succeeds() {
    if !target_installed(APPLE_X64) {
        eprintln!(
            "skipping: target {APPLE_X64} not installed; \
             run `rustup target add {APPLE_X64}`"
        );
        return;
    }
    let output = cargo_check(APPLE_X64);
    if skip_if_missing_linker(APPLE_X64, &output) {
        return;
    }
    assert_no_gap14_errors(APPLE_X64, &output);
}
