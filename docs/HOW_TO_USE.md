# How to Use atomwrite


[Leia em Portugues](HOW_TO_USE.pt-BR.md)

> One CLI replaces dozens of file-manipulation tool calls your agent makes today


## What's New in v0.1.12

## Quickstart: WAL Cleanup (G119)

v0.1.15 ships a three-layer WAL management system (G119). The new `wal-stats` subcommand is read-only telemetry. The new `wal-heal` subcommand is scoped reaping. The new `--wal-policy` flag controls sidecar creation per write. Use the global `--no-auto-heal` to disable automatic healing during sensitive workflows.

### Inspect with `wal-stats`

```bash
atomwrite --workspace . wal-stats
# {"type":"result","journals_total":0,...,"reclaimable":0,...}
```

### Heal with `wal-heal`

```bash
# Remove all terminal journals (skips Started entries)
atomwrite --workspace . wal-heal --threshold-secs 0
```

### Choose a `--wal-policy`

```bash
# Default: let the build decide
atomwrite --workspace . write src/lib.rs < new.rs

# Forbid any sidecar creation (CI hygiene)
atomwrite --workspace . write --wal-policy never ci-output.txt < data.txt
```

### Disable Auto-Heal Globally

When running scripted batch workflows or forensic captures, disable automatic healing so the script controls when reaping happens.

```bash
atomwrite --workspace . --no-auto-heal wal-stats
```

### Policy Decision Table

| Workload | `--wal-policy` | Rationale |
|---|---|---|
| Local dev / interactive use | `auto` (default) | Optimized for general use; balanced trade-off |
| CI builds and ephemeral jobs | `never` | Sidecars have no consumer; skip the overhead |
| Production deploys / audit trails | `always` | Forensic metadata required for postmortem |
| Bulk migrations and batch jobs | `never` + `--no-auto-heal` | Speed and explicit control of reaping |
| Forensic analysis / debugging | `always` + manual `wal-heal` | Keep all sidecars; reap only when you decide |


This section summarizes how-to-use-relevant changes in v0.1.12.

### New Subcommands (Tier 3)

6 new subcommands for structured config and code operations:

- `set <PATH> <KEY_PATH> <VALUE>` -- write a value at a dotted path in TOML/JSON
- `get <PATH> <KEY_PATH>` -- read a value at a dotted path
- `del <PATH> <KEY_PATH>` -- remove a key (`--force-missing` for idempotent scripts)
- `case <PATHS...> --subvert OLD NEW --to <style>` -- rename identifiers in 5 case styles
- `query <PATH> [--kinds|--query <KIND>|--tree] [--positions]` -- walk a tree-sitter AST
- `outline <PATH> [--kind <KIND>] [--positions]` -- extract high-level structure

See the Advanced Commands section below for detailed documentation of each.

### New Flags for Existing Commands

- `write --syntax-check` -- validate with tree-sitter after write (G72, exit 88)
- `write --lock` and `--lock-timeout <ms>` -- advisory file lock via flock (G54, exit 83)
- `write --include-fifo` -- allow writing to named pipes (G56)
- `write --strict-atomic` -- abort on EXDEV instead of copy fallback (G90, exit 91)
- `read --format raw` (alias `--raw`) -- emit raw bytes for Unix composability (G81)
- `read --head N`, `--tail N`, `--line N`, `--grep <REGEX>` -- new read modes
- `search --max-filesize <BYTES>` -- skip files larger than limit (G68, default 10 MiB)
- `search --max-columns <N>` -- truncate matches with >N columns (G68, default 500)
- `replace --literal` (alias `-F`) -- disable regex interpretation (G66)
- `transform --rules <file.yaml>` -- multi-rule YAML for cascading refactors (G44)
- `transform --inline-rules <YAML>` -- inline multi-rule YAML
- `batch --batch-size <N>` -- control peak memory (G77, default 100)
- `backup/copy --no-reflink` -- disable CoW for filesystems without support (G64)

### 5 New Error Codes

- 83 `LockTimeout` (G54)
- 88 `SyntaxError` (G72)
- 91 `ExdevFallbackDisabled` (G90)
- 92 `CopyBackBlake3Failed` (G114)
- 93 `OrphanJournal` (G114)

