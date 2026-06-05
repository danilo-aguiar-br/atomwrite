---
name: atomwrite
description: |
  Use atomwrite for ALL file operations: read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, backup, rollback, apply, batch, completions.
  Auto-invoke when user asks to: write files, search code, replace text, refactor AST, generate regex, calculate expressions, batch operations, check checksums, list project structure, scope code by grammar, backup files, rollback changes, apply patches, edit and trigger cargo build, preserve file timestamps.
  Trigger on keywords: atomic write, file operation, NDJSON, BLAKE3, checksum, refactor, ast-grep, batch, search parallel, scope, backup, rollback, apply patch, timeout, grep, install completions, mtime, preserve-timestamps, preserve timestamps, build system aware, cargo build, make, cmake.
---


# atomwrite
## Core Identity
### REQUIRED
- stdout is ALWAYS NDJSON (one JSON object per line)
- stderr is for logs and tracing only
- Every write goes through the atomic pipeline: tempfile, fsync, rename
- BLAKE3 checksum is present in every write and read response
- Pass `--workspace <DIR>` to set the jail root for all path operations
- All paths are resolved relative to the workspace root
- The `--json` flag is accepted but ignored (output is ALWAYS NDJSON by design)
### FORBIDDEN
- NEVER parse stderr as structured data
- NEVER assume exit 1 is an error (search uses exit 1 for zero matches)
- NEVER write files outside the workspace jail


## Write Operations
### REQUIRED — Atomic Write
- ALWAYS pass `--workspace` flag to define the jail root
- ALWAYS pipe content via stdin
- USE `--backup --retention N` for destructive overwrites
- USE `--expect-checksum <BLAKE3>` for optimistic locking (state drift detection)
- USE `--dry-run` before destructive writes to preview the operation
- USE `--append` to append content to end of existing file
- USE `--prepend` to insert content at beginning of existing file
- USE `--max-size <BYTES>` to limit accepted stdin size
- USE `--line-ending lf|crlf|cr|auto` to normalize line endings (default: auto)
- Response includes `checksum` (BLAKE3) and `bytes_written`
### FORBIDDEN
- NEVER write without `--workspace`
- NEVER pass file content as a CLI argument
### Correct Pattern — Write
```bash
echo "content" | atomwrite --workspace . write target.rs
```
### Correct Pattern — Write with Backup
```bash
cat new_config.toml | atomwrite --workspace . write --backup --retention 3 config.toml
```
### Correct Pattern — Optimistic Locking
```bash
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')
echo "updated" | atomwrite --workspace . write --expect-checksum "$CS" src/main.rs
```
### Correct Pattern — Append and Prepend
```bash
echo "// new line" | atomwrite --workspace . write --append src/main.rs
echo "// header" | atomwrite --workspace . write --prepend src/main.rs
```


## Read Operations
### REQUIRED
- USE `read` for file content with metadata
- USE `read --stat` for metadata only (no body)
- USE `read --lines 1:50` for partial reads by line range
- USE `read --line N` to read a single line with optional context via `--context N`
- USE `read --head N` to read the first N lines
- USE `read --tail N` to read the last N lines
- USE `read --format raw` for raw content without JSON envelope
- USE `read --grep <REGEX>` to filter returned lines to those matching a regex (v0.1.2+)
- USE `read --verify-checksum <BLAKE3>` for integrity verification
- Response includes `checksum`, `size`, `lines`
### Correct Pattern — Read
```bash
atomwrite --workspace . read src/main.rs
```
### Correct Pattern — Partial Read
```bash
atomwrite --workspace . read --lines 1:50 src/main.rs
atomwrite --workspace . read --head 20 src/main.rs
atomwrite --workspace . read --tail 10 src/main.rs
```
### Correct Pattern — Line with Context
```bash
atomwrite --workspace . read --line 42 --context 5 src/main.rs
```
### Correct Pattern — Metadata Only
```bash
atomwrite --workspace . read --stat src/main.rs
```


