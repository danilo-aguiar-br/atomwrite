---
name: atomwrite
description: >-
  This skill MUST activate when the LLM needs atomic file write, read, edit, search, replace, AST transform, grammatical scope, BLAKE3 hash, verify, delete, count, diff, move, copy, list, extract, calc, regex, batch, backup, rollback, apply, set, get, del, case, query, outline, wal-stats, wal-heal, edit-loop, prune-backups, completions, recipe, sparse, semantic-merge, agent-surface, watch, codemod, semantic-search or stat. Covers all 41 subcommands with mandatory fuzzy auto|aggressive, match_count, indent_adjusted, replace-all, best_candidate, SIGTERM 143, durability, reflink_or_copy, bak-skipping recipe hash, real AST sparse outline, line-based semantic-merge, anti-MCP agent-surface and offline Jaccard semantic-search. Output ALWAYS NDJSON via tempfile-fsync-rename. Triggers — atomwrite, atomic write, surgical edit, checksum, optimistic locking, tree-sitter, ast-grep, fuzzy replace, sparse, recipe, codemod, semantic-search
---


# atomwrite


## Operational contract
- atomwrite is the ONLY allowed file write and edit tool for this project
- stdout ALWAYS emits NDJSON (one JSON object per line)
- stderr is ONLY for logs and tracing
- EVERY write uses the atomic pipeline tempfile then fsync then rename
- BLAKE3 checksum is present in EVERY successful write and read response
- ALWAYS pass `--workspace <DIR>` for file operations
- ALL paths resolve relative to the workspace jail
- `--json` is accepted and IGNORED because output is ALWAYS NDJSON
- ALWAYS parse NDJSON from stdout with `jaq` (NEVER `jq`)
- NEVER parse stderr as structured data
- NEVER write outside the workspace jail
- NEVER invent flags absent from `atomwrite <cmd> --help`
- Exit code 1 on search, replace, transform, scope, and semantic-search means ZERO matches and is NOT a system error
- ALWAYS check the exit code BEFORE parsing stdout
- On match failure exit 65 ALWAYS read `best_candidate` from the envelope
- On exit 82 ALWAYS reload checksum and reapply (state drift)
- On exit 143 treat as clean SIGTERM cancel (tempfile removed)
- MCP is FORBIDDEN as a write surface — MUST use the atomwrite CLI


## Required execution flow
- MUST locate the target with `search` or `sparse` before mutating
- MUST capture checksum with `read` when concurrency is possible
- MUST choose the most precise level of the edit hierarchy
- MUST run `--dry-run` before destructive bulk mutation
- MUST apply mutations with `--expect-checksum` on concurrent overwrites
- MUST validate the NDJSON envelope (checksum, match_count, indent_adjusted, platform)
- MUST handle failures via `error_class` and `suggestion`
- NEVER skip dry-run for replace/transform/scope/codemod on large trees
- NEVER full-file `write` when `edit`/`replace`/`transform` can apply the change


## Edit hierarchy
- REQUIRED order from precise to blunt — search → transform → scope → replace → edit → write
- MUST use `search` to locate and inspect
- MUST use `transform` for structural AST rewrites when the pattern is syntactic
- MUST use `scope` for grammatical actions (comments, strings, fn, imports)
- MUST use `replace` for multi-file textual substitution
- MUST use `edit` for surgical single-file edits
- MUST use `write` ONLY for create or intentional full overwrite


## Global flags
- `--workspace <DIR>` — jail root (REQUIRED for file ops)
- `--max-filesize <BYTES>` — global cap (default 1 GiB)
- `-j` / `--threads <N>` — parallelism (0 means all cores)
- `--timeout-secs <SECONDS>` — global timeout (0 means none)
- `--color auto|always|never` and `--no-color`
- `--no-gitignore`, `--hidden`, `--follow-symlinks`
- `--locale <LANG>` — message locale (`en`, `pt-BR`)
- `--json-schema` — emit subcommand schema and exit
- `--no-auto-heal` — skip startup wal-heal
- `-v` info, `-vv` debug, `-vvv` trace
- `-q` error, `-qq` off
- NEVER use global `--lang` for locale — use `--locale`


