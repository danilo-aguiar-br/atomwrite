// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use assert_cmd::Command;
use serde_json::Value;

/// Locale every child process runs under, so a message never depends on the
/// language of the machine that runs the suite.
const HERMETIC_LANG: &str = "en_US.UTF-8";

/// Empty home directory shared by every child of this test process.
///
/// It lives under the cargo target tmp dir, one per process, and starts empty:
/// no `config.toml`, no `locale` preference, nothing a developer saved with
/// `atomwrite config set`. A test that needs its own config redirects `HOME`
/// and `XDG_*` itself after calling [`hermetic`]; the later `env` call wins.
/// The XDG cache is the one exception, see [`real_cache_home`].
pub fn hermetic_home() -> &'static Path {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    HOME.get_or_init(|| {
        let stem = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "test".to_string());
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("hermetic-home")
            .join(format!("{stem}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create the hermetic home");
        dir
    })
}

/// Cache directory the children keep using: the real one of this machine.
///
/// The cache holds only disposable artifacts, and the AST parsers are fetched
/// into it once; an empty cache would send every `outline`/`query`/`scope` test
/// to the network. Config, data and state stay isolated.
fn real_cache_home() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| hermetic_home().join("cache"))
}

/// A command whose environment can be isolated from the machine running it.
pub trait Hermetic {
    /// Point `HOME`, `XDG_*` and the Windows profile variables at an empty
    /// directory and fix the locale, on the child only.
    fn hermetic(self) -> Self;
}

macro_rules! impl_hermetic {
    ($command:ty) => {
        impl Hermetic for $command {
            fn hermetic(mut self) -> Self {
                let home = hermetic_home();
                self.env("HOME", home)
                    .env("USERPROFILE", home)
                    .env("APPDATA", home)
                    .env("LOCALAPPDATA", home)
                    .env("XDG_CONFIG_HOME", home.join("config"))
                    .env("XDG_DATA_HOME", home.join("data"))
                    .env("XDG_STATE_HOME", home.join("state"))
                    .env("XDG_CACHE_HOME", real_cache_home())
                    .env("LANG", HERMETIC_LANG)
                    .env("LC_ALL", HERMETIC_LANG)
                    .env_remove("LANGUAGE")
                    .env_remove("LC_MESSAGES");
                self
            }
        }
    };
}

impl_hermetic!(Command);
impl_hermetic!(std::process::Command);

/// Isolate a command from the real home and locale (see [`Hermetic`]).
pub fn hermetic<C: Hermetic>(command: C) -> C {
    command.hermetic()
}

pub fn atomwrite() -> Command {
    hermetic(Command::cargo_bin("atomwrite").expect("binary not found"))
}

pub fn parse_ndjson(output: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(output)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("invalid NDJSON line"))
        .collect()
}

pub fn create_test_file(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create dirs");
    }
    std::fs::write(&path, content).expect("write test file");
    path
}

/// Build an NDJSON manifest string from a list of JSON operations.
///
/// Uses `serde_json::to_string` to ensure paths and other strings are
/// properly escaped. `format!` with `display()` is unsafe for paths on
/// Windows because the embedded backslashes produce invalid JSON escapes
/// (e.g. `\U`, `\r`, `\T`). Always use this helper to construct manifests
/// in tests so the suite is portable across all platforms.
pub fn manifest(ops: &[serde_json::Value]) -> String {
    ops.iter()
        .map(|op| serde_json::to_string(op).expect("serialize manifest op"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub mod schema_check;
