---
name: atomwrite
description: This skill MUST activate when the LLM needs atomic file write read edit search replace AST transform scope BLAKE3 hash verify delete count diff move copy list extract calc regex batch backup rollback apply set get del case query outline wal-stats wal-heal edit-loop prune-backups completions recipe sparse semantic-merge agent-surface watch codemod semantic-search or stat. Covers all 41 subcommands with fuzzy replace best_candidate SIGTERM cancel sparse recipe progress durability hardlink backup atomic rename anti-MCP agent-surface offline Jaccard semantic-search multi-rule codemod. Output ALWAYS NDJSON via tempfile-fsync-rename. Triggers — atomwrite atomic write surgical edit checksum optimistic locking tree-sitter ast-grep grammatical scoping fuzzy durability sparse recipe codemod semantic-search agent-surface
---


# atomwrite


## Identity
- atomwrite is a self-contained Rust CLI for atomic file operations used by LLM agents
- stdout ALWAYS emits NDJSON (one JSON object per line)
- stderr is ONLY for logs and tracing
- EVERY write uses the atomic pipeline tempfile then fsync then rename
- BLAKE3 checksum is present in EVERY successful write and read response
- ALWAYS pass `--workspace <DIR>` for file operations
- ALL paths resolve relative to the workspace jail root
- NEVER write outside the workspace jail
- NEVER parse stderr as structured data
- NEVER invent flags that are absent from `atomwrite <cmd> --help`
- `--json` is accepted but ignored because output is ALWAYS NDJSON
- Exit code 1 on search, replace, transform, scope, and semantic-search means ZERO matches and is NOT a system error
- Backup is ON by default for mutating commands via `--backup` / `--no-backup` / `--keep-backup` / `--retention <N>`
- `--backup` and `--no-backup` are mutually exclusive and conflict yields exit 2
- Default backup is transactional and writes `<file>.bak.<timestamp>` adjacent to the target
- Most mutators AUTO-REMOVE the transactional `.bak` on success and KEEP it only on failure for rollback
- EXCEPTIONS that KEEP `.bak` on success are `delete` and `replace`
- `rollback` backup is OPT-IN only via explicit `--backup`
- `move` and `copy` require `--force` OR explicit `--backup` OR env `ATOMWRITE_BACKUP` to overwrite an existing destination
- Project TOML `[defaults] backup = true` alone does NOT authorize overwrite for move or copy
- Backup MUST use hardlink on the same filesystem and MUST fall back to reflink or copy when hardlink is impossible
- When NDJSON includes `platform.durability` or `platform.rename_method` after writes, ALWAYS read those fields
- When NDJSON includes backup method hints after backup creation, ALWAYS read those fields
- SIGTERM during a long write cancels cleanly, removes tempfiles, and returns exit 143
- On match failures with exit 65 ALWAYS read structured `best_candidate` from the error envelope when present
- ALWAYS check exit code BEFORE parsing stdout
- ALWAYS choose hierarchy precision over blunt overwrite when changing code


## Edit Hierarchy
- REQUIRED hierarchy for LLM edits from precise to blunt is search then transform then scope then replace then edit then write
- ALWAYS locate targets with `search` before mutating
- ALWAYS use `transform` for structural AST rewrites when language is known
- ALWAYS use `scope` for grammatical bulk actions such as comments or identifiers
- USE `replace` for multi-file text changes with dry-run first
- USE `edit` for surgical single-file pair or line edits
- USE `write` only for full-file create or intentional full overwrite
- NEVER jump to `write` when a surgical command can apply the same change
- ALWAYS dry-run destructive mutators when the command supports `--dry-run`


