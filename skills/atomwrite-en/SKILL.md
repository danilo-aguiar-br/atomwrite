---
name: atomwrite
description: >-
 This skill MUST auto-activate whenever the LLM needs atomwrite for write, read, edit, search, replace, AST transform, scope, BLAKE3 hash, verify, delete, count, diff, move, copy, list, extract, calc, regex, batch, backup, rollback, apply, set, get, del, case, query, outline, wal, edit-loop, prune, completions, recipe, sparse, semantic-merge, agent-surface, watch, codemod, semantic-search, stat, doctor, locale or commands. This skill MUST teach fuzzy auto|aggressive|off exact-only, one-pass, cap 10000, timeout 120 exit 124, MATCH_FAILED best_candidate, ack-overwrite large write, watch_summary, delete --plan, SIGTERM 143, durability, reflink_or_copy, sparse, line merge, anti-MCP, offline Jaccard. Output MUST always be NDJSON via tempfile-fsync-rename. This skill MUST auto-invoke without explicit request. Triggers include atomwrite, atomic write, surgical edit, checksum, optimistic locking, tree-sitter, ast-grep, fuzzy replace, sparse, recipe, codemod, doctor
---

# atomwrite

## Mission
### REQUIRED
- The LLM MUST treat atomwrite as the ONLY allowed file write and edit surface for this project
- The LLM MUST execute this skill as an action runbook and MUST run the CLI with the formulas below
- The LLM MUST run `atomwrite <cmd> --help` before inventing any flag and MUST refuse unknown flags
- The LLM MUST parse stdout with `jaq` and MUST check exit code BEFORE parsing
### FORBIDDEN
- NEVER use MCP, sed, awk, interactive editors or hand rewrites when atomwrite can solve the task
- NEVER invent subcommands, flags or behaviors absent from help
- NEVER invent product `ATOMWRITE_*` env knobs
- NEVER parse stderr as structured data


## Auto-invocation
### REQUIRED
- This skill MUST auto-invoke whenever the task implies files, patches, search, replace, AST, hash, backup, multi-agent merge, monorepo sparse, codemod, watch, doctor or tool discovery
- This skill MUST auto-invoke even when the user does NOT name atomwrite
### FORBIDDEN
- NEVER wait for an explicit skill request when a file operation is required


## Contract
### REQUIRED
- stdout MUST always emit NDJSON one JSON object per line
- stderr MUST be logs and tracing only
- Every mutation MUST use tempfile then fsync then rename
- Every successful write and read MUST expose BLAKE3 `checksum`
- File ops MUST pass `--workspace <DIR>` and MUST resolve paths inside the workspace jail
- `--json` is accepted and MUST be treated as ignored because output is always NDJSON
- Envelope schemas MUST follow `docs/schemas/` when formal validation is required
- Config precedence MUST be CLI flags then `.atomwrite.toml` or XDG then built-in defaults
### FORBIDDEN
- NEVER write outside the workspace jail
- NEVER depend on product env knobs for runtime behavior
- NEVER treat exit 1 on `search`, `replace`, `transform`, `scope` or `semantic-search` as a system crash when zero matches is the documented `NO_MATCHES` outcome
### Correct Pattern
- FORMULA global — `atomwrite --workspace . --config .atomwrite.toml --timeout-secs 120 --max-concurrency 4 --locale en --no-progress <cmd>`


