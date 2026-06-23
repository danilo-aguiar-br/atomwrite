# atomwrite -- Agent Integration Contract


[Leia em Português](AGENTS.pt-BR.md)


## What's New in v0.1.25

- 49 gaps resolved (GAP-071 through GAP-134) in 6 rounds of end-to-end audit (~505 scenarios)
- NEW SUBCOMMAND: `verify <PATH> --checksum <BLAKE3>` — dedicated checksum verification (33 subcommands total)
- NEW: `.atomwrite.toml` config file — hierarchy: CLI > env > local `.atomwrite.toml` > XDG `~/.config/atomwrite/config.toml` > defaults
- NEW FLAGS: `delete --older-than <DURATION>` (s/m/h/d/w), `delete --confirm` (preview mode), `replace --preserve-case` (UPPER/lower/Title adaptation), `search --pcre2`, `edit --fuzzy-threshold <FLOAT>`, `scope --action symbols|normalize`
- NEW FLAGS: `copy --no-reflink`, `copy --preserve-xattr`, `move --preserve-hardlinks`
- FUZZY MATCHING: Jaro-Winkler (`context_aware_jw` strategy) added to the 9-strategy cascade; `diff_preview` field in responses when fuzzy match used
- CRITICAL FIXES: `write --backup` no longer reports phantom `backup_path` (GAP-101), `set` no longer misroutes key into scalar TOML (GAP-102), `size_delta_pct` overflow fixed (GAP-120)
- HIGH FIXES: `copy --preserve` now preserves permissions AND mtime (GAP-103/133), `copy --backup`/`replace --backup` now retain `.bak` files (GAP-104/105), `write --require-backup` enforces retention (GAP-106), I/O errors now emit NDJSON envelope (GAP-098)
- MEDIUM FIXES: `hash` output field renamed `value` → `checksum` (GAP-107), `list --long` ISO 8601 dates (GAP-116), `outline --positions` now emits byte offsets (GAP-109), `list` on nonexistent dir returns exit 4 (GAP-110), `get`/`del` missing key returns exit 65 INVALID_INPUT (GAP-111), `batch move/copy` respects `--force` (GAP-108), `scope --query comments` captures block comments (GAP-123)
- BEHAVIOR CHANGE: `case` without `--subvert` returns exit 65 with helpful message (GAP-127)
- BEHAVIOR CHANGE: `scope --query test-fn` returns actionable error with alternatives (GAP-134)
- Property-based tests for fuzzy matching via proptest (GAP-086)
- Flag conflict validation: `append+prepend`, `fixed+regex`, `literal+regex` rejected at parse time (GAP-093)
- 631 tests passing (10 new), 0 clippy warnings, 0 fmt diffs
- 3 JSON schemas updated (del-result, write-output, read-output)


## What's New in v0.1.24

- 52 bugs fixed (GAP-2026-019 through GAP-2026-070) in comprehensive end-to-end audit
- TYPED ERROR AUDIT: ALL `anyhow::bail!()` in user-facing paths converted to `AtomwriteError` variants. Every error now emits structured JSON on stdout with the correct exit code. No more silent exit 1 without JSON envelope.
- Exit code changes: paths that returned exit 1 now return exit 4 (NotFound) or exit 65 (InvalidInput). Agents MUST update error parsers.
- `delete --recursive` NOW WORKS (was no-op for directories)
- `hash --recursive` NOW WORKS (was accepted but never walked directories)
- `search --multiline` NOW WORKS (flag was not propagated to SearcherBuilder)
- `replace` REJECTS empty pattern (was silently destroying all files)
- `batch --transaction` properly reverts `move`/`copy` on rollback
- `scope`/`count`/`transform`/`diff` now resolve walk roots against `--workspace`
- `get`/`set`/`del` values no longer doubly-quoted (raw value in JSON response)
- Backup timestamp: `YYYYMMDD_HHMMSS_mmm` (millisecond resolution prevents collision)
- `rollback --timestamp` accepts PREFIX match (backward-compatible with old format)
- `prune-backups --max-count` sorts by filename (lexicographic) instead of mtime
- `case --subvert` changed from greedy `num_args=2..` to exact `num_args=2`
- `edit --multi` field `op` now optional (inferred as "exact" when `old`+`new` present)
- `hash --stdin` no longer requires PATHS argument
- `regex` removed `allow_hyphen_values`; use POSIX `--` for hyphen-starting examples
- ARGUMENT_PARSE_ERROR (exit 2) gains context-aware `suggestion` field (ADR-0045)
- 3 new ADRs: 0045 (clap suggestion), 0046 (diff resolve-first), 0047 (scope read-only)
- 621 tests passing (12 new)


