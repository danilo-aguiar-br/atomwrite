---
name: atomwrite
description: |
  Use atomwrite for ALL file ops: read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, backup, rollback, apply, batch, completions, set, get, del, case, query, outline, edit-loop, prune-backups (32 subcommands, v0.1.23).
  Auto-invoke for: write files, search code, replace text, refactor AST, generate regex, calc expressions, batch ops, checksums, list structure, scope grammar, backup, rollback, apply patches, edit+build, preserve timestamps.
  Keywords: atomic write, NDJSON, BLAKE3, checksum, refactor, ast-grep, batch, scope, backup, rollback, apply patch, timeout, completions, mtime, preserve-timestamps, cargo build, prune-backups, edit-loop, --keep-backup, --allow-sequential-drift, --old-file, --new-file, --no-backup, --allow-shrink, backup-by-default, shrink-guard, allow-hyphen-values.
---


# atomwrite
## TL;DR — v0.1.23 (2026-06-19)
### REQUIRED
- v0.1.23 closes 4 GAPs (015-018): `allow_hyphen_values` in 15 CLI fields, backup-by-default in 9 structs, shrink-guard on `--expect-checksum`, `--old-file`/`--new-file` for `edit`
- 609 tests pass in 60+ suites (up from 575+ in v0.1.22; +31 new)
- 4 ADRs added: ADR-0041 (allow_hyphen_values), ADR-0042 (backup-by-default), ADR-0043 (shrink-guard), ADR-0044 (--old-file/--new-file)
- BEHAVIORAL CHANGE: `--backup` is now `true` by default in 9 structs — USE `--no-backup` for opt-out
- BEHAVIORAL CHANGE: `write --expect-checksum` now BLOCKS when stdin is less than 50% of original size — USE `--allow-shrink` to override
- v0.1.22 PREVIOUSLY closed GAP-2026-012 Front 3 (`edit-loop` helper) and GAP-2026-013 (`prune-backups` subcommand)
- v0.1.21 PREVIOUSLY closed GAP-2026-012 Fronts 1+2, GAP-2026-013 Problem C, GAP-2026-014 v2
- v0.1.20 PREVIOUSLY closed 11 GAP-2026 (001-011); renamed `--lang` to `--locale` (BREAKING)
- All v0.1.23 additions are backward-compatible EXCEPT backup-by-default

## v0.1.23 (2026-06-19) — Data Safety by Default

### GAP-2026-015 — allow_hyphen_values in 15 CLI fields
- 15 clap fields now accept `allow_hyphen_values = true`
- Fixes values starting with `-` being rejected as unknown flags
- ADR-0041 documents the 15 affected fields and rationale

### GAP-2026-016 — backup-by-default in 9 structs
- `--backup` is now `true` by default in 9 command structs
- Backup is created BEFORE overwrite and DELETED after success
- USE `--no-backup` to disable backup when performance is priority
- ADR-0042 documents the 9 structs and migration path

### GAP-2026-017 — shrink-guard with --expect-checksum
- `write --expect-checksum` now BLOCKS writes that shrink the file by more than 50%
- Exit 65 with `shrink_blocked: true` in the NDJSON envelope
- USE `--allow-shrink` to permit intentional truncation
- ADR-0043 documents the threshold and rationale

### GAP-2026-018 — --old-file/--new-file for edit
- `edit` accepts `--old-file <PATH>` and `--new-file <PATH>` to read content from files on disk
- Avoids ARG_MAX limits for large match/replacement content
- `--old-file` and `--old` are mutually exclusive via `conflicts_with`
- Cross-mixing (`--old` + `--new-file`) returns exit 65 (`INVALID_INPUT`)
- File content has trailing newline stripped for argv parity
- Response includes `source: "arg"|"file"` in `pair_results`
- ADR-0044 documents the design and traceability


## v0.1.22 (2026-06-17) — prune-backups and edit-loop Sub-commands

- `prune-backups [PATHS]...` — clean up legacy `.bak.YYYYMMDD_HHMMSS` backups by age or count
- `edit-loop [PATH]` — apply N `{old, new}` pairs via NDJSON on stdin in 1 invocation
- 2 ADRs added (0039, 0040)



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
- KNOW that since v0.1.15 append/prepend, line-ending auto-detection, and `--expect-checksum` resolve the target against `--workspace` (G118); on v0.1.14 and earlier ALWAYS keep CWD = workspace as a workaround, or relative targets truncate on append and skip checksum verification
- USE `--prepend` to insert content at beginning of existing file
- USE `--max-size <BYTES>` to limit accepted stdin size
- USE `--line-ending lf|crlf|cr|auto` to normalize line endings (default: auto)
- Response includes `checksum` (BLAKE3) and `bytes_written`
- KNOW that since v0.1.23 `--backup` is `true` by default — backup is created BEFORE overwrite and DELETED after success
- USE `--no-backup` to disable backup when performance is priority
- KNOW that since v0.1.23 `--expect-checksum` BLOCKS writes that shrink the file by more than 50% (exit 65 with `shrink_blocked: true`)
- USE `--allow-shrink` to permit intentional truncation when `--expect-checksum` is active
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
- USE `--max-filesize <BYTES>` to skip files larger than the cap (overrides global `--max-filesize`)
- USE `--max-columns <N>` to truncate output lines wider than N columns (G68)
- USE `--include-fifo` to traverse FIFO/named pipes (G56) — disabled by default for safety
- Response is NDJSON with one object per match
### FORBIDDEN
- NEVER treat exit code 1 as a failure in search
- NEVER use `--include-fifo` on untrusted directories (can hang on slow pipes)
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
### Correct Pattern — Search With Column Truncation
```bash
atomwrite --workspace . search 'error' src/ --max-columns 120
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
- KNOW that since v0.1.15 multi-pair `--old`/`--new` runs the full 9-strategy fuzzy cascade per pair (G117 fixed); success responses include `pairs_total` and `pair_results` (1-based `index`, `matched`, `strategy`, `similarity`)
- KNOW that a failed pair aborts the whole batch by default (all-or-nothing, no write) and the error envelope carries `failed_pair_index`, `pairs_total`, and `pair_results`; pairs after the failure were never attempted and are absent
- USE `--partial` (v0.1.15) to apply the matching pairs and report the rest with `matched: false`; zero applied pairs exits 1 (`NO_MATCHES`) without writing
- NEVER pipe `edit` into `jaq` without verification: the error envelope goes to stdout, so `| jaq '.edits'` masks exit 65 as `{"edits": null}` — use `jaq -e '.edits'` or check `${PIPESTATUS[0]}`
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
- USE `--old-file <PATH>` to read match content from a file on disk (alternative to `--old` for large content)
- USE `--new-file <PATH>` to read replacement content from a file (alternative to `--new` for large content)
- KNOW that `--old-file` and `--old` are mutually exclusive via `conflicts_with` — clap emits exit 2 automatically
- KNOW that cross-mixing (`--old` + `--new-file` or `--old-file` + `--new`) returns exit 65 (`INVALID_INPUT`)
- KNOW that file content has trailing newline stripped via `strip_file_trailing_newline()` for argv parity
- KNOW that the response includes `source: "arg"|"file"` in `pair_results` for traceability
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
### Correct Pattern — Multi-Pair With Per-Pair Verification (v0.1.15)
```bash
# pair_results is the per-item ground truth; jaq -e fails the pipe on error envelopes
atomwrite --workspace . edit src/main.rs --old "foo" --new "bar" --old "baz" --new "qux" \
  | jaq -e '.pair_results'
```
### Correct Pattern — Partial Application (v0.1.15)
```bash
# Apply matching pairs, report the unmatched ones with matched:false
atomwrite --workspace . edit --partial src/main.rs --old "foo" --new "bar" --old "maybe" --new "x" \
  | jaq -e '{edits, pairs_total, missing: [.pair_results[] | select(.matched | not) | .index]}'
