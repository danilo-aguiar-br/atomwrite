# How to Use atomwrite


[Leia em Portugues](HOW_TO_USE.pt-BR.md)

> One CLI replaces dozens of file-manipulation tool calls your agent makes today


## Prerequisites
- Rust toolchain 1.85 or later
- Install via `cargo install atomwrite`
- Verify with `atomwrite --version`
- Works on Linux, macOS and Windows


## First Command in 60 Seconds
- Write a file atomically from stdin:

```bash
echo "hello world" | atomwrite write src/hello.txt
```

- Read it back with metadata and checksum:

```bash
atomwrite read src/hello.txt
```

- You get NDJSON on stdout with path, checksum, bytes and timing
- Every write survives power failure and crashes


## Core Commands
### write
- Create or overwrite files atomically via stdin
- The write follows tempfile, fsync, rename, fsync-dir sequence
- Your data reaches disk or the operation fails cleanly

```bash
echo "fn main() {}" | atomwrite write src/main.rs
cat config.toml | atomwrite write --backup config.toml
echo "data" | atomwrite write --expect-checksum abc123 src/file.txt
```

- Use `--backup` to create a timestamped backup before overwriting
- Use `--expect-checksum` for optimistic locking on concurrent edits
- Use `--line-ending lf|crlf|cr|auto` to normalize line endings (default: auto preserves original)
- Use `--dry-run` to preview the operation without writing

### read
- Read files with metadata, checksum and optional content
- Returns line count, byte size, permissions and modification time

```bash
atomwrite read src/main.rs
atomwrite read --stat src/main.rs
atomwrite read --lines 1:50 src/main.rs
atomwrite read --verify-checksum abc123 src/main.rs
```

- Use `--stat` to get metadata without file content
- Use `--lines 1:50` to read a specific line range
- Binary files are detected and content is omitted automatically

### edit
- Surgically edit files by line number, text marker or exact match
- The edit is atomic: tempfile, fsync, rename

```bash
echo "new line" | atomwrite edit src/main.rs --after-line 5
echo "replacement block" | atomwrite edit src/main.rs --range 10:20
atomwrite edit src/main.rs --old "old_text" --new "new_text"
```

- Use `--fuzzy auto|off|aggressive` for fuzzy text matching when exact match fails
- Use `--multi` to apply multiple NDJSON edits in a single atomic write via stdin
- Use `--line-ending lf|crlf|cr|auto` to normalize line endings (default: auto preserves original)
- Use `--preserve-timestamps` to keep the original mtime of the file (default: mtime is updated to reflect the edit)
- Returns checksums before and after for verification
- Returns line counts before and after for auditing
- Returns `mtime_preserved` flag in the NDJSON response

### search
- Search file contents in parallel using the ripgrep engine
- Returns matches as NDJSON with line numbers and byte offsets

```bash
atomwrite search 'TODO' src/
atomwrite search --regex 'fn\s+\w+' src/
atomwrite search --count 'error' logs/
atomwrite search --files 'deprecated' src/
```

- Use `--regex` for regular expression patterns
- Use `--fixed` (`-F`) for literal string matching (no regex)
- Use `--word` (`-w`) to match whole words only
- Use `--case-insensitive` (`-i`) for case-insensitive search
- Use `--context N` (`-C`) for lines of context around matches
- Use `--max-count N` (`-m`) to limit matches per file
- Use `--invert` to show non-matching lines
- Use `--sort path` to sort results by file path
- Use `--count` (`-c`) for match counts per file
- Use `--files` (`-l`) for file paths only
- Use `--include` (`-g`) and `--exclude` for glob-based file filtering
- Exit code 1 means zero matches (not an error)

### replace
- Replace text across files with atomic writes
- Each modified file goes through the full atomic write sequence

```bash
atomwrite replace 'old_name' 'new_name' src/
atomwrite replace --regex 'v\d+\.\d+' 'v2.0' src/
atomwrite replace --dry-run 'before' 'after' src/
```

- Use `--dry-run` to preview replacements without modifying files
- Use `--preserve-timestamps` to keep the original mtime of modified files (default: mtime is updated to reflect the change)
- Returns per-file NDJSON with replacement count and checksums
- Emits a summary line with total files and replacements


## Utility Commands
### hash
- Compute BLAKE3 checksums for files or stdin

```bash
atomwrite hash src/main.rs
echo "data" | atomwrite hash --stdin
atomwrite hash --verify abc123 src/main.rs
```

### delete
- Delete files with optional backup and dry-run support

```bash
atomwrite delete src/old_file.rs
atomwrite delete --backup src/old_file.rs
atomwrite delete --dry-run src/old_file.rs
atomwrite delete --recursive tmp/
```

### count
- Count lines, blank lines and files grouped by extension

```bash
atomwrite count src/
atomwrite count --by-extension src/
```

### diff
- Compare two files using unified, stat or change-by-change output

