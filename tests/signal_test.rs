// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[cfg(unix)]
#[test]
fn sigpipe_exits_141_or_signal_13() {
    use std::io::Write;
    use std::os::unix::process::ExitStatusExt;
    use std::process::Stdio;

    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("sigpipe.txt");

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");

    let mut child = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "write"])
        .arg(&target)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");

    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(b"sigpipe test content\n");
        let _ = stdin.flush();
    }
    drop(child.stdin.take());

    drop(child.stdout.take());

    let status = child.wait().expect("wait");

    let ok = if let Some(code) = status.code() {
        code == 0 || code == 141
    } else if let Some(sig) = status.signal() {
        sig == 13
    } else {
        false
    };

    assert!(
        ok,
        "expected exit 0, exit 141, or killed by SIGPIPE(13), got {:?}",
        status
    );
}