```
### Forbidden Pattern — Bare Pipe Masks Edit Failures (G117)
```bash
# FORBIDDEN: exit 65 dies in the pipe and jaq prints {"edits": null} with exit 0
atomwrite --workspace . edit src/main.rs --old "missing" --new "x" | jaq '{edits: .edits}'
# REQUIRED: jaq -e turns the missing field into exit 1, or check ${PIPESTATUS[0]}
atomwrite --workspace . edit src/main.rs --old "missing" --new "x" | jaq -e '.edits'
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
### Correct Pattern — Edit via File (v0.1.23, GAP-018)
```bash
# Read match and replacement content from files on disk
atomwrite --workspace . edit src/main.rs --old-file old.txt --new-file new.txt
# Multiple pairs via file
atomwrite --workspace . edit src/main.rs --old-file a.txt --new-file a2.txt --old-file b.txt --new-file b2.txt
# Large content (200+ KB) without ARG_MAX limit
echo "large content..." | atomwrite --workspace . write /tmp/old.txt
echo "new content..." | atomwrite --workspace . write /tmp/new.txt
atomwrite --workspace . edit target.rs --old-file /tmp/old.txt --new-file /tmp/new.txt
```
### Correct Pattern — Shrink Guard (v0.1.23, GAP-017)
```bash
# Write with expect-checksum BLOCKS shrink > 50% by default
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')
echo "small" | atomwrite --workspace . write --expect-checksum "$CS" src/main.rs
# Exit 65: stdin is 95% smaller than target; pass --allow-shrink to confirm

# Explicit override for intentional truncation
echo "small" | atomwrite --workspace . write --expect-checksum "$CS" --allow-shrink src/main.rs
```
### Correct Pattern — Backup by Default (v0.1.23, GAP-016)
```bash
# v0.1.23: backup is created automatically (and deleted after success)
echo "new" | atomwrite --workspace . write target.txt
# Disable backup when performance is priority
echo "new" | atomwrite --workspace . write --no-backup target.txt
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
- USE `--rules <PATH>` (G44) to load multiple rules from a YAML/JSON file
- USE `--inline-rules <JSON>` (G44) to apply multiple rules from an inline JSON string
- Both `--pattern` and `--rewrite` are REQUIRED for single-rule mode (no search-only mode)
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
- USE `--batch-size <N>` (G77) to control the chunk size for large manifests — useful for memory-constrained streaming
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
- USE `--no-reflink` (G64) to disable reflink (copy-on-write) optimization — forces full byte copy
- USE `--preserve-xattr` (G39) to keep extended attributes on copy/move
- USE `--preserve-hardlinks` (G55) on `move` to keep hardlink count intact
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
- Note: `backup` uses `fs::copy` directly (not the atomic write pipeline), so the backup file inherits the SOURCE mtime, not the moment of backup creation. This is intentional and matches POSIbehavior for file copies
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


## Set Operations (v14 Tier 3 — v0.1.12)
### REQUIRED
- USE `set` to write a single value into a TOML or JSON config file
- ACCEPT `<PATH> <KEY_PATH> <VALUE>` as positional args (auto-detects TOML vs JSON by extension)
- USE dotted path notation for nested keys: `package.version`, `database.pool.max`
- USE `--backup` to create a timestamped backup before modification
- USE `--preserve-timestamps` to keep the original mtime/atime of the file
- VALUE is auto-coerced: `true`/`false` to bool, numeric strings to int/float, everything else stays as string
- Response is NDJSON with `type: "result"`, `path`, `key_path`, `checksum`, `action: "set"`
### FORBIDDEN
- NEVER use `set` on plain text or unsupported formats (TOML and JSON only)
- NEVER use `set` without specifying the full dotted path (no implicit current scope)
### Correct Pattern — Set Top-Level Value
```bash
atomwrite --workspace . set Cargo.toml package.version 0.2.0
```
### Correct Pattern — Set Nested Value With Backup
```bash
atomwrite --workspace . set --backup config.toml database.pool.max 20
```
### Correct Pattern — Set JSON Boolean
```bash
atomwrite --workspace . set package.json scripts.test true
```


## Get Operations (v14 Tier 3 — v0.1.12)
### REQUIRED
- USE `get` to read a single value from a TOML or JSON config file
- ACCEPT `<PATH> <KEY_PATH>` as positional args
- USE dotted path notation for nested keys
- Response is NDJSON with `type: "result"`, `value` (auto-parsed), `key_path`
- Returns `FILE_NOT_FOUND` (exit 4) if the key does not exist
### Correct Pattern — Get Top-Level Value
```bash
atomwrite --workspace . get Cargo.toml package.version
# Returns: {"type":"result","key_path":"package.version","value":"0.1.12",...}
```
### Correct Pattern — Get Nested Value
```bash
atomwrite --workspace . get config.toml database.pool.max
```


## Del Operations (v14 Tier 3 — v0.1.12)
### REQUIRED
- USE `del` to remove a key from a TOML or JSON config file
- ACCEPT `<PATH> <KEY_PATH>` as positional args
- USE dotted path notation for nested keys
- USE `--force-missing` to succeed silently if the key is already absent (idempotent)
- USE `--backup` to create a timestamped backup before deletion
- USE `--preserve-timestamps` to keep the original mtime/atime
- Response is NDJSON with `type: "result"`, `action: "deleted"` or `"already_missing"`
### Correct Pattern — Delete a Key
```bash
atomwrite --workspace . del config.toml dependencies.deprecated
```
### Correct Pattern — Idempotent Delete
```bash
atomwrite --workspace . del --force-missing config.toml features.experimental
# Returns: {"type":"result","action":"already_missing",...} if key was already absent
```


## Case Operations (v14 Tier 3 — v0.1.12)
### REQUIRED
- USE `case` to convert identifier case in source files (refactor naming convention)
- ACCEPT one or more `[PATHS]` as positional args
- USE `--to <STYLE>` to set target: `snake` (default), `camel`, `pascal`, `kebab`, `screaming-snake`
- USE `--subvert OLD NEW` (repeatable) to rename specific identifiers that should not follow the global rule
- USE `--backup` to create timestamped backups before modification
- Response is NDJSON with `type: "result"`, `files_modified`, `identifiers_renamed`
### FORBIDDEN
- NEVER run `case` without `--dry-run` first on a large codebase
- NEVER use `case` on generated files (e.g. `target/`, `dist/`)
### Correct Pattern — Snake Case (Default)
```bash
atomwrite --workspace . case --to snake --dry-run src/
atomwrite --workspace . case --to snake src/
```
### Correct Pattern — Camel Case With Exceptions
```bash
# Convert snake_case to camelCase, but keep SCREAMING_SNAKE constants
atomwrite --workspace . case --to camel --subvert MAX_POOL MAX_POOL src/
```


## Query Operations (v14 Tier 3 — v0.1.12)
### REQUIRED
- USE `query` to inspect AST structure of a single source file via tree-sitter
- ACCEPT `<PATH>` as positional arg
- USE `--kinds` to list all named node kinds in the file (with occurrence counts)
- USE `--tree` to print the full parse tree
- USE `--query <PATTERN>` (short `-Q`) to run an S-expression tree-sitter query
- USE `--positions` to include byte offsets and start positions for every match
- USE `--language <LANG>` to override auto-detection from extension
- Auto-detects language from file extension; supports 24 languages via `tree-sitter-language-pack`
- Response is NDJSON with `type: "kinds" | "tree" | "matches"` depending on mode
### FORBIDDEN
- NEVER use `--query` (S-expression) on files in unsupported languages (returns empty result silently)
- NEVER pipe large files (over `--max-filesize`) through `query` without scoping
### Correct Pattern — List Node Kinds
```bash
atomwrite --workspace . query --kinds src/main.rs
# Returns: {"type":"kinds","kinds":[{"name":"function_item","count":42},...]}
```
### Correct Pattern — Print Full Tree
```bash
atomwrite --workspace . query --tree src/main.rs
```
### Correct Pattern — Query With Positions
```bash
atomwrite --workspace . query -Q '(function_item name: (identifier) @name)' --positions src/main.rs
```


## Outline Operations (v14 Tier 3 — v0.1.12)
### REQUIRED
- USE `outline` to extract high-level structure (functions, classes, structs, enums) from a source file
- ACCEPT `<PATH>` as positional arg
- USE `--kind <KIND>` (repeatable) to filter by node kind: `function_item`, `struct_item`, `enum_item`, `impl_item`, `class_definition`, `function_definition`, etc.
- USE `--positions` to include byte offsets and start/end positions
- USE `--language <LANG>` to override auto-detection from extension
- Response is NDJSON with `type: "result"`, `items: [{kind, name, range, ...}]`
### FORBIDDEN
- NEVER use `outline` on binary files (use `read --stat` instead)
- NEVER chain `outline` to `replace` without reviewing the output first
### Correct Pattern — Full Outline
```bash
atomwrite --workspace . outline src/main.rs
# Returns: {"type":"result","items":[{"kind":"function_item","name":"main","range":[...]},...]}
```
### Correct Pattern — Filter by Kind
```bash
atomwrite --workspace . outline --kind function_item --kind struct_item src/lib.rs
```
### Correct Pattern — Outline With Positions
```bash
atomwrite --workspace . outline --kind function_item --positions src/main.rs | jaq '.items[] | {name, start: .range.start}'
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
### Correct Pattern — v0.1.12 TOML Config Editor With Optimistic Locking
```bash
CS=$(atomwrite --workspace . read --stat config.toml | jaq -r '.checksum')
atomwrite --workspace . set --backup --preserve-timestamps config.toml database.pool.max 20
# Or verify before write:
atomwrite --workspace . get config.toml database.pool.max  # confirm current value
atomwrite --workspace . set config.toml database.pool.max 20
```
### Correct Pattern — v0.1.12 AST Query and Extract Positions
```bash
# List all function definitions in a Rust file with their positions
atomwrite --workspace . query -Q '(function_item name: (identifier) @name)' --positions src/main.rs \\
  | jaq -c '{name: .matches[].captures.name.text, line: .matches[].range.start.line}'
# Count functions per file
for f in src/*.rs; do
  count=$(atomwrite --workspace . query --kinds "$f" | jaq '.kinds[] | select(.name=="function_item") | .count')
  echo "$f: $count functions"
done
```
### Correct Pattern — v0.1.12 Outline With Kind Filter
```bash
# Get all structs and enums in lib.rs
atomwrite --workspace . outline --kind struct_item --kind enum_item src/lib.rs
# Find the longest function in main.rs
atomwrite --workspace . outline --kind function_item --positions src/main.rs \\
  | jaq -c '.items[] | {name, length: (.range.end.byte - .range.start.byte)}' \\
  | sort -t: -k2 -rn | head -1
```
### Correct Pattern — v0.1.12 WAL Recovery Consultive
```bash
# Detect orphan journals before resuming work
ls -la .atomwrite.journal.*.json 2>/dev/null | head
# Use the Rust API for full control:
# let report = atomwrite::wal::recover_orphan_journals(Path::new("src/"))?;
# println!("{}", report.to_json()?);
# Agent decision: replay committed, abort in-progress, or skip stale
```
### Correct Pattern — v0.1.12 Case Rename With Audit
```bash
# Dry-run first, then apply
atomwrite --workspace . case --to kebab --dry-run src/
# Capture the count of files that WOULD change
atomwrite --workspace . case --to kebab --dry-run src/ | jaq -s 'map(select(.type=="result") | .files_modified) | add'
# If acceptable, apply
atomwrite --workspace . case --to kebab --backup src/
```
### Correct Pattern — v0.1.12 Syntax Check Pre-Commit
```bash
# Verify Rust file syntax before commit
atomwrite --workspace . write --syntax-check src/lib.rs < new_lib.rs
# Exit 88 (SyntaxError) if tree-sitter detects invalid syntax
# Use this in pre-commit hooks or CI linting
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


## Windows 10/11 Support (v0.1.12)
### REQUIRED
- VERIFY Visual Studio 2019+ Build Tools with C++ workload is installed before `cargo install atomwrite`
- VERIFY Rust 1.88 or later is installed
- USE Windows Terminal or PowerShell 7+ for proper UTF-8 output and ANSI escape sequences
- TRUST `init_console` to set code page 65001 and `ENABLE_VIRTUAL_TERMINAL_PROCESSING` automatically
- BE AWARE that `tree-sitter-language-pack` 1.8 with `download` feature requires network access on first build — the postinstall script downloads parsers from GitHub
- EXPECT that the first `cargo install atomwrite` on Windows may take 5-10 minutes due to parser downloads
- TRUST that the 5 new error codes (83, 88, 91, 92, 93) work on Windows — they are tested in cross-compile gates
### FORBIDDEN
- NEVER use `cmd.exe` legacy console for output (mojibake expected)
- NEVER rely on `cargo install atomwrite` working on v0.1.3 (broken on Windows 10/11; fix is in v0.1.4)
- NEVER use `query` on Windows without first ensuring parsers were downloaded (use `--language` to override if auto-detect fails)
### Correct Pattern — Windows Install (v0.1.12)
```powershell
rustup default stable
rustup target add x86_64-pc-windows-msvc
cargo install atomwrite --locked --version '^0.1.12'
atomwrite --version  # NDJSON output
# First run may take a few seconds to initialize tree-sitter parsers
```


## Cross-Compile Validation (v0.1.12)
### REQUIRED
- RUN `cargo test --test cross_compile_check -- --ignored` before any release that touches `#[cfg(windows)]` code
- INSTALL Windows targets: `rustup target add x86_64-pc-windows-gnu` and `i686-pc-windows-gnu`
- ON Linux, INSTALL mingw-w64: `mingw64-gcc` (Fedora) or `mingw-w64` (Ubuntu) and `mingw32-gcc` for 32-bit
- TRUST the gate to fail on any `E0433`, `E0308`, or `E0507` regression in Windows-only code
- VERIFY that the 10 new v0.1.12 test files compile under all 3 cross-compile targets — `cli_set`, `cli_case`, `cli_query`, `cli_outline`, `cli_get_del`, `cli_v012_syntax_check`, `cli_v012_wal`, `cli_v012_audit_regressions`, `cli_v012_xattr_reflink`, `cli_v012_batch4_regressions`
- BE AWARE that `tree-sitter-language-pack` is downloaded at build time, so offline cross-compile requires pre-downloading parsers
### Correct Pattern — Cross-Compile Gate (v0.1.12)
```bash
rustup target add x86_64-pc-windows-gnu i686-pc-windows-gnu x86_64-pc-windows-msvc
cargo test --test cross_compile_check -- --ignored
# Verify that the 10 v0.1.12 test files build on all 3 Windows targets
cargo check --target x86_64-pc-windows-gnu --tests
cargo check --target i686-pc-windows-gnu --tests
cargo check --target x86_64-pc-windows-msvc --tests
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
- `83` — lock timeout (v0.1.12+)
- `85` — FIFO detected (named pipe cannot be atomically written)
- `86` — device file detected (block or character device)
- `88` — syntax error detected (v0.1.12+, G72 tree-sitter check failed)
- `91` — EXDEV fallback disabled (v0.1.12+, --strict-atomic forbids copy-fallback)
- `92` — copy-back BLAKE3 verification failed (v0.1.12+)
- `93` — orphan journal detected (v0.1.12+, G114 consultive recovery)
- `126` — workspace jail violation (path escapes workspace root)
- `127` — symlink blocked (symlink target outside workspace)
- `128` — immutable (file marked immutable)
- `130` — SIGINT (interrupted by user)
- `141` — SIGPIPE (broken pipe)
- `143` — SIGTERM (terminated by signal)
- `255` — internal error (unexpected failure)


### v0.1.19 Drift Notes — Phase D Exit Code Consolidation
- DRIFT 1 — `STATE_DRIFT` (82) absorbs `CHECKSUM_VERIFY_FAILED` (81) for `--verify-checksum` on reads and writes. Both are conflict class, retryable. The 81-code is now historical, preserved only for the `read` path BLAKE3 mismatch on file content. The 82-code covers optimistic-locking failure including `--expect-checksum` mismatch on writes and edits, and `--verify-checksum` mismatch on reads.
- DRIFT 2 — `--syntax-check` returns `SYNTAX_ERROR_DETECTED`, NOT `SYNTAX_ERROR`. The rename happened in the v0.1.12 G72 tree-sitter rollout but the docs were not updated. The historical name `SYNTAX_ERROR` is preserved only in prose for grep-ability.
- DRIFT 3 — `ORPHAN_JOURNAL` (93) is consultive, NOT auto-detected. The gate is `ATOMWRITE_WAL=1` OR `--strict-atomic`. The default `write` (v0.1.16 G119 `WalPolicy::Auto`) does not write a sidecar and therefore cannot detect orphans. Default invocations never see this code.
- DRIFT 4 — `BROKEN_PIPE` (141) requires real SIGPIPE propagation. A simple `head -1` pipe does NOT trigger it. The v0.1.4+ SIGPIPE restoration puts the default disposition back, so the signal is only raised when the downstream consumer actively closes the pipe mid-stream.
- DRIFT 5 — Binary file reads return exit 0 with `kind=binary` metadata, NOT exit 65. The v0.1.4 `BINARY_FILE` heuristic was changed to emit a structured envelope and exit 0. The 65-code path now only fires for `read` without `--format raw` AND with the binary heuristic bypassed.
- DRIFT 6 — Missing positional argument returns `ARGUMENT_PARSE_ERROR` (exit 2), NOT `INVALID_INPUT` (65). Clap-level argument errors are reported as exit 2. The 65-code is reserved for runtime content validation (malformed TOML, invalid regex, empty stdin default).
- DRIFT 7 — Missing `--workspace` defaults to CWD, NOT an error. `--workspace` is a flag with a CWD default, not a required argument. `WORKSPACE_JAIL` (126) only fires when an absolute path resolves outside the effective jail.
- See `docs/decisions/0033-v0-1-19-exit-code-naming-drift-consolidation.md` for the full rationale and the consequences of accepting the binary behavior as canonical.
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
### REQUIRED — Full Error Code List (25 codes as of v0.1.12)
- `WORKSPACE_JAIL` (exit 126, precondition_failed, not retryable)
- `SYMLINK_BLOCKED` (exit 127, precondition_failed, not retryable)
- `FILE_NOT_FOUND` (exit 4, permanent, not retryable)
- `PERMISSION_DENIED` (exit 13, transient, retryable via `persist_with_retry` on Windows)
- `CHECKSUM_VERIFY_FAILED` (exit 81, conflict, not retryable)
- `STATE_DRIFT` (exit 82, conflict, not retryable)
- `LOCK_TIMEOUT` (exit 83, transient, retryable with backoff — v0.1.12+, G54 lock file contention)
- `FIFO_DETECTED` (exit 85, precondition_failed, not retryable)
- `DEVICE_FILE` (exit 86, precondition_failed, not retryable)
- `SYNTAX_ERROR` (exit 88, permanent, not retryable — v0.1.12+, G72 tree-sitter validation failed)
- `EXDEV_FALLBACK_DISABLED` (exit 91, precondition_failed, not retryable — v0.1.12+, G90 strict atomic mode forbids cross-device copy-fallback)
- `COPY_BACK_BLAKE3_FAILED` (exit 92, conflict, retryable after re-read — v0.1.12+, G114 cross-device copy-back checksum verification failed)
- `ORPHAN_JOURNAL` (exit 93, precondition_failed, not retryable — v0.1.12+, G114 orphan WAL sidecar detected; call `recover_orphan_journals` consultively)
- `DISK_FULL` (exit 28, transient, retryable)
- `QUOTA_EXCEEDED` (exit 30, transient, retryable)
- `CROSS_DEVICE` (exit 73, permanent, not retryable)
- `IO_ERROR` (exit 74, transient, retryable)
- `CONFIG_INVALID` (exit 78, permanent, not retryable)
- `FILE_IMMUTABLE` (exit 128, precondition_failed, not retryable)
- `BINARY_FILE` (exit 65, permanent, not retryable — use `read --format raw` to bypass JSON envelope)
- `FILE_TOO_LARGE` (exit 65, permanent, not retryable — file exceeds `--max-filesize` limit)
- `NO_MATCHES` (exit 1, permanent, not retryable — by design, not an error)
- `INVALID_INPUT` (exit 65, permanent, not retryable)
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


## Versioned Schemas (v0.1.12)
### REQUIRED
- KNOW that stable JSON Schemas are committed under `docs/schemas/`
- KNOW that `error-output.schema.json` is the contract for all error envelopes
- KNOW that the schema field `workspace` (string, optional) was added in v0.1.4
- USE the versioned schema to validate responses in your agent pipeline
- NOT invent your own parsing rules; trust the versioned schema as source of truth
### Required — Schema Index (29 schemas as of v0.1.12)
- `error-output.schema.json` — envelope for all `error: true` responses (v0.1.4)
- `write-output.schema.json` — `write` command response
- `read-output.schema.json` — `read` command response with metadata
- `search-output.schema.json` — `search` command NDJSON matches
- `replace-output.schema.json` — `replace` command batch response
- `edit-output.schema.json` — `edit` command response with `mtime_preserved`
- `transform-output.schema.json` — `transform` AST refactor response
- `scope-output.schema.json` — `scope` grammatical scoping response
- `batch-output.schema.json` — `batch` transactional result
- `hash-output.schema.json` — `hash` BLAKE3 checksum response
- `delete-output.schema.json` — `delete` removal confirmation
- `diff-output.schema.json` — `diff` structured diff hunks
- `move-output.schema.json` — `move` rename confirmation
- `copy-output.schema.json` — `copy` verification response
- `list-output.schema.json` — `list` directory listing
- `count-output.schema.json` — `count` file and line count
- `extract-output.schema.json` — `extract` field extraction
- `calc-output.schema.json` — `calc` math and unit conversion
- `regex-output.schema.json` — `regex` generated pattern
- `backup-output.schema.json` — `backup` with timestamp
- `rollback-output.schema.json` — `rollback` restoration
- `apply-output.schema.json` — `apply` patch application
- `set-result.schema.json` — `set` v14 Tier 3 (v0.1.12, NEW)
- `get-result.schema.json` — `get` v14 Tier 3 (v0.1.12, NEW)
- `del-result.schema.json` — `del` v14 Tier 3 (v0.1.12, NEW)
- `case-result.schema.json` — `case` v14 Tier 3 (v0.1.12, NEW)
- `query-output.schema.json` — `query` v14 Tier 3 (oneOf 3: kinds/tree/matches, v0.1.12, NEW)
- `outline-output.schema.json` — `outline` v14 Tier 3 (oneOf 2: items/empty, v0.1.12, NEW)
- `wal-recovery.schema.json` — WAL recovery report (v0.1.12, NEW)
### Required — Programmatic Validation Example
```bash
# Validate NDJSON response against its schema using ajv-cli
echo '{"type":"result","checksum":"abc...","bytes_written":42}' | \\
  ajv validate -s docs/schemas/write-output.schema.json -d /dev/stdin
