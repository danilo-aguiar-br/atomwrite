// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 terminal-event contract: a run that fails publishes ONE terminal
//! event (`ok: false`, with the counts and the failure detail), never a summary
//! that claims success followed by an error envelope, and a file the run could
//! not read is reported in the summary instead of staying in the log.

mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

struct Out {
    code: Option<i32>,
    events: Vec<Value>,
}

impl Out {
    /// Every line that states a failed outcome (`ok: false`).
    fn failed(&self) -> Vec<&Value> {
        self.events.iter().filter(|e| e["ok"] == false).collect()
    }
}

/// Run the binary with `stdin` piped and closed, so verbs that read it see EOF.
fn run(ws: &Path, args: &[&str], stdin: &str) -> Out {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut child = common::hermetic(Command::new(bin))
        .args([
            "-v",
            "--workspace",
            ws.to_str().expect("utf8 workspace"),
            "--no-progress",
            "--no-auto-heal",
        ])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn atomwrite");
    if let Some(mut pipe) = child.stdin.take() {
        let _ = pipe.write_all(stdin.as_bytes());
    }
    let output = child.wait_with_output().expect("wait atomwrite");
    Out {
        code: output.status.code(),
        events: common::parse_ndjson(&output.stdout),
    }
}

fn path_arg(path: &Path) -> &str {
    path.to_str().expect("utf8 path")
}

#[test]
fn search_without_match_emits_one_terminal_event() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.txt", "alpha\n");

    let out = run(dir.path(), &["search", "NOPE", path_arg(dir.path())], "");

    assert_eq!(out.code, Some(1), "events: {:?}", out.events);
    let failed = out.failed();
    assert_eq!(failed.len(), 1, "one terminal event, got {:?}", out.events);
    let terminal = failed[0];
    assert_eq!(terminal["type"], "summary");
    assert_eq!(terminal["code"], "NO_MATCHES");
    assert_eq!(terminal["exit"], 1);
    assert_eq!(terminal["error_class"], "permanent");
    assert_eq!(terminal["retryable"], false);
    assert_eq!(terminal["match_count"], 0);
    assert_eq!(terminal["files_matched"], 0);
    assert!(
        out.events.iter().all(|e| e.get("type").is_some()),
        "no standalone error envelope after the summary: {:?}",
        out.events
    );
    assert_eq!(
        out.events.last(),
        Some(terminal),
        "the verdict is the last line"
    );
}

#[test]
fn semantic_merge_conflict_emits_one_terminal_event() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "base.txt", "l1\nl2\nl3\n");
    common::create_test_file(dir.path(), "ours.txt", "l1\nl2 ours\nl3\n");
    common::create_test_file(dir.path(), "theirs.txt", "l1\nl2 THEIRS\nl3\n");
    let ws = dir.path();
    let (base, ours, theirs, merged) = (
        ws.join("base.txt"),
        ws.join("ours.txt"),
        ws.join("theirs.txt"),
        ws.join("merged.txt"),
    );

    let out = run(
        ws,
        &[
            "semantic-merge",
            "--base",
            path_arg(&base),
            "--ours",
            path_arg(&ours),
            "--theirs",
            path_arg(&theirs),
            "--output",
            path_arg(&merged),
            "--fail-on-conflict",
        ],
        "",
    );

    assert_eq!(out.code, Some(84), "events: {:?}", out.events);
    let failed = out.failed();
    assert_eq!(failed.len(), 1, "one terminal event, got {:?}", out.events);
    let terminal = failed[0];
    assert_eq!(terminal["type"], "semantic_merge");
    assert_eq!(terminal["code"], "MERGE_CONFLICT");
    assert_eq!(terminal["exit"], 84);
    assert_eq!(terminal["error_class"], "permanent");
    assert!(
        terminal["conflict_count"].as_u64().is_some_and(|n| n >= 1),
        "conflict_count must be present: {terminal}"
    );
    assert!(
        terminal["conflicts"].is_array(),
        "the regions stay on the event"
    );
    assert!(!merged.exists(), "a conflicted merge writes nothing");
}