## Global Flags
- ALWAYS set `--workspace <DIR>` for path-jail file operations
- USE `--max-filesize <BYTES>` to reject oversized files (default 1 GiB globally)
- USE `-j/--threads <N>` for parallelism (0 means all cores)
- USE `--timeout-secs <SECONDS>` for global wall-clock timeout (default 0 means no timeout)
- USE `--color auto|always|never` or `--no-color`
- USE `--no-gitignore` to ignore ignore-files
- USE `--hidden` to include hidden paths
- USE `--follow-symlinks` only when symlink targets stay inside the jail
- USE `--locale <LANG>` for message locale (`en` or `pt-BR`)
- USE `--json-schema` to emit the subcommand schema and exit
- USE `--no-auto-heal` to skip startup WAL heal
- USE `-v` info, `-vv` debug, `-vvv` trace
- USE `-q` error, `-qq` off
- NEVER pass global `--lang` for locale because locale uses `--locale`


## write
- ALWAYS send content via stdin and NEVER as a CLI argument
- ALWAYS use `--workspace`
- USE `--durability full|fast|auto` (default `auto`)
- `full` uses strongest fsync and directory fsync
- `fast` uses file `sync_data` only
- `auto` uses full for config-like targets and fast for source
- USE `--expect-checksum <BLAKE3>` for optimistic locking
- USE `--allow-shrink` when a locked write shrinks the file by more than 50 percent
- USE `--allow-empty-stdin` when empty stdin is intentional
- USE `--no-checksum-when-empty` to skip expect-checksum on empty stdin
- USE `--append` or `--prepend` for partial extension without full rewrite
- USE `--max-size <BYTES>` to cap stdin
- USE `--line-ending lf|crlf|cr|auto`
- USE `--preserve-timestamps` to keep original mtime
- USE `--require-backup` to abort if backup is off and the target exists
- USE `--auto-rotate` to force backup when the target changed within 24 hours
- USE `--confirm` for interactive confirmation on files larger than 100KB
- USE `--risk-threshold <PERCENT>` for size-delta risk warning (default 255 means off)
- USE `--syntax-check` to abort on tree-sitter parse failure with exit 88
- USE `--wal-policy auto|always|never`
- USE `--dry-run` before destructive overwrites
- USE default transactional backup unless performance forces `--no-backup`
- USE `--keep-backup` only when the adjacent `.bak` must survive success
- On SIGTERM the write path cleans the tempfile and exits 143
- FORMULA write `echo "content" | atomwrite --workspace . write target.rs --durability auto`
- FORMULA optimistic lock `CS=$(atomwrite --workspace . read file | jaq -r '.checksum') && echo "new" | atomwrite --workspace . write --expect-checksum "$CS" --durability full file`


## read and stat
- USE `read` for content plus metadata including checksum, size, lines, and mode
- USE `stat PATH` as the alias for `read --stat` metadata without body
- USE `--stat` on `read` for metadata only
- USE `--lines 1:50` for inclusive line ranges
- USE `--line N` with `-C/--context N` for a single line plus context
- USE `--head N` and `--tail N` for partial reads
- USE `--format raw` for raw content without NDJSON envelope
- USE `--grep <REGEX>` to filter returned lines
- USE `--verify-checksum <BLAKE3>` and treat exit 81 as mismatch
- Field `mode` reports full, head, tail, line, lines, grep, or stat
- FORMULA metadata `atomwrite --workspace . stat src/main.rs`
- FORMULA partial `atomwrite --workspace . read --head 20 src/main.rs`


## edit
- USE `--old` and `--new` for exact pair replacement (repeatable multi-pair)
- Multi-pair runs a multi-strategy fuzzy cascade including Jaro-Winkler when fuzzy is enabled
- Response includes `pairs_total` and `pair_results` with strategy and similarity
- Failed pair aborts the whole batch by default
- USE `--partial` to apply matching pairs and report failures
- USE `--fuzzy auto|off|aggressive` and `--fuzzy-threshold <FLOAT>`
- On match failure READ `best_candidate` from the exit 65 error envelope before re-reading files
- USE `--after-line N`, `--before-line N`, `--range N:M`, `--delete-range N:M`
- USE `--after-match`, `--before-match`, and `--between start end` with stdin content
- Stdin modes reject `--old`/`--new`/`--old-file`/`--new-file` at parse time with exit 2
- Terminal stdin on stdin modes fails fast with exit 65 instead of hanging
- USE `--old-file` and `--new-file` for large fragments to avoid ARG_MAX
- Cross-mixing `--old` with `--new-file` returns exit 65
- USE `--expect-checksum`, `--allow-sequential-drift`, `--line-ending`, `--preserve-timestamps`
- USE backup flags and `--dry-run` and `--wal-policy`
- FORMULA pair `atomwrite --workspace . edit src/main.rs --old "old" --new "new" --fuzzy auto`
- FORMULA insert `echo "new line" | atomwrite --workspace . edit src/main.rs --after-line 10`


