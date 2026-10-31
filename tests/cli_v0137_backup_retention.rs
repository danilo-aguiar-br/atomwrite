// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 — backup retention only prunes atomwrite's own backups.
//!
//! Retention used to delete every sibling named `<file>.bak.*`, including a
//! user's `f.txt.bak.0-user-note`. It now prunes only `<file>.bak.<timestamp>`
//! with a valid timestamp.

mod common;

#[test]
fn retention_never_deletes_a_user_file_that_looks_like_a_backup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "f.txt", "one\n");
    let note = dir.path().join("f.txt.bak.0-user-note");
    std::fs::write(&note, "do not delete\n").expect("seed note");
    let ws = dir.path().to_str().unwrap();

    for (old, new) in [("one", "two"), ("two", "three"), ("three", "four")] {
        let output = common::atomwrite()
            .args([
                "--workspace",
                ws,
                "edit",
                "--keep-backup",
                "--retention",
                "1",
            ])
            .arg(&file)
            .args(["--old", old, "--new", new])
            .output()
            .expect("edit");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        // Backups are stamped to the millisecond; keep consecutive ones distinct.
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert!(note.exists(), "the user's note must survive retention");
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "do not delete\n");
    let stamped: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("f.txt.bak.2"))
        .collect();
    assert_eq!(
        stamped.len(),
        1,
        "retention 1 keeps one real backup: {stamped:?}"
    );
}