#[test]
fn edit_loop_partial_emits_one_terminal_event() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "file.txt", "alpha\n");
    let pairs = "{\"old\":\"alpha\",\"new\":\"ALPHA\"}\n{\"old\":\"missing\",\"new\":\"X\"}\n";

    let out = run(
        dir.path(),
        &["edit-loop", "--partial", path_arg(&target)],
        pairs,
    );

    assert_eq!(out.code, Some(87), "events: {:?}", out.events);
    let failed = out.failed();
    assert_eq!(failed.len(), 1, "one terminal event, got {:?}", out.events);
    let terminal = failed[0];
    assert_eq!(terminal["type"], "result");
    assert_eq!(terminal["code"], "EDIT_LOOP_PARTIAL");
    assert_eq!(terminal["error_class"], "permanent");
    assert_eq!(terminal["pairs_total"], 2);
    assert_eq!(terminal["pairs_applied"], 1);
    assert_eq!(terminal["pairs_unmatched"], 1);
    assert_eq!(std::fs::read_to_string(&target).expect("read"), "ALPHA\n");
}

#[test]
fn edit_loop_with_one_missing_pair_fails_with_match_failed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "file.txt", "alpha\n");
    let pairs = "{\"old\":\"alpha\",\"new\":\"ALPHA\"}\n{\"old\":\"missing\",\"new\":\"X\"}\n";

    let out = run(dir.path(), &["edit-loop", path_arg(&target)], pairs);

    assert_eq!(out.code, Some(65), "events: {:?}", out.events);
    let failed = out.failed();
    assert_eq!(failed.len(), 1, "one error envelope, got {:?}", out.events);
    assert_eq!(failed[0]["code"], "MATCH_FAILED");
    assert_eq!(failed[0]["exit"], 65);
    assert_eq!(failed[0]["failed_pair_index"], 2);
    assert_eq!(failed[0]["pairs_total"], 2);
    assert_eq!(std::fs::read_to_string(&target).expect("read"), "alpha\n");
}

#[test]
fn edit_loop_with_two_missing_pairs_fails_with_match_failed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "file.txt", "alpha\n");
    let pairs =
        "{\"old\":\"missing one\",\"new\":\"X\"}\n{\"old\":\"missing two\",\"new\":\"Y\"}\n";

    let out = run(dir.path(), &["edit-loop", path_arg(&target)], pairs);

    assert_eq!(out.code, Some(65), "events: {:?}", out.events);
    let failed = out.failed();
    assert_eq!(failed.len(), 1, "one error envelope, got {:?}", out.events);
    assert_eq!(failed[0]["code"], "MATCH_FAILED");
    assert_eq!(failed[0]["exit"], 65);
    assert_eq!(failed[0]["failed_pair_index"], 1);
    assert_eq!(std::fs::read_to_string(&target).expect("read"), "alpha\n");
}

/// A refused pair (empty `old`) is not a miss: it ends the run with its own
/// error in both modes instead of being counted as an unmatched pair.
#[test]
fn edit_loop_refused_pair_is_not_counted_as_unmatched() {
    for extra in [&[][..], &["--partial"][..]] {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = common::create_test_file(dir.path(), "file.txt", "alpha\n");
        let pairs = "{\"old\":\"alpha\",\"new\":\"ALPHA\"}\n{\"old\":\"\",\"new\":\"X\"}\n";
        let mut args = vec!["edit-loop"];
        args.extend_from_slice(extra);
        args.push(path_arg(&target));

        let out = run(dir.path(), &args, pairs);

        assert_eq!(out.code, Some(65), "{extra:?}: {:?}", out.events);
        let failed = out.failed();
        assert_eq!(failed.len(), 1, "{extra:?}: {:?}", out.events);
        assert_eq!(failed[0]["code"], "INVALID_INPUT", "{extra:?}");
        assert_eq!(std::fs::read_to_string(&target).expect("read"), "alpha\n");
    }
}

