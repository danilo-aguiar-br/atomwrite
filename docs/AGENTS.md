# atomwrite -- Agent Integration Contract


[Leia em Portugues](AGENTS.pt-BR.md)


## What's New in v0.1.12

This section summarizes v0.1.12 changes that are most relevant to AI agents using atomwrite as a tool. All 13 gaps from the PRD audit that were closed in v0.1.11+v0.1.12 are listed below.

### Subcommands Added (v14 Tier 3)

- `set <PATH> <KEY_PATH> <VALUE>` — write a value at a dotted path in a TOML or JSON file, preserving comments and key order via `toml_edit`. Use this instead of rewriting the entire config file (saves tokens, preserves formatting).
- `get <PATH> <KEY_PATH>` — read a value at a dotted path. NDJSON: `{"type":"get","key_path","value","found","format"}`. Use this instead of reading the whole config file.
- `del <PATH> <KEY_PATH>` — remove a key. `--force-missing` flag treats missing keys as a no-op success. Use this for idempotent cleanup scripts.
- `case <PATHS...> --subvert OLD NEW --to <style>` — rename identifiers across multiple files via `heck`. Styles: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`. Use this for renaming 1-N identifiers across an entire module in a single call.
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` — walk a tree-sitter AST and emit nodes as NDJSON. 305 languages via `tree-sitter-language-pack`. Use this for semantic code analysis.
- `outline <PATH> [--kind <KIND>] [--positions]` — extract high-level structure (functions, classes, structs, enums, traits, modules) as NDJSON. Use this for code maps before refactoring.

### Flags Added (Critical for Agents)

- `--format raw` (alias `--raw`) on `read` — emit raw bytes for Unix composability with `sed`, `awk`, `diff`, `patch`. G81.
- `--syntax-check` on `write` — invoke tree-sitter parser (24 languages) to validate code. Exit 88 on syntax error. G72.
- `--max-filesize <BYTES>` on `search` — skip files larger than limit (default 10 MiB). G68.
- `--max-columns <N>` on `search` — truncate matches with >N columns (default 500). G68.
- `--literal` (alias `-F`) on `replace` — disable regex interpretation. G66.
- `--rules <file.yaml>` and `--inline-rules <YAML>` on `transform` — multi-rule YAML for cascading refactors. G44.
- `--batch-size <N>` on `batch` — control peak memory (default 100). G77.
- `--no-reflink` on `backup`/`copy` — disable CoW for filesystems without support. G64.
- `--include-fifo` on `write` — allow writing to named pipes. G56.
- `--strict-atomic` on `write` — abort on EXDEV instead of copy fallback. G90.
- `--lock` and `--lock-timeout <ms>` on `write`/`edit` — advisory file lock via `flock`. G54.

### Error Codes Added (5 New)

- 83 `LockTimeout` (G54 advisory file lock via flock exceeded)
- 88 `SyntaxError` (G72 `--syntax-check` via tree-sitter parser)
- 91 `ExdevFallbackDisabled` (G90 `--strict-atomic` opted out of Docker/NFS fallback)
- 92 `CopyBackBlake3Failed` (G114 in-place write lost checksum integrity)
- 93 `OrphanJournal` (G114 WAL sidecar left over from crash)
- See REQUIRED -- Exit Codes below for the full table including all 25 codes.

### Crash Recovery (G114)

- `atomic_write` writes `.atomwrite.journal.<target>.atomwrite.journal.json` with `Started`/`Committed` entries.
- `recover_orphan_journals(dir)` is consultive (no auto-replay, no auto-delete).
- The agent receives `{"type":"wal_recovery","orphan_journals":[...]}` events and decides.

### Gaps Closed (13 of Top 20 from PRD)

G39 xattr, G41 binary detect (content_inspector), G54 advisory lock, G56 FIFO skip, G58 line endings, G64 reflink CoW, G66 --literal, G68 --max-filesize, G72 syntax check, G74 --threads, G76 --diff-algorithm, G77 --batch-size, G80 SIGPIPE, G81 --format raw, G90 EXDEV fallback, G116 fuzzy match, v14 Tier 3 (set/get/del/case/query/outline).