```bash
atomwrite diff src/a.rs src/b.rs
atomwrite diff --stat src/a.rs src/b.rs
atomwrite diff --unified --context 5 src/a.rs src/b.rs
atomwrite diff --algorithm patience src/a.rs src/b.rs
```

### move
- Move or rename files atomically
- Falls back to copy-then-delete for cross-device moves

```bash
atomwrite move src/old.rs src/new.rs
```

### copy
- Copy files with BLAKE3 checksum verification after write

```bash
atomwrite copy src/main.rs backup/main.rs
```

### list
- List project file structure with optional metadata

```bash
atomwrite list src/
atomwrite list --long src/
atomwrite list --count-by-ext src/
```

### extract
- Extract fields from NDJSON lines or text columns

```bash
echo "a b c" | atomwrite extract 0 2
atomwrite search 'TODO' src/ | atomwrite extract path line_number
```

### calc
- Evaluate math expressions and unit conversions
- Powered by the fend engine with arbitrary precision

```bash
atomwrite calc "2 hours + 30 minutes to seconds"
atomwrite calc "sqrt(144) + 3^2"
atomwrite calc "10 GiB to MB"
```

- Quote expressions to prevent shell interpolation

### regex
- Generate regular expressions from example strings
- Powered by the grex engine

```bash
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"
atomwrite regex --digits --words "user_123" "admin_456"
```


## Advanced Commands
### scope
- Grammatical scoping with AST-based actions on code categories
- Supports Rust, Python, JavaScript/TypeScript and Go
- Use `--delete` to remove matched content
- Use `--action upper|lower|titlecase|squeeze` to transform matched text
- Use `--replace-with "text"` to replace matched content with custom text

```bash
atomwrite scope --query comments --delete src/main.rs
atomwrite scope --query fn --action upper src/lib.rs
atomwrite scope --query strings --action lower src/app.ts
atomwrite scope --pattern '($$$ARGS)' --action squeeze -l rust src/
atomwrite scope --query comments --replace-with "// updated" src/main.rs
```

- Use `--query` for prepared queries (comments, fn, strings, struct, etc)
- Use `--pattern` for custom AST patterns
- Use `--action` to specify the transformation

### backup
- Create timestamped backups with BLAKE3 checksums

```bash
atomwrite backup src/main.rs src/lib.rs
atomwrite backup --retention 30 src/config.toml
atomwrite backup --dry-run src/main.rs
```

- Use `--retention` to set backup retention period in days
- Use `--dry-run` to preview without creating backups

### rollback
- Restore files from a previous backup

```bash
atomwrite rollback src/main.rs --latest
atomwrite rollback src/main.rs --timestamp 2026-05-29T12-00-00
atomwrite rollback --verify --dry-run src/main.rs
```

- Use `--latest` to restore the most recent backup
- Use `--timestamp` to restore a specific backup
- Use `--verify` to validate BLAKE3 checksum before restoring
- Use `--dry-run` to preview without restoring

### apply
- Apply patches from stdin with auto-format detection
- Supports unified diff, SEARCH/REPLACE blocks, markdown-fenced and full file

```bash
cat fix.patch | atomwrite apply src/main.rs
cat changes.md | atomwrite apply --format markdown src/main.rs
cat fix.patch | atomwrite apply --backup src/main.rs
cat fix.patch | atomwrite apply --dry-run src/main.rs
```

- Use `--format` to force a specific patch format
- Use `--backup` to create a backup before applying
- Use `--dry-run` to preview without applying

### transform
- Structural AST search and rewrite powered by ast-grep
- Supports 306 programming languages
- Understands code syntax, not just text patterns

```bash
atomwrite transform --pattern 'println!($$$ARGS)' --rewrite 'tracing::info!($$$ARGS)' -l rust src/
atomwrite transform --pattern 'console.log($$$ARGS)' --rewrite 'logger.info($$$ARGS)' -l js src/
atomwrite transform --pattern '$EXPR.unwrap()' --rewrite '$EXPR?' -l rust src/
```

- Use `$VAR` for single AST node capture
- Use `$$$VAR` for multiple AST nodes capture
- Both `--pattern` and `--rewrite` are required

### batch
- Execute multiple operations from an NDJSON manifest
- Supports write, replace, delete, edit, hash, move and copy operations
- Use `--transaction` for all-or-nothing execution with automatic rollback

```bash
cat manifest.ndjson | atomwrite batch
cat manifest.ndjson | atomwrite batch --dry-run
```

- Each line in the manifest is one operation
- Returns per-operation results plus an aggregate summary
- Use `--dry-run` to validate the manifest without executing


## Global Flags
- `--workspace <PATH>` -- restrict all operations to this root directory
- `--verbose` / `-v` -- enable tracing output on stderr
- `--quiet` / `-q` -- suppress non-essential output
- `--color <auto|always|never>` -- control colored output
- `--no-color` -- disable colored output (equivalent to `--color never`)
- `--no-gitignore` -- do not respect .gitignore rules
- `--hidden` -- include hidden files and directories
- `--follow-symlinks` -- follow symbolic links
- `--threads <N>` / `-j <N>` -- number of parallel threads (0 = all cores)
- `--max-filesize <BYTES>` -- skip files larger than this limit
- `--json-schema` -- emit JSON schema for the subcommand output
- `--lang <LOCALE>` -- override display locale (en, pt-BR)
- `--timeout <SECONDS>` -- global operation timeout (0 = no timeout)
- `--grep <REGEX>` on `read` to filter returned lines to those matching a regex


