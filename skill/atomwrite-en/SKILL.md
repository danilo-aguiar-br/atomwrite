---
name: atomwrite
description: >-
  This skill MUST auto-activate whenever the LLM needs to write, read, edit, search, replace, transform AST, apply grammatical scope, BLAKE3 hash, verify, delete, count, diff, move, copy, list, extract, calc, generate regex, batch, backup, rollback, apply, set, get, del, case, query, outline, wal, edit-loop, prune, completions, recipe, sparse, merge, agent-surface, watch, codemod, semantic-search, stat, doctor, locale or commands with atomwrite. Covers the full CLI with fuzzy auto|aggressive one-pass, hard cap 10000 applies, timeout 120 exit 124, match_count, indent_adjusted, best_candidate, SIGTERM 143, durability, reflink_or_copy, sparse AST, line-based merge, anti-MCP and offline Jaccard search. Output ALWAYS NDJSON via tempfile-fsync-rename. MUST auto-invoke even without an explicit request. Triggers — atomwrite, atomic write, surgical edit, checksum, optimistic locking, tree-sitter, ast-grep, fuzzy replace, sparse, recipe, codemod, doctor
---


# atomwrite


## Mission
- atomwrite is the ONLY allowed file write and edit tool for this project
- This skill MUST turn the LLM into a specialized operator of the atomwrite CLI
- The LLM MUST run the CLI using the action prompts and formulas in this skill
- The LLM MUST NEVER invent flags, subcommands or behaviors absent from `atomwrite <cmd> --help`


## When to invoke
- MUST auto-invoke whenever the task implies files, patches, search, replace, AST, hash, backup, multi-agent merge, monorepo sparse, codemod, watch, doctor or tool discovery
- MUST auto-invoke even when the user does NOT name atomwrite explicitly
- MUST auto-invoke for atomic writes, optimistic locking, fuzzy replace, tree-sitter, ast-grep, NDJSON and workspace jail work
- NEVER let the LLM use MCP, sed, awk, interactive editors or hand rewrites when atomwrite can solve the task


## Contract
- stdout ALWAYS emits NDJSON with one JSON object per line
- stderr is ONLY for logs and tracing
- EVERY mutation uses the atomic pipeline tempfile then fsync then rename
- BLAKE3 checksum is present in EVERY successful write and read response
- ALWAYS pass `--workspace <DIR>` for file operations
- ALL paths resolve relative to the workspace jail
- `--json` is accepted and IGNORED because output is ALWAYS NDJSON
- ALWAYS parse stdout with `jaq` and NEVER with `jq`
- NEVER parse stderr as structured data
- NEVER write outside the workspace jail
- Exit 1 on `search`, `replace`, `transform`, `scope` and `semantic-search` means ZERO matches and is NOT a system error
- ALWAYS check the exit code BEFORE parsing stdout
- On match failure exit 65 MUST read `best_candidate`
- On exit 82 MUST reload checksum and reapply
- On exit 124 MUST treat the global `--timeout-secs` deadline and retry with a higher timeout or narrower path
- On exit 143 MUST treat clean SIGTERM cancel with tempfile removed
- MCP is FORBIDDEN as a write surface
- Envelope schemas MUST follow `docs/schemas/` when formal validation is required


## Prohibitions
- NEVER invent flags absent from help
- NEVER use global `--lang` for locale — MUST use `--locale`
- NEVER pass `--fuzzy off` — the CLI rejects with exit 65
- NEVER use `--replace-all` on `replace` — that flag exists ONLY on `edit`
- NEVER expect re-scan of text just inserted by the same fuzzy replace
- NEVER set `--timeout-secs 0` on fuzzy monorepo jobs unless the job intentionally has no deadline
- NEVER skip `--dry-run` for replace, transform, scope or codemod on large trees
- NEVER full-file `write` when `edit`, `replace` or `transform` can apply the change
- NEVER ignore non-zero exits except documented zero-match cases
- NEVER use set, get or del on plain text
- NEVER rollback a critical file without `--verify`
- NEVER run `watch` without `--max-events` and without a global timeout