## Backup and platform
- Backup defaults to ON for mutators via `--backup` / `--no-backup` / `--keep-backup` / `--retention <N>`
- `--backup` and `--no-backup` are mutually exclusive (exit 2)
- Default backup is transactional — creates adjacent `.bak.<timestamp>` and AUTO-REMOVES on success
- On failure the `.bak` remains for rollback
- EXCEPTION — `delete` and `replace` KEEP `.bak` on success
- EXCEPTION — `rollback` backup is OPT-IN via explicit `--backup`
- EXCEPTION — `move` and `copy` require `--force` OR explicit `--backup` OR env `ATOMWRITE_BACKUP` to overwrite destination
- TOML `[defaults] backup = true` alone does NOT authorize overwrite for move/copy
- Backup uses `reflink_or_copy` (NEVER hardlink of the live file)
- ALWAYS read `platform.backup_method`, `platform.durability`, and `platform.rename_method` when present
- Backup precedence — env `ATOMWRITE_BACKUP` (exact value `0` disables) > CLI flags > `.atomwrite.toml` `[defaults]` > built-in default


## Project configuration
- `.atomwrite.toml` sets project defaults
- `[defaults]` controls `backup` and `retention`
- `[fuzzy]` controls `mode` and `threshold` for edit, replace, edit-loop, batch, and recipe
- Valid `mode` values are ONLY `auto` or `aggressive`
- NEVER set `mode = "off"` — the CLI rejects with exit 65
- `threshold` MUST be between 0.0 and 1.0
- Explicit CLI flags override TOML
- FORMULA config fuzzy — in `.atomwrite.toml` set `[fuzzy]` with `mode = "auto"` and `threshold = 0.85`


## Mandatory fuzzy
- Fuzzy is MANDATORY on edit and replace (default mode `auto`)
- Allowed values — `--fuzzy auto` and `--fuzzy aggressive`
- NEVER pass `--fuzzy off` — rejected with exit 65
- MUST use `--fuzzy-threshold <FLOAT>` to tune similarity (0.0 to 1.0)
- With `--regex` on replace fuzzy is IGNORED
- On match failure ALWAYS read `best_candidate` from the exit 65 envelope


## write
- MUST send content via stdin (NEVER as a CLI argument)
- MUST use `--workspace` on every write
- MUST use `--expect-checksum <BLAKE3>` for optimistic locking on existing files
- MUST use `--durability full|fast|auto` (default `auto`)
- `full` uses strongest fsync; `fast` uses `sync_data`; `auto` picks full for configs and fast for source
- MUST use `--allow-shrink` when a locked write shrinks the file by more than 50 percent
- MUST use `--allow-empty-stdin` when empty stdin is intentional
- MUST use `--no-checksum-when-empty` to skip checksum with empty stdin
- MUST use `--dry-run` before destructive overwrite
- MUST use `--syntax-check` for tree-sitter validation (exit 88 on failure)
- Flags — `--append`, `--prepend`, `--max-size`, `--line-ending lf|crlf|cr|auto`, `--preserve-timestamps`, `--require-backup`, `--auto-rotate`, `--confirm`, `--risk-threshold`, `--wal-policy auto|always|never`, backup opts
- SIGTERM during write cleans the tempfile and returns exit 143
- FORMULA `echo "content" | atomwrite --workspace . write --durability full target.rs`
- FORMULA lock `CS=$(atomwrite --workspace . read file | jaq -r '.checksum') && echo "new" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto file`


## read and stat
- MUST use `read` for content plus metadata (checksum, size, lines, mode)
- MUST use `stat PATH` as the alias for `read --stat` metadata without body
- Flags — `--lines 1:50`, `--line N`, `-C/--context N`, `--head N`, `--tail N`, `--format raw|ndjson`, `--grep <REGEX>`, `--verify-checksum <BLAKE3>`
- Field `mode` reports full, head, tail, line, lines, grep, or stat
- Exit 81 on `--verify-checksum` mismatch
- FORMULA `atomwrite --workspace . read --head 20 src/main.rs`
- FORMULA `atomwrite --workspace . stat src/main.rs`