## Search Operations
### REQUIRED
- USE `search` for parallel ripgrep-powered search across files
- Exit code 1 means zero matches found (NOT an error)
- USE `--include '*.rs'` to filter by file extension
- USE `--exclude '*.log'` to exclude files by glob pattern
- USE `--context N` for surrounding lines around each match
- USE `--fixed` (`-F`) for literal string matching (no regex)
- USE `--regex` (`-e`) to explicitly force regex mode
- USE `--word` (`-w`) for word-boundary matching
- USE `--case-insensitive` (`-i`) for case-insensitive search
- USE `--smart-case` (`-S`) for insensitive when pattern is lowercase
- USE `--count` (`-c`) for match counts per file instead of full matches
- USE `--files` (`-l`) for file paths only
- USE `--max-count N` (`-m`) to limit matches per file
- USE `--multiline` (`-U`) for multi-line matching
- USE `--invert` to show lines that do NOT match
- USE `--sort path|modified|created|none` to sort results
- Response is NDJSON with one object per match
### FORBIDDEN
- NEVER treat exit code 1 as a failure in search
### Correct Pattern — Search
```bash
atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs'
```
### Correct Pattern — Search with Context
```bash
atomwrite --workspace . search 'unsafe' src/ --context 3
```
### Correct Pattern — Count per File
```bash
atomwrite --workspace . search 'unwrap' src/ --count --sort path
```


## Replace Operations
### REQUIRED
- USE `replace` for bulk text substitution with atomic writes
- ALWAYS use `--dry-run` first for destructive replacements
- USE `--regex` for regex-based patterns
- USE `--word` for word-boundary matching
- USE `--literal` (`-F`) to treat pattern as literal string
- USE `--include '*.rs'` to filter files by extension
- USE `--exclude '*.log'` to exclude files by glob pattern
- USE `--preview` to show diff without writing
- USE `--max-replacements N` (`-n`) to limit replacements per file
- USE `--expect-checksum <BLAKE3>` for optimistic locking
- USE `--backup` to create backup before modifying
- USE `--preserve-timestamps` to keep the original mtime of modified files (default: mtime is updated to reflect the change). Add this when integrating with build systems (cargo, make, cmake) that need stable timestamps
- Response includes `matches`, `files_modified`, per-file checksums, and `mtime_preserved` field
### FORBIDDEN
- NEVER run replace without `--dry-run` first
### Correct Pattern — Replace
```bash
atomwrite --workspace . replace --dry-run 'old_api' 'new_api' src/
atomwrite --workspace . replace 'old_api' 'new_api' src/
```
### Correct Pattern — Regex Replace
```bash
atomwrite --workspace . replace --regex 'v\d+\.\d+' 'v2.0' src/ --include '*.toml'
```
### Correct Pattern — Replace With Preserved Mtime
```bash
# v0.1.3+: keep the original mtime of all replaced files
atomwrite --workspace . replace --preserve-timestamps 'old_api' 'new_api' src/
```


## Edit Operations
### REQUIRED
- USE `edit` for surgical modifications by line number or text marker
- USE `--old "text" --new "text"` for exact text replacement (repeatable for multiple)
- USE `--after-line N` for inserting content after a specific line
- USE `--before-line N` for inserting content before a specific line
- USE `--range N:M` for replacing a line range
- USE `--delete-range N:M` for deleting a line range
- USE `--after-match "text"` for inserting content after first match of text
- USE `--before-match "text"` for inserting content before first match
- USE `--between "start" "end"` for replacing content between two markers
- USE `--fuzzy auto|off|aggressive` to control fuzzy matching (default: auto)
- USE `--multi` to apply multiple edits from NDJSON stdin in a single atomic write
- USE `--expect-checksum <BLAKE3>` for optimistic locking
- USE `--line-ending lf|crlf|cr|auto` to normalize line endings
- USE `--preserve-timestamps` to keep the original file mtime (default: mtime is updated to reflect the edit). Add this when integrating with build systems (cargo, make, cmake) that need stable timestamps
- Pipe new content via stdin when using `--range`, `--after-line`, or `--before-line`
- Note: `edit` and `replace` now update the mtime of the file by default (v0.1.3+). This is the correct behavior for cargo/make/cmake so they detect the change. For backup or reproducible builds, pass `--preserve-timestamps` to keep the original timestamp
### Correct Pattern — Edit by Text
```bash
atomwrite --workspace . edit src/main.rs --old "old_text" --new "new_text"
```
### Correct Pattern — Edit With Preserved Mtime
```bash
# v0.1.3+: keep the original file mtime (e.g. for backup or snapshot workflows)
atomwrite --workspace . edit --preserve-timestamps src/main.rs --old "old_text" --new "new_text"
```
### Correct Pattern — Verify Mtime Was Preserved
```bash
# v0.1.3+: read the mtime_preserved field from the NDJSON response
atomwrite --workspace . edit src/main.rs --old "old" --new "new" | jaq -r '.mtime_preserved'
```
### Correct Pattern — Read Full NDJSON Edit Response
```bash
# v0.1.3+: the EditOutput envelope includes mtime_preserved as the last field
atomwrite --workspace . edit src/main.rs --old "old" --new "new" | jaq 'del(.checksum_before, .checksum_after) | {type, mtime_preserved, bytes_after}'
```
### Correct Pattern — Multiple Replacements
```bash
atomwrite --workspace . edit src/main.rs --old "foo" --new "bar" --old "baz" --new "qux"
```
### Correct Pattern — Insert After Line
```bash
echo "new_line_content" | atomwrite --workspace . edit src/main.rs --after-line 10
```
### Correct Pattern — Delete Range
```bash
atomwrite --workspace . edit src/main.rs --delete-range 5:10
```
### Correct Pattern — Replace Between Markers
```bash
echo "new block" | atomwrite --workspace . edit src/main.rs --between "// START" "// END"
```
### Correct Pattern — Multiple Edits via NDJSON
```bash
echo '{"old":"foo","new":"bar"}
{"old":"baz","new":"qux"}' | atomwrite --workspace . edit --multi src/main.rs
```