## Execution flow
- MUST locate the target with `search` or `sparse` before mutating
- MUST capture checksum with `read` when concurrency is possible
- MUST choose the most precise level of the edit hierarchy
- MUST run `--dry-run` before destructive bulk mutation
- MUST apply mutations with `--expect-checksum` on concurrent overwrites
- MUST validate the NDJSON envelope for checksum, match_count, indent_adjusted and platform
- MUST handle failures via `error_class` and `suggestion`
- MUST retry ONLY when `retryable` is true


## Global flags
- MUST use `--workspace <DIR>` as the jail root for file ops
- MUST use `--config <PATH>` to force an explicit `.atomwrite.toml`
- MUST use `--timeout-secs <N>` or alias `--timeout <N>` with default 120
- MUST treat deadline expiry as exit 124
- MUST pass 0 to `--timeout-secs` ONLY to disable the deadline
- MUST use `-j N`, `--threads N` or alias `--max-concurrency N` for parallelism
- MUST treat omitted or 0 threads as all cores with a RAM cap
- MUST use `--max-filesize <BYTES>` with global default 1 GiB
- MUST use `--locale <TAG>` for messages and suggestions
- MUST use `--json-schema` to emit the subcommand schema and exit
- MUST use `--no-auto-heal` to skip startup wal-heal
- MUST use `--no-progress` to disable NDJSON progress heartbeats
- MUST use `--no-gitignore`, `--hidden` and `--follow-symlinks` when scope requires it
- MUST use `--color auto|always|never` or `--no-color`
- MUST use `-v`, `-vv`, `-vvv` or `-q`, `-qq` for verbosity
- FORMULA global — `atomwrite --workspace . --config .atomwrite.toml --timeout-secs 120 --max-concurrency 4 --locale en --no-progress <cmd>`


## Mandatory fuzzy
- Fuzzy is MANDATORY on `edit` and `replace` with default `auto`
- Allowed values are ONLY `--fuzzy auto` and `--fuzzy aggressive`
- MUST use `--fuzzy-threshold <FLOAT>` between 0.0 and 1.0
- With `--regex` on replace fuzzy is IGNORED
- Fuzzy multi-apply is ALWAYS one-pass left-to-right on the original content
- NEVER expect re-search of text inserted by the same replace
- When NEW embeds OLD apply is forced to one occurrence and this is CORRECT and SAFE
- Without `--max-replacements` the fuzzy multi default is 1 apply
- Hard apply ceiling is 10000 and the CLI clamps above that ceiling
- Real caps — pattern 64 KiB, Levenshtein 8192 chars, windows 4096, growth max of 4x or plus 16 MiB
- On match failure MUST read `best_candidate` from the exit 65 envelope
- MUST use `--dry-run` before bulk fuzzy replace
- On exit 124 MUST raise `--timeout-secs` or narrow the path
- FORMULA safe fuzzy — `atomwrite --workspace . replace --dry-run --fuzzy auto --max-replacements 1 'OLD' 'NEW' src/`


## Edit hierarchy
- REQUIRED order from precise to blunt — search → transform → scope → replace → edit → write
- MUST use `search` to locate and inspect
- MUST use `transform` for structural AST rewrites
- MUST use `scope` for grammatical actions
- MUST use `replace` for multi-file textual substitution
- MUST use `edit` for surgical single-file edits
- MUST use `write` ONLY for create or intentional full overwrite


## Backup and configuration
- Backup defaults to ON for mutators via `--backup`, `--no-backup`, `--keep-backup` and `--retention <N>`
- `--backup` and `--no-backup` are mutually exclusive with exit 2
- Default backup creates adjacent `.bak.<timestamp>` and AUTO-REMOVES on success
- On failure the `.bak` remains for rollback
- EXCEPTION — `delete` and `replace` KEEP `.bak` on success
- EXCEPTION — `rollback` requires explicit `--backup` for opt-in backup
- EXCEPTION — `move` and `copy` require `--force` OR explicit `--backup` OR env `ATOMWRITE_BACKUP` to overwrite destination
- TOML `[defaults] backup = true` alone does NOT authorize overwrite for move and copy
- Backup uses `reflink_or_copy` and NEVER hardlinks the live file
- Precedence — env `ATOMWRITE_BACKUP` with exact value 0 disables, then CLI flags, then `.atomwrite.toml`, then built-in default
- MUST read `platform.backup_method`, `platform.durability` and `platform.rename_method` when emitted
- `.atomwrite.toml` governs `[defaults]` and `[fuzzy]`
- Valid fuzzy `mode` values are ONLY `auto` or `aggressive`
- NEVER set `mode = "off"`
- Explicit CLI flags override TOML
- FORMULA config — set `[fuzzy]` with `mode = "auto"` and `threshold = 0.85` in `.atomwrite.toml`


