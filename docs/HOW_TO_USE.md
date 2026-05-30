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
- Returns checksums before and after for verification
- Returns line counts before and after for auditing

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
- Use `--count` for match counts per file
- Use `--files` for file paths only
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
- Actions: delete, upper, lower, titlecase, squeeze, replace

```bash
atomwrite scope --query comments --action delete src/main.rs
atomwrite scope --query functions --action upper src/lib.rs
atomwrite scope --query strings --action lower src/app.ts
atomwrite scope --pattern '($$$ARGS)' --action squeeze -l rust src/
atomwrite scope --query comments --action replace --replacement "// updated" src/main.rs
```

- Use `--query` for prepared queries (comments, functions, strings, etc)
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
atomwrite transform --pattern 'unwrap()' -l rust src/
```

- Use `$VAR` for single AST node capture
- Use `$$$VAR` for multiple AST nodes capture
- Omit `--rewrite` for search-only mode

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
- `--no-gitignore` -- do not respect .gitignore rules
- `--hidden` -- include hidden files and directories
- `--follow-symlinks` -- follow symbolic links
- `--threads <N>` / `-j <N>` -- number of parallel threads (0 = all cores)
- `--max-filesize <BYTES>` -- skip files larger than this limit
- `--json-schema` -- emit JSON schema for the subcommand output
- `--lang <LOCALE>` -- override display locale (en, pt-BR)


## Configuration
- atomwrite requires no configuration files
- All behavior is controlled via command-line flags
- Use `--workspace` to set the project root boundary
- Use `--json-schema` to introspect output format at runtime
- Generate shell completions with `atomwrite completions bash`
- `ATOMWRITE_LANG`: override locale for translated messages


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