## What's New in v0.1.23

- GAP-2026-015 closed — `allow_hyphen_values = true` added to 15 CLI text-accepting fields in 8 structs. Values starting with `-` (Markdown bullets `- item`, negative numbers `-5`, YAML entries `- key: val`, diff content `--- a/file`) are now accepted as data, not parsed as flags. Excluded: `CaseArgs.subvert` (incompatible with `num_args = 2..`). See ADR-0041.
- GAP-2026-016 closed — backup is now enabled by default in 9 content-mutating structs: `WriteArgs`, `EditArgs`, `EditLoopArgs`, `ReplaceArgs`, `TransformArgs`, `ApplyArgs`, `SetArgs`, `DelArgs`, `CaseArgs`. Backup is auto-deleted after success (`keep_backup: false` default unchanged). Opt-out: `--no-backup` flag or `ATOMWRITE_BACKUP=0` env var. Precedence: CLI flag > env var > default (true). 4 non-content structs unchanged: `DeleteArgs`, `MoveArgs`, `CopyArgs`, `RollbackArgs`. See ADR-0042.
- GAP-2026-017 closed — writes that shrink file size by >50% are now BLOCKED when `--expect-checksum` is active. Returns exit 65 (`INVALID_INPUT`) with suggestion to pass `--allow-shrink`. The `risk_assessment` (L1) becomes blocking (not just informative) when `--expect-checksum` is active and the file shrinks. Without `--expect-checksum`, behavior is unchanged. See ADR-0043.
- GAP-2026-018 closed — new `--old-file <PATH>` and `--new-file <PATH>` flags on `edit`. Content is read from files inside the atomwrite process, bypassing shell expansion and kernel ARG_MAX (~131 KB). `conflicts_with` prevents mixing `--old` with `--old-file` (exit 2). Cross-mixing guard rejects `--old` + `--new-file` and `--old-file` + `--new` (exit 65). `strip_file_trailing_newline()` strips one trailing newline for argv parity. `PairResult.source` reports "arg" or "file". See ADR-0044.
- 609+ tests passing (31 new: 12 hyphen + 7 backup + 4 shrink + 8 old-file)
- 4 new ADRs: 0041, 0042, 0043, 0044

## What's New in v0.1.22

- **GAP-2026-012 Front 3 closed** — new subcommand `edit-loop [PATH]` applies N pairs `{old, new}` in 1 invocation via NDJSON on stdin. Reduces 5 sequential `edit` calls (5 subprocess spawns, 5 checksum recaptures) to a single atomic write. Supports `--partial`, `--backup`, `--keep-backup`, `--line-ending`, `--preserve-timestamps`, `--fuzzy`, `--expect-checksum`. See `tests/cli_v0121_edit_loop.rs` and ADR-0039.
- **GAP-2026-013 Front 2 closed** — new subcommand `prune-backups [PATHS]...` provides manual cleanup of legacy `.bak.YYYYMMDD_HHMMSS` files from v0.1.20 and earlier. Flags: `--max-age-secs <SECONDS>`, `--max-count <N>`, `--dry-run` (defaults to true for safety). Reuses `cleanup_old_backups_in` from `src/atomic.rs`. See `tests/cli_v0121_prune_backups.rs` and ADR-0040.
- 2 new NDJSON schemas: `edit-loop-output.schema.json` (with `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`) and `prune-backups-output.schema.json` (with `action`, `path`, `reason`, `total`, `elapsed_ms`).
- 32 subcommands total (added `edit-loop` and `prune-backups` to the previous 30).

