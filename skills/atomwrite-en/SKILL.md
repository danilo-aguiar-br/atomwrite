---
name: atomwrite
description: >-
 This skill MUST auto-activate whenever the LLM needs atomwrite to read, write, edit, search, replace, patch, transform AST, hash BLAKE3, back up, rollback, merge, prune, codemod, extract PDF image and OCR text, watch, sparse, semantic search or configure files. It MUST teach 47 subcommands with absolute --workspace, flag or NDJSON stdin, and always-emit NDJSON with checksum, error_class, retryable and exit code. It MUST auto-invoke on any file mutation or discovery task without an explicit request.
---

# atomwrite


## Objectives
- Spend fewer tokens on every file task, because one NDJSON call replaces the shell pipeline and `commands --brief` answers in 5 020 bytes where the full tree costs 67 492
- Lower the financial cost, because the bill follows the tokens and the round trips
- Reduce errors with atomic writes and a BLAKE3 checksum in every response
- Reduce hallucinations with a closed schema per verb and an exit code that names the failure
- Reduce the time spent, because read, search and write run in one process
- Reduce latency, because every operation is local and no network sits on the file path
- Multiply the speed of operations with `batch`, `edit-loop`, `replace`, `case`, `transform` and `codemod` over many files in one call
- Cut the lead time, because the response already carries the checksum, the `target_resolved` and the next step
- Make the agent faster, more effective and more assertive, because `code` and `suggestion` say what to do next


## Mission
### REQUIRED
- The LLM MUST treat atomwrite as the ONLY surface for file read, write, edit, search, AST, hash, backup, and file orchestration
- The LLM MUST execute this skill as a runbook and MUST run the CLI with the formulas below
- The LLM MUST run `atomwrite <cmd> --help` before inventing a flag and MUST refuse flags absent from help
- The LLM MUST parse the exit code BEFORE stdout and MUST use `jaq` on NDJSON
- The LLM MUST discover the 47-subcommand SSOT via `agent-surface`
### FORBIDDEN
- NEVER use sed, awk, interactive editors, or hand rewrites when atomwrite solves the task
- NEVER invent subcommands, flags, or `ATOMWRITE_*` knobs. NEVER parse stderr as data


## Auto-invocation
### REQUIRED
- This skill MUST auto-invoke on any file, patch, search, replace, AST, hash, backup, merge, sparse, codemod, watch, doctor, scratch, or tool-discovery task even when the user does not name atomwrite
### FORBIDDEN
- NEVER wait for an explicit skill request when a file operation is required
### Correct Pattern
- The LLM MUST run `atomwrite agent-surface --format json` at first need for surface inventory


## Cheapest formulas
### REQUIRED
- The LLM MUST prefer these calls, because each answers in fewer bytes and fewer tool calls than the spelling it replaces
- Locate a symbol with `atomwrite --workspace "$WS" read --format raw --symbol NAME path`, and locate a block with `--line N --enclosing`
- Read several windows of several files in one call with `--lines 1:3 --lines 10:12 a.rs b.rs`, which prints `==> path <==` per file
- Get line numbers with `--number`, which prints `N:text`, and get context with `-C N --grep`, so awk, nl and printf leave the command line
- A capped read stops at 24 000 bytes, prints the next `--lines` on stderr, and `--all` turns the cap off: follow the hint instead of re-reading from line 1
- Use `search --format text` for one `path:line:col:text` per match, and `list --format text` or `outline --format tsv` to fit one screen
- Read `--show` in the reply that `apply` already returned, and do not re-read the file to confirm the swap
- Patch several files in one call: omit the target and put a bare path line above each file's first block, because one call per file costs a full prompt each time
### FORBIDDEN
- NEVER pass `--all` to `apply` by reflex: a repeated SEARCH exits 65 `MATCH_AMBIGUOUS` on purpose, and `--all` replaces every occurrence
- NEVER assume `apply` writes broken code: the syntax check runs before and after, and `--no-syntax-check` is the explicit opt-out


## Contract
### REQUIRED
- stdout MUST always emit NDJSON (one JSON per line), stderr is logs only
- Every mutation MUST use tempfile-fsync-rename, success MUST expose the BLAKE3: `write`, `hash` and `read` carry `checksum`, and `edit` carries `checksum_before` and `checksum_after` (never `checksum`)
- Every terminal envelope MUST carry `workspace_source` set to `argv` when `--workspace` was passed and to `cwd` when it was not, plus `target_resolved` and `target_source`. The LLM MUST check all three before claiming where the CLI touched
- Each schema is a UNION of every event of the verb plus the error envelope, so the LLM MUST fetch the whole union with `--print-schema NAME` and NEVER validate against an imagined fixed shape
- File ops MUST pass `--workspace <DIR>` and MUST stay inside the jail
- `--json` is accepted and MUST be ignored (output is always NDJSON)
- Precedence MUST be CLI flags, then the workspace `.atomwrite.toml`, then the global XDG `config.toml` (the two files are merged key by key), then built-in defaults
### FORBIDDEN
- NEVER write outside the jail. NEVER depend on product env knobs at runtime
- NEVER treat exit 1 on `search`/`replace`/`transform`/`scope`/`semantic-search` as a crash when it is `NO_MATCHES`
### Correct Pattern
- The LLM MUST run `atomwrite --workspace "$WS" --timeout-secs 120 --locale en --no-progress <cmd>`


## Absolute workspace
### REQUIRED
- The LLM MUST set `WS` ONCE to the ABSOLUTE path of the workspace root, before any subcommand, and write `--workspace "$WS"`
- The LLM MUST pass an ABSOLUTE `--workspace` to a detached process and to a subagent, which do not inherit a meaningful CWD
- In PowerShell the same rule holds with `$WS = 'C:\path\to\root'` and `--workspace $WS`
### FORBIDDEN
- NEVER write `--workspace` with a dot as the value, because `.` is the CWD of the PROCESS and no contract in this document fixes which CWD that is
- NEVER forget that the jail is ANCHORED on the value of `--workspace`, so a wrong `.` silently moves the whole jail to the wrong folder
- NEVER forget that NESTED projects make `.` resolve to the child when the parent was meant
### Correct Pattern
- The LLM MUST run `WS=/absolute/path/to/root; atomwrite --workspace "$WS" read --head 20 path`