## edit
- MUST use `--old "text" --new "text"` for exact pairs (repeatable)
- Multi-pair runs a multi-strategy fuzzy cascade including Jaro-Winkler
- Response includes `pairs_total` and `pair_results` (index, matched, strategy, similarity, diff_preview)
- A failed pair aborts the whole batch by default (all-or-nothing)
- MUST use `--partial` to apply matching pairs and report the rest
- MUST use `--fuzzy auto|aggressive` and `--fuzzy-threshold <FLOAT>`
- Multi-occurrence REQUIRES `--replace-all`; without it uniqueness is mandatory
- Success with `--replace-all` emits `match_count` in NDJSON
- When indent realigns the replacement NDJSON emits `indent_adjusted: true`
- ALWAYS read `match_count` and `indent_adjusted` after a successful edit
- Position flags — `--after-line N`, `--before-line N`, `--range N:M`, `--delete-range N:M`, `--after-match`, `--before-match`, `--between`, `--multi`
- Stdin modes reject `--old`/`--new`/`--old-file`/`--new-file` at parse time (exit 2)
- Terminal stdin on those modes fails with exit 65 (does NOT hang)
- MUST use `--old-file` and `--new-file` for large payloads
- MUST use `--expect-checksum` for concurrent edits
- MUST use `--allow-sequential-drift` only in controlled sequential pipelines
- Extra flags — `--line-ending`, `--preserve-timestamps`, `--dry-run`, `--wal-policy`, backup opts
- NEVER pipe edit into `jaq` without checking `${PIPESTATUS[0]}`
- FORMULA pair `atomwrite --workspace . edit src/main.rs --old "old" --new "new" --fuzzy auto`
- FORMULA multi `atomwrite --workspace . edit src/main.rs --old "a" --new "b" --replace-all | jaq '{match_count, indent_adjusted, checksum}'`
- FORMULA insert `echo "line" | atomwrite --workspace . edit src/main.rs --after-line 10`


## search
- Exit 1 means zero results and is NOT a system error
- MUST use search to locate targets before mutating
- Flags — `-g/--include`, `--exclude`, `-C/--context`, `-F/--fixed`, `-e/--regex`, `-w/--word`, `-i`, `-S/--smart-case`, `-c/--count`, `-l/--files`, `-m/--max-count`, `-U/--multiline`, `-P/--pcre2`, `--invert`, `--sort path|modified|created|none`, `--max-columns`, `--no-begin-end`, `--include-fifo`
- Search `--max-filesize` defaults to 10 MiB (differs from the global 1 GiB)
- FORMULA `atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs' --sort path`


## replace
- Exit 1 means zero matches and is NOT a system error
- MUST ALWAYS run `--dry-run` or `--preview` before applying
- MUST use `--fuzzy auto|aggressive` (default `auto`; ignored with `--regex`)
- MUST use `--fuzzy-threshold <FLOAT>` for similarity
- MUST use `--progress-every <N>` for progress NDJSON every N files (default 50; 0 disables)
- Multi-occurrence fixed-string replace REQUIRES `--replace-all`
- On match failure (exit 65) MUST read `best_candidate`
- Flags — `--regex`, `-w`, `-F`, `-g/--include`, `--exclude`, `-n/--max-replacements`, `--expect-checksum`, `--preserve-timestamps`, `--preserve-case`, backup opts
- `replace` KEEPS `.bak` on success
- FORMULA dry-run `atomwrite --workspace . replace --dry-run --fuzzy auto --progress-every 20 'old' 'new' src/`
- FORMULA aggressive `atomwrite --workspace . replace --fuzzy aggressive --fuzzy-threshold 0.8 --replace-all 'API' 'Api' src/ --include '*.rs'`


## transform
- Exit 1 means zero matches
- MUST set `-l/--language`
- MUST use `$NAME` for one node and `$$$ARGS` for many
- MUST use both `-p/--pattern` and `-r/--rewrite` in single-rule mode
- Flags — `--rules <PATH>`, `--inline-rules <JSON>`, `--verify-parse`, `--include`, `--exclude`, `--dry-run`, backup opts
- MUST prefer transform over replace when the pattern is syntactic
- FORMULA `atomwrite --workspace . transform --dry-run -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`


## scope
- Exit 1 means zero matches
- MUST set language with `--language` (alias `--lang`)
- MUST use `--query` for prepared categories or `--pattern` for custom AST
- MUST use `--delete` to remove matched content
- MUST use `--action upper|lower|titlecase|squeeze|symbols|normalize`
- MUST use `--replace-with "text"` for custom substitution
- Useful queries — comments, strings, fn, pub-fn, async-fn, struct, enum, trait, impl, mod, use, class, def, import, export
- NEVER apply `--delete` without dry-run
- FORMULA `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`


