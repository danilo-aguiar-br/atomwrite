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

#[cfg(unix)]
#[test]
fn sigint_during_search_exits_130() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    let dir = tempfile::tempdir().unwrap();
    for i in 0..200 {
        std::fs::write(
            dir.path().join(format!("file_{i}.txt")),
            "needle in haystack\n".repeat(1000),
        )
        .unwrap();
    }

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut child = Command::new(&bin)
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "needle",
        ])
        .arg(dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    std::thread::sleep(Duration::from_millis(100));
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }

    let status = child.wait().unwrap();
    let ok = status.code() == Some(130) || status.signal() == Some(2);
    assert!(ok, "expected exit 130 or signal 2, got {status:?}");
}

#[cfg(unix)]
#[test]
fn sigterm_during_search_exits_143() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    let dir = tempfile::tempdir().unwrap();
    for i in 0..200 {
        std::fs::write(
            dir.path().join(format!("file_{i}.txt")),
            "data content here\n".repeat(1000),
        )
        .unwrap();
    }

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut child = Command::new(&bin)
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "data",
        ])
        .arg(dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    std::thread::sleep(Duration::from_millis(100));
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }

    let status = child.wait().unwrap();
    let ok = status.code() == Some(143) || status.signal() == Some(15);
    assert!(ok, "expected exit 143 or signal 15, got {status:?}");
}

#[cfg(unix)]
#[test]
fn batch_interrupted_by_signal() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    let dir = tempfile::tempdir().unwrap();

    let mut manifest = String::new();
    for i in 0..500 {
        manifest.push_str(&format!(
            r#"{{"op":"write","target":"file_{i}.txt","content":"content {i}"}}"#
        ));
        manifest.push('\n');
    }

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut child = Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(manifest.as_bytes());
        let _ = stdin.flush();
    }
    drop(child.stdin.take());

    std::thread::sleep(Duration::from_millis(20));
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }

    let output = child.wait_with_output().unwrap();
    let _stdout = String::from_utf8_lossy(&output.stdout);

    let files_created: usize = std::fs::read_dir(dir.path())
        .unwrap()
        .filter(|e| e.as_ref().map(|e| e.path().is_file()).unwrap_or(false))
        .count();

    assert!(
        files_created < 500,
        "batch should have been interrupted, created {files_created}/500 files"
    );
}

#[cfg(unix)]
#[test]
fn shutdown_message_on_stderr() {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let dir = tempfile::tempdir().unwrap();
    for i in 0..200 {
        std::fs::write(
            dir.path().join(format!("file_{i}.txt")),
            "searchable content\n".repeat(1000),
        )
        .unwrap();
    }

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let ready_path = dir.path().join("ready");

    let child = Command::new(&bin)
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "searchable",
        ])
        .arg(dir.path())
        .env("ATOMWRITE_READY_FILE", &ready_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Wait for atomwrite to install its signal handlers. Without this
    // barrier, SIGINT can race `posix_spawn` and arrive before
    // `install_handlers_early` calls `signal_hook::flag::register`, in
    // which case the kernel's SIG_DFL disposition kills the child with
    // no shutdown banner. Polling the readiness file is the only
    // race-free way to observe handler installation without modifying
    // the public CLI surface.
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready_path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        ready_path.exists(),
        "atomwrite did not signal handler readiness within 10s"
    );

    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }

    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("shutting down"),
        "expected 'shutting down' in stderr, got: {stderr}"
    );
}