## Configuration
- atomwrite requires no configuration files
- All behavior is controlled via command-line flags
- Use `--workspace` to set the project root boundary
- Use `--json-schema` to introspect output format at runtime
- Generate shell completions with `atomwrite completions bash` or auto-install with `atomwrite completions bash --install` (writes to XDG data dir)
- `ATOMWRITE_LANG`: override locale for translated messages
- `ATOMWRITE_WORKSPACE`: set the workspace root for path jail validation
- `NO_COLOR`: disable colored output when set (see https://no-color.org)
- `RAYON_NUM_THREADS`: override number of parallel threads


## Modification Time And Build Systems

By default, `edit` and `replace` update the file modification time (`mtime`) to the moment the write completes. This is the correct behavior for build systems that use `mtime` to detect source changes (cargo, make, cmake, gradle, sbt, bazel, ninja, msbuild).

What happens if you opt out of mtime updates:
- Cargo compares the mtime of each source file against the `dep-info` file in `target/.fingerprint/`
- When source mtime is older than dep-info mtime, cargo assumes nothing changed and skips the rebuild
- This produces a silent no-op (`Finished in 0.29s`) where the binary is stale but cargo reports success

When to preserve mtime with `--preserve-timestamps`:
- You are creating a backup or snapshot of the file and want to keep its original timestamp
- You are implementing a version control operation that mirrors historical state
- You are generating a reproducible build artifact where source timestamps must match recorded metadata
- You are writing to a file outside any build system context

For interactive agent workflows, the safe default is to let `atomwrite` update the mtime. The `mtime_preserved` field in the NDJSON response tells you whether the timestamp was preserved or updated, which is critical for diagnosing missed rebuilds in build systems.


## Integration With AI Agents
- Every subcommand produces deterministic NDJSON on stdout
- Every write includes a BLAKE3 checksum in the response
- The checksum eliminates the need for verification reads
- Use `--expect-checksum` for optimistic locking in concurrent workflows
- Use `--workspace` to sandbox agents within a project root
- Use `--dry-run` before destructive operations
- Batch mode replaces hundreds of individual tool calls
- Exit codes follow sysexits conventions for programmatic handling
- See [AGENTS.md](AGENTS.md) for the full agent integration contract


## Error Suggestions (v0.1.4)
- Every error envelope on stdout includes a `suggestion` field with actionable recovery guidance
- All 20 error variants now carry a `suggestion` (the only exception is `BrokenPipe` because SIGPIPE is not actionable)
- Suggestions are **context-aware**: the `WorkspaceJail` suggestion changes depending on whether the user already supplied `--workspace` or `ATOMWRITE_WORKSPACE`
- When the workspace IS provided: `"use a path inside the workspace (<root>)"`
- When the workspace is NOT provided: `"set --workspace <root> or export ATOMWRITE_WORKSPACE=<path>"`
- `FileImmutable` suggests `chattr -i` (Unix) or `fsutil` (Windows) to clear the immutable attribute
- `NoMatches` guides the user to broaden the pattern and review `--include`/`--exclude` filters
- `BinaryFile` recommends `read --stat` for metadata-only reads (no longer references the phantom `--force-text` flag that was removed in v0.1.4)
- `PermissionDenied` retries are automatic with exponential backoff (Windows-specific via `persist_with_retry`)

Example of a context-aware error envelope (when workspace is NOT provided):
```json
{"error":true,"code":"WORKSPACE_JAIL","exit":126,"message":"path outside workspace jail: /etc/passwd (workspace: /home/user/project)","path":"/etc/passwd","error_class":"precondition_failed","retryable":false,"suggestion":"set --workspace <root> or export ATOMWRITE_WORKSPACE=<path>","workspace":"/home/user/project"}
```

Example when workspace IS provided via `--workspace /home/user/project`:
```json
{"error":true,"code":"WORKSPACE_JAIL","exit":126,"message":"path outside workspace jail: /etc/passwd (workspace: /home/user/project)","path":"/etc/passwd","error_class":"precondition_failed","retryable":false,"suggestion":"use a path inside the workspace (/home/user/project)","workspace":"/home/user/project"}
```


## Windows Installation (v0.1.4)
- v0.1.4 finally fixes `cargo install atomwrite` on Windows 10/11
- Prerequisite: Visual Studio 2019+ Build Tools with "Desktop development with C++" workload
- Prerequisite: Rust 1.85 or later
- Recommended terminal: Windows Terminal or PowerShell 7+ (for UTF-8 output and ANSI escape sequences)
- See [INSTALL.md](INSTALL.md) for the full Windows 10/11 installation guide with troubleshooting