### G72 REAL Syntax Check

`atomwrite write --syntax-check` invokes the actual tree-sitter parser (24 languages) instead of the bracket-balance heuristic. Exit 88 with first error line/column. The parser is downloaded on first use via `tree-sitter-language-pack`.

### G114 WAL Sidecar for Crash Recovery

`atomic_write` writes `.atomwrite.journal.<target>.atomwrite.journal.json` with `Started`/`Committed` entries. `recover_orphan_journals(dir)` is consultive (no auto-replay, no auto-delete). The agent decides.

### G64 Reflink CoW for Backup/Copy

`backup` and `copy` use `reflink_or_copy` for O(1) backup on APFS/btrfs/XFS. Fallback to `fs::copy` on filesystems without CoW support. Use `--no-reflink` to force copy.

### Test Coverage

- 542 tests passing (445 in v0.1.12 + 2 in v0.1.14 + 8 G117 + 6 G118 in v0.1.15)
- 9 ADRs in `docs/decisions/` (0019-0027)
- 7 new JSON schemas in `docs/schemas/`
- See [docs/decisions/README.md](README.md) for architectural decisions

## Prerequisites
- Rust toolchain 1.88 or later
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
- Since v0.1.15 append/prepend, line-ending auto-detection, and `--expect-checksum` resolve the target against `--workspace` (G118); on v0.1.14 and earlier keep CWD = workspace, or relative targets truncate on append and skip checksum verification
- Use `--dry-run` to preview the operation without writing
- Use `--syntax-check` to validate the file with tree-sitter after writing (G72, exit 88 on error)
- Use `--preserve-timestamps` to keep the original mtime (default: mtime is updated so cargo/make/cmake rebuild)
- Use `--include-fifo` to allow writing to FIFO/named pipes (default: exit 85)
- Use `--strict-atomic` to abort on EXDEV (G90, default: copy fallback for Docker/NFS)
- Use `--lock` to acquire an advisory file lock via `flock` (G54, exit 83 on timeout)
- Use `--no-reflink` to disable CoW backup (G64, default: reflink in APFS/btrfs/XFS)

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
- Use `--head N` to read the first N lines
- Use `--tail N` to read the last N lines
- Use `--line N` to read line N with optional context via `--context N`
- Use `--grep <REGEX>` to filter returned lines to those matching a regex
- Use `--format raw` (or `--raw`) to emit raw bytes for Unix composability (G81, breaks NDJSON envelope)
- Use `--verify-checksum <BLAKE3>` to verify file integrity
- Binary files are detected and content is omitted automatically

### edit
- Surgically edit files by line number, text marker or exact match
- The edit is atomic: tempfile, fsync, rename

```bash
echo "new line" | atomwrite edit src/main.rs --after-line 5
echo "replacement block" | atomwrite edit src/main.rs --range 10:20
atomwrite edit src/main.rs --old "old_text" --new "new_text"
```

- Use `--fuzzy auto|off|aggressive` for fuzzy text matching when exact match fails (9 strategies in cascade, G116)
- Since v0.1.15 repeated `--old`/`--new` pairs also run the fuzzy cascade per pair (G117); responses include `pairs_total` and per-pair `pair_results`, and failures report `failed_pair_index`
- Use `--partial` (v0.1.15) to apply the matching pairs and report the rest; zero applied pairs exits 1 (`NO_MATCHES`) without writing
- Never pipe `edit` into `jaq` without `-e`: the error envelope goes to stdout, so `| jaq '.edits'` masks exit 65 as `null` — use `jaq -e '.edits'` or `${PIPESTATUS[0]}`
- Use `--multi` to apply multiple NDJSON edits in a single atomic write via stdin
- Use `--line-ending lf|crlf|cr|auto` to normalize line endings (default: auto preserves original)
- Use `--preserve-timestamps` to keep the original mtime of the file (default: mtime is updated to reflect the edit)
- Use `--after-line N` to insert content after line N
- Use `--before-line N` to insert content before line N
- Use `--range N:M` to replace a line range
- Use `--delete-range N:M` to delete a line range
- Use `--between START END` to replace content between two marker lines
- Returns checksums before and after for verification
- Returns line counts before and after for auditing
- Returns `mtime_preserved` flag in the NDJSON response
- Returns `fuzzy`, `strategy`, `strategies_tried`, `similarity` when fuzzy matching is used

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
- Use `--max-filesize <BYTES>` to skip files larger than the limit (G68, default 10 MiB)
- Use `--max-columns <N>` to truncate lines longer than N columns (G68, default 500)
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
- Use `--no-reflink` to disable CoW backup (G64, default: reflink in APFS/btrfs/XFS for O(1) copy)
- Use `--output-dir <DIR>` to write backups to a specific directory

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
- Use `--rules <file.yaml>` to apply multiple refactor rules in one pass (G44)
- Use `--inline-rules <YAML>` for inline multi-rule YAML
- Supports all/any/not/inside/has/follows/precedes ast-grep YAML predicates

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
- Use `--batch-size <N>` to control peak memory (G77, default 100, processes in chunks)
- Use `--file <PATH>` to read the manifest from a file instead of stdin


