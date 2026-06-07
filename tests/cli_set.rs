// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for the `set` subcommand (v14 Tier 3, v0.1.12).
//!
//! Covers TOML and JSON formats, dotted path navigation, value coercion
//! (bool/int/float/string), missing keys, and atomic write semantics.

mod common;

#[test]
fn set_toml_top_level_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("Cargo.toml");
    std::fs::write(
        &toml_path,
        "[package]\nname = \"foo\"\nversion = \"0.1.0\"\n",
    )
    .expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            toml_path.to_str().unwrap(),
            "package.version",
            "2.0.0",
        ])
        .output()
        .expect("set");

    assert!(
        output.status.success(),
        "set failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = std::fs::read_to_string(&toml_path).expect("read");
    assert!(content.contains("version = \"2.0.0\""), "got: {content}");
    assert!(content.contains("[package]"), "preserved header: {content}");
    assert!(
        content.contains("name = \"foo\""),
        "preserved sibling: {content}"
    );
}

#[test]
fn set_toml_creates_nested_table() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("config.toml");
    std::fs::write(&toml_path, "[package]\nname = \"x\"\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            toml_path.to_str().unwrap(),
            "dependencies.serde",
            "1.0",
        ])
        .output()
        .expect("set");

    assert!(
        output.status.success(),
        "set failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = std::fs::read_to_string(&toml_path).expect("read");
    assert!(
        content.contains("[dependencies]"),
        "created [dependencies]: {content}"
    );
    assert!(
        content.contains("serde = 1.0") || content.contains("serde = \"1.0\""),
        "set serde=1.0: {content}"
    );
}

#[test]
fn set_toml_value_coercion_int() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("Cargo.toml");
    std::fs::write(&toml_path, "[package]\nedition = \"2021\"\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            toml_path.to_str().unwrap(),
            "package.edition",
            "2024",
        ])
        .output()
        .expect("set");

    assert!(
        output.status.success(),
        "set failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&toml_path).expect("read");
    assert!(
        content.contains("edition = 2024"),
        "coerced to int: {content}"
    );
}

#[test]
fn set_toml_value_coercion_bool() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("config.toml");
    std::fs::write(&toml_path, "[feature]\nnew = false\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            toml_path.to_str().unwrap(),
            "feature.new",
            "true",
        ])
        .output()
        .expect("set");

    assert!(
        output.status.success(),
        "set failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&toml_path).expect("read");
    assert!(content.contains("new = true"), "coerced to bool: {content}");
}

#[test]
fn set_json_dotted_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let json_path = dir.path().join("config.json");
    std::fs::write(
        &json_path,
        "{\n  \"name\": \"x\",\n  \"version\": \"0.1.0\"\n}",
    )
    .expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            json_path.to_str().unwrap(),
            "version",
            "1.0.0",
        ])
        .output()
        .expect("set");

    assert!(
        output.status.success(),
        "set failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&json_path).expect("read");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
    assert_eq!(parsed["version"], "1.0.0");
    assert_eq!(parsed["name"], "x");
}

#[test]
fn set_json_creates_nested_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let json_path = dir.path().join("config.json");
    std::fs::write(&json_path, "{}").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            json_path.to_str().unwrap(),
            "deps.serde.version",
            "1.0",
        ])
        .output()
        .expect("set");

    assert!(
        output.status.success(),
        "set failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&json_path).expect("read");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
    let ver = &parsed["deps"]["serde"]["version"];
    assert!(
        ver == "1.0" || ver.as_f64() == Some(1.0),
        "expected string or 1.0 number, got: {ver}"
    );
}

#[test]
fn set_unsupported_extension_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let yaml_path = dir.path().join("config.yaml");
    std::fs::write(&yaml_path, "name: x\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            yaml_path.to_str().unwrap(),
            "name",
            "y",
        ])
        .output()
        .expect("set");

    assert!(!output.status.success(), "set yaml should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}{stdout}");
    assert!(
        combined.contains("unsupported format")
            || combined.contains("toml")
            || combined.contains("json"),
        "expected unsupported format error, got: {combined}"
    );
}

#[test]
fn set_missing_file_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("nonexistent.toml");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            toml_path.to_str().unwrap(),
            "package.version",
            "1.0.0",
        ])
        .output()
        .expect("set");

    assert!(!output.status.success(), "set on missing file should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}{stdout}");
    assert!(
        combined.contains("does not exist") || combined.contains("not found"),
        "expected file-not-found error, got: {combined}"
    );
}

#[test]
fn set_emits_set_ndjson_envelope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("Cargo.toml");
    std::fs::write(&toml_path, "[package]\nname = \"x\"\nversion = \"0.1.0\"\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            toml_path.to_str().unwrap(),
            "package.version",
            "2.0.0",
        ])
        .output()
        .expect("set");

    assert!(
        output.status.success(),
        "set failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let set_event = events
        .iter()
        .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("set"));
    assert!(
        set_event.is_some(),
        "expected set event in NDJSON, got: {events:?}"
    );
    let event = set_event.unwrap();
    assert_eq!(
        event.get("key_path").and_then(|v| v.as_str()),
        Some("package.version")
    );
    assert_eq!(
        event.get("new_value").and_then(|v| v.as_str()),
        Some("2.0.0")
    );
    assert_eq!(event.get("format").and_then(|v| v.as_str()), Some("toml"));
}
