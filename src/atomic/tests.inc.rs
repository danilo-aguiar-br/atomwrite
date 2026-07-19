#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_to_utc_epoch_zero() {
        assert_eq!(epoch_to_utc(0), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn epoch_to_utc_known_date() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        assert_eq!(epoch_to_utc(1704067200), (2024, 1, 1, 0, 0, 0));
    }

    #[test]
    fn atomic_write_options_default_values() {
        let opts = AtomicWriteOptions::default();
        assert!(opts.backup);
        assert_eq!(opts.retention, 5);
        assert!(!opts.preserve_timestamps);
    }

    #[test]
    fn create_backup_and_retention() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "content").unwrap();

        for _ in 0..7 {
            create_backup(&file, 5).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("test.txt.bak."))
            })
            .collect();

        assert!(
            backups.len() <= 5,
            "retention should keep at most 5 backups, got {}",
            backups.len()
        );
    }

    /// Regression (audit 2026-07-13): backup must NOT share an inode with the
    /// live file. A hardlink backup + `InPlace` write (nlink > 1) used to mutate
    /// the `.bak` in place and turn `rollback` into a silent no-op.
    #[test]
    fn create_backup_is_not_hardlink_and_survives_inplace_write() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("target.txt");
        std::fs::write(&file, b"ORIGINAL").unwrap();

        let bak = create_backup(&file, 5).unwrap();
        assert!(bak.exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let n_file = std::fs::metadata(&file).unwrap().nlink();
            let n_bak = std::fs::metadata(&bak).unwrap().nlink();
            assert_eq!(n_file, 1, "live file must have nlink=1 after backup");
            assert_eq!(n_bak, 1, "backup must have nlink=1 (not hardlinked)");
            let ino_file = std::fs::metadata(&file).unwrap().ino();
            let ino_bak = std::fs::metadata(&bak).unwrap().ino();
            assert_ne!(ino_file, ino_bak, "backup must be a distinct inode");
        }

        // Mutate the live file without creating another sidecar backup.
        let opts = AtomicWriteOptions {
            backup: false,
            keep_backup: false,
            ..AtomicWriteOptions::default()
        };
        atomic_write(&file, b"MODIFIED", &opts, dir.path()).unwrap();

        let live = std::fs::read_to_string(&file).unwrap();
        let snap = std::fs::read_to_string(&bak).unwrap();
        assert_eq!(live, "MODIFIED");
        assert_eq!(
            snap, "ORIGINAL",
            "backup content must survive subsequent write of the live file"
        );
    }

    #[test]
    fn atomic_write_updates_mtime_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "original").unwrap();

        let original_meta = std::fs::metadata(&file).unwrap();
        let original_mtime = filetime::FileTime::from_last_modification_time(&original_meta);
        std::thread::sleep(std::time::Duration::from_millis(50));

        let opts = AtomicWriteOptions::default();
        assert!(
            !opts.preserve_timestamps,
            "GAP 12 fix: default must update mtime so cargo/make detect the change"
        );

        let _ = atomic_write(&file, b"updated content", &opts, dir.path()).unwrap();

        let new_meta = std::fs::metadata(&file).unwrap();
        let new_mtime = filetime::FileTime::from_last_modification_time(&new_meta);
        assert!(
            new_mtime > original_mtime,
            "default behavior must update mtime to now (was {:?}, now {:?})",
            original_mtime,
            new_mtime
        );
    }

    #[test]
    fn atomic_write_preserves_mtime_when_opted_in() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "original").unwrap();

        let original_meta = std::fs::metadata(&file).unwrap();
        let original_mtime = filetime::FileTime::from_last_modification_time(&original_meta);
        std::thread::sleep(std::time::Duration::from_millis(50));

        let opts = AtomicWriteOptions {
            preserve_timestamps: true,
            ..Default::default()
        };

        let _ = atomic_write(&file, b"updated content", &opts, dir.path()).unwrap();

        let new_meta = std::fs::metadata(&file).unwrap();
        let new_mtime = filetime::FileTime::from_last_modification_time(&new_meta);
        assert_eq!(
            new_mtime, original_mtime,
            "preserve_timestamps=true must keep original mtime intact"
        );
    }

    #[test]
    fn write_strategy_rename_for_regular_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("regular.txt");
        std::fs::write(&file, "old").unwrap();
        let opts = AtomicWriteOptions::default();
        let r = atomic_write(&file, b"new", &opts, dir.path()).unwrap();
        assert_eq!(r.write_strategy, "rename", "nlink=1 must use rename");
        assert!(r.hardlink_nlink.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn write_strategy_inplace_for_hardlink_preserves_inode() {
        use std::os::unix::fs::MetadataExt;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("with_hardlink.txt");
        let link = dir.path().join("hardlink.txt");
        std::fs::write(&file, "shared content").unwrap();
        std::fs::hard_link(&file, &link).unwrap();

        let original_ino = std::fs::metadata(&file).unwrap().ino();
        let original_link_ino = std::fs::metadata(&link).unwrap().ino();
        assert_eq!(
            original_ino, original_link_ino,
            "pre-condition: both must point to the same inode"
        );

        let opts = AtomicWriteOptions::default();
        let r = atomic_write(&file, b"new shared content", &opts, dir.path()).unwrap();
        assert_eq!(
            r.write_strategy, "inplace",
            "G55: nlink>1 must auto-switch to InPlace"
        );
        assert_eq!(r.hardlink_nlink, Some(2));

        // Critical assertion: the inode of both files must still be the same.
        let new_file_ino = std::fs::metadata(&file).unwrap().ino();
        let new_link_ino = std::fs::metadata(&link).unwrap().ino();
        assert_eq!(
            new_file_ino, original_ino,
            "G55: file inode must be preserved (was {}, now {})",
            original_ino, new_file_ino
        );
        assert_eq!(
            new_link_ino, original_ino,
            "G55: hardlink inode must be preserved (was {}, now {})",
            original_ino, new_link_ino
        );

        // Both must read the new content (proves hardlink is still active).
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "new shared content"
        );
        assert_eq!(
            std::fs::read_to_string(&link).unwrap(),
            "new shared content"
        );
    }

    #[test]
    fn write_result_includes_strategy_and_xattr_fields() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("fields.txt");
        std::fs::write(&file, "x").unwrap();
        let opts = AtomicWriteOptions::default();
        let r = atomic_write(&file, b"y", &opts, dir.path()).unwrap();
        // write_strategy is always set (rename/inplace/copyback)
        assert!(
            matches!(r.write_strategy, "rename" | "inplace" | "copyback"),
            "write_strategy must be set, got: {}",
            r.write_strategy
        );
        // xattr_preserved <= xattr_count (we never invent xattrs)
        assert!(
            r.xattr_preserved <= r.xattr_count,
            "xattr_preserved ({}) must be <= xattr_count ({})",
            r.xattr_preserved,
            r.xattr_count
        );
        // exdev_fallback is false for the normal case
        assert!(
            !r.exdev_fallback,
            "exdev_fallback must be false in normal flow"
        );
    }

    #[test]
    fn exdev_fallback_disabled_error_when_strict_atomic() {
        // We cannot easily trigger a real EXDEV in a portable unit test, but
        // we can verify the error variant's exit code and code string.
        let err = crate::error::AtomwriteError::ExdevFallbackDisabled {
            path: std::path::PathBuf::from("/tmp/x"),
        };
        assert_eq!(err.exit_code(), 91);
        assert_eq!(err.error_code(), "EXDEV_FALLBACK_DISABLED");
        assert_eq!(
            err.error_class(),
            crate::error::ErrorClass::PreconditionFailed
        );
    }
}
