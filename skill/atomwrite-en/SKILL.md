---
name: atomwrite
description: >-
  This skill MUST activate when the LLM needs atomic file writing, reading, editing, searching, replacing, AST refactoring, grammatical scoping, BLAKE3 hashing, transactional batches, Jaro-Winkler fuzzy editing, calculation, regex generation, backup or rollback. Covers all 33 subcommands of the atomwrite Rust CLI (read, write, edit, search, replace, hash, verify, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, batch, backup, rollback, apply, set, get, del, case, query, outline, wal-stats, wal-heal, edit-loop, prune-backups, completions). Output ALWAYS NDJSON. Atomic pipeline tempfile-fsync-rename. Triggers — atomwrite, atomic write, surgical edit, BLAKE3, checksum, optimistic locking, WAL journal, tree-sitter, ast-grep, grammatical scoping
---


# atomwrite


## Core Identity
- stdout ALWAYS emits NDJSON (one JSON object per line)
- stderr is ONLY for logs and tracing
- Every write goes through the atomic pipeline tempfile, fsync, rename
- BLAKE3 checksum present in EVERY write and read response
- ALWAYS pass `--workspace <DIR>` to set the jail root
- All paths resolve relative to the workspace root
- `--json` flag accepted but ignored (output is ALWAYS NDJSON)
- NEVER parse stderr as structured data
- NEVER assume exit 1 is an error (search, replace, transform, scope use exit 1 for zero matches)
- NEVER write files outside the workspace jail
- `--backup` is `true` by default in 9 structs (write, edit, replace, apply, batch, set, del, case, transform)
- USE `--no-backup` to disable backup when performance is priority
- `.atomwrite.toml` config file sets per-project defaults with hierarchy CLI > env > local > XDG > defaults


## Write Operations (write)
- ALWAYS pipe content via stdin
- USE `--backup --retention N` for destructive overwrites
- USE `--no-backup` to disable backup
- USE `--keep-backup` to preserve backup after success
- USE `--expect-checksum <BLAKE3>` for optimistic locking
- USE `--allow-shrink` to permit truncation when `--expect-checksum` is active (shrink-guard blocks reduction greater than 50%)
- USE `--allow-empty-stdin` when stdin is empty (intentional guard)
- USE `--no-checksum-when-empty` to skip `--expect-checksum` when stdin is empty
- USE `--dry-run` before destructive writes
- USE `--append` to append to end
- USE `--prepend` to insert at beginning
- USE `--max-size <BYTES>` to limit stdin size
- USE `--line-ending lf|crlf|cr|auto` to normalize line endings
- USE `--preserve-timestamps` to keep original mtime
- USE `--require-backup` to ABORT if backup not active and target exists
- USE `--auto-rotate` to force backup when target modified in last 24h
- USE `--confirm` for interactive prompt on files larger than 100KB
- USE `--risk-threshold <PERCENT>` for risk telemetry (default 255 = disabled)
- USE `--syntax-check` to validate syntax via tree-sitter before writing (exit 88 on failure)
- USE `--wal-policy auto|always|never` for WAL policy
- FORMULA write `echo "content" | atomwrite --workspace . write target.rs`
- FORMULA optimistic locking `CS=$(atomwrite --workspace . read file | jaq -r '.checksum') && echo "new" | atomwrite --workspace . write --expect-checksum "$CS" file`
- FORMULA append `echo "line" | atomwrite --workspace . write --append file`
- NEVER write without `--workspace`
- NEVER pass content as CLI argument


## Read Operations (read)
- USE `read` for content with metadata (checksum, size, lines, mode)
- USE `--stat` for metadata only (no body)
- USE `--lines 1:50` for line range
- USE `--line N` with `-C/--context N` for single line with context
- USE `--head N` for first N lines
- USE `--tail N` for last N lines
- USE `--format raw` for raw content without JSON envelope
- USE `--grep <REGEX>` to filter lines by regex
- USE `--verify-checksum <BLAKE3>` for integrity verification (exit 81 on mismatch)
- `mode` field indicates variant used (full, head, tail, line, lines, grep, stat)
- FORMULA partial read `atomwrite --workspace . read --head 20 src/main.rs`
- FORMULA metadata `atomwrite --workspace . read --stat src/main.rs`