# Or with Python jsonschema:
python3 -c "import json, jsonschema; \\
  s = json.load(open('docs/schemas/write-output.schema.json')); \\
  d = json.loads('{\"type\":\"result\",\"checksum\":\"abc\",\"bytes_written\":42}'); \\
  jsonschema.validate(d, s); print('OK')"
```


## Tests and Quality Gates (v0.1.12)
### REQUIRED — Quality Posture
- 609 tests in 60+ test suites pass with zero regressions as of v0.1.23
- Test count decomposition: 320 baseline (v0.1.10) + +29 (v0.1.11) + +96 (v0.1.12) + +2 (v0.1.14) + +14 (v0.1.15: 8 G117 + 6 G118) = 461 (v0.1.15) + +41 (v0.1.18: G118 + G119 + G120 + 2 ADRs) = 502 (v0.1.18) + +13 (v0.1.19: ADR-0031 G121 path resolution + ADR-0032 query S-expr + ADR-0033 exit code consolidation) = 515 (v0.1.19) + +27 (v0.1.20: 11 GAP-2026 + 4 ADRs) = 542 total
- v0.1.21 to v0.1.23 decomposition: +13 (v0.1.21: drift + backup parity) + +16 (v0.1.22: edit-loop + prune-backups) + +31 (v0.1.23: 12 hyphen + 7 backup + 4 shrink + 8 old-file) = 609 total
- New v0.1.23 test files (4): `cli_v0123_hyphen_values`, `cli_v0123_backup_default`, `cli_v0123_shrink_guard`, `cli_v0123_old_file`
- v0.1.12 new test files (10): `cli_set`, `cli_case`, `cli_query`, `cli_outline`, `cli_get_del`, `cli_v012_syntax_check`, `cli_v012_wal`, `cli_v012_audit_regressions` (27 tests), `cli_v012_xattr_reflink`, `cli_v012_batch4_regressions` (23 tests)
- v0.1.12 test coverage by category: G72 syntax check (16 tests), G114 WAL (8 tests), v14 query/outline (10 tests), TOML dotted path (6 tests), set/get/del/case (15 tests), audit regressions (50 tests)
- 8 official gates pass on every commit: `fmt`, `clippy`, `build`, `test`, `doc`, `deny`, `audit`, `msrv`
- 3 cross-compile targets pass: `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, `x86_64-pc-windows-msvc`
- Cargo deny and cargo audit both report zero vulnerabilities (time 0.3.47+ resolved RUSTSEC-2026-0009 via DEPTH_LIMIT=32)
- MSRV is Rust 1.88 stable
- Coverage by `cargo tarpaulin`: 20.19% line coverage (935/4631 lines) — coverage is integration-test heavy
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