### set
- Write a value at a dotted path in a TOML or JSON file
- Preserves comments, key order and whitespace via `toml_edit`
- Auto-coerces the value to int, float, bool or string
- Returns NDJSON with `old_value`, `new_value`, `format`, `comments_preserved`

```bash
atomwrite set Cargo.toml package.version 0.2.0
atomwrite set package.json scripts.build "tsc -b"
```

- Use `--type int|float|bool|string|array` to force type coercion
- Use `--type null` to set a key to `null` in JSON
- Use `--force-missing` to create intermediate keys
- TOML arrays use `key[N]` notation: `dependencies.serde[0].version = "1.0"`

### get
- Read a value at a dotted path in a TOML or JSON file
- Returns NDJSON with `value`, `found`, `format`

```bash
atomwrite get Cargo.toml package.version
atomwrite get package.json scripts.build
```

- If the key is missing, returns `{"found": false, "value": null}`
- TOML dotted path: `dependencies.serde.features[0]`
- JSON pointer (RFC 6901): `/dependencies/serde/features/0`
- Exit 0 even when key is missing; use `found` field to detect

### del
- Remove a key at a dotted path in a TOML or JSON file
- Returns NDJSON with `removed`, `path_was_array_index`, `old_value`

```bash
atomwrite del Cargo.toml package.metadata.deprecated
atomwrite del package.json scripts.build
```

- Use `--force-missing` to treat missing keys as a no-op success (exit 0 instead of error)
- Removing an array element shifts subsequent indices (TOML) or uses nulls (JSON)
- Cannot remove a key whose parent does not exist; use `--force-missing` for idempotent scripts

### case
- Rename identifiers across multiple files using `heck` for case conversion
- Renames `old_id` → `new_id` and all 5 case variants: `oldId`, `OLD_ID`, `old-id`, `OldId`, `old_id`

```bash
atomwrite case src/ --subvert user_id account_id --to snake
atomwrite case src/ lib/ --subvert user_id account_id --to camel
```