## Edit Operations (edit)
- USE `--old "text" --new "text"` for exact replacement (repeatable for multiple pairs)
- Multi-pair runs 9-strategy fuzzy cascade per pair including Jaro-Winkler (context_aware_jw)
- Response includes pairs_total and pair_results (index, matched, strategy, similarity, diff_preview, old, new)
- Failed pair aborts entire batch by default (all-or-nothing)
- USE `--partial` to apply matching pairs and report the rest
- USE `--fuzzy auto|off|aggressive` to control approximate matching
- USE `--fuzzy-threshold <FLOAT>` for configurable sensitivity (0.0 to 1.0)
- USE `--after-line N` to insert after line
- USE `--before-line N` to insert before line
- USE `--range N:M` to replace range
- USE `--delete-range N:M` to delete range
- USE `--after-match "text"` to insert after match
- USE `--before-match "text"` to insert before match
- USE `--between "start" "end"` to replace between markers
- USE `--multi` for multiple edits via NDJSON on stdin
- USE `--expect-checksum <BLAKE3>` for optimistic locking
- USE `--allow-sequential-drift` for sequential pipeline without checksum recapture
- USE `--line-ending lf|crlf|cr|auto` to normalize endings
- USE `--preserve-timestamps` to keep mtime
- USE `--backup`, `--no-backup`, `--keep-backup`, `--retention N`
- USE `--old-file <PATH>` and `--new-file <PATH>` for large content (avoids ARG_MAX)
- `--old-file` and `--old` are mutually exclusive (clap emits exit 2)
- Cross-mixing (`--old` + `--new-file`) returns exit 65
- USE `--wal-policy auto|always|never`
- NEVER pipe edit into jaq without verification (use `jaq -e` or `${PIPESTATUS[0]}`)
- FORMULA edit `atomwrite --workspace . edit src/main.rs --old "old" --new "new"`
- FORMULA multi-pair `atomwrite --workspace . edit src/main.rs --old "a" --new "b" --old "c" --new "d" | jaq -e '.pair_results'`
- FORMULA via file `atomwrite --workspace . edit src/main.rs --old-file old.txt --new-file new.txt`
- FORMULA insert after line `echo "new" | atomwrite --workspace . edit src/main.rs --after-line 10`


## Search Operations (search)
- Exit code 1 means zero matches (NOT an error)
- USE `-g/--include '*.rs'` and `--exclude '*.log'` to filter
- USE `-C/--context N` for surrounding lines
- USE `-F/--fixed` for literal search
- USE `-e/--regex` to force regex mode
- USE `-w/--word` for word boundary
- USE `-i/--case-insensitive`, `-S/--smart-case`
- USE `-c/--count` for per-file count
- USE `-l/--files` for file names only
- USE `-m/--max-count N` to limit matches per file
- USE `-U/--multiline` for multiline matching
- USE `-P/--pcre2` for PCRE2 engine (exit 65 if feature not compiled)
- USE `--invert` for non-matching lines
- USE `--sort path|modified|created|none` to sort (path guarantees deterministic global ordering)
- USE `--max-filesize <BYTES>` to skip large files
- USE `--max-columns <N>` to truncate wide lines
- USE `--no-begin-end` to suppress begin/end events on files without matches
- USE `--include-fifo` for FIFO/named pipes (NEVER on untrusted directories)
- FORMULA search `atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs'`
- FORMULA count `atomwrite --workspace . search 'unwrap' src/ --count --sort path`


## Replace Operations (replace)
- Exit code 1 for zero matches
- ALWAYS use `--dry-run` first
- USE `--regex`, `-w/--word`, `-F/--literal`
- USE `-g/--include`, `--exclude` to filter
- USE `--preview` for diff without writing
- USE `-n/--max-replacements N` to limit per file
- USE `--expect-checksum <BLAKE3>` for locking
- USE `--backup`, `--no-backup`, `--keep-backup`
- USE `--preserve-timestamps` to keep mtime
- USE `--preserve-case` to preserve original case (UPPER, lower, Title)
- FORMULA `atomwrite --workspace . replace --dry-run 'old' 'new' src/`
- FORMULA regex `atomwrite --workspace . replace --regex 'v\d+\.\d+' 'v2.0' src/ --include '*.toml'`


