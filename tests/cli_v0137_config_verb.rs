// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 C13 — the `config` verb: `path`, `list`, `get`, `set`, `unset`.
//!
//! Every test runs the binary with `HOME` and `XDG_*` pointed at a temporary
//! directory, on the child process only, so the real user config is never read
//! or written. The tests learn the global file from `config path`, so they do
//! not depend on where `directories` puts it on a given Unix.

#![cfg(unix)]

mod common;

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// Every key `config list` must report, in table order.
const EXPECTED_KEYS: &[&str] = &[
    // `write` reads both: they bound the stdin payload and the target, so the
    // key that changes behaviour is listed like every other live key.
    "defaults.line_ending",
    "defaults.backup",
    "delete.backup",
    "defaults.retention",
    "defaults.max_filesize",
    "fuzzy.mode",
    "fuzzy.threshold",
    "fuzzy.threshold_aggressive",
    "fuzzy.threshold_context",
    "fuzzy.threshold_jw",
    "fuzzy.soft_floor_delta",
    "fuzzy.max_content_bytes",
    "apply.max_ellipsis_lines",
    "write.confirm_large_bytes",
    "write.shrink_block_percent",
    "write.auto_rotate_max_age_secs",
    "write.content_risk_patterns",
    "watch.idle_exit_ms",
    "watch.debounce_ms",
    "log.file",
    "log.rotation",
    "storage.home",
    "locale",
    "concurrency.max_threads",
    "concurrency.ram_fraction_percent",
    "search.context",
    "search.smart_case",
    "search.max_filesize",
    "search.max_columns",
    "search.max_inflight_bytes",
    // v0.1.38: `search` reads the shape of its own answer, `read` reads its
    // byte budget and its default numbering, and every verb reads the output
    // profile.
    "search.format",
    "read.max_output_bytes",
    "read.number",
    "output.profile",
    "summary.errors_max_entries",
    "lock.acquire_timeout_ms",
    "retry.base_ms",
    "retry.max_ms",
    "retry.max_elapsed_ms",
    "extract.max_sample_rows",
    "extract.max_text_chars",
    "extract.max_sheets",
    "extract.max_slides",
];

struct Out {
    code: Option<i32>,
    events: Vec<Value>,
    stdout: String,
}

impl Out {
    fn first(&self) -> &Value {
        self.events.first().unwrap_or_else(|| {
            panic!("no NDJSON event on stdout: {:?}", self.stdout);
        })
    }
}

/// A sandbox: a fake home for the global scope and a workspace.
struct Sandbox {
    home: TempDir,
    ws: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            home: tempfile::tempdir().expect("home"),
            ws: tempfile::tempdir().expect("workspace"),
        }
    }

    fn command(&self) -> Command {
        let home = self.home.path();
        let mut cmd = common::hermetic(Command::cargo_bin("atomwrite").expect("binary"));
        cmd.env("HOME", home)
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_STATE_HOME", home.join("state"))
            .env("XDG_DATA_HOME", home.join("data"))
            .env("XDG_CACHE_HOME", home.join("cache"))
            .args(["--workspace", self.ws.path().to_str().expect("utf8")])
            .args(["--no-progress", "--no-auto-heal"]);
        cmd
    }

    fn run(&self, args: &[&str]) -> Out {
        self.run_with_stdin(args, "")
    }

    fn run_with_stdin(&self, args: &[&str], stdin: &str) -> Out {
        let output = self
            .command()
            .args(args)
            .write_stdin(stdin.as_bytes().to_vec())
            .output()
            .expect("run atomwrite");
        Out {
            code: output.status.code(),
            events: common::parse_ndjson(&output.stdout),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        }
    }

    /// Global config file as `config path` reports it right now.
    fn global_file(&self) -> PathBuf {
        let out = self.run(&["config", "path", "--scope", "global"]);
        assert_eq!(out.code, Some(0), "{}", out.stdout);
        PathBuf::from(out.first()["config_path"].as_str().expect("config_path"))
    }

    fn workspace_file(&self) -> PathBuf {
        self.ws.path().join(".atomwrite.toml")
    }

    fn ws_path(&self, name: &str) -> String {
        self.ws
            .path()
            .join(name)
            .to_str()
            .expect("utf8")
            .to_string()
    }
}