## batch
- Input is NDJSON on stdin with required field `op` — write, replace, delete, edit, move, copy, hash
- move/copy in the manifest require `"force":true` to overwrite
- MUST use `--transaction` for all-or-nothing rollback
- MUST use `--file <PATH>` for an on-disk manifest
- Flags — `--dry-run`, `--batch-size <N>`, `--input-schema`, backup opts and `--retention`
- FORMULA `echo '{"op":"write","target":"a.txt","content":"hello"}' | atomwrite --workspace . batch --transaction`


## sparse recipe semantic-merge agent-surface watch codemod semantic-search
- MUST use `sparse list` with `--max-files` and `--max-bytes` for monorepo budgets
- MUST use `sparse list --include` and `--exclude` to filter
- MUST use `sparse read --paths-file <FILE> --head N --max-files N` for budgeted reads
- MUST use `sparse outline` for real AST outline under budget (emits `outline_item` with real `kind`)
- MUST use `recipe list` to list built-in recipes
- MUST use `recipe run --name search-replace-verify --pattern OLD --replacement NEW --path . --dry-run --fuzzy auto`
- Recipe search/replace/hash EXCLUDES `*.bak.*` by default
- Built-in recipes — `search-replace-verify` and `edit-loop-syntax-check`
- `recipe run` accepts `--fuzzy-threshold`, `--include`, `--exclude`, `--pairs-file`, `--target`, `--syntax-check`
- MUST use `semantic-merge --base A --ours B --theirs C --output OUT` for multi-agent three-way merge
- semantic-merge is line-based (NOT AST and NOT embedding)
- Merge flags — `--fail-on-conflict`, `--write-conflict-markers`, `--expect-checksum`, backup opts
- MUST use `agent-surface --format json` for the CLI tool manifesto
- MUST use `watch [PATH] --debounce-ms N --max-events N --checksum --gitignore true|false`
- MUST cap `watch` with `--max-events` or an external signal (avoids infinite hang)
- `watch` requires the binary `watch` feature
- MUST use `codemod --rules rules.yaml --dry-run` before applying multi-rule campaigns
- MUST use `semantic-search "query" PATH --k 20 --min-score 0.05`
- `semantic-search` is offline token Jaccard — NEVER a remote embedding API
- MUST use `--index-dir .atomwrite/semantic-index` for an optional local inverted index
- Exit 1 on zero semantic-search results is NOT a system error
- FORMULA sparse list `atomwrite --workspace . sparse list src/ --max-files 50 --max-bytes 524288 --include '*.rs'`
- FORMULA sparse read `atomwrite --workspace . sparse read --paths-file paths.txt --head 30 --max-files 10`
- FORMULA sparse outline `atomwrite --workspace . sparse outline src/ --max-files 40 --include '*.rs'`
- FORMULA recipe `atomwrite --workspace . recipe run --name search-replace-verify --pattern old_api --replacement new_api --path src --dry-run --fuzzy auto`
- FORMULA merge `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output out.rs --fail-on-conflict`
- FORMULA agent `atomwrite --workspace . agent-surface --format json`
- FORMULA watch `atomwrite --workspace . watch src/ --debounce-ms 200 --max-events 100 --checksum`
- FORMULA codemod `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- FORMULA semantic-search `atomwrite --workspace . semantic-search "optimistic lock checksum" src/ --k 15 --min-score 0.1 --index-dir .atomwrite/semantic-index`


## hash and verify
- MUST use `hash` for BLAKE3 of one or more files
- Output field is `checksum` (NEVER `value`)
- Hash flags — `--verify <BLAKE3>`, `--stdin`, `-r/--recursive`, `--exclude <GLOB>`
- MUST use `verify <PATH> <EXPECTED_HASH>` for positional checks
- Exit 0 on match; exit 81 on mismatch
- FORMULA `atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'`
- FORMULA recursive `atomwrite --workspace . hash -r src/ --exclude '*.bak.*'`
- FORMULA `atomwrite --workspace . verify src/main.rs abc123def456`