## What's New in v0.1.21

- **GAP-2026-012 closed** — new flag `--allow-sequential-drift` on `edit` opts into accepting checksUM drift between sequential edits on the same file, eliminating the false-positive `STATE_DRIFT` (exit 82) when the same agent owns the file across iterations. Default behavior (without flag) is unchanged: re-capture checksum between edits.
- **GAP-2026-013 Front 4 closed** — `edit` and `rollback` now expose `--backup` and `--retention` flags for parity with `write` and `replace`. Default `backup: false` preserves existing behavior.
- **GAP-2026-014 v2 closed** — backup default changed to **delete after success**. Previously, backups accumulated with the `retention: u8` (default 5) policy. Now, `--backup` creates a `.bak.{timestamp}` which is `rm`'d inline on success. New opt-in `--keep-backup` preserves the backup. Failed operations always preserve the backup for inspection. See ADR-0038 and `tests/cli_v0121_backup_keep_flag.rs`.
- New pattern documented: capture checksum at the top of each loop iteration via `atomwrite read --json | jaq -r '.checksum'`, then pass to `--expect-checksum`. Eliminates STATE_DRIFT for sequential edits by the same agent.

## What's New in v0.1.15 (extended in v0.1.18)

- G117: `edit` multi-pair `--old/--new` now runs the same 9-strategy fuzzy cascade as the single path, per pair. Success envelopes gain `pairs_total` and `pair_results[{index, matched, strategy, similarity}]`; failures gain `failed_pair_index` (exit 65, file intact). New opt-in `--partial` applies the pairs that match and reports the unmatched ones.
- G118: `write` resolves the target against `--workspace` BEFORE append/prepend, automatic line ending detection, and `--expect-checksum`. With diverging CWD, checksum drift now returns exit 82 (`STATE_DRIFT`) instead of silently overwriting, and targets outside the jail return exit 126 early.
- 542 tests passing; ADRs 0031-0037 document the closure of seven gaps between v0.1.19-v0.1.20.

## What's New in v0.1.12

This section summarizes the v0.1.12 changes most relevant to AI agents using atomwrite as a tool. All 13 gaps closed in the PRD audit in v0.1.11+v0.1.12 are listed below.

### Subcommands Added (v14 Tier 3)

- `set <PATH> <KEY_PATH> <VALUE>` — writes a value at a dotted path in a TOML or JSON file, preserving comments and key order via `toml_edit`. Use this instead of rewriting the entire config file (saves tokens, preserves formatting).
- `get <PATH> <KEY_PATH>` — reads a value at a dotted path. NDJSON: `{"type":"get","key_path","value","found","format"}`. Use this instead of reading the entire config file.
- `del <PATH> <KEY_PATH>` — removes a key. The `--force-missing` flag treats absent keys as no-op success. Use this for idempotent cleanup scripts.
- `case <PATHS...> --subvert OLD NEW --to <style>` — renames identifiers across multiple files via `heck`. Styles: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`.
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` — walks a tree-sitter AST and emits nodes as NDJSON. 305 languages via `tree-sitter-language-pack`.
- `outline <PATH> [--kind <KIND>] [--positions]` — extracts high-level structure (functions, classes, structs, enums, traits, modules) as NDJSON.

### Flags Added (Critical for Agents)

- `--format raw` (alias `--raw`) on `read` — emits raw bytes for Unix composability with `sed`, `awk`, `diff`, `patch`. G81.
- `--syntax-check` on `write` — invokes the tree-sitter parser (24 languages) to validate code. Exit 88 on syntax error. G72.
- `--max-filesize <BYTES>` on `search` — skips files larger than the limit (default 10 MiB). G68.
- `--max-columns <N>` on `search` — truncates matches wider than N columns (default 500). G68.
- `--literal` (alias `-F`) on `replace` — disables regex interpretation. G66.
- `--rules <file.yaml>` and `--inline-rules <YAML>` on `transform` — multi-rule YAML for cascading refactors. G44.
- `--batch-size <N>` on `batch` — controls memory peak (default 100). G77.
- `--no-reflink` on `backup`/`copy` — disables CoW for filesystems without support. G64.
- `--include-fifo` on `write` — allows writing to named pipes. G56.
- `--strict-atomic` on `write` — aborts on EXDEV instead of copy fallback. G90.
- `--lock` and `--lock-timeout <ms>` on `write`/`edit` — advisory lock via `flock`. G54.