## Transform Operations AST (transform)
- Exit code 1 for zero matches
- ALWAYS specify `-l/--lang` for target language
- USE `$NAME` for single node capture and `$$$ARGS` for multiple
- 306 languages supported via ast-grep
- USE `-p/--pattern` and `-r/--rewrite` (BOTH required in single-rule mode)
- USE `--dry-run`, `--backup`, `--no-backup`
- USE `--include`, `--exclude` to filter
- USE `--rules <PATH>` for YAML/JSON rule files
- USE `--inline-rules <JSON>` for inline rules
- USE `--verify-parse` to validate parse tree
- FORMULA `atomwrite --workspace . transform -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`


## Grammatical Scoping Operations (scope)
- Exit code 1 for zero matches
- ALWAYS specify `-l/--language` for target language (accepts `--lang` as alias)
- USE `--query` for prepared queries
- USE `--pattern` for custom AST patterns
- USE `--delete` to remove matched content
- USE `--action upper|lower|titlecase|squeeze|symbols|normalize`
- `--action symbols` converts ASCII operators to Unicode
- `--action normalize` normalizes text to NFC
- USE `--replace-with "text"` for custom replacement
- USE `-g/--include`, `--exclude`, `--backup`, `--dry-run`
- Rust queries: comments, strings, fn, pub-fn, async-fn, unsafe-fn, struct, pub-struct, enum, pub-enum, trait, impl, mod, use, closure, unsafe, attribute, derive, return, match, if-let, while-let, for, loop, const, static, type-alias, macro-rules
- Python queries: comments, strings, class, def, async-def, lambda, import, from-import, with, for, while, decorator, try-except
- JS/TS queries: comments, strings, fn, arrow-fn, async-fn, class, import, export, try-catch, const, let
- Go queries: fn, struct, interface, goroutine, defer, import, const, var
- Known limitations: `test-fn` and `doc-comment` are DISABLED (return InvalidInput)
- FORMULA `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`


## Batch Operations (batch)
- Input is NDJSON on stdin (`op` field required: write, replace, delete, edit, move, copy, hash)
- move/copy require `"force":true` to overwrite
- USE `--file <PATH>` to read manifest from file
- USE `--transaction` for all-or-nothing with automatic rollback
- USE `--dry-run`, `--keep-backup`, `--batch-size <N>`, `--input-schema`
- FORMULA `echo '{"op":"write","target":"a.txt","content":"hello"}' | atomwrite --workspace . batch --transaction`


## Hash and Verify Operations (hash, verify)
- hash computes BLAKE3 checksums for one or more files
- USE `--verify <BLAKE3>` to check against expected hash
- USE `--stdin` to hash stdin content
- USE `--recursive` (`-r`) to hash directories
- Output field is `checksum` (NOT `value`)
- verify accepts `<PATH> <EXPECTED_HASH>` as positional arguments
- verify returns exit 0 on match, exit 81 on mismatch
- FORMULA hash `atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'`
- FORMULA verify `atomwrite --workspace . verify src/main.rs abc123def456`


## Delete Operations (delete)
- USE `--backup --retention N` to keep backups
- USE `--recursive` (`-r`) for directories (traverses via WalkBuilder, removes empty subdirectories)
- USE `--include`, `--exclude` to filter
- USE `--yes` (`-y`) to skip confirmation
- USE `--dry-run` or `--confirm` to preview
- USE `--older-than <DURATION>` to filter by age (s/m/h/d/w)
- FORMULA `atomwrite --workspace . delete --older-than 7d --yes tmp/`


## Diff Operations (diff)
- USE `--unified` for unified format
- USE `--stat` for summary statistics
- USE `-C/--context N` for context lines (default 3)
- USE `--algorithm myers|patience|lcs` (default patience)
- FORMULA `atomwrite --workspace . diff src/old.rs src/new.rs --unified`


## Move and Copy Operations (move, copy)
- USE `--force` to overwrite destination
- USE `--dry-run`, `--backup`
- copy accepts `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr`
- move accepts `--preserve-hardlinks`, `--retention N`
- FORMULA move `atomwrite --workspace . move src/old.rs src/new.rs`
- FORMULA copy `atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/`


## List, Count and Extract Operations (list, count, extract)
- list: `-g/--include`, `--exclude`, `--long`, `--depth N`, `--count-by-ext`, `--all`
- count: `--by-extension`, `--by-size` with `--top N`, `--include`, `--exclude`
- extract: positional fields (path, line_number), `--delimiter <SEP>`, `--stdin`
- FORMULA list `atomwrite --workspace . list --long --depth 2 src/`
- FORMULA count `atomwrite --workspace . count --by-size --top 10 src/`
- FORMULA extract `atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number`