fn names_in(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("read_dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn set_global_writes_atomically_owner_only_and_get_reports_value_and_origin() {
    let sb = Sandbox::new();
    let file = sb.global_file();

    let set = sb.run(&[
        "config",
        "set",
        "fuzzy.max_content_bytes",
        "1000000",
        "--scope",
        "global",
    ]);
    assert_eq!(set.code, Some(0), "{}", set.stdout);
    let event = set.first();
    assert_eq!(event["type"], "config");
    assert_eq!(event["action"], "set");
    assert_eq!(event["ok"], true);
    assert_eq!(event["changed"], true);
    assert_eq!(event["value"], 1_000_000);
    assert_eq!(event["target_source"], "argv");
    assert_eq!(
        PathBuf::from(event["target_resolved"][0].as_str().expect("target")),
        file
    );

    let mode = std::fs::metadata(&file).expect("meta").permissions().mode();
    assert_eq!(mode & 0o777, 0o600, "global config must be owner-only");
    let text = std::fs::read_to_string(&file).expect("read config");
    assert!(text.contains("max_content_bytes = 1000000"), "{text}");
    assert_eq!(
        names_in(file.parent().expect("parent")),
        vec!["config.toml".to_string()],
        "tempfile, sidecar and backup must not survive the commit"
    );

    let get = sb.run(&["config", "get", "fuzzy.max_content_bytes"]);
    assert_eq!(get.code, Some(0), "{}", get.stdout);
    assert_eq!(get.first()["value"], 1_000_000);
    assert_eq!(get.first()["origin"], "global");
    assert!(get.first()["default"].as_u64().expect("default") > 1_000_000);
}

#[test]
fn verbs_that_write_or_name_a_file_exit_2_without_scope_and_touch_nothing() {
    let sb = Sandbox::new();
    let before = names_in(sb.ws.path());
    for args in [
        vec!["config", "set", "fuzzy.threshold", "0.9"],
        vec!["config", "unset", "fuzzy.threshold"],
        vec!["config", "path"],
    ] {
        let out = sb.run(&args);
        assert_eq!(out.code, Some(2), "{args:?}: {}", out.stdout);
    }
    assert_eq!(names_in(sb.ws.path()), before, "workspace untouched");
    assert!(
        !sb.home.path().join("config").exists(),
        "the global config directory must not be created"
    );
}

#[test]
fn unknown_key_exits_65_and_lists_the_valid_keys() {
    let sb = Sandbox::new();
    for args in [
        vec!["-v", "config", "set", "bogus.key", "1", "--scope", "global"],
        vec!["-v", "config", "get", "bogus.key"],
        vec!["-v", "config", "unset", "bogus.key", "--scope", "global"],
    ] {
        let out = sb.run(&args);
        assert_eq!(out.code, Some(65), "{args:?}: {}", out.stdout);
        let message = out.first()["message"].as_str().expect("message");
        assert!(message.contains("valid keys"), "{message}");
        for key in EXPECTED_KEYS {
            assert!(message.contains(key), "{key} missing from {message}");
        }
    }
    assert!(!sb.home.path().join("config").exists());
}

#[test]
fn invalid_values_exit_65_and_write_nothing() {
    let sb = Sandbox::new();
    for (key, value) in [
        ("fuzzy.max_content_bytes", "0"),
        ("fuzzy.max_content_bytes", "-1"),
        ("fuzzy.max_content_bytes", "many"),
        ("fuzzy.threshold", "1.5"),
        ("fuzzy.mode", "sloppy"),
        ("write.confirm_large_bytes", "0"),
        ("lock.acquire_timeout_ms", "5"),
        ("concurrency.ram_fraction_percent", "101"),
        ("concurrency.max_threads", "0"),
        ("concurrency.max_threads", "1000"),
        ("lock.acquire_timeout_ms", "99999999999"),
        ("retry.base_ms", "99999999"),
        ("retry.max_elapsed_ms", "99999999999"),
        ("search.max_inflight_bytes", "99999999999"),
        ("extract.max_text_chars", "99999999999"),
        ("extract.max_sample_rows", "99999999"),
        ("search.max_inflight_bytes", "1024"),
        ("retry.base_ms", "0"),
        ("extract.max_sheets", "0"),
        ("storage.home", "relative/dir"),
        ("locale", "klingon"),
    ] {
        let out = sb.run(&["config", "set", key, value, "--scope", "global"]);
        assert_eq!(out.code, Some(65), "{key}={value}: {}", out.stdout);
    }
    let bool_key = sb.run(&["config", "set", "log.file", "yes", "--scope", "global"]);
    assert_eq!(bool_key.code, Some(65), "{}", bool_key.stdout);
    assert!(!sb.home.path().join("config").exists());
}

#[test]
fn retry_keys_are_checked_against_each_other_when_set_and_when_loaded() {
    let sb = Sandbox::new();
    let set = |key: &str, value: &str| {
        sb.run(&["-v", "config", "set", key, value, "--scope", "workspace"])
    };
    // 1000 is above the 400 ms the cap would still have at run time.
    let lone_base = set("retry.base_ms", "1000");
    assert_eq!(lone_base.code, Some(65), "{}", lone_base.stdout);
    assert!(
        !sb.workspace_file().exists(),
        "a rejected set writes nothing"
    );

    assert_eq!(set("retry.base_ms", "10").code, Some(0));
    let cap_below_base = set("retry.max_ms", "5");
    assert_eq!(cap_below_base.code, Some(65), "{}", cap_below_base.stdout);
    let message = cap_below_base.first()["message"].as_str().expect("message");
    assert!(
        message.contains("base_ms") && message.contains("max_ms"),
        "{message}"
    );
    assert!(
        !std::fs::read_to_string(sb.workspace_file())
            .expect("read")
            .contains("max_ms"),
        "the rejected key is not written"
    );
    assert_eq!(set("retry.max_ms", "10").code, Some(0), "equal is coherent");

    // A hand-edited file is checked at startup with the same rule.
    std::fs::write(sb.workspace_file(), "[retry]\nbase_ms = 10\nmax_ms = 5\n").expect("seed");
    let a = sb.ws_path("a.txt");
    std::fs::write(&a, "hello\n").expect("write");
    let read = sb.run(&["read", &a]);
    assert_eq!(read.code, Some(65), "{}", read.stdout);
}

#[test]
fn validation_message_stays_english_and_only_the_hint_is_localized() {
    let sb = Sandbox::new();
    let cases = [
        // (key, value, marker in the English message, hint per locale)
        (
            "concurrency.max_threads",
            "1000",
            "is out of range; must be",
            [("en", "outside the range"), ("pt-BR", "fora da faixa")],
        ),
        (
            "retry.base_ms",
            "1000",
            "must not exceed",
            [("en", "contradict"), ("pt-BR", "se contradizem")],
        ),
    ];
    for (key, value, marker, hints) in cases {
        for (locale, hint) in hints {
            let out = sb.run(&[
                "-v",
                "--locale",
                locale,
                "config",
                "set",
                key,
                value,
                "--scope",
                "workspace",
            ]);
            assert_eq!(out.code, Some(65), "{key}: {}", out.stdout);
            let message = out.first()["message"].as_str().expect("message");
            assert!(
                message.starts_with("invalid input: config ") && message.contains(marker),
                "[{locale}] the message is English whatever the locale: {message}"
            );
            let suggestion = out.first()["suggestion"].as_str().expect("suggestion");
            assert!(
                suggestion.contains(hint),
                "[{locale}] expected `{hint}` in {suggestion}"
            );
        }
    }
}

#[test]
fn global_only_keys_are_rejected_in_the_workspace_scope() {
    let sb = Sandbox::new();
    // `log.*` is no longer global-only: v0.1.37 makes the log sink read the
    // workspace `.atomwrite.toml` too, so refusing the write here while
    // honouring the key there would be the split this release removed.
    for (key, value) in [("storage.home", "/tmp"), ("locale", "en")] {
        let out = sb.run(&["-v", "config", "set", key, value, "--scope", "workspace"]);
        assert_eq!(out.code, Some(65), "{key}: {}", out.stdout);
        let message = out.first()["message"].as_str().expect("message");
        assert!(message.contains("global-only"), "{message}");
    }
    for (key, value) in [("log.file", "true"), ("log.rotation", "hourly")] {
        let out = sb.run(&["config", "set", key, value, "--scope", "workspace"]);
        assert_eq!(out.code, Some(0), "{key}: {}", out.stdout);
    }
}

/// A file of 3000 bytes the fuzzy cascade must scan for a miss.
fn big_file(sb: &Sandbox) -> String {
    std::fs::write(sb.ws_path("big.txt"), "a".repeat(3000)).expect("write big");
    sb.ws_path("big.txt")
}

#[test]
fn the_value_changes_what_edit_does() {
    let sb = Sandbox::new();
    let big = big_file(&sb);
    let edit = |sb: &Sandbox| {
        sb.run(&[
            "-v", "edit", &big, "--old", "zzzz", "--new", "y", "--fuzzy", "auto",
        ])
    };

    let before = edit(&sb);
    assert_eq!(before.code, Some(65), "{}", before.stdout);
    assert!(
        !before.stdout.contains("too large for the fuzzy cascade"),
        "default ceiling must not trip on 3000 bytes: {}",
        before.stdout
    );

    let set = sb.run(&[
        "config",
        "set",
        "fuzzy.max_content_bytes",
        "1000",
        "--scope",
        "global",
    ]);
    assert_eq!(set.code, Some(0), "{}", set.stdout);
    let capped = edit(&sb);
    assert_eq!(capped.code, Some(65), "{}", capped.stdout);
    assert!(
        capped.stdout.contains("too large for the fuzzy cascade")
            && capped.stdout.contains("max_content_bytes"),
        "the lowered ceiling must reject the file with a hint: {}",
        capped.stdout
    );

    let unset = sb.run(&[
        "config",
        "unset",
        "fuzzy.max_content_bytes",
        "--scope",
        "global",
    ]);
    assert_eq!(unset.code, Some(0), "{}", unset.stdout);
    let restored = edit(&sb);
    assert!(
        !restored.stdout.contains("too large for the fuzzy cascade"),
        "unset must restore the default ceiling: {}",
        restored.stdout
    );
}

#[test]
fn a_broken_config_does_not_stop_the_config_verb() {
    let sb = Sandbox::new();
    let read_target = sb.ws_path("a.txt");
    std::fs::write(&read_target, "hello\n").expect("write a.txt");
    std::fs::write(
        sb.workspace_file(),
        "# keep this note\n[defaults]\nbogus = 1\n",
    )
    .expect("write broken workspace config");

    let read = sb.run(&["read", &read_target]);
    assert_eq!(read.code, Some(78), "other verbs still reject the file");

    let set = sb.run(&[
        "config",
        "set",
        "fuzzy.threshold",
        "0.9",
        "--scope",
        "workspace",
    ]);
    assert_eq!(set.code, Some(0), "{}", set.stdout);
    let warnings = set.first()["warnings"].as_array().expect("warnings");
    assert!(!warnings.is_empty(), "the untouched damage is reported");
    let text = std::fs::read_to_string(sb.workspace_file()).expect("read");
    assert!(text.contains("# keep this note"), "{text}");
    assert!(
        text.contains("bogus = 1"),
        "damage is left as found: {text}"
    );
    assert!(text.contains("threshold = 0.9"), "{text}");

    let get = sb.run(&["config", "get", "fuzzy.threshold"]);
    assert_eq!(get.code, Some(0), "{}", get.stdout);
    assert_eq!(get.first()["value"], 0.9);
    assert_eq!(get.first()["origin"], "workspace");

    let path = sb.run(&["config", "path", "--scope", "workspace"]);
    assert_eq!(path.code, Some(0), "{}", path.stdout);

    let bad_value = sb.run(&[
        "config",
        "set",
        "fuzzy.mode",
        "sloppy",
        "--scope",
        "workspace",
    ]);
    assert_eq!(bad_value.code, Some(65), "the touched key is still checked");
}

#[test]
fn a_file_that_is_not_toml_is_read_with_a_warning_and_never_rewritten() {
    let sb = Sandbox::new();
    std::fs::write(sb.workspace_file(), "not valid { toml").expect("write");

    let get = sb.run(&["config", "get", "fuzzy.threshold"]);
    assert_eq!(get.code, Some(0), "{}", get.stdout);
    assert_eq!(get.first()["origin"], "default");
    assert!(
        !get.first()["warnings"]
            .as_array()
            .expect("warnings")
            .is_empty()
    );

    let set = sb.run(&[
        "config",
        "set",
        "fuzzy.threshold",
        "0.9",
        "--scope",
        "workspace",
    ]);
    assert_eq!(set.code, Some(78), "{}", set.stdout);
    assert_eq!(
        std::fs::read_to_string(sb.workspace_file()).expect("read"),
        "not valid { toml",
        "the file must be left exactly as found"
    );
}

#[test]
fn set_get_unset_round_trip_keeps_comments() {
    let sb = Sandbox::new();
    std::fs::write(
        sb.workspace_file(),
        "# keep me\n[fuzzy]\n# why\nthreshold = 0.9 # inline\n",
    )
    .expect("seed");

    let set = sb.run(&[
        "config",
        "set",
        "retry.base_ms",
        "25",
        "--scope",
        "workspace",
    ]);
    assert_eq!(set.code, Some(0), "{}", set.stdout);
    assert_eq!(set.first()["changed"], true);
    assert!(set.first().get("old_value").is_none(), "{}", set.stdout);

    let get = sb.run(&["config", "get", "retry.base_ms"]);
    assert_eq!(get.first()["value"], 25);
    assert_eq!(get.first()["origin"], "workspace");

    let unset = sb.run(&["config", "unset", "retry.base_ms", "--scope", "workspace"]);
    assert_eq!(unset.code, Some(0), "{}", unset.stdout);
    assert_eq!(unset.first()["changed"], true);
    assert_eq!(unset.first()["old_value"], 25);

    let after = sb.run(&["config", "get", "retry.base_ms"]);
    assert_eq!(after.first()["origin"], "default");
    let text = std::fs::read_to_string(sb.workspace_file()).expect("read");
    assert!(!text.contains("base_ms"), "{text}");
    assert!(
        !text.contains("[retry]"),
        "an emptied section goes away: {text}"
    );
    for kept in ["# keep me", "# why", "# inline", "threshold = 0.9"] {
        assert!(text.contains(kept), "{kept} lost: {text}");
    }

    let again = sb.run(&["config", "unset", "retry.base_ms", "--scope", "workspace"]);
    assert_eq!(again.code, Some(0), "{}", again.stdout);
    assert_eq!(again.first()["changed"], false);
}

#[test]
fn workspace_wins_over_global_key_by_key() {
    let sb = Sandbox::new();
    let set = |key: &str, value: &str, scope: &str| {
        let out = sb.run(&["config", "set", key, value, "--scope", scope]);
        assert_eq!(out.code, Some(0), "{key}: {}", out.stdout);
    };
    set("fuzzy.max_content_bytes", "111111", "global");
    set("fuzzy.threshold", "0.8", "global");
    set("fuzzy.max_content_bytes", "222222", "workspace");

    let max = sb.run(&["config", "get", "fuzzy.max_content_bytes"]);
    assert_eq!(max.first()["value"], 222_222);
    assert_eq!(max.first()["origin"], "workspace");
    let threshold = sb.run(&["config", "get", "fuzzy.threshold"]);
    assert_eq!(threshold.first()["value"], 0.8);
    assert_eq!(
        threshold.first()["origin"],
        "global",
        "a workspace file that sets one key keeps the other global keys"
    );

    let a = sb.ws_path("a.txt");
    std::fs::write(&a, "hello\n").expect("write");
    let read = sb.run(&["read", &a]);
    assert_eq!(read.code, Some(0), "merged config loads: {}", read.stdout);
}

#[test]
fn list_reports_every_key_once_with_type_scopes_and_no_null() {
    let sb = Sandbox::new();
    let list = sb.run(&["config", "list"]);
    assert_eq!(list.code, Some(0), "{}", list.stdout);
    assert_eq!(list.events.len(), 1, "one envelope");
    let keys = list.first()["keys"].as_array().expect("keys");
    let names: Vec<&str> = keys
        .iter()
        .map(|k| k["key"].as_str().expect("key"))
        .collect();
    assert_eq!(names, EXPECTED_KEYS);
    assert!(
        names.contains(&"defaults.line_ending") && names.contains(&"defaults.max_filesize"),
        "a key `write` reads is listed, because an unlisted key is a promise the product does not keep"
    );
    for entry in keys {
        let object = entry.as_object().expect("object");
        assert!(object.values().all(|v| !v.is_null()), "{entry}");
        assert!(matches!(
            entry["kind"].as_str(),
            Some("integer" | "float" | "boolean" | "string" | "list")
        ));
        assert!(entry["scopes"].as_array().is_some_and(|s| !s.is_empty()));
    }
    let log = keys
        .iter()
        .find(|k| k["key"] == "log.file")
        .expect("log.file");
    // v0.1.37: the log sink reads both layers, so `log.file` reports both scopes.
    assert_eq!(log["scopes"], serde_json::json!(["global", "workspace"]));
}

#[test]
fn storage_home_is_written_to_the_bootstrap_file_and_warns() {
    let sb = Sandbox::new();
    let before = sb.global_file();
    let new_home = tempfile::tempdir().expect("new home");
    let new_home_str = new_home.path().to_str().expect("utf8");

    let set = sb.run(&[
        "config",
        "set",
        "storage.home",
        new_home_str,
        "--scope",
        "global",
    ]);
    assert_eq!(set.code, Some(0), "{}", set.stdout);
    assert_eq!(
        PathBuf::from(set.first()["target_resolved"][0].as_str().expect("target")),
        before,
        "storage.home goes to the file that ProjectDirs locates, not to the relocated one"
    );
    assert!(
        !set.first()["warnings"]
            .as_array()
            .expect("warnings")
            .is_empty()
    );
    let text = std::fs::read_to_string(&before).expect("bootstrap file");
    assert!(text.contains("home ="), "{text}");

    let after = sb.global_file();
    assert_eq!(after, new_home.path().join("config").join("config.toml"));
    let get = sb.run(&["config", "get", "storage.home"]);
    assert_eq!(get.first()["value"], new_home_str);
    assert_eq!(get.first()["origin"], "global");
}

#[test]
fn locale_key_is_persisted_atomically_and_round_trips() {
    let sb = Sandbox::new();
    let set = sb.run(&["config", "set", "locale", "pt-BR", "--scope", "global"]);
    assert_eq!(set.code, Some(0), "{}", set.stdout);
    let file = PathBuf::from(set.first()["config_path"].as_str().expect("path"));
    assert_eq!(file.file_name().and_then(|n| n.to_str()), Some("locale"));
    assert_eq!(std::fs::read_to_string(&file).expect("read"), "pt-BR\n");
    let mode = std::fs::metadata(&file).expect("meta").permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    assert_eq!(
        names_in(file.parent().expect("parent")),
        vec!["locale".to_string()],
        "no tempfile or sidecar may remain"
    );

    let get = sb.run(&["config", "get", "locale"]);
    assert_eq!(get.first()["value"], "pt-BR");

    let through_locale_verb = sb.run(&["locale", "--set", "en"]);
    assert_eq!(
        through_locale_verb.code,
        Some(0),
        "{}",
        through_locale_verb.stdout
    );
    assert_eq!(std::fs::read_to_string(&file).expect("read"), "en\n");
    assert_eq!(
        names_in(file.parent().expect("parent")),
        vec!["locale".to_string()]
    );

    let unset = sb.run(&["config", "unset", "locale", "--scope", "global"]);
    assert_eq!(unset.code, Some(0), "{}", unset.stdout);
    assert!(!file.exists());
}

#[test]
fn concurrency_max_threads_bounds_the_automatic_worker_count() {
    let sb = Sandbox::new();
    let set = sb.run(&[
        "config",
        "set",
        "concurrency.max_threads",
        "1",
        "--scope",
        "global",
    ]);
    assert_eq!(set.code, Some(0), "{}", set.stdout);
    let doctor = sb.run(&["doctor"]);
    assert!(
        doctor.stdout.contains("effective_threads=1"),
        "the ceiling must reach effective_threads: {}",
        doctor.stdout
    );
}

#[cfg(target_os = "linux")]
#[test]
fn concurrency_ram_fraction_scales_the_ram_cap() {
    fn ram_cap(sb: &Sandbox) -> u64 {
        let doctor = sb.run(&["doctor"]);
        let rest = doctor.stdout.split("ram_cap=").nth(1).expect("ram_cap");
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().expect("ram_cap number")
    }
    let sb = Sandbox::new();
    let set = |value: &str| {
        let out = sb.run(&[
            "config",
            "set",
            "concurrency.ram_fraction_percent",
            value,
            "--scope",
            "global",
        ]);
        assert_eq!(out.code, Some(0), "{}", out.stdout);
    };
    set("1");
    let low = ram_cap(&sb);
    set("100");
    let high = ram_cap(&sb);
    assert!(
        high >= low,
        "100% ({high}) must allow at least as many tasks as 1% ({low})"
    );
    assert!(
        high > low || high == 1,
        "100% ({high}) must raise the cap above 1% ({low}) unless RAM only fits one task"
    );
}

#[test]
fn tuning_keys_persist_and_the_binary_still_runs_with_them() {
    let sb = Sandbox::new();
    for (key, value) in [
        ("retry.base_ms", "10"),
        ("retry.max_ms", "80"),
        ("retry.max_elapsed_ms", "500"),
        ("lock.acquire_timeout_ms", "1000"),
        ("search.max_inflight_bytes", "8388608"),
        ("concurrency.ram_fraction_percent", "80"),
        ("extract.max_sample_rows", "0"),
        ("extract.max_text_chars", "30"),
        ("extract.max_sheets", "3"),
        ("extract.max_slides", "5"),
        ("write.confirm_large_bytes", "4096"),
        ("log.rotation", "hourly"),
        ("fuzzy.mode", "off"),
    ] {
        let out = sb.run(&["config", "set", key, value, "--scope", "global"]);
        assert_eq!(out.code, Some(0), "{key}: {}", out.stdout);
        let got = sb.run(&["config", "get", key]);
        assert_eq!(got.first()["origin"], "global", "{key}");
        let shown = got.first()["value"].to_string();
        assert_eq!(shown.trim_matches('"'), value, "{key}");
    }
    let target = sb.ws_path("w.txt");
    let write = sb.run_with_stdin(&["write", &target], "hello\n");
    assert_eq!(
        write.code,
        Some(0),
        "a write under the tuned lock and retry: {}",
        write.stdout
    );
    let search = sb.run(&["search", "hello", sb.ws.path().to_str().expect("utf8")]);
    assert_eq!(
        search.code,
        Some(0),
        "a search under the tuned budget: {}",
        search.stdout
    );
}

#[test]
fn lock_acquire_timeout_key_sets_how_long_a_write_waits() {
    let sb = Sandbox::new();
    let target = sb.ws_path("held.txt");
    std::fs::write(&target, "x\n").expect("seed");
    let set = sb.run(&[
        "config",
        "set",
        "lock.acquire_timeout_ms",
        "300",
        "--scope",
        "workspace",
    ]);
    assert_eq!(set.code, Some(0), "{}", set.stdout);

    // This process holds the advisory lock of the target; the child must give up.
    let held = atomwrite::lock::acquire_exclusive(std::path::Path::new(&target), 1_000)
        .expect("take the lock");
    let started = std::time::Instant::now();
    let out = sb.run_with_stdin(&["write", "--ack-overwrite", &target], "y\n");
    let waited = started.elapsed();
    drop(held);

    assert_eq!(out.code, Some(83), "LOCK_TIMEOUT expected: {}", out.stdout);
    assert!(
        waited < std::time::Duration::from_secs(3),
        "the 300 ms key must beat the 5 s default, waited {waited:?}"
    );
}

/// Minimal OOXML container from `(part name, body)` pairs.
fn package(parts: &[(&str, String)]) -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default();
        for (name, body) in parts {
            writer.start_file(*name, options).expect("start part");
            writer.write_all(body.as_bytes()).expect("write part");
        }
        writer.finish().expect("finish package");
    }
    buffer.into_inner()
}