## Transform Operations (AST)
### REQUIRED
- USE `transform` for structural refactoring via ast-grep
- ALWAYS specify `--lang` (`-l`) for the target language
- USE `$NAME` for single AST node captures
- USE `$$$ARGS` for multiple AST node captures (variadic)
- 306 languages supported via ast-grep
- USE `--dry-run` to preview transformations
- USE `--backup` to create backup before modifying
- USE `--include` and `--exclude` to filter files by extension
- Both `--pattern` and `--rewrite` are REQUIRED (no search-only mode)
### Correct Pattern — Transform
```bash
atomwrite --workspace . transform -p 'console.log($$$A)' -r 'logger.info($$$A)' -l js src/
```
### Correct Pattern — Rust Refactor
```bash
atomwrite --workspace . transform -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/
```
### Correct Pattern — Dry Run
```bash
atomwrite --workspace . transform --dry-run -p 'old_fn($$$A)' -r 'new_fn($$$A)' -l rust src/
```


## Scope Operations (Grammatical Scoping)
### REQUIRED
- USE `scope` to select AST categories and apply actions on matched code
- ALWAYS specify `--lang` for the target language
- USE `--query` for prepared queries by language (see list below)
- USE `--pattern` for custom AST patterns
- USE `--delete` to remove matched content
- USE `--action upper|lower|titlecase|squeeze` for text transformations
- USE `--replace-with "text"` for custom replacement
- USE `--include '*.rs'` to filter files by extension
- USE `--exclude '*.log'` to exclude files by glob pattern
- USE `--backup` to create backup before modifying
- USE `--dry-run` to preview changes
### Prepared Queries — Rust
- `comments`, `doc-comment`, `strings`
- `fn`, `pub-fn`, `async-fn`, `unsafe-fn`, `test-fn`
- `struct`, `pub-struct`, `enum`, `pub-enum`
- `trait`, `impl`, `mod`, `use`
- `closure`, `unsafe`, `attribute`, `derive`
- `return`, `match`, `if-let`, `while-let`
- `for`, `loop`, `const`, `static`
- `type-alias`, `macro-rules`
### Prepared Queries — Python
- `comments`, `strings`
- `class`, `def`, `async-def`, `lambda`
- `import`, `from-import`
- `with`, `for`, `while`
- `decorator`, `try-except`
### Prepared Queries — JavaScript and TypeScript
- `comments`, `strings`
- `fn`, `arrow-fn`, `async-fn`
- `class`, `import`, `export`
- `try-catch`, `const`, `let`
### Prepared Queries — Go
- `fn`, `struct`, `interface`
- `goroutine`, `defer`, `import`
- `const`, `var`
### Correct Pattern — Scope
```bash
atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run
atomwrite --workspace . scope src/ --lang rust --query fn --action upper --dry-run
atomwrite --workspace . scope src/ --lang python --query def --action lower
```


