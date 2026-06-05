// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(dead_code)]

use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;

pub fn atomwrite() -> Command {
    Command::cargo_bin("atomwrite").expect("binary not found")
}

pub fn parse_ndjson(output: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(output)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("invalid NDJSON line"))
        .collect()
}

pub fn create_test_file(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create dirs");
    }
    std::fs::write(&path, content).expect("write test file");
    path
}

/// Build an NDJSON manifest string from a list of JSON operations.
///
/// Uses `serde_json::to_string` to ensure paths and other strings are
/// properly escaped. `format!` with `display()` is unsafe for paths on
/// Windows because the embedded backslashes produce invalid JSON escapes
/// (e.g. `\U`, `\r`, `\T`). Always use this helper to construct manifests
/// in tests so the suite is portable across all platforms.
pub fn manifest(ops: &[serde_json::Value]) -> String {
    ops.iter()
        .map(|op| serde_json::to_string(op).expect("serialize manifest op"))
        .collect::<Vec<_>>()
        .join("\n")
}