## search
- Exit 1 means zero matches and is NOT a system error
- USE `-g/--include` and `--exclude` for globs
- USE `-C/--context N`, `-F/--fixed`, `-e/--regex`, `-w/--word`
- USE `-i/--case-insensitive` and `-S/--smart-case`
- USE `-c/--count`, `-l/--files`, `-m/--max-count N`, `-U/--multiline`
- USE `-P/--pcre2` only when the feature is compiled or exit 65
- USE `--invert`, `--sort path|modified|created|none`, `--max-columns <N>`
- Search `--max-filesize` defaults to 10 MiB which differs from the global 1 GiB default
- ALWAYS raise search max-filesize when scanning large sources
- USE `--no-begin-end` to suppress empty begin/end events
- USE `--include-fifo` only on trusted paths
- FORMULA `atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs' --sort path`


## replace
- Exit 1 means zero matches and is NOT a system error
- ALWAYS run `--dry-run` or `--preview` before applying
- USE `--regex`, `-w/--word`, `-F/--literal`
- USE `-g/--include` and `--exclude`
- USE `-n/--max-replacements N` and `--expect-checksum`
- USE `--preserve-case` and `--preserve-timestamps`
- USE `--fuzzy auto|off|aggressive` (default `auto`) when not using `--regex`
- Fuzzy is ignored when `--regex` is set
- USE `--fuzzy-threshold <FLOAT>` to override similarity
- USE `--progress-every <N>` to emit progress NDJSON every N files (default 50, 0 means off)
- On match failure READ `best_candidate` from the error envelope
- `replace` PRESERVES `.bak` on success
- FORMULA dry-run `atomwrite --workspace . replace --dry-run --fuzzy auto 'oldApi' 'newApi' src/`
- FORMULA progress `atomwrite --workspace . replace --fuzzy aggressive --progress-every 10 'old' 'new' src/`


## transform
- Exit 1 means zero matches and is NOT a system error
- ALWAYS set `-l/--language` in single-rule mode
- ALWAYS provide both `-p/--pattern` and `-r/--rewrite` in single-rule mode
- USE `$NAME` for one node and `$$$ARGS` for many nodes
- USE `--rules <PATH>` or `--inline-rules <JSON>` for multi-rule campaigns
- USE `--verify-parse`, `--include`, `--exclude`, `--dry-run`, and backup flags
- ALWAYS use transform over replace for syntactic renames
- FORMULA `atomwrite --workspace . transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`


## scope
- Exit 1 means zero matches and is NOT a system error
- ALWAYS set `-l/--language` (accepts `--lang` as alias)
- USE `--query` for prepared categories or `--pattern` for custom AST shapes
- USE `--delete` to remove matches
- USE `--action upper|lower|titlecase|squeeze|symbols|normalize`
- USE `--replace-with "text"` for custom substitution
- USE include, exclude, dry-run, and backup flags
- Useful queries include comments, strings, fn, pub-fn, struct, enum, trait, impl, import, class, def
- FORMULA `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`


## batch
- Input is NDJSON on stdin with required field `op`
- Supported ops include write, replace, delete, edit, move, copy, and hash
- move and copy require `"force":true` to overwrite
- USE `--file <PATH>` for a manifest file
- USE `--transaction` for all-or-nothing rollback
- USE `--dry-run`, `--batch-size <N>`, and `--input-schema`
- USE backup and retention flags to govern the whole batch including transactional pre-backup
- FORMULA `echo '{"op":"write","target":"a.txt","content":"hello"}' | atomwrite --workspace . batch --transaction`


