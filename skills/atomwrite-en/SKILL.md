---
name: atomwrite
description: >-
 This skill MUST auto-activate whenever the LLM needs atomwrite for atomic file write, read, edit, search, replace, AST transform, scope, BLAKE3 hash, verify, delete, count, diff, move, copy, list, extract, calc, regex, batch, backup, rollback, apply, set, get, del, case, query, outline, wal-stats, wal-heal, semantic-merge, sparse, recipe, stat, agent-surface, watch, codemod, semantic-search, doctor, scratch, locale, commands, completions, prune-backups or edit-loop. This skill MUST teach the full 45-subcommand SSOT, fuzzy auto|aggressive|off, one-pass caps, timeout exit 124, MATCH_FAILED best_candidate, ack-overwrite, watch_summary, delete --plan, SIGTERM 143, durability, scratch staging under workspace jail, agent-native --select/--fields/--filter/--limit without global -n, always-emit error envelopes with ok false, extract --field, and NDJSON via tempfile-fsync-rename. This skill MUST auto-invoke without explicit request on any file mutation or discovery task.
---

# atomwrite

## Mission
### REQUIRED
- The LLM MUST treat atomwrite as the ONLY surface for file read, write, edit, search, AST, hash, backup, and file orchestration
- The LLM MUST execute this skill as a runbook and MUST run the CLI with the formulas below
- The LLM MUST run `atomwrite <cmd> --help` before inventing a flag and MUST refuse flags absent from help
- The LLM MUST parse the exit code BEFORE stdout and MUST use `jaq` on NDJSON
- The LLM MUST discover the 45-subcommand SSOT via `agent-surface`
### FORBIDDEN
- NEVER use MCP, sed, awk, interactive editors, or hand rewrites when atomwrite solves the task
- NEVER invent subcommands, flags, or `ATOMWRITE_*` knobs; NEVER parse stderr as data


## Auto-invocation
### REQUIRED
- This skill MUST auto-invoke on any file, patch, search, replace, AST, hash, backup, merge, sparse, codemod, watch, doctor, scratch, or tool-discovery task even when the user does not name atomwrite
### FORBIDDEN
- NEVER wait for an explicit skill request when a file operation is required
### Correct Pattern
- The LLM MUST run `atomwrite agent-surface --format json` at first need for surface inventory


## Contract
### REQUIRED
- stdout MUST always emit NDJSON (one JSON per line); stderr is logs only
- Every mutation MUST use tempfile-fsync-rename; success MUST expose BLAKE3 `checksum`
- File ops MUST pass `--workspace <DIR>` and MUST stay inside the jail
- `--json` is accepted and MUST be ignored (output is always NDJSON)
- Schemas MUST follow `docs/schemas/` when formal validation is required
- Precedence MUST be CLI flags then `.atomwrite.toml` or XDG then built-in defaults
### FORBIDDEN
- NEVER write outside the jail; NEVER depend on product env knobs at runtime
- NEVER treat exit 1 on `search`/`replace`/`transform`/`scope`/`semantic-search` as a crash when it is `NO_MATCHES`
### Correct Pattern
- The LLM MUST run `atomwrite --workspace . --timeout-secs 120 --locale en --no-progress <cmd>`


## Agent safety contracts
### REQUIRED
- Large overwrite MUST pass `--ack-overwrite` (XDG `[write].confirm_large_bytes`, default 100 KiB); missing flag = exit 65
- Delete plan-only MUST use `delete --plan`; execute MUST omit `--plan` and confirm flags
- `watch` MUST end with NDJSON `watch_summary` and MUST cap with `--max-events` or timeout (idle ~500 ms)
- `semantic-merge` MUST write line-based markers by default; prefer-ours without markers MUST use `--no-conflict-markers`
- Exit 65 MUST read `best_candidate` and `match_count`; 82 MUST reload checksum; 124 MUST raise timeout or narrow path; 143 = clean SIGTERM; 126 = jail; 81 = checksum; 88 = syntax
### FORBIDDEN
- NEVER pass `delete --confirm`/`--yes`/`-y`; NEVER large-overwrite without `--ack-overwrite`
- NEVER use interactive confirm; NEVER assume empty `watch` stdout; NEVER disable markers by accident; NEVER embed MCP
### Correct Pattern
- The LLM MUST run `printf 'x' | atomwrite --workspace . write --ack-overwrite big.txt` and `atomwrite --workspace . delete --plan tmp/`


