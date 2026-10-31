//! Repository maintenance tasks for the `atomwrite` crate.
//!
//! Tasks are reached through the `cargo xtask <task>` alias declared in
//! `.cargo/config.toml`. Diagnostics always go to stderr so that stdout stays
//! reserved for machine-readable payloads.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, ExitStatus};

use clap::{CommandFactory, Parser, Subcommand};

/// Top-level maintenance task dispatcher.
#[derive(Debug, Parser)]
#[command(
    name = "xtask",
    about = "Repository maintenance tasks for atomwrite",
    version
)]
struct Xtask {
    /// Maintenance task to execute.
    #[command(subcommand)]
    task: Task,
}

/// Maintenance tasks available to repository maintainers.
#[derive(Debug, Subcommand)]
enum Task {
    /// Render the manual pages of the CLI into the given directory.
    Man {
        /// Directory that receives the rendered `*.1` files.
        #[arg(long, value_name = "DIR", default_value = "target/man")]
        out_dir: PathBuf,
    },
    /// Run `cargo check --locked` for the slim feature sets (`core`, `core,watch`).
    CheckFeatures {
        /// Cargo target directory forwarded to every `cargo check` run.
        #[arg(long, value_name = "DIR")]
        target_dir: Option<PathBuf>,
    },
    /// Write every published JSON Schema into the given directory.
    Schemas {
        /// Directory that receives the `*.schema.json` files.
        #[arg(long, value_name = "DIR", default_value = "docs/schemas")]
        out_dir: PathBuf,
    },
}

/// Feature sets that must keep compiling without the default features.
const SLIM_FEATURE_SETS: [&str; 2] = ["core", "core,watch"];

/// Render `cmd` into `out_dir` under `file_name`.
///
/// # Errors
///
/// Returns an error when the rendered page cannot be written to disk.
fn render_man(cmd: &clap::Command, out_dir: &Path, file_name: &str) -> std::io::Result<()> {
    let man = clap_mangen::Man::new(cmd.clone());
    let mut buf = Vec::new();
    man.render(&mut buf)?;
    fs::write(out_dir.join(file_name), &buf)
}

/// Render `atomwrite.1` plus one manual page per subcommand.
///
/// # Errors
///
/// Returns an error when the output directory cannot be created or when any
/// page cannot be written.
fn generate_man_pages(out_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(out_dir)?;

    let cmd = atomwrite::cli::Cli::command();
    render_man(&cmd, out_dir, "atomwrite.1")?;

    for sub in cmd.get_subcommands() {
        let name = format!("atomwrite-{}", sub.get_name());
        render_man(sub, out_dir, &format!("{name}.1"))?;
    }

    Ok(())
}

/// Map a child exit status to this process' exit code.
///
/// A normal exit keeps the child's code (values above 255 collapse to 1). On
/// Unix, death by signal maps to `128 + signal`, the shell convention, so a
/// timeout kill or Ctrl-C stays visible to the caller.
fn exit_code_of(status: ExitStatus) -> ExitCode {
    if let Some(code) = status.code() {
        return ExitCode::from(u8::try_from(code).unwrap_or(1));
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return ExitCode::from(u8::try_from(128 + signal).unwrap_or(1));
        }
    }
    ExitCode::FAILURE
}

/// Run `cargo check --locked --no-default-features` for every slim feature set.
///
/// Stops at the first failing set and returns the exit code of that `cargo`
/// run. Cargo output goes to this process' stderr so stdout stays clean.
///
/// # Errors
///
/// Returns an error when `cargo` cannot be spawned.
fn check_features(target_dir: Option<&Path>) -> std::io::Result<ExitCode> {
    for features in SLIM_FEATURE_SETS {
        eprintln!("xtask: cargo check --no-default-features --features {features}");
        let mut cmd = Command::new("cargo");
        cmd.args(["check", "--locked", "--package", "atomwrite"])
            .args(["--no-default-features", "--features", features])
            .stdout(std::process::Stdio::null());
        if let Some(dir) = target_dir {
            cmd.arg("--target-dir").arg(dir);
        }
        let status = cmd.status()?;
        if !status.success() {
            eprintln!("xtask: feature set `{features}` failed ({status})");
            return Ok(exit_code_of(status));
        }
    }
    eprintln!("xtask: all slim feature sets compile");
    Ok(ExitCode::SUCCESS)
}

/// Write `docs/schemas/<FILE>.schema.json` for every published schema name.
///
/// The name-to-file table comes from the crate, so the file the generator
/// writes and the file the sync test reads are the same pairing. A name the
/// binary cannot serve is an error, never an empty document.
///
/// # Errors
///
/// Returns an error when the directory cannot be created, a name is unknown, a
/// schema cannot be rendered, or a file cannot be written.
fn generate_schemas(out_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(out_dir)?;
    for (name, file) in atomwrite::SCHEMA_FILE_NAMES {
        let mut buf = Vec::new();
        let emitted = atomwrite::emit_schema_by_name(name, &mut buf)
            .map_err(|e| std::io::Error::other(format!("schema {name}: {e}")))?;
        if !emitted {
            return Err(std::io::Error::other(format!(
                "unknown schema name `{name}`"
            )));
        }
        fs::write(out_dir.join(format!("{file}.schema.json")), &buf)?;
    }
    eprintln!(
        "xtask: {} schemas written to {}",
        atomwrite::SCHEMA_FILE_NAMES.len(),
        out_dir.display()
    );
    Ok(())
}

fn main() -> std::io::Result<ExitCode> {
    let cli = Xtask::parse();

    match cli.task {
        Task::Man { out_dir } => {
            generate_man_pages(&out_dir)?;
            eprintln!("Man pages generated in {}", out_dir.display());
        }
        Task::CheckFeatures { target_dir } => return check_features(target_dir.as_deref()),
        Task::Schemas { out_dir } => generate_schemas(&out_dir)?,
    }

    Ok(ExitCode::SUCCESS)
}
