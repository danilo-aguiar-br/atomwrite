// SPDX-License-Identifier: MIT OR Apache-2.0

//! Locale file parity and CLI `locale` subcommand smoke tests.
//!
//! Rules Rust i18n MVP: `en` and `pt-BR` must have identical non-empty keys.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_atomwrite"));
    c.current_dir(workspace_root());
    c
}

fn locale_keys(toml: &str) -> BTreeSet<String> {
    toml.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('_') {
                return None;
            }
            let key = line.split('=').next()?.trim();
            if key.is_empty() {
                None
            } else {
                Some(key.to_string())
            }
        })
        .collect()
}

fn locale_values_nonempty(toml: &str) -> bool {
    for line in toml.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('_') {
            continue;
        }
        let Some((_, rhs)) = line.split_once('=') else {
            continue;
        };
        let val = rhs.trim().trim_matches('"');
        if val.is_empty() {
            return false;
        }
    }
    true
}

#[test]
fn en_and_pt_br_key_parity() {
    let en = std::fs::read_to_string(workspace_root().join("locales/en.toml")).unwrap();
    let pt = std::fs::read_to_string(workspace_root().join("locales/pt-BR.toml")).unwrap();
    let en_keys = locale_keys(&en);
    let pt_keys = locale_keys(&pt);
    assert_eq!(
        en_keys, pt_keys,
        "only en: {:?}\nonly pt-BR: {:?}",
        en_keys.difference(&pt_keys).collect::<Vec<_>>(),
        pt_keys.difference(&en_keys).collect::<Vec<_>>()
    );
    assert!(!en_keys.is_empty());
    assert!(locale_values_nonempty(&en), "en has empty translation");
    assert!(locale_values_nonempty(&pt), "pt-BR has empty translation");
}

#[test]
fn locale_subcommand_emits_report() {
    let out = bin()
        .args(["--locale", "en", "locale"])
        .output()
        .expect("run atomwrite locale");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    assert_eq!(v["type"], "locale_report");
    assert_eq!(v["resolved"], "en");
    assert!(v["available"].as_array().unwrap().iter().any(|x| x == "en"));
    assert!(v["available"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "pt-BR"));
}

#[test]
fn locale_override_pt_br_on_error_suggestion() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist.txt");
    let out = bin()
        .args([
            "--locale",
            "pt-BR",
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            missing.to_str().unwrap(),
        ])
        .output()
        .expect("run");
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    // Machine fields stay English codes.
    assert_eq!(v["code"], "FILE_NOT_FOUND");
    assert!(
        v["message"]
            .as_str()
            .unwrap_or("")
            .contains("file not found"),
        "Display message must stay English: {}",
        v["message"]
    );
    let sug = v["suggestion"].as_str().unwrap_or("");
    assert!(
        sug.contains("verifique") || sug.contains("caminho") || sug.contains("arquivo"),
        "suggestion should follow pt-BR, got: {sug}"
    );
}