## File mutation
### write
- MUST send content via stdin and NEVER as a CLI argument
- MUST use `--expect-checksum <BLAKE3>` for optimistic locking
- MUST use `--durability full|fast|auto` with default `auto`
- MUST use `--allow-shrink` when a locked write shrinks the file by more than 50 percent
- MUST use `--allow-empty-stdin` and `--no-checksum-when-empty` when empty stdin is intentional
- MUST use `--syntax-check` for tree-sitter validation with exit 88 on failure
- Flags — `--append`, `--prepend`, `--max-size`, `--line-ending`, `--preserve-timestamps`, `--require-backup`, `--auto-rotate`, `--confirm`, `--risk-threshold`, `--wal-policy`, `--dry-run`, backup opts
- SIGTERM during write cleans the tempfile and returns exit 143
- FORMULA — `echo "content" | atomwrite --workspace . write --durability full target.rs`
- FORMULA lock — `CS=$(atomwrite --workspace . read file | jaq -r '.checksum') && echo "new" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto file`

### edit
- MUST use `--old` and `--new` for exact repeatable pairs
- Multi-pair runs a fuzzy cascade and emits `pairs_total` with `pair_results`
- A failed pair aborts the batch by default and `--partial` applies only matching pairs
- Multi-occurrence on edit REQUIRES `--replace-all`
- Success on edit with `--replace-all` emits `match_count`
- Indent realignment emits `indent_adjusted: true`
- Position flags — `--after-line`, `--before-line`, `--range`, `--delete-range`, `--after-match`, `--before-match`, `--between`, `--multi`
- MUST use `--old-file` and `--new-file` for large payloads
- MUST use `--expect-checksum` and `--allow-sequential-drift` only in controlled pipelines
- Terminal stdin on insert modes fails with exit 65 and does NOT hang
- NEVER pipe edit into `jaq` without checking `${PIPESTATUS[0]}`
- FORMULA — `atomwrite --workspace . edit src/main.rs --old "old" --new "new" --fuzzy auto`
- FORMULA multi — `atomwrite --workspace . edit src/main.rs --old "a" --new "b" --replace-all --fuzzy auto | jaq '{match_count, indent_adjusted, checksum}'`
- FORMULA insert — `echo "line" | atomwrite --workspace . edit src/main.rs --after-line 10`

### replace
- Exit 1 means zero matches and is NOT a system error
- MUST ALWAYS run `--dry-run` or `--preview` before applying
- MUST use `--fuzzy auto|aggressive` and `--fuzzy-threshold`
- MUST use `--progress-every <N>` with default 50 and 0 to disable
- Fuzzy multi-occurrence uses `--max-replacements` and NEVER `--replace-all`
- Fuzzy multi default is 1 apply and hard ceiling is 10000
- When NEW embeds OLD fuzzy forces one apply even with a high max
- Flags — `--regex`, `-w`, `-F`, `-g/--include`, `--exclude`, `-n/--max-replacements`, `--expect-checksum`, `--preserve-timestamps`, `--preserve-case`, backup opts
- `replace` KEEPS `.bak` on success
- FORMULA dry-run — `atomwrite --workspace . replace --dry-run --fuzzy auto --progress-every 20 'old' 'new' src/`
- FORMULA aggressive — `atomwrite --workspace . replace --fuzzy aggressive --fuzzy-threshold 0.8 --max-replacements 3 'API' 'Api' src/ --include '*.rs'`
- FORMULA section expand — `atomwrite --workspace . replace --fuzzy auto $'section {\n  old\n}' $'section {\n  old\n  new_line\n}' path.rs`