## delete
- Backup is ON by default and the deletion `.bak` is KEPT on success
- MUST use `--no-backup` only when recovery is not required
- MUST use `-y/--yes` to skip confirmation in automation
- Flags — `-r/--recursive`, `--include`, `--exclude`, `--dry-run`, `--confirm`, `--older-than <DURATION>`, `--retention`
- `--keep-backup` on delete is redundant and emits `warnings`
- FORMULA `atomwrite --workspace . delete --older-than 7d --yes tmp/`


## diff move copy list count extract
- diff — `--unified`, `--stat`, `-C/--context N`, `--algorithm myers|patience|lcs`
- move/copy — overwrite requires `--force` OR explicit `--backup` OR env `ATOMWRITE_BACKUP`
- copy — `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr`
- move — `--preserve-hardlinks`, `--retention`
- list — `-g/--include`, `--exclude`, `--long`, `--depth N`, `--count-by-ext`, `--all`
- count — `--by-extension`, `--by-size` with `--top N`
- extract — positional fields (`path`, `line_number`), `--delimiter`, `--stdin`
- FORMULA diff `atomwrite --workspace . diff src/old.rs src/new.rs --unified`
- FORMULA move `atomwrite --workspace . move src/old.rs src/new.rs`
- FORMULA copy `atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/`
- FORMULA list `atomwrite --workspace . list --long --depth 2 src/`
- FORMULA count `atomwrite --workspace . count --by-size --top 10 src/`
- FORMULA extract `atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number`


## calc and regex
- calc evaluates math and unit conversions (stateless; no `--workspace` required)
- MUST use `calc` with a quoted expression or `--stdin`
- regex generates a pattern from examples (provide 3+ samples)
- Regex flags — `-d/--digits`, `-w/--words`, `-s/--spaces`, `-r/--repetitions`, `-i`, `--no-anchors`, `--stdin`
- FORMULA `atomwrite calc "2 hours + 30 minutes to seconds"`
- FORMULA `atomwrite regex "abc-12" "xyz-99" "id-7" --digits`


## backup and rollback
- MUST use `backup` for timestamped BLAKE3 snapshots
- Backup flags — `--retention N`, `--output-dir`, `--dry-run`
- Backup uses `reflink_or_copy` (NEVER hardlink of the live file)
- MUST use `rollback --latest --verify` to restore and validate
- Rollback flags — `--timestamp YYYYMMDD_HHMMSS`, `--latest`, `--verify`, `--dry-run`, OPT-IN `--backup`
- NEVER rollback a critical file without `--verify`
- FORMULA `atomwrite --workspace . backup src/main.rs --retention 3`
- FORMULA `atomwrite --workspace . rollback src/config.toml --latest --verify`


## apply set get del
- apply auto-detects unified, SEARCH/REPLACE, markdown-fenced, or full file
- MUST use `--format auto|unified|search-replace|full|markdown` when auto-detect is ambiguous
- set/get/del operate ONLY on TOML or JSON via dotted path
- NEVER use set/get/del on plain text
- get returns exit 65 when the key is missing
- del accepts `--force-missing` to succeed when the key is absent
- FORMULA apply `echo "content" | atomwrite --workspace . apply src/file.txt --format full`
- FORMULA set `atomwrite --workspace . set Cargo.toml package.name my-crate`
- FORMULA get `atomwrite --workspace . get config.toml database.pool.max`
- FORMULA del `atomwrite --workspace . del --force-missing config.toml features.experimental`


## case query outline
- case converts identifiers — snake, camel, pascal, kebab, screaming-snake
- MUST pass `--subvert OLD NEW` (otherwise exit 65)
- MUST run case with `--dry-run` on large trees
- query inspects tree-sitter AST — `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- outline extracts structure — `--kind` (repeatable), `--positions`, `--language`
- FORMULA case `atomwrite --workspace . case --to kebab --subvert API API --dry-run src/`
- FORMULA query `atomwrite --workspace . query --kinds src/main.rs`
- FORMULA outline `atomwrite --workspace . outline --kind function_item --positions src/main.rs`


## wal edit-loop prune completions
- `wal-stats` inspects journals (advisory; does NOT modify)
- `wal-heal` removes stale terminal journals — `--threshold-secs` (default 3600), `--max-duration-ms` (default 100)
- `edit-loop` applies N `{old,new}` pairs in one write (JSON array or NDJSON)
- edit-loop accepts `--line-ending`, `--syntax-check`, `--allow-sequential-drift`, backup opts and inherits fuzzy from config
- `prune-backups` cleans legacy backups — `--max-age-secs` (NOT `--max-age`), `--max-count`, `--dry-run`
- `completions` generates shell completions — `bash|zsh|fish|elvish|powershell` with `--install`
- FORMULA edit-loop `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- FORMULA prune `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .`
- FORMULA completions `atomwrite completions bash --install`


