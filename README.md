[Read in Portuguese / Leia em Portugues](README.pt-BR.md)


# atomwrite

> Atomic file operations for LLM agents -- one CLI, zero corruption

[![Crates.io](https://img.shields.io/crates/v/atomwrite)](https://crates.io/crates/atomwrite)
[![License](https://img.shields.io/crates/l/atomwrite)](LICENSE)
[![CI](https://github.com/comandoaguiar/atomwrite/actions/workflows/ci.yml/badge.svg)](https://github.com/comandoaguiar/atomwrite/actions)


## What Is It
- A single Rust binary that handles every file operation an LLM agent needs
- Read, write, edit, search, replace, diff, copy, move, delete, transform, scope, backup, rollback, apply -- all in one tool
- Every write is atomic: tempfile, fsync, rename, fsync directory
- Every response is NDJSON: one JSON object per line, machine-readable by default
- Every file gets a BLAKE3 checksum: detect drift, verify integrity, enable optimistic locking


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

### Grammatical Scoping
- Select AST categories like comments, functions, classes, and strings
- Apply actions: delete, uppercase, lowercase, titlecase, squeeze, or replace
- Covers Rust, Python, JavaScript, TypeScript, and Go with prepared queries
- Use `--pattern` for custom AST patterns beyond the built-in queries

### Batch Operations
- Execute write, replace, delete, edit, hash, move, and copy operations from an NDJSON manifest
- Use `--transaction` for all-or-nothing execution with automatic rollback
- All operations in a batch share the same atomic guarantees
- Use `backup` and `rollback` commands for manual snapshot and restore workflows
- One CLI call replaces hundreds of individual tool invocations


## Quick Start
```bash
cargo install atomwrite

# Write a file atomically from stdin
echo "hello world" | atomwrite write src/hello.txt

# Read it back with checksum
atomwrite read src/hello.txt

# Search across a directory
atomwrite search 'hello' src/

# Replace text with atomic writes
atomwrite replace 'hello' 'world' src/

# Evaluate math and unit conversions
atomwrite calc "2 hours + 30 minutes to seconds"
```


## Installation
### From crates.io
```bash
cargo install atomwrite
```

### From source
```bash
git clone https://github.com/comandoaguiar/atomwrite.git
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
- Use `--dry-run` before destructive operations
- Use `--expect-checksum <HASH>` for optimistic locking
- Use `--lang <LOCALE>` to override the display language (en, pt-BR)
- Pipe stdin for `write` and `batch` commands


## Commands

### read
- Read one or more files with metadata, size, permissions, and BLAKE3 checksum
- Use `--stat` to skip file content and return only metadata
```bash
atomwrite read src/main.rs
```

### write
- Create or overwrite a file atomically from stdin
- Returns the BLAKE3 checksum of the written content
- Use `--line-ending lf|crlf|cr|auto` to normalize line endings (default: auto preserves original)
```bash
echo "fn main() {}" | atomwrite write src/main.rs
```

### edit
- Surgically edit a file by line number, text marker, or exact match
- Supports insert, replace, and delete operations
- Use `--expect-checksum` to prevent concurrent edit conflicts
- Use `--fuzzy auto|off|aggressive` for fuzzy text matching
- Use `--line-ending lf|crlf|cr|auto` to normalize line endings
- Use `--multi` to apply multiple edits via NDJSON stdin in a single atomic write
```bash
echo "new content" | atomwrite edit src/main.rs --after-line 5
atomwrite edit src/main.rs --old "old text" --new "new text"
```

### search
- Search file contents in parallel using the ripgrep engine
- Returns structured matches with file, line, column, and context
- Exits with code 1 when zero matches are found (not an error)
- Use `--context N` for surrounding context lines
- Use `--max-count N` to limit matches per file
- Use `--invert` to show lines without a match
- Use `--sort path` to sort results by file path
```bash
atomwrite search 'TODO' src/ --include '*.rs'
```

### replace
- Replace text across files with atomic writes
- Supports regex patterns and literal strings
- Use `--dry-run` to preview changes
```bash
atomwrite replace 'old_name' 'new_name' src/ --include '*.rs'
```

### hash
- Calculate BLAKE3 checksums for one or more files
```bash
atomwrite hash src/main.rs src/lib.rs
```

### delete
- Delete files with optional backup before removal
- Use `--backup` to create a `.bak` copy first
```bash
atomwrite delete src/temp.rs --backup
```

### count
- Count lines in files or count files by extension in a directory
```bash
atomwrite count src/ --by-extension
```

### diff
- Compare two files with unified, stat, or changes-only output
```bash
atomwrite diff src/old.rs src/new.rs --unified
```

### move
- Move or rename files atomically
- Falls back to copy+delete for cross-device moves
```bash
atomwrite move src/old.rs src/new.rs
```

### copy
- Copy files with BLAKE3 checksum verification after copy
```bash
atomwrite copy src/template.rs src/new_module.rs
```

### list
- List project file structure with metadata
- Respects `.gitignore` by default
```bash
atomwrite list src/ --depth 2
```

### extract
- Extract fields from NDJSON input or text columns from stdin
```bash
atomwrite search 'TODO' src/ | atomwrite extract path line
```

### calc
- Evaluate math expressions and unit conversions
- Powered by fend for arbitrary-precision arithmetic
```bash
atomwrite calc "2 GiB to bytes"
atomwrite calc "sqrt(144) + 3^2"
```

### regex
- Generate regex patterns from example strings
- Powered by grex for automatic inference
```bash
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"
```

### transform
- Structural AST search and rewrite powered by ast-grep
- Covers 306 programming languages
- Use `--rewrite` to apply transformations
```bash
atomwrite transform -p 'println!($$$ARGS)' -l rust src/
atomwrite transform -p 'println!($$$ARGS)' -r 'tracing::info!($$$ARGS)' -l rust src/
```

### batch
- Execute multiple operations from an NDJSON manifest on stdin
- Supports write, replace, delete, edit, hash, move, and copy operations
- Use `--transaction` for all-or-nothing execution with automatic rollback
```bash
cat manifest.ndjson | atomwrite batch
cat manifest.ndjson | atomwrite batch --transaction
```

### scope
- Grammatical scoping: select AST categories and apply actions
- Use `--query` for prepared queries (fn, comments, strings, struct, etc.)
- Use `--pattern` for custom AST patterns
- Use `--delete` to remove matched content or `--action upper|lower|titlecase|squeeze`
- Covers Rust, Python, JavaScript, TypeScript, and Go
```bash
atomwrite scope src/ --lang rust --query comments --delete
atomwrite scope src/ --lang rust --query fn --action upper --dry-run
```

### backup
- Create timestamped backups of files with BLAKE3 checksums
- Use `--retention N` to control how many backups to keep
```bash
atomwrite backup src/config.toml
atomwrite backup src/main.rs src/lib.rs --retention 3
```

### rollback
- Restore a file from a previous backup
- Use `--verify` to check BLAKE3 checksum after restoration
```bash
atomwrite rollback src/config.toml
atomwrite rollback src/config.toml --timestamp 20260530_120000
```

### apply
- Apply a patch from stdin (unified diff, SEARCH/REPLACE blocks, or full file replacement)
- Auto-detects patch format or use `--format` to specify
```bash
echo "new content" | atomwrite apply src/file.txt --format full
git diff src/file.txt | atomwrite apply src/file.txt --format unified
```

### completions
- Generate shell completion scripts for bash, zsh, fish, elvish, or PowerShell
```bash
atomwrite completions bash
```


## Environment Variables
- `NO_COLOR`: disable colored output when set to any value
- `RUST_LOG`: control log verbosity (e.g., `RUST_LOG=debug`)
- `ATOMWRITE_LANG`: override locale for translated messages (e.g., `en`, `pt-BR`)


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
- `82`: state drift (checksum mismatch, optimistic lock failed)
- `126`: workspace jail violated (path escapes workspace)
- `127`: symlink blocked (symlink target outside workspace)
- `85`: FIFO detected (named pipe cannot be atomically written)
- `86`: device file detected (block or character device)
- `128`: file immutable (cannot modify)
- `130`: interrupted by SIGINT
- `141`: broken pipe (SIGPIPE)
- `143`: terminated by SIGTERM
- `255`: internal error


## Error Handling
- All errors emit a JSON object on stdout with `error: true`
- Error fields: `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`
- Error classes: `permanent`, `transient`, `conflict`, `precondition_failed`
- Transient and conflict errors set `retryable: true`
- The `suggestion` field provides actionable recovery guidance for agents


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
- Example: `echo "content" | atomwrite write file.txt`

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


## Architecture
- See [ARCHITECTURE.md](ARCHITECTURE.md) for module map, data flow, and design decisions


## Contributing
- See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines


## Security
- See [SECURITY.md](SECURITY.md) for vulnerability reporting


## Changelog
- See [CHANGELOG.md](CHANGELOG.md) for release history


## License
- Licensed under MIT OR Apache-2.0
- See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details