#[test]
fn extract_keys_bound_the_artifact_and_a_flag_still_wins() {
    let sb = Sandbox::new();

    let long_text = "palavra ".repeat(500);
    let docx = package(&[(
        "word/document.xml",
        format!(
            "<w:document><w:body><w:p><w:r><w:t>{long_text}</w:t></w:r></w:p></w:body></w:document>"
        ),
    )]);
    std::fs::write(sb.ws_path("d.docx"), docx).expect("write docx");

    let mut ids = String::new();
    let mut rels = String::new();
    let mut parts: Vec<(String, String)> = Vec::new();
    for index in 1..=12u32 {
        ids.push_str(&format!(
            r#"<p:sldId id="{}" r:id="rId{index}"/>"#,
            255 + index
        ));
        rels.push_str(&format!(
            r#"<Relationship Id="rId{index}" Target="slides/slide{index}.xml"/>"#
        ));
        parts.push((
            format!("ppt/slides/slide{index}.xml"),
            format!(r#"<p:sld xmlns:a="x"><p:cSld><a:t>Assunto {index}</a:t></p:cSld></p:sld>"#),
        ));
    }
    let mut pptx_parts: Vec<(&str, String)> = vec![
        (
            "ppt/presentation.xml",
            format!(
                r#"<p:presentation xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst>{ids}</p:sldIdLst></p:presentation>"#
            ),
        ),
        ("ppt/_rels/presentation.xml.rels", rels),
    ];
    for (name, body) in &parts {
        pptx_parts.push((name.as_str(), body.clone()));
    }
    std::fs::write(sb.ws_path("s.pptx"), package(&pptx_parts)).expect("write pptx");

    for (key, value) in [
        ("extract.max_text_chars", "30"),
        ("extract.max_slides", "5"),
    ] {
        let out = sb.run(&["config", "set", key, value, "--scope", "workspace"]);
        assert_eq!(out.code, Some(0), "{key}: {}", out.stdout);
    }

    let doc = sb.run(&["extract-artifact", &sb.ws_path("d.docx")]);
    assert_eq!(doc.code, Some(0), "{}", doc.stdout);
    assert_eq!(
        doc.first()["text_excerpt"]
            .as_str()
            .expect("excerpt")
            .chars()
            .count(),
        30,
        "extract.max_text_chars is the default bound"
    );
    let flagged = sb.run(&[
        "extract-artifact",
        &sb.ws_path("d.docx"),
        "--max-text-chars",
        "50",
    ]);
    assert_eq!(
        flagged.first()["text_excerpt"]
            .as_str()
            .expect("excerpt")
            .chars()
            .count(),
        50,
        "the flag wins over the config"
    );
    let deck = sb.run(&["extract-artifact", &sb.ws_path("s.pptx")]);
    assert_eq!(deck.code, Some(0), "{}", deck.stdout);
    assert_eq!(
        deck.first()["slides"].as_array().expect("slides").len(),
        5,
        "extract.max_slides caps the inventory"
    );
}

/// Keys the served `config` schema marks as required.
fn required_keys(sb: &Sandbox) -> Vec<String> {
    let out = sb
        .command()
        .args(["--print-schema", "config"])
        .output()
        .expect("run");
    assert_eq!(out.status.code(), Some(0));
    let schema: Value = serde_json::from_slice(&out.stdout).expect("schema JSON");
    schema["required"]
        .as_array()
        .expect("required array")
        .iter()
        .map(|k| k.as_str().expect("key").to_string())
        .collect()
}

#[test]
fn every_real_envelope_carries_the_required_keys_of_the_served_schema() {
    let sb = Sandbox::new();
    let required = required_keys(&sb);
    assert!(!required.is_empty());
    assert!(
        !required.iter().any(|k| k == "warnings"),
        "warnings is omitted when empty, so it cannot be required: {required:?}"
    );
    let runs: Vec<Vec<&str>> = vec![
        vec!["config", "path", "--scope", "workspace"],
        vec!["config", "list"],
        vec!["config", "get", "fuzzy.threshold"],
        vec![
            "config",
            "set",
            "fuzzy.threshold",
            "0.9",
            "--scope",
            "workspace",
        ],
        vec!["config", "unset", "fuzzy.threshold", "--scope", "workspace"],
    ];
    for args in runs {
        let out = sb.run(&args);
        assert_eq!(out.code, Some(0), "{args:?}: {}", out.stdout);
        let envelope = out.first();
        for key in &required {
            assert!(
                envelope.get(key).is_some(),
                "{args:?} lacks `{key}`: {envelope}"
            );
        }
    }
}

#[test]
fn the_schema_file_in_docs_is_what_the_binary_serves() {
    let sb = Sandbox::new();
    let served = sb
        .command()
        .args(["--print-schema", "config"])
        .output()
        .expect("run");
    let served: Value = serde_json::from_slice(&served.stdout).expect("served schema");
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("schemas")
        .join("config.schema.json");
    let on_disk: Value =
        serde_json::from_slice(&std::fs::read(file).expect("read schema file")).expect("file JSON");
    assert_eq!(
        on_disk, served,
        "regenerate with `atomwrite --print-schema config`"
    );
}

#[test]
fn config_is_listed_in_the_schema_names_and_has_a_schema_file() {
    let sb = Sandbox::new();
    let schema = sb
        .command()
        .args(["config", "list", "--json-schema"])
        .output()
        .expect("run");
    assert_eq!(schema.status.code(), Some(0));
    let served: Value = serde_json::from_slice(&schema.stdout).expect("schema JSON");
    assert!(
        served["properties"]["keys"].is_object(),
        "the served schema describes the list envelope: {served}"
    );
    let names = sb.command().arg("--print-schema").output().expect("run");
    let names = String::from_utf8_lossy(&names.stdout).into_owned();
    assert!(
        names.lines().any(|l| l.trim_matches('"') == "config"),
        "{names}"
    );
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("schemas")
        .join("config.schema.json");
    assert!(file.is_file(), "docs/schemas/config.schema.json must exist");
}