## Errors
- MUST check exit code BEFORE parsing stdout
- MUST parse the JSON envelope when `error` is true
- Fields — error, code, exit, message, path, error_class, retryable, suggestion, workspace, best_candidate
- Strategy by `error_class` — permanent (NEVER retry), transient (backoff), conflict (re-read state), precondition_failed (fix precondition)
- MUST retry ONLY when `retryable` is true
- MUST use `suggestion` for remediation
- NEVER ignore non-zero exits except documented zero-match cases
- NEVER parse stderr for structured errors


## Exit codes
- 0 success
- 1 zero matches on search/replace/transform/scope/semantic-search (NOT a system error)
- 2 invalid argument / flag conflict
- 4 not found
- 13 permission denied
- 28 disk full
- 30 quota exceeded
- 65 invalid input (empty pattern, fuzzy off, missing key, best_candidate on match failure)
- 73 cross-device
- 74 I/O error
- 78 invalid configuration
- 81 checksum mismatch (verify / read --verify-checksum)
- 82 state drift (optimistic locking)
- 83 lock timeout
- 85 FIFO detected
- 86 device file
- 88 tree-sitter syntax error
- 91 EXDEV fallback disabled
- 92 copy-back BLAKE3 verification failed
- 93 orphan journal (advisory)
- 126 workspace jail violation
- 127 blocked symlink
- 128 immutable file
- 130 SIGINT
- 141 SIGPIPE
- 143 SIGTERM (clean cancel)
- 255 internal error


## Ready formulas
- Optimistic lock — `CS=$(atomwrite --workspace . read file | jaq -r '.checksum') && echo "new" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto file`
- Strong durability — `echo "cfg" | atomwrite --workspace . write --durability full config.toml`
- Edit with match_count — `atomwrite --workspace . edit src/f.rs --old "x" --new "y" --replace-all --fuzzy auto | jaq '{match_count, indent_adjusted, checksum}'`
- Fuzzy replace with progress — `atomwrite --workspace . replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run 'old_api' 'new_api' src/`
- Best candidate — after exit 65 run `jaq '.best_candidate'` on the envelope
- Recipe search-replace-verify — `atomwrite --workspace . recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- Sparse monorepo — `atomwrite --workspace . sparse list . --max-files 100 --max-bytes 1048576 --include '*.rs'`
- Sparse AST outline — `atomwrite --workspace . sparse outline src/ --max-files 50 --include '*.rs' | jaq 'select(.type=="outline_item") | {kind, name, path}'`
- Sparse read — `atomwrite --workspace . sparse read --paths-file paths.txt --head 40 --max-files 15`
- Line-based semantic merge — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict`
- Offline semantic search — `atomwrite --workspace . semantic-search "fsync rename atomic" src/ --k 20 --min-score 0.08 --index-dir .atomwrite/semantic-index`
- Codemod dry-run — `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- Agent surface — `atomwrite --workspace . agent-surface --format json`
- Transactional batch — `atomwrite --workspace . batch --file ops.ndjson --transaction --dry-run`
- Transform dry-run — `atomwrite --workspace . transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- Rollback verify — `atomwrite --workspace . rollback src/config.toml --latest --verify`
- Hash without bak — `atomwrite --workspace . hash -r . --exclude '*.bak.*' | jaq -r '.checksum'`
- Capped watch — `atomwrite --workspace . watch src/ --debounce-ms 200 --max-events 50 --checksum --gitignore true`
- Fast stat — `atomwrite --workspace . stat src/main.rs`
- Scope comments — `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`
- Cancel awareness — on exit 143 after SIGTERM treat as clean cancel and NEVER reuse the tempfile