### Error Codes Added (5 New)

- 83 `LockTimeout` (G54 advisory lock via flock exceeded)
- 88 `SyntaxError` (G72 `--syntax-check` via tree-sitter parser)
- 91 `ExdevFallbackDisabled` (G90 `--strict-atomic` opts out of Docker/NFS fallback)
- 92 `CopyBackBlake3Failed` (G114 in-place write lost checksum integrity)
- 93 `OrphanJournal` (G114 WAL sidecar left by crash)
- See REQUIRED -- Exit Codes below for the full table with all 25 codes.

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
- `reflink-copy = "0.1"` (backup CoW)
- `content_inspector = "0.2"` (UTF-16 detection)
- `xattr = "1"` (extended attributes)

### Test Coverage

- **542 tests passing** (461 baseline v0.1.15 + 8 G117 edge cases v0.1.18 + 2 G118 replace pre-validation v0.1.18 + 16 cross-platform/WAL/audit increments v0.1.16-v0.1.18)
- 9 ADRs in `docs/decisions/` (0019-0027)
- 7 new JSON schemas in `docs/schemas/` (set, get, del, case, query, outline, wal-recovery)
- See [docs/decisions/README.md](README.md) for architectural decisions

## Why atomwrite
- Your agent makes dozens of tool calls to read, write, search, and replace files
- Each call costs tokens, latency, and context window space
- atomwrite replaces all of this with one CLI that handles every file operation
- Every write is atomic: tempfile, fsync, rename, fsync-dir
- Every output is NDJSON: one JSON object per line on stdout
- Every response includes a BLAKE3 checksum
- The checksum in the response eliminates verification reads


## Economy
### Token Economy
- Each subcommand costs ~50-200 output tokens
- A batch of 100 writes costs 1 bash call instead of 100 tool calls
- The checksum in write responses saves one read per write
- A typical refactoring session saves 500+ tool calls

### Context Window
- NDJSON output is compact and structured
- No verbose human formatting to interpret
- Agents consume the output directly without extraction steps


## Sovereignty
- atomwrite is a standalone Rust binary with zero runtime dependencies
- No cloud service, no API key, no network access required
- All operations execute locally with sub-millisecond latency
- The agent controls all aspects of file operations
- No vendor lock-in to any agent framework or MCP server


## Compatible Agents
- Claude Code (Anthropic)
- Cursor (Anysphere)
- Windsurf (Codeium)
- Aider
- OpenAI Codex CLI
- Any agent that invokes bash commands and interprets JSON


## Quickstart

```bash
cargo install atomwrite
echo "hello" | atomwrite write src/hello.txt
atomwrite read src/hello.txt
atomwrite search 'hello' src/
atomwrite replace 'hello' 'world' src/
atomwrite calc "2 hours + 30 minutes to seconds"
```


