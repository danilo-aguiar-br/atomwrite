// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.21 GAP-2026-012: sequential checksum drift regression tests.
//!
//! When an agent chains 5+ `edit` calls with --expect-checksum, each
//! edit changes the file's checksum. Without re-capturing the checksum
//! between calls, all but the first edit hit a STATE_DRIFT (exit 82).
//! v0.1.21 adds --allow-sequential-drift to opt out of this safety
//! check for sequential pipelines.

mod common;

/// Test 1: 5 sequential edits with the SAME initial checksum must
/// produce 4 STATE_DRIFT errors (exit 82) because the checksum
/// changes after each successful edit. This is the default
/// fail-loud behavior.
#[test]
fn sequential_drift_without_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("chain.txt");
    let initial = "version_alpha\n";
    std::fs::write(&target, initial).expect("seed");

    // Capture the initial checksum.
    let hash_out = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
        .arg(&target)
        .output()
        .expect("hash");
    let initial_checksum = common::parse_ndjson(&hash_out.stdout)[0]["value"]
        .as_str()
        .expect("value")
        .to_string();

    // Run 5 sequential edits. The pattern "version_alpha" is kept
    // in every --new value so the pattern remains findable, but the
    // content changes so the checksum also changes. Edits 2-5 reuse
    // the initial checksum and must therefore hit STATE_DRIFT.
    let new_values = [
        "version_alpha_v1",
        "version_alpha_v2",
        "version_alpha_v3",
        "version_alpha_v4",
        "version_alpha_v5",
    ];
    let mut drift_count = 0;
    let mut success_count = 0;

    for new in &new_values {
        let output = common::atomwrite()
            .args([
                "--workspace",
                dir.path().to_str().unwrap(),
                "edit",
                "--old",
                "version_alpha",
                "--new",
                new,
                "--expect-checksum",
                &initial_checksum,
            ])
            .arg(&target)
            .output()
            .expect("run edit");

        if output.status.code() == Some(82) {
            drift_count += 1;
        } else if output.status.success() {
            success_count += 1;
        }
    }

    // The first edit succeeds (checksum matches initial).
    // Edits 2-5 should fail with STATE_DRIFT (exit 82) because
    // we reused the initial_checksum.
    assert_eq!(
        success_count, 1,
        "exactly 1 edit should succeed (the first), got {success_count}"
    );
    assert_eq!(
        drift_count, 4,
        "exactly 4 edits should hit STATE_DRIFT, got {drift_count}"
    );
}

/// Test 2: 5 sequential edits that re-capture the checksum via `read`
/// before each edit must all succeed. This is the canonical correct
/// pattern for sequential agent pipelines.
#[test]
fn sequential_drift_with_recapture() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("chain_recapture.txt");
    let initial = "alpha beta gamma delta epsilon\n";
    std::fs::write(&target, initial).expect("seed");

    let labels = [
        ("alpha", "ALPHA"),
        ("beta", "BETA"),
        ("gamma", "GAMMA"),
        ("delta", "DELTA"),
        ("epsilon", "EPSILON"),
    ];

    for (old, new) in &labels {
        // Re-capture the current checksum before each edit.
        let hash_out = common::atomwrite()
            .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
            .arg(&target)
            .output()
            .expect("hash");
        let checksum = common::parse_ndjson(&hash_out.stdout)[0]["value"]
            .as_str()
            .expect("value")
            .to_string();

        let output = common::atomwrite()
            .args([
                "--workspace",
                dir.path().to_str().unwrap(),
                "edit",
                "--old",
                old,
                "--new",
                new,
                "--expect-checksum",
                &checksum,
            ])
            .arg(&target)
            .output()
            .expect("run edit");

        assert!(
            output.status.success(),
            "edit {old}->{new} should succeed with recaptured checksum, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // All labels should be uppercase now.
    let content = std::fs::read_to_string(&target).expect("read");
    assert!(content.contains("ALPHA"));
    assert!(content.contains("BETA"));
    assert!(content.contains("GAMMA"));
    assert!(content.contains("DELTA"));
    assert!(content.contains("EPSILON"));
    assert!(!content.contains("alpha"));
    assert!(!content.contains("beta"));
}

/// Test 3: 5 sequential edits with --allow-sequential-drift must
/// all succeed without re-capturing the checksum. The first edit
/// matches the initial checksum; subsequent edits skip the checksum
/// verification and emit a warning to stderr.
#[test]
fn sequential_drift_with_allow_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("chain_allow.txt");
    let initial = "alpha beta gamma delta epsilon\n";
    std::fs::write(&target, initial).expect("seed");

    // Capture only the initial checksum; reuse it for all 5 edits.
    let hash_out = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
        .arg(&target)
        .output()
        .expect("hash");
    let initial_checksum = common::parse_ndjson(&hash_out.stdout)[0]["value"]
        .as_str()
        .expect("value")
        .to_string();

    let labels = [
        ("alpha", "ALPHA"),
        ("beta", "BETA"),
        ("gamma", "GAMMA"),
        ("delta", "DELTA"),
        ("epsilon", "EPSILON"),
    ];

    for (i, (old, new)) in labels.iter().enumerate() {
        let output = common::atomwrite()
            .args([
                "--workspace",
                dir.path().to_str().unwrap(),
                "edit",
                "--allow-sequential-drift",
                "--old",
                old,
                "--new",
                new,
                "--expect-checksum",
                &initial_checksum,
            ])
            .arg(&target)
            .output()
            .expect("run edit");

        assert!(
            output.status.success(),
            "edit {} ({old}->{new}) should succeed with --allow-sequential-drift, stderr={}",
            i,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // All labels should be uppercase.
    let content = std::fs::read_to_string(&target).expect("read");
    assert!(content.contains("ALPHA"));
    assert!(content.contains("BETA"));
    assert!(content.contains("GAMMA"));
    assert!(content.contains("DELTA"));
    assert!(content.contains("EPSILON"));
    assert!(!content.contains("alpha"));
}