## Agent safety contracts
### REQUIRED
- Large existing overwrite MUST pass `--ack-overwrite` when size exceeds XDG `[write].confirm_large_bytes` default 100 KiB
- Missing `--ack-overwrite` on large overwrite MUST be treated as exit 65
- Delete plan-only MUST use `delete --plan`
- Delete execute MUST omit `--plan` and MUST omit rejected confirm flags
- `watch` MUST always end with NDJSON `type:watch_summary` even on idle or zero events
- Default watch idle exit is about 500 ms via XDG or built-in unless overridden
- `semantic-merge` MUST write conflict markers by default
- Prefer-ours without markers MUST pass `--no-conflict-markers` intentionally
- On match failure exit 65 MUST parse the envelope and MUST read `best_candidate`
- On exit 82 MUST reload checksum and reapply
- On exit 124 MUST raise `--timeout-secs` or narrow the path
- On exit 143 MUST treat clean SIGTERM cancel with tempfile removed
### FORBIDDEN
- NEVER pass `delete --confirm`, `delete --yes` or `-y` because they are rejected
- NEVER overwrite large existing files without `--ack-overwrite`
- NEVER use interactive Y/N prompts for write confirmation
- NEVER treat legacy write `--confirm` as a substitute for `--ack-overwrite`
- NEVER assume empty stdout from `watch`
- NEVER disable merge markers by accident
- NEVER ship product telemetry assumptions and NEVER embed GitHub Actions inside this product CLI
### Correct Pattern
- FORMULA large write — `printf 'payload' | atomwrite --workspace . write --ack-overwrite big.txt`
- FORMULA delete plan — `atomwrite --workspace . delete --plan tmp/`
- FORMULA delete exec — `atomwrite --workspace . delete tmp/stale.txt`
- FORMULA watch — `atomwrite --workspace . watch src/ --debounce-ms 200 --max-events 100 --idle-exit-ms 500 --checksum`
- FORMULA merge — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output out.rs --fail-on-conflict`


## Global flags
### REQUIRED
- MUST use `--workspace <DIR>` as jail root
- MUST use `--config <PATH>` to force an explicit config file
- MUST use `--timeout-secs <N>` or alias `--timeout <N>` default 120 and MUST treat deadline expiry as exit 124
- MUST pass `0` to disable deadline only when intentional
- MUST use `-j N`, `--threads N` or alias `--max-concurrency N` for parallelism
- MUST treat omitted or zero threads as all cores with RAM cap
- MUST use `--max-filesize <BYTES>` global default 1 GiB
- MUST use `--locale <TAG>` for messages and suggestions and MUST never use global `--lang` for locale
- MUST use `--json-schema` to emit the subcommand schema and exit
- MUST use `--no-auto-heal` to skip startup wal-heal
- MUST use `--no-progress` to disable NDJSON progress heartbeats
- MUST use `--no-gitignore`, `--hidden` and `--follow-symlinks` when scope requires them
- MUST use `--color auto|always|never` or `--no-color`
- MUST use `-v`, `-vv`, `-vvv` or `-q`, `-qq` for verbosity
### FORBIDDEN
- NEVER invent global flags absent from `atomwrite --help`


## Fuzzy
### REQUIRED
- Fuzzy is MANDATORY on `edit` and `replace` with default `auto`
- Allowed values MUST be only `--fuzzy auto`, `--fuzzy aggressive` and `--fuzzy off`
- `--fuzzy off` MUST mean exact-only with no cascade
- Exact-only miss on edit MUST yield exit 65 `MATCH_FAILED`
- Tree-wide miss on replace with zero hits MUST often yield exit 1 `NO_MATCHES`
- MUST use `--fuzzy-threshold <FLOAT>` between 0.0 and 1.0 when tuning
- With `--regex` on replace fuzzy MUST be treated as ignored
- Multi-apply MUST be one-pass left-to-right on original content
- When NEW embeds OLD apply MUST force one occurrence
- Without `--max-replacements` fuzzy multi default MUST be 1 apply
- Hard apply ceiling MUST be 10000 and the CLI clamps above it
- Real caps MUST be treated as pattern 64 KiB, Levenshtein 8192 chars, windows 4096, growth max of 4x or plus 16 MiB
- Agents MUST default to `--fuzzy auto` or `--fuzzy aggressive` unless exact-only is intentional
### FORBIDDEN
- NEVER expect re-search of text inserted by the same fuzzy replace
- NEVER use `--replace-all` on `replace`
- NEVER set `--timeout-secs 0` on fuzzy monorepo jobs unless the job intentionally has no deadline
### Correct Pattern
- FORMULA safe fuzzy — `atomwrite --workspace . replace --dry-run --fuzzy auto --max-replacements 1 'OLD' 'NEW' src/`
- FORMULA aggressive — `atomwrite --workspace . replace --fuzzy aggressive --fuzzy-threshold 0.8 --max-replacements 3 'API' 'Api' src/ --include '*.rs'`
- FORMULA exact-only — `atomwrite --workspace . edit src/f.rs --old "exact" --new "next" --fuzzy off`
- FORMULA section expand — `atomwrite --workspace . replace --fuzzy auto $'section {\n old\n}' $'section {\n old\n new_line\n}' path.rs`


## Edit hierarchy
### REQUIRED
- MUST choose tools from precise to blunt in this order — search then transform then scope then replace then edit then write
- MUST use `search` to locate
- MUST use `transform` for structural AST rewrites
- MUST use `scope` for grammatical actions
- MUST use `replace` for multi-file textual substitution
- MUST use `edit` for surgical single-file edits
- MUST use `write` ONLY to create or intentionally overwrite a full file
### FORBIDDEN
- NEVER full-file `write` when `edit`, `replace` or `transform` can apply the change


## Execution flow
### REQUIRED
- MUST locate the target with `search` or `sparse` before mutating
- MUST capture checksum with `read` when concurrency is possible
- MUST run `--dry-run` before destructive bulk mutation
- MUST apply concurrent overwrites with `--expect-checksum`
- MUST validate NDJSON fields `checksum`, `match_count`, `indent_adjusted` and `platform` when present
- MUST handle failures via `error_class` and `suggestion`
- MUST retry ONLY when `retryable` is true
### FORBIDDEN
- NEVER ignore non-zero exits except documented zero-match cases


## Backup and configuration
### REQUIRED
- Backup defaults ON for mutators via `--backup`, `--no-backup`, `--keep-backup` and `--retention <N>`
- Default backup MUST create adjacent `.bak.<timestamp>` and auto-remove on success
- On failure `.bak` MUST remain for rollback
- `delete` and `replace` MUST keep `.bak` on success
- `rollback` MUST use explicit `--backup` for opt-in backup of the restore target
- `move` and `copy` MUST pass `--force` or explicit `--backup` to overwrite destination
- Backup MUST use `reflink_or_copy` and MUST never hardlink the live file
- `.atomwrite.toml` MUST govern `[defaults]` and `[fuzzy]` with modes `auto`, `aggressive` and `off`
- Explicit CLI flags MUST override TOML
### FORBIDDEN
- NEVER pass both `--backup` and `--no-backup` because that is exit 2
- NEVER assume TOML `[defaults] backup = true` alone authorizes move or copy overwrite
### Correct Pattern
- FORMULA config fuzzy — set `[fuzzy]` `mode = "auto"` and `threshold = 0.85` in `.atomwrite.toml`


## File mutation
### write REQUIRED
- MUST send content via stdin and NEVER as a CLI argument
- MUST use `--expect-checksum <BLAKE3>` for optimistic locking
- MUST use `--durability full|fast|auto` default `auto`
- MUST use `--allow-shrink` when a locked write shrinks more than 50 percent
- MUST use `--allow-empty-stdin` and `--no-checksum-when-empty` when empty stdin is intentional
- MUST use `--syntax-check` for tree-sitter validation with exit 88 on failure
- MUST pass `--ack-overwrite` for large existing overwrites
- MUST know flags `--append`, `--prepend`, `--max-size`, `--line-ending`, `--preserve-timestamps`, `--require-backup`, `--auto-rotate`, `--ack-overwrite`, `--require-large-ack`, `--risk-threshold`, `--wal-policy`, `--dry-run` and backup opts
- Large overwrite default-deny MUST use `--ack-overwrite` and MUST never rely on interactive confirm
- SIGTERM during write MUST clean tempfile and return exit 143
### write Correct Pattern
- FORMULA write — `echo "content" | atomwrite --workspace . write --durability full target.rs`
- FORMULA lock — `CS=$(atomwrite --workspace . read file | jaq -r '.checksum') && echo "new" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto file`

### edit REQUIRED
- MUST use `--old` and `--new` for exact repeatable pairs
- Multi-pair MUST run fuzzy cascade and emit `pairs_total` with `pair_results`
- Failed pair MUST abort the batch by default and `--partial` MUST apply only matching pairs
- Multi-occurrence on edit MUST pass `--replace-all`
- Success with `--replace-all` MUST emit `match_count`
- Indent realignment MUST emit `indent_adjusted: true` when adjusted
- MUST use position flags `--after-line`, `--before-line`, `--range`, `--delete-range`, `--after-match`, `--before-match`, `--between`, `--multi`
- MUST use `--old-file` and `--new-file` for large payloads
- MUST use `--expect-checksum` and `--allow-sequential-drift` only in controlled pipelines
- Terminal stdin on insert modes MUST fail with exit 65 and MUST not hang
### edit FORBIDDEN
- NEVER pipe edit into `jaq` without checking `${PIPESTATUS[0]}`
### edit Correct Pattern
- FORMULA edit — `atomwrite --workspace . edit src/main.rs --old "old" --new "new" --fuzzy auto`
- FORMULA multi — `atomwrite --workspace . edit src/main.rs --old "a" --new "b" --replace-all --fuzzy auto | jaq '{match_count, indent_adjusted, checksum}'`
- FORMULA insert — `echo "line" | atomwrite --workspace . edit src/main.rs --after-line 10`

### replace REQUIRED
- Exit 1 MUST mean zero matches and MUST not be treated as system crash
- MUST always run `--dry-run` or `--preview` before applying bulk replace
- MUST use `--fuzzy auto|aggressive|off` and `--fuzzy-threshold`
- MUST use `--progress-every <N>` default 50 and 0 to disable
- Fuzzy multi-occurrence MUST use `--max-replacements` only
- MUST know flags `--regex`, `-w`, `-F`, `-g/--include`, `--exclude`, `-n/--max-replacements`, `--expect-checksum`, `--preserve-timestamps`, `--preserve-case` and backup opts
### replace FORBIDDEN
- NEVER use `--replace-all` on replace
- NEVER skip dry-run on large trees
### replace Correct Pattern
- FORMULA dry-run — `atomwrite --workspace . replace --dry-run --fuzzy auto --progress-every 20 'old' 'new' src/`


## Path ops and structured mutation
### REQUIRED
- `delete` MUST keep backup ON and MUST keep `.bak` on success
- Delete age cleanup MUST use `--older-than`
- Delete recursive MUST use `-r/--recursive` and include globs with `-g/--include` when needed
- `move` and `copy` MUST pass `--force` or `--backup` to overwrite destination
- `copy` MUST use `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr` when required
- `apply` MUST detect unified, SEARCH/REPLACE, markdown and full file
- `apply` MUST use `--format auto|unified|search-replace|full|markdown` when auto-detect is ambiguous
- `set`, `get` and `del` MUST operate only on TOML or JSON via dotted path
- Missing key on get MUST be exit 65
- `del` MUST accept `--force-missing` when intentional
- `case` MUST pass `--subvert OLD NEW` and MUST dry-run on large trees
### FORBIDDEN
- NEVER pass delete confirm aliases
- NEVER use set, get or del on plain text
### Correct Pattern
- FORMULA move — `atomwrite --workspace . move src/old.rs src/new.rs`
- FORMULA copy — `atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/`
- FORMULA apply — `echo "content" | atomwrite --workspace . apply src/file.txt --format full`
- FORMULA set — `atomwrite --workspace . set Cargo.toml package.name my-crate`
- FORMULA get — `atomwrite --workspace . get config.toml database.pool.max`
- FORMULA del — `atomwrite --workspace . del --force-missing config.toml features.experimental`
- FORMULA case — `atomwrite --workspace . case --to kebab --subvert API API --dry-run src/`


## Read and search
### REQUIRED
- MUST use `read` for content plus checksum, size, lines and mode
- MUST use `stat PATH` as alias of `read --stat`
- MUST use read flags `--lines`, `--line`, `-C/--context`, `--head`, `--tail`, `--format raw|ndjson`, `--grep`, `--verify-checksum`
- Exit 81 on `--verify-checksum` mismatch MUST be treated as integrity failure
- MUST use `search` before mutating when target location is unknown
- MUST use search flags `-g/--include`, `--exclude`, `-C`, `-F`, `-e/--regex`, `-w`, `-i`, `-S`, `-c`, `-l`, `-m`, `-U`, `-P`, `--invert`, `--sort`, `--max-columns`, `--no-begin-end`, `--include-fifo`
- Search local default for `--max-filesize` MUST be treated as 10 MiB
- MUST use `list` with `--include`, `--exclude`, `--long`, `--depth`, `--count-by-ext`, `--all`
- MUST use `count` with `--by-extension`, `--by-size`, `--top`
- MUST use `extract` with positional fields plus `--delimiter` and `--stdin`
- MUST use `diff` with `--unified`, `--stat`, `-C`, `--algorithm myers|patience|lcs`
- Diff second path `-` MUST mean stdin
### Correct Pattern
- FORMULA read — `atomwrite --workspace . read --head 20 src/main.rs`
- FORMULA stat — `atomwrite --workspace . stat src/main.rs`
- FORMULA search — `atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs' --sort path`
- FORMULA list — `atomwrite --workspace . list --long --depth 2 src/`
- FORMULA count — `atomwrite --workspace . count --by-size --top 10 src/`
- FORMULA extract — `atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number`
- FORMULA diff — `atomwrite --workspace . diff src/old.rs src/new.rs --unified`
- FORMULA diff stdin — `atomwrite --workspace . diff file.txt - --unified`