## v0.1.12 Migration Quick Reference
### REQUIRED — Know What Changed Since v0.1.11
- **6 new subcommands ADDITIVE**: `set`, `get`, `del`, `case`, `query`, `outline` (v14 Tier 3 structured config editors + tree-sitter AST tools). No existing subcommand was renamed or removed
- **5 new error variants ADDITIVE**: `LockTimeout` (83), `SyntaxError` (88), `ExdevFallbackDisabled` (91), `CopyBackBlake3Failed` (92), `OrphanJournal` (93). All bilingual with actionable suggestions
- **`atomwrite write --syntax-check` is OPT-IN**: default `write` behavior is unchanged. G72 REAL tree-sitter syntax check (24 languages)
- **WAL sidecar is consultive only**: `atomic_write` writes `.atomwrite.journal.<target>.atomwrite.journal.json` only when `ATOMWRITE_WAL=1` is set OR `--strict-atomic` is passed. Default `write` does NOT write the sidecar. `recover_orphan_journals(dir)` is consultive
- **542 tests pass in 47 test suites** (was 320 in v0.1.10). Coverage is full across all 30 subcommands
- **7 ADRs added** in `docs/decisions/` (0019-0025): tree-sitter-language-pack, WAL sidecar, query/outline kind-name only, G72 replaces heuristic, G114 consultive, get_toml_path manual, positions opt-in
- **7 new JSON Schemas** in `docs/schemas/` (set-result, get-result, del-result, case-result, query-output, outline-output, wal-recovery)
- **New dependency**: `tree-sitter-language-pack = "1.8"` with `download` + `dynamic-loading` features. Install footprint stays around 5-10 MB
- **DO NOT upgrade from v0.1.11 to v0.1.12 if you parse stderr** looking for the new exit codes 83, 88, 91, 92, 93 — they are emitted in JSON `error: true` envelopes on stdout, not on stderr