## sparse recipe semantic-merge agent-surface watch codemod semantic-search
- USE `sparse list` for budgeted monorepo path discovery
- USE `sparse list --max-files N --max-bytes N` with optional include and exclude
- USE `sparse read --paths-file FILE --head N --max-files N` to read only listed paths
- USE `recipe list` to discover built-in recipes
- USE `recipe run --name NAME` with recipe-specific flags
- For search-replace-verify ALWAYS pass `--pattern` and `--replacement`
- Recipe run accepts `--path`, `--dry-run`, `--fuzzy`, `--fuzzy-threshold`, `--include`, `--exclude`
- Edit-loop recipe accepts `--pairs-file`, `--target`, and `--syntax-check <LANG>`
- USE `semantic-merge --base --ours --theirs --output` for multi-agent three-way merge
- USE `--fail-on-conflict` for hard fail on conflicts
- USE `--write-conflict-markers` when markers must be written
- USE `--expect-checksum` on merge output when locking is required
- USE `agent-surface --format json` for the CLI tool manifesto
- MCP integration is FORBIDDEN for atomwrite workflows
- ALWAYS use subprocess CLI plus NDJSON and NEVER use MCP servers for atomwrite
- USE `watch [PATH] --debounce-ms N --max-events N --checksum --gitignore true|false`
- USE `watch` only when the binary was built with the `watch` feature — otherwise the command is unavailable
- USE `codemod --rules rules.yaml [PATHS...]` with `--dry-run` first
- USE `semantic-search "query" [PATHS...] --k N --min-score F --index-dir DIR`
- Semantic-search is offline token Jaccard ranking and NEVER a remote embedding API
- FORMULA sparse list `atomwrite --workspace . sparse list . --max-files 100 --max-bytes 1048576`
- FORMULA sparse read `atomwrite --workspace . sparse read --paths-file paths.txt --head 50 --max-files 20`
- FORMULA recipe `atomwrite --workspace . recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- FORMULA merge `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict`
- FORMULA agent `atomwrite --workspace . agent-surface --format json`
- FORMULA watch `atomwrite --workspace . watch src --debounce-ms 200 --max-events 100 --checksum`
- FORMULA codemod `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- FORMULA semantic-search `atomwrite --workspace . semantic-search "optimistic lock checksum" src --k 20 --min-score 0.05`


## hash and verify
- USE `hash` to compute BLAKE3 for one or more files
- USE `--verify <BLAKE3>` on hash when checking a single expected value
- USE `--stdin` to hash stdin content
- USE `-r/--recursive` to hash directories
- Output field is always `checksum` and NEVER `value`
- USE `verify <PATH> <EXPECTED_HASH>` for dedicated verification
- Verify returns exit 0 on match and exit 81 on mismatch
- FORMULA hash `atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'`
- FORMULA verify `atomwrite --workspace . verify src/main.rs abc123def456`


## delete
- Backup is ON by default and the deletion `.bak` is PRESERVED on success
- USE `--no-backup` only when no recovery is required
- USE `--retention N` to limit retained backups
- `--keep-backup` is redundant on delete and emits `warnings` when used
- USE `-r/--recursive` for directories
- USE `--include`, `--exclude`, `-y/--yes`, `--dry-run`, `--confirm`
- USE `--older-than <DURATION>` with units s m h d w
- FORMULA `atomwrite --workspace . delete --older-than 7d --yes tmp/`


## diff
- USE `--unified` for unified format
- USE `--stat` for summary statistics
- USE `-C/--context N` for context lines (default 3)
- USE `--algorithm myers|patience|lcs` (default patience)
- FORMULA `atomwrite --workspace . diff src/old.rs src/new.rs --unified`