## Batch Operations
### REQUIRED
- USE `batch` for multiple operations in a single call
- Input is NDJSON on stdin (one JSON object per line)
- Each line requires an `op` field: `write`, `replace`, `delete`, `edit`, `move`, `copy`, `hash`
- For `move` and `copy`: use `source` field (origin) and `target` field (destination)
- USE `--file <PATH>` to read manifest from file instead of stdin
- USE `--transaction` for all-or-nothing execution with automatic rollback on failure
- USE `--dry-run` to preview the entire batch
- USE `--input-schema` to get the JSON Schema for the input manifest format
- Response is NDJSON with one result per operation
### Correct Pattern — Batch with Write and Delete
```bash
echo '{"op":"write","target":"a.txt","content":"hello"}
{"op":"delete","target":"tmp.log"}' | atomwrite --workspace . batch
```
### Correct Pattern — Batch with Move and Copy
```bash
echo '{"op":"move","source":"src/old.rs","target":"src/new.rs"}
{"op":"copy","source":"src/template.rs","target":"src/module.rs"}' | atomwrite --workspace . batch
```
### Correct Pattern — Transactional Batch
```bash
cat ops.ndjson | atomwrite --workspace . batch --transaction --dry-run
cat ops.ndjson | atomwrite --workspace . batch --transaction
```
### Correct Pattern — Batch from File
```bash
atomwrite --workspace . batch --file ops.ndjson --transaction
```


## Hash Operations
### REQUIRED
- USE `hash` for standalone BLAKE3 checksums
- Accepts one or more file paths
- USE `--verify <BLAKE3>` to check file hash against expected value
- USE `--stdin` to hash content from stdin
- USE `--recursive` (`-r`) to hash directories recursively
- Response includes `path` and `checksum` per file
### Correct Pattern — Hash
```bash
atomwrite --workspace . hash src/main.rs
atomwrite --workspace . hash src/*.rs
atomwrite --workspace . hash --verify abc123 src/main.rs
echo "content" | atomwrite hash --stdin
```


## Delete Operations
### REQUIRED
- USE `delete` for atomic file removal
- USE `--backup --retention N` to keep backups before deletion
- USE `--recursive` (`-r`) to remove directories recursively
- USE `--include '*.log'` to filter by extension
- USE `--exclude '*.rs'` to exclude by extension
- USE `--yes` (`-y`) to skip confirmation
- USE `--dry-run` to preview
### Correct Pattern — Delete
```bash
atomwrite --workspace . delete --backup --retention 1 tmp/scratch.rs
atomwrite --workspace . delete --recursive --include '*.log' --dry-run logs/
```


## Diff Operations
### REQUIRED
- USE `diff` for comparing two files
- USE `--unified` for unified diff format
- USE `--stat` for summary statistics only
- USE `--context N` (`-C`) for context lines in diff (default: 3)
- USE `--algorithm myers|patience|lcs` to choose diff algorithm (default: patience)
- Response includes structured NDJSON diff hunks
### Correct Pattern — Diff
```bash
atomwrite --workspace . diff src/old.rs src/new.rs
atomwrite --workspace . diff --stat src/old.rs src/new.rs
atomwrite --workspace . diff --unified --context 5 src/old.rs src/new.rs
```


## Move and Copy Operations
### REQUIRED
- USE `move` for atomic rename/move within the workspace
- USE `copy` for atomic copy with checksum verification
- Both respect the workspace jail
- USE `--force` to overwrite destination if it exists
- USE `--dry-run` to preview
- USE `--backup` to backup destination if it exists
- `copy` accepts `--recursive` for directories and `--preserve` for timestamps
### Correct Pattern — Move
```bash
atomwrite --workspace . move src/old.rs src/new.rs
atomwrite --workspace . move --force src/old.rs src/existing.rs
```
### Correct Pattern — Copy
```bash
atomwrite --workspace . copy src/template.rs src/new_module.rs
atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/
```


## List Operations
### REQUIRED
- USE `list` for directory and file listing
- USE `--include '*.rs'` to filter by extension
- USE `--exclude '*.log'` to exclude by extension
- USE `--long` for size and modification time
- USE `--depth N` to limit directory depth
- USE `--count-by-ext` for file count grouped by extension
- USE `--all` to include hidden files
### Correct Pattern — List
```bash
atomwrite --workspace . list --include '*.rs' src/
atomwrite --workspace . list --long --depth 2 src/
atomwrite --workspace . list --count-by-ext src/
atomwrite --workspace . list --all --long src/
```