## WAL Subcommands (v0.1.18)
### REQUIRED — wal-stats
- USE `wal-stats` to inspect WAL journal state for telemetry and debugging
- KNOW that `wal-stats` is consultive: it scans the workspace and reports a snapshot of journal files without modifying them
- ALWAYS combine `wal-stats` with `--workspace <DIR>` to scope the scan to a specific project
- USE `--dry-run` to preview what the scan would find without doing the walk
- Response is NDJSON with `type: "result"`, terminal state counts, total size, age breakdown, and per-directory breakdown
- JSON response: `{action: "scanned", terminal_committed, terminal_aborted, terminal_started, total_bytes, oldest_age_secs, breakdown_by_dir}`
- USE this for ops debugging when suspecting stale journals or unexpected sidecar growth

### REQUIRED — wal-heal
- USE `wal-heal` to remove stale terminal journals older than a threshold
- DEFAULT threshold is 3600 seconds (1 hour) via `--threshold-secs <N>`
- DEFAULT wall-clock budget is 100ms via `--max-duration-ms <N>`
- USE `--threshold-secs` and `--max-duration-ms` to tune for your environment
- USE this when the workspace accumulates terminal journals from crashed or interrupted processes
- Auto-pass equivalent runs at startup with 3600s threshold and 100ms budget; skip via `--no-auto-heal` global flag or `ATOMWRITE_WAL_NO_AUTO_HEAL=1` env var

### Correct Pattern — Inspect WAL State
```bash
# Snapshot the current WAL state of the project
atomwrite --workspace . wal-stats
# Output: {"type":"result","action":"scanned","terminal_committed":42,...}
```

### Correct Pattern — Heal Stale Journals
```bash
# Remove terminal journals older than 1 hour
atomwrite --workspace . wal-heal --threshold-secs 3600
# Custom threshold and budget
atomwrite --workspace . wal-heal --threshold-secs 7200 --max-duration-ms 500
```


## v0.1.21 — Sequential Edit Pattern with Checksum Re-capture
### REQUIRED — Gap-2026-012 Pattern
- KNOW that chaining multiple `edit` calls on the same file without re-capturing `checksum_after` produces `STATE_DRIFT` (exit 82) on every call after the first
- TWO valid patterns for sequential pipelines — choose one per pipeline
- PATTERN A — re-capture `checksum_after` after each `edit` and pass to the next call. Reduces drift risk to zero but doubles the CLI invocations (one `read` per `edit`)
- PATTERN B — pass `--allow-sequential-drift` to every `edit` call. Same number of CLI invocations as the naive approach; the flag suppresses `STATE_DRIFT` and emits `tracing::warn!` naming the drift
- DO NOT use `--allow-sequential-drift` in a TRUE parallel scenario. The flag exists for the single-agent sequential case; concurrent agents should use Pattern A with fresh reads