## Agent safety contracts
### REQUIRED
- Large overwrite MUST pass `--ack-overwrite` (XDG `[write].confirm_large_bytes`, default 100 KiB). Missing flag = exit 65, `--ack-overwrite` covers only the large file, and shrinking by more than 50% needs `--allow-shrink` separately (without it exit 65)
- Delete plan-only MUST use `delete --plan`
- Execute MUST omit `--plan` and confirm flags
- A directory needs `-r` even in a plan (without it exit 65 with `missing_flag`)
- `watch` MUST end with NDJSON `watch_summary` and MUST cap with `--max-events` or timeout (idle ~500 ms)
- `watch` exists only in a build with `--features watch` (`cargo install atomwrite --features watch`), and without it exits 78 `CONFIG_INVALID`, so MUST check `atomwrite agent-surface | jaq '.tools[]|select(.name=="watch").available'` first
- `semantic-merge` MUST write line-based markers by default. Prefer-ours without markers MUST use `--no-conflict-markers`
- A verb whose feature is missing from the build MUST exit 78 `CONFIG_INVALID`, as `outline` does in a core-only build, so MUST read the `available` field of `agent-surface` before planning that call
- Exit 65 MUST read `best_candidate` and `match_count`, 82 MUST reload checksum, 124 MUST raise timeout or narrow path, 143 = clean SIGTERM, 126 = jail, 81 = checksum, 88 = syntax
### FORBIDDEN
- NEVER pass `delete --confirm`/`--yes`/`-y`. NEVER large-overwrite without `--ack-overwrite`
- NEVER use interactive confirm
- NEVER assume empty `watch` stdout
- NEVER disable markers by accident
### Correct Pattern
- The LLM MUST run `printf 'x' | atomwrite --workspace "$WS" write --ack-overwrite --allow-shrink big.txt` and `atomwrite --workspace "$WS" delete --plan -r tmp/`


## Global flags
### REQUIRED
- MUST use `--workspace <DIR>`, `--config <PATH>`, `--timeout-secs <N>` or `--timeout <N>` (default 120 → exit 124, `0` only if intentional)
- MUST use `-j N` / `--threads N` / `--max-concurrency N` (omitted or zero = all cores up to a built-in ceiling of 16, RAM-capped, an explicit N is honoured as given, even above 16)
- MUST use `--max-filesize <BYTES>` (global default 1 GiB), `--locale <TAG>` (NEVER global `--lang`)
- MUST use `--json-schema`, `--no-auto-heal`, `--no-progress`, `--no-gitignore`, `--hidden`, `--follow-symlinks`
- `--strict-atomic` on a writing verb such as `write` or `apply` MUST fail with exit 91 when the atomic rename is impossible instead of falling back to a cross-device copy
- MUST use `--color auto|always|never` or `--no-color`, `-v`/`--verbose` up to `-vvv` or `-q`/`--quiet` up to `-qq`
- `--ready-file PATH` MUST write the process ID once the signal handlers are installed, which is how a host learns the process accepts signals
- Agent-native shape is REQUIRED: `--select`/`--fields KEYS`, `--filter key=value` (AND, `!=`/`==`), `--limit N`
- Always-emit errors: `--filter` MUST NOT silence `ok` false. MUST prefer shape over jq/sed pipes
- MUST discover surface via `atomwrite agent-surface` (SSOT 47)
- Stream shaping, all optional and global: `--sort-by path-line|line-path|KEY`, `--dedupe KEYS` (CSV), `--aggregate KEY[:OP[:FIELD]]` (OP `count|sum|min|max`), `--join KEY:SUFFIX` (the table comes from stdin), `--diff-against PATH` (baseline NDJSON inside the jail, emits one `stream_diff` line), `--redact`, `--normalize-encoding` (NFC), `--truncate-content KEY:LEN`
- `--max-output-bytes N` cuts every string value, `checksum` included, and MUST be paired with `--allow-truncate` (each is rejected without the other, exit 2). MUST NOT use them when the full `checksum` or `content` is needed
- `--non-interactive` declares agent-hosted execution (never prompts, never reads a confirmation from stdin), `--confirm-side-effect` acknowledges a side effect only where the command contract asks for it (see `commands --include-globals`)
- `--print-schema [NAME]` lists the 55 schema names, or emits one schema
- An unknown name exits 65
- Each verb that writes more than one record serves a union of every record plus the error envelope, so `search`, `replace`, `list`, `backup`, `batch`, `recipe`, `codemod`, `sparse`, `case`, `watch`, `semantic-search` and `semantic-merge` need one name only
- The signal schema names are `best-candidate`, `cancelled`, `error`, `progress`, `stream_diff`, `write-risk`, `count-by-ext` and `count-by-size`
### FORBIDDEN
- NEVER invent flags absent from `atomwrite --help`
- NEVER use global `-n` for limit (`-n` belongs to `replace`, use `--limit`)
- NEVER dump full NDJSON when `--select` is enough
### Correct Pattern
- The LLM MUST run `atomwrite --workspace "$WS" --select ok,error,code,match_count,suggestion --filter ok=false --limit 5 <cmd>`


## Full command surface
### REQUIRED
- The LLM MUST invoke each of the 47 subcommands, and the typical prefix `atomwrite --workspace "$WS"` may be omitted below when obvious
- The LLM MUST include `--workspace` on file ops