## Count Operations
### REQUIRED
- USE `count` for file and line counting
- USE `--by-extension` to group counts by file extension
- USE `--by-size` with `--top N` to list largest files
- USE `--include` and `--exclude` to filter
- Response includes `files`, `lines`, `bytes`
### Correct Pattern — Count
```bash
atomwrite --workspace . count --include '*.rs' src/
atomwrite --workspace . count --by-extension src/
atomwrite --workspace . count --by-size --top 20 src/
```


## Extract Operations
### REQUIRED
- USE `extract` for NDJSON field extraction from piped input
- Pass field names as positional arguments to select specific JSON fields
- USE `--delimiter <SEP>` for text mode with custom separator
### Correct Pattern — Extract
```bash
atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number
```


## Calc Operations
### REQUIRED
- USE `calc` for math expressions and unit conversions
- ALWAYS quote the expression
- USE `--stdin` to read expressions from stdin (one per line)
- No `--workspace` needed (stateless)
### Correct Pattern — Calc
```bash
atomwrite calc "2 hours + 30 minutes to seconds"
atomwrite calc "1.5 GiB to bytes"
atomwrite calc "sqrt(144) + 2^10"
```


## Regex Operations
### REQUIRED
- USE `regex` for generating regex from examples
- Pass 3+ examples for accurate patterns
- USE `--digits` (`-d`) for `\d` generalization
- USE `--words` (`-w`) for `\w` generalization
- USE `--spaces` (`-s`) for `\s` generalization
- USE `--repetitions` (`-r`) to detect repetitions
- USE `--case-insensitive` (`-i`) for case-insensitive matching
- USE `--no-anchors` to remove `^` and `$` from result
- USE `--stdin` to read examples from stdin (one per line)
- No `--workspace` needed (stateless)
### Correct Pattern — Regex
```bash
atomwrite regex "192.168.1.1" "10.0.0.255" --digits
atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" --digits
atomwrite regex -d -w -s -r "example1" "example2"
```


## Backup Operations
### REQUIRED
- USE `backup` to create timestamped backups with BLAKE3 checksums
- USE `--retention N` to control how many backups to keep (default: 5)
- USE `--output-dir <DIR>` to direct backups to a specific directory
- USE `--dry-run` to preview
- Note: `backup` uses `fs::copy` directly (not the atomic write pipeline), so the backup file inherits the SOURCE mtime, not the moment of backup creation. This is intentional and matches POSIX behavior for file copies
### Correct Pattern — Backup
```bash
atomwrite --workspace . backup src/config.toml
atomwrite --workspace . backup src/main.rs src/lib.rs --retention 3
atomwrite --workspace . backup src/main.rs --output-dir /tmp/backups/
```


## Rollback Operations
### REQUIRED
- USE `rollback` to restore a file from a previous backup
- USE `--latest` to restore the most recent backup (default)
- USE `--timestamp YYYYMMDD_HHMMSS` to restore a specific backup
- USE `--verify` to check BLAKE3 checksum after restoration
- USE `--dry-run` to preview
### Correct Pattern — Rollback
```bash
atomwrite --workspace . rollback src/config.toml
atomwrite --workspace . rollback src/config.toml --timestamp 20260530_120000 --verify
```


## Apply Operations (Patch)
### REQUIRED
- USE `apply` to apply patches from stdin to a target file
- Auto-detects patch format: unified diff, SEARCH/REPLACE blocks, markdown-fenced, full file
- USE `--format auto|unified|search-replace|full|markdown` to force format
- USE `--backup` to create backup before patching
- USE `--dry-run` to preview
- Note (v0.1.3+): `apply` updates the mtime of the target file by default (same as `edit` and `replace`). This ensures build systems detect the change. Use `--preserve-timestamps` to opt out (not yet exposed in the CLI for `apply`; if needed, edit the target before/after)
### Correct Pattern — Apply
```bash
echo "new content" | atomwrite --workspace . apply src/file.txt --format full
git diff src/file.txt | atomwrite --workspace . apply src/file.txt
```


## Completions
### REQUIRED
- USE `completions` to generate shell completions
- Supports `bash`, `zsh`, `fish`, `elvish`, `powershell`
### Correct Pattern — Completions
```bash
atomwrite completions bash > ~/.local/share/bash-completion/completions/atomwrite
atomwrite completions zsh > ~/.zfunc/_atomwrite
```