### Pattern A — Re-capture Checksum After Each Edit
```bash
# Initial seed of the target
echo "line 1" > /tmp/seq.txt
# Read the initial checksum
CS=$(atomwrite --workspace /tmp read seq.txt | jaq -r '.checksum')
# Edit 1 — pass the captured checksum
echo "line 2" | atomwrite --workspace /tmp edit --expect-checksum "$CS" seq.txt --append
# Re-capture the post-edit checksum
CS=$(atomwrite --workspace /tmp read seq.txt | jaq -r '.checksum')
# Edit 2 — uses the new checksum
printf 'line 1\nline 2\n' | atomwrite --workspace /tmp edit --expect-checksum "$CS" seq.txt --append
```

### Pattern B — Allow Sequential Drift
```bash
# Initial seed of the target
echo "line 1" > /tmp/seq.txt
# Read the initial checksum
CS=$(atomwrite --workspace /tmp read seq.txt | jaq -r '.checksum')
# Edit 1 — pass the captured checksum (drift would fire here without the flag on edit 2+)
echo "line 2" | atomwrite --workspace /tmp edit --expect-checksum "$CS" seq.txt --append
# Edit 2 — drift is allowed, the pre-state differs from CS but the flag suppresses STATE_DRIFT
printf 'line 1\nline 2\n' | atomwrite --workspace /tmp edit --allow-sequential-drift seq.txt --append
```

### FORBIDDEN
- NEVER use `--allow-sequential-drift` to bypass drift caused by a CONCURRENT writer — that is a true race, not a sequential pipeline, and the warning exists to surface it
- NEVER pass `--allow-sequential-drift` to a pipeline that runs multiple `edit` invocations in parallel against the same file

## v0.1.21 — Backup Deletion After Success
### REQUIRED — Gap-2026-014 v2 Behavior
- KNOW that `write --backup`, `replace --backup`, and `edit --backup` DELETE the backup file by default after the write succeeds
- USE `--keep-backup` on `write`, `edit`, `replace`, `rollback`, `apply`, or `batch` to preserve the backup after success
- KNOW that backups are ALWAYS preserved when the write FAILS. The `--keep-backup` flag affects only the success path
- USE `--keep-backup` in CI scripts that need the backup as forensic evidence after the write completes
- DO NOT assume backups persist after a successful `--backup` operation. The pre-v0.1.21 behavior of backup-lives-forever is removed

### Pattern — Preserve Backup for Audit
```bash
# Backup survives after success when --keep-backup is passed
echo "new" | atomwrite --workspace /tmp write --backup --keep-backup config.toml
# Default behavior: backup is deleted after success
echo "new" | atomwrite --workspace /tmp write --backup config.toml
# Verify the post-success state
test -f /tmp/config.toml.bak.* && echo "backup present" || echo "backup deleted"
```

## v0.1.20 Notes
### REQUIRED — Global --lang to --locale Rename
- GLOBAL flag `--lang` is RENAMED to `--locale` (breaking change in v0.1.20)
- Env var `ATOMWRITE_LANG` is UNCHANGED
- Rust field name `lang` on the Cli struct is UNCHANGED
- See ADR-0037 for the full rename rationale and migration notes
- UPDATE existing invocations from `--lang pt-BR` to `--locale pt-BR`
- DO NOT confuse with `transform --lang` or `scope --lang` (subcommand language flags stay)

### REQUIRED — Write Intention Guard Flags (v0.1.20)
- KNOW that v0.1.20 introduces four new write-safety flags
- USE `--require-backup` to ABORT if `--backup` is not set AND the target exists
- USE `--confirm` to trigger an interactive Y/N prompt for files larger than 100KB
- USE `--auto-rotate` to FORCE a backup when the target was modified within the last 24 hours
- USE `--risk-threshold <PERCENT>` to emit `risk_assessment` telemetry when size delta exceeds the threshold (default: 50)
- Default behavior of `write` is UNCHANGED — these flags are additive
- USE `--require-backup` in CI pipelines to prevent destructive overwrites without backup

### Correct Pattern — Require Backup Before Overwrite
```bash
# v0.1.20+: abort if --backup is missing and target exists
atomwrite --workspace . write --require-backup src/main.rs < new_main.rs
# Exit 1 (Validation) if --backup was not also set
```

### Correct Pattern — Confirm Large File Writes
```bash
# v0.1.20+: interactive prompt for files > 100KB
atomwrite --workspace . write --confirm big_dataset.csv < new_data.csv
# Prompts y/N before applying
```

### Correct Pattern — Auto-Rotate Recent Targets
```bash
# v0.1.20+: force backup when target modified within 24h
atomwrite --workspace . write --auto-rotate src/frequently_changed.rs < new.rs
# Backup auto-created if mtime within 24h
```

### Correct Pattern — Risk Threshold Telemetry
```bash
# v0.1.20+: emit risk_assessment when size delta > 50%
atomwrite --workspace . write --risk-threshold 30 src/data.json < new.json
# NDJSON response includes risk_assessment block
```

### REQUIRED — Count --by-size Mode (v0.1.20)
- KNOW that `--by-size` produces a structured NDJSON output (v0.1.20+)
- Response includes `mode: "by_size"`, `items[path, bytes]` array, sorted DESCENDING by size
- USE `--top N` to truncate the items list (default: 10)
- Replaces the old text-table output with a parseable contract
- CONSUME via `jaq '.items[] | {path, bytes}'` for downstream pipelines

### Correct Pattern — Largest Files
```bash
# v0.1.20+: top 10 largest files with structured output
atomwrite --workspace . count --by-size --top 10
# Output: {"type":"result","mode":"by_size","items":[{"path":"...","bytes":N},...]}
```

### REQUIRED — Read --mode Discriminator (v0.1.20)
- KNOW that the `mode` field in `read` output is now POPULATED
- `mode` value is one of: `full`, `head`, `tail`, `line`, `lines`, `grep`, `stat`
- USE this to disambiguate which read variant produced the response
- Previously the field was always absent or null

### Correct Pattern — Inspect Read Mode
```bash
# v0.1.20+: read reports which mode was used
atomwrite --workspace . read --head 20 src/main.rs | jaq '.mode'
# Output: "head"

atomwrite --workspace . read --grep 'TODO' src/main.rs | jaq '.mode'
# Output: "grep"

atomwrite --workspace . read --stat src/main.rs | jaq '.mode'
# Output: "stat"
```

### REQUIRED — Search --no-begin-end (v0.1.20)
- USE `--no-begin-end` to suppress per-file `begin`/`end` events when files have zero matches
- Useful for streaming pipelines that only care about match content
- Default behavior UNCHANGED — `begin`/`end` events still emitted unless suppressed
- Combine with `--count` for compact per-file match counts

### Correct Pattern — Suppress Empty File Markers
```bash
# v0.1.20+: silence begin/end events for files without matches
atomwrite --workspace . search --no-begin-end 'TODO' src/ --include '*.rs'
# Output: only files with matches emit begin/end; zero-match files are silent
```

### REQUIRED — Write --preserve-timestamps (v0.1.20)
- USE `--preserve-timestamps` on `write` to keep the original mtime of the target
- Default behavior UNCHANGED — mtime is updated to reflect the write by default
- Useful for backup, snapshot, and reproducible build workflows
- Mirror of the existing `--preserve-timestamps` flag on `edit` and `replace`

### Correct Pattern — Preserve mtime on Write
```bash
# v0.1.20+: keep original mtime on write
atomwrite --workspace . write --preserve-timestamps src/snapshot.rs < new.rs
# Target mtime unchanged after the atomic write
```

### REQUIRED — Scope --lang Alias (v0.1.20)
- After the global `--lang` to `--locale` rename, `scope --lang <LANG>` is now a working alias for `--language`
- USE `scope --lang rust` as shorthand for `scope --language rust`
- Both forms are accepted — `--lang` on `scope` is the subcommand-local language selector
- This avoids collision with the global locale flag that was renamed to `--locale`