#[test]
fn edit_with_a_missing_pair_fails_with_match_failed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "file.txt", "alpha\nbeta\n");

    let out = run(
        dir.path(),
        &[
            "edit",
            "--old",
            "alpha",
            "--new",
            "ALPHA",
            "--old",
            "NOPE",
            "--new",
            "X",
            path_arg(&target),
        ],
        "",
    );

    assert_eq!(out.code, Some(65), "events: {:?}", out.events);
    let failed = out.failed();
    assert_eq!(failed.len(), 1, "one error envelope, got {:?}", out.events);
    assert_eq!(failed[0]["code"], "MATCH_FAILED");
    assert_eq!(failed[0]["failed_pair_index"], 2);
}

/// `apply` SEARCH/REPLACE: a block that is not in the file is `MATCH_FAILED`
/// (with `best_candidate` when something is close), and `MATCH_AMBIGUOUS` is
/// reserved for a block that matches in more than one place.
#[test]
fn apply_search_replace_separates_a_missing_block_from_an_ambiguous_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let many: String = (1..=1000)
        .map(|i| format!("line {i} some text\n"))
        .collect();
    let many = common::create_test_file(dir.path(), "many.txt", &many);
    let small = common::create_test_file(dir.path(), "small.txt", "alpha\nbeta\n");
    let twice = common::create_test_file(dir.path(), "twice.txt", "dup line\nother\ndup line\n");
    let patch = |search: &str| format!("<<<<<<< SEARCH\n{search}\n=======\nX\n>>>>>>> REPLACE\n");
    let apply = |file: &Path, search: &str| {
        let out = run(
            dir.path(),
            &["apply", "--format", "search-replace", path_arg(file)],
            &patch(search),
        );
        let failed = out.failed().first().map(|e| (*e).clone());
        (out.code, failed.expect("a failure event"))
    };

    // Looks like many lines but is in none of them: a miss, with the closest line.
    let (code, event) = apply(&many, "line 999 NOT THERE");
    assert_eq!(code, Some(65));
    assert_eq!(event["code"], "MATCH_FAILED", "{event}");
    assert_eq!(event["best_candidate"]["line"], 999, "{event}");

    // Nothing like it anywhere.
    let (code, event) = apply(&small, "not in the file");
    assert_eq!(code, Some(65));
    assert_eq!(event["code"], "MATCH_FAILED", "{event}");

    // Two equivalent places: the refusal is ambiguity, with the count.
    let (code, event) = apply(&twice, "dup  line");
    assert_eq!(code, Some(65));
    assert_eq!(event["code"], "MATCH_AMBIGUOUS", "{event}");
    assert_eq!(event["match_count"], 2, "{event}");
    assert_eq!(
        std::fs::read_to_string(&twice).expect("read"),
        "dup line\nother\ndup line\n"
    );
}

/// `--aggregate` reduces the terminal event to a count, and the count must
/// still say the run failed (`ok`, `code`, `exit`).
#[test]
fn aggregate_keeps_the_failure_verdict_of_the_terminal_event() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.txt", "alpha\n");
    let path = path_arg(dir.path());

    let out = run(
        dir.path(),
        &["--aggregate", "type:count", "search", "NOPE", path],
        "",
    );

    assert_eq!(out.code, Some(1), "events: {:?}", out.events);
    // `--aggregate` reduces the DATA stream; the terminal summary is not data,
    // so it keeps its own shape and still carries the verdict.
    let summary = out
        .events
        .iter()
        .find(|e| e["type"] == "summary")
        .unwrap_or_else(|| panic!("no summary terminal: {:?}", out.events));
    assert_eq!(summary["ok"], false, "{summary}");
    assert_eq!(summary["code"], "NO_MATCHES", "{summary}");
    assert_eq!(summary["exit"], 1, "{summary}");
    let stream = out.events.iter().find(|e| e["type"] == "begin");
    assert!(
        stream.is_none_or(|g| g.get("ok").is_none()),
        "stream records carry no verdict: {stream:?}"
    );

    let ok = run(
        dir.path(),
        &["--aggregate", "type:count", "search", "alpha", path],
        "",
    );
    let summary = ok
        .events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary group");
    assert_eq!(summary["ok"], true, "{summary}");
}