- `read`: read with checksum, `read --head 40 path`
- `write`: create/overwrite via stdin, `echo body | write --durability auto path`
- `edit`: surgical edit, `edit path --old a --new b --fuzzy auto`
- `search`: locate patterns, `search TODO src/ --include '*.rs'`
- `replace`: multi-file replace, `replace --dry-run --fuzzy auto OLD NEW src/`
- `hash`: BLAKE3, `hash path | jaq -r .checksum`
- `delete`: remove with plan, `delete --plan path`
- `count`: counts, `count --by-size --top 10 src/`
- `diff`: unified diff, `diff a.rs b.rs --unified`
- `move`: move/rename, `move src/old.rs src/new.rs`
- `copy`: copy, `copy --recursive --preserve src/ dest/`
- `list`: list tree, `list --long --depth 2 src/`
- `extract`: project NDJSON fields, `extract --field path` or `extract path`
- `calc`: expressions/units, `atomwrite calc "2 hours to seconds"`
- `regex`: generate regex, `atomwrite regex abc-12 xyz-99 id-7 --digits`
- `transform`: AST rewrite, `transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- `scope`: grammatical actions, `scope src/ --lang rust --query comments --delete --dry-run`
- `batch`: NDJSON manifest, `echo '{"op":"write","target":"a.txt","content":"x"}' | batch --transaction`
- `backup`: backup with retention, `backup path --retention 3`
- `rollback`: restore, `rollback path --latest --verify`
- `apply`: unified/SR patch, `echo body | apply path --format full`
- `set`: write dotted TOML/JSON, `set Cargo.toml package.name x`
- `get`: read dotted, `get config.toml database.pool.max`
- `del`: delete key, `del --force-missing config.toml features.x`
- `case`: case/subvert on a FILE (a directory does nothing), `case --to pascal --subvert user_id UserId --dry-run src/a.rs`
- `query`: AST nodes, `query --kinds src/main.rs`
- `outline`: outline, `outline --kind function_item --positions src/main.rs`
- `wal-stats`: WAL journals, `wal-stats`
- `wal-heal`: remove stale terminal journals, `wal-heal . --threshold-secs 3600`
- `semantic-merge`: line-based merge, `semantic-merge --base b --ours o --theirs t --output out --fail-on-conflict`
- `sparse`: budgeted monorepo, `sparse list src/ --max-files 50 --max-bytes 524288`
- `recipe`: recipes, `recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run`
- `stat`: alias `read --stat`, `stat path`
- `agent-surface`: SSOT 47 tools, `atomwrite agent-surface --format json`
- `watch`: FS with cap, only in a `--features watch` build (without it exit 78), `watch src/ --max-events 100 --idle-exit-ms 500`
- `codemod`: bulk rewrites, `codemod --rules rules.yaml --dry-run src/`
- `semantic-search`: offline Jaccard, `semantic-search query src/ --k 20 --min-score 0.05`
- `doctor`: host, `atomwrite doctor --strict`
- `scratch`: jail staging, `scratch init .atomwrite/scratch`
- `locale`: locale, `atomwrite locale --set en`
- `commands`: command tree JSON, `atomwrite commands --include-globals`
- `completions`: shell, `atomwrite completions bash --install`
- `prune-backups`: prune, `prune-backups --max-age-secs 86400 --dry-run src/lib.rs`, only `.bak.<stamp>` names are candidates
- `edit-loop`: N pairs one write, all-or-nothing (a pair with no match exits 65), `--partial` writes the matches and exits 87, `echo '[{"old":"a","new":"b"}]' | edit-loop path`
- `verify`: expected hash, `verify path EXPECTED_HASH`
- `extract-artifact`: bounded signal from PDF, OOXML and image headers, `extract-artifact path`
- `config`: persistent tuning keys with `--scope global|workspace`, `config set fuzzy.max_content_bytes 1000000 --scope global`
### FORBIDDEN
- NEVER invent a 48th subcommand. NEVER skip `agent-surface` when surface is unknown
- NEVER omit `--workspace` on file ops inside the jail


## Fuzzy
### REQUIRED
- Fuzzy on `edit`/`replace` defaults to `auto`, values ONLY `--fuzzy auto|aggressive|off`
- `--fuzzy off` = exact-only. Edit miss = exit 65 `MATCH_FAILED`, tree replace miss often exit 1 `NO_MATCHES`
- MUST use `--fuzzy-threshold` 0.0–1.0. With `--regex` on replace fuzzy MUST be ignored
- Multi-apply MUST be one-pass L→R on original
- NEW embeds OLD MUST force 1 occurrence
- Without `--max-replacements` multi default = 1, hard cap 10000
- Caps: pattern 64 KiB, Levenshtein 8192, windows 4096, growth max 4x or +16 MiB
- Normalized strategies (`line_trimmed`, `whitespace_normalized`, `punctuation_normalized`, `indent_flexible`, `trimmed_boundary`, `escape_normalized`) emit `similarity` null: NEVER treat fake 1.0 as byte equality unless strategy is `exact`
- Soft-floor `context_aware`: XDG `[fuzzy].soft_floor_delta` default 0.05. Effective accept = threshold OR threshold minus soft_floor_delta
- `block_anchor` dual-gate (edges + middle ratio)
- Content above `[fuzzy].max_content_bytes` (default 256 MiB) never runs the cascade: `edit` exits 65 `INVALID_INPUT` naming the key and `--fuzzy off`, and `replace` reports the file in `files_failed`. An exact match and `--fuzzy off` still work above it
- `escape_normalized` is skipped above 32 MiB of content, and the cascade goes on with the other strategies
- A miss on a huge file still honours `--timeout-secs`: exit 124 with a `cancelled` envelope, never a late `MATCH_FAILED`
- `MATCH_AMBIGUOUS` MUST include `best_candidate`, and MUST expand `--old` or use `--replace-all` (edit) or narrow scope instead of guessing
- It happens only when two or more candidates pass the dual gate
- An absent block that merely looks like many lines is `MATCH_FAILED` with `best_candidate`
- Fuzzy is NOT AST: MUST use `transform`/`scope`/`codemod` for semantics
### FORBIDDEN
- NEVER expect re-search of text inserted by the same fuzzy replace. NEVER use `--replace-all` on `replace`
- NEVER set `--timeout-secs 0` on fuzzy monorepo unless intentional no-deadline job
- NEVER run tree-wide `--fuzzy aggressive` without `--dry-run`. NEVER pass invented `--strategy` on edit
### Correct Pattern
- The LLM MUST run `atomwrite --workspace "$WS" replace --dry-run --fuzzy auto --max-replacements 1 OLD NEW src/`


## Edit hierarchy + execution flow
### REQUIRED
- Order precise to blunt: `search` → `transform` → `scope` → `codemod` → `replace` → `edit` → `write`
- `write` ONLY to create or intentionally overwrite a full file
- MUST locate with `search` or `sparse` before mutate. MUST capture checksum with `read` under concurrency
- MUST run `--dry-run` before destructive bulk mutation. MUST use `--expect-checksum` for concurrent overwrite
- MUST validate `checksum`, `match_count`, `indent_adjusted`, `platform` when present
- MUST handle failures via `error_class` and `suggestion`. Retry ONLY if `retryable` true
### FORBIDDEN
- NEVER full-file `write` when `edit`/`replace`/`transform` solves it
- NEVER ignore non-zero exit except documented zero-match
### Correct Pattern
- The LLM MUST run `atomwrite --workspace "$WS" search target src/` and only then mutate


## Backup and configuration
### REQUIRED
- Backup default true on mutators via `--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`
- Default backup MUST create adjacent `.bak.<timestamp>` and auto-remove on success, on failure MUST remain
- The LLM MUST pass `--no-backup` on every `delete` and `replace`, so no `.bak` is left
- No hook erases `.bak`, so `--no-backup` is the ONLY guarantee against residue
- `rollback` MUST use explicit `--backup` for opt-in backup of restore target
- `move` and `copy` MUST pass `--force` or `--backup` to overwrite destination
- Backup MUST use `reflink_or_copy` and NEVER hardlink the live file
- `.atomwrite.toml` governs `[defaults]` and `[fuzzy]` (`auto|aggressive|off`), CLI flags win over TOML
### FORBIDDEN
- NEVER pass both `--backup` and `--no-backup` (exit 2)
- NEVER assume TOML `backup = true` alone authorizes move/copy overwrite
### Correct Pattern
- The LLM MUST set `[fuzzy] mode = "auto"` and `threshold = 0.85` in `.atomwrite.toml`


## Config verb
### REQUIRED
- `config path|list|get|set|unset` is the ONLY way to change a persistent tuning key. MUST NOT edit `config.toml` or `.atomwrite.toml` by hand
- `config set KEY VALUE --scope global|workspace`, `config unset KEY --scope ...` and `config path --scope ...` MUST pass `--scope` (no default, without it exit 2 and nothing is touched)
- `config list` and `config get KEY` MUST be read before a `set`. They print `value`, `origin` (`default`, `global`, `workspace` or `explicit` with `--config`) and `default`, with no `--scope`
- Scope `global` is the XDG `config.toml` (mode 0600), scope `workspace` is the `.atomwrite.toml` at the workspace root, `set` keeps comments, validates the whole file and writes through tempfile-fsync-rename without backup
- The terminal event carries `target_resolved` and `target_source: "argv"`. MUST check that the path is the file of the intended scope
- The 43 keys are `defaults.line_ending`, `defaults.backup`, `delete.backup`, `defaults.retention`, `defaults.max_filesize`, `fuzzy.mode`, `fuzzy.threshold`, `fuzzy.threshold_aggressive`, `fuzzy.threshold_context`, `fuzzy.threshold_jw`, `fuzzy.soft_floor_delta`, `fuzzy.max_content_bytes`, `apply.max_ellipsis_lines`, `write.confirm_large_bytes`, `write.shrink_block_percent`, `write.auto_rotate_max_age_secs`, `write.content_risk_patterns`, `watch.idle_exit_ms`, `watch.debounce_ms`, `log.file`, `log.rotation`, `storage.home`, `locale`, `concurrency.max_threads`, `concurrency.ram_fraction_percent`, `search.context`, `search.smart_case`, `search.max_filesize`, `search.max_columns`, `search.max_inflight_bytes`, `search.format`, `read.max_output_bytes`, `read.number`, `output.profile`, `summary.errors_max_entries`, `lock.acquire_timeout_ms`, `retry.base_ms`, `retry.max_ms`, `retry.max_elapsed_ms`, `extract.max_sample_rows`, `extract.max_text_chars`, `extract.max_sheets` and `extract.max_slides`
- `output.profile` MUST stay `full` unless the session must spend the fewest tokens, because `lean` drops the digests, the byte counts and the timing from a successful mutation and keeps the decision fields, which took one `apply` on a three-line file from 433 bytes to 196
- `delete.backup` MUST override `defaults.backup` for `delete` only, and an unset key MUST fall back to `defaults.backup`
- `read.max_output_bytes` (default 24000) MUST be the cap of a raw read, and `read.number` and `search.format` MUST set the default of `--number` and `--format`
- `log.file`, `log.rotation`, `storage.home` and `locale` are global-only (workspace scope = exit 65). An unknown key or an out-of-range value exits 65 and writes nothing
- Accepted ranges: `concurrency.max_threads` 1 to 256, `concurrency.ram_fraction_percent` 1 to 100, `lock.acquire_timeout_ms` 100 to 3600000, `retry.base_ms` and `retry.max_ms` 1 to 60000 (base must not exceed max), `retry.max_elapsed_ms` 1 to 3600000, `search.max_inflight_bytes` 4 MiB to 1 GiB, `extract.max_sample_rows` up to 100000, `extract.max_text_chars` up to 10000000, `extract.max_sheets` and `extract.max_slides` up to 10000
- A flag still wins over the config (`extract-artifact --max-text-chars` and `--max-sample-rows`), and `--config PATH` replaces both files on `get` and `list`
- `config` works with a broken `config.toml`. A file that is not valid TOML is never rewritten (`set` and `unset` exit 78)
### FORBIDDEN
- NEVER invent a key, `defaults.line_ending`, `defaults.backup`, `defaults.retention` and `defaults.max_filesize` ARE `config` keys and change the default of every mutating verb
- NEVER run `config set` or `config unset` without `--scope`
### Correct Pattern
- The LLM MUST run `atomwrite --workspace "$WS" config get fuzzy.max_content_bytes` and then `atomwrite --workspace "$WS" config set fuzzy.max_content_bytes 1000000 --scope global`


## File mutation write/edit/replace
### REQUIRED
- `write` MUST send content via stdin. MUST use `--expect-checksum`, `--durability full|fast|auto` (default `auto`)
- Rewrite that shrinks >50% MUST use `--allow-shrink`
- MUST use `--allow-empty-stdin`, `--no-checksum-when-empty`, `--syntax-check` (exit 88), `--ack-overwrite`
- Write flags: `--append`, `--prepend`, `--max-size`, `--line-ending`, `--preserve-timestamps`, `--require-backup`, `--auto-rotate`, `--require-large-ack`, `--risk-threshold`, `--wal-policy`, `--dry-run`, backup opts
- SIGTERM during write MUST clean tempfile and return exit 143
- `edit` MUST use `--old`/`--new` (repeatable pairs)
- Multi-pair emits `pairs_total`/`pair_results`
- Failure aborts the batch unless `--partial`
- Multi-occurrence on edit MUST use `--replace-all` and MUST emit `match_count`. Re-indented emits `indent_adjusted` true
- Position flags: `--after-line`, `--before-line`, `--range`, `--delete-range`, `--after-match`, `--before-match`, `--between`, `--multi`
- Patches with `//`, jq, quotes, multiline MUST use `--old-file`/`--new-file` staged under `$WS/.atomwrite/scratch/` after `scratch init .atomwrite/scratch`
- Patches with `//`, jq, quotes, `$`, backticks or multiline MUST use `apply --no-backup` with a `<<'EOF'` heredoc and SEARCH/REPLACE blocks, because the text goes raw, with no JSON and no escaping
- The LLM MUST prefer `apply` with a heredoc over `edit-loop`, which requires JSON on stdin and pushes the model to use `python` only to build that JSON
- MUST use `--expect-checksum` and `--allow-sequential-drift` only in controlled pipelines
- Terminal stdin on insert MUST fail exit 65 without hanging
- Multi-root: if target is under home skills, MUST pass `--workspace` at the allowed root
- `replace` exit 1 = zero matches (NOT a crash). MUST ALWAYS `--dry-run`/`--preview` before bulk apply
- Fuzzy multi on replace MUST use only `--max-replacements` (`-n`)
- Replace flags: `--regex`, `-w/--word`, `-F/--literal`, `-g/--include`, `--exclude`, `--expect-checksum`, `--preserve-timestamps`, `--preserve-case`, backup opts
- A pattern or a replacement that starts with a hyphen MUST come after a bare `--`, so an unknown flag is reported as an argument error instead of being read as the pattern
### FORBIDDEN
- NEVER pipe edit to `jaq` without `${PIPESTATUS[0]}`
- NEVER put old/new-file under OS `/tmp` outside the jail
- NEVER pass unquoted `$VAR` when literal dollar must survive: MUST quote `"$VAR"`
- NEVER treat fuzzy as AST. NEVER shell-quote fragile patches as `--old`/`--new` when scratch exists
- NEVER skip dry-run on large trees. NEVER shrink >50% without `--allow-shrink`
### Correct Pattern
- The LLM MUST run multiline edit: `atomwrite --workspace "$WS" apply --no-backup path <<'EOF'`, then a line with `<<<<<<< SEARCH`, the old text, a line with `=======`, the new text, a line with `>>>>>>> REPLACE`, and finally `EOF`. Several blocks fit in the same heredoc
- The LLM MUST run staging: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch; SCR="$WS/.atomwrite/scratch"; printf %s "$OLD" >"$SCR/old.txt"; printf %s "$NEW" >"$SCR/new.txt"; atomwrite --workspace "$WS" edit --old-file "$SCR/old.txt" --new-file "$SCR/new.txt" path`
- The LLM MUST run `atomwrite --workspace "$WS" write --durability auto --no-backup --allow-shrink "$path" < "$new"`


## Path and structured
### REQUIRED
- `delete` MUST use `--no-backup`, age via `--older-than`, recursive `-r`, globs `-g/--include`, plan via `--plan`
- `delete -r <dir>` MUST own the whole named tree: hidden, ignored, linked, fifo, socket and device entries are removed, and a link is unlinked without following it
- `delete -r` MUST be narrowed only by `--include`, `--exclude`, `--older-than` or `--limit`, and a run that keeps entries on purpose exits 0 with `dirs_left` and `entries_left`
- `--hidden`, `--no-gitignore` and `--follow-symlinks` MUST NOT be sent to `delete`, because they are no-ops that only emit one diagnostic line
- `delete --backup` MUST NOT be combined with a directory root, because the verb refuses the pair before any change, so run `copy -r` first or pass `--no-backup`
- `move` MUST use `--preserve-hardlinks` when the target has a hardlink
- `move`/`copy` MUST use `--force` or `--backup` to overwrite destination
- `copy` MUST use `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr` when required
- `apply` detects unified, SEARCH/REPLACE, markdown, full. MUST use `--format auto|unified|search-replace|full|markdown` if ambiguous
- `set`/`get`/`del` ONLY TOML or JSON via dotted path. Missing get = exit 65, `del` accepts `--force-missing`
- `case` MUST `--subvert OLD NEW` with OLD different from NEW (identical ones exit 65) and MUST dry-run on large codebases
- `case` only processes a FILE: a directory exits 0 with `identifiers_total` 0 and touches nothing, so MUST pass the file
### FORBIDDEN
- NEVER pass `--confirm`/`--yes`/`-y` on delete. NEVER use set/get/del on plain text
### Correct Pattern
- The LLM MUST run `atomwrite --workspace "$WS" move src/old.rs src/new.rs` and `atomwrite --workspace "$WS" set Cargo.toml package.name x`


## Read and search
### REQUIRED
- `read`: content + checksum/size/lines/mode, `stat PATH` is an alias of `read --stat`
- `read --format raw` MUST accept several paths and a glob in one call: a relative one resolves against `--workspace`, a glob with no match exits 4 `GLOB_NO_MATCHES`, a target outside the jail exits 126 without printing a byte, and a literal metacharacter that exists on disk is read literally
- `read --format raw` MUST write the bytes to stdout and move the JSON envelope to stderr. The LLM MUST separate both channels before parsing anything
- `read --format raw` MUST honour `--max-filesize` and MUST exit 65 above it, and `--all` MUST NOT lift that gate
- A file whose image signature is damaged MUST fail `read` with exit 65 `INVALID_INPUT` instead of returning its bytes as text
- `case --limit` MUST publish `walker_stopped_early`, and `list --limit` or `hash --limit` MUST publish `files_budget_dropped`. A missing field means the walker finished without cutting
- Read flags: `--lines` (repeatable, `--range` alias), `--line`, `--enclosing`, `--symbol`, `-C/--context`, `--head`, `--tail`, `--all`, `--number`, `--no-comments`, `--format raw|ndjson`, `--bytes` (redundant with `--format raw`, also accepted by `stat`), `--grep`, `--verify-checksum` (mismatch exit 81), `--crop`, `--max-tokens`, `--pages`, `--high-res`
- `--no-comments` MUST come with `--number`, because dropping lines without renumbering would break the next `apply`, and it exits 65 without it
- `read` on a picture MUST emit one `data:` URI line on stdout, never raw bytes: the type comes from the magic bytes, the size from the 28-pixel patch rule, and `--max-tokens N`, `--crop X,Y,W,H` and `--high-res` choose the cost before the encode
- `read --stat` on a picture MUST answer width, height, format, `visual_tokens` and GIF frame count without sending a pixel
- `read doc.pdf` MUST return the text of each page, `--pages N` or `--pages A:B` selects windows repeatably, and a page with no text MUST appear in `needs_vision`: a scanned page is not an empty document
- `read doc.pdf --page N` MUST render that page as a `data:` URI, and `--dpi N` MUST set the resolution: use `--page` instead of reading a scanned page as an empty one
- `--ocr` MUST turn a picture or a rendered page into text in `content`, and the model files MUST be fetched once into the cache directory
- The visual cost of a picture MUST be computed as `ceil(width/28) * ceil(height/28)`, and `--stat` MUST report it before any encode
- `search` BEFORE mutate if target unknown, flags `-g/--include`, `--exclude`, `-C`, `-F/--fixed`, `-e/--regex`, `-w/--word`, `-i/--case-insensitive`, `-S/--smart-case`, `-c/--count`, `-l/--files`, `-m/--max-count`, `-U/--multiline`, `-P/--pcre2`, `--offset`, `--invert`, `--binary`, `--sort`, `--max-columns`, `--no-begin-end`, `--include-fifo`
- Search local default `--max-filesize` = 10 MiB
- `list`: `--include`, `--exclude`, `--long`, `--depth`, `--count-by-ext`, `--all`
- `count`: `--by-extension`, `--by-size`, `--top`
- `extract`: positionals OR local `--field` (repeatable, do NOT mix both) + `--delimiter` + `--stdin`
- `diff`: `--unified`, `--stat`, `-C`, `--algorithm myers|patience|lcs`, path `-` = stdin
### FORBIDDEN
- NEVER pass `extract --select`: `--select` is global shape. Local field flag is `--field`
- NEVER treat search zero matches as a crash
### Correct Pattern
- The LLM MUST run `atomwrite --workspace "$WS" read --head 20 path` and `printf '{"a":1}\n' | atomwrite extract --field a`


## AST
### REQUIRED
- Exit 1 on `transform` = zero matches. MUST set `-l/--language`, `$NAME` one node, `$$$ARGS` many
- Single-rule MUST pass both `-p/--pattern` and `-r/--rewrite`
- Transform flags: `--rules` (same YAML as `codemod`), `--inline-rules`, `--verify-parse`, `--include`, `--exclude`, `--dry-run`, backup opts
- MUST prefer transform over replace when the pattern is syntactic
- `scope` MUST use `--language` (alias `--lang`), `--query` or `--pattern`, `--delete`/`--action`/`--replace-with`
- Scope queries: comments, strings, fn, pub-fn, async-fn, struct, enum, trait, impl, mod, use, class, def, import, export
- `query`: `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- `outline`: repeatable `--kind`, `--positions`, `--language`
- `outline` MUST read a Markdown heading as a `section` that runs to the end of its own section, and `read --symbol NAME` MUST return that whole section
### FORBIDDEN
- NEVER apply `scope --delete` without dry-run. NEVER use textual fuzzy when transform/scope solves it
### Correct Pattern
- The LLM MUST run `atomwrite --workspace "$WS" transform --dry-run -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`