- Styles: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`
- Multi-file: pass multiple paths to rename across an entire module
- Detects identifier boundaries in 5 case styles; pure ASCII only
- Preserves comments, strings and other code structure

### query
- Walk a tree-sitter AST and emit nodes as NDJSON
- 305 languages via `tree-sitter-language-pack` (parsers download on first use)
- Modes: `--kinds` (list all kinds), `--query <KIND>` (filter by kind), `-Q <KIND>` (alias), `--tree` (full tree), `--positions` (line:column)

```bash
atomwrite query src/main.rs --kinds
atomwrite query src/main.rs --query function_item --positions
atomwrite query src/main.rs --tree
```

- `--positions` adds `line` and `column` to each node
- `--query` and `--kinds` are mutually exclusive
- Returns one NDJSON object per node with `kind`, `start_byte`, `end_byte`, `text`
- S-expression query support is deferred to v0.1.13 (see ADR-0021)

### outline
- Extract high-level structure (functions, classes, structs, enums, traits, modules) as NDJSON
- 305 languages via `tree-sitter-language-pack`
- Returns one object per top-level definition with `kind`, `name`, `line`, `column`

```bash
atomwrite outline src/main.rs
atomwrite outline src/lib.rs --kind function_item
atomwrite outline src/main.rs --positions
```

- `--kind` filters to a specific tree-sitter kind (e.g. `function_item`, `struct_item`, `impl_item`)
- `--positions` adds `start_line`, `start_column`, `end_line`, `end_column`
- Returns 28 structural node kinds across all languages
- Faster than `query --kinds` because it skips leaf nodes

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
- All 25 error variants now carry a `suggestion` (the only exception is `BrokenPipe` because SIGPIPE is not actionable)
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
- Prerequisite: Rust 1.88 or later
- Recommended terminal: Windows Terminal or PowerShell 7+ (for UTF-8 output and ANSI escape sequences)
- See [INSTALL.md](INSTALL.md) for the full Windows 10/11 installation guide with troubleshooting


## v0.1.20 — What Is New

This release introduces a new safety layer called **intention guards** and renames the global `--lang` flag to `--locale` to disambiguate from the tree-sitter `--lang` selector used by `scope` and `transform`.

### Intention Guards (5 OPT-IN flags)

- `--require-backup <N>` — refuse the operation when fewer than `N` retained backups exist for the target
- `--confirm` — emit a confirmation prompt listing the planned mutation in NDJSON before executing
- `--auto-rotate <N>` — automatically rotate the backup ring down to `N` entries after a successful write
- `--risk-threshold <LOW|MEDIUM|HIGH>` — block operations whose classified risk meets or exceeds the threshold
- `--locale <en|pt-BR>` — renamed from `--lang` to disambiguate from the tree-sitter `--lang`

### Other Additions

- `count --by-size` — list the largest files in the tree with sizes and line counts
- `read --mode raw|envelope` — select between byte-stream output and structured NDJSON envelope
- `search --no-begin-end` — disable the implicit `^` and `$` anchor decoration in regex output
- `write --preserve-timestamps` — keep the source file mtime when overwriting
- `scope --lang rust` — explicit alias accepted for ergonomic symmetry with `transform --lang`

### Statistics

- 542 tests passing in 47 integration suites, 0 failures
- 11 GAP-2026 closed
- 3 Windows cross-compile targets green
- 19 ADRs in `docs/decisions/` (0019-0037)

### Migration `--lang` to `--locale`

```bash
# Discover all files using --lang
rg -l -- '--lang\b' .

# Bulk replace while preserving other matches
fd -e sh -e md -e toml -e yml -e yaml -e json -x sd -- '--lang\b' '--locale' {}

# Or via ruplacer
ruplacer --subvert --lang --locale
```

## v0.1.21 — What Is New

This release closes 3 GAP-2026 items (012, 013 Problem C, 014 v2) and adds one ADR (0038 backup cumprido deleta). The most visible change is that `--backup` operations now DELETE the backup after success by default; add `--keep-backup` to preserve it. The second visible change is that `edit` and `rollback` now accept `--backup`, closing the API parity hole from v0.1.20. The third change is `--allow-sequential-drift` on `edit` for sequential pipelines.

### Backup Operations

- `write --backup` and `replace --backup` DELETE the backup after success by default
- `edit --backup` and `rollback --backup` are NEW in v0.1.21; the flag is honored on all 4 mutating subcommands
- `--keep-backup` is the OPT-IN flag to preserve the backup after success on `write`, `edit`, `replace`, `rollback`, `apply`, and `batch`
- `apply --keep-backup` and `batch --keep-backup` are NEW in v0.1.21 for parity
- Backups are ALWAYS preserved on the FAILURE path, regardless of `--keep-backup`

#### Examples

```bash
# Backup deleted after success (default v0.1.21 behavior)
echo "new" | atomwrite --workspace . write --backup config.toml

# Backup preserved after success (opt-in to v0.1.20 behavior)
echo "new" | atomwrite --workspace . write --backup --keep-backup config.toml

# Edit with backup (new in v0.1.21)
echo "new" | atomwrite --workspace . edit --backup src/main.rs --old foo --new bar

# Batch with keep-backup (new in v0.1.21)
echo '{"op":"write","target":"config.toml","content":"new","keep_backup":true}' \
  | atomwrite --workspace . batch
