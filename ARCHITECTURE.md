[Leia em Portugues](ARCHITECTURE.pt-BR.md)


# Architecture


## Overview
- atomwrite is a single Rust binary CLI for atomic file operations
- Designed for LLM agents that need safe, structured file manipulation
- Every write is atomic: tempfile, fsync, rename, fsync directory
- Every response is NDJSON on stdout with BLAKE3 checksums
- All logs go to stderr via tracing


## Module Map

### Entry Point
- `src/main.rs` -- binary entry: signal setup, tracing init, dispatch
- `src/lib.rs` -- library root: module declarations and `run()` dispatcher
- `src/cli.rs` -- clap `#[derive(Parser)]` with global flags
- `src/cli_args.rs` -- per-subcommand argument structs and value enums

### Core Pipeline
- `src/atomic.rs` -- atomic write pipeline: tempfile + fsync + rename + fsync dir
- `src/checksum.rs` -- BLAKE3 hash computation for files and byte slices (uses memmap2 for large files)
- `src/file_io.rs` -- smart file reading with automatic memmap2 above 1 MiB threshold
- `src/platform.rs` -- platform-specific fsync: F_FULLFSYNC on macOS via libc::fcntl

### Safety and Validation
- `src/path_safety.rs` -- workspace jail: path traversal prevention, symlink validation, FIFO/device detection
- `src/signal.rs` -- SIGINT/SIGTERM handling via signal-hook with graceful shutdown coordination
- `src/error.rs` -- domain error enum with exit codes, error classification, and retryable flag

### Output
- `src/output.rs` -- NDJSON writer with broken-pipe handling (SIGPIPE-safe)
- `src/ndjson_types.rs` -- output type definitions with schemars JSON Schema derivation
- `src/constants.rs` -- named constants for buffer sizes, thresholds, and exit codes

### Utilities
- `src/binary_detect.rs` -- null-byte heuristic for binary content detection
- `src/line_endings.rs` -- LF/CRLF/CR detection and normalization
- `src/lang_utils.rs` -- locale initialization and i18n helpers for rust-i18n

### Subcommand Handlers
- `src/commands/` -- 22 subcommand implementations, each in its own module
- Each handler receives parsed args, global config, an NDJSON writer, and shutdown signal
- All handlers follow the same signature: `fn cmd_*(args, global, writer, shutdown) -> Result<()>`


## Data Flow

```
stdin ──> content bytes
             │
             ├── [write/edit/apply] ──> atomic_write() ──> tempfile
             │                              │                 │
             │                              │              fsync(fd)
             │                              │                 │
             │                              │           rename(temp, target)
             │                              │                 │
             │                              │           fsync(dir)
             │                              │                 │
             │                              └──> BLAKE3 checksum
             │
             ├── [search/replace] ──> WalkParallel ──> ripgrep engine
             │                              │
             │                       crossbeam channel
             │                              │
             │                         NDJSON events
             │
             └── [read/hash/list] ──> direct fs ops ──> NDJSON events
                                                              │
                                                         stdout (NDJSON)
```


## Key Decisions

### BLAKE3 over SHA-256
- BLAKE3 is 5-14x faster than SHA-256 for file checksums
- Single-threaded performance matters for CLI latency
- Not used for cryptographic security, only integrity detection

### NDJSON over JSON Array
- Streaming: each result is emitted as soon as computed
- No need to buffer entire output before writing
- Pipe-friendly: downstream tools process line by line
- Errors emit in the same format with `error: true` discriminator

### tempfile + rename over In-Place Write
- Atomic: target file is never left half-written
- Survives power loss, OOM kill, SIGKILL
- Backup of original is a copy (not hardlink) to avoid shared-inode corruption

### Workspace Jail
- All paths validated against --workspace root
- Prevents path traversal via `..` components
- Blocks symlinks pointing outside the workspace
- Rejects FIFO and device files (non-atomic by nature)

### Signal Handling via signal-hook
- SIGINT and SIGTERM set atomic flag for cooperative shutdown
- Second signal forces immediate exit via process::exit
- SIGPIPE reset to default disposition for standard Unix pipe behavior


## Internationalization
- Translations embedded at compile time via rust-i18n
- Locale detection via sys-locale on startup
- Supported locales: en (default fallback), pt-BR
- Override via `--lang` flag or `ATOMWRITE_LANG` environment variable
- Precedence: --lang flag, ATOMWRITE_LANG env, OS locale, en fallback
- NDJSON stdout is NOT translated (machine-readable contract)
- Only stderr messages and error suggestions are locale-aware


## Error Strategy
- `AtomwriteError` enum with thiserror derives Display
- Each variant maps to a sysexits-compatible exit code
- Error classification: permanent, transient, conflict, precondition_failed
- Transient and conflict errors are marked retryable for agent retry loops
- All errors serialize to NDJSON on stdout with machine-readable fields
- `suggestion` field in `ErrorJson` provides actionable recovery guidance for each error variant
- `ErrorContext` struct (added in v0.1.4) carries `workspace_provided: bool` and `workspace: Option<PathBuf>` from the CLI parser to the error output
- `ErrorJson::from_error_with_context(err, &ErrorContext)` produces context-aware suggestions
- `WorkspaceJail` suggestion adapts based on whether the user supplied `--workspace` or `ATOMWRITE_WORKSPACE`
- Legacy `ErrorJson::from_error(err)` delegates to `from_error_with_context` with `ErrorContext::default()` (backward compatible)
- `output::write_error_json_with_context()` propagates the context from the CLI to the NDJSON stream