## Global flags
### REQUIRED
- MUST use `--workspace <DIR>`, `--config <PATH>`, `--timeout-secs <N>` or `--timeout <N>` (default 120 → exit 124; `0` only if intentional)
- MUST use `-j N` / `--threads N` / `--max-concurrency N` (omitted or zero = all cores with RAM cap)
- MUST use `--max-filesize <BYTES>` (global default 1 GiB), `--locale <TAG>` (NEVER global `--lang`)
- MUST use `--json-schema`, `--no-auto-heal`, `--no-progress`, `--no-gitignore`, `--hidden`, `--follow-symlinks`
- MUST use `--color auto|always|never` or `--no-color`; `-v`/`-vv`/`-vvv` or `-q`/`-qq`
- Agent-native shape is REQUIRED — `--select`/`--fields KEYS`, `--filter key=value` (AND; `!=`/`==`), `--limit N`
- Always-emit errors — `--filter` MUST NOT silence `ok` false; MUST prefer shape over jq/sed pipes
- MUST discover surface via `atomwrite agent-surface` (SSOT 45)
### FORBIDDEN
- NEVER invent flags absent from `atomwrite --help`
- NEVER use global `-n` for limit (`-n` belongs to `replace`; use `--limit`)
- NEVER dump full NDJSON when `--select` is enough
### Correct Pattern
- The LLM MUST run `atomwrite --workspace . --select ok,error,code,match_count,suggestion --filter ok=false --limit 5 <cmd>`


## Full command surface
### REQUIRED
The LLM MUST invoke each of the 45 subcommands. Typical prefix `atomwrite --workspace .` may be omitted below when obvious; the LLM MUST include `--workspace` on file ops.