### Dependencies Added

- `tree-sitter-language-pack = "1.8"` (305 languages, download + dynamic-loading, ~5-10MB footprint)
- `toml_edit` (preserves TOML formatting)
- `heck = "0.5"` (case conversion)
- `reflink-copy = "0.1"` (CoW backup)
- `content_inspector = "0.2"` (UTF-16 detection)
- `xattr = "1"` (extended attributes)

### Test Coverage

- **445 tests passing** (was 320 baseline, +125 new in v0.1.11+v0.1.12)
- 7 new ADRs in `docs/decisions/` (0019-0025)
- 7 new JSON schemas in `docs/schemas/` (set, get, del, case, query, outline, wal-recovery)
- See [docs/decisions/README.md](README.md) for architectural decisions

## Why atomwrite
- Your agent makes dozens of tool calls to read, write, search and replace files
- Each call costs tokens, latency and context window space
- atomwrite replaces that with one CLI that handles all file operations
- Every write is atomic: tempfile, fsync, rename, fsync-dir
- Every output is NDJSON: one JSON object per line on stdout
- Every response includes a BLAKE3 checksum
- The checksum in the response eliminates verification reads


## Economy
### Token Savings
- Each subcommand costs ~50-200 output tokens
- A batch of 100 writes costs 1 bash call instead of 100 tool calls
- The checksum in write responses saves one read call per write
- A typical refactoring session saves 500+ tool calls

### Context Window
- NDJSON output is compact and structured
- No verbose human-readable formatting to parse
- Agents consume output directly without extraction steps


## Sovereignty
- atomwrite is a standalone Rust binary with zero runtime dependencies
- No cloud service, no API key, no network access required
- All operations execute locally with sub-millisecond latency
- The agent controls every aspect of file operations
- No vendor lock-in to any specific agent framework or MCP server


## Compatible Agents
- Claude Code (Anthropic)
- Cursor (Anysphere)
- Windsurf (Codeium)
- Aider
- OpenAI Codex CLI
- Any agent that can invoke bash commands and parse JSON


## Quickstart

```bash
cargo install atomwrite
echo "hello" | atomwrite write src/hello.txt
atomwrite read src/hello.txt
atomwrite read --format raw src/hello.txt | wc -l
atomwrite search 'hello' src/
atomwrite replace 'hello' 'world' src/
atomwrite calc "2 hours + 30 minutes to seconds"
```