## 33 Subcommands
- `read` -- reads files with metadata, checksum, optional content; `--format raw` (alias `--raw`) emits raw bytes for Unix composability (G81); `--grep <REGEX>` filters returned lines
- `write` -- creates or overwrites files atomically via stdin; `--syntax-check` valida com tree-sitter após escrita (G72, exit 88)
- `edit` -- edits surgically by line number, text marker, or exact match; `--fuzzy auto|off|aggressive` for fuzzy matching; `--multi` for NDJSON multi-edit
- `search` -- searches file content in parallel (ripgrep engine); supports `--context N`, `--max-count N`, `--invert`, `--sort path`, `--fixed`, `--word`, `--case-insensitive`, `--include`, `--exclude`
- `replace` -- replaces text across multiple files with atomic writes
- `hash` -- computes BLAKE3 checksums
- `delete` -- deletes files with optional backup
- `count` -- counts lines, files by extension
- `diff` -- compares two files (unified, stat, or changes)
- `move` -- moves or renames files atomically
- `copy` -- copies files with checksum verification
- `list` -- lists project file structure with metadata
- `extract` -- extracts fields from NDJSON or text columns
- `calc` -- evaluates mathematical expressions and unit conversions (fend engine)
- `regex` -- generates regex from examples (grex engine)
- `transform` -- structural AST search and rewrite (ast-grep, 306 languages)
- `scope` -- grammatical scope over code categories; `--delete` to remove matches; `--action upper|lower|titlecase|squeeze` for text transformations; `--replace-with "text"` for custom substitution; `--query` for prepared queries (comments, fn, strings, struct, etc); `--pattern` for custom AST patterns; supports Rust (30 queries), Python (13), JS/TS (11), Go (8)
- `backup` -- creates timestamped backups with BLAKE3 checksums; `--retention` for retention period, `--dry-run` for preview
- `rollback` -- restores from backup; `--timestamp` or `--latest` to select backup, `--verify` for checksum validation, `--dry-run` for preview
- `apply` -- applies patches from stdin with automatic format detection (unified diff, SEARCH/REPLACE blocks, markdown-fenced, full file); `--format` to force format, `--backup` for safety, `--dry-run` for preview
- `batch` -- executes multiple operations from NDJSON manifest (write, replace, delete, edit, hash, move, copy); supports `--transaction` for all-or-nothing
- `completions` -- generates shell completions
- `set` -- (v0.1.12, v14 Tier 3) writes a value at a dotted path in a TOML or JSON file via `toml_edit`; auto-coerces int/bool/float/string
- `get` -- (v0.1.12, v14 Tier 3) reads a value at a dotted path; NDJSON: `{"type":"get","key_path","value","found","format"}`
- `del` -- (v0.1.12, v14 Tier 3) removes a key; `--force-missing` flag treats absent keys as no-op success
- `case` -- (v0.1.12, v14 Tier 3) renames identifiers across multiple files via `heck`; styles: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`
- `query` -- (v0.1.12, v14 Tier 3, G72) walks a tree-sitter AST and emits nodes as NDJSON; 305 languages via `tree-sitter-language-pack`; modes: `--kinds`, `--query <KIND>`, `-Q <KIND>`, `--tree`, `--positions`
- `outline` -- (v0.1.12, v14 Tier 3) extracts high-level structure (functions, classes, structs, enums, traits, modules) as NDJSON
- `wal-stats` -- (v0.1.18) inspects WAL journal state for telemetry and debugging; scope via `--workspace <DIR>`; NDJSON report with `terminal_committed`, `terminal_aborted`, `total_bytes`, `oldest_age_secs`
- `wal-heal` -- (v0.1.18) removes orphan terminal journals older than `--threshold-secs` (default 3600s); wall-clock budget via `--max-duration-ms` (default 100ms)
- `edit-loop` -- (v0.1.22) applies N `{old, new}` pairs in 1 invocation via NDJSON on stdin; supports `--partial`, `--backup`, `--keep-backup`, `--line-ending`, `--preserve-timestamps`, `--fuzzy`, `--expect-checksum`
- `prune-backups` -- (v0.1.22) manual cleanup of legacy `.bak.YYYYMMDD_HHMMSS` files (v0.1.20 and earlier); flags `--max-age-secs <SECONDS>`, `--max-count <N>`, `--dry-run` (default `true` for safety); NDJSON output with `path`, `reason`, `action`, `total`
- `verify` -- (v0.1.25) verify a file checksum against an expected BLAKE3 hash; delegates to `hash --verify`; exit 0 on match, exit 81 on mismatch


## REQUIRED -- Output Contract
- stdout: ALWAYS structured NDJSON (one JSON object per line)
- stderr: logs only (tracing format, only with `--verbose`)
- Every object has a discriminator field `"type"`
- Flush after each line
- NEVER interpret stderr as structured data
- ALWAYS interpret stdout line by line as JSON


## REQUIRED -- CRUD Contract
### Create (write)
- Send content via stdin
- Receive path, bytes_written, checksum, platform info
- Use `--backup` to preserve the previous version
- Use `--expect-checksum` for optimistic locking

### Read (read)
- Receive path, content, lines, bytes, checksum, permissions, modified, kind
- Use `--stat` to skip content (metadata only)
- Use `--lines START:END` for partial reads (1-based inclusive)
- Use `--head N` for first N lines, `--tail N` for last N lines
- Use `--grep <REGEX>` to filter returned lines to those matching the regex
- Binary files are auto-detected and content is omitted

### Update (edit, replace, transform)
- `edit` -- surgical: by line number, text marker, or exact match
- `replace` -- bulk: across multiple files with regex support
- `transform` -- structural: AST-based rewrite across codebases
- All three return checksums before and after modification
- All three support `--dry-run` for preview
- `edit` and `replace` support `--preserve-timestamps` to skip mtime update (default: mtime is updated to reflect the change, so build systems like cargo/make/cmake detect the source change without manual `touch`)
- The NDJSON output of `edit` and `replace` includes the `mtime_preserved: bool` field to verify which path was taken

### Delete (delete)
- Receive path, bytes, checksum_before
- Use `--backup` for reversible deletion
- Use `--recursive` for directories
- Use `--dry-run` for preview


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

- The `workspace` field only appears on `WORKSPACE_JAIL` errors and reports the resolved workspace root (may be `null`)
- `suggestion` is context-aware: `WORKSPACE_JAIL` suggestion changes based on whether `--workspace` was provided
- See `docs/schemas/` for complete JSON Schema definitions of all output types (`error-output.schema.json` defines all 20 error codes and the `workspace` field)


## REQUIRED -- Exit Codes
- 0: success
- 1: no matches (search/replace/transform/scope found nothing)
- 4: file not found
- 13: permission denied
- 28: disk full
- 30: quota exceeded
- 65: invalid input, file too large, or binary file
- 73: rename across devices
- 74: I/O error
- 78: invalid configuration
- 81: checksum verification failed (hash --verify did not match)
- 82: state drift (checksum did not match on write)
- 83: lock timeout (G54 advisory lock via flock, `--lock-timeout` exceeded)
- 85: FIFO detected (named pipe cannot be written atomically)
- 86: device file detected (block or character)
- 88: syntax error detected (G72 `--syntax-check` via tree-sitter parser)
- 91: EXDEV fallback disabled (`--strict-atomic` opts out of G90 Docker/NFS fallback)
- 92: copy-back BLAKE3 failed (G114 in-place write lost checksum integrity)
- 93: orphan journal recovered (G114 WAL sidecar left by crash)
- 126: workspace jail violation
- 127: symlink blocked
- 128: immutable file
- 130: SIGINT
- 141: SIGPIPE (broken pipe)
- 143: SIGTERM
- 255: internal error


## REQUIRED -- Error Handling
- Errors emit JSON on stdout with `error: true`
- Fields: `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `workspace`
- `error_class` values: `permanent`, `transient`, `conflict`, `precondition_failed`
- `retryable` is true for `transient` and `conflict` classes
- The `workspace` field only appears on `WORKSPACE_JAIL` errors and reports the resolved workspace root
- All 20 error variants carry actionable `suggestion` text (added in v0.1.4, GAP 13)
- The `WorkspaceJail` suggestion is **context-aware**: when `--workspace` or `ATOMWRITE_WORKSPACE` is already set, the suggestion says "use a path inside the workspace (<root>)" instead of re-asking for the flag
- The `BinaryFile` suggestion recommends `read --stat` for metadata-only reads (phantom reference to `--force-text` was removed)
- The `FileImmutable` suggestion mentions `chattr -i` (Unix) and `fsutil` (Windows)
- The `NoMatches` suggestion guides pattern broadening and reviewing `--include`/`--exclude` filters
- Only `BrokenPipe` (SIGPIPE) has no suggestion because the error is not user-actionable