## Orchestration
### REQUIRED
- `batch`: NDJSON stdin with `op` in write/replace/delete/edit/move/copy/hash, move/copy with `"force":true` to overwrite
- MUST use `--transaction`, `--file <PATH>`, `--dry-run`, `--batch-size`, `--input-schema`, backup opts
- `batch --transaction` on failure MUST publish ONE `rollback` event with `ok` false, `code` and `exit`, and the first target already reverted. NEVER expect one error per operation
- `sparse list` accepts `--max-files`, `--max-bytes`, `--include`, `--exclude` (real globs, both applied, an invalid one exits 65) and `--all` (every entry, hidden included), `sparse read --paths-file` accepts `--head` and `--max-files`, `sparse outline` runs under budget and takes `--all` too
- `hash`, `copy -r` and `semantic-search` hide dotfiles by default, so a walk that needs a `.env` MUST pass `--hidden`; a path named in the invocation is still read without the flag
- `copy`, `move`, `delete`, `rollback`, `semantic-merge`, `config set`, `config unset`, each `batch` operation, `edit`, `replace`, `transform`, `scope`, `case`, `set`, `del`, `edit-loop` and `locale` publish `write_strategy`, and `delete` publishes `none` because it unlinks instead of writing atomically
- A preview publishes `none`, `del` on an absent key omits the field, and `batch delete` refuses to copy a copy, so a `x.bak.<stamp>` target is removed without a second backup
- `sparse list` MUST publish `files_scanned` and `path_bytes`. The LLM MUST check both to know how much of the budget the walker consumed
- `recipe list` and `recipe run --name search-replace-verify`. Built-ins `search-replace-verify` and `edit-loop-syntax-check`
- `recipe` without a limit MUST emit every `match`, one `hash`, one `plan` and two `recipe_step` with `child_summary`, `--limit N` MUST add `steps_skipped` and NEVER truncate in silence
- Recipe search/replace/hash MUST exclude `*.bak.*` by default
- `recipe run` accepts `--fuzzy-threshold`, `--include`, `--exclude`, `--pairs-file`, `--target`, `--syntax-check`
- `semantic-merge --base A --ours B --theirs C --output OUT` is line-based with markers ON (NEVER AST/embedding)
- Merge flags: `--fail-on-conflict`, `--write-conflict-markers`, `--no-conflict-markers`, `--expect-checksum`, backup opts
- `agent-surface --format json` = SSOT manifesto
- `watch` flags: `--debounce-ms`, `--max-events`, `--idle-exit-ms`, `--checksum`, `--gitignore`
- Binary feature `watch` (build with `--features watch`, else exit 78)
- `watch` ends with `watch_summary`
- `codemod --rules rules.yaml --dry-run` BEFORE apply, `rules.yaml` is a YAML list of items with `id`, `language`, `pattern` and `rewrite`, and the ast-grep format with `rule:` and `fix:` exits 65
- `codemod` MUST publish `rule_id` per rule and `by_rule_id` summing the matches. The LLM MUST read `by_rule_id` to know which rule scored before blaming the target
- `semantic-search "query" PATH --k 20 --min-score 0.05` is offline Jaccard, `--index-dir` only if local index is required, `--include-backups` includes the `*.bak.*` files the default excludes
### FORBIDDEN
- NEVER run `watch` without max-events or timeout cap
- NEVER treat semantic-search zero hits as a crash. NEVER treat semantic-merge as AST
### Correct Pattern
- The LLM MUST run `echo '{"op":"write","target":"a.txt","content":"x"}' | atomwrite --workspace "$WS" batch --transaction`