## Calc and Regex Operations (calc, regex)
- calc: expression in quotes, `--stdin` to read from stdin (stateless, no --workspace)
- regex: generates regex from examples (3+ for accuracy)
- regex: `-d/--digits`, `-w/--words`, `-s/--spaces`, `-r/--repetitions`
- regex: `-i/--case-insensitive`, `--no-anchors`, `--stdin`
- FORMULA calc `atomwrite calc "2 hours + 30 minutes to seconds"`
- FORMULA regex `atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" --digits`


## Backup and Rollback Operations (backup, rollback)
- backup: `--retention N` (default 5), `--output-dir <DIR>`, `--dry-run`
- rollback: `--latest` (default), `--timestamp YYYYMMDD_HHMMSS` (accepts prefix match with milliseconds)
- rollback: `--verify` for BLAKE3 checksum after restoration
- rollback: `--backup`, `--keep-backup`, `--retention N`, `--dry-run`
- FORMULA backup `atomwrite --workspace . backup src/main.rs --retention 3`
- FORMULA rollback `atomwrite --workspace . rollback src/config.toml --verify`


## Patch Operations (apply)
- Detects format: unified diff, SEARCH/REPLACE, markdown-fenced, full file
- USE `--format auto|unified|search-replace|full|markdown` to force format
- USE `--backup`, `--no-backup`, `--keep-backup`, `--retention N`, `--dry-run`
- FORMULA `echo "content" | atomwrite --workspace . apply src/file.txt --format full`


## Config Operations (set, get, del)
- set: writes value to TOML or JSON via dotted path (auto-coerce bool/int/float/string)
- get: reads value via dotted path (exit 65 INVALID_INPUT if key does not exist)
- del: removes key via dotted path
- USE `--backup`, `--no-backup`, `--preserve-timestamps` on set and del
- USE `--force-missing` on del to succeed silently if key absent
- NEVER use set/get/del on plain text (TOML and JSON only)
- FORMULA set `atomwrite --workspace . set Cargo.toml package.version 0.2.0`
- FORMULA get `atomwrite --workspace . get config.toml database.pool.max`
- FORMULA del `atomwrite --workspace . del --force-missing config.toml features.experimental`


## Case, Query and Outline Operations (case, query, outline)
- case: converts identifier case (snake, camel, pascal, kebab, screaming-snake)
- case: `--subvert OLD NEW` is REQUIRED (without it returns exit 65)
- case: `--to <STYLE>`, `--backup`, `--no-backup`, `--preserve-timestamps`, `--dry-run`
- NEVER run case without --dry-run on a large codebase
- query: inspects AST via tree-sitter (24 languages)
- query: `--kinds` lists node kinds (response is NDJSON stream, one object per kind)
- query: `--tree` for full tree
- query: `-Q/--query <PATTERN>` for S-expression
- query: `--positions`, `--language <LANG>`
- outline: extracts high-level structure (functions, structs, enums)
- outline: `--kind <KIND>` (repeatable), `--positions`, `--language <LANG>`
- FORMULA case `atomwrite --workspace . case --to kebab --subvert API API --dry-run src/`
- FORMULA query `atomwrite --workspace . query --kinds src/main.rs`
- FORMULA outline `atomwrite --workspace . outline --kind function_item --positions src/main.rs`