### Context-Aware Suggestion API (v0.1.4)
- New Rust API: `ErrorJson::from_error_with_context(err, &ErrorContext)` accepts workspace provenance
- The `ErrorContext` struct has `workspace_provided: bool` and `workspace: Option<PathBuf>`
- The legacy `ErrorJson::from_error(err)` still works and produces the same output as the new API with default context
- Programmatic consumers can call `from_error_with_context` to influence the suggestion text


## REQUIRED -- Retry Strategy
### Transient Errors (retryable: true)
- `DISK_FULL` (exit 28) -- wait for space and retry
- `QUOTA_EXCEEDED` (exit 30) -- wait for quota reset and retry
- `IO_ERROR` (exit 74) -- retry with exponential backoff

### Conflict Errors (retryable: true)
- `STATE_DRIFT` (exit 82) -- re-read the file, get the new checksum, retry with updated `--expect-checksum`
- `CROSS_DEVICE` (exit 73) -- atomwrite handles internally via copy-then-delete

### Permanent Errors (retryable: false)
- `FILE_NOT_FOUND` (exit 4) -- verify the path exists before retrying
- `PERMISSION_DENIED` (exit 13) -- do not retry without fixing permissions
- `INVALID_INPUT` (exit 65) -- correct the input and retry
- `CONFIG_INVALID` (exit 78) -- correct the configuration and retry
- `CHECKSUM_VERIFY_FAILED` (exit 81) -- the hash passed to `--verify` did not match; re-read the file
- `FILE_TOO_LARGE` (exit 65) -- increase `--max-filesize` or process a smaller file
- `WORKSPACE_JAIL` (exit 126) -- do not retry, path is outside the workspace
- `SYMLINK_BLOCKED` (exit 127) -- do not retry with symlinks disabled
- `IMMUTABLE_FILE` (exit 128) -- do not retry without removing the immutability flag
- `INTERNAL_ERROR` (exit 255) -- report as a bug; not user-actionable