## Host
### REQUIRED
- MUST use `doctor` and `doctor --strict` when failure MUST abort the pipeline
- MUST use `locale`, `locale --set en|pt-BR`, `locale --clear`
- MUST use `commands` and `commands --include-globals`
### FORBIDDEN
- NEVER ignore `doctor --strict` failures in CI or agent pipelines
### Correct Pattern
- The LLM MUST run `atomwrite doctor --strict && atomwrite commands --include-globals`


## Utilities
### REQUIRED
- `hash` is BLAKE3, so MUST read field `checksum` and NEVER `value`
- `hash` flags: `--verify`, `--stdin`, `-r/--recursive`, `--exclude`
- `verify <PATH> <EXPECTED_HASH>`: exit 0 if match, 81 if not
- `backup`: `--retention`, `--output-dir`, `rollback --latest --verify`. Rollback flags `--timestamp`, `--latest`, `--verify`, `--dry-run`, opt-in `--backup`
- `prune-backups`: `--max-age-secs` or `--max-count <N>` and NEVER `--max-age`. A bare directory MUST be refused with 65 `INVALID_INPUT` plus a suggestion, because the sweep needs a file root
- `calc` without workspace, `regex` from ≥3 examples (`-d`, `-w`/`--words`, `-s`/`--spaces`, `-r`/`--repetitions`, `-i`/`--case-insensitive`, `--no-anchors`, `--stdin`)
- `wal-stats` inspect only, `wal-heal PATH` removes only `committed` and `aborted` journals older than `--threshold-secs` (`--max-duration-ms`, `--dry-run`) and never removes `started`. The PATH is required
- `--wal-policy always` MUST journal every write
- In `auto`, after 10 journals opened in 60 s the next writes open none, and they stay atomic
- A committed journal MUST always be removed, and `startup_wal_heal` runs only on verbs that mutate and only on the directories named in argv
- A run cancelled by SIGTERM, SIGINT or the deadline leaves no journal
- A crash from `kill -9` leaves the `started` journal, the tempfile and the `.lock` with the target intact
- `wal-heal` does NOT remove a `started` journal, so MUST check the target and handle the leftover by hand
- `edit-loop` applies N pairs in one write and is all-or-nothing, `--partial` exits 87 (`EDIT_LOOP_PARTIAL`), verbs that write require an explicit path (exit 2 without one) and the terminal event reports `target_source: "argv"`, `completions` bash/zsh/fish/elvish/powershell with `--install`
- `scratch init .atomwrite/scratch` creates staging under `$WS/.atomwrite/scratch/`, and the PATH is required (without it exit 2)
### FORBIDDEN
- NEVER use OS `/tmp` outside the jail for staging. NEVER read `value` instead of `checksum`
### Correct Pattern
- The LLM MUST run `atomwrite --workspace "$WS" hash path | jaq -r .checksum`


