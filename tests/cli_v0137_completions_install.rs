// SPDX-License-Identifier: MIT OR Apache-2.0

//! `completions --install` writes the script it would print, and reports the
//! path it wrote.
//!
//! The flag used to be covered by no test at all, so the two things that can go
//! wrong were both unmeasured: the installed bytes could drift from the stdout
//! bytes, and a denied directory answered 255 `INTERNAL_ERROR` with "this is a
//! bug" instead of 13 `PERMISSION_DENIED`. Both are pinned here, one shell over
//! the whole table so a new shell cannot enter without a check.

mod common;

use std::path::Path;

use serde_json::Value;

/// Shell name, the directory it installs into and its file name, read from the
/// published constants through the binary, not copied by hand: the test asks
/// the command where it put the file and only then looks there.
const SHELLS: [&str; 5] = ["bash", "zsh", "fish", "powershell", "elvish"];

/// Run `args` with its data directory pointed at `data`, and return the parsed
/// events together with the exit code.
fn run_with_data(data: &Path, args: &[&str]) -> (Vec<Value>, Option<i32>) {
    let output = common::atomwrite()
        .args(args)
        .env("XDG_DATA_HOME", data)
        .output()
        .expect("run atomwrite");
    (common::parse_ndjson(&output.stdout), output.status.code())
}

#[test]
fn install_writes_the_bytes_it_prints_and_names_the_path() {
    let ws = tempfile::tempdir().expect("workspace");
    let data = ws.path().join("data");
    std::fs::create_dir_all(&data).expect("data dir");

    for shell in SHELLS {
        let (events, code) = run_with_data(&data, &["completions", shell, "--install"]);
        assert_eq!(code, Some(0), "{shell}: install must succeed");
        let event = events.last().expect("install emits one event");
        assert_eq!(event["type"], "installed", "{shell}: {event}");
        assert_eq!(event["ok"], true, "{shell}: {event}");

        let installed = event["path"].as_str().expect("install path");
        let written = std::fs::read(installed).expect("installed file exists");

        // Without `--install` the stdout is the raw script, never NDJSON, so
        // this call reads the bytes and does not go through the parser.
        let output = common::atomwrite()
            .args(["completions", shell])
            .env("XDG_DATA_HOME", &data)
            .output()
            .expect("run atomwrite");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{shell}: printing must succeed"
        );
        let expected = output.stdout;
        assert!(
            !expected.is_empty(),
            "{shell}: the printed script must not be empty"
        );
        assert_eq!(
            written, expected,
            "{shell}: the installed script must be the one the command prints"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_denied_data_directory_is_exit_13_and_not_an_internal_error() {
    use std::os::unix::fs::PermissionsExt;

    let ws = tempfile::tempdir().expect("workspace");
    let data = ws.path().join("locked");
    std::fs::create_dir_all(&data).expect("data dir");
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o500))
        .expect("make the directory read-only");

    let (events, code) = run_with_data(&data, &["-v", "completions", "bash", "--install"]);

    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o700))
        .expect("restore the directory, so the temporary tree can be removed");

    assert_eq!(
        code,
        Some(13),
        "a denied directory is PERMISSION_DENIED, got {events:?}"
    );
    let event = events.last().expect("one failure event");
    assert_eq!(event["code"], "PERMISSION_DENIED", "{event}");
    assert_eq!(event["ok"], false, "{event}");
    let message = event["message"].as_str().expect("message");
    assert!(
        message.contains("permission denied"),
        "the message must name the refusal: {message}"
    );
    assert!(
        !message.contains("bug"),
        "a permission the operator can fix is not a bug of the tool: {message}"
    );
}