## v0.1.20 GAP-2026 — Complete Coverage
### REQUIRED — The 11 Gaps Closed in v0.1.20
- GAP-2026-001: `count --by-size` finally implements the help flag (regression test `count_by_size_top_n_returns_sorted`)
- GAP-2026-002: `write --preserve-timestamps` (parity with edit/replace)
- GAP-2026-003: `scope --lang` alias after global `--lang` → `--locale` rename (ADR-0037)
- GAP-2026-004: `write --line-ending crlf` accepts both `crlf` and `cr-lf` (4 variants with `value` + `alias`)
- GAP-2026-005b: `edit --partial` semantics (single-pair returns NO_MATCHES exit 1; multi-pair applies matched and reports unmatched) — ADR-0036
- GAP-2026-006: `diff --algorithm` regression tests for myers/patience/lcs
- GAP-2026-007: `count --by-extension` filters backup timestamp suffixes via `BACKUP_RE` `\.bak\.\d{8}_\d{6}$`
- GAP-2026-008: `read --head/--line/--lines` reports FILTERED line count (new `lines_total` field preserves original)
- GAP-2026-009: `read` emits `mode` discriminator (`full|head|tail|line|lines|grep|stat`)
- GAP-2026-010: `search --no-begin-end` for cleaner output of walks with zero matches
- GAP-2026-011: `write` intention guards (defense-in-depth after c24-framework34.html incident 2026-06-15) — 6 layers L1-L6 (telemetry, --require-backup, --confirm, --preview, --auto-rotate, risk_assessment in envelope) — ADR-0035

### REQUIRED — Write Intention Guards Origin (c24-framework34.html)
- On 2026-06-15, `atomwrite write` without `--append` truncated c24-framework34.html (491,827 bytes) to a few bytes
- Approximately 127 lines (~9 KB) of 2026-06-15 work were lost
- A 2026-06-14 23:49 manual `cp` backup existed but did not cover the 2026-06-15 work
- Without `--backup`, without confirmation prompt, without size-delta telemetry, the path between operator intent and on-disk content was a direct syscall
- v0.1.20 adds 6 defense-in-depth layers (L1-L6) to prevent recurrence — see ADR-0035
- L1 telemetry (size_delta_pct) is INFORMATIVE (default off, opt-in via `--risk-threshold`)
- L2 `--require-backup` ABORTS with `InvalidInput` (exit 65) if `--backup` is not also set
- L3 `--confirm` prompts `Overwrite <path> (<bytes> bytes)? [y/N]` for targets > 100 KB
- L4 `--preview` emits structural diff before atomic write
- L5 `--auto-rotate` forces backup when target modified within 24h
- L6 `risk_assessment` field in envelope (only when a guard fires)

### REQUIRED — v0.1.19 (predecessor release) — 3 ADRs Added
- ADR-0031 — G121 path resolution helper: `search` and `replace` resolve root paths against workspace via shared helper (CWE-367)
- ADR-0032 — query S-expr real implementation: `query` accepts S-expressions tree-sitter via `Query::new` (v0.1.12 docs promised but code never implemented)
- ADR-0033 — v0.1.19 exit code drift consolidation: 7 drifts of exit code between published docs and binary (STATE_DRIFT, SYNTAX_ERROR_DETECTED, ORPHAN_JOURNAL, BROKEN_PIPE, binary read, ARGUMENT_PARSE_ERROR, missing --workspace)

### REQUIRED — Help-Driven Testing Anti-Pattern (ADR-0034)
- 5 of the 11 GAP-2026 (001, 003, 004, 005b, 006) had clap `--help` declaring flag before implementation existed
- v0.1.21+ rule: `cargo test --doc` must parse each help block and validate each flag is wired to a regression test whose name includes the flag


## WAL Recovery Flow (v0.1.12)
### REQUIRED
- KNOW that `atomic_write` only writes a WAL sidecar when `ATOMWRITE_WAL=1` env var is set OR `--strict-atomic` CLI flag is passed
- KNOW that sidecar path is `.atomwrite.journal.<target_basename>.atomwrite.journal.json`
- KNOW that `recover_orphan_journals(dir)` is CONSULTIVE — it does NOT auto-replay or auto-delete
- KNOW that each sidecar contains `JournalEntry::{Started, Committed, Aborted}` with `op_id` and `pid`
### Required — Recovery Decision Tree
1. **Detect orphans**: scan directory for `*.atomwrite.journal.json` files
2. **Read entries**: parse each sidecar to determine which operations were `Started` but not `Committed`/`Aborted`
3. **Decide per entry**:
   - `Committed` → safe to delete sidecar (operation completed successfully)
   - `Aborted` → safe to delete sidecar (operation was rolled back)
   - `Started` without `Committed`/`Aborted` → AMBIGUOUS: consult the user or check inode of the target file
4. **Atomic action**: apply decision via `recover_orphan_journals` Rust API
### Required — Rust API Pattern
```rust
use atomwrite::wal::{recover_orphan_journals, OrphanJournalReport};
use std::path::Path;

let report: OrphanJournalReport = recover_orphan_journals(Path::new("src/"))?;
// Inspect report.entries: Vec<JournalEntry>
// Apply your decision logic per entry
// Use atomwrite delete with --force to clean up reconciled sidecars
```
### FORBIDDEN
- NEVER auto-delete sidecars without user confirmation
- NEVER replay WAL entries without verifying the target file's current state
- NEVER treat WAL as the only source of truth for atomicity (the rename syscall is the real atomic primitive; WAL is for crash forensics only)


## v0.1.12 Gaps Closed
### REQUIRED — Know What the 20 Gaps Were
- The v0.1.12 release closes 20 named technical gaps from `gaps.md`. Each gap has an ADR in `docs/decisions/0019-0025` and a test in `tests/`.
- **G72 — Tree-sitter REAL syntax check**: `atomwrite write --syntax-check` validates content against 24 languages via `tree_sitter_language_pack`. Replaces heuristic bracket-balance check. Returns `SyntaxError` (88) on failure
- **G90 — EXDEV copy-fallback controlled**: `--strict-atomic` mode forbids copy-fallback on cross-device moves. Returns `ExdevFallbackDisabled` (91) when triggered
- **G114 — WAL sidecar consultive**: `ATOMWRITE_WAL=1` or `--strict-atomic` writes `.atomwrite.journal.<target>.json`. `recover_orphan_journals` is the consultive recovery API
- **G114 — Copy-back BLAKE3 verification**: cross-device copy-back verifies the destination checksum before deleting the source. Returns `CopyBackBlake3Failed` (92) on mismatch
- **G54 — Lock file with timeout**: every write acquires a file lock with 30s timeout. Returns `LockTimeout` (83) on contention
- **G44 — Transform multirule**: `transform --rules <PATH>` and `--inline-rules <JSON>` accept multiple rules
- **G66 — Literal search/replace**: `--literal` (`-F`) treats pattern as literal string, no regex escaping
- **G64 — Reflink detection**: `--no-reflink` on `copy`/`move` disables reflink (copy-on-write) optimization
- **G68 — max-filesize and max-columns**: `--max-filesize <BYTES>` global cap; `--max-columns <N>` caps `search` output width
- **G56 — FIFO inclusion**: `--include-fifo` in `search` traverses FIFO/named pipes
- **G39 — xattr preservation**: `--preserve-xattr` on `copy`/`move` keeps extended attributes
- **G41 — Binary handling**: `read --format raw` outputs raw bytes without JSON envelope, avoids `BinaryFile` (65) for known-binary content
- **G58 — Line ending normalization**: `--line-ending lf|crlf|cr|auto` in `write` and `edit`
- **G76 — Diff algorithm choice**: `diff --algorithm myers|patience|lcs` selects algorithm
- **G74 — Parallel threads**: `--threads <N>` / `-j <N>` global flag controls Rayon pool
- **G80 — SIGPIPE restoration**: SIGPIPE is restored to default disposition on Unix so pipes to `head`/`wc`/`jaq` exit cleanly
- **G55 — Hardlink preservation**: `--preserve-hardlinks` on `move` keeps hardlink count
- **G77 — Batch stream size**: `--batch-size <N>` controls `batch` chunk size for large manifests
- **G81 — Raw read format**: `read --format raw` outputs raw content, skips JSON parsing
- **v14 Tier 3 — 6 new subcommands**: `set`, `get`, `del`, `case`, `query`, `outline` (this release)