## 28 Subcommands
- `read` -- read files with metadata, checksum, optional content; `--format raw` (alias `--raw`) emits raw bytes for Unix composability (G81); `--grep <REGEX>` filters returned lines
- `write` -- create or overwrite files atomically via stdin; `--syntax-check` validates with tree-sitter after write (G72, exit 88)
- `edit` -- surgically edit by line number, text marker or exact match; `--fuzzy auto|off|aggressive` for fuzzy matching; `--multi` for NDJSON multi-edit
- `search` -- search file contents in parallel (ripgrep engine); supports `--context N`, `--max-count N`, `--invert`, `--sort path`, `--fixed`, `--word`, `--case-insensitive`, `--include`, `--exclude`
- `replace` -- replace text across files with atomic writes
- `hash` -- calculate BLAKE3 checksums
- `delete` -- delete files with optional backup
- `count` -- count lines, files by extension
- `diff` -- compare two files (unified, stat, or changes)
- `move` -- move or rename files atomically
- `copy` -- copy files with checksum verification
- `list` -- list project file structure with metadata
- `extract` -- extract fields from NDJSON or text columns
- `calc` -- evaluate math expressions and unit conversions (fend engine)
- `regex` -- generate regex from examples (grex engine)
- `transform` -- structural AST search and rewrite (ast-grep, 306 languages)
- `scope` -- grammatical scoping on code categories; `--delete` to remove matches; `--action upper|lower|titlecase|squeeze` for text transforms; `--replace-with "text"` for custom replacement; `--query` for prepared queries (comments, fn, strings, struct, etc); `--pattern` for custom AST patterns; supports Rust (30 queries), Python (13), JS/TS (11), Go (8)
- `backup` -- create timestamped backups with BLAKE3 checksums; `--retention` for retention period, `--dry-run` for preview
- `rollback` -- restore from backup; `--timestamp` or `--latest` to select backup, `--verify` for checksum validation, `--dry-run` for preview
- `apply` -- apply patches from stdin with auto-format detection (unified diff, SEARCH/REPLACE blocks, markdown-fenced, full file); `--format` to force format, `--backup` for safety, `--dry-run` for preview
- `batch` -- execute multiple operations from NDJSON manifest (write, replace, delete, edit, hash, move, copy); supports `--transaction` for all-or-nothing
- `completions` -- generate shell completions; use `--install` to install to XDG data directory
- `set` -- (v0.1.12, v14 Tier 3) write a value at a dotted path in a TOML or JSON file via `toml_edit`; auto-coerces int/bool/float/string
- `get` -- (v0.1.12, v14 Tier 3) read a value at a dotted path; NDJSON: `{"type":"get","key_path","value","found","format"}`
- `del` -- (v0.1.12, v14 Tier 3) remove a key; `--force-missing` flag treats missing keys as a no-op success
- `case` -- (v0.1.12, v14 Tier 3) rename identifiers across multiple files via `heck`; styles: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`
- `query` -- (v0.1.12, v14 Tier 3, G72) walk a tree-sitter AST and emit nodes as NDJSON; 305 languages via `tree-sitter-language-pack`; modes: `--kinds`, `--query <KIND>`, `-Q <KIND>`, `--tree`, `--positions`
- `outline` -- (v0.1.12, v14 Tier 3) extract high-level structure (functions, classes, structs, enums, traits, modules) as NDJSON


## REQUIRED -- Output Contract
- stdout: ALWAYS NDJSON structured (one JSON object per line)
- stderr: logs only (tracing format, only with `--verbose`)
- Every object has a `"type"` discriminator field
- Flush after each line
- NEVER parse stderr for structured data
- ALWAYS parse stdout line by line as JSON


## REQUIRED -- CRUD Contract
### Create (write)
- Pipe content to stdin
- Receive path, bytes_written, checksum, platform info
- Use `--backup` to preserve previous version
- Use `--expect-checksum` for optimistic locking

### Read (read)
- Receive path, content, lines, bytes, checksum, permissions, modified, kind
- Use `--stat` to skip content (metadata only)
- Use `--lines START:END` for partial reads (1-based inclusive)
- Use `--head N` for first N lines, `--tail N` for last N lines
- Use `--grep <REGEX>` to filter returned lines to those matching a regex
- Binary files are auto-detected and content is omitted

### Update (edit, replace, transform)
- `edit` -- surgical: by line number, text marker or exact match
- `replace` -- bulk: across multiple files with regex support
- `transform` -- structural: AST-aware rewrite across codebases
- All three return checksums before and after modification
- All three support `--dry-run` for preview
- `edit` and `replace` support `--preserve-timestamps` to opt out of mtime updates (default: mtime is updated to reflect the change, so build systems like cargo/make/cmake detect the source change without manual `touch`)
- `edit` and `replace` NDJSON output include `mtime_preserved: bool` field to verify which path was taken

### Delete (delete)
- Receive path, bytes, checksum_before
- Use `--backup` for reversible deletion
- Use `--recursive` for directories
- Use `--dry-run` to preview


## REQUIRED -- JSON Output Format
### Write Response

```json
{"type":"write","status":"ok","path":"/abs/path","bytes_written":42,"checksum":"abc...","elapsed_ms":1,"platform":{"fsync":"sync_data","dir_fsync":"sync_all"}}
```

### Read Response

```json
{"type":"read","path":"/abs/path","content":"...","lines":10,"bytes":42,"checksum":"abc...","permissions":"rw-r--r--","modified":"2026-01-01T00:00:00Z","kind":"file","binary":false}
```

### Edit Response

```json
{"type":"edit","path":"/abs/path","edits":1,"mode":"old_new","bytes_before":100,"bytes_after":110,"checksum_before":"abc...","checksum_after":"def...","lines_before":10,"lines_after":11,"elapsed_ms":1,"fuzzy":true,"strategy":"exact_whitespace","strategies_tried":2,"similarity":null}
```

- `fuzzy`: whether fuzzy matching was used (only present in `--old/--new` mode)
- `strategy`: fuzzy strategy that succeeded (only present when `fuzzy=true`)
- `strategies_tried`: number of strategies tried before success (only present in `--old/--new` mode)
- `similarity`: similarity score 0.0-1.0 (only present for `block_anchor` strategy)

### Search Match

```json
{"type":"match","path":"/abs/path","line_number":5,"lines":"matched line content","byte_offset":120,"submatches":[{"match":"text","start":0,"end":4}]}
```

### Replace Result

```json
{"type":"replace","path":"/abs/path","replacements":3,"bytes_before":100,"bytes_after":105,"checksum_before":"abc...","checksum_after":"def...","elapsed_ms":1}
```

### Error Envelope

```json
{"error":true,"code":"FILE_NOT_FOUND","exit":4,"message":"file not found: src/missing.rs","path":"src/missing.rs","error_class":"permanent","retryable":false,"suggestion":"verify the file path exists","workspace":null}
```

- `workspace` field appears only on `WORKSPACE_JAIL` errors and reports the resolved workspace root (may be `null`)
- `suggestion` is context-aware: `WORKSPACE_JAIL` suggestion changes based on whether `--workspace` was provided
- See `docs/schemas/` for full JSON Schema definitions of all output types (the `error-output.schema.json` defines all 20 error codes and the `workspace` field)


## REQUIRED -- Exit Codes
- 0: success
- 1: no matches (search/replace found nothing)
- 4: file not found
- 13: permission denied
- 28: disk full
- 30: quota exceeded
- 65: invalid input, file too large, or binary file
- 73: cross-device rename
- 74: I/O error
- 78: config invalid
- 81: checksum verification failed (hash --verify mismatch)
- 82: state drift (checksum mismatch on write)
- 83: lock timeout (G54 advisory file lock via flock, `--lock-timeout` exceeded)
- 85: FIFO detected (named pipe cannot be atomically written)
- 86: device file detected (block or character device)
- 88: syntax error detected (G72 `--syntax-check` via tree-sitter parser)
- 91: EXDEV fallback disabled (`--strict-atomic` opted out of G90 Docker/NFS fallback)
- 92: copy-back BLAKE3 failed (G114 in-place write lost checksum integrity)
- 93: orphan journal recovered (G114 WAL sidecar left over from crash)
- 126: workspace jail violated
- 127: symlink blocked
- 128: file immutable
- 130: SIGINT
- 141: SIGPIPE (broken pipe)
- 143: SIGTERM
- 255: internal error


## REQUIRED -- Error Handling
- Errors emit JSON on stdout with `error: true`
- Fields: `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `workspace`
- `error_class` values: `permanent`, `transient`, `conflict`, `precondition_failed`
- `retryable` is true for `transient` and `conflict` classes
- `workspace` field appears only on `WORKSPACE_JAIL` errors and reports the resolved workspace root
- All 25 error variants carry actionable `suggestion` text (added in v0.1.4, GAP 13)
- `WorkspaceJail` suggestion is **context-aware**: when `--workspace` or `ATOMWRITE_WORKSPACE` is already set, the suggestion says "use a path inside the workspace (<root>)" rather than re-prompting the flag
- The `BinaryFile` suggestion recommends `read --stat` for metadata-only reads (the previous phantom `--force-text` reference is removed)
- The `FileImmutable` suggestion mentions `chattr -i` (Unix) and `fsutil` (Windows)
- The `NoMatches` suggestion guides pattern broadening and `--include`/`--exclude` filter review
- Only `BrokenPipe` (SIGPIPE) has no suggestion because the error is not actionable by the user

