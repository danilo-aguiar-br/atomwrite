---
name: atomwrite
description: |
  Use atomwrite for ALL file operations: read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, backup, rollback, apply, batch, completions.
  Auto-invoke when user asks to: write files, search code, replace text, refactor AST, generate regex, calculate expressions, batch operations, check checksums, list project structure, scope code by grammar, backup files, rollback changes, apply patches.
  Trigger on keywords: atomic write, file operation, NDJSON, BLAKE3, checksum, refactor, ast-grep, batch, search parallel, scope, backup, rollback, apply patch.
---


## Core Identity
### REQUIRED
- stdout is ALWAYS NDJSON (one JSON object per line)
- stderr is for logs and tracing only
- Every write goes through the atomic pipeline: tempfile, fsync, rename
- BLAKE3 checksum is present in every write and read response
- Pass `--workspace <DIR>` to set the jail root for all path operations
- All paths are resolved relative to the workspace root
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
echo "updated" | atomwrite --workspace . write --expect-checksum abc123 config.toml
```


## Read Operations
### REQUIRED
- USE `read` for file content with metadata
- USE `read --stat` for metadata only (no body)
- USE `read --lines 1:50` for partial reads by line range
- USE `read --verify-checksum <BLAKE3>` for integrity verification
- Response includes `checksum`, `size`, `lines`
### Correct Pattern — Read
```bash
atomwrite --workspace . read src/main.rs
```
### Correct Pattern — Partial Read
```bash
atomwrite --workspace . read --lines 1:50 src/main.rs
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
- USE `--context N` for surrounding lines around each match
- USE `--fixed` (`-F`) for literal string matching (no regex)
- USE `--count` (`-c`) for match counts per file instead of full matches
- USE `--files` (`-l`) for file paths only
- USE `--max-count N` (`-m`) to limit matches per file
- USE `--invert` to show lines that do NOT match
- USE `--sort path` to sort results by file path
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


## Replace Operations
### REQUIRED
- USE `replace` for bulk text substitution with atomic writes
- ALWAYS use `--dry-run` first for destructive replacements
- USE `--regex` for regex-based patterns
- USE `--word` for word-boundary matching
- Response includes `matches`, `files_modified`, per-file checksums
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


## Edit Operations
### REQUIRED
- USE `edit` for surgical modifications by line number or text marker
- USE `--old "text" --new "text"` for exact text replacement within a file
- USE `--fuzzy auto|off|aggressive` to control fuzzy matching in `--old/--new` mode (default: auto)
- USE `--multi` to apply multiple edit operations from NDJSON stdin in a single atomic write
- USE `--after-line N` for inserting content after a specific line
- USE `--before-line N` for inserting content before a specific line
- USE `--range N:M` for replacing a line range
- Pipe new content via stdin when using `--range`, `--after-line`, or `--before-line`
### Correct Pattern — Edit by Text
```bash
atomwrite --workspace . edit src/main.rs --old "old_text" --new "new_text"
```
### Correct Pattern — Insert After Line
```bash
echo "new_line_content" | atomwrite --workspace . edit src/main.rs --after-line 10
```


## Transform Operations (AST)
### REQUIRED
- USE `transform` for structural refactoring via ast-grep
- ALWAYS specify `--lang` (`-l`) for the target language
- USE `$NAME` for single AST node captures
- USE `$$$ARGS` for multiple AST node captures (variadic)
- 306 languages supported via ast-grep
- USE `--dry-run` to preview transformations
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


## Batch Operations
### REQUIRED
- USE `batch` for multiple operations in a single call
- Input is NDJSON on stdin (one JSON object per line)
- Each line requires an `op` field: `write`, `replace`, `delete`, `edit`, `hash`, `move`, `copy`
- USE `--transaction` for all-or-nothing execution with automatic rollback on failure
- USE `--dry-run` to preview the entire batch
- Response is NDJSON with one result per operation
### Correct Pattern — Batch
```bash
echo '{"op":"write","target":"a.txt","content":"hello"}
{"op":"replace","path":"b.txt","pattern":"old","replacement":"new"}
{"op":"delete","target":"tmp.log"}' | atomwrite --workspace . batch
```
### Correct Pattern — Batch Dry Run
```bash
cat ops.ndjson | atomwrite --workspace . batch --dry-run
```


## Hash Operations
### REQUIRED
- USE `hash` for standalone BLAKE3 checksums
- Accepts one or more file paths
- Response includes `path` and `checksum` per file
### Correct Pattern — Hash
```bash
atomwrite --workspace . hash src/main.rs
atomwrite --workspace . hash src/*.rs
```


## Delete Operations
### REQUIRED
- USE `delete` for atomic file removal
- USE `--backup --retention N` to keep backups before deletion
- USE `--dry-run` to preview
### Correct Pattern — Delete
```bash
atomwrite --workspace . delete --backup --retention 1 tmp/scratch.rs
```


## Diff Operations
### REQUIRED
- USE `diff` for comparing two files or a file against stdin
- Response includes structured NDJSON diff hunks
### Correct Pattern — Diff
```bash
atomwrite --workspace . diff src/old.rs src/new.rs
```


