# atomwrite -- Agent Integration Contract


[Leia em Portugues](AGENTS.pt-BR.md)


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
atomwrite search 'hello' src/
atomwrite replace 'hello' 'world' src/
atomwrite calc "2 hours + 30 minutes to seconds"
```


## 22 Subcommands
- `read` -- read files with metadata, checksum, optional content
- `write` -- create or overwrite files atomically via stdin
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
{"error":true,"code":"FILE_NOT_FOUND","exit":4,"message":"file not found: src/missing.rs","path":"src/missing.rs","error_class":"permanent","retryable":false,"suggestion":"verify the file path exists"}
```

- See `docs/schemas/` for full JSON Schema definitions of all output types


## REQUIRED -- Exit Codes
- 0: success
- 1: no matches (search/replace found nothing)
- 4: file not found
- 13: permission denied
- 28: disk full
- 30: quota exceeded
- 65: invalid input
- 73: cross-device rename
- 74: I/O error
- 78: config invalid
- 82: state drift (checksum mismatch)
- 85: FIFO detected (named pipe cannot be atomically written)
- 86: device file detected (block or character device)
- 126: workspace jail violated
- 127: symlink blocked
- 128: file immutable
- 130: SIGINT
- 141: SIGPIPE (broken pipe)
- 143: SIGTERM
- 255: internal error


## REQUIRED -- Error Handling
- Errors emit JSON on stdout with `error: true`
- Fields: `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`
- `error_class` values: `permanent`, `transient`, `conflict`, `precondition_failed`
- `retryable` is true for `transient` and `conflict` classes


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
- `WORKSPACE_JAIL` (exit 126) -- do not retry, path is outside workspace
- `SYMLINK_BLOCKED` (exit 127) -- do not retry with symlinks disabled
- `IMMUTABLE_FILE` (exit 128) -- do not retry without removing immutable flag

### Precondition Failed (retryable: false)
- `BINARY_FILE` (exit 65) -- use `--stat` mode to read metadata without content
- `IMMUTABLE_FILE` (exit 128) -- remove immutable flag first
- `WORKSPACE_JAIL` (exit 126) -- adjust `--workspace` boundary


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