### delete move copy apply set get del case
- `delete` keeps backup ON and KEEPS `.bak` on success
- MUST use `-y/--yes` in automation and `--older-than` for age-based cleanup
- `move` and `copy` require `--force` or `--backup` or env to overwrite destination
- `copy` accepts `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr`
- `apply` detects unified, SEARCH/REPLACE, markdown and full file
- MUST use `--format auto|unified|search-replace|full|markdown` when auto-detect is ambiguous
- set, get and del operate ONLY on TOML or JSON via dotted path
- get returns exit 65 when the key is missing
- del accepts `--force-missing`
- case requires `--subvert OLD NEW` and MUST run with `--dry-run` on large trees
- FORMULA delete — `atomwrite --workspace . delete --older-than 7d --yes tmp/`
- FORMULA move — `atomwrite --workspace . move src/old.rs src/new.rs`
- FORMULA copy — `atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/`
- FORMULA apply — `echo "content" | atomwrite --workspace . apply src/file.txt --format full`
- FORMULA set — `atomwrite --workspace . set Cargo.toml package.name my-crate`
- FORMULA get — `atomwrite --workspace . get config.toml database.pool.max`
- FORMULA del — `atomwrite --workspace . del --force-missing config.toml features.experimental`
- FORMULA case — `atomwrite --workspace . case --to kebab --subvert API API --dry-run src/`


## Read and search
### read and stat
- MUST use `read` for content plus checksum, size, lines and mode
- MUST use `stat PATH` as the alias for `read --stat`
- Flags — `--lines`, `--line`, `-C/--context`, `--head`, `--tail`, `--format raw|ndjson`, `--grep`, `--verify-checksum`
- Exit 81 on `--verify-checksum` mismatch
- FORMULA — `atomwrite --workspace . read --head 20 src/main.rs`
- FORMULA — `atomwrite --workspace . stat src/main.rs`

### search
- Exit 1 means zero results and is NOT a system error
- MUST use search to locate targets before mutating
- Flags — `-g/--include`, `--exclude`, `-C`, `-F`, `-e/--regex`, `-w`, `-i`, `-S`, `-c`, `-l`, `-m`, `-U`, `-P`, `--invert`, `--sort`, `--max-columns`, `--no-begin-end`, `--include-fifo`
- Search local default for `--max-filesize` is 10 MiB
- FORMULA — `atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs' --sort path`

### list count extract diff
- list — `--include`, `--exclude`, `--long`, `--depth`, `--count-by-ext`, `--all`
- count — `--by-extension`, `--by-size`, `--top`
- extract — positional fields plus `--delimiter`, `--stdin`
- diff — `--unified`, `--stat`, `-C`, `--algorithm myers|patience|lcs`
- FORMULA list — `atomwrite --workspace . list --long --depth 2 src/`
- FORMULA count — `atomwrite --workspace . count --by-size --top 10 src/`
- FORMULA extract — `atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number`
- FORMULA diff — `atomwrite --workspace . diff src/old.rs src/new.rs --unified`


## AST and structure
### transform
- Exit 1 means zero matches
- MUST set `-l/--language`
- MUST use `$NAME` for one node and `$$$ARGS` for many
- MUST use both `-p/--pattern` and `-r/--rewrite` in single-rule mode
- Flags — `--rules`, `--inline-rules`, `--verify-parse`, `--include`, `--exclude`, `--dry-run`, backup opts
- MUST prefer transform over replace when the pattern is syntactic
- FORMULA — `atomwrite --workspace . transform --dry-run -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`

### scope
- Exit 1 means zero matches
- MUST set `--language` with subcommand alias `--lang`
- MUST use `--query` or `--pattern`
- MUST use `--delete`, `--action` or `--replace-with`
- Useful queries — comments, strings, fn, pub-fn, async-fn, struct, enum, trait, impl, mod, use, class, def, import, export
- NEVER apply `--delete` without dry-run
- FORMULA — `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`