1. `read` — read with checksum; `read --head 40 path`
2. `write` — create/overwrite via stdin; `echo body | write --durability auto path`
3. `edit` — surgical edit; `edit path --old a --new b --fuzzy auto`
4. `search` — locate patterns; `search TODO src/ --include '*.rs'`
5. `replace` — multi-file replace; `replace --dry-run --fuzzy auto OLD NEW src/`
6. `hash` — BLAKE3; `hash path | jaq -r .checksum`
7. `delete` — remove with plan; `delete --plan path`
8. `count` — counts; `count --by-size --top 10 src/`
9. `diff` — unified diff; `diff a.rs b.rs --unified`
10. `move` — move/rename; `move src/old.rs src/new.rs`
11. `copy` — copy; `copy --recursive --preserve src/ dest/`
12. `list` — list tree; `list --long --depth 2 src/`
13. `extract` — project NDJSON fields; `extract --field path` or `extract path`
14. `calc` — expressions/units; `atomwrite calc "2 hours to seconds"`
15. `regex` — generate regex; `atomwrite regex abc-12 xyz-99 id-7 --digits`
16. `transform` — AST rewrite; `transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
17. `scope` — grammatical actions; `scope src/ --lang rust --query comments --delete --dry-run`
18. `batch` — NDJSON manifest; `echo '{"op":"write","target":"a.txt","content":"x"}' | batch --transaction`
19. `backup` — backup with retention; `backup path --retention 3`
20. `rollback` — restore; `rollback path --latest --verify`
21. `apply` — unified/SR patch; `echo body | apply path --format full`
22. `set` — write dotted TOML/JSON; `set Cargo.toml package.name x`
23. `get` — read dotted; `get config.toml database.pool.max`
24. `del` — delete key; `del --force-missing config.toml features.x`
25. `case` — case/subvert; `case --to kebab --subvert API API --dry-run src/`
26. `query` — AST nodes; `query --kinds src/main.rs`
27. `outline` — outline; `outline --kind function_item --positions src/main.rs`
28. `wal-stats` — WAL journals; `wal-stats`
29. `wal-heal` — clean orphans; `wal-heal --threshold-secs 3600`
30. `semantic-merge` — line-based merge; `semantic-merge --base b --ours o --theirs t --output out --fail-on-conflict`
31. `sparse` — budgeted monorepo; `sparse list src/ --max-files 50 --max-bytes 524288`
32. `recipe` — recipes; `recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run`
33. `stat` — alias `read --stat`; `stat path`
34. `agent-surface` — SSOT 45 tools; `atomwrite agent-surface --format json`
35. `watch` — FS with cap; `watch src/ --max-events 100 --idle-exit-ms 500`
36. `codemod` — bulk rewrites; `codemod --rules rules.yaml --dry-run src/`
37. `semantic-search` — offline Jaccard; `semantic-search query src/ --k 20 --min-score 0.05`
38. `doctor` — host; `atomwrite doctor --strict`
39. `scratch` — jail staging; `scratch init`
40. `locale` — locale; `atomwrite locale --set en`
41. `commands` — command tree JSON; `atomwrite commands --include-globals`
42. `completions` — shell; `atomwrite completions bash --install`
43. `prune-backups` — prune; `prune-backups --max-age-secs 86400 --dry-run .`
44. `edit-loop` — N pairs one write; `echo '[{"old":"a","new":"b"}]' | edit-loop path`
45. `verify` — expected hash; `verify path EXPECTED_HASH`
### FORBIDDEN
- NEVER invent a 46th subcommand; NEVER skip `agent-surface` when surface is unknown
- NEVER omit `--workspace` on file ops inside the jail


## Fuzzy
### REQUIRED
- Fuzzy on `edit`/`replace` defaults to `auto`; values ONLY `--fuzzy auto|aggressive|off`
- `--fuzzy off` = exact-only; edit miss = exit 65 `MATCH_FAILED`; tree replace miss often exit 1 `NO_MATCHES`
- MUST use `--fuzzy-threshold` 0.0–1.0; with `--regex` on replace fuzzy MUST be ignored
- Multi-apply MUST be one-pass L→R on original; NEW embeds OLD → force 1 occurrence; without `--max-replacements` multi default = 1; hard cap 10000
- Caps — pattern 64 KiB, Levenshtein 8192, windows 4096, growth max 4x or +16 MiB
- Normalized strategies (`line_trimmed`, `whitespace_normalized`, `punctuation_normalized`, `indent_flexible`, `trimmed_boundary`, `escape_normalized`) emit `similarity` null — NEVER treat fake 1.0 as byte equality unless strategy is `exact`
- Soft-floor `context_aware` — XDG `[fuzzy].soft_floor_delta` default 0.05; effective accept = threshold OR threshold minus soft_floor_delta
- `block_anchor` dual-gate (edges + middle ratio)
- `MATCH_AMBIGUOUS` MUST include `best_candidate`; MUST expand `--old` or use `--replace-all` (edit) or narrow scope — NEVER guess
- Fuzzy is NOT AST — MUST use `transform`/`scope`/`codemod` for semantics
### FORBIDDEN
- NEVER expect re-search of text inserted by the same fuzzy replace; NEVER use `--replace-all` on `replace`
- NEVER set `--timeout-secs 0` on fuzzy monorepo unless intentional no-deadline job
- NEVER run tree-wide `--fuzzy aggressive` without `--dry-run`; NEVER pass invented `--strategy` on edit
### Correct Pattern
- The LLM MUST run `atomwrite --workspace . replace --dry-run --fuzzy auto --max-replacements 1 OLD NEW src/`


## Edit hierarchy + execution flow
### REQUIRED
- Order precise to blunt — `search` → `transform` → `scope` → `codemod` → `replace` → `edit` → `write`
- `write` ONLY to create or intentionally overwrite a full file
- MUST locate with `search` or `sparse` before mutate; MUST capture checksum with `read` under concurrency
- MUST run `--dry-run` before destructive bulk mutation; MUST use `--expect-checksum` for concurrent overwrite
- MUST validate `checksum`, `match_count`, `indent_adjusted`, `platform` when present
- MUST handle failures via `error_class` and `suggestion`; retry ONLY if `retryable` true
### FORBIDDEN
- NEVER full-file `write` when `edit`/`replace`/`transform` solves it
- NEVER ignore non-zero exit except documented zero-match
### Correct Pattern
- The LLM MUST run `atomwrite --workspace . search target src/` and only then mutate


## Backup and configuration
### REQUIRED
- Backup default true on mutators via `--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`
- Default backup MUST create adjacent `.bak.<timestamp>` and auto-remove on success; on failure MUST remain
- `delete` and `replace` MUST keep `.bak` on success
- `rollback` MUST use explicit `--backup` for opt-in backup of restore target
- `move` and `copy` MUST pass `--force` or `--backup` to overwrite destination
- Backup MUST use `reflink_or_copy` and NEVER hardlink the live file
- `.atomwrite.toml` governs `[defaults]` and `[fuzzy]` (`auto|aggressive|off`); CLI flags win over TOML
### FORBIDDEN
- NEVER pass both `--backup` and `--no-backup` (exit 2)
- NEVER assume TOML `backup = true` alone authorizes move/copy overwrite
### Correct Pattern
- The LLM MUST set `[fuzzy] mode = "auto"` and `threshold = 0.85` in `.atomwrite.toml`


## File mutation write/edit/replace
### REQUIRED
- `write` MUST send content via stdin; MUST use `--expect-checksum`, `--durability full|fast|auto` (default `auto`)
- Rewrite that shrinks >50% MUST use `--allow-shrink`
- MUST use `--allow-empty-stdin`, `--no-checksum-when-empty`, `--syntax-check` (exit 88), `--ack-overwrite`
- Write flags — `--append`, `--prepend`, `--max-size`, `--line-ending`, `--preserve-timestamps`, `--require-backup`, `--auto-rotate`, `--require-large-ack`, `--risk-threshold`, `--wal-policy`, `--dry-run`, backup opts
- SIGTERM during write MUST clean tempfile and return exit 143
- `edit` MUST use `--old`/`--new` (repeatable pairs); multi-pair emits `pairs_total`/`pair_results`; failure aborts batch unless `--partial`
- Multi-occurrence on edit MUST use `--replace-all` and MUST emit `match_count`; re-indented emits `indent_adjusted` true
- Position flags — `--after-line`, `--before-line`, `--range`, `--delete-range`, `--after-match`, `--before-match`, `--between`, `--multi`
- Patches with `//`, jq, quotes, multiline MUST use `--old-file`/`--new-file` staged under `$WS/.atomwrite/scratch/` after `scratch init`
- MUST use `--expect-checksum` and `--allow-sequential-drift` only in controlled pipelines
- Terminal stdin on insert MUST fail exit 65 without hanging
- Multi-root — if target is under home skills, MUST pass `--workspace` at the allowed root
- `replace` exit 1 = zero matches (NOT a crash); MUST ALWAYS `--dry-run`/`--preview` before bulk apply
- Fuzzy multi on replace MUST use only `--max-replacements` (`-n`)
- Replace flags — `--regex`, `-w`, `-F`, `-g/--include`, `--exclude`, `--expect-checksum`, `--preserve-timestamps`, `--preserve-case`, backup opts
### FORBIDDEN
- NEVER pipe edit to `jaq` without `${PIPESTATUS[0]}`; NEVER pass `--strategy` on edit
- NEVER put old/new-file under OS `/tmp` outside the jail
- NEVER pass unquoted `$VAR` when literal dollar must survive — MUST quote `"$VAR"`
- NEVER treat fuzzy as AST; NEVER shell-quote fragile patches as `--old`/`--new` when scratch exists
- NEVER use `--replace-all` on replace; NEVER skip dry-run on large trees; NEVER shrink >50% without `--allow-shrink`
### Correct Pattern
- The LLM MUST run staging — `atomwrite --workspace "$WS" scratch init; SCR="$WS/.atomwrite/scratch"; printf %s "$OLD" >"$SCR/old.txt"; printf %s "$NEW" >"$SCR/new.txt"; atomwrite --workspace "$WS" edit --old-file "$SCR/old.txt" --new-file "$SCR/new.txt" path`
- The LLM MUST run `atomwrite --workspace "$WS" write --durability auto --no-backup --allow-shrink "$path" < "$new"`