## Move and Copy Operations
### REQUIRED
- USE `move` for atomic rename/move within the workspace
- USE `copy` for atomic copy with checksum verification
- Both respect the workspace jail
### Correct Pattern — Move
```bash
atomwrite --workspace . move src/old.rs src/new.rs
```
### Correct Pattern — Copy
```bash
atomwrite --workspace . copy src/template.rs src/new_module.rs
```


## List Operations
### REQUIRED
- USE `list` for directory and file listing
- USE `--include '*.rs'` to filter by extension
- USE `--long` for size and modification time
- USE `--depth N` to limit directory depth
- USE `--count-by-ext` for file count grouped by extension
### Correct Pattern — List
```bash
atomwrite --workspace . list --include '*.rs' src/
atomwrite --workspace . list --long --depth 2 src/
```


## Count Operations
### REQUIRED
- USE `count` for file and line counting
- Response includes `files`, `lines`, `bytes`
### Correct Pattern — Count
```bash
atomwrite --workspace . count --include '*.rs' src/
```


## Extract Operations
### REQUIRED
- USE `extract` for NDJSON field extraction from piped input
- Pass field names as positional arguments to select specific JSON fields
### Correct Pattern — Extract
```bash
atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number
```


## Calc Operations
### REQUIRED
- USE `calc` for math expressions and unit conversions
- ALWAYS quote the expression
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
- USE `--digits` for `\d` generalization
- USE `--words` for `\w` generalization
- No `--workspace` needed (stateless)
### Correct Pattern — Regex
```bash
atomwrite regex "192.168.1.1" "10.0.0.255" --digits
atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" --digits
```


## Scope Operations (Grammatical Scoping)
### REQUIRED
- USE `scope` to select AST categories and apply actions on matched code
- ALWAYS specify `--lang` for the target language (rust, python, js, ts, go)
- USE `--query` for prepared queries (comments, fn, class, struct, etc.)
- USE `--pattern` for custom AST patterns
- USE `--delete` to remove matched content
- USE `--action upper|lower|titlecase|squeeze` for text transformations
- USE `--replace-with "text"` for custom replacement
- USE `--dry-run` to preview changes
### Correct Pattern — Scope
```bash
atomwrite --workspace . scope src/ --lang rust --query comments --delete
atomwrite --workspace . scope src/ --lang rust --query fn --action upper --dry-run
atomwrite --workspace . scope src/ --lang python --query def --action lower
```

## Backup Operations
### REQUIRED
- USE `backup` to create timestamped backups with BLAKE3 checksums
- USE `--retention N` to control how many backups to keep (default: 5)
- USE `--dry-run` to preview
### Correct Pattern — Backup
```bash
atomwrite --workspace . backup src/config.toml
atomwrite --workspace . backup src/main.rs src/lib.rs --retention 3
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


## Error Handling
### REQUIRED
- CHECK exit code first before parsing stdout
- PARSE stdout JSON when `error: true` for structured error details
- USE `error_class` to determine retry strategy
- RETRY when `retryable: true`
- USE `suggestion` field for actionable remediation
### FORBIDDEN
- NEVER ignore non-zero exit codes (except exit 1 in search)
- NEVER parse stderr for error data
- NEVER retry when `retryable: false`
### Correct Pattern — Error Handling
```bash
output=$(atomwrite --workspace . read missing.txt 2>/dev/null)
exit_code=$?
if [ $exit_code -ne 0 ]; then
  echo "$output" | jaq '{code: .code, class: .error_class, suggestion: .suggestion}'
fi
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
- `82` — state drift (checksum mismatch, optimistic locking failed)
- `126` — workspace jail violation (path escapes workspace root)
- `127` — symlink blocked (symlink target outside workspace)
- `85` — FIFO detected (named pipe cannot be atomically written)
- `86` — device file detected (block or character device)
- `128` — immutable (file marked immutable)
- `130` — SIGINT (interrupted by user)
- `141` — SIGPIPE (broken pipe)
- `143` — SIGTERM (terminated by signal)
- `255` — internal error (unexpected failure)


## Error JSON Schema
### REQUIRED — Fields
- `error` (bool) — always `true` when an error occurs
- `code` (string) — machine-readable error code
- `exit` (u8) — exit code number
- `message` (string) — human-readable description
- `path` (string, optional) — file path involved in the error
- `error_class` (string) — one of: `permanent`, `transient`, `conflict`, `precondition_failed`
- `retryable` (bool) — whether the operation can be retried
- `suggestion` (string, optional) — actionable remediation step


## Global Flags
### REQUIRED — Reference
- `--workspace <DIR>` — set the workspace jail root (REQUIRED for file operations)
- `--json-schema` — print the output JSON schema for any subcommand
- `--dry-run` — preview operation without writing
- `--backup` — create backup before destructive operation
- `--retention <N>` — number of backups to retain (used with `--backup`)
- `--verbose` / `-v` — increase log verbosity on stderr
- `--quiet` / `-q` — suppress stderr logs
- `--lang <LOCALE>` — override display locale (en, pt-BR) via `ATOMWRITE_LANG` env


## JSON Schema Introspection
### REQUIRED
- USE `--json-schema` flag to get the output schema for any subcommand
- USE schema output for programmatic validation of responses
### Correct Pattern — Schema
```bash
atomwrite write --json-schema
atomwrite search --json-schema
```