### query and outline
- query inspects AST with `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- outline extracts structure with repeatable `--kind`, `--positions` and `--language`
- FORMULA query — `atomwrite --workspace . query --kinds src/main.rs`
- FORMULA outline — `atomwrite --workspace . outline --kind function_item --positions src/main.rs`


## Multi-file orchestration
### batch
- Input is NDJSON on stdin with required field `op` — write, replace, delete, edit, move, copy, hash
- move and copy in the manifest require `"force":true` to overwrite
- MUST use `--transaction` for all-or-nothing atomicity
- MUST use `--file <PATH>` for an on-disk manifest
- Flags — `--dry-run`, `--batch-size`, `--input-schema`, backup opts
- FORMULA — `echo '{"op":"write","target":"a.txt","content":"hello"}' | atomwrite --workspace . batch --transaction`

### recipe sparse semantic-merge agent-surface watch codemod semantic-search
- MUST use `sparse list` with `--max-files` and `--max-bytes`
- MUST use `sparse read --paths-file` with `--head` and `--max-files`
- MUST use `sparse outline` for real AST outline under budget
- MUST use `recipe list` and `recipe run --name search-replace-verify`
- Recipe search, replace and hash EXCLUDE `*.bak.*` by default
- Built-in recipes — `search-replace-verify` and `edit-loop-syntax-check`
- `recipe run` accepts `--fuzzy-threshold`, `--include`, `--exclude`, `--pairs-file`, `--target`, `--syntax-check`
- MUST use `semantic-merge --base A --ours B --theirs C --output OUT`
- semantic-merge is line-based and is NOT AST and NOT embedding
- Merge flags — `--fail-on-conflict`, `--write-conflict-markers`, `--expect-checksum`, backup opts
- MUST use `agent-surface --format json` for the tool manifesto
- MUST use `watch` with `--debounce-ms`, `--max-events`, `--checksum` and `--gitignore`
- MUST cap watch with `--max-events` or a global timeout
- `watch` requires the binary `watch` feature
- MUST use `codemod --rules rules.yaml --dry-run` before applying
- MUST use `semantic-search "query" PATH --k 20 --min-score 0.05`
- semantic-search is offline Jaccard and NEVER depends on a remote embedding API
- MUST pass `--index-dir` ONLY when a local index is required
- Exit 1 on zero semantic-search results is NOT a system error
- FORMULA sparse list — `atomwrite --workspace . sparse list src/ --max-files 50 --max-bytes 524288 --include '*.rs'`
- FORMULA sparse read — `atomwrite --workspace . sparse read --paths-file paths.txt --head 30 --max-files 10`
- FORMULA sparse outline — `atomwrite --workspace . sparse outline src/ --max-files 40 --include '*.rs'`
- FORMULA recipe — `atomwrite --workspace . recipe run --name search-replace-verify --pattern old_api --replacement new_api --path src --dry-run --fuzzy auto`
- FORMULA merge — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output out.rs --fail-on-conflict`
- FORMULA agent — `atomwrite --workspace . agent-surface --format json`
- FORMULA watch — `atomwrite --workspace . watch src/ --debounce-ms 200 --max-events 100 --checksum`
- FORMULA codemod — `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- FORMULA semantic-search — `atomwrite --workspace . semantic-search "optimistic lock checksum" src/ --k 15 --min-score 0.1 --index-dir .atomwrite/semantic-index`


## Host discovery
- MUST use `doctor` to diagnose host environment and dependencies
- MUST use `doctor --strict` when any failed check MUST abort the pipeline
- MUST use `locale` to inspect the resolved locale
- MUST use `locale --set en` or `locale --set pt-BR` to persist preference
- MUST use `locale --clear` to clear persisted preference
- MUST use `commands` to emit the full command tree as JSON
- MUST use `commands --include-globals` to include global flags in the manifesto
- FORMULA doctor — `atomwrite doctor --strict`
- FORMULA locale — `atomwrite locale --set en`
- FORMULA commands — `atomwrite commands --include-globals`


## Utilities
### hash and verify
- MUST use `hash` for BLAKE3 checksums
- Output field is `checksum` and NEVER `value`
- Flags — `--verify`, `--stdin`, `-r/--recursive`, `--exclude`
- MUST use `verify <PATH> <EXPECTED_HASH>` with exit 0 on match and 81 on mismatch
- FORMULA — `atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'`
- FORMULA recursive — `atomwrite --workspace . hash -r src/ --exclude '*.bak.*'`
- FORMULA verify — `atomwrite --workspace . verify src/main.rs abc123def456`