## Common Pipelines
### Correct Pattern — Optimistic Locking (Read, Modify, Write)
```bash
CS=$(atomwrite --workspace . read src/config.rs | jaq -r '.checksum')
echo "new content" | atomwrite --workspace . write --expect-checksum "$CS" src/config.rs
```
### Correct Pattern — Search and Extract Fields
```bash
atomwrite --workspace . search 'TODO' src/ --include '*.rs' | atomwrite extract path line_number
```
### Correct Pattern — Hash for Auditing
```bash
atomwrite --workspace . hash src/main.rs src/lib.rs | jaq -r '.checksum'
```
### Correct Pattern — Structured Diff
```bash
atomwrite --workspace . diff src/old.rs src/new.rs | jaq '.type'
```
### Correct Pattern — Transactional Batch with Verification
```bash
cat ops.ndjson | atomwrite --workspace . batch --transaction --dry-run
cat ops.ndjson | atomwrite --workspace . batch --transaction
```
### Correct Pattern — Verify mtime Behavior of Edit (v0.1.3+)
```bash
# Edit and confirm whether the mtime was preserved or updated (boolean)
atomwrite --workspace . edit src/main.rs --old "old" --new "new" | jaq -r '.mtime_preserved'
```
### Correct Pattern — Edit and Trigger Build Without Manual Touch (v0.1.3+)
```bash
# Default behavior of edit updates the mtime, so cargo/make/cmake detect the change
atomwrite --workspace . edit src/main.rs --old "old" --new "new"
cargo build
```


## Agent-First Patterns (v0.1.3+)

### Edit Source File and Trigger Build Without Manual Touch

```bash
# New default: edit updates the mtime, so cargo/make/cmake rebuild automatically
atomwrite --workspace . edit src/main.rs --old "old_text" --new "new_text"
cargo build  # rebuilds without needing `touch` first
```

### Read mtime_preserved From Edit Response

```bash
# Parse the NDJSON response to verify whether the timestamp was kept
atomwrite --workspace . edit src/main.rs --old "old" --new "new" | jaq -r '.mtime_preserved'
```

### Preserve Original mtime For Backup or Snapshot Workflows

```bash
# Opt back into the v0.1.2 behavior of preserving the original file mtime
atomwrite --workspace . edit --preserve-timestamps src/snapshot.rs --old "old" --new "new"
atomwrite --workspace . replace --preserve-timestamps 'old_api' 'new_api' src/
```

### Verify Edit Did Not Silently Skip a Build

```bash
# Diagnostic: confirm the mtime was updated, not preserved
result=$(atomwrite --workspace . edit src/main.rs --old "old" --new "new" | jaq -r '.mtime_preserved')
if [ "$result" = "true" ]; then
  echo "WARNING: mtime was preserved. Build systems may skip the rebuild. Use --preserve-timestamps=false or pass it explicitly."
fi
```


## Error Handling
### REQUIRED
- CHECK exit code first before parsing stdout
- PARSE stdout JSON when `error: true` for structured error details
- USE `error_class` to determine retry strategy
- RETRY when `retryable: true`
- USE `suggestion` field for actionable remediation
- EXPECT that `suggestion` is context-aware: `WorkspaceJail` differs based on whether `--workspace` was supplied
- TRUST `suggestion` for `FileImmutable` (mentions `chattr -i` / `fsutil`), `NoMatches` (broaden pattern), and `BinaryFile` (use `read --stat`)
- NOTE that only `BrokenPipe` (SIGPIPE) returns no `suggestion` because it is not actionable
### FORBIDDEN
- NEVER ignore non-zero exit codes (except exit 1 in search)
- NEVER parse stderr for error data
- NEVER retry when `retryable: false`
- NEVER invent suggestions that are not in the response (the `suggestion` field is the single source of truth)
### Correct Pattern — Error Handling
```bash
output=$(atomwrite --workspace . read missing.txt 2>/dev/null)
exit_code=$?
if [ $exit_code -ne 0 ]; then
  echo "$output" | jaq '{code: .code, class: .error_class, suggestion: .suggestion, workspace: .workspace}'
fi
```