### Precondition Failed (retryable: false)
- `BINARY_FILE` (exit 65) -- use `--stat` mode to read metadata without content
- `IMMUTABLE_FILE` (exit 128) -- remove the immutability flag first (chattr -i on Unix, fsutil on Windows)
- `WORKSPACE_JAIL` (exit 126) -- adjust the `--workspace` boundary
- `FIFO_DETECTED` (exit 85) -- skip this file or use stdin redirection
- `DEVICE_FILE` (exit 86) -- skip this file or use stdin redirection


## REQUIRED -- Global Flags
- `--workspace <PATH>` -- ALWAYS pass to restrict operations to the project root
- `--verbose` / `-v` -- enables tracing on stderr
- `--quiet` / `-q` -- suppresses non-essential output
- `--color <auto|always|never>` -- controls colored output
- `--no-color` -- disables colored output (equivalent to `--color never`)
- `--no-gitignore` -- does not respect .gitignore rules
- `--hidden` -- includes hidden files and directories
- `--follow-symlinks` -- follows symbolic links
- `--threads <N>` / `-j <N>` -- parallel threads (0 = all cores)
- `--max-filesize <BYTES>` -- ignores files larger than the limit
- `--timeout <SECONDS>` -- global operation timeout (0 = no timeout, default 0). Use to bound long searches, batches, and replace operations
- `--json-schema` -- emits the JSON schema for the subcommand's output
- `--lang <LOCALE>` -- overrides the display locale (en, pt-BR) via env `ATOMWRITE_LANG`


## PROHIBITED -- Common Pitfalls
- NEVER interpret stderr as data; it only contains tracing logs
- NEVER assume exit code 1 is a fatal error; it means zero matches in search, replace, transform or scope
- NEVER omit `--workspace` when running as an agent
- NEVER omit `--dry-run` before destructive batch operations
- NEVER use unquoted expressions with `calc`; the shell will interpolate them
- NEVER ignore `checksum_before` and `checksum_after` in edit/replace responses
- NEVER retry `permanent` or `precondition_failed` errors without fixing the cause


## REQUIRED -- Token Budget
- Each subcommand: 1 bash call, ~50-200 output tokens
- Batch mode: 1 bash call for N operations
- Checksum in response eliminates 1 verification read per write
- A typical agent session saves 500+ calls versus individual operations