### backup rollback prune
- MUST use `backup` with `--retention` and `--output-dir`
- MUST use `rollback --latest --verify` for critical restore
- Rollback flags — `--timestamp`, `--latest`, `--verify`, `--dry-run`, opt-in `--backup`
- MUST use `prune-backups` with `--max-age-secs` and NEVER `--max-age`
- FORMULA backup — `atomwrite --workspace . backup src/main.rs --retention 3`
- FORMULA rollback — `atomwrite --workspace . rollback src/config.toml --latest --verify`
- FORMULA prune — `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .`

### calc regex wal edit-loop completions
- calc evaluates expressions and units without requiring workspace
- regex generates a pattern from 3 or more examples
- Regex flags — `-d`, `-w`, `-s`, `-r`, `-i`, `--no-anchors`, `--stdin`
- `wal-stats` inspects journals and does NOT modify
- `wal-heal` removes orphan journals with `--threshold-secs` and `--max-duration-ms`
- `edit-loop` applies N old and new pairs in one write
- `completions` generates bash, zsh, fish, elvish or powershell with `--install`
- FORMULA calc — `atomwrite calc "2 hours + 30 minutes to seconds"`
- FORMULA regex — `atomwrite regex "abc-12" "xyz-99" "id-7" --digits`
- FORMULA edit-loop — `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- FORMULA completions — `atomwrite completions bash --install`


## Parsing and errors
- MUST check exit code BEFORE parsing stdout
- MUST parse the JSON envelope when `error` is true
- Fields — error, code, exit, message, path, error_class, retryable, suggestion, workspace, best_candidate
- Strategy by `error_class` — permanent NEVER retries, transient uses backoff, conflict re-reads state, precondition_failed fixes the precondition
- MUST retry ONLY when `retryable` is true
- MUST use `suggestion` for remediation
- NEVER parse stderr for structured errors


## Exit codes
- 0 success
- 1 zero matches on search, replace, transform, scope and semantic-search
- 2 invalid argument or flag conflict
- 4 not found
- 13 permission denied
- 28 disk full
- 30 quota exceeded
- 65 invalid input, fuzzy off, missing key or best_candidate on match failure
- 73 cross-device
- 74 I/O error
- 78 invalid configuration
- 81 checksum mismatch
- 82 state drift
- 83 lock timeout
- 85 FIFO detected
- 86 device file
- 88 tree-sitter syntax error
- 91 EXDEV fallback disabled
- 92 copy-back BLAKE3 verification failed
- 93 orphan journal advisory
- 124 global `--timeout-secs` timeout
- 126 workspace jail violation
- 127 blocked symlink
- 128 immutable file
- 130 SIGINT
- 141 SIGPIPE
- 143 SIGTERM clean cancel
- 255 internal error


## Execution formulas
- MUST run optimistic lock — `CS=$(atomwrite --workspace . read file | jaq -r '.checksum') && echo "new" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto file`
- MUST run edit with match_count — `atomwrite --workspace . edit src/f.rs --old "x" --new "y" --replace-all --fuzzy auto | jaq '{match_count, indent_adjusted, checksum}'`
- MUST run fuzzy replace with progress — `atomwrite --workspace . replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run 'old_api' 'new_api' src/`
- MUST run section expansion with NEW embedding OLD — `atomwrite --workspace . replace --fuzzy auto $'section {\n  old\n}' $'section {\n  old\n  new_line\n}' path.rs`
- MUST recover exit 124 by raising `--timeout-secs` or narrowing the path
- MUST read best_candidate after exit 65 with `jaq '.best_candidate'`
- MUST run recipe — `atomwrite --workspace . recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- MUST run sparse monorepo — `atomwrite --workspace . sparse list . --max-files 100 --max-bytes 1048576 --include '*.rs'`
- MUST run line-based merge — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict`
- MUST run offline semantic-search — `atomwrite --workspace . semantic-search "fsync rename atomic" src/ --k 20 --min-score 0.08 --index-dir .atomwrite/semantic-index`
- MUST run host readiness — `atomwrite doctor --strict && atomwrite commands --include-globals`
- MUST treat exit 143 as clean cancel and NEVER reuse the tempfile
