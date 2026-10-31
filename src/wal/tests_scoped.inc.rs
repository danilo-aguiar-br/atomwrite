#[cfg(test)]
mod scoped_tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use tempfile::TempDir;

    const STALE: &str = "{\"phase\":\"started\",\"op_id\":\"a\",\"started_at_unix\":1700000000}\n\
         {\"phase\":\"committed\",\"op_id\":\"a\",\"committed_at_unix\":1700000001}\n";

    fn seed(dir: &Path, name: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(format!(".atomwrite.journal.{name}.atomwrite.journal.json"));
        std::fs::write(&path, STALE).unwrap();
        path
    }

    fn far_deadline() -> Instant {
        Instant::now() + Duration::from_secs(60)
    }

    fn scope(root: &Path, max_depth: Option<usize>) -> HealScope {
        HealScope {
            root: root.to_path_buf(),
            max_depth,
        }
    }

    /// A failed or crashed write never calls `release()`, so its journal stays.
    #[test]
    fn unreleased_guard_keeps_the_journal() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("file.txt");
        let (_op, guard) =
            journal_started_with_guard(&target, JournalOp::Write, None, blake3::hash(b"new"))
                .unwrap();
        drop(guard);
        assert!(journal_path(&target).exists());
    }

    /// A released guard removes the journal on drop.
    #[test]
    fn released_guard_removes_the_journal() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("file.txt");
        let (op, mut guard) =
            journal_started_with_guard(&target, JournalOp::Write, None, blake3::hash(b"new"))
                .unwrap();
        journal_committed(&target, &op).unwrap();
        guard.release();
        drop(guard);
        assert!(!journal_path(&target).exists());
    }

    #[test]
    fn bounded_walk_skips_build_dirs_and_honours_ignore_files() {
        let tmp = TempDir::new().unwrap();
        let keep = seed(&tmp.path().join("src"), "keep.rs");
        seed(&tmp.path().join("target/debug"), "built.rs");
        seed(&tmp.path().join(".git/objects"), "packed.rs");
        seed(&tmp.path().join("node_modules/dep"), "dep.js");
        seed(&tmp.path().join("ignored"), "ignored.rs");
        std::fs::write(tmp.path().join(".ignore"), "ignored/\n").unwrap();

        let found = walk_journal_paths_bounded(&[scope(tmp.path(), None)], far_deadline());
        assert_eq!(found, vec![keep]);
    }

    #[test]
    fn bounded_walk_depth_one_ignores_subdirectories() {
        let tmp = TempDir::new().unwrap();
        let beside = seed(tmp.path(), "beside.txt");
        seed(&tmp.path().join("sub"), "deeper.txt");

        let found = walk_journal_paths_bounded(&[scope(tmp.path(), Some(1))], far_deadline());
        assert_eq!(found, vec![beside]);
    }

    #[test]
    fn bounded_walk_stops_at_an_expired_deadline() {
        let tmp = TempDir::new().unwrap();
        seed(tmp.path(), "late.txt");

        let found = walk_journal_paths_bounded(&[scope(tmp.path(), None)], Instant::now());
        assert!(found.is_empty(), "an expired budget must cut the walk");
    }

    #[test]
    fn scoped_heal_reaps_only_inside_the_scope() {
        let tmp = TempDir::new().unwrap();
        let inside = seed(&tmp.path().join("work"), "in.txt");
        let outside = seed(&tmp.path().join("other"), "out.txt");

        let report = auto_heal_scoped(&[scope(&tmp.path().join("work"), None)], 3600, 1000);
        assert_eq!(report.removed, 1);
        assert!(!inside.exists());
        assert!(outside.exists());
    }
}