## REQUIRED -- Optimistic Locking
- Read a file and capture its `checksum` from the response
- Pass the checksum via `--expect-checksum` on the next write or edit
- If the file changed between read and write, atomwrite returns exit 82 (`STATE_DRIFT`)
- Re-read the file to get the current checksum and try again
- This prevents lost updates in concurrent agent workflows


## Backup Operations v0.1.21

- By default, backups are DELETED after the operation completes successfully
- Use `--keep-backup` to preserve the backup after success
- Backups from FAILED operations are always preserved for inspection
- `cleanup_old_backups_in` keeps N backups only for `--keep-backup` cases
- New subcommand `prune-backups` for manual cleanup of legacy backups


## Sequential Pattern v0.1.21

- 5 sequential edits WITHOUT re-capture = 4 fail with `STATE_DRIFT` (exit 82)
- 5 sequential edits WITH re-capture = all pass (canonical pattern)
- 5 sequential edits WITH `--allow-sequential-drift` = all pass with warning
- `edit-loop` applies N pairs in 1 invocation (no internal STATE_DRIFT)


## Subcommands v0.1.22

Two new subcommands close the rejected fronts from previous plans (`gaps.md` lines 82-83, 201):

### `edit-loop` — N Pairs in 1 Invocation

- **When to use**: applying a batch of textual transformations to one file where today you would invoke `edit` N times in a shell loop
- **Input**: NDJSON via stdin with one `{old, new}` object per line
- **Behavior**: reads the file ONCE, applies all pairs in memory, writes atomically ONCE
- **Advantage over shell loop**: 1 CLI invocation instead of N; 1 read + 1 write instead of N+N; no internal `STATE_DRIFT` between pairs

```bash
# Aplicar 3 pares em 1 invocação
printf '%s\n' \
  '{"old":"v0_1_20","new":"v0_1_22"}' \
  '{"old":"foo","new":"bar"}' \
  '{"old":"baz","new":"qux"}' \
  | atomwrite --workspace . edit-loop src/version.rs

# Com backup preservado (linha do tempo forense)
printf '%s\n' '{"old":"foo","new":"bar"}' \
  | atomwrite --workspace . edit-loop --backup --keep-backup src/foo.rs

# Modo best-effort com --partial
printf '%s\n' '{"old":"existe","new":"X"}' '{"old":"ausente","new":"Y"}' \
  | atomwrite --workspace . edit-loop --partial src/foo.rs
```

- **NDJSON Schema**: `docs/schemas/edit-loop-output.schema.json` -- fields `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`
- **ADR**: `docs/decisions/0039-edit-loop-helper.md`

### `prune-backups` — Manual Cleanup of Legacy Files

- **When to use**: operators who upgraded from v0.1.20 inherit `.bak.YYYYMMDD_HHMMSS` siblings that v0.1.21 no longer creates (and therefore no longer cleans up automatically)
- **Algorithm**: scans `[PATHS]...` for `.bak.YYYYMMDD_HHMMSS` files, applies age or count filters
- **Safety**: `--dry-run` is DEFAULT `true` to prevent accidental data loss

```bash
# Default --dry-run true: lista o que SERIA removido
atomwrite --workspace . prune-backups --max-age-secs 86400 .

# Remove backups mais antigos que 24 horas
atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run false .

# Mantém apenas os 3 backups mais recentes por diretório
atomwrite --workspace . prune-backups --max-count 3 --dry-run false .

# Pipeline CI: afirma zero backups órfãos após limpeza
atomwrite --workspace . prune-backups --max-age-secs 0 --dry-run false . \
  && fd '*.bak.*' . | wc -l | jaq -e '. == 0'
```

- **NDJSON Schema**: `docs/schemas/prune-backups-output.schema.json` -- fields `path`, `reason`, `action`, `total`, `elapsed_ms`
- **ADR**: `docs/decisions/0040-prune-backups-subcommand.md`
- **Operational note**: automatic retroactive cleanup was explicitly rejected (see ADR-0038 Addendum); operators must run `prune-backups` on demand