## Parsing errors + exit codes
### REQUIRED
- MUST check exit code BEFORE parsing stdout
- Error envelope: `ok` false, `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `suggestion_lang`, `missing_flag` (the required flag that was missing), `workspace`, `best_candidate`, `match_count`, and no `error` key, because `ok` false already carries it
- Always-emit: `--filter` MUST NOT silence `ok` false failures
- `MATCH_FAILED` exit 65: edit/fuzzy miss with `best_candidate`
- `MATCH_AMBIGUOUS`: includes `best_candidate`. MUST expand context or `--replace-all` (edit): NEVER guess
- `NO_MATCHES` exit 1: search/replace zero matches
- permanent NEVER retries
- Transient MUST backoff
- Conflict MUST re-read
- Precondition_failed MUST fix the precondition
- The LLM MUST use `suggestion`
- Exit 0 success, 1 `NO_MATCHES` or `DOCTOR_CHECKS_FAILED`, 2 argument or unsupported combination, 4 `FILE_NOT_FOUND` or `GLOB_NO_MATCHES`, 13 `PERMISSION_DENIED`, 28 disk full, 30 quota, 65 `INVALID_INPUT`/`MATCH_FAILED`/`MATCH_AMBIGUOUS`/`BINARY_FILE`/`FILE_TOO_LARGE`/ack/unknown `config` key, 69 grammar unavailable, 73 cross-device, 74 I/O error, 78 config or feature (also an unknown key inside `config.toml`), 80 rollback failed, 81 checksum, 82 drift, 83 lock timeout, 84 merge conflict, 85 FIFO, 86 device file, 87 edit-loop partial, 88 syntax, 91 strict-atomic, 92 copy-back checksum, 93 orphan journal, 124 timeout, 126 jail, 127 symlink blocked, 128 immutable file, 129 SIGHUP, 130 SIGINT or cancelled, 141 closed pipe, 143 SIGTERM, 255 internal error
- MUST map remaining codes from `exit` and `error_class` without inventing semantics
- A failed run ends in exactly ONE terminal event with `ok` false (`error_class`, `retryable` and `suggestion` travel inside it). NEVER wait for a second error line
- A cancelled run ends in `{"type":"cancelled","reason",...,"deadline_exceeded"}` with exit 124 (deadline, `deadline_exceeded` true), 130 (SIGINT) or 143 (SIGTERM), and NEVER in a late `MATCH_FAILED`
- `search`, `replace`, `transform` and `scope` report a file they could not read or write as `files_failed` and `errors[]` (first 50 entries) in the summary. Zero matches exits with the first error, matches with a failure exit 0, so MUST read `files_failed` before trusting a result
- Exit 141 means the reader closed the pipe and the output is incomplete. The process runs its cleanup first (no lock, journal or tempfile is left), and MUST re-check the state instead of assuming the work finished
- Read-modify-write verbs (`edit`, `edit-loop`, `set`, `del`, `config set`, `apply`, `write --append`) serialise on the advisory lock, `case`, `transform` and `scope` fail a file that changed since it was read with `STATE_DRIFT` (82, retryable), so MUST re-run and NEVER assume the write landed
- `apply` with a unified diff MUST compare context and removed lines with the file and allow a shift up to 64 lines, so a mismatch is `MATCH_FAILED` (65) with `best_candidate` and nothing written
- SEARCH/REPLACE with any missing block fails the whole batch the same way, and a block equivalent in two places is `MATCH_AMBIGUOUS` (65) with `match_count`
- The LLM MUST NOT expect partial application
- `get`, `set` and `del` on an invalid TOML or JSON file exit 65 `INVALID_INPUT` (line and column only). An error without a type publishes `INTERNAL_ERROR` on stdout
- A user input error (invalid regex or glob in any verb, `sparse` and `hash -r` included, AST pattern or YAML rules, `query -Q`, `read --grep`, `locale --set`, `edit --multi` ops) exits 65 `INVALID_INPUT`, 255 `INTERNAL_ERROR` means a real internal failure and MUST NOT be retried as an input fix
### FORBIDDEN
- NEVER parse stderr for errors. NEVER retry if `retryable` false
- NEVER treat a line with `ok` false as success. NEVER guess among ambiguous candidates


## Mandatory execution formulas
### REQUIRED
- The LLM MUST run optimistic lock: `CS=$(atomwrite --workspace "$WS" read file | jaq -r .checksum) && echo new | atomwrite --workspace "$WS" write --expect-checksum "$CS" --durability auto file`
- The LLM MUST run edit with match_count: `atomwrite --workspace "$WS" edit src/f.rs --old x --new y --replace-all --fuzzy auto | jaq '{match_count,indent_adjusted,checksum_after}'`
- The LLM MUST run fuzzy replace: `atomwrite --workspace "$WS" replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run old_api new_api src/`
- The LLM MUST recover exit 124 by raising `--timeout-secs` or narrowing the path
- The LLM MUST read `jaq .best_candidate` after exit 65
- The LLM MUST run recipe: `atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- The LLM MUST run sparse: `atomwrite --workspace "$WS" sparse list . --max-files 100 --max-bytes 1048576 --include '*.rs'`
- The LLM MUST run line-based merge: `atomwrite --workspace "$WS" semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict`
- The LLM MUST run offline semantic-search: `atomwrite --workspace "$WS" semantic-search "fsync rename atomic" src/ --k 20 --min-score 0.08 --index-dir .atomwrite/semantic-index`
- The LLM MUST run host readiness: `atomwrite doctor --strict && atomwrite commands --include-globals`
- The LLM MUST treat exit 143 as clean cancel and NEVER reuse tempfile
- The LLM MUST run raw-text edit: `atomwrite --workspace "$WS" apply --no-backup path <<'EOF'` with SEARCH/REPLACE blocks, never JSON built by hand nor generated by `python`
- The LLM MUST run scratch staging: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch; SCR="$WS/.atomwrite/scratch"; printf %s "$OLD" >"$SCR/old.txt"; printf %s "$NEW" >"$SCR/new.txt"; atomwrite --workspace "$WS" edit --old-file "$SCR/old.txt" --new-file "$SCR/new.txt" path`
- The LLM MUST run rewrite shrink: `atomwrite --workspace "$WS" write --durability auto --no-backup --allow-shrink "$path" < "$new"`
- The LLM MUST run agent-native shape: `atomwrite --workspace "$WS" --select ok,error,code,match_count,suggestion --filter ok=false --limit 5 edit --old x --new y path`
- The LLM MUST run extract field: `printf '{"a":1}\n' | atomwrite extract --field a`
- The LLM MUST run delete plan: `atomwrite --workspace "$WS" delete --plan -r tmp/`
- The LLM MUST run verify: `atomwrite --workspace "$WS" verify path EXPECTED_HASH`
- The LLM MUST run prune: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run src/lib.rs`, naming FILES because a directory exits 65
- The LLM MUST run edit-loop: `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace "$WS" edit-loop src/foo.rs`
- The LLM MUST run completions: `atomwrite completions bash --install`
- The LLM MUST run codemod dry-run: `atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/`
- The LLM MUST run agent-surface SSOT: `atomwrite agent-surface --format json`
- The LLM MUST run capped watch: `atomwrite --workspace "$WS" watch src/ --max-events 100 --idle-exit-ms 500 --checksum`
- The LLM MUST treat exit 78 from `watch` as a binary without the feature and reinstall with `cargo install atomwrite --features watch`
- The LLM MUST run wal: `atomwrite --workspace "$WS" wal-stats` and `atomwrite --workspace "$WS" wal-heal . --threshold-secs 3600`
- The LLM MUST run batch transaction: `echo '{"op":"write","target":"a.txt","content":"x"}' | atomwrite --workspace "$WS" batch --transaction`
- The LLM MUST run config: `atomwrite --workspace "$WS" config list` and `atomwrite --workspace "$WS" config set fuzzy.threshold 0.9 --scope workspace`
- The LLM MUST run schema discovery: `atomwrite --print-schema` lists the 55 names and `atomwrite --print-schema read` emits one
- The LLM MUST run stream shape: `atomwrite --workspace "$WS" --sort-by path-line --aggregate type:count list src/ --depth 1`
### FORBIDDEN
- NEVER skip dry-run on bulk mutation
- NEVER ignore `best_candidate` on exit 65
- NEVER run watch without a cap