## Windows 10/11 Support (v0.1.4)
### REQUIRED
- VERIFY Visual Studio 2019+ Build Tools with C++ workload is installed before `cargo install atomwrite`
- VERIFY Rust 1.85 or later is installed
- USE Windows Terminal or PowerShell 7+ for proper UTF-8 output and ANSI escape sequences
- TRUST `init_console` to set code page 65001 and `ENABLE_VIRTUAL_TERMINAL_PROCESSING` automatically
### FORBIDDEN
- NEVER use `cmd.exe` legacy console for output (mojibake expected)
- NEVER rely on `cargo install atomwrite` working on v0.1.3 (broken on Windows 10/11; fix is in v0.1.4)
### Correct Pattern — Windows Install
```powershell
rustup default stable
rustup target add x86_64-pc-windows-msvc
cargo install atomwrite --locked --version '^0.1.4'
atomwrite --version  # NDJSON output
```


## Cross-Compile Validation (v0.1.4)
### REQUIRED
- RUN `cargo test --test cross_compile_check -- --ignored` before any release that touches `#[cfg(windows)]` code
- INSTALL Windows targets: `rustup target add x86_64-pc-windows-gnu` and `i686-pc-windows-gnu`
- ON Linux, INSTALL mingw-w64: `mingw64-gcc` (Fedora) or `mingw-w64` (Ubuntu) and `mingw32-gcc` for 32-bit
- TRUST the gate to fail on any `E0433`, `E0308`, or `E0507` regression in Windows-only code
### Correct Pattern — Cross-Compile Gate
```bash
rustup target add x86_64-pc-windows-gnu i686-pc-windows-gnu x86_64-pc-windows-msvc
cargo test --test cross_compile_check -- --ignored
```


## Exit Codes
### REQUIRED — Full Reference
- `0` — success
- `1` — no matches (search/replace found zero results, NOT an error)
- `4` — not found (file or directory does not exist)
- `13` — permission denied
- `28` — disk full
- `30` — quota exceeded
- `65` — invalid input (malformed arguments or content)
- `73` — cross-device (move across filesystem boundaries)
- `74` — I/O error (generic filesystem failure)
- `78` — config invalid (malformed configuration)
- `81` — checksum verify failed (BLAKE3 hash mismatch on read or hash)
- `82` — state drift (checksum mismatch, optimistic locking failed)
- `85` — FIFO detected (named pipe cannot be atomically written)
- `86` — device file detected (block or character device)
- `126` — workspace jail violation (path escapes workspace root)
- `127` — symlink blocked (symlink target outside workspace)
- `128` — immutable (file marked immutable)
- `130` — SIGINT (interrupted by user)
- `141` — SIGPIPE (broken pipe)
- `143` — SIGTERM (terminated by signal)
- `255` — internal error (unexpected failure)


## Error JSON Schema
### REQUIRED — Fields
- `error` (bool) — always `true` when an error occurs
- `code` (string) — machine-readable error code (see full list below)
- `exit` (u8) — exit code number
- `message` (string) — human-readable description
- `path` (string, optional) — file path involved in the error
- `error_class` (string) — one of: `permanent`, `transient`, `conflict`, `precondition_failed`
- `retryable` (bool) — whether the operation can be retried
- `suggestion` (string, optional) — actionable remediation step (context-aware for `WorkspaceJail`)
- `workspace` (string, optional) — current workspace jail root (v0.1.4+, GAP 13 fix)
### REQUIRED — Full Error Code List (20 codes)
- `WORKSPACE_JAIL` (exit 126, precondition_failed, not retryable)
- `SYMLINK_BLOCKED` (exit 127, precondition_failed, not retryable)
- `FILE_NOT_FOUND` (exit 4, permanent, not retryable)
- `PERMISSION_DENIED` (exit 13, transient, retryable via `persist_with_retry` on Windows)
- `CHECKSUM_VERIFY_FAILED` (exit 81, conflict, not retryable)
- `STATE_DRIFT` (exit 82, conflict, not retryable)
- `DISK_FULL` (exit 28, transient, retryable)
- `QUOTA_EXCEEDED` (exit 30, transient, retryable)
- `CROSS_DEVICE` (exit 73, permanent, not retryable)
- `IO_ERROR` (exit 74, transient, retryable)
- `FIFO_DETECTED` (exit 85, precondition_failed, not retryable)
- `DEVICE_FILE` (exit 86, precondition_failed, not retryable)
- `FILE_IMMUTABLE` (exit 128, precondition_failed, not retryable)
- `BINARY_FILE` (exit 65, permanent, not retryable)
- `NO_MATCHES` (exit 1, permanent, not retryable — by design, not an error)
- `INVALID_INPUT` (exit 65, permanent, not retryable)
- `CONFIG_INVALID` (exit 78, permanent, not retryable)
- `BROKEN_PIPE` (exit 141, transient, not retryable — SIGPIPE not actionable)
- `INTERNAL_ERROR` (exit 255, permanent, not retryable — file a bug)
### REQUIRED — Retry Strategy by Class
- `permanent` — NEVER retry (caller bug or invalid input)
- `transient` — RETRY with exponential backoff (1s, 2s, 4s, 8s, max 30s)
- `conflict` — RETRY only after re-reading state (e.g. re-fetch checksum)
- `precondition_failed` — NEVER retry; fix the precondition (path, permissions, type)