## Path and structured
### REQUIRED
- `delete` MUST keep backup and preserve `.bak` on success; age via `--older-than`; recursive `-r`; globs `-g/--include`; plan via `--plan`
- `move`/`copy` MUST use `--force` or `--backup` to overwrite destination
- `copy` MUST use `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr` when required
- `apply` detects unified, SEARCH/REPLACE, markdown, full; MUST use `--format auto|unified|search-replace|full|markdown` if ambiguous
- `set`/`get`/`del` ONLY TOML or JSON via dotted path; missing get = exit 65; `del` accepts `--force-missing`
- `case` MUST `--subvert OLD NEW` and MUST dry-run on large codebases
### FORBIDDEN
- NEVER pass `--confirm`/`--yes`/`-y` on delete; NEVER use set/get/del on plain text
### Correct Pattern
- The LLM MUST run `atomwrite --workspace . move src/old.rs src/new.rs` and `atomwrite --workspace . set Cargo.toml package.name x`


## Read and search
### REQUIRED
- `read` — content + checksum/size/lines/mode; `stat PATH` = alias of `read --stat`
- Read flags — `--lines`, `--line`, `-C/--context`, `--head`, `--tail`, `--format raw|ndjson`, `--grep`, `--verify-checksum` (mismatch exit 81)
- `search` BEFORE mutate if target unknown; flags `-g/--include`, `--exclude`, `-C`, `-F`, `-e/--regex`, `-w`, `-i`, `-S`, `-c`, `-l`, `-m`, `-U`, `-P`, `--invert`, `--sort`, `--max-columns`, `--no-begin-end`, `--include-fifo`
- Search local default `--max-filesize` = 10 MiB
- `list` — `--include`, `--exclude`, `--long`, `--depth`, `--count-by-ext`, `--all`
- `count` — `--by-extension`, `--by-size`, `--top`
- `extract` — positionals OR local `--field` (repeatable; do NOT mix both) + `--delimiter` + `--stdin`
- `diff` — `--unified`, `--stat`, `-C`, `--algorithm myers|patience|lcs`; path `-` = stdin
### FORBIDDEN
- NEVER pass `extract --select` — `--select` is global shape; local field flag is `--field`
- NEVER treat search zero matches as a crash
### Correct Pattern
- The LLM MUST run `atomwrite --workspace . read --head 20 path` and `printf '{"a":1}\n' | atomwrite extract --field a`