## move and copy
- Overwrite of an existing destination REQUIRES `--force` OR explicit `--backup` OR env `ATOMWRITE_BACKUP`
- TOML default backup alone NEVER authorizes overwrite
- USE `--dry-run` before destructive move or copy
- move accepts `--preserve-hardlinks` and retention flags
- copy accepts `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr`
- FORMULA move `atomwrite --workspace . move src/old.rs src/new.rs`
- FORMULA copy `atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/`


## list count extract
- USE `list` with `-g/--include`, `--exclude`, `--long`, `--depth N`, `--count-by-ext`, `--all`
- USE `count` with `--by-extension`, `--by-size`, `--top N`, include, and exclude
- USE `extract` with positional fields such as path and line_number
- extract accepts `--delimiter <SEP>` and `--stdin`
- FORMULA list `atomwrite --workspace . list --long --depth 2 src/`
- FORMULA count `atomwrite --workspace . count --by-size --top 10 src/`
- FORMULA extract `atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number`


## calc and regex
- `calc` evaluates math and unit conversions and does not require workspace
- USE `calc --stdin` when the expression is piped
- `regex` generates a pattern from examples and needs three or more samples for accuracy
- USE regex flags `-d/--digits`, `-w/--words`, `-s/--spaces`, `-r/--repetitions`
- USE `-i/--case-insensitive`, `--no-anchors`, and `--stdin` when needed
- FORMULA calc `atomwrite calc "2 hours + 30 minutes to seconds"`
- FORMULA regex `atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" --digits`


## backup and rollback
- USE `backup` to create timestamped adjacent backups with BLAKE3
- Backup MUST use hardlink on the same filesystem and MUST fall back otherwise
- USE `--retention N` (default 5) and optional `--output-dir <DIR>`
- USE `--dry-run` to preview backup creation
- USE `rollback --latest` (default) or `--timestamp YYYYMMDD_HHMMSS` with prefix match
- ALWAYS use `--verify` after rollback to confirm BLAKE3 integrity
- rollback pre-snapshot backup is OPT-IN via explicit `--backup`
- FORMULA backup `atomwrite --workspace . backup src/main.rs --retention 3`
- FORMULA rollback `atomwrite --workspace . rollback src/config.toml --latest --verify`


## apply
- Applies patches from stdin and auto-detects unified, SEARCH/REPLACE, markdown-fenced, or full file
- USE `--format auto|unified|search-replace|full|markdown` to force a format
- USE dry-run and backup flags before applying untrusted patches
- FORMULA `echo "content" | atomwrite --workspace . apply src/file.txt --format full`


## set get del
- Operate only on structured TOML or JSON via dotted paths
- NEVER use set get del on plain text
- set auto-coerces bool int float and string values
- get returns exit 65 when the key is missing
- del accepts `--force-missing` to succeed when the key is absent
- USE backup and `--preserve-timestamps` on set and del
- FORMULA set `atomwrite --workspace . set Cargo.toml package.name demo-crate`
- FORMULA get `atomwrite --workspace . get config.toml database.pool.max`
- FORMULA del `atomwrite --workspace . del --force-missing config.toml features.experimental`


## case query outline
- case converts identifier case with styles snake camel pascal kebab screaming-snake
- case REQUIRES `--subvert OLD NEW` or exits 65
- USE case `--to <STYLE>`, dry-run, and backup flags
- NEVER run case without dry-run on large trees
- query inspects tree-sitter AST with `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- outline extracts high-level structure with `--kind <KIND>` and optional positions and language
- FORMULA case `atomwrite --workspace . case --to kebab --subvert API API --dry-run src/`
- FORMULA query `atomwrite --workspace . query --kinds src/main.rs`
- FORMULA outline `atomwrite --workspace . outline --kind function_item --positions src/main.rs`


## wal edit-loop prune-backups completions
- USE `wal-stats` to inspect journal counts, ages, and directory breakdown without mutation
- USE `wal-heal --threshold-secs N --max-duration-ms N` to remove stale terminal journals
- ALWAYS dry-run heal before removal when dry-run is available
- USE `edit-loop` to apply many old/new pairs from JSON array or NDJSON stdin in one write
- edit-loop accepts backup flags, `--line-ending`, `--syntax-check <LANG>`, and `--allow-sequential-drift`
- USE `prune-backups` with `--max-age-secs N` and/or `--max-count N` plus dry-run first
- USE `completions <shell> --install` for bash zsh fish elvish or powershell
- FORMULA edit-loop `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- FORMULA prune `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .`
- FORMULA completions `atomwrite completions bash --install`


