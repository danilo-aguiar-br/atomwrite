// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3))]

    #[test]
    fn backup_retention_respects_limit(retention in 1u8..=3) {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("retained.txt");
        let writes = (retention as usize) + 2;

        for i in 0..writes {
            let content = format!("version {i}");
            let output = common::atomwrite()
                .args([
                    "--workspace", dir.path().to_str().unwrap(),
                    "write",
                    "--backup",
                    "--retention", &retention.to_string(),
                ])
                .arg(&target)
                .write_stdin(content.as_bytes())
                .output()
                .expect("write");

            prop_assert!(output.status.success(), "write {i} failed");

            std::thread::sleep(std::time::Duration::from_millis(1100));
        }

        let prefix = "retained.txt.bak.";
        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .expect("readdir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with(prefix))
            })
            .collect();

        prop_assert!(
            backups.len() <= retention as usize,
            "expected at most {} backups with retention={}, got {}",
            retention,
            retention,
            backups.len()
        );
    }

    #[test]
    fn atomic_write_preserves_content(content in "\\PC{1,2048}") {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("content.txt");

        let output = common::atomwrite()
            .args(["--workspace", dir.path().to_str().unwrap(), "write"])
            .arg(&target)
            .write_stdin(content.as_bytes())
            .output()
            .expect("write");

        prop_assert!(output.status.success());

        let read_back = std::fs::read_to_string(&target).expect("read back");
        prop_assert_eq!(content, read_back);
    }
}