## AST
### REQUIRED
- Exit 1 on `transform` = zero matches; MUST set `-l/--language`; `$NAME` one node, `$$$ARGS` many
- Single-rule MUST pass both `-p/--pattern` and `-r/--rewrite`
- Transform flags — `--rules`, `--inline-rules`, `--verify-parse`, `--include`, `--exclude`, `--dry-run`, backup opts
- MUST prefer transform over replace when the pattern is syntactic
- `scope` MUST use `--language` (alias `--lang`); `--query` or `--pattern`; `--delete`/`--action`/`--replace-with`
- Scope queries — comments, strings, fn, pub-fn, async-fn, struct, enum, trait, impl, mod, use, class, def, import, export
- `query` — `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- `outline` — repeatable `--kind`, `--positions`, `--language`
### FORBIDDEN
- NEVER apply `scope --delete` without dry-run; NEVER use textual fuzzy when transform/scope solves it
### Correct Pattern
- The LLM MUST run `atomwrite --workspace . transform --dry-run -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`


## Orchestration
### REQUIRED
- `batch` — NDJSON stdin with `op` in write/replace/delete/edit/move/copy/hash; move/copy with `"force":true` to overwrite
- MUST use `--transaction`, `--file <PATH>`, `--dry-run`, `--batch-size`, `--input-schema`, backup opts
- `sparse list` — `--max-files`, `--max-bytes`; `sparse read --paths-file` — `--head`, `--max-files`; `sparse outline` under budget
- `recipe list` and `recipe run --name search-replace-verify`; built-ins `search-replace-verify` and `edit-loop-syntax-check`
- Recipe search/replace/hash MUST exclude `*.bak.*` by default
- `recipe run` accepts `--fuzzy-threshold`, `--include`, `--exclude`, `--pairs-file`, `--target`, `--syntax-check`
- `semantic-merge --base A --ours B --theirs C --output OUT` is line-based with markers ON (NEVER AST/embedding)
- Merge flags — `--fail-on-conflict`, `--write-conflict-markers`, `--no-conflict-markers`, `--expect-checksum`, backup opts
- `agent-surface --format json` = SSOT manifesto
- `watch` — `--debounce-ms`, `--max-events`, `--idle-exit-ms`, `--checksum`, `--gitignore`; binary feature `watch`; ends with `watch_summary`
- `codemod --rules rules.yaml --dry-run` BEFORE apply
- `semantic-search "query" PATH --k 20 --min-score 0.05` is offline Jaccard; `--index-dir` only if local index is required
### FORBIDDEN
- NEVER run `watch` without max-events or timeout cap
- NEVER treat semantic-search zero hits as a crash; NEVER treat semantic-merge as AST
### Correct Pattern
- The LLM MUST run `echo '{"op":"write","target":"a.txt","content":"x"}' | atomwrite --workspace . batch --transaction`


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
- `hash` — BLAKE3; read field `checksum` and NEVER `value`; flags `--verify`, `--stdin`, `-r/--recursive`, `--exclude`
- `verify <PATH> <EXPECTED_HASH>` — exit 0 if match, 81 if not
- `backup` — `--retention`, `--output-dir`; `rollback --latest --verify`; rollback flags `--timestamp`, `--latest`, `--verify`, `--dry-run`, opt-in `--backup`
- `prune-backups` — `--max-age-secs` and NEVER `--max-age`
- `calc` without workspace; `regex` from ≥3 examples (`-d`, `-w`, `-s`, `-r`, `-i`, `--no-anchors`, `--stdin`)
- `wal-stats` inspect only; `wal-heal` removes orphans (`--threshold-secs`, `--max-duration-ms`)
- `edit-loop` applies N pairs in one write; `completions` bash/zsh/fish/elvish/powershell with `--install`
- `scratch init` creates staging under `$WS/.atomwrite/scratch/`
### FORBIDDEN
- NEVER use OS `/tmp` outside the jail for staging; NEVER read `value` instead of `checksum`
### Correct Pattern
- The LLM MUST run `atomwrite --workspace . hash path | jaq -r .checksum` and `atomwrite --workspace . scratch init`


## Parsing errors + exit codes
### REQUIRED
- MUST check exit code BEFORE parsing stdout
- Error envelope — `error` true, `ok` false, `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `workspace`, `best_candidate`, `match_count`
- Always-emit — `--filter` MUST NOT silence `ok` false failures
- `MATCH_FAILED` exit 65 — edit/fuzzy miss with `best_candidate`
- `MATCH_AMBIGUOUS` — includes `best_candidate`; MUST expand context or `--replace-all` (edit) — NEVER guess
- `NO_MATCHES` exit 1 — search/replace zero matches
- permanent NEVER retries; transient MUST backoff; conflict MUST re-read; precondition_failed MUST fix precondition; MUST use `suggestion`
- Exit 0 success; 1 `NO_MATCHES`; 2 invalid arg; 4 not found; 65 input/`MATCH_FAILED`/ack; 78 config/feature; 81 checksum; 82 drift; 88 syntax; 124 timeout; 126 jail; 143 SIGTERM
- MUST map remaining codes from `exit` and `error_class` without inventing semantics
### FORBIDDEN
- NEVER parse stderr for errors; NEVER retry if `retryable` false
- NEVER treat a line with `ok` false as success; NEVER guess among ambiguous candidates


