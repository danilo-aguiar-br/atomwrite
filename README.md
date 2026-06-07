[Read in Portuguese / Leia em Portugues](README.pt-BR.md)


# atomwrite

> Atomic file operations for LLM agents -- one CLI, zero corruption

[![Crates.io](https://img.shields.io/crates/v/atomwrite)](https://crates.io/crates/atomwrite)
[![docs.rs](https://img.shields.io/docsrs/atomwrite)](https://docs.rs/atomwrite)
[![License](https://img.shields.io/crates/l/atomwrite)](LICENSE)
[![CI](https://github.com/daniloaguiarbr/atomwrite/actions/workflows/ci.yml/badge.svg)](https://github.com/daniloaguiarbr/atomwrite/actions)


## What Is It
- A single Rust binary that handles every file operation an LLM agent needs
- **28 subcommands**: read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, batch, backup, rollback, apply, set, get, del, case, query, outline, completions
- Every write is atomic: tempfile, fsync, rename, fsync directory
- Every response is NDJSON: one JSON object per line, machine-readable by default
- Every file gets a BLAKE3 checksum: detect drift, verify integrity, enable optimistic locking


## What Is New In v0.1.12 (2026-06-07)
- **6 new subcommands (v14 Tier 3)**: `set`, `get`, `del`, `case`, `query`, `outline` for structured config editing and AST analysis
- **G72 REAL syntax check** -- `atomwrite write --syntax-check` invokes the real `tree-sitter` parser (24 languages covered) instead of the bracket-balance heuristic. Exit code 88 on the first syntax error
- **G114 WAL sidecar for crash recovery** -- `atomic_write` writes `.atomwrite.journal.<target>.atomwrite.journal.json` with `Started` and `Committed` entries. `recover_orphan_journals(dir)` is consultative: reports orphans without auto-replay
- **5 new error variants**: `LockTimeout` (83), `SyntaxError` (88), `ExdevFallbackDisabled` (91), `CopyBackBlake3Failed` (92), `OrphanJournal` (93). All bilingual EN/PT-BR with actionable `ErrorContext` suggestions
- **Dependency added**: `tree-sitter-language-pack = "1.8"` with `download` + `dynamic-loading` features. Parsers download on first use, install footprint stays around 5-10 MB instead of 1+ GB
- **445 tests passing** (was 320 baseline in v0.1.10, +125 new across v0.1.11 and v0.1.12). 43 integration test suites, 0 failures
- **7 ADRs** in `docs/decisions/` documenting all architectural decisions for v0.1.12 (tree-sitter-language-pack choice, WAL sidecar design, query/outline kind-name only, etc.)
- **7 new JSON Schemas** in `docs/schemas/` (set, get, del, case, query, outline, wal-recovery)

See `docs/HOW_TO_USE.md` for the full v0.1.12 quickstart and `docs/MIGRATION.md` for the v0.1.11 → v0.1.12 upgrade path. v0.1.11 is the previous release.


## What Was New In v0.1.11 (2026-06-05)
- **`signal_test::shutdown_message_on_stderr` no longer fails on Windows CI (windows-2025-vs2026)** — `libc::write(STDERR_FILENO, ...)` was moved from `src/main.rs` to `src/signal.rs` and gated by `#[cfg(unix)]`. The Windows `ctrlc` path was kept as-is. Also added an `EAGAIN` and `EINTR` retry loop to make the write robust against interrupted syscalls and tight pipe buffer limits in CI sandboxes
- **Race-free readiness detection in `signal_test`** — The test now sets `ATOMWRITE_READY_FILE` to a path under the tempdir, and atomwrite writes its PID there as soon as `install_handlers_early` returns. The test polls the file with a 10 s deadline before sending SIGINT, eliminating the microsecond window where SIGINT could race with `posix_spawn` and arrive before the kernel's `sigaction` is configured
- **Idempotent signal-handler installation** — `install_handlers_early` and `install_handlers` now share a single `Arc<ShutdownSignal>` via a `OnceCell`. Previously each function created its own instance, and only the first instance was flipped by the signal-hook chain, so the main thread's `is_shutdown()` check stayed `false` and the banner was never written


## What Was New In v0.1.10 (2026-06-05)
- **GAP 20 follow-up**: `signal_test::shutdown_message_on_stderr` flushes the shutdown message via `io::stderr().lock()`. The v0.1.8 fix moved `eprintln!` from the signal handler to the main thread but used `writeln!(io::stderr(), ...)` which is fully buffered when stderr is redirected to a pipe. The fix uses the `StderrLock` guard that flushes on Drop


## What Was New In v0.1.8 (2026-06-05)
- **`signal_test::shutdown_message_on_stderr` no longer fails on Linux CI** — `eprintln!` removed from SIGINT/SIGTERM signal handlers per POSIX.1-2017 `signal-safety(7)`. Message now emitted by main thread in `src/main.rs` after `atomwrite::run` returns
- **`atomic::tests::create_backup_and_retention` no longer fails on Windows CI** — `platform::fsync_file_best_effort` logs a warning and continues on `ERROR_ACCESS_DENIED`
- **CI matrix pinned to `windows-2025-vs2026`** — Replaced `windows-latest` to silence migration NOTICE


## What Was New In v0.1.7 (2026-06-05)
- **CI GitHub Actions fully green** — All 6 jobs (check matrix x3, deny, doc, msrv, security) pass after fixing 4 distinct failures
- **MSRV bumped to 1.88** — Required to allow `time` 0.3.47 which resolves RUSTSEC-2026-0009
- **GitHub Actions Node 24 ready** — `actions/checkout@v6`, `actions/cache@v5`, `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true`
- **Cross-compile Windows validated locally on macOS** — Both `x86_64-pc-windows-gnu` and `i686-pc-windows-gnu` pass `cargo check`, `cargo build`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --no-run`


## What Was New In v0.1.4
- **`cargo install atomwrite` works on Windows 10/11** — Three compilation errors in `#[cfg(windows)]` blocks are fixed (E0433, E0507, E0308)
- **Context-aware error suggestions** — `WorkspaceJail` suggestion adapts when `--workspace` is already provided
- **Cross-compile gate** — `tests/cross_compile_check.rs` fails on any `E0433`, `E0308`, or `E0507` regression in `cfg(windows)` blocks


## What Was New In v0.1.3
- `--preserve-timestamps` flag on `edit` and `replace` to control file mtime (default: mtime is updated to reflect the change)
- `mtime_preserved` field in `EditOutput` and `ReplaceResult` NDJSON responses
- BREAKING: atomic write no longer preserves the original file mtime by default. Fixes a silent no-op in `cargo build` / `make` / `cmake` / `gradle`


## Why
- LLM agents juggle dozens of shell commands to manipulate files
- A single power failure or crash mid-write corrupts the file
- Parsing unstructured CLI output wastes tokens and causes hallucinations
- Agents need checksums to detect concurrent edits but rarely compute them
- atomwrite solves all four problems with one `cargo install`


## Superpowers
### Atomic Writes
- Uses tempfile + fsync + rename + directory fsync on every write
- Guarantees all-or-nothing: the file is never left half-written
- Survives power loss, OOM kills, and SIGKILL
- Optional G72 REAL syntax check via tree-sitter (`--syntax-check` flag on `write`)
- Optional G114 WAL sidecar for crash recovery (`.atomwrite.journal.<target>.atomwrite.journal.json`)

### NDJSON Output
- stdout is ALWAYS structured JSON, one object per line
- Every object carries a `"type"` discriminator field
- Agents parse output without regex or brittle text scraping
- Errors also emit JSON with `error: true` on stdout

### BLAKE3 Checksums
- Every `read` and `write` response includes a BLAKE3 hash
- Use `--expect-checksum` for optimistic locking on concurrent edits
- Detect state drift before applying changes

### Parallel Search
- Built on the ripgrep engine for file content search
- Respects `.gitignore` automatically
- Returns structured matches with file, line, column, and context

### AST-Aware Transforms
- Structural search and rewrite powered by ast-grep
- Covers 306 programming languages
- Refactor code by syntax tree, not fragile regex
- New `query` and `outline` subcommands walk tree-sitter ASTs (305 languages) via `tree-sitter-language-pack`

### Grammatical Scoping
- Select AST categories like comments, functions, classes, and strings
- Apply actions: delete, uppercase, lowercase, titlecase, squeeze, or replace
- Covers Rust, Python, JavaScript, TypeScript, and Go with prepared queries
- Use `--pattern` for custom AST patterns beyond the built-in queries

### Batch Operations
- Execute write, replace, delete, edit, hash, move, and copy operations from an NDJSON manifest
- Use `--transaction` for all-or-nothing execution with automatic rollback
- All operations in a batch share the same atomic guarantees

### Structured Config Editing (v0.1.12)
- `set` writes a value at a dotted path in TOML or JSON files (preserves comments via `toml_edit`)
- `get` reads a value at a dotted path with auto-detected format
- `del` removes a key (with `--force-missing` to treat missing keys as no-op success)
- `case` renames identifiers across multiple files via `heck` (snake, camel, pascal, kebab, screaming-snake)


## Quick Start
```bash
cargo install atomwrite

# Write a file atomically from stdin
echo "hello world" | atomwrite --workspace . write src/hello.txt

# Read it back with checksum
atomwrite --workspace . read src/hello.txt

# Search across a directory
atomwrite --workspace . search 'hello' src/

# Replace text with atomic writes
atomwrite --workspace . replace 'hello' 'world' src/

# Evaluate math and unit conversions
atomwrite calc "2 hours + 30 minutes to seconds"

# v0.1.12: set a value in a TOML file (preserves comments)
atomwrite --workspace . set Cargo.toml package.version 0.2.0

# v0.1.12: walk the AST of a Rust file
atomwrite --workspace . query src/main.rs --kinds

# v0.1.12: extract the outline of a Python file
atomwrite --workspace . outline src/app.py

# v0.1.12: REAL tree-sitter syntax check before committing
echo "fn broken(" | atomwrite --workspace . write --syntax-check src/x.rs
```


## Installation
### From crates.io
```bash
cargo install atomwrite
```

### From source
```bash
git clone https://github.com/daniloaguiarbr/atomwrite.git
cd atomwrite
cargo build --release
```

### Shell Completions
```bash
# Bash
atomwrite completions bash > ~/.local/share/bash-completion/completions/atomwrite

# Zsh
atomwrite completions zsh > ~/.zfunc/_atomwrite

# Fish
atomwrite completions fish > ~/.config/fish/completions/atomwrite.fish
```


## Usage
- All output goes to stdout as NDJSON
- All logs go to stderr (only with `--verbose`)
- Use `--workspace <DIR>` to restrict operations to a project root
- Use `ATOMWRITE_WORKSPACE` to set the workspace root via env var
- Use `--dry-run` before destructive operations
- Use `--expect-checksum <HASH>` for optimistic locking
- Use `--lang <LOCALE>` to override the display language (en, pt-BR)
- Pipe stdin for `write` and `batch` commands


## Commands (28 total)

### Core I/O
- `read` — read files with metadata, checksum, optional content
- `write` — create or overwrite files atomically via stdin
- `edit` — surgically edit by line number, text marker, or exact match
- `delete` — delete files with optional backup
- `copy` — copy files with checksum verification
- `move` — move or rename files atomically (EXDEV copy-fallback)
- `apply` — apply patches from stdin (unified diff, search/replace, full, markdown)

### Search and replace
- `search` — search file contents in parallel (ripgrep engine)
- `replace` — replace text across files with atomic writes
- `transform` — AST refactoring via ast-grep (306 languages)

### Inspection
- `hash` — calculate BLAKE3 checksums
- `count` — count lines, files by extension
- `diff` — compare two files (unified, stat, or changes)
- `list` — list files in a directory tree
- `extract` — extract fields from NDJSON input via pipe
- `scope` — grammatical scoping (delete all comments, etc.)
- `regex` — generate regex from examples
- `calc` — math and unit conversions
- `completions` — generate shell completions (bash, zsh, fish, elvish, powershell)

### Backup and recovery
- `backup` — create timestamped backups with BLAKE3 checksums
- `rollback` — restore from a previous backup
- `batch` — NDJSON-driven batch operations (transactional)

### Structured config editors (v0.1.12, v14 Tier 3)
- `set <PATH> <KEY_PATH> <VALUE>` — write a value at a dotted path in a TOML or JSON file. Preserves comments and key order via `toml_edit`. Auto-coerces int/bool/float/string.
- `get <PATH> <KEY_PATH>` — read a value at a dotted path. NDJSON: `{"type":"get","key_path","value","found","format"}`.
- `del <PATH> <KEY_PATH>` — remove a key. `--force-missing` flag treats missing keys as a no-op success.
- `case <PATHS...> --subvert OLD NEW --to <style>` — rename identifiers across multiple files via `heck`. Styles: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`.

### AST tools (v0.1.12, v14 Tier 3 + G72, via tree-sitter-language-pack)
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` — walk a tree-sitter AST and emit nodes as NDJSON. 305 languages supported.
- `outline <PATH> [--kind <KIND>] [--positions]` — extract high-level structure (functions, classes, structs, enums, traits, modules) as NDJSON.
- `write --syntax-check` — G72 REAL syntax check via tree-sitter. 24 languages covered. Exit 88 with first error line/column/kind/message.

### Common patterns for each subcommand

```bash
# read
atomwrite --workspace . read src/main.rs

# write
echo "fn main() {}" | atomwrite --workspace . write src/main.rs

# edit
atomwrite --workspace . edit src/main.rs --old "old text" --new "new text"

# search
atomwrite --workspace . search 'TODO' src/ --include '*.rs'

# replace
atomwrite --workspace . replace 'old_name' 'new_name' src/ --include '*.rs'

# hash
atomwrite --workspace . hash src/main.rs src/lib.rs

# delete
atomwrite --workspace . delete src/temp.rs --backup

# count
atomwrite --workspace . count src/ --by-extension

# diff
atomwrite --workspace . diff src/old.rs src/new.rs --unified

# move
atomwrite --workspace . move src/old.rs src/new.rs

# copy
atomwrite --workspace . copy src/template.rs src/new_module.rs

# list
atomwrite --workspace . list src/ --depth 2

# extract
atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line

# calc
atomwrite calc "2 GiB to bytes"
atomwrite calc "sqrt(144) + 3^2"

# regex
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"

# transform
atomwrite --workspace . transform -p 'println!($$$ARGS)' -r 'tracing::info!($$$ARGS)' -l rust src/

# scope
atomwrite --workspace . scope src/ --lang rust --query comments --delete

# batch
cat manifest.ndjson | atomwrite --workspace . batch --transaction

# backup
atomwrite --workspace . backup src/config.toml

# rollback
atomwrite --workspace . rollback src/config.toml

# apply
echo "new content" | atomwrite --workspace . apply src/file.txt --format full

# set (v0.1.12)
atomwrite --workspace . set Cargo.toml package.version 0.2.0

# get (v0.1.12)
atomwrite --workspace . get Cargo.toml package.version

# del (v0.1.12)
atomwrite --workspace . del Cargo.toml package.metadata.boring

# case (v0.1.12)
atomwrite --workspace . case src/ --subvert user_id UserId --to pascal

# query (v0.1.12)
atomwrite --workspace . query src/main.rs --kinds
atomwrite --workspace . query src/main.rs -Q function_item --positions

# outline (v0.1.12)
atomwrite --workspace . outline src/app.py

# completions
atomwrite completions bash
```


## Environment Variables
- `NO_COLOR`: disable colored output when set to any value
- `RUST_LOG`: control log verbosity (e.g., `RUST_LOG=debug`)
- `ATOMWRITE_LANG`: override locale for translated messages (e.g., `en`, `pt-BR`)
- `ATOMWRITE_WORKSPACE`: set the workspace root for path jail validation (alternative to `--workspace`)
- `RAYON_NUM_THREADS`: override number of parallel threads for search, replace, transform and scope


## Exit Codes
- `0`: success
- `1`: no matches found (search, not an error)
- `4`: file not found
- `13`: permission denied
- `28`: disk full (no space left on device)
- `30`: quota exceeded
- `65`: invalid input (bad arguments or malformed data)
- `73`: cross-device rename (filesystem boundary)
- `74`: I/O error
- `78`: configuration invalid
- `81`: checksum verify failed
- `82`: state drift (checksum mismatch, optimistic lock failed)
- `83`: lock timeout (v0.1.12+)
- `85`: FIFO detected (named pipe cannot be atomically written)
- `86`: device file detected (block or character device)
- `88`: syntax error detected (v0.1.12+, `--syntax-check` failed)
- `91`: EXDEV fallback disabled (v0.1.12+, `--strict-atomic` forbids cross-device copy-fallback)
- `92`: copy-back BLAKE3 verification failed (v0.1.12+)
- `93`: orphan journal detected (v0.1.12+, G114 consultive recovery)
- `126`: workspace jail violated (path escapes workspace)
- `127`: symlink blocked (symlink target outside workspace)
- `128`: file immutable (cannot modify)
- `130`: interrupted by SIGINT
- `141`: broken pipe (SIGPIPE)
- `143`: terminated by SIGTERM
- `255`: internal error


## Signal Handling
- Unix: SIGINT (Ctrl+C) and SIGTERM intercepted for graceful shutdown
- Unix: SIGPIPE reset to SIG_DFL for standard pipe behavior (exit 141)
- Windows: Ctrl+C intercepted via console handler
- First signal: sets shutdown flag, prints "shutting down..." to stderr
- Second signal: immediate process termination via `_exit` (Unix) or `exit` (Windows)
- Walker threads (search, replace, transform, scope) stop between files
- Batch operations stop between operations


## Error Handling
- All errors emit a JSON object on stdout with `error: true`
- Error fields: `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `workspace`
- Error classes: `permanent`, `transient`, `conflict`, `precondition_failed`
- 25 error variants total (20 baseline from v0.1.4 + 5 added in v0.1.12)
- Transient and conflict errors set `retryable: true`
- The `suggestion` field provides actionable recovery guidance for agents
- See `docs/schemas/error-output.schema.json` for the full contract


## Performance
- Single static binary with zero runtime dependencies
- Release builds use LTO, single codegen unit, and symbol stripping
- Memory-mapped file reads via `memmap2` for large files
- Parallel search via rayon and the ripgrep engine
- Typical file operation latency: under 5 ms for small files


## Troubleshooting FAQ

### atomwrite write hangs with no output
- Ensure you are piping content to stdin
- `write` reads from stdin and waits for EOF
- Example: `echo "content" | atomwrite --workspace . write file.txt`

### search returns exit code 1
- Exit code 1 means zero matches were found
- This is expected behavior, not an error
- Check the pattern and target path

### cross-device rename fails with exit 73
- The source and destination are on different filesystems
- atomwrite falls back to copy+delete for `move` across devices
- Use `copy` followed by `delete` as an alternative

### checksum mismatch with exit 82
- Another process modified the file between read and write
- Re-read the file to get the current checksum
- Retry the operation with the updated `--expect-checksum`

### workspace jail violated with exit 126
- The target path resolves outside the `--workspace` boundary
- Verify the path does not contain `..` traversals or symlinks escaping the workspace

### syntax check failed with exit 88 (v0.1.12+)
- The G72 REAL tree-sitter syntax check found a syntax error in the file
- Inspect the first error line/column/kind/message in the JSON error envelope
- Fix the syntax and retry, or remove `--syntax-check` to bypass


## Architecture
- See [ARCHITECTURE.md](ARCHITECTURE.md) for module map, data flow, and design decisions
- See [docs/decisions/](docs/decisions/README.md) for 7 ADRs covering v0.1.12 architecture (G72, G114, v14 Tier 3)
- See [docs/schemas/](docs/schemas/README.md) for 22 stable JSON Schema contracts for all NDJSON output


## Contributing
- See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines
- See [docs/decisions/README.md](docs/decisions/README.md) for the decision log


## Security
- See [SECURITY.md](SECURITY.md) for vulnerability reporting
- See [SECURITY.md](#) section "Known Security Advisories" for resolved and active advisories


## Changelog
- See [CHANGELOG.md](CHANGELOG.md) for English release history
- See [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) for Portuguese release history


## License
- Licensed under MIT OR Apache-2.0
- See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details