## Errors
- ALWAYS check exit code BEFORE parsing stdout
- When `error` is true parse the NDJSON envelope for code, message, error_class, retryable, suggestion, path, and workspace
- Permanent error_class means NEVER retry
- Transient error_class means exponential backoff retry
- Conflict error_class means re-read state then retry with fresh checksum
- Precondition_failed means fix the precondition first
- RETRY ONLY when `retryable` is true
- ALWAYS use `suggestion` when present for remediation
- On match failures READ `best_candidate` instead of guessing nearby text
- NEVER ignore non-zero exits except intentional exit 1 zero-match on search replace transform scope semantic-search
- NEVER parse stderr for structured errors


## Exit codes
- 0 success
- 1 zero matches for search replace transform scope semantic-search and NOT a system error
- 2 invalid argument or flag conflict
- 4 not found
- 13 permission denied
- 28 disk full
- 30 quota exceeded
- 65 invalid input or match failure that includes `best_candidate` when available
- 73 cross-device move
- 74 I/O error
- 78 invalid configuration
- 81 checksum verification failed
- 82 state drift on optimistic locking
- 83 lock timeout
- 85 FIFO detected
- 86 device file detected
- 88 tree-sitter syntax error
- 91 EXDEV fallback disabled
- 92 copy-back BLAKE3 verification failed
- 93 orphan journal detected advisory
- 126 workspace jail violation
- 127 symlink target outside workspace
- 128 immutable file
- 130 SIGINT
- 141 SIGPIPE
- 143 SIGTERM cancel with tempfile cleanup
- 255 internal error


## Ready formulas
- Optimistic lock `CS=$(atomwrite --workspace . read file | jaq -r '.checksum') && echo "new" | atomwrite --workspace . write --expect-checksum "$CS" --durability full file`
- Fuzzy replace `atomwrite --workspace . replace --dry-run --fuzzy auto --fuzzy-threshold 0.85 'oldApi' 'newApi' src/`
- Progress replace `atomwrite --workspace . replace --fuzzy aggressive --progress-every 25 'old' 'new' src/`
- Recipe run `atomwrite --workspace . recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- Sparse list `atomwrite --workspace . sparse list . --max-files 50 --include '*.rs'`
- Sparse read `atomwrite --workspace . sparse read --paths-file paths.txt --head 40 --max-files 15`
- Semantic merge `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output out.rs --fail-on-conflict --write-conflict-markers`
- Semantic search `atomwrite --workspace . semantic-search "BLAKE3 optimistic locking" . --k 20 --min-score 0.05 --index-dir .atomwrite/semantic-index`
- Codemod dry-run `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- Durability write `echo "body" | atomwrite --workspace . write --durability full config.toml`
- Agent surface `atomwrite --workspace . agent-surface --format json`
- Transactional batch `atomwrite --workspace . batch --file ops.ndjson --transaction`
- AST transform `atomwrite --workspace . transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- Rollback verify `atomwrite --workspace . rollback src/config.toml --latest --verify`
- Search extract `atomwrite --workspace . search 'TODO' src/ --include '*.rs' | atomwrite extract path line_number`
- Hash audit `atomwrite --workspace . hash src/main.rs src/lib.rs | jaq -r '.checksum'`
- Edit multi-pair `atomwrite --workspace . edit src/main.rs --old "a" --new "b" --old "c" --new "d" --fuzzy auto`
- Scope delete comments `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`