## Mandatory execution formulas
### REQUIRED
- The LLM MUST run optimistic lock — `CS=$(atomwrite --workspace . read file | jaq -r .checksum) && echo new | atomwrite --workspace . write --expect-checksum "$CS" --durability auto file`
- The LLM MUST run edit with match_count — `atomwrite --workspace . edit src/f.rs --old x --new y --replace-all --fuzzy auto | jaq '{match_count,indent_adjusted,checksum}'`
- The LLM MUST run fuzzy replace — `atomwrite --workspace . replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run old_api new_api src/`
- The LLM MUST recover exit 124 by raising `--timeout-secs` or narrowing the path
- The LLM MUST read `jaq .best_candidate` after exit 65
- The LLM MUST run recipe — `atomwrite --workspace . recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- The LLM MUST run sparse — `atomwrite --workspace . sparse list . --max-files 100 --max-bytes 1048576 --include '*.rs'`
- The LLM MUST run line-based merge — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict`
- The LLM MUST run offline semantic-search — `atomwrite --workspace . semantic-search "fsync rename atomic" src/ --k 20 --min-score 0.08 --index-dir .atomwrite/semantic-index`
- The LLM MUST run host readiness — `atomwrite doctor --strict && atomwrite commands --include-globals`
- The LLM MUST treat exit 143 as clean cancel and NEVER reuse tempfile
- The LLM MUST run scratch staging — `atomwrite --workspace "$WS" scratch init; SCR="$WS/.atomwrite/scratch"; printf %s "$OLD" >"$SCR/old.txt"; printf %s "$NEW" >"$SCR/new.txt"; atomwrite --workspace "$WS" edit --old-file "$SCR/old.txt" --new-file "$SCR/new.txt" path`
- The LLM MUST run rewrite shrink — `atomwrite --workspace "$WS" write --durability auto --no-backup --allow-shrink "$path" < "$new"`
- The LLM MUST run agent-native shape — `atomwrite --workspace . --select ok,error,code,match_count,suggestion --filter ok=false --limit 5 edit --old x --new y path`
- The LLM MUST run extract field — `printf '{"a":1}\n' | atomwrite extract --field a`
- The LLM MUST run delete plan — `atomwrite --workspace . delete --plan tmp/`
- The LLM MUST run verify — `atomwrite --workspace . verify path EXPECTED_HASH`
- The LLM MUST run prune — `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .`
- The LLM MUST run edit-loop — `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- The LLM MUST run completions — `atomwrite completions bash --install`
- The LLM MUST run codemod dry-run — `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- The LLM MUST run agent-surface SSOT — `atomwrite agent-surface --format json`
- The LLM MUST run capped watch — `atomwrite --workspace . watch src/ --max-events 100 --idle-exit-ms 500 --checksum`
- The LLM MUST run wal — `atomwrite --workspace . wal-stats` and `atomwrite --workspace . wal-heal --threshold-secs 3600`
- The LLM MUST run batch transaction — `echo '{"op":"write","target":"a.txt","content":"x"}' | atomwrite --workspace . batch --transaction`
### FORBIDDEN
- NEVER skip dry-run on bulk mutation; NEVER ignore `best_candidate` on exit 65; NEVER run watch without a cap
