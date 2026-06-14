// SPDX-License-Identifier: MIT OR Apache-2.0

//! G121 — search and replace must resolve caller-supplied root paths
//! against the workspace jail (not the process CWD) before constructing
//! the `ignore::WalkBuilder`.
//!
//! Pre-v0.1.19, both commands validated paths against the workspace
//! but then handed the ORIGINAL (often CWD-relative) path to the walk
//! directory engine. With `CWD != --workspace`, the walker resolved
//! the path against the CWD — producing either silent escape to a
//! CWD-resident tree, or `JailViolation` per file (waste of work, spam
//! in stderr). These three tests assert the canonicalized-path fix.

mod common;

#[test]
fn test_search_in_subdir_with_different_cwd() {
    let dir = tempfile::tempdir().expect("workspace tempdir");
    let src = dir.path().join("src");
    std::fs::create_dir(&src).expect("mkdir src");
    common::create_test_file(
        &src,
        "hello.rs",
        "fn main() {\n    println!(\"needle_g121\");\n}\n",
    );

    // outer CWD has NO src/ — if the walker resolves `src` against
    // the CWD, zero matches are returned and the test fails.
    let outer = tempfile::tempdir().expect("outer tempdir");
    let output = common::atomwrite()
        .current_dir(outer.path())
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "needle_g121",
        ])
        .arg("src")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "search with CWD != workspace must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let events = common::parse_ndjson(&output.stdout);
    let matches: Vec<_> = events.iter().filter(|e| e["type"] == "match").collect();
    assert!(
        !matches.is_empty(),
        "G121: search must find needle in workspace/src/ even when CWD != workspace; \
         events: {events:?}"
    );
    assert!(
        matches[0]["lines"]
            .as_str()
            .unwrap_or("")
            .contains("needle_g121"),
        "matched line should contain the needle"
    );
}

#[test]
fn test_replace_in_subdir_with_different_cwd() {
    let dir = tempfile::tempdir().expect("workspace tempdir");
    let src = dir.path().join("src");
    std::fs::create_dir(&src).expect("mkdir src");
    let target = src.join("rep.rs");
    std::fs::write(&target, "old_token_g121(x)\nold_token_g121(y)\n").expect("seed");

    let outer = tempfile::tempdir().expect("outer tempdir");
    let output = common::atomwrite()
        .current_dir(outer.path())
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "old_token_g121",
            "new_token_g121",
        ])
        .arg("src")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "replace with CWD != workspace must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let events = common::parse_ndjson(&output.stdout);
    let replaced: Vec<_> = events.iter().filter(|e| e["type"] == "replaced").collect();
    assert_eq!(
        replaced.len(),
        1,
        "G121: replace must modify the file under workspace/src/; events: {events:?}"
    );
    assert_eq!(replaced[0]["replacements"], 2);

    let content = std::fs::read_to_string(&target).expect("read");
    assert!(content.contains("new_token_g121"), "token must be replaced");
    assert!(
        !content.contains("old_token_g121"),
        "old token must be gone"
    );
}

#[test]
fn test_search_absolute_path_with_different_cwd() {
    let dir = tempfile::tempdir().expect("workspace tempdir");
    let sub = dir.path().join("abs");
    std::fs::create_dir(&sub).expect("mkdir abs");
    common::create_test_file(&sub, "data.txt", "abs_path_marker_g121\n");

    let outer = tempfile::tempdir().expect("outer tempdir");
    // Absolute path inside the workspace must work regardless of CWD.
    // The G121 helper canonicalizes absolute paths against the jail
    // before handing them to the walker, so this is the symmetry test
    // for the relative-path cases above.
    let abs_target = sub.to_str().expect("abs utf8");
    let output = common::atomwrite()
        .current_dir(outer.path())
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "abs_path_marker_g121",
        ])
        .arg(abs_target)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "search with absolute workspace path and outer CWD must succeed; \
         stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let events = common::parse_ndjson(&output.stdout);
    let matches: Vec<_> = events.iter().filter(|e| e["type"] == "match").collect();
    assert!(
        !matches.is_empty(),
        "G121: absolute path inside workspace must be found; events: {events:?}"
    );
}