## Tree-sitter-language-pack Notes (v0.1.12)
### REQUIRED
- KNOW that `tree-sitter-language-pack = "1.8"` is the only new runtime dependency
- KNOW that the `download` feature pulls parsers from GitHub on first use
- KNOW that the `dynamic-loading` feature loads parsers as shared libraries (.so/.dll/.dylib) at runtime
- KNOW that 24 languages have built-in parser coverage: bash, c, cpp, css, elixir, go, html, java, javascript, json, kotlin, lua, markdown, ocaml, php, python, ql, ruby, rust, scala, sql, swift, toml, typescript, yaml
- KNOW that 305+ additional languages are available via dynamic-loading
- KNOW that on Windows, the download step requires network access during the first `cargo install` or `cargo build`
- KNOW that on Linux, parsers are cached in `~/.cache/tree-sitter-language-pack/` (or `$XDG_CACHE_HOME`)
- KNOW that on macOS, the dynamic loader looks in `/usr/local/lib/` and `DYLD_LIBRARY_PATH`
### FORBIDDEN
- NEVER rely on tree-sitter parsers being available offline unless you have pre-downloaded them
- NEVER call `query` on a file with an extension not mapped to a language (it will return an error)


## v0.1.5-v0.1.14 Changelog Summary
### REQUIRED — What Changed In Intermediate Releases
- This section consolidates changes from releases v0.1.5 through v0.1.14 that the skill previously skipped. For full details, see `CHANGELOG.md`
- **v0.1.5**: Added `--color auto|always|never` global flag; fixed locale fall-through bug in error messages
- **v0.1.6**: Added `--follow-symlinks` to traversal commands; `cargo deny` license allowlist expanded
- **v0.1.7**: Fixed `RUSTSEC-2026-0009` via `time = "0.3.47+" DEPTH_LIMIT=32`; added `--invert` to `search`
- **v0.1.8**: Added `--sort` to `search` and `count --by-size`; improved `--max-count` semantics
- **v0.1.9**: Added `--max-filesize` global flag; `transform` rewritten with proper error context
- **v0.1.10**: Added `--batch-size` to `batch`; miri CI gate added (nightly-only); 320 tests baseline
- **v0.1.11**: Added `set`, `get`, `del` skeleton (incomplete — completed in v0.1.12); `--preserve-timestamps` to `edit`; +29 tests
- **v0.1.12**: +96 tests, 5 new error codes, 6 new subcommands, WAL sidecar, tree-sitter, 7 ADRs, 7 schemas
- **v0.1.13/v0.1.14**: Windows CI fixes (libc E0433; deterministic `write --line-ending auto` on new files); +2 unit tests
- **v0.1.15**: This release. G117 (multi-pair edit fuzzy parity + `pair_results` + `--partial`), G118 (`write` resolves the target via `validate_path` before pre-steps), GAP 18 (snapshot `dir_fsync` redacted), CI MSRV 1.85→1.88; 461 tests, ADRs 0026-0027
- **v0.1.18**: G118 extended to replace (G118+R), G119 intelligent WAL cleanup (wal-heal subcommand), G120 empty-stdin guard for read/hash/edit/apply, GAP 18 follow-up; 502 tests (44 suites, 0 failed, 3 ignored), ADRs 0028-0030, 30 subcommands total


## Agent-First Patterns v0.1.12
### Required — v0.1.12 Specific Patterns
- USE `set`/`get`/`del` instead of parsing TOML/JSON manually in agent code
- USE `query --kinds` first to discover node kinds before running expensive S-expression queries
- USE `outline --kind` to extract function signatures without parsing source code
- USE `case --dry-run` before any bulk rename, then capture the file count from the dry-run output
- USE `--syntax-check` on `write` when modifying source files, to fail fast on syntax errors
- USE `recover_orphan_journals` consultively — never auto-replay or auto-delete
- USE the new exit codes 83, 88, 91, 92, 93 in retry logic: LockTimeout is retryable, SyntaxError is not, Orphanjournal requires user decision
- USE `tree-sitter-language-pack` download offline pre-flight in CI: `cargo install --locked atomwrite` will download parsers on first use

### Required — Pattern: Pre-Flight Syntax Check
```bash
# Validate Rust source before commit
atomwrite --workspace . write --syntax-check src/lib.rs < new_lib.rs
# Exit 0 on success, exit 88 (SyntaxError) on failure
```

### Required — Pattern: Batch Config Update With Locking
```bash
# Update multiple TOML keys atomically with optimistic locking
{
  echo '{"op":"set","target":"config.toml","key_path":"database.pool.max","value":"20"}'
  echo '{"op":"set","target":"config.toml","key_path":"features.experimental","value":"true"}'
} | atomwrite --workspace . batch --transaction --dry-run
```

### Required — Pattern: AST-Aware Code Search
```bash
# Find all functions named "main" across the codebase
atomwrite --workspace . query -Q '(function_item name: (identifier) @name (#eq? @name "main"))' src/
```

### Required — Pattern: Outline-Based Code Review
```bash
# Get a quick map of all top-level items in a file
atomwrite --workspace . outline src/lib.rs | jaq '.items[] | "\(.kind): \(.name)"'
```

## v0.1.22 (2026-06-17) — Sequential Edit Patterns Re-capture and edit-loop

### Pattern — Sequential Edits with Checksum Re-capture

When you chain multiple `edit` calls on the same file, each `edit` changes the BLAKE3 checksum. Without re-capturing the checksum before each `--expect-checksum`, you receive spurious `STATE_DRIFT` (exit 82).

**Pattern A — explicit re-capture**:

```bash
CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
echo "novo" | atomwrite --workspace . edit --after-line 10 \
  --expect-checksum "$CS" src/foo.rs

# Re-capture before the next edit
CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
echo "outro" | atomwrite --workspace . edit --after-line 20 \
  --expect-chksum "$CS" src/foo.rs
```

**Pattern B — `--allow-sequential-drift` flag** (opt-in):

```bash
CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
echo "novo" | atomwrite --workspace . edit --allow-sequential-drift \
  --after-line 10 --expect-checksum "$CS" src/foo.rs
echo "outro" | atomwrite --workspace . edit --allow-sequential-drift \
  --after-line 20 --expect-checksum "$CS" src/foo.rs
```

**Pattern C — `edit-loop` sub-command** (N edits in 1 invocation):

```bash
echo '[{"old":"foo","new":"bar"},{"old":"baz","new":"qux"}]' \
  | atomwrite --workspace . edit-loop --backup --keep-backup src/foo.rs
```

### Pattern — Prune-Backups

```bash
# List backups that would be removed (without deleting)
atomwrite --workspace . prune-backups --max-age 86400 --dry-run /path/

# Remove backups older than 24 hours
atomwrite --workspace . prune-backups --max-age 86400 /path/

# Keep only the 3 most recent backups
atomwrite --workspace . prune-backups --max-count 3 /path/
```