## AST and structure
### REQUIRED
- `transform` exit 1 MUST mean zero matches
- `transform` MUST set `-l/--language`
- `transform` MUST use `$NAME` for one node and `$$$ARGS` for many
- Single-rule mode MUST pass both `-p/--pattern` and `-r/--rewrite`
- MUST know transform flags `--rules`, `--inline-rules`, `--verify-parse`, `--include`, `--exclude`, `--dry-run` and backup opts
- MUST prefer transform over replace when the pattern is syntactic
- `scope` MUST set `--language` with subcommand alias `--lang`
- `scope` MUST use `--query` or `--pattern`
- `scope` MUST use `--delete`, `--action` or `--replace-with`
- Scope queries MUST include comments, strings, fn, pub-fn, async-fn, struct, enum, trait, impl, mod, use, class, def, import, export
- `query` MUST use `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- `outline` MUST use repeatable `--kind`, `--positions` and `--language`
### FORBIDDEN
- NEVER apply `scope --delete` without dry-run
### Correct Pattern
- FORMULA transform — `atomwrite --workspace . transform --dry-run -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`
- FORMULA scope — `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`
- FORMULA query — `atomwrite --workspace . query --kinds src/main.rs`
- FORMULA outline — `atomwrite --workspace . outline --kind function_item --positions src/main.rs`


## Multi-file orchestration
### batch REQUIRED
- Input MUST be NDJSON on stdin with required field `op` among write, replace, delete, edit, move, copy, hash
- move and copy in the manifest MUST set `"force":true` to overwrite
- MUST use `--transaction` for all-or-nothing atomicity
- MUST use `--file <PATH>` for on-disk manifest
- MUST know flags `--dry-run`, `--batch-size`, `--input-schema` and backup opts
### batch Correct Pattern
- FORMULA batch — `echo '{"op":"write","target":"a.txt","content":"hello"}' | atomwrite --workspace . batch --transaction`

### sparse recipe merge surface watch codemod semantic-search REQUIRED
- MUST use `sparse list` with `--max-files` and `--max-bytes`
- MUST use `sparse read --paths-file` with `--head` and `--max-files`
- MUST use `sparse outline` for real AST outline under budget
- MUST use `recipe list` and `recipe run --name search-replace-verify`
- Recipe search, replace and hash MUST exclude `*.bak.*` by default
- Built-in recipes MUST be `search-replace-verify` and `edit-loop-syntax-check`
- `recipe run` MUST accept `--fuzzy-threshold`, `--include`, `--exclude`, `--pairs-file`, `--target`, `--syntax-check`
- MUST use `semantic-merge --base A --ours B --theirs C --output OUT`
- semantic-merge MUST be treated as line-based and MUST never be treated as AST or embedding
- Merge flags MUST include `--fail-on-conflict`, `--write-conflict-markers`, `--no-conflict-markers`, `--expect-checksum` and backup opts
- MUST use `agent-surface --format json` for the tool manifesto
- MUST use `watch` with `--debounce-ms`, `--max-events`, `--idle-exit-ms`, `--checksum` and `--gitignore`
- MUST cap watch with `--max-events` or global timeout
- `watch` requires the binary feature `watch`
- MUST use `codemod --rules rules.yaml --dry-run` before applying
- MUST use `semantic-search "query" PATH --k 20 --min-score 0.05`
- semantic-search MUST be offline Jaccard and MUST never depend on a remote embedding API
- MUST pass `--index-dir` only when a local index is required
### FORBIDDEN
- NEVER run unbounded `watch` without max-events or timeout
- NEVER treat zero semantic-search hits as a system crash
### Correct Pattern
- FORMULA sparse list — `atomwrite --workspace . sparse list src/ --max-files 50 --max-bytes 524288 --include '*.rs'`
- FORMULA sparse read — `atomwrite --workspace . sparse read --paths-file paths.txt --head 30 --max-files 10`
- FORMULA sparse outline — `atomwrite --workspace . sparse outline src/ --max-files 40 --include '*.rs'`
- FORMULA recipe — `atomwrite --workspace . recipe run --name search-replace-verify --pattern old_api --replacement new_api --path src --dry-run --fuzzy auto`
- FORMULA agent — `atomwrite --workspace . agent-surface --format json`
- FORMULA codemod — `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- FORMULA semantic-search — `atomwrite --workspace . semantic-search "optimistic lock checksum" src/ --k 15 --min-score 0.1 --index-dir .atomwrite/semantic-index`


