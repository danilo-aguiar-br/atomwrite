// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for `get` and `del` subcommands (v14 Tier 3, v0.1.12).

mod common;

#[test]
fn get_toml_top_level() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("Cargo.toml");
    std::fs::write(&f, "[package]\nname = \"foo\"\nversion = \"0.1.0\"\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "get",
            f.to_str().unwrap(),
            "package.version",
        ])
        .output()
        .expect("get");

    assert!(
        output.status.success(),
        "get failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let events = common::parse_ndjson(stdout.as_bytes());
    let get_event = events
        .iter()
        .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("get"));
    assert!(get_event.is_some(), "expected get event: {events:?}");
    let event = get_event.unwrap();
    assert_eq!(event.get("found").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(event.get("value").and_then(|v| v.as_str()), Some("0.1.0"));
}

#[test]
fn get_toml_missing_key_returns_not_found() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("Cargo.toml");
    std::fs::write(&f, "[package]\nname = \"x\"\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "get",
            f.to_str().unwrap(),
            "package.missing",
        ])
        .output()
        .expect("get");

    assert_eq!(
        output.status.code(),
        Some(65),
        "get on missing key should return exit 65 (INVALID_INPUT): {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let events = common::parse_ndjson(stdout.as_bytes());
    let err_event = events
        .iter()
        .find(|e| e.get("error").and_then(|v| v.as_bool()) == Some(true));
    assert!(err_event.is_some(), "should emit JSON error envelope");
    let err = err_event.unwrap();
    assert_eq!(
        err.get("code").and_then(|v| v.as_str()),
        Some("INVALID_INPUT")
    );
}

#[test]
fn get_json_dotted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("config.json");
    std::fs::write(&f, "{\"name\":\"x\",\"version\":\"1.0\"}").expect("write");

    let output = common::atomwrite()
        .args(["--workspace", workspace, "get", f.to_str().unwrap(), "name"])
        .output()
        .expect("get");

    assert!(
        output.status.success(),
        "get failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("x"), "expected value: {stdout}");
}

#[test]
fn del_toml_top_level() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("Cargo.toml");
    std::fs::write(
        &f,
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\nauthors = [\"a\"]\n",
    )
    .expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "del",
            f.to_str().unwrap(),
            "package.authors",
        ])
        .output()
        .expect("del");

    assert!(
        output.status.success(),
        "del failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert!(content.contains("name = \"x\""), "preserved: {content}");
    assert!(
        content.contains("version = \"0.1.0\""),
        "preserved: {content}"
    );
    assert!(!content.contains("authors"), "deleted: {content}");
}

#[test]
fn del_force_missing_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("Cargo.toml");
    std::fs::write(&f, "[package]\nname = \"x\"\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "del",
            f.to_str().unwrap(),
            "package.missing",
            "--force-missing",
        ])
        .output()
        .expect("del");

    assert!(
        output.status.success(),
        "del with --force-missing should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert!(content.contains("name = \"x\""), "preserved: {content}");
}

#[test]
fn del_without_force_missing_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("Cargo.toml");
    std::fs::write(&f, "[package]\nname = \"x\"\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "del",
            f.to_str().unwrap(),
            "package.missing",
        ])
        .output()
        .expect("del");

    assert!(
        !output.status.success(),
        "del on missing key without --force-missing should fail"
    );
}