```

### Sequential Edit Pattern

- Chaining multiple `edit` calls on the same file without re-capturing `checksum_after` produces `STATE_DRIFT` (exit 82) on every call after the first
- Two valid patterns: re-capture checksum (Pattern A) or pass `--allow-sequential-drift` (Pattern B)
- Default behavior is unchanged: `STATE_DRIFT` still fires on checksum mismatch when the flag is absent

#### Example — Pattern A

```bash
# Initial checksum
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')

# Edit 1 — pass the captured checksum
echo "line 2" | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --append

# Re-capture the post-edit checksum
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')

# Edit 2 — uses the new checksum
printf 'line 1\nline 2\n' | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --append
```

#### Example — Pattern B

```bash
# Edit 1 — initial checksum
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')
echo "line 2" | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --append

# Edit 2 — drift allowed, the pre-state differs from CS
printf 'line 1\nline 2\n' | atomwrite --workspace . edit --allow-sequential-drift src/main.rs --append
```


#### Example Copy-Paste — `while` Loop with Re-capture

```bash
#!/usr/bin/env bash
set -euo pipefail

WORKSPACE="/path/to/workspace"
TARGET="$WORKSPACE/src/foo.rs"

# Canonical pattern: re-capture checksum before each edit
while IFS= read -r change; do
  CS=$(atomwrite --workspace "$WORKSPACE" read "$TARGET" | jaq -r '.checksum')
  echo "$change" | atomwrite --workspace "$WORKSPACE" edit \
    --expect-checksum "$CS" "$TARGET"
done <<INNER
primeira mudança
segunda mudança
terceira mudança
INNER
```


## v0.1.22 — What Is New

This release adds 2 new subcommands to address the legacy backup cleanup and the N-edits-in-1-invocation operator pattern. Both are additive: no flag, schema, or default behavior changed for existing commands.

### `prune-backups` Subcommand

- Manual cleanup of `.bak.YYYYMMDD_HHMMSS` siblings left by v0.1.20-era `--backup` operations
- Flags: `--max-age <SECONDS>` (delete older than N), `--max-count <N>` (keep at most N most-recent), `--dry-run` (default `true` for safety)
- NDJSON output: one line per backup (`path`, `age_secs`, `size_bytes`, `action`) plus a `summary` line
- Exit 0 (scan complete), 1 (NO_MATCHES), 65 (precondition failed)
- Refuses to run without `--max-age` or `--max-count` (VAI-PSIQUE-CHECK)

```bash
# List backups that would be removed (default --dry-run true)
atomwrite --workspace . prune-backups --max-age 86400 .

# Actually remove backups older than 24 hours
atomwrite --workspace . prune-backups --max-age 86400 --dry-run false .

# Keep only the 3 most recent backups per directory
atomwrite --workspace . prune-backups --max-count 3 --dry-run false .
```

### `edit-loop` Subcommand

- Apply N `{old, new}` substitution pairs in 1 invocation via NDJSON on stdin
- Flags: `--workspace`, `--expect-checksum`, `--partial`, `--fuzzy`, `--line-ending`, `--preserve-timestamps`, `--backup`, `--keep-backup`, `--retention`
- NDJSON output: one `pair_result` per input line plus a `summary` line with `pairs_total`, `pairs_matched`, `pairs_unmatched`
- Exit 0 (all matched, or `--partial` with ≥1 matched), 1 (NO_MATCHES), 65 (precondition failed)

```bash
# Apply 2 pairs to one file in 1 invocation
printf '%s
' '{"old":"foo","new":"bar"}' '{"old":"baz","new":"qux"}'   | atomwrite --workspace . edit-loop src/foo.rs

# With backup preserved
printf '%s
' '{"old":"foo","new":"bar"}'   | atomwrite --workspace . edit-loop --backup --keep-backup src/foo.rs

# With --partial (apply matched, report unmatched)
printf '%s
' '{"old":"exists","new":"X"}' '{"old":"absent","new":"Y"}'   | atomwrite --workspace . edit-loop --partial src/foo.rs
```

### Statistics

- 575+ tests passing in 56+ integration suites, 0 failures
- 2 new ADRs: 0039 (edit-loop helper), 0040 (prune-backups subcommand)
- 2 new NDJSON schemas: `edit-loop-output.schema.json`, `prune-backups-output.schema.json`
- 32 subcommands total (up from 30 in v0.1.20)