## Global Flags
### REQUIRED — Reference
- `--workspace <DIR>` — set the workspace jail root (REQUIRED for file operations)
- `--max-filesize <BYTES>` — maximum accepted file size in bytes (default: 1 GiB)
- `--threads <N>` / `-j` — number of parallel threads (0 = all cores, env: `RAYON_NUM_THREADS`)
- `--timeout <SECONDS>` — global operation timeout in seconds, 0 means no timeout (v0.1.2+, default: 0)
- `--json-schema` — print the output JSON schema for any subcommand
- `--json` — accepted for compatibility but ignored (output is ALWAYS NDJSON)
- `--color auto|always|never` — control colored output
- `--no-color` — disable colored output (equivalent to `--color never`)
- `--no-gitignore` — do not respect `.gitignore` files
- `--hidden` — include hidden files and directories
- `--follow-symlinks` — follow symbolic links during traversal
- `--verbose` / `-v` — increase log verbosity on stderr (-v info, -vv debug, -vvv trace)
- `--quiet` / `-q` — decrease verbosity (-q error, -qq off)
- `--lang <LOCALE>` — override display locale (en, pt-BR) via `ATOMWRITE_LANG` env


## JSON Schema Introspection
### REQUIRED
- USE `--json-schema` flag to get the output schema for any subcommand
- USE schema output for programmatic validation of responses
- REFER to versioned schemas in `docs/schemas/` for stable contracts
- DO NOT re-parse `--json-schema` output on every call; cache the schema locally
### Correct Pattern — Schema
```bash
atomwrite write --json-schema
atomwrite search --json-schema
```


## Versioned Schemas (v0.1.4)
### REQUIRED
- KNOW that stable JSON Schemas are committed under `docs/schemas/`
- KNOW that `error-output.schema.json` is the contract for all error envelopes
- KNOW that the schema field `workspace` (string, optional) was added in v0.1.4
- USE the versioned schema to validate responses in your agent pipeline
- NOT invent your own parsing rules; trust the versioned schema as source of truth


## Tests and Quality Gates (v0.1.4)
### REQUIRED — Quality Posture
- 300+ tests in 34 test suites pass with zero regressions
- 8 official gates pass on every commit: `fmt`, `clippy`, `build`, `test`, `doc`, `deny`, `audit`, `msrv`
- 3 cross-compile targets pass: `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, `x86_64-pc-windows-msvc`
- Cargo deny ignores the pre-existing `RUSTSEC-2026-0009` in `time` 0.3.45
- MSRV is Rust 1.85 stable
### FORBIDDEN
- NEVER publish a release without all 8 gates passing
- NEVER publish a release without the 3 cross-compile targets passing
- NEVER accept "works on my Linux" as a release quality bar


## v0.1.4 Migration Quick Reference
### REQUIRED — Know What Changed Since v0.1.3
- GAP 14 fix: `cargo install atomwrite` now works on Windows 10/11 (was broken in v0.1.3)
- GAP 13 fix: error suggestions are now context-aware (WorkspaceJail suggestion changes based on whether `--workspace` was supplied)
- GAP 13 fix: all 20 error variants now carry actionable `suggestion` fields
- GAP 13 fix: phantom `--force-text` flag reference removed from BinaryFile suggestions
- Schema: `workspace` field added to error output envelope
- New tests: `tests/cross_compile_check.rs` with 3 gated cross-compile tests
- New tests: 7 unit tests + 1 integration test for error suggestion context
- Bilingual docs: 22 markdown files updated across 3 audit rounds
- DO NOT upgrade from v0.1.3 to v0.1.4 if you depend on phantom `--force-text` behavior