/// `--select` must not turn every `begin` / `end` line into a bare `{}`, and
/// the terminal failure still arrives.
#[test]
fn select_drops_events_it_leaves_empty_and_keeps_the_terminal() {
    let dir = tempfile::tempdir().expect("tempdir");
    for name in ["a.txt", "b.txt", "c.txt"] {
        common::create_test_file(dir.path(), name, "alpha\n");
    }
    let path = path_arg(dir.path());

    let out = run(
        dir.path(),
        &["--select", "ok,code", "search", "NOPE", path],
        "",
    );

    assert_eq!(out.code, Some(1), "events: {:?}", out.events);
    assert!(
        out.events
            .iter()
            .all(|e| e.as_object().is_some_and(|o| !o.is_empty())),
        "no empty object may be written: {:?}",
        out.events
    );
    assert_eq!(out.events.len(), 1, "only the terminal: {:?}", out.events);
    assert_eq!(out.events[0]["ok"], false);
    assert_eq!(out.events[0]["code"], "NO_MATCHES");
}

/// Restores the mode of a path on drop so the temp dir can always be removed.
#[cfg(unix)]
struct RestoreMode(std::path::PathBuf, u32);

#[cfg(unix)]
impl Drop for RestoreMode {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(self.1));
    }
}

/// True when a file with mode 0 can still be read (running as root).
#[cfg(unix)]
fn mode_zero_is_readable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let readable = std::fs::read(path).is_ok();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).expect("chmod 644");
    readable
}

/// An unreadable file with no match elsewhere: the run exits with the error of
/// that file, not with a `NO_MATCHES` that would claim the file was searched.
#[cfg(unix)]
#[test]
fn search_unreadable_file_reports_files_failed_instead_of_no_matches() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let secret = common::create_test_file(dir.path(), "secret.txt", "needle\n");
    if mode_zero_is_readable(&secret) {
        eprintln!("skipped: running as root, mode 000 does not deny reads");
        return;
    }
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let _restore = RestoreMode(secret.clone(), 0o644);

    let out = run(dir.path(), &["search", "needle", path_arg(dir.path())], "");

    assert_ne!(out.code, Some(1), "not a NO_MATCHES run: {:?}", out.events);
    assert_eq!(
        out.code,
        Some(13),
        "PERMISSION_DENIED exit: {:?}",
        out.events
    );
    let failed = out.failed();
    assert_eq!(failed.len(), 1, "one terminal event, got {:?}", out.events);
    let terminal = failed[0];
    assert_eq!(terminal["type"], "summary");
    assert_eq!(terminal["code"], "PERMISSION_DENIED");
    assert_eq!(terminal["files_failed"], 1);
    assert_eq!(terminal["errors"][0]["code"], "PERMISSION_DENIED");
    assert!(
        terminal["errors"][0]["path"]
            .as_str()
            .is_some_and(|p| p.ends_with("secret.txt")),
        "errors[0] names the file: {terminal}"
    );
}

/// An unreadable file next to a match: the run still succeeds (exit 0) and the
/// summary says one file failed.
#[cfg(unix)]
#[test]
fn search_with_a_match_and_an_unreadable_file_exits_zero_with_files_failed() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "open.txt", "needle\n");
    let secret = common::create_test_file(dir.path(), "secret.txt", "needle\n");
    if mode_zero_is_readable(&secret) {
        eprintln!("skipped: running as root, mode 000 does not deny reads");
        return;
    }
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let _restore = RestoreMode(secret.clone(), 0o644);

    let out = run(dir.path(), &["search", "needle", path_arg(dir.path())], "");

    assert_eq!(out.code, Some(0), "events: {:?}", out.events);
    let summary = out.events.last().expect("summary");
    assert_eq!(summary["type"], "summary");
    assert_eq!(summary["ok"], true);
    assert_eq!(summary["files_matched"], 1);
    assert_eq!(summary["files_failed"], 1);
    assert_eq!(summary["errors"][0]["code"], "PERMISSION_DENIED");
}