## WAL and Maintenance Operations (wal-stats, wal-heal, edit-loop, prune-backups, completions)
- wal-stats: inspects WAL journal state (consultive, does not modify)
- wal-heal: removes terminal orphan journals older than threshold
- wal-heal: `--threshold-secs <N>` (default 3600), `--max-duration-ms <N>` (default 100)
- edit-loop: applies N {old, new} pairs in 1 invocation (accepts JSON array AND NDJSON)
- edit-loop: response includes pair_results with old and new fields for each processed pair
- edit-loop: `--backup`, `--no-backup`, `--keep-backup`, `--retention N`, `--line-ending`, `--syntax-check <LANG>`, `--allow-sequential-drift`
- prune-backups: cleans up legacy backups by age or count
- prune-backups: `--max-age-secs <N>` (NOT --max-age), `--max-count <N>`, `--dry-run`
- completions: generates completions for bash, zsh, fish, elvish, powershell
- completions: `--install` to install automatically
- FORMULA edit-loop `echo '[{"old":"foo","new":"bar"},{"old":"baz","new":"qux"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- FORMULA prune `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run /path/`
- FORMULA completions `atomwrite completions bash --install`


## Error Handling
- CHECK exit code BEFORE parsing stdout
- PARSE stdout JSON when `error: true` for structured details
- Envelope fields: error, code, exit, message, path, error_class, retryable, suggestion, workspace
- Strategy by error_class: permanent (NEVER retry), transient (exponential backoff), conflict (re-read state first), precondition_failed (fix precondition)
- RETRY ONLY when `retryable: true`
- USE `suggestion` field for actionable remediation (context-aware)
- NEVER ignore non-zero exit codes (except exit 1 in search/replace/transform/scope)
- NEVER parse stderr for error data
- NEVER retry when `retryable: false`


## Exit Codes
- 0 success
- 1 no matches (search, replace, transform, scope = zero matches, NOT an error)
- 4 not found (file or directory)
- 13 permission denied
- 28 disk full
- 30 quota exceeded
- 65 invalid input (malformed arguments, empty pattern, missing key in get/del)
- 73 cross-device (move across filesystems)
- 74 I/O error
- 78 config invalid
- 81 checksum verify failed (BLAKE3 mismatch in read --verify-checksum or verify)
- 82 state drift (checksum mismatch in optimistic locking)
- 83 lock timeout
- 85 FIFO detected
- 86 device file detected
- 88 syntax error (tree-sitter)
- 91 EXDEV fallback disabled
- 92 copy-back BLAKE3 verification failed
- 93 orphan journal detected (consultive)
- 126 workspace jail violation
- 127 symlink blocked (target outside workspace)
- 128 immutable file
- 130 SIGINT
- 141 SIGPIPE
- 143 SIGTERM
- 255 internal error


## Global Flags
- `--workspace <DIR>` jail root (REQUIRED for file operations)
- `--max-filesize <BYTES>` max accepted size (default 1 GiB)
- `-j/--threads <N>` parallel threads (0 = all cores)
- `--timeout-secs <SECONDS>` global timeout (default 0 = no timeout)
- `--color auto|always|never`
- `--no-color` disable colored output
- `--no-gitignore` ignore .gitignore
- `--hidden` include hidden files
- `--follow-symlinks` follow symbolic links
- `--locale <LANG>` override message locale (pt-BR, en)
- `--json-schema` emit JSON Schema for subcommand output and exit
- `--no-auto-heal` skip automatic WAL heal on startup
- `-v` info, `-vv` debug, `-vvv` trace
- `-q` error, `-qq` off


## Ready-Made Pipeline Formulas
- Optimistic locking: `CS=$(atomwrite --workspace . read file | jaq -r '.checksum') && echo "new" | atomwrite --workspace . write --expect-checksum "$CS" file`
- Search and extract: `atomwrite --workspace . search 'TODO' src/ --include '*.rs' | atomwrite extract path line_number`
- Hash audit: `atomwrite --workspace . hash src/main.rs src/lib.rs | jaq -r '.checksum'`
- Transactional batch: `atomwrite --workspace . batch --file ops.ndjson --transaction`
- TOML config: `atomwrite --workspace . get config.toml db.pool.max && atomwrite --workspace . set config.toml db.pool.max 20`
- Pre-commit check: `atomwrite --workspace . write --syntax-check src/lib.rs < new.rs`
- Sequential edits: `echo '[{"old":"a","new":"b"},{"old":"c","new":"d"}]' | atomwrite --workspace . edit-loop --backup src/foo.rs`
- AST refactor: `atomwrite --workspace . transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- Bulk replace: `atomwrite --workspace . replace --dry-run 'old' 'new' src/`
- Backup and rollback: `atomwrite --workspace . backup src/config.toml && atomwrite --workspace . rollback src/config.toml --verify`
- Integrity verify: `atomwrite --workspace . verify src/main.rs $(atomwrite --workspace . hash src/main.rs | jaq -r '.checksum')`
- Grammatical scope: `atomwrite --workspace . scope src/ --lang rust --query pub-fn --dry-run`