### Context-Aware Suggestion API (v0.1.4)
- New Rust API: `ErrorJson::from_error_with_context(err, &ErrorContext)` accepts workspace provenance
- `ErrorContext` struct has `workspace_provided: bool` and `workspace: Option<PathBuf>`
- Legacy `ErrorJson::from_error(err)` still works and produces the same output as the new API with a default context
- Programmatic consumers can call `from_error_with_context` to influence the suggestion text


## REQUIRED -- Retry Strategy
### Transient Errors (retryable: true)
- `DISK_FULL` (exit 28) -- wait for space and retry
- `QUOTA_EXCEEDED` (exit 30) -- wait for quota reset and retry
- `IO_ERROR` (exit 74) -- retry with exponential backoff

### Conflict Errors (retryable: true)
- `STATE_DRIFT` (exit 82) -- re-read file, get new checksum, retry with updated `--expect-checksum`
- `CROSS_DEVICE` (exit 73) -- atomwrite handles this internally via copy-then-delete

### Permanent Errors (retryable: false)
- `FILE_NOT_FOUND` (exit 4) -- verify path exists before retrying
- `PERMISSION_DENIED` (exit 13) -- do not retry without permission fix
- `INVALID_INPUT` (exit 65) -- fix the input and retry
- `CONFIG_INVALID` (exit 78) -- fix the configuration and retry
- `CHECKSUM_VERIFY_FAILED` (exit 81) -- the hash passed to `--verify` did not match; re-read the file
- `FILE_TOO_LARGE` (exit 65) -- increase `--max-filesize` or process a smaller file
- `WORKSPACE_JAIL` (exit 126) -- do not retry, path is outside workspace
- `SYMLINK_BLOCKED` (exit 127) -- do not retry with symlinks disabled
- `IMMUTABLE_FILE` (exit 128) -- do not retry without removing immutable flag
- `INTERNAL_ERROR` (exit 255) -- report as a bug; not actionable by the user

