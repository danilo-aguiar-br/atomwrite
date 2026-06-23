// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

use proptest::prelude::*;

proptest! {
    #[test]
    fn checksum_roundtrip_write_read(content in "\\PC{1,4096}") {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("prop.txt");

        let write_out = common::atomwrite()
            .args(["--workspace", dir.path().to_str().unwrap(), "write"])
            .arg(&target)
            .write_stdin(content.as_bytes())
            .output()
            .expect("write");

        assert!(write_out.status.success());
        let w_events = common::parse_ndjson(&write_out.stdout);
        let write_checksum = w_events[0]["checksum"].as_str().unwrap().to_owned();

        let read_out = common::atomwrite()
            .args(["--workspace", dir.path().to_str().unwrap(), "read"])
            .arg(&target)
            .output()
            .expect("read");

        assert!(read_out.status.success());
        let r_events = common::parse_ndjson(&read_out.stdout);
        let read_checksum = r_events[0]["checksum"].as_str().unwrap();

        prop_assert_eq!(write_checksum, read_checksum);
    }

    #[test]
    fn hash_deterministic(data in proptest::collection::vec(any::<u8>(), 1..4096)) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("det.txt");
        let text_data: String = data.iter().map(|b| (b % 94 + 32) as char).collect();
        std::fs::write(&path, &text_data).expect("write file");

        let out1 = common::atomwrite()
            .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
            .arg(&path)
            .output()
            .expect("hash1");

        let out2 = common::atomwrite()
            .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
            .arg(&path)
            .output()
            .expect("hash2");

        prop_assert!(out1.status.success());
        prop_assert!(out2.status.success());

        let e1 = common::parse_ndjson(&out1.stdout);
        let e2 = common::parse_ndjson(&out2.stdout);

        prop_assert!(e1[0]["checksum"].is_string());
        prop_assert_eq!(
            e1[0]["checksum"].as_str().unwrap(),
            e2[0]["checksum"].as_str().unwrap()
        );
    }
}
