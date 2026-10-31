//! Build script: embeds the build source (short git SHA or `no-git`) and the
//! target triple, and declares the files that must invalidate the build.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Marker used when the crate is built outside a git repository.
const NO_GIT: &str = "no-git";

/// Walk up from `start` and return the first `.git` entry (directory or file).
fn find_git_entry(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|dir| dir.join(".git"))
        .find(|candidate| candidate.exists())
}

/// Run `git` with `args` inside `dir`; `Some(stdout)` only on a zero exit.
fn git_stdout(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Short SHA of `HEAD` when `dir` belongs to a git repository that tracks this
/// crate's `Cargo.toml`. A registry download that happens to sit inside an
/// unrelated repository does not track `Cargo.toml`, so it reports `no-git`.
///
/// The identifier names the commit and nothing else. It used to carry a
/// `-dirty` suffix driven by `git status`, which made the same source tree
/// report two different builds depending on whether an untracked file happened
/// to exist. The consumer cannot act on that difference, and a version string
/// that changes without the code changing is worse than a version string that
/// says less.
fn git_source(dir: &Path) -> String {
    let tracked = git_stdout(dir, &["ls-files", "--error-unmatch", "Cargo.toml"]).is_some();
    git_stdout(dir, &["rev-parse", "--short", "HEAD"])
        .filter(|s| !s.is_empty() && tracked)
        .unwrap_or_else(|| NO_GIT.to_string())
}

/// Emit `cargo:rerun-if-changed` only for paths that exist.
fn rerun_if_exists(path: &Path) {
    if path.exists() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn main() {
    // rust_i18n::i18n!("locales") embeds translations at compile time. Cargo does
    // not always see those file deps through the proc-macro alone (see longbridge
    // rust-i18n#46); declare them so locale edits invalidate the binary.
    for path in [
        "build.rs",
        "locales",
        "locales/en.toml",
        "locales/pt-BR.toml",
    ] {
        rerun_if_exists(Path::new(path));
    }

    let manifest_dir =
        std::env::var_os("CARGO_MANIFEST_DIR").map_or_else(|| PathBuf::from("."), PathBuf::from);
    let source = match find_git_entry(&manifest_dir) {
        Some(git_entry) => {
            // Rebuild when HEAD or the index change (best-effort; cargo may still cache).
            if git_entry.is_dir() {
                rerun_if_exists(&git_entry.join("HEAD"));
                rerun_if_exists(&git_entry.join("index"));
            } else {
                rerun_if_exists(&git_entry);
            }
            git_source(&manifest_dir)
        }
        None => NO_GIT.to_string(),
    };
    println!("cargo:rustc-env=ATOMWRITE_GIT_SHA={source}");

    // Cargo always sets TARGET for build scripts.
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=TARGET={target}");
}