/// A walk error is a failure too, and it does not depend on the user running
/// the test (a symlink loop is an error even for root).
#[cfg(unix)]
#[test]
fn search_symlink_loop_reports_the_walk_error_in_the_summary() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "open.txt", "needle\n");
    std::os::unix::fs::symlink(dir.path(), dir.path().join("loop")).expect("symlink");

    let out = run(
        dir.path(),
        &[
            "--follow-symlinks",
            "search",
            "needle",
            path_arg(dir.path()),
        ],
        "",
    );

    assert_eq!(out.code, Some(0), "events: {:?}", out.events);
    let summary = out.events.last().expect("summary");
    assert_eq!(summary["type"], "summary");
    assert_eq!(summary["ok"], true);
    assert!(
        summary["files_failed"].as_u64().is_some_and(|n| n >= 1),
        "the loop must be counted: {summary}"
    );
    assert_eq!(summary["errors"][0]["code"], "IO_ERROR");
}

/// A run that reached every file keeps the summary free of failure keys.
#[test]
fn search_summary_has_no_failure_keys_when_every_file_was_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.txt", "needle\n");

    let out = run(dir.path(), &["search", "needle", path_arg(dir.path())], "");

    assert_eq!(out.code, Some(0));
    let summary = out.events.last().expect("summary");
    assert_eq!(summary["type"], "summary");
    assert!(summary.get("files_failed").is_none());
    assert!(summary.get("errors").is_none());
}

/// `replace` that could not read a file reports it in the summary.
#[cfg(unix)]
#[test]
fn replace_unreadable_file_is_counted_in_files_failed() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "open.txt", "needle\n");
    let secret = common::create_test_file(dir.path(), "secret.txt", "needle\n");
    if mode_zero_is_readable(&secret) {
        eprintln!("skipped: running as root, mode 000 does not deny reads");
        return;
    }
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let _restore = RestoreMode(secret.clone(), 0o644);

    let out = run(
        dir.path(),
        &[
            "replace",
            "--no-backup",
            "needle",
            "thread",
            path_arg(dir.path()),
        ],
        "",
    );

    assert_eq!(out.code, Some(0), "events: {:?}", out.events);
    let summary = out.events.last().expect("summary");
    assert_eq!(summary["type"], "summary");
    assert_eq!(summary["files_failed"], 1);
    assert_eq!(summary["errors"][0]["code"], "PERMISSION_DENIED");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("open.txt")).expect("read"),
        "thread\n"
    );
}

#[test]
fn batch_with_a_failed_operation_ends_with_one_failed_summary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = "{\"op\":\"write\",\"target\":\"ok.txt\",\"content\":\"x\"}\n\
                    {\"op\":\"write\",\"target\":\"bad.txt\"}\n";

    let out = run(dir.path(), &["batch"], manifest);

    assert_eq!(out.code, Some(65), "events: {:?}", out.events);
    let failed = out.failed();
    assert_eq!(failed.len(), 1, "one failed verdict, got {:?}", out.events);
    let terminal = failed[0];
    assert_eq!(terminal["type"], "summary");
    assert_eq!(terminal["code"], "INVALID_INPUT");
    assert_eq!(terminal["error_class"], "permanent");
    assert_eq!(terminal["succeeded"], 1);
    assert_eq!(terminal["failed"], 1);
    assert_eq!(
        out.events.last(),
        Some(terminal),
        "nothing follows the summary"
    );
    assert!(
        out.events
            .iter()
            .all(|e| e["type"] != "summary" || e["ok"] == false),
        "no summary may claim success: {:?}",
        out.events
    );
}

#[test]
fn batch_that_succeeds_keeps_a_plain_success_summary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = "{\"op\":\"write\",\"target\":\"ok.txt\",\"content\":\"x\"}\n";

    let out = run(dir.path(), &["batch"], manifest);

    assert_eq!(out.code, Some(0), "events: {:?}", out.events);
    let summary = out.events.last().expect("summary");
    assert_eq!(summary["type"], "summary");
    assert_eq!(summary["ok"], true);
    assert!(summary.get("code").is_none(), "no failure stamp on success");
}