### Precondition Failed (retryable: false)
- `BINARY_FILE` (exit 65) -- use `--stat` mode to read metadata without content
- `IMMUTABLE_FILE` (exit 128) -- remove immutable flag first (chattr -i on Unix, fsutil on Windows)
- `WORKSPACE_JAIL` (exit 126) -- adjust `--workspace` boundary
- `FIFO_DETECTED` (exit 85) -- skip this file or use stdin redirection
- `DEVICE_FILE` (exit 86) -- skip this file or use stdin redirection


## REQUIRED -- Global Flags
- `--workspace <PATH>` -- ALWAYS pass to restrict operations to project root
- `--verbose` / `-v` -- enable tracing on stderr
- `--quiet` / `-q` -- suppress non-essential output
- `--color <auto|always|never>` -- control colored output
- `--no-color` -- disable colored output (equivalent to `--color never`)
- `--no-gitignore` -- do not respect .gitignore rules
- `--hidden` -- include hidden files and directories
- `--follow-symlinks` -- follow symbolic links
- `--threads <N>` / `-j <N>` -- parallel threads (0 = all cores)
- `--max-filesize <BYTES>` -- skip files larger than limit
- `--timeout <SECONDS>` -- global operation timeout (0 = no timeout, default 0). Use to bound long-running searches, batches, and replace operations
- `--json-schema` -- emit JSON schema for the subcommand output
- `--lang <LOCALE>` -- override display locale (en, pt-BR) via `ATOMWRITE_LANG` env


## FORBIDDEN -- Common Pitfalls
- NEVER parse stderr for data; it contains only tracing logs
- NEVER assume exit code 1 is a fatal error; it means zero matches in search
- NEVER skip `--workspace` when running as an agent
- NEVER skip `--dry-run` before destructive batch operations
- NEVER use unquoted expressions with `calc`; the shell will interpolate
- NEVER ignore `checksum_before` and `checksum_after` in edit/replace responses
- NEVER retry `permanent` or `precondition_failed` errors without fixing the cause


## REQUIRED -- Token Budget
- Each subcommand: 1 bash call, ~50-200 output tokens
- Batch mode: 1 bash call for N operations
- Checksum in response eliminates 1 verification read per write
- A typical agent session saves 500+ tool calls versus individual operations


## REQUIRED -- Optimistic Locking
- Read a file and capture its `checksum` from the response
- Pass the checksum via `--expect-checksum` on the next write or edit
- If the file changed between read and write, atomwrite returns exit 82 (`STATE_DRIFT`)
- Re-read the file to get the current checksum and retry
- This prevents lost updates in concurrent agent workflows