## Host discovery
### REQUIRED
- MUST use `doctor` to diagnose host environment and dependencies
- MUST use `doctor --strict` when any failed check MUST abort the pipeline
- MUST use `locale` to inspect the resolved locale
- MUST use `locale --set en` or `locale --set pt-BR` to persist preference
- MUST use `locale --clear` to clear persisted preference
- MUST use `commands` to emit the full command tree as JSON
- MUST use `commands --include-globals` to include global flags in the manifesto
### Correct Pattern
- FORMULA doctor — `atomwrite doctor --strict`
- FORMULA locale — `atomwrite locale --set en`
- FORMULA commands — `atomwrite commands --include-globals`


## Utilities
### REQUIRED
- MUST use `hash` for BLAKE3 and MUST read field `checksum` never `value`
- MUST know hash flags `--verify`, `--stdin`, `-r/--recursive`, `--exclude`
- MUST use `verify <PATH> <EXPECTED_HASH>` with exit 0 on match and 81 on mismatch
- MUST use `backup` with `--retention` and `--output-dir`
- MUST use `rollback --latest --verify` for critical restore
- Rollback flags MUST include `--timestamp`, `--latest`, `--verify`, `--dry-run` and opt-in `--backup`
- MUST use `prune-backups` with `--max-age-secs` and NEVER `--max-age`
- `calc` MUST evaluate expressions and units without requiring workspace
- `regex` MUST generate a pattern from 3 or more examples with flags `-d`, `-w`, `-s`, `-r`, `-i`, `--no-anchors`, `--stdin`
- `wal-stats` MUST inspect journals and MUST not modify
- `wal-heal` MUST remove orphan journals with `--threshold-secs` and `--max-duration-ms`
- `edit-loop` MUST apply N old and new pairs in one write
- `completions` MUST generate bash, zsh, fish, elvish or powershell with `--install`
### Correct Pattern
- FORMULA hash — `atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'`
- FORMULA hash recursive — `atomwrite --workspace . hash -r src/ --exclude '*.bak.*'`
- FORMULA verify — `atomwrite --workspace . verify src/main.rs abc123def456`
- FORMULA backup — `atomwrite --workspace . backup src/main.rs --retention 3`
- FORMULA rollback — `atomwrite --workspace . rollback src/config.toml --latest --verify`
- FORMULA prune — `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .`
- FORMULA calc — `atomwrite calc "2 hours + 30 minutes to seconds"`
- FORMULA regex — `atomwrite regex "abc-12" "xyz-99" "id-7" --digits`
- FORMULA edit-loop — `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- FORMULA completions — `atomwrite completions bash --install`


## Parsing and errors
### REQUIRED
- MUST check exit code BEFORE parsing stdout
- MUST parse the JSON envelope when `error` is true
- MUST read fields error, code, exit, message, path, error_class, retryable, suggestion, workspace, best_candidate
- `MATCH_FAILED` exit 65 MUST mean edit or fuzzy miss and MUST expose `best_candidate` for agent recovery
- `NO_MATCHES` exit 1 MUST mean search or replace zero matches across the tree
- permanent MUST never retry, transient MUST use backoff, conflict MUST re-read state, precondition_failed MUST fix the precondition
- MUST use `suggestion` for remediation
### FORBIDDEN
- NEVER parse stderr for structured errors
- NEVER retry when `retryable` is false


## Exit codes
### REQUIRED
- 0 success
- 1 zero matches on search, replace, transform, scope and semantic-search as `NO_MATCHES`
- 2 invalid argument or flag conflict
- 4 not found
- 65 invalid input, large overwrite without `--ack-overwrite`, missing key, or `MATCH_FAILED`
- 78 invalid configuration or missing feature
- 81 checksum mismatch
- 82 state drift
- 88 tree-sitter syntax error
- 124 global `--timeout-secs` timeout
- 126 workspace jail violation
- 143 SIGTERM clean cancel
- MUST map remaining codes from the envelope fields `exit` and `error_class` and MUST never invent semantics


## Mandatory execution formulas
### REQUIRED
- MUST run optimistic lock — `CS=$(atomwrite --workspace . read file | jaq -r '.checksum') && echo "new" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto file`
- MUST run edit with match_count — `atomwrite --workspace . edit src/f.rs --old "x" --new "y" --replace-all --fuzzy auto | jaq '{match_count, indent_adjusted, checksum}'`
- MUST run fuzzy replace with progress — `atomwrite --workspace . replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run 'old_api' 'new_api' src/`
- MUST recover exit 124 by raising `--timeout-secs` or narrowing the path
- MUST read best_candidate after exit 65 with `jaq '.best_candidate'`
- MUST run recipe — `atomwrite --workspace . recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- MUST run sparse monorepo — `atomwrite --workspace . sparse list . --max-files 100 --max-bytes 1048576 --include '*.rs'`
- MUST run line-based merge — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict`
- MUST run offline semantic-search — `atomwrite --workspace . semantic-search "fsync rename atomic" src/ --k 20 --min-score 0.08 --index-dir .atomwrite/semantic-index`
- MUST run host readiness — `atomwrite doctor --strict && atomwrite commands --include-globals`
- MUST treat exit 143 as clean cancel and MUST never reuse the tempfile
