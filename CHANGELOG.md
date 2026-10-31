[Leia em Portugues](CHANGELOG.pt-BR.md)


# Changelog

- All notable changes to this project are documented in this file
- Format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/)
- Versioning follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)


## [0.1.37] - 2026-10-31
- This entry merges the 2026-10-15 draft of `0.1.37` and the work of 2026-10-28 through 2026-10-31, because `0.1.37` was never published: crates.io carries `0.1.36`, and the higher numbers this tree briefly declared were folded back into `0.1.37`
- The draft keeps its own sub-sections, labelled by the date it was written, so a reader can tell which half a line came from

### Changed — one schema per verb, describing the whole stream (2026-10-31)
- `--print-schema recipe` served only `RecipeResult` and now serves `RecipeOutput`: eleven verb branches, every reemitted child record tagged with `step`, plus the error envelope
- `--print-schema codemod` served only `CodemodSummaryEvent` and now serves `CodemodOutput`: seven verb branches, every child record tagged with `rule_id`, plus the error envelope
- `--print-schema batch` served only `BatchSummary` and now serves `BatchOutput`: `batch_op`, `progress`, `rollback`, the terminal summary and the `--input-schema` manifest, plus the error envelope
- `search`, `replace`, `list` and `backup` each served two names and now serve one: the verb name carries the records and the terminal summary the `*-summary` names used to hold
- The names `search-summary`, `replace-summary`, `backup-summary` and `list-summary` are no longer served, so `--print-schema` lists 55 names backed by 54 files
- `semantic-search`, `sparse`, `semantic-merge`, `case` and `watch` went from a single branch to the union of the verb plus the error envelope
- The error envelope is a branch of every union, so a run that fails before the first record validates against the verb's own schema
- `diff` gained the error envelope as a fifth `oneOf` branch, exclusive because every record of the verb lacks `error`
- `close_terminal_schema` leaves a union root untouched, which also fixed `prune-backups`, whose root carried `additionalProperties: false` and `required: ["ok"]` beside the `anyOf` and rejected every per-entry record
- `union_of`, `tagged_record`, `terminal_record` and `error_branch` replace the three hand-written copies of the `$defs` loop
- `README.md`, `README.pt-BR.md`, `docs/HOW_TO_USE.md`, `docs/HOW_TO_USE.pt-BR.md`, `llms.txt`, `llms.pt-BR.txt`, `llms-full.txt`, `llms-full.pt-BR.txt` and the two `skills/atomwrite-*/SKILL.md` now say 55 names backed by 54 files, and the skills describe the union instead of the removed summary names
- Eleven further places kept the old total and are fixed too: `docs/COOKBOOK.pt-BR.md`, `docs/INSTALL.md`, `docs/INSTALL.pt-BR.md`, `docs/MIGRATION.md`, `docs/MIGRATION.pt-BR.md`, `llms.pt-BR.txt`, `llms-full.pt-BR.txt` and `docs/schemas/README.md`
- `watch` is proven by execution with the opt-in feature: the build with `--features watch` emitted one `watch` and one `watch_summary`, both valid against the served schema

### Changed — locks, drift, workspace origin and the widened docs gate (2026-10-31)
- `replace` compares `--expect-checksum` under the lock, so the window between the comparison and the write is closed; `--dry-run` and `--preview` still compare the content they read, because they never take the lock
- `edit-loop` hands the checksum it read under the lock to `atomic_write_based`, so a concurrent edit ends as `STATE_DRIFT` (exit 82) instead of a lost update
- The lock sidecar name becomes a short BLAKE3 digest when `.<name>.atomwrite.lock` would pass 255 bytes, so a 240-byte target name locks instead of failing with `ENAMETOOLONG`
- The lock uses `std::fs::File::try_lock` on every platform, which is `flock` on Unix and `LockFileEx` on Windows, so the Windows build no longer ships a no-op lock
- `batch --transaction` takes every target lock, sorted by path, before the snapshot and holds them until commit or rollback, so two batches with the same targets wait instead of interleaving and cannot deadlock
- `wal-heal` removes a `Started` journal when the owning PID no longer exists and publishes `dead_owner_removed`, so a crash journal stops surviving forever while a slow live writer keeps its own
- Terminal envelopes carry `workspace_source` (`argv` or `cwd`), so a run that took its root from the process working directory says so instead of hiding the ambient authority
- The hardcode guard scans `build.rs` and `xtask/src` as well, with a reasoned exception for the `git` and `cargo` calls the build tooling owns
- The docs gate also checks `N names` and `N files` claims, not only `N schemas`
- `resolve_workspace`, `rename_path` and the fsync helpers raise typed errors instead of `anyhow!`, so an unreadable working directory or a permission denial keeps exit 13 or 74 instead of falling to 255

### Fixed — `replace` stops leaving a `.bak` behind (2026-10-31)
- `replace` derived retention from the effective `backup` value, and `[defaults].backup` resolves to true, so a plain run left a `.bak.<timestamp>` sibling that nobody asked for
- Retention now follows `--keep-backup` alone, as ADR-0038 decides, so `--backup` still creates the crash net and the successful write removes it
- The `GAP-105` behavior is superseded: making `--backup` imply `--keep-backup` turned one flag into two and reproduced exactly the Alternative 4 that ADR-0038 rejected
- `tests/cli_v0123_backup_default.rs` still demanded a `.bak` after `replace` and was the only red test of the suite, so it now asserts zero like its `write` and `edit` siblings
- `docs/AGENTS.md` and `docs/AGENTS.pt-BR.md` mark the v0.1.25 retention claim as superseded, and `gaps.md` recounts the W44 items the code already closed
- A scan with `fd -I -H -p '\.(bak|orig|old)\.'` found 19 siblings under `docs/`, and all 19 are deleted

### Fixed — cuts that stayed silent (2026-10-31)
- A record cut by `--truncate-content` or `--max-output-bytes` now stamps `truncated: true` on itself in every verb, and the writer adds `truncated_records` to the terminal, so a shortened field is never read as the source
- `search --max-count` publishes `matches_suppressed`
- The search asks for one match beyond the ceiling, so the number has a floor instead of silence
- `replace --max-replacements` publishes `replacements_suppressed`, because the helper already counted every occurrence and returned only the applied ones
- `semantic-search --min-score` publishes `below_min_score`, so an empty answer says whether the corpus was empty or the floor was high
- `--filter` and an empty `--select` publish `records_suppressed` on the terminal instead of returning `None` in silence
- `FlushOutcome.dropped` reaches the terminal as `records_suppressed`, covering the dedupe, the ungrouped aggregate and the block cut by `--limit`
- `--dedupe path` keeps one match per file
- The `begin` and `end` frames no longer enter the dedupe identity and no longer consume the key
- The writer cut by `--limit` charges the same `files_budget_dropped` counter as `cap_collected`, and the terminal stamps the tally when the verb does not declare it
- `replace`, `transform`, `scope` and `case` publish `files_planned` in `--dry-run` and `--preview`, while `files_modified` stays zero without a real write
- `outline` publishes `items_omitted` for the `--limit` cut, and `items_truncated` now says it counts a cut signature, never a missing item
- `--print-schema` declares `records_suppressed`, `files_budget_dropped`, `files_planned`, `truncated` and `truncated_records`, so a validator stops seeing undeclared keys
- `docs/schemas/README.md` states 54 files with no orphans
- `config` reads 37 keys in every document, and `config doctor` appears in every subcommand list
- The two families of ADR-0077 are cited without `count` and `list`, whose `ok:false` with exit `13` was measured

### Fixed — closing the tail (2026-10-31)
- `--max-count` is documented as a ceiling on records and not on reading, because proving that a file held more costs at most one match read past the ceiling in that file
- The probe has no measurable cost: `search 'let ' . -c -m 1 --filter type=summary` runs in 0.01 s over 603 files and the same run without `-m` runs in 0.01 s
- `records_suppressed` is documented as a count of records and never of files, so it does not compare with `files_matched`, because one file can contribute several records
- Measured: `search 'fn ' src/commands/search.rs --max-count 1 --filter type=summary` publishes `records_suppressed: 3` against `files_matched: 1`, and the three are one match plus the two frames the filter refused
- `files_budget_dropped` names the unit the verb produces: files for a verb that walks paths, records for a verb that streams them
- `recipe` and `codemod` forward the captured child records without charging `--limit` a second time, so the budget counts each record once
- The child `summary` is folded into `recipe_step.child_summary` and into `codemod_summary.transform_summary`, so each run ends in exactly one terminal record
- `recipe` forwards the replace step records, which used to be read for the checksum and thrown away
- `recipe` marks the remaining steps `skipped` when `--limit` stops the stream, and `recipe_result` carries `steps_skipped` with `ok: true`
- Measured: `recipe run search-replace-verify ... --dry-run --limit 20` exits 0 with 20 `match` records and two skipped steps, where it used to exit 1 with `code: "NO_MATCHES"`
- The `recipe_result` terminal goes through `write_terminal`, so the budget can no longer remove the record that says the recipe finished
- `codemod` publishes the child failure envelope with the campaign `rule_id` and returns the error tagged `VerdictPublished`, so the failure carries one terminal and not two
- `batch` has no child process, so it keeps publishing `batch_op` and a single `summary`, and the earlier claim that it re-emitted child events was wrong for it

### Fixed — answer family, target stamps and summary honesty (2026-10-30)
- `search --target files` publishes `files_visited` as the files the walk examined and `files_matched` as the names that matched, so the page omitted by `--offset` and `--limit` is visible instead of disguised as the size of the walk
- `hash --stdin` stamps `target_resolved: "-"` with `target_source: "stdin"`, and every schema carrying the envelope declares the third value
- `sparse` and `semantic-search` keep the answer they published and exit 0 with `partial: true`, instead of discarding it behind exit 13
- `scope` keeps a rewrite that landed and reports the failed write in `errors[]` instead of failing the run
- `backup` keeps its terminal summary and counts `files_failed` when a source cannot be copied, and `prune-backups` exits with the removal that left a backup on disk
- `copy` always publishes its summary, and `files_visited` counts the sources it examined rather than the copies that landed
- `stat`, `verify`, `get`, `query`, `outline`, `sparse`, `semantic-search`, `watch`, `extract-artifact` and `diff` with a `-` operand now carry `target_resolved` and `target_source`, and the error envelope names the subject with the same origin the success path uses
- `summary.errors_max_entries` makes the `errors[]` cap configurable from 1 to 1000, 50 by default, with `files_failed` still carrying the full count
- `files_visited`, `files_modified`, `total_matches` and `list`'s `total_bytes` now describe every verb that publishes them
- `error-output.schema.json` declares `target_resolved` and `target_source`, which the writer already stamped on every failing run, so a validator no longer sees undeclared keys

### Fixed — counters before the cut, truncation visible and closed schemas (2026-10-30)
- `semantic-search` publishes `files_scanned` and `hits_before_cut`, so the `--k` cut no longer needs a second run of the whole tree
- `search` publishes `offset_skipped`, which the tally already counted and never serialized
- `scope` publishes `files_read` and `total_bytes` in its summary, in dry-run and in rewrite alike
- `hash` publishes `total_bytes`, and `--stdin` measures the stream through `hash_reader_with_len`
- `copy` publishes `total_bytes` of what it copied
- `cap_collected` counts what `--limit` dropped, and every verb that calls it publishes `files_budget_dropped`
- `files_skipped` no longer serializes as `null` on a verb that skipped nothing
- `count` publishes `total` in all three modes, honours `--top` in `--by-extension`, and declares `items_omitted`
- `sparse` publishes `files_scanned`, `path_bytes`, `files_unparsed` and `files_budget_dropped`
- `delete` publishes `total_bytes` from the counter it already accumulated, and serves its summary in `--print-schema`
- `transform` publishes `total_bytes` and serves its summary in `--print-schema`
- `diff` declares the `change` and `summary` events it writes, and stamps `timed_out` when `similar` spends the similarity budget
- `search --target files` counts `files_matched` as the files that contributed a published name
- `read` drops `content_b64`, which no producer ever filled
- `outline` and `sparse outline` declare a truncated signature through `signature_truncated` and `items_truncated`
- `query` declares `text_truncated` per node and `nodes_truncated` in the summary
- `extract-artifact` declares `truncated_parts` for a member cut by the zip-bomb ceiling, and the defence is unchanged
- `backup` counts an origin that was not a regular file as `files_skipped`, and `prune-backups` counts a missing target as `targets_missing`
- `replace` publishes `binary_rejects`, so a rejected binary stops hiding inside `files_skipped`, and `files_total` carries the eligible count whenever the progress precount runs
- The `required` list of the closed schemas matches what each verb always emits, for `list-summary`, `replace-summary` and `delete-output`
- `docs/schemas/README.md` states 59 names served by 58 files with no orphans, in all four places that state a count
- The six documents cite the two families of ADR-0077 instead of repeating a drifted list
- The four exit-code counts read 41 entries and 33 distinct numbers, the thirteen README fields are `config` keys proved by `config get`, and `config doctor` is listed in the six subcommand lists
- The schema sentence says 58 files serving 59 names, with `read` and `stat` sharing one
- `delete --recursive` stops using `remove_dir_all`, so it removes only the directories the run emptied, and a file kept out by `--include`, `--exclude`, the hidden filter or an ignore file survives instead of vanishing while the summary counted zero
- `watch` publishes its terminal `watch_summary` on the success path too, proved with `reason: idle` and exit 0 in an empty directory, so the stamp no longer has evidence only on the error envelope
- `transform` accepts 29 built-in languages, proved by the refusal of `-l fortran` with exit 65 naming all 29, and the drift that claimed 306 is corrected here and in both `docs/AGENTS` files
- Both changelogs drop every bold marker (183 pairs in this file and 157 in the Portuguese one), so they follow the same bullet format as the rest of the documentation
- The slim build is measured rather than inherited: `cargo build --release --no-default-features --features core` and the result is recorded in `README`, `ARCHITECTURE`, `TESTING` and `COOKBOOK`
- `gaps.md` closes this round, and both changelogs list the same parts

### Added
- `read` takes one or more operands, and `--format raw` concatenates them in argv order with no header, no separator and no added newline, the way `cat` does
- An operand that carries `*`, `?` or `[` is expanded by the binary itself against one level of its directory: byte order, hidden names skipped, no descent into subdirectories
- `read --format raw` of several files streams block by block, so the peak memory stays at one 64 KiB block instead of the sum of the file sizes
- The concatenation advances a `CancelCheck::for_bytes`, so `--timeout-secs` cuts a multi-file read in the middle instead of after the last file
- `--lines`, `--line`, `--context`, `--head`, `--tail`, `--grep`, `--stat` and `--verify-checksum` name themselves in a clear `INVALID_INPUT` when the call has more than one target
- `risk_assessment` on `edit`, `edit-loop`, `replace`, `case`, `transform` and `codemod`: the content-pattern guard now runs in every verb that writes, with the same `risk_level` and `guard_triggered` values `write` already published (G-RISK-1)
- The same field on the three plan records, so a dry-run carries the diagnostic the write would: `DryRunPlan` (`write`, `edit`, `replace`, `set`, `del`, `delete`, `semantic-merge`, `wal-stats`, `wal-heal`), `ReplacePreview` and `ScopeResult`
- `scope` calls `assess_content_risk` at all now: it wrote files with no risk diagnostic, while every other writing verb published one
- `rule_ids_from_file`, `load_rules_file` and `parse_rules` in `transform_multi.inc.rs`: one manifest reader shared by `transform --rules` and `codemod`, so the labels of a campaign and the rules actually applied cannot drift
- `codemod` labels a rule without `id` as `rule-N` by its 1-based position, instead of dropping it from the list
- The size barriers stay out of those six verbs on purpose: they rewrite a region, so a shorter result is the operation working, and applying the shrink barrier there blocked `edit --delete-range` outright
- `Kind::List` in the config key table, so `write.content_risk_patterns` is reachable with `config get` and `config set` (36 keys, G-CONFIG-2)
- Exit status `69` `GRAMMAR_UNAVAILABLE` with class `precondition_failed`, raised when the tree-sitter grammar for a language is missing and the download does not happen
- `src/grammar.rs`: one function decides whether a pack error is a wrong name (`INVALID_INPUT`, 65) or a grammar this host cannot provide (`GRAMMAR_UNAVAILABLE`, 69), and `outline`, `query` and `syntax_check` all call it
- `grammar_cache` check on `doctor`: reports how many of the 23 grammars the AST verbs can need are already on disk, with status `pass` when all are cached, `info` when some are missing (the pack downloads them on first use) and `skipped` when the build has no `ast` feature
- `src/constants.rs::EXTENSION_LANGUAGES`: one table maps file extension to tree-sitter language, read by `syntax_check` and by the doctor check, so the check measures the languages the verbs actually ask for

### Changed
- `search` publishes `target_resolved` and `target_source` in every terminal envelope: a root typed on the line is reported as `argv`, and an omitted root is reported as `workspace` against the workspace root, where the summary used to carry neither key
- `read --format raw` reports its failures on stderr instead of writing the NDJSON error envelope on stdout: a JSON record on the byte channel is not something a `cat` replacement may emit
- A missing file, a directory or a glob that matched nothing prints one line on stderr and the other targets still reach stdout, with the FIRST failure's semantic code as the status
- An absolute operand is judged by the part that sits under `--workspace`: `/w/*.rs` used to be refused with 65 while `/w/a.rs` worked, because the level limit counted the absolute prefix as if the caller had asked for depth
- A pattern that matches nothing reports `GLOB_NO_MATCHES` with `no file matches the pattern: …` and keeps exit 4, and several targets on `stat` name the verb, get a hint about the verb itself instead of one about flags nobody passed, and `stat --format raw` concatenates instead of being vetoed by the verb's own `--stat` flag
- `[profile.dev] opt-level = 1`: the fuzzy cascade of `replace --fuzzy auto` over the same 15 MB corpus went from 18.5 s to 4.2 s on this host, measured before and after
- `tree-sitter-language-pack` 1.8.1 to 1.22.1, `ast-grep-core` and `ast-grep-language` 0.43 to 0.51, and `tree-sitter` 0.26 to 0.27: the pack 1.19 and later requires `tree-sitter ^0.27`, and `ast-grep` 0.43 requires `^0.26`, so the three move together or not at all
- `zstd` moves from 0.13.3 to 0.14.3 through that pack, and with it `zstd-safe` 7.2.4 to 8.2.0 and `zstd-sys` 2.0.16 to 2.1.2, which clears the four vulnerabilities and ten unsound warnings of RUSTSEC-2026-0391, 0395 to 0406 and 0410
- `rust-version` rises from 1.89 to 1.90, the floor of `ast-grep-core` 0.51 and `tree-sitter` 0.27, and every document that states the MSRV follows
- The grammar cache is versioned by pack version, so the bump invalidates the `v1.8.1` grammar directory and the first AST verb downloads those grammars again under `v1.22.1`
- `log.file` and `log.rotation` accept the workspace scope too, and `startup_log_section` merges the global layer with the `.atomwrite.toml` of the workspace, so the `[log]` section now starts the sink instead of being validated and discarded
- `atomwrite --version` no longer appends `-dirty`: the identifier is the short commit id of the tracked tree, so an untracked file can no longer change the string
- `batch --transaction` publishes one verdict: the `rollback` event left `SUCCESS_ENVELOPE_TYPES` and is written through the failure path, so it carries `ok: false` with the `code` and `exit` of the failure that caused it
- `syntax_check` degrades to `Skipped` with the typed reason instead of an opaque pack error string


### Fixed
- `scope` publishes `files_matched` from a dedicated counter, so a file that failed no longer counts as a match and a read-only run reports the files that answered
- `transform --rules` counts a failure once per distinct file instead of once per rule, and `errors[]` lists each path once
- `case` keeps exit 0 and publishes `partial: true` when it renamed at least one file and another one failed, the same rule `search`, `replace`, `transform` and `scope` follow
- `transform --rules` keeps exit 0 and publishes `partial: true` when at least one replacement landed and another file failed, which is the rule single-rule `transform` already followed and the multi-rule path did not
- A `.gitignore` or `.ignore` with bad syntax or an unreadable mode is reported in `files_failed` and `errors[]` with the ignore file's own path, instead of being dropped by the walk in silence
- `read`, `count`, `list`, `hash` and `diff` publish `target_resolved` and `target_source`, with `workspace` only for the `.` default of `count` and `list`
- The `Summary` field descriptions match what the producers publish: `files_skipped` covers every case the run did not answer, `search-files` reports name hits in `files_visited`, and `transform` publishes replacements in `total_matches`
- `delete` publishes `files_skipped` as the age-skipped count, and the tautological expression that always equalled the same number is gone
- The gap inventory is no longer a live pointer: `gaps.md` left the package `include`, and the documents that cited it point to the ADR series
- `replace`, `transform` and `scope` fail with exit 4 `FILE_NOT_FOUND` when a root named in the invocation does not exist, even when they matched or rewrote: they used to answer `ok: true` and exit 0 with the missing root listed only in `errors[]`
- `scope` publishes `partial: true` beside `files_failed`, because it kept a private copy of the failure summary while the shared one had already gained the field
- `count`, `list`, `sparse`, `semantic-search` and `case` publish `partial: true` beside `files_failed`, through the same `partial_of` helper, so no producer decides the pair by hand
- `case` publishes its terminal summary when a file cannot be read or rewritten: a per-file error is tallied instead of aborting through `item?`, so `files_failed`, `partial` and `errors[]` name the file before the same exit code ends the run
- `scope` publishes `total_matches` as the sum of `scopes_matched`, a field the schema already declared and no run filled
- `transform --rules` adds the failures of every rule into `files_failed`, `partial` and `errors[]`, and counts a missing root once, instead of answering with a summary that knew no file had failed
- A walk failure that names no path of its own is attributed to the marker `<walk>`, so `errors[]` never carries an empty `path`
- `scope` serializes the shared failure summary instead of its own copy, so a future field of the failure report reaches the verb without a second definition
- `locale --set` and `--clear` stop promising an effect on the next invocation, because the report already shows the state the call leaves behind
- `read --format raw` expands a pattern whose directory prefix runs several levels deep, and only a wildcard in a directory name is refused, because the old check counted every component of the path instead of asking which one is a pattern
- A name with `*`, `?` or `[` that exists on disk is read as it is instead of being expanded, so `read` and `cat` agree on a file such as `report[1].txt`
- `search` derives the line buffer ceiling from `search.max_filesize` instead of a fixed cap, so a file the caller already allowed to be read is no longer refused for having a single line above that cap
- `search` and `replace` no longer take a flag the verb does not have as the pattern: the attribute that allowed it is gone from `pattern` and from `replacement`, so `search -Z 'needle'` exits 2 with the parser's own `use '-- -Z'` tip, where it used to search for the literal `-Z` and answer `ok: true` or `FILE_NOT_FOUND`
- A pattern that starts with a hyphen still works, now behind `--`, exactly like `rg` and `grep`: `search -F -- '-deprecated' .`
- `search` fails when a root named in the invocation does not exist, even when it matched: the missing root is reported before the walk, the roots that exist are still searched, and the summary carries `ok: false`, `code: FILE_NOT_FOUND` and exit 4 (measured: `search -F 'pub pattern' src/cli_args nao-existe.rs` printed its three matches and exited 4)
- The failure summary gained `partial: true` whenever `files_failed > 0`, so a run that answered from part of its targets says so in one key; a file that fails during the walk still keeps exit 0
- `--format text` writes every failure to stderr with its path and reason, and follows the same status rules
- The `search --sort --limit` summary asserted `walker_stopped_early: true` in a test whose field documents the opposite: the sorted path buffers the whole tree, so the walker never quits early, and the assertion now matches the contract
- `read --format raw` of a directory exited 74 with a JSON envelope on stdout, and now exits 74 with the diagnostic on stderr and an empty byte channel
- `read --format raw` with several targets skipped the workspace jail: `/etc/passwd` named beside a jailed file, or reached through a symbolic link that a glob matched, streamed its bytes and exited 0, while the same name as the only target exited 126. Every target now faces `validate_path`, so the breach reports 126, the other targets still reach stdout, and a relative operand resolves against `--workspace` exactly as a single operand already did
- A terminal record that names a failure survives `--filter`: `search --filter ok=true` over a tree with an unreadable directory printed nothing at all and still exited 0, so the `PERMISSION_DENIED` entry inside `errors[]` never reached the caller. The filter still narrows a clean run
- `walker_stopped_early` was asserted against 12-file trees where the walker sometimes finished before `--limit` cut it, in `--count`, `--files` and `--offset` with `--limit`; those three existing tests now use a 400-file tree, which is the condition the field documents
- The documentation guards were red for two stale facts: `INTEGRATIONS` kept `v0.1.37` in the heading that carries the subcommand total, so the count read as history, and the README exit table had no `2 UNSUPPORTED_COMBINATION` row
- `read --format raw` printed the first batch failure last: the first error was held for the status and the later ones were printed on the spot, so stderr ran against argv order (measured with two oversized targets, `b.rs` before `a.rs`). Every failure now prints where it happens and the returned error carries `VerdictPublished`, so the status is still the first failure's and no line is written twice
- `--timeout-secs` overshot by about 900 ms on a file that is one unterminated line, and the cause was measured instead of guessed: capping each `CancelRead` read changed nothing (five paired runs inside the noise), because the cancel flag is only consulted when the searcher asks for bytes — and between two reads grep rebuilt its line buffer by copying everything already read, copies that cost 43, 131, 385 and 1097 ms on a 4 GiB line. The searcher now has a 64 MiB `heap_limit`, so the buffer and the copy stay small and a longer line is reported per file as `INVALID_INPUT` naming the line: the same fixture went from 1946 to 2267 ms against a 1 s deadline to 76 ms, and a control of fifty 50 MiB lines went from 1053-1071 ms to 1017-1022 ms
- `search --limit` now reads `walker_stopped_early` off the walker instead of off the writer's output budget: on a 4000-file tree with `--limit 1`, 7 of 10 runs stopped after 9 to 58 files and still published `false`, and after the fix 12 of 12 publish `true` when the walk stops
- The same defect was fixed in `replace` (all three `Quit` points now record it; 10 of 10 runs publish `true`, against 8 of 10 publishing `false` before) and in `search_files`, where the field is now absent because that path collects every name hit and slices the page afterwards, so it never cuts its own walk
- `risk_assessment.size_delta_pct` is derived from the two sizes in `write_guards::assess_content_risk` instead of being a hardcoded `0`: the same field had two meanings across verbs, and a 22-to-45 byte growth published `0` where the real delta is 104
- `scope` carried the same writer-budget stamp and now reads its own `walk_quit`: on a 410-file tree with `--limit 1` it publishes `files_visited: 32` with `walker_stopped_early: true`, and without a limit it visits 410 and omits the field
- `semantic-search` stopped stamping `walker_stopped_early` from the writer's budget too; it collects the whole tree and slices the ranking afterwards, so the field is now absent and the truncation is visible as `results` against `k`
- `codemod` counted one match per NDJSON line instead of reading the `matches` field, so a campaign of two rules published `alpha.matches: 4` for two real matches; it now sums the event's own count, and `by_rule_id` adds up to the `total_matches` beside it
- `codemod` tagged the second rule's `transformed` events with the first rule's id, because it fell back to `rule_ids.first()`; attribution now follows the `rule_begin` stream, so `beta` reports its own matches and its own files instead of `files: 0`
- `codemod` read rule ids with a line-based heuristic that lost flow-style sequences, quoted ids and `id :` with a space before the colon; the ids come from the same typed manifest parse `transform` already runs
- `--config` did not isolate the `[log]` section: a workspace file with an invalid rotation printed `invalid configuration` to stderr and the command still ran with defaults and exited 0, while the same file without `--config` failed with 78. `startup_log_section` now takes the explicit path and reads only it
- `transform`, `replace`, `case`, `edit`, `write` and `scope` published no `risk_assessment` on their dry-run records, because the assessment lived inside the `if !dry_run` branch; the mode that exists for an agent to decide before writing was the one without the diagnostic
- `codemod --help` described the rules file as "ast-grep multi-rule format" while the parser accepts exactly `id`, `language`, `pattern` and `rewrite`, and the refusal said "invalid YAML" even when the YAML was valid and only the schema was wrong; the message now names the layer and the field that failed
- `transform_stub.rs` kept five parameters while both callers passed six, so no `--no-default-features` build compiled at all: `cargo xtask check-features` exited 101 with two E0061
- `startup_log_section` linked the private `merge_layers` from a public doc comment, and `#![deny(rustdoc::private_intra_doc_links)]` turned that into `cargo doc` exiting 101
- `case` stamped `walker_stopped_early` as a literal `None` while `cap_collected` already raised the process-wide flag every other budget-spending verb reads, so `case --limit 1` over five files published `files_visited: 1` with no word about the cut; it now reads the same flag and publishes `true`, which is what turned `case_converts_one_file` from a declared pre-existing failure into a pass
- `CodemodRuleStats` documented `matches` as a count of events, which is the semantics the code implemented and the one this release removes

### Validated
- Every proof below is a real run of `target/debug/atomwrite` or `cargo check`, never a test: the project rule is that a test is a mock-up and only running the program counts
- Cross-compilation measured target by target with `cargo check --offline --target <T> --lib`: `x86_64-pc-windows-gnu` finished with exit 0 in 12.28s, which is the only Windows target this host can link
- Blocked by a missing C linker, not by a code defect: `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` stop inside `zstd-sys` under `cc-rs` because `aarch64-linux-gnu-gcc`, `x86_64-linux-musl-gcc` and `aarch64-linux-musl-gcc` are absent from `PATH`
- NOT VALIDATED for lack of an installed target: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`, `i686-pc-windows-gnu`, `aarch64-apple-darwin` and `x86_64-apple-darwin`
- The `9 passed` line of `cross_compile_check` is not proof of nine targets: the test returns early when a target is absent, so it approves by absence; the honest count is one compiled, three blocked, five without a target


### Added in the 2026-10-15 draft of this entry
- Global stream shaping on the data channel: `--sort-by` (`path-line` / `line-path` / `key`), `--dedupe KEY`, `--aggregate`, `--join`, `--diff-against FILE`, `--redact`, `--normalize-encoding`, `--max-output-bytes` (E4)
- `--print-schema [NAME]` schema discovery, answered before clap, listing every name or emitting one (E3)
- `extract-artifact` subcommand: bounded signal from PDF, OOXML (document / spreadsheet / presentation) and image headers — text, structure and metadata, never raw artifact bytes (E5)
- Side-effect guards accepted globally: `--non-interactive` and `--confirm-side-effect`, with truncation behind an explicit opt-in (E6)
- Single cancellation primitive with block granularity (`CancelCheck`), shared by the scan loop instead of copied checks (A1)
- `deadline_exceeded` boolean on the `cancelled` envelope, always serialized and never skipped, so a consumer separates an elapsed budget from a signal without reading the human text (A3)
- Single terminal-cancel point in the dispatch: every scanning verb emits the same `cancelled` envelope when interrupted, instead of each one choosing between an envelope, silence and a partial summary on its own (A3)
- The terminal cancel record is written unshaped, so `--filter`, `--select` and `--limit` cannot drop it
- A shaping flag describes how to present a complete answer, and must not erase the notice that the answer is incomplete (A3)
- `discovery` entry on `agent-surface`: publishes the schema-discovery door with both forms and the name list, generated from `SCHEMA_NAMES` so the manifest and the door can never disagree (E3)
- `MERGE_CONFLICT` domain error with `error_class: permanent` and `conflict_count`, exit 84 (C1)
- `missing_flag` structured field on the directory-without-recursive error (C3)
- Derived exit-status list published on `agent-surface` from the error enum, so a new code enters by construction (B3)
- `src/stdin_cancel.rs`: a cancellable stdin reader, a reader thread plus a bounded channel polled with a short timeout (R2)
- The thread is the only portable way to abandon a blocked `read(2)` on Linux, macOS and Windows, because `signal-hook` installs handlers with `SA_RESTART` and the kernel restarts the read
- `edit-loop --partial`: writes the pairs that match and exits 87 (`EDIT_LOOP_PARTIAL`) when at least one pair missed (R13)
- `pairs_applied` on the `edit-loop` envelope, next to the pair counters it already carried (R13)
- SIGHUP handling: a closed terminal ends the run as a cancellation with exit 129 and `signal: "SIGHUP"` on the `cancelled` envelope (R22)
- `ctrlc` is built with its `termination` feature, so the Ctrl-C handler also covers the termination signals (R22)
- `files_skipped` on the `search` summary, counting files skipped for exceeding the size limit (R29)
- `files_failed` and `errors[]` on the `scope` summary, so a file that failed to write is reported in the data channel and not only in a log line (R11)
- Advisory write lock wired into `atomic_write` (R23): a hidden sidecar `.<target>.atomwrite.lock` next to the target, waited on for up to 5000 ms
- A lock that is not granted in time fails with `LOCK_TIMEOUT`, exit 83, carrying the real path of the target
- Config schema version: `version` key in `config.toml` and `CONFIG_SCHEMA_VERSION = 1` (R25)
- `[log] file = true` and `[log] rotation = "daily" | "hourly"` in the global XDG `config.toml`, to turn the log file on (R26)
- `target_resolved` and `target_source` on the terminal event of every verb with side effects: `edit`, `write`, `edit-loop`, `set`, `del`, `apply`, `rollback`, `copy`, `move`, `delete`, `backup`, `prune-backups`, `case`, `replace`, `transform`, `scope`, `codemod`, `wal-heal`, `scratch init` and `recipe run` (R14)
- `target_source` is always `argv` for these verbs, and `target_resolved` is the canonical absolute path (several roots are joined with a comma, in argv order)
- `STREAM_RECORD_TYPES`: the closed list of stream record types (`begin`, `match`, `progress` and the like), next to a corrected `SUCCESS_ENVELOPE_TYPES`, so every terminal type is classified (R6)
- `NdjsonWriter::write_event_failed` (R7): stamped `ok: false`, `code` and `exit` on a summary followed by an error; it was replaced in this release by `publish_failure`, described below, and no longer exists
- `exit_codes_help()`: the exit-code list in `--help`, derived from the same source as `agent-surface` (R12)
- Exit-code table between `<!-- exit-codes:begin -->` and `<!-- exit-codes:end -->` markers in both readmes, guarded by `tests/docs_exit_codes.rs` (R12)
- Real-deadline test with a child process and a real signal (A4)
- Test of the displaced token: ambient target X with argv target Y must fail closed and never act on X, now run for every verb with side effects (F3)
- Suites `cli_v0137_cancel`, `cli_v0137_envelope`, `cli_v0137_dominio`, `cli_v0137_agent_native`, `cli_v0137_artefato`, `cli_v0137_etd`, `cli_v0137_etd_targets`, `cli_v0137_config`, `cli_v0137_contract`, `cli_v0137_lock`, `cli_v0137_safety`, `cli_v0137_search_rss`, `cli_v0137_search_stream` and `cli_v0137_stdin_cancel`
- `config` subcommand, the 47th: `config path`, `config list`, `config get KEY`, `config set KEY VALUE --scope global|workspace` and `config unset KEY --scope global|workspace`, so a tuning value no longer needs a code edit (`src/commands/config_cmd.rs`)
- `--scope` is mandatory on `path`, `set` and `unset`: without it the verb exits 2 and touches nothing, and `get` and `list` print the effective value with the layer it came from (`default`, `global` or `workspace`)
- `config set` reads the file, changes the key keeping the comments, validates the whole file, and only then writes it with `atomic_write` (global file mode 0600, no backup)
- `config` carries `target_resolved` and `target_source: "argv"` like every verb with side effects, with a stamp branch of its own in `src/lib_dispatch.inc.rs` because the global XDG file lives outside the jail
- `config` is handled before the config load, so a broken `config.toml` does not block it, and a file that is not valid TOML is read with a warning and never rewritten (`set` and `unset` exit 78 with the path)
- The terminal envelope is `type: "config"` and `docs/schemas/config.schema.json` describes it
- 23 typed keys, one table feeding `list`, `get`, `set` and `unset`: `fuzzy.mode`, `fuzzy.threshold`, `fuzzy.threshold_aggressive`, `fuzzy.threshold_context`, `fuzzy.threshold_jw`, `fuzzy.soft_floor_delta`, `fuzzy.max_content_bytes`, `write.confirm_large_bytes`, `log.file`, `log.rotation`, `storage.home`, `locale`, `concurrency.max_threads`, `concurrency.ram_fraction_percent`, `search.max_inflight_bytes`, `lock.acquire_timeout_ms`, `retry.base_ms`, `retry.max_ms`, `retry.max_elapsed_ms`, `extract.max_sample_rows`, `extract.max_text_chars`, `extract.max_sheets` and `extract.max_slides`
- `log.file`, `log.rotation`, `storage.home` and `locale` are global-only: the workspace scope rejects them with exit 65, and an unknown key exits 65 listing the valid ones
- `storage.home` is circular (it decides where the config lives), so `set` writes it straight to the bootstrap file of the project directories and warns in the envelope where the global config moved
- Four limits that `extract-artifact` had hardcoded (`max_sample_rows`, `max_text_chars`, `max_sheets`, `max_slides`) are config keys now, and the existing `--max-sample-rows` and `--max-text-chars` flags still win over the config
- `[fuzzy].max_content_bytes` (default 256 MiB, `FUZZY_MAX_CONTENT_BYTES`): above it the fuzzy cascade does not run, because every strategy costs O(lines)
- `FUZZY_ESCAPE_MAP_MAX_CONTENT_BYTES` (32 MiB): above it the `escape_normalized` strategy is skipped without building its offset map, and the cascade goes on with the other strategies
- Cooperative cancellation polls in every fuzzy loop (`CancelPoll`: a local counter plus a mask, in `src/fuzzy/util.rs`), and `check_cancel()?` after each strategy and before `MatchFailed`
- `NdjsonWriter::publish_failure`: writes the terminal summary with `ok: false`, `code`, `exit` and the whole error detail (`error_class`, `retryable`, `suggestion`, `suggestion_lang`, `conflict_count`, `match_count`, `pairs_*`), then returns the error marked as already published, so `main` does not write a second envelope
- `files_failed` and `errors[]` on the `search`, `transform` and `replace` summaries (as `scope` already had), with `errors[]` capped at `SUMMARY_ERRORS_MAX_ENTRIES` (50) and `files_failed` keeping the full count
- Six new `--print-schema` names, `backup-summary`, `count-by-ext`, `count-by-size`, `list-summary`, `replace-summary` and `search-summary`, so the schema of a terminal summary is no longer the schema of its event (58 names in total)
- `docs/schemas/search-summary`, `replace-summary`, `count-by-ext-output`, `backup-summary` and `list-summary` files, and a regenerated `count-summary`
- `src/retry.rs`: one retry policy with full jitter, a ceiling, `max_attempts` and `max_elapsed` as two stop conditions, waits in 15 ms slices that poll the shutdown flag, and a budget cut to the time left of `--timeout-secs`
- `fastrand = "2"` as a direct dependency, for the jitter (it was already in the tree through `tempfile`, locked at 2.4.1)
- `src/best_effort.rs`: one helper for cleanup whose failure must not change the result, logged at debug instead of dropped with `let _ =`
- `SEARCH_MAX_INFLIGHT_BYTES` (8 MiB): one byte budget shared by every `search` worker, in `src/commands/search_budget.inc.rs`
- `MAX_AUTO_THREADS` (16) and `WALK_AUTO_MAX_THREADS` (12): the ceiling of the automatic thread count and of the automatic walker
- `cargo xtask check-features`: runs `cargo check --locked` on the `core` and `core,watch` feature sets and stops at the first error
- `feature.ast-required` message in the `en` and `pt-BR` catalogues, for a slim build that is asked for `outline` or `sparse outline`
- Suites `cli_v0137_terminal_event`, `cli_v0137_terminal_pipe`, `cli_v0137_terminal_schema`, `cli_v0137_wal_journals`, `cli_v0137_wal_startup_heal`, `cli_v0137_let_underscore_guard`, `cli_v0137_retry_deadline`, `cli_v0137_retry_guards`, `cli_v0137_hardcode_guard`, `cli_v0137_version` and `cli_v0137_config_verb`
- Source guards that fail the build: no `let _ =` and no `.unwrap()`, `.expect(`, `unreachable!`, `panic!`, `todo!` or `unimplemented!` in production code (both lists are empty), no time or sidecar literal outside `constants*`, no `env::var`, no `Command::new`, no `.sh` and no `.github`
- `acquire_for_update` in `src/lock.rs` and `atomic_write_based` in `src/atomic/write.rs`: a read-modify-write verb takes the advisory lock before it reads, and the write layer can compare the checksum of the base content under the lock and fail with `STATE_DRIFT` (see Changed)
- `CancelRead` and `search_file_cancellable` in `src/commands/search_stream.inc.rs`: a reader that checks the cancel flag on every read, so the deadline and SIGTERM are seen inside one huge file
- A SIGPIPE handler (`register_broken_pipe`, `ShutdownSignal::record_broken_pipe`) and `is_global_hard_shutdown()` in `src/signal.rs` (see Changed)
- Exit 129 (`SIGHUP`) in `agent-surface` `exit_codes`, in `--help` and in the exit-code tables of both readmes
- `PATCH_MAX_HUNK_SHIFT` (64 lines) and `PATCH_ERROR_SNIPPET_CHARS` (120) for `apply`
- Dedicated localized hints `suggestion.fuzzy-content-cap` and `suggestion.fuzzy-pattern-cap`, chosen by two shared marker constants, so the hint cannot drift from the `reason` it explains
- `common::hermetic` in `tests/common/mod.rs`: every test that runs the binary gets its own empty `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME` and `APPDATA`, and `LANG` fixed to `en_US.UTF-8` (31 test files), so the suite no longer depends on the config of the machine that runs it; `XDG_CACHE_HOME` is deliberately left on the real cache, because the `ast` feature downloads tree-sitter parsers there at run time and an empty cache would send `outline`, `query` and `scope` to the network
- Suites `cli_v0137_apply_strict`, `cli_v0137_structured_errors`, `cli_v0137_backup_retention`, `cli_v0137_rmw_concurrent`, `cli_v0137_search_cancel_file`, `cli_v0137_audit_regressions` and `cli_v0137_completions_install`
- `tests/common/schema_check.rs`: a JSON Schema validator for the keyword subset these schemas use (`type`, `properties`, `required`, `additionalProperties`, `items`, `enum`, `const`, `minimum`, `maximum`, `anyOf`, `oneOf`, `if`/`then`, `$defs`, `$ref`), written against the standard library so the suite gains no dependency
- The schema check of `docs_schemas_sync.rs` compared `required` and unknown keys only, so a schema declaring `integer` passed with a string in the event; it runs the validator now, and `the_validator_rejects_a_wrong_type_the_shape_only_check_let_through` proves the stronger check is stronger and not assumed to be
- `every_published_schema_uses_only_keywords_the_validator_implements` refuses a schema that starts using a keyword the validator would ignore, so the check cannot silently stop covering a file

### Changed in the 2026-10-15 draft of this entry
- `config_cmd.rs` and `output.rs` fell below the 500-line ceiling: the `config` argument and output types moved to `config_cmd_types.inc.rs`, and the error-envelope emitters moved to `output_error.inc.rs`, with no behaviour change
- Version 0.1.37
- `search` honours the global deadline at about 1x: on a 400 MB tree, a 1 s budget ends in 1.02 s, 2 s in 2.03 s and 5 s in 5.02 s, where it took 5.7x before (R1)
- `search` resident memory on the same 400 MB tree measured 37 MB, where it was 950 MB (R1, R5); that figure had no machine and no thread count, and the re-measurement with both is a few lines below
- `search` keeps a bounded channel per file instead of one global map, so memory stays bounded whatever the match count (R5)
- Every `match` now sits inside the `begin` / `end` pair of its own file with the default thread count, where the match used to go to the longest open path (R3)
- A matching line with invalid UTF-8 is emitted lossy, where it used to be replaced by an empty string (R28)
- `replace`, `transform` and `scope` join the walker threads before returning and do not write after the consumer gave up (R4)
- `write`, `edit`, `edit-loop`, `apply`, `hash --stdin`, `extract`, `diff`, `calc` and `regex` obey SIGTERM (exit 143) and the deadline (exit 124) while stdin is still open (R2)
- `cancelled` is the last line of the output also with `--sort-by`, `--dedupe` and `--aggregate`: the retained tail is dropped when the run is cancelled (R1)
- A non-UTF-8 argument no longer aborts the process: it is converted lossy and reaches clap (R18)
- `read --verify-checksum` with a wrong hash emits an error envelope and exits 81, where stdout and stderr were empty (R8)
- `edit-loop` is all-or-nothing by default, as `edit` is: a pair with no match exits 65 and nothing is written (R13)
- This replaces the earlier draft behaviour (a non-zero verdict after writing the matching pairs), and the suggestion now names `--partial` instead of the flag `--allow-partial`, which never existed
- Paths are mandatory in `replace`, `scope`, `codemod`, `case`, `delete`, `backup` and `prune-backups`, and positional in `wal-heal` and `scratch init`
- `recipe run` needs `--path` or `--target`, and `transform` already required its positionals (F1)
- A verb with side effects and no target exits 2, where `replace foo src/` used to read `src/` as the new text and scan the current directory (R15)
- Read-only verbs (`search`, `count`, `list`, `sparse`, `semantic-search`) keep the implicit `.` and do not carry `target_resolved`
- `doctor` and `env_detect` read no environment variable: detection uses file probes only (`/.dockerenv`, cgroup, `/proc/version`, `/.flatpak-info`, the Kubernetes service-account mount, the Termux prefix, the executable path) (R36)
- The `doctor` environment block loses `ci`, `github_actions` and `sudo`
- The `doctor` report loses its policy check about a tool-protocol server
- `agent-surface` no longer publishes the tool-protocol key, and that word is gone from `src/`
- Configuration is strict (R25): every config struct denies unknown fields, and an invalid file or an unknown key exits 78 (`CONFIG_INVALID`)
- A `version` newer than `CONFIG_SCHEMA_VERSION` is rejected the same way
- The log file is off by default (R26)
- When `[log] file = true` is set in the global XDG `config.toml`, the file rotates daily or hourly, keeps 7 files and is created with mode 0600
- `set` on TOML keeps the inline comment of the replaced value, and `comments_preserved` is computed from the result, where it was a fixed `true` (R24)
- The subcommand count is 47 in every doc and skill, checked against `CANONICAL_SUBCOMMAND_COUNT` (R39); it was 46 before `config`
- `zip` moved from 2.4.2 to 9.0.5 with the `deflate-flate2` backend (R31)
- `serde_json` built with `preserve_order`, so an envelope serialized through `Value` keeps the key order its struct declares
- Without it the error envelope, which must pass through `Value` for the credential floor to run, came out alphabetical and changed the byte shape of every error line
- Measured cost of `preserve_order`: +2 ms on 47 ms for 3000 events (about 4%)
- Residual risk of `preserve_order`: `indexmap` trades O(log n) for amortized O(1) insertion at a higher cost per map, which is irrelevant for envelopes of 10 to 20 keys
- The unused `DIAGNOSTIC_MARKER_KEY` and `DIAGNOSTIC_MARKER_VALUE` constants are gone, because they published a `channel` key that nothing stamped (`tracing-subscriber`'s JSON formatter cannot inject a constant field)
- The discriminator that does hold is guarded by a test: every diagnostic carries `level` and `timestamp`, and no payload carries either
- Every success envelope carries `ok: true`, stamped once by the writer instead of declared per subcommand (B1)
- The classification of terminal types was corrected, because `SUCCESS_ENVELOPE_TYPES` held names such as `copy`, `move`, `apply`, `delete` and `scope` that no verb emitted (R6)
- A summary is no longer followed by a contradicting error: `ok: true` does not precede `NO_MATCHES` in `search`, `replace`, `transform` and `scope` (R7)
- The success envelope key no longer collides with the checksum verdict: `verified` requires `--verify` (B2)
- `search` emits a closing event whenever it emitted an opening one (B5)
- `--sort-by path-line` and `--sort-by line-path` read both `line_number` and `line` spellings, so ordering is a property of the data and not of the thread interleaving (E4)
- `watch` left the default feature set: the default binary answers with the closed-feature error and `agent-surface` reports `available: false` with the opt-in hint (E7)
- Error envelope carries `suggestion_lang`, because `message` comes from the error variant (English by contract) while `suggestion` comes from the i18n catalogue, so one envelope can mix languages (B4)
- `--print-schema=NAME` is accepted exactly as the separated form is, because the help renders the value as optional and clap accepts both spellings
- `--print-schema` placed after the command name, with no value, no longer answers the listing, because a trailing flag is not a discovery request
- The cancel reason of `delete`, `copy`, `hash`, `backup` and the stdin reader comes from the live signal through `cancel_reason`, where a fixed sentence said `cancelled by signal` even when the global deadline fired (A3)
- Data-channel contract change: `search`, `count` and the other scanning verbs emit the `cancelled` envelope when interrupted, where they previously emitted silence or a partial summary
- A consumer that reads the last line of a cancelled `search` as a final result now sees the cancel record
- The large modules were split by responsibility: `output_shape.inc.rs` (1951 lines) became nine files, and `run_replace`, `cmd_search`, `run`, `cmd_batch`, `schema_dispatch` and `sparse` were split by responsibility (R40); re-measured with `wc -l` over `src/`, four production files remain above 500 lines, all by a narrow margin: `commands/mod.rs` (520), `commands/query.rs` (513), `commands/search_stream.inc.rs` (507) and `lib_dispatch.inc.rs` (503), plus six `tests.inc.rs` modules that are test code; `commands/read.rs` left the list in this round when its line counting and selection moved to `commands/read_lines.inc.rs`
- `locale.rs` was split earlier by responsibility: persisting the UI locale preference moved to `locale_persist.inc.rs`, following the existing `.inc.rs` pattern (G2)
- `replace_run.inc.rs` was split by responsibility: the closing verdict and terminal emission live in `replace_finish.inc.rs` (G1)
- Behaviour change in configuration: the global `config.toml` and the workspace `.atomwrite.toml` are now merged key by key, with the workspace winning, where a local file used to replace the global one whole (`--config PATH` still replaces both on `get` and `list`)
- `locale --set` and the `locale` key of `config` write the preference through `atomic_write` (tempfile, fsync, rename, mode 0600), where it was a plain non-atomic write
- The global config is read once per process and shared with the log setup, where it was read twice
- `search` resident memory, measured again on a machine with 72 CPUs and 160 GB of RAM, a tree of 1620 files and 380.9 MiB, warm page cache, output to `/dev/null`: 142.6 MB at the default `-j` before this work and 45.3 MB after, 37.9 MB and 39.1 MB at `-j 8`, 22.5 MB at `-j 1` after
- The 37 MB quoted earlier in this entry came from a run whose thread count was not recorded, and it matches the `-j 8` figure, not the default thread count
- The cause was a cost of about 1.5 MB per walker thread (31 MB at 1 thread, 38 MB at 8, 142 MB at 72), not the event channel
- `read` at the default `-j` went from 44.3 MB to 17.9 MB, because the rayon pool is no longer built for it
- An explicit `-j 72` gave 143.6 MB in the T3b measurement and 71 to 80 MB in the V1 measurement of the final build, since an explicit request is honoured above the ceiling; the 64 MB target applies to the default `-j`, which measured 31 to 38 MB
- Dense `search` (3.24 million matching lines) went from 10.4 s to 12.9 s on a single sample taken under load 50, and the 2.5 s difference was not investigated
- Thread ceiling: the automatic thread count is `min(CPUs, 16, RAM cap)`, and the automatic walker is capped at 12, the same guard the `ignore` crate applies internally
- The rayon pool is built on demand and only for verbs that use `par_iter`; `read`, `stat`, `write`, `move`, `get`, `set`, `del`, `rollback`, `calc`, `regex`, `extract`, `extract-artifact`, `verify`, `query`, `outline`, `scratch`, `locale`, `commands`, `agent-surface` and `completions` build none
- `hash_bytes` and the mmap hash take the bounded pool before the first rayon hash (files from 1 MiB), so a large `read` or `verify` no longer builds one thread per CPU
- `search` shares one 8 MiB byte budget among its workers, with the stream being drained exempt only while it has no bytes in flight, so memory stays bounded whatever the thread count and a slow consumer cannot deadlock the pipeline
- `watch` uses a bounded channel of 4096 events, where it was unbounded
- Fuzzy deadline, measured with a 2 s budget on a fixture of repeated lines with a pattern that never matches, on the debug binary: `replace` 50 MB took 6.16 s and 150 MB took 18.6 s before, and both take 2.02 s after; `edit` 50 MB took 5.73 s and 150 MB took 17.7 s before (exit 65), and both take 2.02 s after with exit 124 and a `cancelled` envelope
- 900 MB: `replace` exits 65 in 1.07 s with a per-file `content too large` error and `files_failed: 1`; `edit` takes 4.19 s with exit 124 on the debug binary, above the 3.5 s target, because the file is read, hashed and split into lines before the cascade starts, and the release figure is still to be measured
- Only a real miss (`MatchFailed`, `MatchAmbiguous`) becomes zero matches now: `Cancelled`, `InvalidInput` (content above the ceiling) and internal errors reach the caller in `edit`, `edit-loop`, `replace` and `batch`, with their own code
- Behaviour change in `batch`: a `replace` op whose fuzzy match is ambiguous now fails the op, where it was reported as "no matches"
- The fuzzy one-pass result keeps the edited text as `Option<String>`, so a file with no match is no longer copied; `edit` with several pairs borrows the buffer until a pair applies
- File reads open the file once, use the `fstat` of that handle and read straight into a `Vec` of the known size, where they mapped the file and then copied it (twice the file size resident)
- A `search`, `transform` or `replace` that cannot read, open, probe or walk a file reports it in the data channel: `files_failed` and `errors[]` in the summary, the error of the first failure when nothing matched, and exit 0 with `files_failed` visible when something matched
- A file that is too large, not UTF-8 or binary is still skipped by design and not counted as a failure, so a tree with images does not turn `NO_MATCHES` into `INVALID_INPUT`
- Every failure now ends in exactly one terminal event: `search` without a match, `semantic-merge` with a conflict, `edit-loop --partial`, `replace`, `transform` and `recipe` carry `ok: false`, `error_class` and the rest of the detail in that event, and `main` no longer writes a second envelope
- The failure event ignores `--filter` and `--limit` and honours `--select` and `--redact`
- `EditPairFailed` (an `edit-loop` or `edit` pair with no match) has the code `MATCH_FAILED`, same exit 65, where it reported another code
- The `recipe` summary no longer repeats the failing child `search` line, because the verdict is the single `recipe_result`
- `--print-schema` serves `count` as `CountTotalOutput`, and `search-summary` and `replace-summary` are two distinct schemas (`SearchSummary` and `ReplaceSummary`) that carry `files_failed` and `errors`
- WAL, H3 rate limit: the journal `Drop` no longer consults it, so a journal that committed is removed. H3 now applies only at creation, in `Auto` policy: after 10 journals opened in 60 s the next `Auto` writes open none (they stay atomic through tempfile and rename), and `--wal-policy always` is never limited
- Consequence to know: in a `replace` over 120 files only the first 10 get a crash-recovery journal
- Measured on 120 files in one directory with `replace foo bar`: 110 residual `.atomwrite.journal.*` files before and 0 after
- `startup_wal_heal` runs only for verbs that mutate and only on the directories named in argv (the subtree for a directory target, the parent only for a file target), skipping `.git`, `target` and `node_modules`, honouring `.gitignore` and `.ignore`, and checking its 100 ms budget inside the walk and not after it
- Measured on `read` of one file in a workspace whose `target/` has 40203 entries: 0.02 s and 73 to 90 MB before, 0.00 s and 23 to 24 MB after (release before, debug after)
- `wal-heal` keeps its full-scan behaviour, and the verbs that mutate but are not in the explicit-target list (`semantic-merge`, `batch`) get no automatic heal
- `inplace.rs` removes only the tempfile of its own operation, never every `.atomwrite-*` file of the directory
- Retry: `lock.rs` waits through `crate::retry` with a 50 ms base, a 400 ms ceiling, 512 attempts and a budget equal to `lock.acquire_timeout_ms` (default `LOCK_ACQUIRE_TIMEOUT_MS`, 5000 ms), and cancellation ends it as `Cancelled` while a spent budget ends it as `LOCK_TIMEOUT` (exit 83)
- When the budget is the global deadline, the wait gives the watchdog up to 100 ms to raise the flag, so the run ends in 124 and not in 83
- `persist_retry.rs` (Windows) retries through the same module, and now also treats `ERROR_SHARING_VIOLATION` as transient
- The `EAGAIN` loop of the shutdown banner waits with `poll(2)` and `POLLOUT` in 20 ms slices up to 200 ms, where it spun `yield_now` at 100% CPU
- The first retry interval is 50 ms, the minimum the owner's retry rule allows
- `read` returns 141 on a closed pipe and propagates a failed flush, where it returned 0
- `copy --preserve` fails when the permissions or timestamps cannot be copied, where the failure was dropped
- Restoring permissions and timestamps after an `atomic_write` that already committed logs a warning and keeps the result, because failing the command at that point would be worse
- Behaviour change: when the error envelope itself cannot be written because the reader closed the pipe, the run exits 141, where it kept the status of the original error; any other write failure of the envelope goes to stderr and keeps that status
- Behaviour change: a failed flush of the log file turns the file sink off with a warning, and a failed 0600 permission change on the log file is warned about on stderr
- Channel sends to a consumer that is gone go through `best_effort::send_to_consumer`, and the other `let _ =` of production code are gone: they propagate when they change the result, go through `best_effort` when they are cleanup, and were removed when they only silenced an unused value
- Nine production panics are gone: seven in `rollback`, `apply`, the `completions` dispatch, `locale` (two) and `commands::mod` became an internal error (`apply_replacement` is test-only now), and the `expect` of the fuzzy cascade became a `let ... else` with an internal error
- The static regex of `normalize_punctuation_whitespace` became a `char` loop, so it has no `expect` either
- `language_identifier` in `locale.rs` returns a `Result`
- `Idioma` and `direcao()` are `Language` and `direction()` (returning `TextDirection`) in `src/locale.rs`, so the code and its rustdoc are in English
- The comment of `locale.rs` that said "never raw LANG" was corrected: on Linux `sys-locale` reads `LANG`, and that crate is kept as the system source
- CONSCIOUS DEVIATION from the literal owner rule on environment variables: `sys-locale` reads `LANG` and its siblings on Linux, and `directories` reads `XDG_*` and `HOME` to place the config, data, cache and state paths. Both are crates the owner rules sanction, neither is a product knob, and the product code calls no `env::var` (`rg` is empty and a test guards it); see `PRIVACY.md:14`, `ARCHITECTURE.md:230` and `gaps.md` (W54)
- CONSCIOUS DEVIATION from the literal owner rule on language: `message` and `reason` of the NDJSON errors and the clap help text stay in English by contract, and only `suggestion` is localized (en and pt-BR, with `suggestion_lang`); see `ARCHITECTURE.md:331` and `gaps.md` (W55)
- The literals of the hardcode inventory moved to `constants*`: sidecar and backup names (one `BACKUP_INFIX`), time units, risk thresholds, buffer and block sizes, `/proc` and container probe paths, errno values, the BOM, thread names and the calendar constants of `time_fmt.rs`
- `list` no longer carries its own copy of the calendar algorithm: it calls `atomic::epoch_to_utc`
- `src/constants.rs` was split by responsibility into `constants_concurrency.inc.rs`, `constants_config.inc.rs` and `constants_literals.inc.rs`, and `config`, `fuzzy`, `ndjson` and `replace` gained `.inc.rs` files for the same reason (`config_tuning.inc.rs`, `config_cmd_keys.inc.rs`, `config_cmd_write.inc.rs`, `part3.inc.rs`, `replace_file.inc.rs`)
- `--version` prints `atomwrite 0.1.37 (no-git) x86_64-unknown-linux-gnu` outside a repository: `build.rs` walks up looking for `.git`, calls `git` only when it finds one, accepts the SHA only when `git ls-files --error-unmatch Cargo.toml` proves the repository tracks this crate, and prints `no-git` otherwise, never `unknown`
- `build.rs` emits `rerun-if-changed` only for paths that exist
- Slim build: `sparse outline` without the `ast` feature exits 78 `CONFIG_INVALID` with a rebuild hint in the locale language, and `outline` uses the same message key; neither fakes a success
- `cargo check --locked --no-default-features --features core` and `core,watch` pass, and `cargo xtask check-features` runs both
- The `-j` help says "omitted → all logical CPUs up to the built-in ceiling, RAM-capped; `0` → same as omitted; `N` → exactly N workers, also above the automatic ceiling"
- `deny.toml` no longer allows `BSL-1.0`, which no crate in the tree uses and which raised a `license-not-encountered` warning
- `Cargo.toml` declares `rust-version = "1.89"` (the earlier text of this entry said 1.88), `rust-i18n` is 4.2 and the YAML reader of `transform` uses `serde-saphyr` 1.3
- Broken pipe, final decision: SIGPIPE has a handler that marks a cooperative shutdown with exit 141 and does not force an exit when it repeats; the write that hits the closed pipe fails with `BrokenPipe`, the normal error path runs every `Drop`, and `main` exits 141
- An in-flight write finishes and commits (`is_global_hard_shutdown` in `src/atomic/rename_path.rs`), the checks between files stop new work, and SIGINT, SIGTERM, SIGHUP and the timeout still abort; the shutdown banner is not written for a broken pipe
- DELIBERATE DEVIATION from the literal owner rule, which says to reset SIGPIPE to `SIG_DFL` (`reset_sigpipe`, now removed): with `SIG_DFL`, `replace | head -1` left 7 to 11 locks, 3 to 7 journals (committed ones included) and sometimes a `.atomwrite-*.tmp` (V1 FAIL-1 in 5 runs, V2 M1 in 6 runs), and `wal-heal` never collected the `started` journals
- With the handler, 20 runs of `replace | head -1` and 20 runs each of `list`, `search` and `read` exit 141 by code, not by signal, and leave no lock, no journal and no tempfile (T2); a repeat of 5 runs of `replace | head -1` over 600 files (T5b) gave exit 141 and zero residue
- The graceful-shutdown rule and exit 141 are preserved, and the owner can revert the deviation by restoring `SIG_DFL`, at the cost of the residue above
- On Windows there is no SIGPIPE, and `BrokenPipe` from `std` follows the same path; this is not validated there
- Read-modify-write is serialised now. Before, 30 parallel `edit` on distinct lines of one file all exited 0 with `ok: true` and only 4 to 10 of the 30 changes survived (V2 A1), 30 `set` kept 1 key, 30 `replace` kept 1 and 19 parallel `config set` kept 2
- `edit`, `edit-loop`, `set`, `del`, `config set`, `config unset`, `apply`, `write --append`, `write --prepend` and the `replace` and `edit` ops of `batch` take the lock before the read; `edit-loop` reads stdin before the lock so it never holds the lock while waiting for input
- `replace` reads each file without the lock, and at write time takes the lock, compares the checksum and, if the file changed, redoes the replacement over the new content, so every writer succeeds and no update is lost
- `case`, `transform`, `scope` and `codemod` use `atomic_write_based`: a file that changed since it was read fails with `STATE_DRIFT` (exit 82, retryable) for that file, and the loser re-runs
- The lock is reentrant per thread (the inner `atomic_write` of the same verb shares it), and `LockGuard` is no longer `Send`
- Measured after (`cli_v0137_rmw_concurrent`): 30 parallel `edit`, 30 `replace`, 30 `set` and 16 `config set` all exit 0 and every change is in the file
- Residual declared: where `flock` does not exist (NFS) and on Windows (the lock is a no-op) only `edit`, `replace`, `case`, `transform`, `scope` and `codemod` keep the checksum net; `edit-loop` passes no base checksum; `batch --transaction` locks per operation and not per batch
- `apply` with a unified diff (C1, critical, already in 0.1.36): the context and the removed lines were never compared with the file. A patch `@@ -1 +1 @@ -zzz +yyy` turned line 1 into `yyy` with exit 0, and a hunk with wrong context erased three lines
- Now each hunk compares context and removed lines with the file (exact, trailing spaces ignored), is found at the header line or at the nearest exact neighbour within 64 lines, all hunks are applied in memory and written once, and a mismatch is `MATCH_FAILED` (exit 65) with `best_candidate` and nothing written
- Behaviour change in `apply`: an insertion-only hunk inserts AFTER the named line (it used to insert one line above), an unreadable `@@` header is `INVALID_INPUT`, a patch with no hunk is `INVALID_INPUT`, and blank padding lines the header does not count are dropped
- `apply` with SEARCH/REPLACE (A2): a missing block used to leave the file half applied with exit 0 and `hunks_applied: 1`; now any missing block fails the whole batch with `MATCH_FAILED` and nothing written (with `best_candidate` when the text resembles some line), and a block that is equivalent in two places fails with `MATCH_AMBIGUOUS` carrying `match_count`, also with nothing written; `Cancelled` and `InvalidInput` propagate instead of being swallowed. There is no `--partial` on `apply`
- `get`, `set` and `del` on an invalid TOML or JSON file (M2) exit 65 `INVALID_INPUT` with an envelope that carries only line, column and a short cause, where they exited 1 with no NDJSON and put the whole file in the message
- Behaviour change in `main.rs`: any error without a type now publishes an `INTERNAL_ERROR` envelope on stdout, where it printed one stderr line (exit 1 collided with `NO_MATCHES`)
- `search.max_inflight_bytes` (M3): the accepted minimum rose from 1 MiB to 4 MiB, the effective budget is `max(configured, workers x 128 KiB)`, and a `.atomwrite.toml` that still says 1 MiB is now refused when it loads. Before, 1 MiB made `search` 10 to 15 times slower with 16 or more workers (`-j 72`: 39.99 to 59.18 s)
- Cause and fix of that slowdown: every release called `notify_all` and dozens of sleeping workers woke to fight for a few bytes (futex storm, 423 s of system time against 83 s of user time); a release now wakes one worker, and only when the total falls under the limit. `-j 72` went from 58.8 s to 4.33 s at 4 MiB and 4.09 s at 8 MiB, on a loaded host
- `search` honours the deadline and SIGTERM inside one huge file with no match: on a 492 MB file and a regex with no literal, `--timeout-secs 1` exits 124 in 1.01 s where it took 1.63 s, and SIGTERM at 0.3 s exits 143 in 0.307 s where it took 1.60 s (T3b); V1 measured 124 in 1021 ms on 398 MB
- The first version of the search byte budget cost 23% of wall time on dense output; it uses an atomic counter now and touches the mutex only when a worker sleeps, which gives 9.29 s against 9.59 s of the installed binary (T3b, interleaved runs on a loaded host)
- `semantic-merge` stamps `target_resolved` and `target_source: "argv"` from `--output`, and `locale --set` and `locale --clear` stamp the preference file; `batch` stays without the stamp because its targets come from stdin, and each target already appears in its own `batch_op` event
- Backup retention deletes only `<name>.bak.<valid stamp>`: `f.txt.bak.0-user-note` used to be deleted with three `edit --keep-backup --retention 1`
- `--aggregate` keeps `ok`, `code` and `exit` of the failing terminal event, and `--select` no longer emits an event it left empty (a search with no match printed 16 `{}` lines); the failure still goes through the failure path
- The `config` keys are validated by range and by relation, shared by `config set` and the startup load: `concurrency.max_threads` 1 to 256, `concurrency.ram_fraction_percent` 1 to 100, `lock.acquire_timeout_ms` 100 to 3600000, `retry.base_ms` and `retry.max_ms` 1 to 60000, `retry.max_elapsed_ms` 1 to 3600000, `search.max_inflight_bytes` 4 MiB to 1 GiB, `extract.max_sample_rows` up to 100000, `extract.max_text_chars` up to 10000000, `extract.max_sheets` and `extract.max_slides` up to 10000
- `retry.base_ms` cannot exceed `retry.max_ms` nor `retry.max_elapsed_ms`; `config set retry.max_ms 5` with a base of 50 exits 65
- The dedicated fuzzy hints no longer embed the English `reason` in the localized hint; `suggestion.invalid-input`, `config-invalid`, `internal-error`, `orphan-journal` and `transaction-rollback-failed` stopped embedding it too, so a pt-BR hint is entirely in pt-BR and cites `atomwrite config set fuzzy.max_content_bytes <BYTES> --scope global` and `--fuzzy off`
- `--print-schema diff` serves `DiffOutput`: `type: "object"` with a `oneOf` of four branches (`change`, `summary`, `unified`, `stat`), each with `required` and `additionalProperties: false`, and `docs/schemas/diff-output.schema.json` was regenerated; the first version had no `required` and listed only five keys (V1 FAIL-8)
- The terminal summary schemas are closed: `search-summary` (title `SearchSummary`) and `replace-summary` (title `ReplaceSummary`) are distinct, and `count`, `count-by-ext`, `count-by-size`, `backup-summary`, `list-summary`, `search-summary`, `replace-summary` and `config` carry a non-null `required`, `additionalProperties: false` and every key the writer stamps
- The two summaries that can end in their own failure add a rule to the schema: `ok: false` requires `code`, `exit`, `error_class`, `retryable`, `suggestion_lang` and `message`; `code` and `exit` are not always required because a success has none; a test validates 12 real invocations (failures of `search` and `replace` included) and a negative test proves the validator rejects a missing key, an undeclared key and a failure without `code`
- Input errors of the user are `INVALID_INPUT` (65) and no longer `INTERNAL_ERROR` (255), through `AtomwriteError::invalid` and the `invalid_input!` macro: the regex of `replace` and `search`, the globs of include, exclude and set in `search`, `replace`, `transform`, `scope`, `list`, `count` and `delete`, the pattern and the YAML rules (file and inline) of `transform` and `codemod`, the pattern of `scope`, the S-expression of `query`, `read --grep` (it used to be ignored in silence), an invalid `locale --set` and `--set --clear`, the ops of `edit --multi` (invalid NDJSON, missing field) and the line range and JSON line of `extract`
- The fallback of `main.rs` that publishes `INTERNAL_ERROR` is left for real internal failures; `batch` keeps reporting per-operation errors as results, usage errors of clap stay 2, and a test runs 24 such inputs, each requiring exit 65, one `ok: false` event, a one-line message of up to 240 bytes and a non-empty hint
- The range and relation messages of `config` are fixed English in `message`, and the localized explanation is the hint `suggestion.config-out-of-range` or `suggestion.config-relation` (en and pt-BR); this replaces a first version that localized the `message` itself
- Glob compilation has a single point, `src/walk_globs.rs` (`add_overrides`, `add_include_types`, `validate`), used by `list`, `count`, `delete`, `transform`, `scope`, `replace`, `search`, `sparse list`, `sparse outline` and `hash`; an invalid glob is `INVALID_INPUT` (65) with one event. Before, `list --include '['` and `count --include '['` exited 255 `INTERNAL_ERROR`, and `sparse list --include '['` and `hash -r --exclude '['` exited 0 in silence (V1 FAIL-9, FAIL-10)
- Behaviour change in `sparse list`: `--include` is applied now (it used to be ignored) and `--exclude` is a real glob (the old `add_ignore` call was broken); `hash -r --exclude '['` fails with 65 where it exited 0
- `query -Q` with an invalid S-expression says `invalid S-expression: ...` with the cause (the count of unclosed parentheses), where an explicit `--language` made it say `unsupported language override`; the check (`check_sexpr_balance` in `src/commands/query.rs`) runs before the file is read (V1 FAIL-11)
- Behaviour change in the fuzzy ambiguous case: the single-line Jaro-Winkler branch of `src/fuzzy/cascade.inc.rs` counted every line above the threshold as a tie without passing the dual gate, so only a candidate that passes the dual gate ties now. An absent block among similar lines is `MATCH_FAILED` (65, `match_count` 0, with `best_candidate`) where it was `MATCH_AMBIGUOUS` (200 lines `line NNN` and a block `line 099 WRONG` gave 21 matches); a real tie (`dup  line` against two `dup line`) stays `MATCH_AMBIGUOUS` with `match_count: 2`. It affects every verb that uses the single-line fuzzy match, such as `edit`, `replace`, `apply` and `batch`
- `docs/schemas/` has 58 files, 31 of them regenerated from the binary and 4 completed with `ok`, `code`, `exit`, `target_resolved` and `target_source` (the old files had `additionalProperties: false` and no verdict keys, so a real event would have failed validation); `README.md` and `README.pt-BR.md` use `--max-age-secs` (not `--max-age`) and the ADR flags the pt-BR translation had changed (`--dry-executar` and others) are back to the real names
- Binary sizes measured with `cargo build --release --locked` (T5a): slim 10,348,424 bytes (DERIVED: 10348424 / 1048576 = 9.87 MiB, above the 5 to 8 MiB PRD target and under the 15 MiB local gate) and default 55,632,960 bytes (53.05 MiB); the old "about 7.7 MiB" slim figure came from 0.1.29

### Fixed in the 2026-10-15 draft of this entry
- A cancelled stdin read keeps its typed `Cancelled` error across the `.context` wrapper, so `edit --after-line` with an open pipe exits 124 with `deadline_exceeded` where it exited 255 `INTERNAL_ERROR` (R2)
- `lang_utils` compiles only with the `ast` feature, so the `core` slim build no longer asks for grammar crates it does not carry
- The documented schema total reads 59 names served by 58 files, matching `--print-schema` and the `g7` gate in every document that quotes it
- `write` applies `--max-filesize` to the stdin payload and to the target with one ceiling: a payload above it exits 65 `FILE_TOO_LARGE` (it was `INVALID_INPUT`), and an oversized target is refused before the overwrite guard, in the create, overwrite, `--append` and `--prepend` paths
- `scope` separates an unreadable file (`files_failed` plus `errors[]`) from a file skipped by design (`files_skipped`), and a run that answered for at least one file exits 0 instead of `NO_MATCHES`
- `count`, `list`, `sparse` and `semantic-search` report `files_failed` and `errors[]` instead of an incomplete total with `ok: true` (V2 M4)
- `case` expands a directory argument through the shared walker, honours `--include` and `--exclude`, reports `files_visited` and exits 65 when the tree yields no file, where a directory exited 0 and renamed nothing
- `prune-backups` refuses a directory with exit 65 and a suggestion naming the file list, where it exited 0 without deleting
- `get`, `set` and `del` read and write inline TOML tables (`pool = { max = 5 }`), where `as_table()` returned `None` for `Value::InlineTable`
- `transform` and `scope` refuse a language whose extension set does not cover the target, instead of parsing a `.rb` file with the `haskell` grammar
- `locale --set` and `locale --clear` report the state of the call itself, where the frozen `OnceLock` described the state before it
- `--confirm-side-effect` is rejected with exit 65 `INVALID_INPUT` on a verb whose contract does not ask for it, where `read` accepted it and exited 0
- `--strict-atomic` is back as a real flag on every verb that writes, so exit 91 `EXDEV_FALLBACK_DISABLED` is reachable from the CLI instead of being unreachable dead code
- `--print-schema extract` serves the `extract` union instead of the `calc` schema, and `write-risk` serves `write-risk-assessment.schema.json`, so no schema file is an orphan
- The global `--limit` is a work budget: the target list is cut before the action and the walk stops at the record that reaches the cap, so `--limit 1` touches exactly one target, where the walker used to read the whole tree, act on every target and only stop printing; the terminal summary carries `walker_stopped_early: true`
- `--filter`, `--select` and `--limit` never drop a failure envelope, because a failure record is terminal and not a shaped success
- `transform` in multi-rule mode ends with one summary of the whole invocation; a failing rule adds a `rule_error` instead of a second summary claiming to describe everything
- A closed reader no longer produces a `data channel write failed` warning on stderr, because a fulfilled pipe is the contract and not a defect
- `batch --transaction` reaches exit 80 `TRANSACTION_ROLLBACK_FAILED` by execution, with a manifest that makes the reversal fail without injection, and the failure envelope says the tree may be half reverted
- `config` error envelopes for `set` and `unset` carry `target_resolved` and `target_source`, also when the key is unknown
- The H4 sentinel is `.atomwrite_keep_wal`, the name that says what it does (preserve the journal); `.atomwrite_no_wal` is honoured as the legacy name
- A binary built without an optional feature suggests a rebuild with that feature instead of naming a config key that cannot fix it
- `defaults.line_ending` and `defaults.max_filesize` are consumed by `write`, so both are listed by `config list`, `config get` and `config set` instead of being validated dead keys
- `webpki-roots` 1.0.10 and `rustls-webpki` 0.103.16 close RUSTSEC-2026-0381 and RUSTSEC-2026-0382; the `serde_yaml` and `unsafe-libyaml` exemptions left `supply-chain/config.toml`
- `list` and `count` no longer build a rayon pool of their own for a small tree, and an explicit `-j N` is bounded by the documented ceiling
- `edit` refuses an oversized file from the `stat` before the allocation, so a file above the fuzzy ceiling is no longer read whole before the refusal
- Global deadline now interrupts the scan inside a single large file instead of only between files (A1), also when that file has no match at all: `CancelRead` checks the flag on every read, and a 492 MB file with `--timeout-secs 1` exits 124 in 1.01 s (see Changed)
- The cancel notice is kept off the data channel, and diagnostics travel only on the diagnostic channel (A2)
- `edit --multi` rejects overlapping ranges with exit 65 and leaves the file untouched (R20)
- Internal panics in production code were replaced by an internal error (R20)
- Correction (F-1), not a behaviour change: `doctor --strict` with a failing check now exits 1 with code `DOCTOR_CHECKS_FAILED` (`AtomwriteError::DoctorChecksFailed`) and a single terminal line, the `doctor_report` itself, where it used to exit 255
- Correction (F-1): an interrupted `write` now exits with the real status of the signal or deadline (143, 130 or 124) instead of 255
- Correction (F-1): `batch` without `--transaction` publishes the failure on its own `summary`, with no second error envelope after it
- Correction (F-1): the `doctor` and `batch` schemas are closed (`additionalProperties: false`), the name served by `--print-schema` for the second is `batch`, and the list still has 58 names
- Broken pipe: `completions`, `--print-schema`, `--help` and `--version` piped into a reader that closes early exited 134, 1 or 0 depending on the command, and now exit 141 (`BROKEN_PIPE`) like the other verbs. The cause was `clap_complete::generate`, which panics when the write fails, and `human-panic` then wrote a crash report file in the temporary directory (a local file, never transmitted); test `cli_v0137_broken_pipe_outputs`
- Schemas: `--print-schema completions` served the `write` schema and `scope-result.schema.json` rejected a real `scope` event. Both are corrected and `docs/schemas/*.json` is regenerated from the binary, with the sync test `docs_schemas_sync`
- Packaging: the `exclude` list in `Cargo.toml` is anchored to the repository root (`/AGENTS.md`, `/CLAUDE.md`, `/MEMORY.md`, `/*atomwrite-*`, `/*atom_*`, `/*atom-*`), so `docs/AGENTS.md` and the whole `skills/` directory now enter the published package, where the old unanchored patterns removed them
- The global `--join KEY:SUFFIX` was accepted and ignored, because the output flush handed it an empty table list. It now reads the NDJSON table from stdin once (ceiling of 16 MiB) and attaches the matching row under SUFFIX; it is refused with exit 65 for an invalid spec, a table line that is not an object, a key absent from the table, a command that already reads stdin, a terminal stdin, a table above the ceiling or a table that is not UTF-8. Test `cli_v0137_join` has 13 cases
- `semantic-search --index-dir` outside the workspace jail now exits 126, where it read and wrote outside the jail (R17)
- A file name in NFD form on Linux is found and used byte for byte, where it gave `FILE_NOT_FOUND` because the path was rewritten to NFC for I/O (R19)
- `scope` counts `files_modified` only after the write succeeded, where it counted before and carried on after a failed write (R11)
- An unknown schema name on `--print-schema` leaves the data channel through an error envelope carrying `code` and exit 65, where before it exited 1 with stdout and stderr empty
- `--print-schema` is no longer a dead field: the value clap accepts after the command name is read and rejected with an explanation, instead of being parsed and ignored
- `verify` with a wrong checksum emits one event with one verdict instead of two events contradicting each other (B2)
- `case --subvert` rejects an OLD identical to NEW instead of accepting it silently (C4)
- `delete --plan` on a directory without `--recursive` returns a structured required flag (C3)
- `xtask` dispatches subcommands and never writes on the data channel (D1, E1)
- Stale `docs/exit-codes.md` citations removed from both readmes, because the file was never meant to exist (D3)
- Dead `.sh` gate scripts removed, because they contradicted the cross-platform rule and one depended on an environment variable (D4, D6)
- Docs no longer describe `write --lock` and `--lock-timeout`, which never existed as flags: the lock is automatic
- Docs no longer cite `ATOMWRITE_BACKUP` and `ATOMWRITE_READY_FILE`, which the binary stopped reading
- Docs no longer mention a tool-protocol server or a container engine as a supported route: the integration route is the CLI plus NDJSON through a subprocess
- A1 reopened and closed: `replace --dry-run` and `edit` with `--fuzzy auto` ignored `--timeout-secs` on a large file with no match, because the fuzzy loops never polled for cancellation, and the cascade ended in `MatchFailed` (exit 65 on `edit`) even when the deadline had already passed
- Residual journals: a `replace` over many files left one hidden `.atomwrite.journal.*` file per file after the tenth, because the H3 rate limit counted in the journal `Drop`
- `inplace.rs` could delete the tempfile of another process, since it removed every `.atomwrite-*` file in the directory (the doctor probe file included)
- `search`, `transform` and `replace` returned `NO_MATCHES` for a tree they could not fully read, because the read, open, binary probe and walk errors were swallowed
- A failing run wrote two terminal events (a summary and then an error envelope) in `search` without a match, `semantic-merge` with a conflict and `edit-loop --partial`, so a parser counting one terminal event per run got it wrong
- `edit-loop` counted a refused pair (an empty `old`) as unmatched; only a real miss counts as one now, and a refused pair exits 65 with `INVALID_INPUT`
- `search` had no global byte budget, so its resident memory grew with the thread count
- The `EAGAIN` loop of the shutdown banner spun at 100% CPU when stderr was a full pipe
- `read` exited 0 on a closed pipe and ignored a failed flush, and now exits 141 and propagates the flush error (see Changed for the SIGPIPE decision)
- `copy --preserve` reported success when the permissions or timestamps were not copied
- A `batch` `replace` turned a cancelled or refused fuzzy match into "no matches"
- `set`, `del` and `case` carried a dead `let _ =` or a discarded result, and `set` built a `WriteOutput` it threw away
- The slim build (`--no-default-features --features core`) did not compile: `sparse_outline` used symbols that exist only with the `ast` feature
- `--version` printed `(unknown)` outside a repository
- `deny.toml` carried an allowed license that no crate uses, which `cargo deny` reported as a warning
- Hash of a large buffer or file created a rayon pool with one thread per CPU in verbs that were meant to have none
- `apply` with a unified diff destroyed content with exit 0 when the context or the removed lines did not match the file (C1, critical, already in 0.1.36), and `apply` with SEARCH/REPLACE left the file half applied with exit 0 and swallowed `Cancelled` and `InvalidInput` (A2)
- Lost updates in read-modify-write: concurrent `edit`, `set`, `replace` and `config set` returned success while most changes disappeared, because the lock was taken only at write time (A1 of the second review)
- `get`, `set` and `del` on an invalid TOML file exited 1 without NDJSON and quoted the whole file in the message (M2)
- `search` byte budget starvation: `search.max_inflight_bytes` at its old minimum made `search` up to 15 times slower with many workers (M3)
- Backup retention deleted a user file that only started with `<name>.bak.` (B1)
- `--aggregate` lost the failure verdict of the terminal event and `--select` printed empty `{}` events (B5, B6)
- `replace | head -1` left locks, journals and tempfiles under the default SIGPIPE action (V1 FAIL-1)
- `--print-schema diff` served the schema of a dry-run plan instead of the real `diff` output (V1 FAIL-3), and the `config` schema required a `warnings` key that `config path`, `config set` and `config unset` do not emit (V1 FAIL-2)
- pt-BR hints mixed Portuguese and English for the fuzzy ceiling and for `config` errors (V1 FAIL-5)
- A cooperative SIGTERM or SIGINT in the middle of `replace` left a `started` journal in 16 of 100 runs (V1 FAIL-6): the cancel is seen before the `rename`, with the target intact and the tempfile removed on drop, so there was nothing to recover. `JournalGuard::discard()` now removes the journal in the `Rename` arm when the error is `Cancelled`; 0 of 100 and 0 of 100 afterwards (T3a, two series of 100, SIGTERM). A `kill -9` still leaves the journal, by design (46 of 100 runs), and `wal-heal` keeps the `started` journal of a dead process (`removed: 0, preserved: 1`), so it does not clean it
- `replace --regex '('`, an invalid inline YAML rule of `transform` and `query -Q '((('` exited 255 `INTERNAL_ERROR` instead of 65 `INVALID_INPUT` (V1 FAIL-7b, see Changed)
- `apply` with SEARCH/REPLACE and an absent block that looked like many lines exited `MATCH_AMBIGUOUS` instead of `MATCH_FAILED` (V1 FAIL-7, root cause in the fuzzy cascade, see Changed)
- `completions --install` answered 255 `INTERNAL_ERROR` with "this is a bug" when the data directory was not writable, because the three filesystem calls went through `anyhow::anyhow!` and lost the kind; they now use `AtomwriteError::from_io_at`, so a denied directory is 13 `PERMISSION_DENIED` and the message names the path
- `prune-backups` deleted a user file that merely started with `<name>.bak.`, because its filter was `starts_with(prefix)` alone; only a real backup stamp qualifies now, through the same `is_backup_timestamp_name` the retention path already used (fourth audit round)
- `edit --multi` read a field the chosen op never consumes, so a `new` on a `replace-range` deleted the line and answered `ok: true`; an unconsumed field is exit 65 with nothing written
- `edit --multi` handed a JSON array answered `invalid type: map, expected a string`; it now names the contract, one JSON object per line
- `apply` read a `---`/`+++` header with no `@@` hunk as a whole-file replacement and overwrote the target with the patch text; that header is now `INVALID_INPUT` (65) and the file stays intact
- `apply` counted a hunk with no context, no removed and no added line (`@@ -2,0 +2,0 @@`) as applied; a no-op hunk is refused with exit 65
- `search --help` promised a `skipped` event per oversized file, an event the command never emits; it now documents `files_skipped` on the `summary`
- `wal-stats` published an empty `path` for a journal sitting in the workspace root, which reads as a missing directory; the root is now `.`
- The unstable test `r1b` (files-only) of `cli_v0137_search_stream` failed under load because its 2 MiB probe measured mostly process start, so the real scan sometimes ended before the 2 s deadline; the fixture now grows 4 times and the run repeats up to 3 times, and only a run still alive at the deadline is judged. 52 runs under a load average of 54 to 74 had 0 failures (T3b)
- `apply` in format autodetection replaced the whole file with the patch text when the body carried no marker at all, and answered `ok: true`: 51 bytes became 45 and 34 became 19 in one audit run. `detect_format` now returns `None` for a marker-less body, and the verb refuses with exit 65 `INVALID_INPUT` naming `--format full` as the explicit door, with the checksum unchanged
- `apply --format full` now walks the same two barriers as `write`, through the shared `check_replacement_size`: `--ack-overwrite` above `[write].confirm_large_bytes`, and `--allow-shrink` below `[write].shrink_block_percent`, so the verb that replaces a whole file is no longer the one without a guard
- The two replacement guards live in `src/commands/write_guards.rs` and serve `write` and `apply` from one source, where each verb used to carry its own copy of the threshold arithmetic
- `config list` publishes 35 keys instead of 25: the 10 that the structs declared and no verb exposed are now readable and writable, which ends `INVALID_INPUT` for a key the `write --help` already cited as tunable. They are `defaults.backup`, `defaults.retention`, `write.shrink_block_percent`, `write.auto_rotate_max_age_secs`, `watch.idle_exit_ms`, `watch.debounce_ms`, `search.context`, `search.smart_case`, `search.max_filesize` and `search.max_columns`, each with a range derived from the constant its consumer already uses
- `config set write.shrink_block_percent` now changes the threshold the next `apply` run enforces, where the value was a hardcoded constant: the same file and the same body exit 65 at `50` and 0 at `90`
- The `config` unit tests no longer read the operator's `~/.config/atomwrite/config.toml`: `load_local` drives the pure `merge_layers` with the global layer set to `None`, so the suite answers 482 of 482 in a machine whose real config sets a fuzzy ceiling, where it used to fail 1 of 482
- `doctor` gains two checks that measure instead of trusting the text: `docs_test_targets` compares every statement about the test target count against the real `tests/` listing, and `docs_config_keys` compares every statement about the config key count against the live `KEYS` table. Both answer `skipped` and never `pass` on a tree without documents, and both name the file and the line that drifted
- The documented test target count is the measured 131 in the 16 documents that claimed 133, 117 or 1375, and the config key count is the measured 35 in every document that claimed 23, including the name list in `llms.txt`
- `src/commands/mod.rs` drops from 522 to 469 lines, `src/commands/query.rs` from 513 to 445 and `src/lib_dispatch.inc.rs` from 505 to 427, by moving the backup-name policy, the S-expression parser and the startup WAL heal into their own files, so no source file above 500 lines mixes unrelated responsibilities
- Decisions 0062 and 0069 carry a dated correction in both languages, where they stated target counts, explicit-target membership and file sizes that measurement no longer supports
- `config --help` publishes the key table from the live `KEYS` through `Cli::parser`, where the page carried a hand-written copy that listed 22 keys against the 35 the verb answers. The table and the verb can no longer disagree, because there is one table

### Corrections to earlier drafts of this entry
- F2 (`target_resolved`, `target_source`): an earlier draft listed the fields as delivered while no code emitted them. They are emitted only from this release, as the Added section describes
- F3 (displaced token): the earlier test covered only `transform` and required a rewrite with exit 0. It now covers every verb with side effects and requires a closed failure
- D2 (`TRANSACTION_ROLLBACK_FAILED` reachable): the earlier draft said reachability by execution was not demonstrated. It is demonstrated now by `tests/cli_v0137_batch_rollback.rs`, whose manifest makes a created file become a non-empty directory before a later operation fails, so the reversal hits `EISDIR` without injection and the run exits 80
- Supply chain: the earlier text said `cargo audit` reported 0 vulnerabilities while `zip` 2.4.2 carried RUSTSEC-2026-0367, -0368 and -0369. The claim holds only after the `zip` 9.0.5 upgrade listed under Security
- `serde_yaml`: the earlier text said it had been replaced. It was replaced in the `transform` rules reader only, and it remains in the tree through `rust-i18n`, as the Security section states
- Supply chain, second correction: the Security text named `instant`, `unsafe-libyaml` and `serde_yaml` as part of the tree. `cargo tree --locked --all-features -i` answers "did not match any packages" for `instant`, `unsafe-libyaml`, `serde_yaml` and `serde_yaml_ng`, and `Cargo.lock` has no such package, so that text was false for this tree and is replaced under Security
- `search` memory: the 37 MB figure had no machine and no thread count. On 72 CPUs the default thread count measured 142.6 MB before the thread ceiling and 45.3 MB after, and the 37 MB matches `-j 8` (see Changed)
- Subcommand count: an earlier line said 46. The count is 47 with `config`, and `CANONICAL_SUBCOMMAND_COUNT` is 47 in `src/constants.rs`
- `NdjsonWriter::write_event_failed` no longer exists: it was replaced by `publish_failure`, which marks the error as published
- Items C5, D5 and E2 are not cited by this changelog, and the `gaps.md` line saying they were was false. C5 and E2 survive only as test headings (`tests/cli_v0137_dominio.rs:474`, `tests/cli_v0137_agent_native.rs:628`), D5 appears nowhere, and the D3 of those tests is not the D3 of `gaps.md`, so the ids do not form one scheme
- Lock (N23, R23): the entry first said the advisory lock was closed, but it was taken only at write time, after the verb had read the file, so it did not stop lost updates. It is closed with a cut now: read-modify-write verbs lock before reading (Unix, filesystems with `flock`), and the rest rely on the base-checksum check described under Changed
- Broken pipe, second correction: the shipped behaviour is neither `SIG_DFL` nor ignoring SIGPIPE; it is a handler that marks a cooperative shutdown with exit 141 (see Changed, with the deliberate deviation from the owner rule)
- `search` memory and deadline: the note that a huge file with no match only saw the deadline at the end of its scan no longer holds, `CancelRead` fixed it
- Exit 129: the note that `agent-surface` did not list it no longer holds
- Schema totals: earlier bullets of this entry said 59 names served by 58 files. The final tree serves 55 names backed by 54 files, and ten places in seven documents still carried the old total until this audit fixed them (`llms.txt`, `llms-full.txt`, `docs/MIGRATION.md`, `docs/COOKBOOK.md`, `docs/schemas/README.md`, `docs/TESTING.md` and `docs/TESTING.pt-BR.md`)
- Key totals: earlier bullets of this entry said 23 typed keys. The final tree serves 37 keys, and `docs/MIGRATION.md`, `docs/MIGRATION.pt-BR.md`, `docs/decisions/README.md` and both ADR-0058 files carried the old total until this audit fixed them

### Removed
- The tool-protocol key of `agent-surface`, the matching `doctor` policy check and every mention of a tool-protocol server in `src/` and in the docs
- The `ci`, `github_actions` and `sudo` fields of the `doctor` environment block
- The container-engine section of `docs/CROSS_PLATFORM.md` and `docs/CROSS_PLATFORM.pt-BR.md`
- The `--allow-partial` suggestion of `edit-loop`, a flag that never existed
- The scripts `scripts/check-gaps-ssot.sh` and `scripts/regen-schemas.sh`

### Security
- `zip` 9.0.5 closes RUSTSEC-2026-0367, RUSTSEC-2026-0368 and RUSTSEC-2026-0369 (one of them critical, 9.1) in the OOXML container reader
- `quick-xml` moved from 0.37 to 0.42, closing two reachable high (7.5) advisories in the artifact reader: unbounded namespace allocation (`RUSTSEC-2026-0195`) and quadratic duplicate-attribute checking (`RUSTSEC-2026-0194`)
- `crossbeam-epoch`, `rustls`, `anyhow` and `memmap2` updated to their patched versions, and `inout` updated by `cargo update -p inout` because the old version was yanked
- `cargo deny check` returned advisories ok, bans ok, licenses ok and sources ok with no warning after `BSL-1.0` left `deny.toml` (run by the build executor in this phase)
- `cargo audit --deny warnings` exits 0 on the final tree (see Final gates below)
- `instant` (RUSTSEC-2024-0384), `unsafe-libyaml` (RUSTSEC-2024-0449), `serde_yaml` and `serde_yaml_ng` are not in this tree: `cargo tree --locked --all-features -i <crate>` says "did not match any packages" for each, and `Cargo.lock` has no such package
- `notify` 8.2.0 (optional `watch` feature) no longer pulls `instant`, and `rust-i18n` 4.2.4 and the YAML rules of `transform` (feature `ast`) both use `serde-saphyr` 1.3.0
- `Cargo.toml` declares `rust-version = "1.89"`, which `rust-i18n` 4.2 and `serde-saphyr` need, and the comment in `deny.toml` (ADR-0057, 2026-10-19) records that the two maintenance notices are resolved
- The `[[exemptions.serde_yaml]]` and `[[exemptions.unsafe-libyaml]]` entries left `supply-chain/config.toml`, because an exemption for a crate that is not in the tree only makes `cargo vet` describe a tree that does not exist
- `semantic-search --index-dir` can no longer read or write outside the workspace jail (exit 126)
- Unconditional credential-key floor: a credential-shaped top-level key is redacted on every envelope, with or without `--redact`
- Before this floor, only the shaped path redacted, and a bare call passed `api_token` through while `--limit 5` redacted it
- Content redaction reaches the error envelope: it ran only on the success channel before, so a credential inside a `path` or `message` survived `--redact`
- `--redact` needle list widened to cloud and forge issuance prefixes (AWS, GitHub, Slack, Stripe, Google, Anthropic), so a secret inside a text field is redacted
- Concurrent writers: a read-modify-write verb takes the advisory lock before it reads and the write layer can check the base checksum under the lock (see Changed), and `--expect-checksum` stays the check against drift
- The lock is advisory and Unix only: on Windows it is a no-op, and on filesystems that reject `flock` (NFS) only `edit`, `replace`, `case`, `transform`, `scope` and `codemod` keep the checksum net
- `get`, `set` and `del` no longer put the content of an invalid file in any channel, so a secret inside a broken TOML or JSON file no longer reaches stderr (M2)
- Residual risk declared: `--print-schema` serves short names (`read`, `edit`) while `docs/schemas/` publishes descriptive ones (`read-output`, `edit-output`), and the two sets do not share a naming rule
- Workspace jail retained (no global `/tmp` allowlist): a subcommand still never escapes the workspace
- ETD stage 2 is not done: the ambient workspace (the current directory) is still allowed as the boundary and is not yet reported as `workspace_source`

### Library API
- Breaking change warning: compared with the published 0.1.36, the public API of the `atomwrite` library changed in ways that SemVer calls major, and the supported interface of this crate is the CLI with its NDJSON contract, not the library
- `cargo semver-checks` 0.50.0 against 0.1.36 exits 100 with 10 major checks failed, and each one is listed here
- `auto_trait_impl_removed`: `LockGuard` is no longer `Send` and no longer `Sync`
- `constructible_struct_adds_field`: new public fields in `GlobalArgs`, `ErrorJson`, `AtomwriteConfig`, `FuzzySection`, `SearchSection`, `OutputShape`, `EditLoopArgs`, `WalHealArgs`, `CancelledEvent`, `ScratchInitEvent`, `LocaleState` and `MatchOpts`
- `enum_missing`: `locale::Idioma` is removed (renamed `Language`)
- `enum_no_repr_variant_discriminant_changed`: `LocaleSource::Persisted`, `System` and `Default` changed their numeric value
- `enum_variant_added`: `Commands::ExtractArtifact` and `Commands::Config` on an enum without `#[non_exhaustive]`
- `enum_variant_missing`: `LocaleSource::EnvVar` is removed
- `feature_not_enabled_by_default`: the `watch` feature is no longer a default feature, so `watch` needs a build with `--features watch`
- `function_missing`: `signal::reset_sigpipe`, `locale::negotiate_to_idioma` and `locale::resolved_idioma` are removed
- `pub_module_level_const_missing`: `LOCK_POLL_FAST_ATTEMPTS`, `LOCK_POLL_FAST_MS`, `LOCK_POLL_SLOW_MS` and `PERSIST_RETRY_DELAYS_MS` are removed
- `struct_pub_field_missing`: the fields `ci` and `sudo` of `RuntimeEnvironment` and `idioma` of `LocaleState` are removed
- A version that follows SemVer for the library would be 0.2.0, and the decision between marking these types `#[non_exhaustive]`, hiding the library surface or publishing 0.2.0 belongs to the maintainer

### Documentation
- A documentation audit of the whole repository ran against the 0.1.37 binary, in six reports that cover the gaps, the CLI commands and flags, the contracts (exit codes, schemas, config), the root documents, the `docs/` tree and the `llms` files, the skills, `Cargo.toml`, rustdoc and the `--help` texts
- `README.md` is reorganized, and all 47 subcommands appear in every guide with a one-line summary and a tested example
- ADRs 0058 to 0074 record the decisions of this version, in English and in pt-BR, and ADR 0074 covers the serialization of read-modify-write and the strict `apply`; `docs/decisions` now runs from 0019 to 0074 (56 ADRs) and `docs/adr` keeps 3 more, and the older ADRs gained their pt-BR mirrors
- Every one of the 47 subcommands has a tested example in the README, INTEGRATIONS, AGENTS, HOW_TO_USE, COOKBOOK, INSTALL, TESTING, MIGRATION, the `llms` files and the three skills
- `--help` has an `EXAMPLES` block on all 47 commands and no internal jargon in any of the 59 help pages, the doctests went from 3 to 20, and the rustdoc lints are `deny` in `src/lib.rs`
- `llms-full.pt-BR.txt` is new, the skills (repository and global) are corrected, and `config --help` lists the 35 keys
- `PRIVACY.md` states that the tree-sitter parsers are downloaded at run time on first use, and that the crash report of `human-panic` is a local file in the temporary directory
- `watch` needs a build with `--features watch`, and the binary built without it exits 78 on that verb

### Notes
- Not validated: macOS and Windows were not run. The Unix-only code has a `cfg(windows)` pair (SIGPIPE, `restrict_to_owner`, `persist_retry.rs`), `persist_retry.rs` was only checked by compiling a copy of its body on Linux, and `cargo check --target x86_64-pc-windows-gnu` could not run for lack of `x86_64-w64-mingw32-gcc`
- Not validated: the nine `cross_compile_*` tests
- Partial (P3): `batch --transaction` with a failed operation emits `rollback` with `ok: true` followed by the `ok: false` envelope, and the contract is kept as it is
- Partial (P3): errors without a type still fall to 255 in `cli.rs` and in the rename of `persist_retry.rs` and `rename_path.rs`; the completions install in `lib.rs` left this list because it now uses `AtomwriteError::from_io_at`, so a denied directory is 13 `PERMISSION_DENIED` with the path in the message
- Partial (P3): the `doctor` and `batch` schema test compared `required` and the field names only, so a schema declaring a type the binary does not emit stayed green; `tests/common/schema_check.rs` closed that without a dependency, and `every_published_schema_uses_only_keywords_the_validator_implements` refuses a schema that starts using a keyword the validator would ignore
- Partial (P3): `cargo test --no-default-features --features core` now compiles (`tests/cli_v012_audit_regressions.rs` has `#[cfg(feature = "ast")]` on its four YAML tests, and `cargo xtask check-features` passes), but 44 tests in 11 test targets (`cli_scope`, `cli_transform`, `cli_query`, `cli_outline`, `cli_concurrency`, `cli_e2e_v0135` and others) fail there because they need the `ast` feature and carry no `#[cfg(feature = "ast")]`; testing core locally stays PARTIAL (source: V3)
- `--max-output-bytes` requires `--allow-truncate` and each flag is rejected without the other with exit 2 (`ARGUMENT_PARSE_ERROR`), and top-level `null` values are no longer emitted (11 fields; test `r9_no_null_top_level_values` in `tests/cli_v0137_contract.rs:371`)
- Not done (P3): `--expect-checksum` runs outside the lock, so there is a TOCTOU window between the check and the write
- Not done (P3): a file name above about 238 bytes can overflow the 255-byte limit through the lock sidecar name (`.` + name + `.atomwrite.lock`), and an in-place write in a directory where the sidecar cannot be created has no lock
- Not done (P3): the RAM ceiling works on Linux only and does not read cgroup limits
- Kept by the maintainer: `human-panic` stays, and its crash report is a local file in the temporary directory that is never transmitted
- Kept by the maintainer, and declared: `--print-schema completions` describes only the `installed` event, because `completions` without `--install` prints a shell script as text and emits no NDJSON, so there is no record to describe
- Run-time network access (P3, W18): the `ast` feature downloads tree-sitter parsers at run time when the XDG cache is empty, so the `outline`, `query` and `scope` tests depend on a filled cache and the hermetic test helper leaves `XDG_CACHE_HOME` on the real cache; the `download` feature of `tree-sitter-language-pack` stays out of the scope of this release
- Declared (V2 B2): a new file written by `write` is born with mode 0600 in the old and the new binary (inherited)
- Measured in release by the verifier (V1, `lead-rel` build): `replace --dry-run` on 150 MB exits 124 in 2020 ms and `edit` in 2019 ms with a 2 s deadline, SIGTERM at 0.5 s exits 143 in about 505 ms, and on 900 MB `replace` exits 65 in 1243 ms and `edit` exits 65 `INVALID_INPUT` in 1808 ms
- Measured by the verifier (V1, round 2, `lead-rel` build of 19:20): `search` peaks at 31 to 38 MB at the default `-j` on a 382 MB tree (the 64 MB target applies to the default `-j`), 43.3 MB on 4.8 million matching lines (round 1) and 71 to 80 MB at an explicit `-j 72`
- Measured by the verifier (V1, round 2): `replace --dry-run --fuzzy auto` on a file under the ceiling (233,437 KB) peaks at 304,672 KB in 27.6 s, DERIVED: 304672 / 233437 = 1.305, so the ratio is about 1.3 times the file and not at most 1.3 (round 1 had 307,440 KB, DERIVED: 307440 / 233437 = 1.317, after the verifier corrected the file size)
- Measured by the verifier (V1, round 2, final build): 150 MB with a 2 s deadline exits 124 in 2010 ms (`replace`) and 2009 ms (`edit`), SIGTERM at 0.5 s exits 143 in 505 and 503 ms, and on 900 MB `replace` exits 65 in 685 ms and `edit` exits 65 `INVALID_INPUT` in 1024 ms
- Final gates, run by the team lead on the final tree (release gate on the final tree): `cargo fmt --check` 0, `cargo build --release --locked` 0 with no warning, `cargo clippy --locked --all-targets --all-features -- -D warnings` 0, `RUSTDOCFLAGS=-D warnings cargo doc` 0, `cargo audit --deny warnings` 0, `cargo deny check` 0 with no warning and `cargo xtask check-features` 0
- Final test run, same source: `cargo test --locked --no-fail-fast` gave 131 targets ok, 0 targets with failures, 1486 tests passed, 0 failed and 9 ignored (the `cross_compile_*` tests); the base of this version was 1105 tests. The verifier V3 approved this tree in round 2
- Not validated: the advisory lock on filesystems without `flock` (NFS), and the rename retry of `persist_retry.rs` on Windows, as well as macOS and Windows as a whole (see the first Notes line)


## [0.1.36] - 2026-07-31

### Added
- Error envelope sentinel `ok: false` on all `ErrorJson` lines (GAP-2026-020)
- Stable `match_count: 0` on `MatchFailed` / `NoMatches` error envelopes
- `MatchAmbiguous.best_candidate` populated for agent disambiguation (GAP-2026-025)
- XDG `[fuzzy].soft_floor_delta` wired into context-aware matching (GAP-2026-022)
- Escape-normalized match with original-byte offset map (`escape_map.inc.rs`, GAP-2026-024)
- Canonical scratch relative path constant `SCRATCH_DIR_REL = ".atomwrite/scratch"` (GAP-2026-019)
- Locale suggestions EN/pt-BR mention staging under `.atomwrite/scratch/` for jail escapes
- Content-source-specific jail suggestion (`suggestion.workspace-jail-content-source`) when `--old-file`/`--new-file` escapes the jail
- Global agent-native stdout shaping: `--select`/`--fields`, repeatable `--filter key=value`, `--limit N` (zero-cost identity path when unset; no global `-n`)
- Error envelopes always emit under shape flags (`--filter` does not silence failures; `--limit` is success-only)
- `extract --field` (local field selection; not local `--select` — that name collides with global shape; global `--select` still projects the emitted line). Agents/skills must never teach `extract --select`
- Subcommand `scratch init` creates `{workspace}/.atomwrite/scratch` (idempotent; never opens OS `/tmp`)
- Fuzzy `collect.inc.rs` SRP extract for non-overlapping range scan
- proptest FP oracle (unrelated strings + radical-middle dual-gate)
- Local gate script `scripts/check-gaps-ssot.sh` (no product GHA)
- ADR-0055 agent-first contract (EN + pt-BR); skills EN/PT staging / anti-`--strategy` / `$` literal / shrink / multi-root / shape formulas
- Integration suite `tests/cli_v0136_gaps.rs`

### Changed
- Version 0.1.36
- Normalized fuzzy strategies emit `similarity: null` instead of fake `1.0` (GAP-2026-021)
- `collect_all` single robust scan via `collect.inc.rs` (GAP-2026-026)
- `block_anchor` dual-gates middle ratio when pattern has a middle (GAP-2026-023)
- `docs/schemas/error-output.schema.json` requires `ok`
- Aggressive default threshold 0.50 re-evaluated with FP corpus; dual-gate keeps FP=0 — default retained; override via XDG `[fuzzy].threshold_aggressive`
- Subcommand surface 45 (SSOT: `Commands` enum / `agent-surface`; includes `scratch`, `doctor`, `locale`, `commands`, `stat`, `verify`, …). Constant `CANONICAL_SUBCOMMAND_COUNT` + unit test

### Fixed
- GAP-2026-019…033 inventory closed or mitigated per class (jail/shrink remain policy)
- Agent “all-null” success-field probe on error envelopes

### Security
- Workspace jail on `--old-file`/`--new-file` retained (no global `/tmp` allowlist)

## [0.1.35] - 2026-07-19

### Added
- Large overwrite default-deny: existing targets above XDG `[write].confirm_large_bytes` (bootstrap default 100 KiB) require `--ack-overwrite` (agent one-shot; no Y/N prompt)
- Independent flag `--require-large-ack` (no clap alias collision with `--confirm`)
- `watch` always emits final NDJSON `type:watch_summary` (`events`, `reason`: idle|max_events|timeout|signal) even with zero events
- `semantic-merge` writes conflict markers into output by default (`<<<<<<< ours` / `=======` / `>>>>>>> theirs`); opt-out with `--no-conflict-markers` or `--write-conflict-markers false`
- XDG `[watch].debounce_ms` + CLI `--debounce-ms`; idle default `DEFAULT_WATCH_IDLE_EXIT_MS=500` (was ~3s)
- `delete --plan` for plan-only listing (one-shot safe); `delete --confirm` and `delete --yes` / `-y` are rejected (fail-closed)
- Agent-surface notes for write/watch/merge anti-patterns (A-DISC-001)
- Local contract suite `tests/cli_e2e_v0135` + residual A-* closeout (gaps.md §20)

### Changed
- Version 0.1.35
- Monolith SRP splits via `include!` (write/recipe/lib/replace/scope/concurrency/cli_args/runtime)
- `scope` reuses shared `summary_metrics` (DRY)
- Honest multi-OS matrix: Linux full e2e; Windows-gnu `cargo check` on Linux host; macOS requires real Mac/osxcross (no product GitHub Actions)

### Fixed
- A-TEST-001: suite no longer uses `delete --yes` after B-015 rejection (green `cargo test --lib --tests`)
- A-WATCH-001: idle watch no longer leaves empty stdout
- A-WRITE-001: large overwrite was opt-in via `--confirm`; now always default-deny
- A-WRITE-002: `--require-large-ack` was alias of `--confirm` ("used multiple times"); now independent
- A-MERGE-001: conflict status wrote prefer-ours without markers; markers default ON
- A-ONESHOT-001: watch idle default reduced to 500 ms
- A-GAPS-DOC-001: gaps.md banner + §20 supersede stale "suite 100% / FALHA XDG" headers
- Doctor-only env detect allowlisted (A-ENV-001); zero product env knobs

### Notes
- No product telemetry; no `.github` CI in the product repo
- Configuration is CLI + XDG / `.atomwrite.toml` only (no `ATOMWRITE_*` runtime knobs)
- See `docs/TESTING.md`, `docs/CROSS_PLATFORM.md`, `docs/MIGRATION.md` (0.1.34 → 0.1.35)

## [0.1.34] - 2026-07-19

### Changed
- Full public documentation alignment for one-shot fuzzy contract (README, llms*, AGENTS, HOW_TO_USE, COOKBOOK, TESTING, INSTALL, ARCHITECTURE, INTEGRATIONS, SECURITY, skills, ADR-0054)
- Version 0.1.34

### Fixed
- Documentation drift: many files still claimed v0.1.30 / timeout default 0 / unbounded fuzzy multi-apply after the 0.1.33 code fix

### Notes
- Runtime one-shot hang fix remains as described under [0.1.33]; 0.1.34 is the publishable docs-complete release

## [0.1.33] - 2026-07-19

### Fixed
- CRITICAL one-shot: `replace --fuzzy` no longer infinite-loops when `replacement` contains `pattern` (agent “expand section” footgun). Was: re-scan full buffer with `max_replacements = usize::MAX` → multi-day 100% CPU / multi-GB RSS.
- Fuzzy path uses one-pass apply (`apply_fuzzy_one_pass`): never re-searches text just inserted; default max applies = 1; embeds force 1; hard ceiling 10_000.
- Cooperative cancel polled inside fuzzy cascade (strategies + sliding windows) so `--timeout-secs` works mid-file.
- Pattern size cap (64 KiB), levenshtein char cap (8 KiB), window budget (4096), buffer growth cap (4× / +16 MiB).

### Changed
- Global `--timeout-secs` default is 120 (was 0). Pass `0` to disable (not recommended for agents).
- Version 0.1.33.

### Tests
- `tests/cli_v0133_oneshot_fuzzy.rs` — hang regression wall-clock < 2s.
- Unit: `one_pass_embeds_pattern_applies_once`.

## [0.1.32] - 2026-07-15

### Changed
- Removed public contribute/community surface: `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` (EN/PT-BR) and README Contributing sections
- No GitHub community “Contribute” / Code of Conduct files on the migrated repository


## [0.1.31] - 2026-07-15

### Changed
- Migrated crate ownership and source repository to GitHub account `danilo-aguiar-br`
- `Cargo.toml` `repository` and `homepage` now point to https://github.com/danilo-aguiar-br/atomwrite
- Author contact email updated to `daniloaguiarbr@proton.me`
- Removed GitHub Actions workflows and CI badges (no Actions in the new repository)

### Notes
- crates.io package ownership invited/accepted under `danilo-aguiar-br`
- Publish and metadata refresh require this new version (repository URL only updates on publish)


## [0.1.30] - 2026-07-13

### Added
- Fuzzy product policy: auto/aggressive only; --fuzzy off rejected
- edit --replace-all with uniqueness requirement and match_count
- Escape-drift guard; indent delta; unicode_normalized strategy
- best_candidate always ranked; candidates[]; diff_preview filled
- MatchAmbiguous exit 65 / MATCH_AMBIGUOUS
- search --target content|files|both with --offset and --limit
- similar_paths on NotFound when workspace is known
- replace progress pre-count total rate_per_s eta_ms
- recipe default exclude *.bak.* and progress_every 50
- sparse outline budgeted subcommand
- Honest backup_method reflink_or_copy
- Agent-surface notes for mandatory fuzzy and watch install

### Changed
- Version 0.1.30
- Skills EN/PT fuzzy off removed from recommended surface

### Fixed
- semantic-search tokenizer splits snake_case and camelCase
- semantic-merge docs and help: line-based three-way honesty (not AST)
- EditOutput NDJSON serializes match_count and indent_adjusted (P17/P18)
- recipe/hash recursive walk skips *.bak.* backups (P19)
- config [fuzzy] mode off rejected; fuzzy section applied to edit/replace/loop/batch (P20)
- sparse outline emits real AST outline_item under budget (P21)
- agent-contract e2e tests for residual NDJSON fields (P23)


## [0.1.29] - 2026-07-13

### Added
- Shared `fuzzy` module: 9-strategy cascade for edit, replace, batch, edit-loop (P0-1)
- `best_candidate` structured diagnostics on match failure (P0-2)
- `replace --fuzzy auto|off|aggressive` and `--fuzzy-threshold`
- Cooperative cancel during stdin read and chunked atomic write (P0-4)
- Cargo features: `core`, `ast`, `lang-*`, `watch`, `semantic`, `full` (P0-3)
- CI `size-gate` job: slim core release must be ≤ 15 MiB; `core-test` job; `schema-diff` job
- `semantic-merge` three-way merge with shutdown checks (P1-1)
- `sparse list|read` budgeted monorepo surface (P1-2)
- `replace --progress-every` and batch progress heartbeats (P1-3)
- `recipe list|run` with real dispatch search→replace→hash (P1-4)
- Versioned recipe docs under `recipes/*.yaml`
- Backup prefers hardlink before reflink/copy (P2-3)
- `stat` subcommand alias for `read --stat` (P2-4)
- `agent-surface` clap-derived inventory (P2-5)
- `write --durability full|fast|auto` with NDJSON `platform.durability` (P2-1)
- Linux `renameat2` in `atomic_rename` with ENOSYS fallback (P2-2)
- `watch` debounce coalesce + optional `--checksum` + gitignore (P3-1, feature watch)
- `semantic-search --index-dir` offline inverted index (P3-2)
- `codemod` campaign `codemod_summary.by_rule_id` (P3-3)
- Regenerated schemas: write durability/rename_method, replace fuzzy*, error best_candidate, recipe/progress/cancelled

### Changed
- Default fixed-string `replace` uses fuzzy=auto after exact multi-match misses (breaking vs 0.1.28 exact-only exit 1)
- Strategy numbering normalized to nine named strategies
- At release: skill surface documents 41 subcommands (was 33)

### Fixed
- Match failures no longer discard near-miss scores
- Recipe run no longer plan-only stub
- Clippy `result_large_err` via boxed `BestCandidate`
- Core-only build works without AST crates (stubs exit 78)

### Binary size (measured honesty)
- Slim (`--no-default-features --features core`): ~7.7 MiB (CI asserts ≤ 15 MiB)
- Default/full with AST: ~52 MB — PRD 5–8 MB applies to core only, not default AST build


## [0.1.28] - 2026-07-06

### BREAKING Changes
- `delete` now creates a backup by default (was opt-in) — pass `--no-backup` to disable; `--keep-backup` on `delete` is redundant (deletion backups are always preserved and never auto-removed) and now surfaces a `warnings` field in the NDJSON envelope instead of a silent no-op
- `move`/`copy` overwriting an existing destination now require `--force` OR an explicit `--backup` — previously the two subcommands enforced this inconsistently

### Bug Fixes (Critical) — GAP-CLI-SURFACE-DRIFT
- Extracted a single `BackupOpts` struct (`--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`) flattened via `#[command(flatten)]` into 15 mutating subcommands: `write`, `edit`, `edit-loop`, `replace`, `transform`, `scope`, `apply`, `set`, `del`, `case`, `batch`, `delete`, `move`, `copy`, `rollback`
- `replace --retention` and `delete --no-backup` now parse successfully — previously both returned exit 2 `ARGUMENT_PARSE_ERROR` because each struct redeclared the backup flags by hand with divergent presence and defaults
- `--backup` and `--no-backup` are now mutually exclusive via `conflicts_with` (exit 2) instead of the last flag silently winning
- `rollback` keeps its pre-rollback safety snapshot opt-in via explicit `--backup`, documented as an intentional exception to the unified default-true contract
- Added a shared `edit` stdin-tty guard: `--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between`, and `--multi` now fail fast with exit 65 `INVALID_INPUT` and an actionable `suggestion` when stdin is a terminal, instead of blocking indefinitely (previously required an external timeout and exited 143)
- `edit`'s stdin-consuming modes now reject `--old`/`--new`/`--old-file`/`--new-file` at parse time via `conflicts_with_all` (exit 2) instead of silently dispatching into `edit_by_marker`
- Removed hardcoded `retention: 5` from 9 call sites; retention now flows end-to-end from `BackupOpts` through a single `resolve_backup()`
- `batch --retention <N>` was accepted but silently ignored — now effective end-to-end with precedence CLI `--retention` > `.atomwrite.toml` `[defaults].retention` > built-in default (`5`), including the batch's transactional pre-backup step
- `batch --backup` passed explicitly now forces a backup for every operation in the batch, symmetric to `--no-backup`
- Removed the last hardcoded `retention: 5` sites in `batch.rs` (transactional pre-backup and the `delete` operation) — both now honor the resolved retention
- `Default` impls in `config.rs` and `atomic.rs` now read `constants::DEFAULT_BACKUP_RETENTION` instead of a duplicated literal

### Bug Fixes (High) — GAP-CONFIG-DEFAULTS-DEAD
- `.atomwrite.toml` `[defaults]` `backup`/`retention` keys are now effective across every mutating subcommand — `resolve_backup()` previously took only `(backup: bool, no_backup: bool)` and never consulted the parsed config
- Precedence is now `ATOMWRITE_BACKUP` env var > CLI flags (`--backup`/`--no-backup`/`--retention`) > `.atomwrite.toml` `[defaults]` > built-in default (`true` / `5`)

### Validation
- `cargo test` — 661 tests pass (0 failures, 3 ignored)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diffs
- New test files: `cli_v0128_backup_matrix.rs` (12 tests), `cli_v0128_config_defaults.rs` (6 tests), `cli_v0128_edit_stdin_guard.rs` (3 tests), `cli_v0128_batch_backup.rs` (6 tests)


## [0.1.27] - 2026-06-24

### Bug Fixes (Critical) — Security
- BUG-SEC-001: Symlink-directory escape from workspace jail — `ln -s /tmp $WS/link && atomwrite write link/file` could create files OUTSIDE the jail. Fixed with `canonicalize_existing_prefix` in `path_safety.rs` that resolves symlinks before jail check. Affected: write, read, edit, edit-loop, apply, set, del, copy, move. NOT affected: search, replace (own symlink checks via WalkBuilder)
- BUG-SCOPE-004: `scope --query comments --delete` destroyed code on lines with inline comments — `fn foo() {} // comment` deleted the ENTIRE line including `fn foo() {}`. Fixed `expand_to_full_line` to detect code before the match and preserve it

### Bug Fixes (Medium)
- BUG-001: `edit-loop` accepted empty stdin returning exit 0 — now returns exit 65 INVALID_INPUT
- BUG-002: `edit-loop` with invalid JSON emitted raw text on stderr instead of NDJSON envelope — fixed with `map_err` to `AtomwriteError::InvalidInput`
- BUG-005: `--syntax-check` error suggestion said `--syntax-check=false` (invalid clap syntax) — fixed to "remove --syntax-check"
- BUG-GET: `get` with missing key returned exit 4 (FILE_NOT_FOUND) — now returns exit 65 (INVALID_INPUT) with message "key 'X' not found in Y"
- BUG-008: `edit-loop` `pair_results` only had `[index, matched]` — now includes `old` and `new` fields for traceability
- BUG-SCOPE-002: `scope` queries `fn`, `struct`, `enum`, `trait`, `async-fn`, `unsafe-fn` missed `pub` items — ast-grep treats `pub` as a structural AST modifier; added `pub` variant patterns
- BUG-SCOPE-003: `scope` queries `const`, `static`, `type-alias`, `mod`, `use` missed `pub` items — same root cause as BUG-SCOPE-002; added `pub` variant patterns

### Bug Fixes (Low)
- BUG-SCOPE-005: Go `scope --query var` missed type-inferred declarations (`var x = 0`) — added pattern `var $NAME = $$$EXPR`; refactored `lookup_go_query` to `lookup_go_queries` returning `Vec<String>`

### Known Limitations
- GAP-01: `scope --query test-fn` unavailable — ast-grep cannot match patterns spanning multiple AST nodes (`#[test]` + `fn`). Workaround: `--pattern "#[test]"` or `query -Q "(function_item (attribute_item) @attr)"`
- GAP-02: `scope --query doc-comment` unavailable — tree-sitter parses `///` as `line_comment` identical to `//`; no structural distinction. Workaround: `--query comments` for all comments, or `rg "///"`
- GAP-03: `scope --query export` in JS/TS unavailable — pattern `export $$$DECL` spans multiple AST nodes. Same ast-grep limitation as GAP-01

### Validation
- `cargo test` — 631+ tests pass (0 failures, 3 ignored)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diffs
- E2E audit: 3 rounds, ~180 additional scenarios, 10 bugs fixed, 3 known limitations documented


## [0.1.26] - 2026-06-23

### Bug Fixes (High) — Exit Code Consistency
- GAP-141: `replace` with zero matches now returns exit 1 (NO_MATCHES) — was silently returning exit 0
- GAP-145: `transform` with zero matches now returns exit 1 (NO_MATCHES) — was silently returning exit 0
- GAP-146: `scope` with zero matches now returns exit 1 (NO_MATCHES) — was silently returning exit 0
- GAP-147: `search --sort path` now guarantees deterministic global ordering — was non-deterministic with parallel walker

### Bug Fixes (Medium)
- GAP-136: `read --verify-checksum` with wrong hash now returns exit 81 (CHECKSUM_VERIFY_FAILED) — was returning exit 82 (STATE_DRIFT)
- GAP-137: `get` for missing key now returns exit 4 (NOT_FOUND) — was returning exit 65 (INVALID_INPUT), reverting GAP-111 from v0.1.25
- GAP-144: `scope` Rust query patterns (fn, pub-fn, async-fn, unsafe-fn, struct, pub-struct, enum, pub-enum, trait, impl) now match return types, generics, and trait implementations via dual AST patterns

### Bug Fixes (Low)
- GAP-135: `verify` subcommand now takes positional arguments: `verify <PATH> <HASH>` — was requiring `--checksum` flag
- GAP-142: `edit-loop` now accepts JSON array input `[{...},{...}]` in addition to NDJSON (one object per line)

### Documentation Clarifications (BY-DESIGN)
- GAP-138: `query --kinds` output is NDJSON stream of `{type:"query_kind"}` per kind (not a single JSON object with `kinds` array)
- GAP-139: `case --to <style>` requires `--subvert OLD NEW` pairs (global identifier scanning is not implemented)
- GAP-140: `write` with empty stdin requires `--allow-empty-stdin` flag (guard G120 is intentional)
- GAP-143: `prune-backups` flag is `--max-age-secs` (not `--max-age`) to make the unit explicit

### Validation
- `cargo test` — 631 tests pass (0 failures, 3 ignored)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diffs
- E2E audit: 535 scenarios across 11 rounds against real binary
- 13 gaps audited: 9 CODE resolved, 4 BY-DESIGN documented


## [0.1.25] - 2026-06-22

### New Features
- GAP-072: `.atomwrite.toml` configuration file with hierarchy: CLI > env > local > XDG global > defaults
- GAP-079: `verify` subcommand (delegates to `hash --verify` for BLAKE3 checksum verification)
- GAP-085: Jaro-Winkler fuzzy matching (`context_aware_jw` strategy) for short strings in edit
- GAP-086: Property-based tests for fuzzy matching via proptest (5 properties)
- GAP-087: `edit --fuzzy-threshold <FLOAT>` for configurable fuzzy match sensitivity
- GAP-088: Fuzzy diff highlighting via `similar::TextDiff` in `FuzzyInfo.diff_preview`
- GAP-081: `scope` actions `symbols` (ASCII→Unicode operators) and `normalize` (NFC normalization)

### Bug Fixes (Critical)
- GAP-101: `write --backup` no longer reports phantom `backup_path` for auto-deleted backups
- GAP-102: `set` no longer misroutes key when descending into scalar TOML values (now exits 65)
- GAP-127: `case` without `--subvert` now returns exit 65 with clear message instead of silent no-op

### Bug Fixes (High)
- GAP-073/098: I/O errors via `anyhow::Context` now emit NDJSON envelope instead of raw text on stderr
- GAP-084: Removed all `.unwrap()` calls from production code paths (replaced with typed errors)
- GAP-093: Flag conflict validation via `conflicts_with` for append/prepend, fixed/regex, literal/regex
- GAP-103: `copy --preserve` now preserves source file permissions (Unix)
- GAP-104: `copy --backup` now retains `.bak` file on disk (was auto-deleted)
- GAP-105: `replace --backup` now retains `.bak` file on disk (was auto-deleted)
- GAP-106: `write --require-backup` now forces backup retention on disk
- GAP-120: `size_delta_pct` in risk_assessment changed from u8 to u32 (fixes overflow for deltas >255%)

### Bug Fixes (Medium)
- GAP-075: `delete --older-than` with human-readable duration (s/m/h/d/w)
- GAP-076: `delete --confirm` as preview mode (same as `--dry-run`)
- GAP-077: `replace --preserve-case` with `adapt_case()` (UPPER/lower/Title)
- GAP-078: `search --pcre2` flag returns exit 65 when PCRE2 feature is not enabled
- GAP-080: `transform --verify-parse` re-parses output with tree-sitter after rewrite
- GAP-091: `edit --multi` help text updated to mention `--fuzzy` mode inheritance
- GAP-095: `replace --preserve-case` regex now case-insensitive (added `(?i)` to pattern)
- GAP-096: `write --preserve-timestamps` now emits `mtime_preserved` field in NDJSON
- GAP-097: 3 JSON schemas updated to match real binary output (del-result, write-output, read-output)
- GAP-107: `hash` output field renamed from `value` to `checksum` (matches schema)
- GAP-108: `batch` move/copy now requires `"force":true` to overwrite existing targets
- GAP-109: `outline --positions` now emits `start_byte`, `end_byte`, `start_column`, `end_column`
- GAP-110: `list` on non-existent directory now returns exit 4 (was exit 0 with empty summary)
- GAP-111: `get`/`del` on missing key returns INVALID_INPUT (exit 65) instead of FILE_NOT_FOUND (exit 4)
- GAP-112: `move --backup` now includes `backup_path` in NDJSON output
- GAP-116: `list --long` modified field now emits ISO 8601 format instead of raw epoch
- GAP-117: Documented flags `--no-reflink`, `--preserve-xattr`, `--preserve-hardlinks` added to CLI
- GAP-118: `copy --backup` no longer requires `--force` to overwrite (parity with `move --backup`)
- GAP-119: `write --auto-rotate` now retains backup on disk
- GAP-121: `--json-schema` for hash, delete, move, copy now emits correct type-specific schemas
- GAP-123: `scope --query comments` now captures block comments (`/* */`) in addition to line comments
- GAP-124: `prescan_json_schema()` now recognizes get, del, set, outline, query, case subcommands
- GAP-128: `list --count-by-ext` now filters backup timestamp suffixes (consistent with `count --by-extension`)
- GAP-133: `copy --preserve` now copies source mtime/atime via `filetime::set_file_times()`
- GAP-134: `scope --query test-fn` returns actionable error instead of cryptic ast-grep multi-node error

### Bug Fixes (Low)
- GAP-099: Fixed 2 clippy warnings in test files (collapsible_if, len_zero)
- GAP-100: Fixed 27 rustfmt divergences across source and test files
- GAP-113: Risk telemetry default changed to 255 (disabled) — user must opt-in via `--risk-threshold`
- GAP-114: `extract` now filters begin/end/summary events from search pipeline
- GAP-115: `wal-stats --dry-run` now emits NDJSON plan event (was silent)

### Validation
- `cargo test` — 631 tests pass (0 failures, 3 ignored)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diffs
- E2E audit: ~505 scenarios across 6 rounds against real binary
- 64 gaps audited total: 49 resolved, 5 pre-existing, 10 reclassified, 0 pending


## [0.1.24] - 2026-06-21

### Bug Fixes (Critical)
- GAP-027: `delete --recursive` now traverses directories (was silently skipping)
- GAP-035: `read --line/--lines` no longer panics on out-of-range indices
- GAP-037: `search --multiline` now correctly propagates flag to SearcherBuilder
- GAP-046: `batch --transaction` now properly reverts `move`/`copy` operations on rollback
- GAP-050: `replace` rejects empty pattern (previously destroyed all files silently)

### Bug Fixes (Medium)
- GAP-019: `ARGUMENT_PARSE_ERROR` (exit 2) gains context-aware `suggestion` field pointing to `--old-file`/`--new-file` (ADR-0045)
- GAP-020: `diff` now resolves paths against `--workspace` (ADR-0046)
- GAP-021: `scope` read-only mode now correctly reports `files_matched` (ADR-0047)
- GAP-022: `scope`/`count`/`transform` now resolve walk roots against `--workspace`
- GAP-023: Backup timestamp format changed to `YYYYMMDD_HHMMSS_mmm` (millisecond resolution prevents collisions)
- GAP-028: `scope --query comments --delete` now removes entire `line_comment` AST nodes
- GAP-029: `wal-stats`/`wal-heal` NDJSON output now includes `type` field
- GAP-032: `hash --stdin` no longer requires PATHS argument (`required_unless_present = "stdin"`)
- GAP-033: `--require-backup` now correctly detects when backup is disabled via `--no-backup`
- GAP-036: `edit --multi` field `op` is now optional (inferred as `"exact"` when `old`+`new` present)
- GAP-038: `delete --recursive` now removes empty subdirectories (depth-first traversal)
- GAP-039: `get` JSON values no longer doubly-quoted (returns raw string value)
- GAP-040: `get` of non-existent key returns exit 4 (`NOT_FOUND`) with JSON envelope
- GAP-042: `get` TOML string values no longer doubly-quoted
- GAP-043: `set`/`del` `old_value`/`removed_value` no longer doubly-quoted
- GAP-044: `hash` of non-existent file returns exit 4 (`NOT_FOUND`) with JSON envelope
- GAP-045: `set` TOML now correctly reads nested key old_value via manual table descent
- GAP-047: `case --subvert` changed from `num_args=2..` (greedy) to `num_args=2` (exact pair)
- GAP-048: `hash --recursive` now implements directory traversal via WalkBuilder
- GAP-049: `prune-backups --max-count` now sorts by filename (lexicographic) instead of mtime

### Bug Fixes (Low)
- GAP-024: `write` risk_assessment skipped for `--append`/`--prepend` (no data loss possible)
- GAP-025: `regex` removed `allow_hyphen_values`; use POSIX `--` separator for hyphen-starting examples
- GAP-026: `scope` summary no longer reports `files_modified` in read-only mode
- GAP-030: `read --format raw` skips binary heuristic (no longer suggests non-existent `--force-text`)
- GAP-031: `edit --multi` format corrected (field `op` now optional)
- GAP-034: `regex` flags after positionals warning added (duplicate fix of GAP-025)
- GAP-041: `read --lines/--head/--tail` with empty range returns empty content instead of `"\n"`

### Typed Error Audit (GAP-051 through GAP-070)
- Converted 20 `anyhow::bail!()` calls to typed `AtomwriteError` variants
- All user-facing errors now emit structured JSON on stdout with correct exit codes
- Exit code mapping: `NotFound` → exit 4, `InvalidInput` → exit 65
- Commands affected: `set`, `del`, `get`, `query`, `outline`, `edit`, `batch`, `write`, `prune-backups`, `extract`
- GAP-051: `set`/`del` file not found → exit 4 with JSON
- GAP-052: `del` key not found without `--force-missing` → exit 4 with JSON
- GAP-053: `query`/`outline` file not found → exit 4 with JSON
- GAP-054: `edit --delete-range`/`--range` inverted → exit 65 with JSON
- GAP-055: `set`/`del`/`get` unsupported format → exit 65 with JSON
- GAP-056: `batch` empty manifest → exit 65 with JSON
- GAP-057: `query` no mode specified → exit 65 with JSON
- GAP-058: `query --language` unsupported → exit 65 with JSON
- GAP-059: `query` language detection failed → exit 65 with JSON
- GAP-060: `edit` no mode specified → exit 65 with JSON
- GAP-061: `edit --old/--new` count mismatch → exit 65 with JSON
- GAP-062: `edit --multi` empty stdin → exit 65 with JSON
- GAP-063: `edit --between` with 1 marker → exit 65 with JSON
- GAP-064: `edit` no line/marker operation → exit 65 with JSON
- GAP-065: `write --confirm` aborted → exit 65 with JSON
- GAP-066: `batch` with failed operations → exit 65 with JSON
- GAP-067: `batch` transaction rollback → exit 65 with JSON
- GAP-068: `prune-backups` without filter → exit 65 with JSON
- GAP-069: `extract` JSON nesting overflow → exit 65 with JSON
- GAP-070: `edit --multi` per-op validation → exit 65 with JSON

### ADRs
- ADR-0045: Actionable suggestion for clap parse errors
- ADR-0046: diff resolve-first retrofit
- ADR-0047: scope read-only mode fix

### Validation
- `cargo test` — 621 tests pass (0 failures, 3 ignored)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diffs


## [0.1.23] - 2026-06-19

### GAP-2026-015 — `allow_hyphen_values` for 15 CLI fields across 8 structs
- Added `allow_hyphen_values = true` to 15 text-accepting fields in `EditArgs`, `SearchArgs`, `ReplaceArgs`, `CalcArgs`, `RegexArgs`, `TransformArgs`, `ReadArgs`, `QueryArgs`
- Fixes exit 2 (`ARGUMENT_PARSE_ERROR`) when values start with `-` (Markdown bullets, negative numbers, YAML entries, diff content)
- Excluded: `CaseArgs.subvert` (`num_args = 2..` incompatible with `allow_hyphen_values`)
- ADR: `docs/decisions/0041-allow-hyphen-values-edit.md`
- 12 new regression tests in `tests/cli_v0123_hyphen_values.rs`

### GAP-2026-016 — backup-by-default for content-mutating commands
- Changed `backup` default from `false` to `true` in 9 content-mutating structs: `WriteArgs`, `EditArgs`, `EditLoopArgs`, `ReplaceArgs`, `TransformArgs`, `ApplyArgs`, `SetArgs`, `DelArgs`, `CaseArgs`
- Changed `AtomicWriteOptions::default().backup` from `false` to `true`
- Added `--no-backup` flag to all 9 structs for explicit opt-out
- Added `ATOMWRITE_BACKUP=0` environment variable for global opt-out
- Added `resolve_backup()` helper in `src/commands/mod.rs`
- Existing `keep_backup: false` default unchanged — backup auto-deleted after success
- 4 non-content structs unchanged: `DeleteArgs`, `MoveArgs`, `CopyArgs`, `RollbackArgs`
- ADR: `docs/decisions/0042-backup-by-default.md`
- 7 new regression tests in `tests/cli_v0123_backup_default.rs`

### GAP-2026-017 — shrink guard with --expect-checksum
- Added `--allow-shrink` flag to `WriteArgs`
- Writes that shrink file by >50% are now BLOCKED when `--expect-checksum` is active
- Returns exit 65 (`INVALID_INPUT`) with suggestion to pass `--allow-shrink`
- Made `risk_assessment` (L1) blocking when `--expect-checksum` is active and file shrinks
- Without `--expect-checksum`, behavior is unchanged (backward compatible)
- ADR: `docs/decisions/0043-shrink-guard.md`
- 4 new regression tests in `tests/cli_v0123_shrink_guard.rs`

### GAP-2026-018 — --old-file/--new-file for edit command
- Added `--old-file <PATH>` and `--new-file <PATH>` flags to `EditArgs` as alternatives to `--old`/`--new`
- Content is read from files inside the atomwrite process, bypassing shell expansion and kernel ARG_MAX (~131 KB limit)
- `conflicts_with` prevents mixing `--old` with `--old-file` (exit 2 on conflict)
- Paths validated against workspace jail (`validate_path`)
- Added `source` field to `PairResult` ("arg" or "file") for content origin traceability
- Trailing newline stripping: `strip_file_trailing_newline()` strips exactly one trailing `\n` or `\r\n` from file content for parity with argv behavior (files created by `echo` have trailing newlines that argv values don't)
- Cross-mixing validation: runtime guard rejects `--old` + `--new-file` and `--old-file` + `--new` with exit 65 (`INVALID_INPUT`) and message "cannot mix --old with --new-file or --old-file with --new"
- ADR: `docs/decisions/0044-edit-old-file-new-file.md`
- 8 new regression tests in `tests/cli_v0123_old_file.rs`

### Validation
- `cargo test` — all tests pass (609 total, 31 new for v0.1.23)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diffs


## [0.1.22] - 2026-06-17

### Added
- `prune-backups` subcommand for manual cleanup of legacy backups (flags `--max-age`, `--max-count`, `--dry-run`)
- `edit-loop` subcommand for N edits in one invocation, read as NDJSON from stdin
- ADR-0039 (`docs/decisions/0039-edit-loop-helper.md`)
- ADR-0040 (`docs/decisions/0040-prune-backups-subcommand.md`)
- 2 NDJSON schemas (`prune-backups-output.schema.json`, `edit-loop-output.schema.json`)
- 18 rust rules applied (5 of them new in the graphrag)

### Tests
- 16 new regression tests (3+4+3+3+2+2)
- 2 new property tests under the `slow-tests` feature
- Coverage ≥ 80% on the new code

### Documentation
- `[FECHADO v0.1.21]` markers in `gaps.md` for 3 gaps
- "Padrão Correto — Edits Sequenciais com Re-captura de Checksum" section in the EN/PT SKILLs
- Copy-paste `while` loop example in `docs/HOW_TO_USE.md`
- v0.1.21 sections in `docs/AGENTS.pt-BR.md`


## [0.1.21] - 2026-06-17

### GAP-2026-012 — `--allow-sequential-drift` for sequential `edit` pipelines
- Context — Agents that chain multiple `edit` calls on the same file without re-capturing `checksum_after` between invocations receive `STATE_DRIFT` (exit 82) on every call after the first. Documentation covered the parallel scenario but not the sequential one (single agent, single file, lock-step edits).
- Fix — new `--allow-sequential-drift` opt-in flag on `edit` — when set, `cmd_edit` emits `tracing::warn!` naming the drift and proceeds with the edit (exit 0 on success). The default behavior is unchanged: `STATE_DRIFT` (exit 82) still fires on checksum mismatch when the flag is absent. Two valid patterns for sequential pipelines: (a) re-capture `checksum_after` after each `edit` and pass it to the next call; (b) pass `--allow-sequential-drift` once on every call and let each call's pre-state differ from the original. See `SKILL.md` for the `while` loop recipe and `docs/HOW_TO_USE.md` for the copy-paste example.

### GAP-2026-013 Problem C — `--backup` and `--keep-backup` exposed on `edit`, `rollback`, `replace`, `apply`, `batch`
- Bug (API parity violation) — `edit` and `rollback` hardcoded `backup: false` in `AtomicWriteOptions` while `write` and `replace` exposed `--backup`. Users who tried `--backup` on `edit`/`rollback` were silently ignored. The `ReplaceArgs`, `ApplyArgs`, and `BatchArgs` structs had the same hole.
- Fix — `--backup`, `--retention`, and `--keep-backup` plumbed through 6 subcommands — `edit` gains `backup`, `retention`, `keep_backup`; `rollback` gains `backup`, `keep_backup`; `replace`, `apply`, `batch` gain `keep_backup`. The 3 hardcoded `backup: false` sites in `src/commands/edit.rs:139`, `src/commands/edit.rs:393`, and `src/commands/rollback.rs:108` are replaced with `args.backup`. Subcommand parity for `--backup` is now 4/4 (write, edit, replace, rollback); 6/6 subcommands honor `--keep-backup` (write, edit, replace, rollback, apply, batch).

### GAP-2026-014 v2 — backups are deleted after successful writes by default
- Context — `cleanup_old_backups_in` pruned by count, leaving backups live indefinitely until 5 newer ones pushed them out. Every successful `--backup` operation left persistent trash on disk; CI scripts that ran `fd '*.bak.*' . | wc -l` saw growing counts proportional to write volume.
- Fix — `keep_backup: bool` on `AtomicWriteOptions`, default `false` — new helper `delete_backup_quietly(path)` removes the backup after `atomic_write_inner` returns success. `ErrorKind::NotFound` is mapped to `Ok(())` (idempotence). On non-NotFound errors, `tracing::warn!` is emitted and the operation proceeds (cleanup is logged, not propagated). On failure paths the backup is preserved as before. `keep_backup: true` is the explicit opt-in to preserve the backup; the `--backup` flag's previous behavior of leaving backups on disk is now reachable only via `--keep-backup`. 6 subcommands accept the flag: `write`, `edit`, `replace`, `rollback`, `apply`, `batch`. See `docs/decisions/0038-backup-cumprido-deleta.md`.

### Parity — `apply` and `batch` now honor `--keep-backup`
- `apply` propagates `args.keep_backup` to the inner `atomic_write` call so a successful patch no longer leaves a `.bak` sibling behind by default.
- `batch` propagates `--keep-backup` to every `write`/`edit`/`replace` op in the NDJSON manifest. Per-op `keep_backup` in the NDJSON overrides the batch-level default.

### ADR
- ADR-0038 — backup cumprido deleta: `keep_backup` default `false` + `delete_backup_quietly` helper rationale; alternatives rejected are scheduler, prune-backups subcommand, and age-based cleanup (all subsumed by deletion-after-success).

### Migration Notes
- Breaking change — `write --backup` and `replace --backup` no longer leave a `.bak` sibling on disk after a successful write. The pre-v0.1.21 behavior of "backup lives forever" is gone. Add `--keep-backup` to any script that depends on the backup persisting across the operation, or rewrite to read the backup before the write completes.
- Breaking change — `edit` and `rollback` now accept `--backup` but ignore it without complaint if the atomic layer's pre-conditions reject it. The new opt-in is the explicit `--backup` flag; old scripts that called `edit` with the assumption of "no backup" still get "no backup" by default.
- Non-breaking — `apply --keep-backup` and `batch --keep-backup` are additive. Default behavior (no backup) is unchanged.

### Validation
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 555+ tests passing (542 baseline v0.1.20 + 13 new: 6 in `cli_v0121_backup_keep_flag`, 2 in `cli_v0121_edit_backup`, 3 in `cli_v0121_sequential_drift`, 1 in `cli_v0121_rollback_backup`, 1 in `cli_v0121_apply_keep`, 1 in `cli_v0121_batch_keep`, 1 in `proptest_v0121_backup_delete`)
- 1 new ADR: 0038 (backup cumprido deleta)
- 3 new GAP-2026 closures (012, 013 Problem C, 014 v2)
- Cross-compile verified in 3 Windows targets: x86_64-gnu, i686-gnu, x86_64-msvc
- Migration smoke test: `fd '*.bak.*' . | wc -l` reports 0 on a fresh post-success run; reports 1 when `--keep-backup` is set

## [0.1.20] - 2026-06-15

### GAP-2026-001 — `count --by-size` finally implements the help flag
- Bug — `cmd_count` ignored `args.by_size` despite the struct field existing in `src/cli_args.rs`. Help promised top-N by size; code always returned `mode: "lines"`. HELP-FIRST-DRIFT (flag announced in help before implementation existed).
- Fix — New `CountBySizeOutput` struct in `ndjson_types.rs`. `cmd_count` collects `Vec<(PathBuf, u64)>` during the walk, sorts descending by size, truncates to `args.top` (default 10), and emits the top-N output.

### GAP-2026-002 — `write --preserve-timestamps` (parity with edit/replace)
- Bug — `cmd_write` hardcoded `preserve_timestamps: false` in `AtomicWriteOptions` while 5 of 6 mutating subcommands exposed the flag.
- Fix — Added `preserve_timestamps: bool` to `WriteArgs`; `cmd_write` now passes `args.preserve_timestamps` to the atomic layer.

### GAP-2026-003 — `scope --lang` alias
- Bug — `ScopeArgs.language` was declared with `short='l' long="language"` only, with no `alias = "lang"`. Users following the SKILL docs that mention `--lang` got `ARGUMENT_PARSE_ERROR`.
- Fix — Added `alias = "lang"` to the `language` field.

### GAP-2026-004 — `write --line-ending crlf` (accepts both forms)
- Bug — `LineEnding::CrLf` was rendered by clap as `cr-lf` (kebab-case) while docs and prior user expectations wrote `crlf` (no hyphen).
- Fix — Added `#[value(name = "cr-lf", alias = "crlf")]` and equivalent `value` attributes to all 4 variants for explicit, stable CLI surface.

### GAP-2026-005b — `edit --partial` semantics documented
- Doc-only — `--partial` is implemented for multi-pair; single-pair failure path now returns `NoMatches` (exit 1) with no write. Documented in skill EN/PT.

### GAP-2026-006 — `diff --algorithm` regression tests
- Doc + tests — `algorithm: DiffAlgorithm` was already implemented and dispatched to `similar::Algorithm::{Myers,Patience,Lcs}`. Added regression tests for `myers` vs `patience` vs `lcs`.

### GAP-2026-007 — `count --by-extension` filters backup timestamps
- Bug — `validated.extension()` returned the last `.` component, so `foo.txt.bak.20260615_035515` was categorized as `20260615_035515`.
- Fix — New `BACKUP_RE` regex `\.bak\.\d{8}_\d{6}$` matches backup filenames and routes them to a dedicated `"backup"` category.

### GAP-2026-008 — `read --head/--line/--lines` reports filtered line count
- Bug — `ReadOutput.lines` reported the unfiltered file total even with `--head`, `--line`, or `--lines`.
- Fix — `line_count` is now computed from the filtered `content_str`; `lines_total` (new optional field) preserves the original file total for downstream consumers.

### GAP-2026-009 — `read` emits `mode` discriminator
- API — `ReadOutput` gains a `mode` field: `"full" | "head" | "tail" | "line" | "lines" | "grep" | "stat"`. Downstream consumers no longer need to parse content to discriminate partial reads from full reads.

### GAP-2026-010 — `search --no-begin-end` for cleaner empty-walk output
- API — New `no_begin_end: bool` flag on `SearchArgs`. Default (off) preserves pre-v0.1.20 behavior. With the flag, `begin`/`end` NDJSON events are suppressed for files with zero matches.

### GAP-2026-011 — `write` intention guards (defense-in-depth after 2026-06-15 incident)
- Incident — During a 2026-06-15 audit, `atomwrite write` was called without `--append` on `c24-framework34.html` (491827 bytes). The file was truncated to a few bytes; ~127 lines of 15-jun work (~9 KB) were lost.
- Fix — 5 new flags on `WriteArgs` (all opt-in, default off to preserve backward compat):
  - `--require-backup` (L2): abort with `InvalidInput` (exit 65) if target exists and `--backup` is not set.
  - `--confirm` (L3): when target exists and is larger than 100KB, prompt `Overwrite <path> (<N> bytes)? [y/N]` and read from stdin. Aborts on any answer other than `y`/`yes`.
  - `--auto-rotate` (L5): when `--backup` is active, force a rotation backup if the target was modified within the last 24 hours.
  - `--risk-threshold <PERCENT>` (default 50): L1 size-guard threshold; emits a stderr warning (`low`/`medium`/`high`) when the size delta exceeds the threshold.
  - Telemetry: `WriteOutput.risk_assessment` (optional, GAP-2026-011 L6) carries the original/new bytes, delta percentage, risk level, and which guard triggered.
### ADR
- ADR-0034 — help-driven testing anti-pattern: clap `--help` nunca mais declarado antes da implementação (5 dos 11 GAP-2026 tinham esse anti-pattern)
- ADR-0035 — write intention guards: 4 flags defense-in-depth (--require-backup, --confirm, --auto-rotate, --risk-threshold) + `risk_assessment` no envelope, motivadas pelo incident c24-framework34.html de 2026-06-15
- ADR-0036 — `edit --partial`: single-pair com zero matches retorna NO_MATCHES (exit 1); multi-pair aplica matched e relata unmatched em `pair_results`
- ADR-0037 — rename `--lang` global para `--locale` (env var `ATOMWRITE_LANG` permanece, campo `args.global.lang` permanece, namespace `--lang` liberado para subcomandos como alias de `--language`)

### Migration Notes
- Breaking change na CLI surface: scripts que passavam `--lang <locale>` devem migrar para `--locale <locale>` (one-liner: `rg -l '\\-\\-lang\\b' bin/ scripts/ && sd -- '\\-\\-lang\\b' '--locale' bin/ scripts/`)
- Env var `ATOMWRITE_LANG` e campo programático `args.global.lang` permanecem estáveis — CI matrices, container wrappers e consumidores Rust não precisam de mudança
- Subcomandos que tinham `--language` agora também aceitam `--lang` como alias (ex.: `atomwrite scope --lang rust`)

### Validation
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 542 testes passando em 47 suites (up from 515 in 46 suites in v0.1.19, +27 new: 11 GAP-2026 closures + 16 intention-guard tests)
- 4 novos ADRs: 0034 (help-driven testing), 0035 (write intention guards), 0036 (edit partial), 0037 (locale rename)
- 11 GAP-2026 fechados (001-011), 100% cobertura dos gaps de auditoria local
- Cross-compile verificado em 3 targets Windows: x86_64-gnu, i686-gnu, x86_64-msvc


## [0.1.19] - 2026-06-14

### G121 — `search` and `replace` resolve root paths against the workspace
- Bug (CWE-367 — TOCTOU/path-confusion) — `cmd_search` and `cmd_replace` took the caller-supplied root paths, validated them with `path_safety::validate_path`, then fed the ORIGINAL (CWD-relative) path to `ignore::WalkBuilder`. `validate_path` returned the canonical absolute path but the per-call result was discarded. When `CWD != --workspace`, the walker either (a) silently walked the wrong tree if a same-named path existed under CWD, or (b) produced a `JailViolation` event per file walked. The G118 fix in `write.rs:44` (ADR-0027) never propagated to these two commands because the per-entry `validate_path` inside the worker thread masked the missing pre-step resolution.
- Fix — new `path_resolution::resolve_paths_against_workspace` helper — `cmd_search` and `cmd_replace` now call this helper once at the top of the command (after `global.resolve_workspace()()`) and pass the canonical `Vec<PathBuf>` to `build_walker`. The pre-step `for path in &args.paths { validate_path(path, &workspace)?; }` loop in `replace.rs` is removed; search gets a parallel resolution call. Both `build_walker` signatures gain a `canonical_paths: &[PathBuf]` parameter; they no longer read `&args.paths[0]` directly.
- Consequence — Search and replace now honor `CWD != --workspace`. A relative path like `src/` passed with `--workspace /path/to/ws` walks `/path/to/ws/src/` regardless of process CWD. Out-of-jail paths fail once with `WORKSPACE_JAIL` (exit 126) at command start instead of per-file inside the worker. See `docs/decisions/0031-g121-path-resolution-helper.md`.

### G122 — Real S-expression matching in `query` subcommand
- Bug (silent feature, documented but never implemented) — The `query` subcommand (v14 Tier 3, introduced in v0.1.12) always promised S-expression support. The docs in `COOKBOOK.md`, `HOW_TO_USE.md` and both bilingual SKILL files show examples like `atomwrite --workspace . query src/main.rs --query "(function_item name: (identifier) @name)"`. In practice, `cmd_query` called `walk_kind_filter` which does `wanted.iter().any(|w| w == &kind)` — a literal STRING comparison with `node.kind()`. The whole string `"(function_item name: (identifier) @name)"` never matched any real `node.kind()`, so the S-expression feature never worked.
- Fix — new `QueryType` enum + auto-classification — `classify_pattern(pattern) -> QueryType` detects S-expression by the presence of `(`, `)`, or `@`. `walk_sexpr` compiles the pattern via `tree_sitter::Query::new`, executes via `QueryCursor::matches`, and emits `query_match` NDJSON with the new `capture_name` field for each `@capture`. `cmd_query` branches on the classification.
- New direct dep `tree-sitter = "0.26"` — the language-pack re-exports `Language` but `Query`/`QueryCursor`/`StreamingIterator` require the `tree-sitter` crate itself.
- Consequence — Patterns with `(`, `)`, or `@` are routed to `tree_sitter::Query::new`. Parse errors (e.g. `(unclosed`) return exit 1 with `invalid S-expression pattern: ...` message (via `anyhow::Context`). The kind-filter path is preserved bit-by-bit: users passing `--query function_item` (without S-expression chars) keep getting the same results. See `docs/decisions/0032-query-sexp-real-implementation.md`.

### Exit code documentation drift consolidation (ADR-0033)
- Context — Phase D testing on 2026-06-14 ran 7 concrete binary-level probes against the v0.1.18 release and surfaced 7 places where the published docs diverged from the actual binary behavior. The 7 drifts are:
  1. `STATE_DRIFT` (82) absorbs `CHECKSUM_VERIFY_FAILED` (81) for `--verify-checksum` — both are conflict class, retryable. The 81-code is now historical (preserved only for the `read` path BLAKE3 mismatch on file content).
  2. `--syntax-check` returns `SYNTAX_ERROR_DETECTED`, NOT `SYNTAX_ERROR` — the rename happened in the v0.1.12 G72 tree-sitter rollout but docs were not updated.
  3. `ORPHAN_JOURNAL` (93) is consultive, NOT auto-detected — the gate is `ATOMWRITE_WAL=1` OR `--strict-atomic`. The default `write` (v0.1.16 G119 `WalPolicy::Auto`) does not write a sidecar and therefore cannot detect orphans.
  4. `BROKEN_PIPE` (141) requires real SIGPIPE propagation — a simple `head -1` pipe does NOT trigger it. The v0.1.4+ SIGPIPE restoration puts the default disposition back, so the signal is only raised when the downstream consumer actively closes the pipe mid-stream.
  5. Binary file reads return exit 0 with `kind: binary` metadata, NOT exit 65 — the v0.1.4 `BINARY_FILE` heuristic was changed to emit a structured envelope and exit 0. The 65-code path now only fires for `read` without `--format raw` AND with the binary heuristic bypassed.
  6. Missing positional argument returns `ARGUMENT_PARSE_ERROR` (exit 2), NOT `INVALID_INPUT` (65) — clap-level argument errors are reported as exit 2. The 65-code is reserved for runtime content validation (malformed TOML, invalid regex, empty stdin default).
  7. Missing `--workspace` defaults to CWD, NOT an error — `--workspace` is documented as a flag with a CWD default, not a required argument. `WORKSPACE_JAIL` (126) only fires when an absolute path resolves outside the effective jail.
- Decision — Accept the binary behavior as canonical. Consolidate the docs in v0.1.19 to match. See `docs/decisions/0033-v0-1-19-exit-code-naming-drift-consolidation.md`.
- Note on `SYNTAX_ERROR` legacy name — the v0.1.12 docs used `SYNTAX_ERROR`; the binary in v0.1.18 emits `SYNTAX_ERROR_DETECTED`. The historical name is preserved only in prose for grep-ability.

### Validation
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 515 tests passing in 46 suites (up from 502 in 44 suites in v0.1.18, +13 new)
- 3 new ADRs: 0031 (path resolution helper), 0032 (query S-expression), 0033 (exit code drift consolidation)
## [0.1.18] - 2026-06-14

### G118 — `replace` pre-validates root paths against the workspace jail
- `cmd_replace` resolve-first para todas as raízes — após `global.resolve_workspace()`, o comando itera sobre `args.paths` e chama `path_safety::validate_path(path, &workspace)?` para CADA raiz ANTES de construir o `WalkBuilder`. Falha rápida com `WORKSPACE_JAIL` (exit 126) na primeira violação. Comportamento legado per-entry (v0.1.12-v0.1.17) emitia um evento `JailViolation` por arquivo caminhado e o usuário via o diagnóstico enterrado sob N eventos. Agora `replace /etc/passwd` aborta em microssegundos com um único envelope de erro estruturado.
- Convenção resolve-first agora universal — `write` (ADR-0027), `edit`, `copy`, `apply`, `move`, `rollback`, `set`, `del`, `case` e agora `replace` todos validam o alvo contra o jail workspace ANTES de qualquer `exists()` ou read. Um único modelo mental para todos os comandos mutantes.
- Teste de regressão atualizado — `replace_jail_violation_does_not_inflate_counter` em `tests/cli_v012_regressions.rs` agora afirma exit 126 + envelope `WORKSPACE_JAIL` + arquivos inside/outside inalterados. O nome é preservado (a invariante subjacente — "jail violation não pode inflar o counter de substituições" — é mantida) mas o corpo da asserção mudou.
- 2 novos testes integrados em `tests/cli_replace.rs`: `replace_root_path_outside_workspace_exits_126` (caminho absoluto `/etc/passwd`) e `replace_relative_dotdot_root_outside_workspace_exits_126` (caminho relativo `../escape`).

### G120 L3 — test coverage for cross-validation
- 2 novos testes integrados em `tests/cli_write.rs`:
  - `g120_l3_append_empty_stdin_with_matching_checksum_succeeds` — escreve arquivo seed, hasheia, roda `write --append --allow-empty-stdin --expect-checksum <HASH> < /dev/null`, afirma exit 0 + `stdin_bytes_read: 0` + arquivo inalterado (no-op append preserva checksum).
  - `g120_l3_append_empty_stdin_without_opt_in_rejects_at_l1` — sem `--allow-empty-stdin`, a guarda L1 dispara primeiro (exit 65) e o arquivo é preservado. Documenta que L3 é inalcançável sem opt-in explícita.

### G117 follow-up — edge case coverage
- 3 novos testes integrados em `tests/cli_edit.rs`:
  - `edit_unicode_old_new_exact_match` — diacríticos UTF-8 (`ção` → `AÇÃO`) com casamento exato byte-a-byte. Documenta o contrato de single-pair (substitui apenas a PRIMEIRA ocorrência; multi-pair para múltiplas).
  - `edit_crlf_line_endings_preserve_eol_after_replace` — input com `\r\n`, replace, afirma preservação byte-a-byte (sem colapso para `\n`).
  - `edit_multi_pair_same_old_appears_twice_applies_both` — multi-par onde ambos os pares referenciam o mesmo token `--old`. Garante que o segundo par consome a versão pós-primeiro-substituição (proteção contra off-by-one).

### ADR
- `docs/decisions/0030-v0-1-18-g118-replace-pre-validation-g120-l3-tests-g117-edge-cases.md` — registra as 3 decisões, alternativas consideradas e gatilhos para revisitar.

### Validation
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- Suíte completa de testes: 502 testes passando, 0 falhas, 0 regressões introduzidas pelas 3 mudanças
- 2 flakes pré-existentes confirmados como não relacionados (`signal_test::batch_interrupted_by_signal`, `tracing_test::span_captures_path_field` + `tracing_test::debug_level_includes_filter_info`) — falhas idênticas no baseline `git stash` antes das mudanças


## [0.1.17] - 2026-06-13

### G119 — wires L3 startup auto-heal and L4 into the Drop guard
- L3 startup pass — every invocation now calls `auto_heal_on_startup(&workspace, threshold_secs=3600, max_duration_ms=100)` once, BEFORE dispatching the subcommand. Default threshold is 1h, budget is 100ms, and `Started` orphans are NEVER reaped (only `Committed`/`Aborted` older than the threshold). On a 60-stale-sidecar workspace the pass reaps all of them in <5ms; on a 10k workspace the budget still bounds cost.
- New global flag `--no-auto-heal` — disables the L3 pass for tight CI loops, benchmarks, and forensics. Also bound to env `ATOMWRITE_WAL_NO_AUTO_HEAL=1`. Exists to keep per-invocation overhead predictable when the workspace has 0 sidecars (avoids the walk+parse cost on every command).
- L4 wired into `JournalGuard::drop` — the drop now consults `heuristics_should_preserve` before removing. `h1_ttl`, `h3_rate_limit`, `h4_sentinel`, `h5_archive` all vote; `h2_lru_within_cap` is intentionally bypassed (no cheap way to know global count from a per-file drop) by passing `u64::MAX` for both `workspace_committed_count` and `age_rank`. OR-composition: any preserve vote wins.
- New field `committed_at_unix: Option<u64>` on `JournalGuard` — `release()` stamps the current Unix timestamp so the L4 heuristics that reason about post-commit age (h1_ttl, h5_archive) have data. The inert guard and the `keep()` path leave it `None` (no Committed entry existed).
- Test fixes — 2 tests in `tests/cli_v012_wal.rs` (`wal_heal_reaps_stale_committed_journals`, `wal_stats_counts_committed_orphans_malformed`) and 2 in `tests/cli_wal.rs` (`g119_l5_wal_stats_reports_journal_state`, `g119_l5_wal_stats_reports_zeros_on_empty_workspace`) now pass `--no-auto-heal` to prevent the L3 startup pass from reaping pre-seeded stale sidecars before the assertion runs.
- AdR update — `docs/decisions/0028-g119-wal-cleanup-intelligent.md` gained a "Atualização v0.1.17 — Fiação de L3 startup + L4 no Drop guard" section documenting the `u64::MAX` trick, the `committed_at_unix` field, and the test-isolation rationale.

### Validation
- `cargo fmt --check` clean
- `cargo clippy --bin atomwrite --lib --all-targets -- -D warnings` clean (0 warnings)
- 6 new unit tests in `src/wal.rs::tests` for L3 (`l3_auto_heal_on_empty_workspace_reports_zero`, `l3_auto_heal_reaps_old_committed_preserves_started`, `l3_auto_heal_respects_budget`) and L4 (`l4_release_records_committed_at_unix`, `l4_drop_preserves_sidecar_when_h4_sentinel_votes`, `l4_drop_removes_sidecar_when_no_heuristic_preserves`)
- 3 new integration tests in `tests/cli_wal.rs` (`g119_l3_startup_auto_heal_reaps_stale_committed`, `g119_l3_no_auto_heal_preserves_stale_committed`, `g119_l4_sentinel_preserves_sidecar_on_successful_write`)
- Full suite: 474 tests passing, 0 failures, 0 regressions


## [0.1.16] - 2026-06-13

### G119 — closes the 5-layer autonomous cleanup (L1 prevention + L4 heuristics)
- L1 — `WalPolicy` enum + `--wal-policy` flag — `Auto` (default) skips the sidecar for trivial writes (file ≤ 1 MiB AND not Edit/Replace AND parent dir under git AND file size ≤ 4 KiB). `Always` forces the sidecar (legacy semantics, equivalent to `--strict-atomic`). `Never` suppresses sidecar creation even when `--strict-atomic` is set. Decision happens inside `atomic_write` BEFORE `journal_started_with_guard`; cost is O(0) when the policy votes "no sidecar". Expected reduction: 60-80% of sidecars for typical agent LLM workloads.
- L4 — `HeuristicsEngine` with 5 composable rules — `h1_ttl` (preserve for N seconds after `Committed`, default 0), `h2_lru_within_cap` (preserve within count cap, default 100), `h3_rate_limit` (throttle when >K sidecars/min, default 10), `h4_sentinel` (`.atomwrite_no_wal` file disables per-directory), `h5_archive` (flag for archival when older than 7 days). Env vars: `ATOMWRITE_WAL_KEEP_SECS`, `ATOMWRITE_WAL_MAX_COUNT`, `ATOMWRITE_WAL_RATE_LIMIT`, `ATOMWRITE_WAL_ARCHIVE_DAYS`. H3 uses lock-free `AtomicU64` for the 60s window. `heuristics_should_preserve(target, committed_at_unix, count, rank)` composes via OR.
- `AtomicWriteOptions.wal_policy: WalPolicy` — new field defaulting to `Auto`. All 16 call-sites in `src/commands/` (write, edit, set, get, del, case, copy, replace, transform, scope, apply, rollback, batch×4) updated to pass the policy through.
- Telemetry — `WriteOutput` NDJSON envelope gains `wal_policy: "auto" | "always" | "never"` so callers can audit which policy was applied.
- AdR — `docs/decisions/0028-g119-wal-cleanup-intelligent.md` documents the 5-layer architecture and the OR-composition semantics of the L4 engine.
- New unit tests — 12 tests in `src/wal.rs::tests`: `l1_never_policy_always_returns_false`, `l1_always_policy_always_returns_true`, `l1_auto_policy_returns_true_for_large_file`, `l1_auto_policy_returns_true_for_edit_op`, `l1_auto_policy_skips_trivial_file`, `l4_h1_ttl_default_zero_returns_false`, `l4_h2_lru_within_cap_returns_true_when_count_low`, `l4_h2_lru_returns_true_when_count_at_or_below_default_cap`, `l4_h3_rate_limit_returns_false_below_threshold`, `l4_h4_sentinel_returns_true_when_file_exists`, `l4_h4_sentinel_returns_false_when_absent`, `l4_h5_archive_returns_false_for_recent_journal_under_default`, `l4_h5_archive_returns_true_for_journal_older_than_7_days`, `l4_engine_returns_false_when_all_heuristics_disabled`.

### G120 — closes the 4-layer content validation (L3 cross-validation)
- L3 — `--append`/`--prepend` + `--expect-checksum` + empty stdin emits structured warning — when the caller combines an append/prepend flag with `--expect-checksum` AND the stdin is empty (the L1-bypass case via `--allow-empty-stdin`), `cmd_write` logs a `tracing::info!` warning that names the cross-flag combination and proceeds to `verify_checksum` (which still validates the pre-mutation state). Operators monitoring stderr get an explicit signal without changing the exit code.
- L3 opt-out — `--no-checksum-when-empty` — caller that INTENDS the empty-stdin + checksum combination (no-op append with locking guarantee) can pass this flag to skip `verify_checksum` entirely. Emits `tracing::warn!` recording the decision.
- L1+L2 unchanged — `read_stdin_content` still rejects empty stdin by default (exit 65); `handle_append_prepend` still rejects empty stdin when `--append`/`--prepend` is set. L3 is the third layer that runs only when L1+L2 are explicitly bypassed.
- AdR — `docs/decisions/0029-g120-empty-stdin-guard.md` documents the L3 warning semantics and the opt-out flag.

### Validation
- `cargo fmt --check` clean
- `cargo clippy --bin atomwrite --lib` clean (0 warnings)
- Full suite: 487 tests passing (469 baseline v0.1.15 + 18 new in `src/wal.rs::tests`), 0 failures, 0 regressions
- New `WalPolicy` exported from `crate::wal` and registered in clap's `ValueEnum` derive
- `WriteOutput` schema regenerated to include `wal_policy` field; `tests/snapshots/snapshot_write__write_output_structure.snap` updated


## [0.1.15] - 2026-06-11

### G119 — WAL sidecar cleanup in 3 layers (L2 Drop guard + L3 `wal-heal` + L5 `wal-stats`)
- L2 — Drop guard — the sidecar `.atomwrite.journal.<basename>.json` is now wrapped in a `JournalGuard` (RAII, modeled on `tempfile::TempPath`). The `Committed` entry's append calls `wal_guard.release()`; if the rename succeeds the sidecar is removed on scope exit. On panic or early return the guard is `keep()`'d and the sidecar survives for `recover_orphan_journals`. 60+ pre-existing sidecars observed in this repository's working tree during the G119 audit are now reaped by the next invocation.
- L3 — `wal-heal` subcommand — the explicit operator-facing version of the auto-heal pass. Walks the workspace with a `--max-duration-ms` budget (default 100ms), reaps every `Committed`/`Aborted` sidecar older than `--threshold-secs` (default 3600s), and emits `{"removed":N,"preserved":N,"malformed":N,"bytes_reclaimed":N,"threshold_secs":N}`. `Started` orphans are NEVER removed automatically — they are the only signal worth operator attention.
- L5 — `wal-stats` subcommand — read-only snapshot: `total_journals`, `by_state{started,committed,aborted,malformed}`, `oldest_journal_age_secs`, `total_size_bytes`, `by_directory` (top 10), and `auto_heal_recommended` (true when total > 100 OR oldest > 7d). Used by CI gates and agent health checks to detect accumulating junk before it pollutes `git status --porcelain`.
- Sidecar pattern shared — `JournalGuard`, `WalStats`, and `AutoHealReport` re-export through `ndjson_types` for `--json-schema` introspection. Schemas regen in `docs/schemas/`.

### G120 — empty-stdin guard in 3 layers (L1 stdin guard + L2 append guard + L4 telemetry)
- L1 — `read_stdin_content` rejects `&[]` by default — the upstream-pipe failure mode (`cat missing.txt`, heredoc that expands to nothing, a failing `find`) silently produced a "success" exit with 0 bytes written. Default is now exit 65 `INVALID_INPUT` with an actionable message naming the opt-out flag. Callers that genuinely intend to write zero bytes (`echo -n`, intentional truncate) must pass `--allow-empty-stdin`.
- L2 — `handle_append_prepend` rejects empty stdin with `--append`/`--prepend` — second line of defence for the L1-bypass case. Error message names the operation so the caller can tell which flag the empty stream hit.
- L4 — `stdin_bytes_read: u64` field in the `write` NDJSON envelope — telemetry that lets CI/agent gates validate `if .stdin_bytes_read == 0 then fail` even when the operation succeeded. Always present (not behind `Option`).
- Schema regenerated — `docs/schemas/write-output.schema.json` now includes `stdin_bytes_read`. Snapshot test updated.
- Anti-masking guidance — combined with the G117 `jaq -e '.edits'` recipe, callers should validate `.stdin_bytes_read > 0` for non-truncation writes.
- Cross-flag note — `--append` + `--expect-checksum` is legitimate (locking otimista de append); the G120 guard does NOT interfere with this combination. The pre-existing G118 path-resolution fix means the checksum is now compared against the real file even when CWD diverges from workspace.

### Test-legacy cleanup (G119 L2 side effect)
- `recover_orphan_journals_with_committed_entry` and `wal_journal_created_on_set` flipped from "sidecar must exist" to "sidecar must NOT exist" — the new behavior is the G119 L2 contract.
- `syntax_check_large_streaming_file` updated to pipe its 1 MiB payload via `write_stdin` instead of relying on the now-rejected empty-stdin default.
- All other legacy tests preserved verbatim.

### Validation — G119, G120 and the test-legacy cleanup
- 8 new integration tests in `tests/cli_write.rs` (G120 L1+L2+L4) and `tests/cli_wal.rs` (G119 L2+L3+L5): empty-stdin reject, empty-stdin opt-in truncate, append empty-stdin reject, stdin bytes telemetry, drop-guard on success, drop-guard on failure, `wal-stats` empty/non-empty, `wal-heal` reap-stale-preserve-started, `wal-stats` state counts
- 1 new unit test for `parse_journal_state` classification
- Full suite: 469 tests passing (461 in baseline v0.1.15 + 8 new), 0 failures
- `cargo fmt --check` clean, `cargo clippy --bin atomwrite --lib --all-features` clean (0 warnings)
- End-to-end smoke test: `wal-heal --threshold-secs 0` on a 60-sidecar workspace reports `{"removed":60,"preserved":0,"malformed":0,"bytes_reclaimed":47624}` and leaves 0 sidecars

### G117 — multi-pair `edit --old/--new`: fuzzy parity, per-pair reporting, and opt-in `--partial`
- Fuzzy parity — the multi-pair path previously used exact `find_str` matching only, so a whitespace-divergent pair that the single-pair fuzzy cascade rescues killed the whole batch. The cascade is now extracted into `match_pair` and shared by both paths: every pair runs the full 9-strategy cascade (`exact`, `line_trimmed`, `whitespace_normalized`, `punctuation_normalized`, `indent_flexible`, `escape_normalized`, `trimmed_boundary`, `block_anchor`, `context_aware`). `--fuzzy off` preserves the exact-only pre-G117 behavior.
- Per-pair reporting — success envelopes gain `pairs_total` and `pair_results` (array of `{index (1-based), matched, strategy, similarity}`); `mode` becomes `fuzzy-multi(N)` when any pair matched fuzzily and stays `exact-multi(N)` otherwise. Error envelopes gain `failed_pair_index`, `pairs_total`, and `pair_results` via the new `AtomwriteError::EditPairFailed` variant, which reuses the `INVALID_INPUT` code and exit 65 (no new error code). Pairs after the failed index were never attempted and are absent from the array.
- New `--partial` flag (opt-in) — applies the matching pairs in one atomic write (exit 0, `edits < pairs_total`) and reports the unmatched ones with `matched: false`. Zero applied pairs exits 1 (`NO_MATCHES`) without writing, matching `replace` semantics. The default remains all-or-nothing: when atomicity is possible it is strictly better than partial-failure reporting.
- Anti-masking guidance — the NDJSON error envelope goes to stdout by contract, so `edit ... | jaq '.edits'` masked exit 65 as `{"edits": null}` with pipeline exit 0. README, SKILL, COOKBOOK, and HOW_TO_USE (both languages) now document the `jaq -e '.edits'` / `${PIPESTATUS[0]}` recipe.
- Schemas regenerated — `docs/schemas/edit-output.schema.json` is regenerated from `edit --json-schema` (now includes `mtime_preserved`, `pairs_total`, `pair_results`); `docs/schemas/error-output.schema.json` gains the three G117 fields plus the five error codes introduced in v0.1.12 that were missing from its enum (`LOCK_TIMEOUT`, `SYNTAX_ERROR_DETECTED`, `EXDEV_FALLBACK_DISABLED`, `COPY_BACK_BLAKE3_FAILED`, `ORPHAN_JOURNAL`).
- Scope note — the `--multi` NDJSON-stdin mode is unchanged; G117 covers only repeated `--old`/`--new` pairs. See `docs/decisions/0026-g117-edit-multi-pair-fuzzy-partial.md`.

### G118 — `write` resolves the target against the workspace before all pre-steps
- Bug (double path identity, CWE-367) — `cmd_write` handed the raw CLI path (relative to the CWD) to `handle_append_prepend`, `normalize_line_endings` (auto), and `verify_checksum`, while only `atomic_write` resolved it via `validate_path`. With a relative target and a CWD different from `--workspace`, append/prepend silently TRUNCATED the file, line-ending auto-detection was skipped, and `--expect-checksum` was skipped entirely (any hash accepted, exit 0). Detected in production: this repository's `gaps.md` was truncated and recovered via `rollback --latest --verify`.
- Fix — the target is resolved once at the top of `cmd_write` and the resolved path feeds all three pre-steps and `atomic_write`. Checksum drift with a divergent CWD now fails with `STATE_DRIFT` (exit 82); out-of-jail targets fail early with `WORKSPACE_JAIL` (exit 126). The NDJSON `path` field still echoes the user-supplied path. See `docs/decisions/0027-g118-write-path-resolution.md`.
- Why tests never caught it — the suite only used ABSOLUTE targets, which are immune to the CWD. Five regression tests now use a RELATIVE target with `current_dir` outside the workspace (append, prepend, drift exit 82, matching checksum, CRLF auto-detection), plus a conformance guard asserting `&args.target` appears exactly once in `write.rs`.

### GAP 18 — Windows CI green again
- `tests/snapshot_write.rs` now redacts `platform.dir_fsync` as `[platform_dir_fsync]`, the same technique already used for `platform.fsync`. The snapshot previously pinned `"dir_fsync": "sync_all"`, which Windows reports as `best_effort`, keeping the `windows-2025-vs2026` job red since v0.1.12.

### MSRV job aligned with the manifest
- The CI job named `MSRV 1.85` (toolchain pinned at 1.85) now tests the documented MSRV: `MSRV 1.88` with `dtolnay/rust-toolchain@1.88`, matching `Cargo.toml` `rust-version = "1.88"`.

### Validation — G117, G118, the Windows CI fix and the MSRV job
- 8 new integration tests in `tests/cli_edit.rs` (21 total in the suite): fuzzy pair in multi, `exact-multi(N)` mode compat, `pair_results` on success, `failed_pair_index`/`pairs_total`/`pair_results` on error with file intact, `--fuzzy off` compat, `--partial` happy path, `--partial` zero matches exits 1, `--partial --dry-run` writes nothing
- `cargo test --lib` 152 passed (variant-suggestion coverage extended to `EditPairFailed`); `cargo test --test snapshot_write` 9 passed
- Deterministic reproduction from `gaps.md` re-run against the new binary: mixed batch reports `failed_pair_index: 2` with the file intact; `--partial` applies pair 1 with `edits: 1`; `| jaq -e '.edits'` exits 1 on the error envelope
- G118: 6 new integration tests in `tests/cli_write.rs` (14 total); deterministic reproduction re-run with divergent CWD: append preserves all lines, the all-zeros checksum now exits 82 with the file intact
- Full suite after G118: 461 tests passing (445 in v0.1.12 + 2 in v0.1.14 + 8 G117 + 6 G118), 0 failures; `fmt`/`clippy -D warnings`/`doc`/`deny`/`audit` green; Windows cross-check `x86_64-pc-windows-gnu` with `RUSTFLAGS=-Dwarnings` clean

## [0.1.14] - 2026-06-07

### Cross-platform parity for `write --line-ending auto` on new files
- `write_creates_file_with_ndjson_output` no longer fails on `windows-2025-vs2026` — the test writes 12 bytes of input (`"hello world\n"`) and expects `bytes_written == 12`. v0.1.13 returned 13 bytes on Windows because the legacy fallback in `normalize_line_endings` returned `LineEnding::CrLf` when the target file did not exist and the host OS was Windows, and the subsequent `line_endings::normalize(..., CrLf)` inserted a `\r` before every `\n`. Linux and macOS were unaffected (their fallback returned `LineEnding::Lf`, which preserved the input byte count).
- `Auto` on a new file is now a true no-op — the fallback branches that returned `LineEnding::CrLf` on Windows (when the target did not exist or could not be read) have been removed. `Auto` now matches its docstring (`Preserve the dominant ending of the original file`): when there is no original, the input bytes pass through verbatim. This makes the CLI deterministic across Linux, macOS, and Windows for the same stdin content.
- Round-trip parity for CRLF input — a follow-on bug was discovered while writing regression tests: the previous fallback did not just convert LF to CRLF, it also *removed* `\r` from CRLF input by resolving `Auto` → `Lf` and then running the `normalize` canonicalization step. With the new behavior, CRLF input on a new file now stays CRLF byte-for-byte. This preserves the `--expect-checksum` round-trip when the user supplies CRLF content.
- Two new regression tests in `src/commands/write.rs::tests` — `auto_on_new_file_preserves_lf_input` and `auto_on_new_file_preserves_crlf_input`. They exercise the `Auto` branch with both LF and CRLF input and assert that the output equals the input byte-for-byte. The tests are platform-agnostic and would have caught both the `cfg!(windows) ? CrLf : Lf` bug and the canonicalization bug on any CI runner.
- No changes to the existing detection logic — when the target file exists, `Auto` still calls `line_endings::detect(&existing)` and applies the dominant style. When the user passes an explicit `--line-ending lf|cr-lf|cr`, the explicit value is still applied as before. Only the `Auto` + non-existent target branch changed.

### Validation
- Linux CI: `cargo build --all-features`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features` (152 lib tests + integration suites + 3 doctests) — all green
- Windows CI cross-check: `cargo check --target x86_64-pc-windows-gnu --lib` with `RUSTFLAGS=-Dwarnings` and a `cc` stub for the missing mingw-gcc linker — zero errors, zero warnings
- 8/8 `cli_write` integration tests pass, including the formerly Windows-only failure


## [0.1.13] - 2026-06-07

### CI cross-platform (windows-2025-vs2026 build fix)
- `unused import: Duration` fix in `src/lock.rs` — `std::time::Duration` is now gated by `#[cfg(unix)]` since it is only consumed by the `flock`-based retry loop in `try_acquire_loop`. On Windows the import is omitted, eliminating the `unused_imports` warning that escalated to a build error under `RUSTFLAGS=-Dwarnings`.
- `unused variable: strict_atomic` fix in `src/atomic.rs` — the `strict_atomic` parameter of `write_rename_path` is consumed exclusively inside the `#[cfg(unix)]` EXDEV fallback branch. The parameter is now annotated with `#[cfg_attr(not(unix), allow(unused_variables))]`, mirroring the pattern established in `src/signal.rs:15-17` (GAP 06 fix).
- `dead_code: copy_tempfile_to_target` fix in `src/atomic.rs` — the EXDEV copy-fallback helper is only invoked from the `#[cfg(unix)]` branch of `write_rename_path`. The function itself is now gated by `#[cfg(unix)]`, removing it entirely from the Windows compilation unit.
- `clippy::unnecessary_literal_unwrap` fix in `src/atomic.rs:195` — the `hardlink_nlink.unwrap_or(1) > 1` heuristic was rewritten as `hardlink_nlink.is_some_and(|n| n > 1)`. The new form has identical semantics on both platforms (returns `false` when `None`, returns the boolean comparison when `Some(n)`) and avoids triggering clippy 1.94+ on the `None.unwrap_or(1)` path that Windows produces when hardlink detection is unavailable.

### Validation
- Linux CI: `cargo build --all-features`, `cargo clippy --all-features -- -D warnings`, `cargo test --all-features` (150 lib tests passing) — all green
- Windows CI: the four `RUSTFLAGS=-Dwarnings` errors are eliminated by gating the unix-only symbols with `cfg(unix)` / `cfg_attr(not(unix), allow(...))`


## [0.1.12] - 2026-06-07

### Atomic write pipeline (G55, G39)
- Hardlink preservation (G55, CRITICAL) — `atomwrite edit/write/replace/copy` now auto-detects when the target has `nlink > 1` (Unix) or is a symlink, and switches to in-place `ftruncate(0) + write_all + fsync_data` instead of `rename(2)`. Both the file and any hardlinks point to the same inode after the write, so BorgBackup / restic / Nix store / git-annex stay consistent. NDJSON gains `write_strategy: "rename" | "inplace" | "copyback"` and `hardlink_nlink` fields.
- xattr preservation (G39) — macOS `com.apple.quarantine` (Gatekeeper), Linux `security.selinux` (SELinux), `security.capability` (POSIX caps), and arbitrary `user.*` attributes are now saved before the write and restored after. Best-effort (FAT32 / tmpfs / overlayfs return `EOPNOTSUPP` and we log a warning, not an error). New module `src/xattr_restore.rs`. NDJSON gains `xattr_preserved` and `xattr_count`.

### Detection (G41, G56, G90)
- UTF-16LE-aware binary detection (G41) — `is_binary` is now backed by the `content_inspector` crate (0.2.x, 11M+ downloads). UTF-16LE / UTF-16BE files without BOM (Windows `.reg` exports, Excel CSV) are no longer misclassified as binary. New `ContentType` enum with `Utf8 | Utf16Le | Utf16Be | Binary` variants.
- FIFO skip in `search` (G56) — `atomwrite search` no longer hangs on `/tmp/*.fifo` (CI, Docker). New `--include-fifo` opt-in restores the legacy behavior. Walker's `filter_entry` rejects FIFO files by default.
- EXDEV copy-fallback (G90) — `rename(2)` across filesystems (Docker overlay2 + named volumes, NFS) now falls back to `read tempfile → copy stream → fsync target → delete tempfile` automatically. New `--strict-atomic` flag aborts with exit 91 instead of fallback.

### Locking (G54)
- Advisory file locking (G54) — new global `--lock` flag (with `--lock-timeout <ms>`, default 5000) takes an exclusive `flock(2)` on a `.<target>.atomwrite.lock` sidecar before the atomic write. Two atomwrite processes editing the same file now serialize instead of silently losing the first write. Implemented via `nix::fcntl::flock` with `LockExclusiveNonblock` polling. New exit code 83 (`LOCK_TIMEOUT`).

### Performance and limits (G64, G68, G77)
- Reflink on backup/copy (G64) — `atomwrite backup` and `atomwrite copy` now use `reflink_copy::reflink_or_copy` for O(1) copy-on-write on APFS / btrfs / XFS. Falls back to `fs::copy` on unsupported filesystems. New `--no-reflink` opt-out.
- `search --max-filesize` and `--max-columns` (G68) — `search` now skips files larger than 10 MiB by default (overridable via `--max-filesize N`), and truncates matches longer than 500 columns (overridable via `--max-columns N`). Eliminates `node_modules/.cache`, `target/`, minified `bundle.js` from blowing up LLM context windows.
- `batch --batch-size` (G77) — explicit hint for NDJSON streaming. Default 100. (atomwrite was already streaming line-by-line; this flag documents the behavior.)

### Refactoring (G116, G44)
- Fuzzy match strategy 9: context-aware (G116) — `edit --old/--new` now uses `strsim::normalized_levenshtein` (threshold 0.80) to find a sliding window of the target pattern in the content. Catches edits where leading AND trailing context are also wrong (e.g. when an LLM adds comments near the match). Only active in `--fuzzy auto` or `aggressive`.
- Multi-rule YAML for `transform` (G44) — new `--rules PATH` and `--inline-rules "YAML"` flags let you apply multiple AST refactoring rules in a single atomwrite call. YAML format: `[{language, pattern, rewrite, id?}]`. Emits `rule_begin` and `rule_error` events so consumers can correlate matches with the rule that produced them.

### v14 Tier 3: structured config editors
- `set <path> <key-path> <value>` — sets a value at a dotted path in a TOML or JSON file, preserving comments and key order in TOML (via `toml_edit`) and rewriting JSON canonically. NDJSON: `{type: "set", key_path, old_value, new_value, format, comments_preserved}`.
- `get <path> <key-path>` — reads a value at a dotted path. NDJSON: `{type: "get", key_path, value, found, format}`.
- `del <path> <key-path>` — removes a key. Preserves formatting. `--force-missing` flag treats missing keys as a no-op success.
- `case <paths...> --subvert OLD NEW --to <style>` — renames identifiers across multiple files. Supports `snake_case`, `camelCase`, `PascalCase`, `kebab-case`, and `SCREAMING_SNAKE_CASE` via the `heck` crate. NDJSON: `{type: "case", before, after, from_style, to_style}`.

### v14 Tier 3 (continued): tree-sitter AST via `tree-sitter-language-pack`
- `query <path> [--kinds|--query <KIND>|-Q <PATTERN>|--tree] [--positions]` — walks a tree-sitter parse of the file and emits AST nodes as NDJSON (`query_match` lines + final `query_summary`). `--kinds` aggregates all node kinds with counts; `--query <KIND>` emits nodes matching one kind name (overloaded short flag `-Q` to avoid clash with the global `--quiet/-q`); `--tree` dumps every named node in pre-order DFS. 305 languages supported via download-on-demand (parsers cache locally). Iterative DFS via `Vec<Node>` stack — no stack overflow on large files. Schema: `docs/schemas/query-output.schema.json`.
- `outline <path> [--kind <KIND>] [--positions]` — extracts the high-level structure (functions, classes, structs, enums, traits, modules, top-level consts) as one `outline_item` NDJSON line per item + final `outline_summary`. Iterative DFS via `Vec<Node>`. Schema: `docs/schemas/outline-output.schema.json`.
- G72 REAL syntax check via tree-sitter — `--syntax-check` flag on `atomwrite write` (and `AtomicWriteOptions::syntax_check` for library users) now invokes the actual tree-sitter parser via `tree-sitter-language-pack` instead of the previous bracket-balance heuristic. Walks the tree counting `is_error` and `is_missing` nodes; reports the first one (line, column, kind, message) as a `SYNTAX_ERROR_DETECTED` NDJSON error (exit 88). Languages covered: rust, python, javascript, typescript, tsx, go, c, cpp, java, ruby, php, bash, html, css, json, yaml, toml, markdown, lua, scala, swift, kotlin, sql. Files with no parser available fall back to the legacy heuristic. New module `src/syntax_check.rs` (16 unit tests).
- G114 WAL sidecar for crash recovery — `atomic_write` now appends a `Started` entry to `.atomwrite.journal.<target>.atomwrite.journal.json` before the rename, and a `Committed` entry after success. On crash (SIGKILL, OOM, power loss), the orphan journal surfaces a structured `wal_recovery` report with `target`, `expected_new_checksum`, `op_id` (16-hex-char correlation ID), `started_at_unix`, and `pid`. New module `src/wal.rs` (8 unit tests). Recovery is consultative — `recover_orphan_journals(dir)` reads sidecars and reports orphans without touching the filesystem. Schema: `docs/schemas/wal-recovery.schema.json`.

### Error model
- 5 new error variants: `LockTimeout` (83), `SyntaxError` (88), `ExdevFallbackDisabled` (91), `CopyBackBlake3Failed` (92), `OrphanJournal` (93). All have bilingual (EN/PT-BR) messages with actionable suggestions via `ErrorContext`.

### i18n
- 5 new `error.*` keys and 5 new `suggestion.*` keys in `locales/en.toml` and `locales/pt-BR.toml`. `AtomwriteError::suggestion` for the new variants is fully translated.

### Dependencies
- Added 8 direct dependencies: `xattr = "1"`, `content_inspector = "0.2"`, `strsim = "0.11"`, `heck = "0.5"`, `serde_yaml = "0.9"`, `indexmap = { version = "2", features = ["serde"] }`, `toml_edit = "0.22"`, `reflink-copy = "0.1"`. Linux-only `rustix = "0.38"`. v0.1.12 final: `tree-sitter-language-pack = "1.8"` with `download` + `dynamic-loading` features (parsers download on first use; install footprint stays small because the 305 grammars are NOT bundled — only the loader library is).
- Removed from declared deps: `fd-lock = "4"` (was incompatible with the project's `#![deny(unsafe_code)]` due to the `RwLockWriteGuard` self-referential pattern; advisory locking re-implemented via `nix::fcntl::flock` which is safe to call).

### Fixed (CI Failures - GAP 23 Windows path backslash in JSON manifests)
- 11 `cli_batch` tests no longer fail on `windows-2025-vs2026` — Tests were building the NDJSON manifest via `format!` + `Path::display()`. On Windows the platform-native path uses backslashes (`C:\Users\...\Temp\.tmpXXXX\file.txt`), and `format!` does not escape them. The result is a JSON string with invalid escape sequences (`\U`, `\r`, `\A`, `\L`, `\T`), which `serde_path_to_error::deserialize` rejects with `invalid escape`. `cmd_batch` then returns `bail!` and exits non-zero, failing the `assert!(output.status.success())` check. The test was passing on Linux/macOS only because their paths use forward slashes, which are valid in JSON strings without escaping.
  - Added `common::manifest(&[serde_json::Value]) -> String` helper in `tests/common/mod.rs` that serializes each op via `serde_json::to_string`, guaranteeing correct JSON escaping of backslashes, quotes, control characters, and Unicode.
  - Refactored all 11 affected tests in `tests/cli_batch.rs` to use the new helper via the `serde_json::json!` macro.
  - Refactored 2 additional tests with the same bug pattern: `tests/snapshot_write.rs::batch_summary_ndjson_structure_snapshot` and `tests/ndjson_valid_test.rs::ndjson_batch_output_valid` (also added the missing `mod common;` to the latter).
  - Added regression test `batch_write_escapes_backslash_in_target_path` that constructs a path string with a forced backslash on any platform, so the bug would be caught on every CI run, not just Windows.
- Total tests: 303/303 PASS (was 302; +1 from the new regression test).

## [0.1.11] - 2026-06-05

### Fixed (CI Failures - windows-2025-vs2026 + Linux flaky signal test)
- Windows `windows-2025-vs2026` E0433 resolved — `libc::write(STDERR_FILENO, ...)` and `libc::STDERR_FILENO` were referenced from `src/main.rs:22-23` in a function that was compiled on every platform, but `libc` is declared only under `[target.'cfg(unix)'.dependencies]`. The build failed on Windows with `error[E0433]: failed to resolve: use of unresolved module or unlinked crate 'libc'`. The shutdown-message writer was moved to `src/signal.rs` and gated with `#[cfg(unix)]` (with a `#[cfg(not(unix))]` no-op body), so Windows uses the existing ctrlc path that emits the banner inline. The new function `atomwrite::signal::write_shutdown_message()` also loops on `EINTR` and `EAGAIN` to be robust against interrupted `write(2)` syscalls and tight pipe-buffer limits imposed by some CI sandboxes.
- `signal_test::shutdown_message_on_stderr` no longer flakes on ubuntu-latest — The previous test slept 2 s before sending SIGINT and asserted that the captured stderr contained "shutting down". Two independent failure modes were observed:
  1. The search command returned `Err(NoMatches)` when the `shutdown.is_shutdown()` flag tripped mid-scan, because parallel walker threads had buffered Begin events that were never paired with End events, leaving `has_matches = false`. `main.rs` then took the `Err` branch and never reached the shutdown-banner write. `cmd_search` now short-circuits to `Ok(())` whenever `shutdown.is_shutdown()` is true, so the main thread takes the `Ok(())` branch and emits the banner as designed.
  2. `install_handlers_early` and `install_handlers` each created their own `Arc<ShutdownSignal>` (`signal A` for the search polling inside `atomwrite::run`, `signal B` for the main-thread `is_shutdown()` check). Under the `signal-hook` chain-of-handlers ordering, only the first instance was flipped when SIGINT arrived — the second instance's flag remained `false`, so the main thread took the `is_shutdown() == false` branch and exited 0 without writing the banner. Both functions now share a single `ShutdownSignal` instance: `install_handlers_early` installs the full handler chain (flag + counter) and `install_handlers` is idempotent (returns the existing `Arc` when `GLOBAL_SHUTDOWN` is already populated).
- Test uses `ATOMWRITE_READY_FILE` for race-free readiness detection — `signal_test::shutdown_message_on_stderr` now sets `ATOMWRITE_READY_FILE` to a path under the test tempdir and atomwrite writes its PID to that path as soon as `install_handlers_early` returns. The test polls the file with a 10 s deadline before sending SIGINT, eliminating the microsecond window where SIGINT could race `posix_spawn` and arrive before the kernel `sigaction` was configured. This change is internal to the test harness and has no effect on the published CLI surface.

### Validation
- `cargo fmt -- --check`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `cargo build --release`: PASS (1 m 14 s)
- `cargo test --all-features`: 302/302 tests PASS across 33 test suites (5 successive full-suite runs)
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`: PASS
- `cargo audit`: PASS (no vulnerabilities)
- `cargo deny check`: PASS (advisories, bans, sources OK; one cosmetic `license-not-encountered` warning for the unused ISC allowance in `deny.toml`)

### Notes
- v0.1.11 is a NON-BREAKING change. No public API was modified.
- The shutdown-message write moved from `src/main.rs` to `src/signal.rs` as a documented `pub fn`. The function is `#[cfg(unix)]` (libc dependency) and no-op on non-Unix. Internal API move only.
- v0.1.10 has been yanked from crates.io. New `cargo install` will resolve to v0.1.11.

## [0.1.10] - 2026-06-05

### Fixed (CI Failures - GAP 20 follow-up)
- `signal_test::shutdown_message_on_stderr` flushes the shutdown message via `io::stderr().lock()` — The first v0.1.8 fix moved `eprintln!` from the signal handler to the main thread, but used `writeln!(io::stderr(), ...)` which is fully-buffered when stderr is redirected to a pipe (as in `cargo test`'s `Stdio::piped()`). The buffer was never flushed before the process exited with the signal exit code, so the parent test saw an empty stderr. The fix uses `io::stderr().lock()` to acquire the `StderrLock` guard, which flushes the buffer on Drop. This guarantees the message reaches the captured stderr pipe before the process exits. CI ubuntu-latest will confirm on push.

## [0.1.9] - 2026-06-05

### Notes
- No change record is available for this version: the repository holds no entry, ADR or gap for it, and it has no version history to rebuild it from
- The registry lists 0.1.9 as published on 2026-06-05 at 19:45 UTC, not yanked, with `rust-version` 1.88 (source: `https://crates.io/api/v1/crates/atomwrite/0.1.9`)

## [0.1.8] - 2026-06-05

### Fixed (CI Failures - GAP 17 and GAP 18)
- `signal_test::shutdown_message_on_stderr` no longer fails on Linux CI — Removed `eprintln!("\natomwrite: shutting down...")` from the SIGINT and SIGTERM signal handlers. Per POSIX.1-2017 `signal-safety(7)`, stdio functions like `eprintln!` are NOT async-signal-safe; Rust's stderr uses a global `Mutex` that can deadlock or lose buffered output if the signal arrives while another thread holds the lock. The user-facing shutdown message is now emitted by the main thread in `src/main.rs` when it observes `is_shutdown() == true` after `atomwrite::run` returns, which is the only async-signal-safe way to guarantee the message reaches the captured stderr pipe before the process exits. The Windows `ctrlc` path still emits the message inline (ctrlc handlers run in a normal thread, not signal context).
- `atomic::tests::create_backup_and_retention` no longer fails on Windows CI — Added `platform::fsync_file_best_effort` which logs a warning and continues instead of returning an error. On Windows, antivirus products (Windows Defender, third-party AV) can transiently hold a read handle on files in `%TEMP%` with `FILE_SHARE_READ` but without `FILE_SHARE_WRITE`, causing `FlushFileBuffers` to return `ERROR_ACCESS_DENIED` (os error 5). The primary write path still uses the strict `fsync_file`; only the backup-durability fsync is best-effort because the backup itself has already been created via `fs::copy`.
- CI matrix pinned to `windows-2025-vs2026` — The matrix entry for Windows was changed from `windows-latest` to `windows-2025-vs2026` (its successor before the June 15 2026 GitHub-hosted runner migration). This silences the "windows-latest requests are being redirected to windows-2025-vs2026 by June 15, 2026" NOTICE and prevents unexpected runner changes from breaking the build.

### Validation
- `cargo build --all-features`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `cargo test --all-features`: 302/302 tests PASS (all 5 `signal_test` cases pass; `atomic::create_backup_and_retention` passes)
- `cargo fmt -- --check`: PASS
- `cargo audit`: PASS (no vulnerabilities, no `--ignore` flag)
- `cargo deny check`: PASS (advisories, bans, licenses, sources all OK)

### Notes
- v0.1.8 is a NON-BREAKING change. No public API was modified.
- The signal-handler change is internal: external consumers that relied on the shutdown message appearing on stderr continue to see it; it is now emitted by the main thread instead of the signal handler.
- The Windows backup fsync change is internal: backup files are still created and atomic; the only difference is that the durability flush for backup metadata is best-effort. If a future user reports data loss on backup, we can re-tighten the fsync.

## [0.1.7] - 2026-06-05

### Fixed (CI Failures - GAP 15)
- `cargo audit` no longer reports RUSTSEC-2026-0009 — Upgraded `time` transitive dependency from 0.3.45 to 0.3.47 (DoS via stack exhaustion in RFC 2822 parser, fixed upstream via DEPTH_LIMIT=32). The upgrade required bumping MSRV from 1.85 to 1.88. The `deny.toml` ignore for RUSTSEC-2026-0009 was removed because the advisory no longer applies.
- macos-latest CI failure — `src/platform.rs:31` no longer uses `return Ok(())` (removed redundant return). clippy 1.94+ `needless_return` lint no longer triggers; the `RUSTFLAGS: -Dwarnings` env in CI no longer aborts the build.
- windows-latest CI failure — `src/signal.rs:15-16` constants `EXIT_SIGINT` and `EXIT_SIGTERM` now have `#[cfg_attr(not(unix), allow(dead_code))]`. The `RUSTFLAGS: -Dwarnings` env no longer aborts on `dead_code` in Windows builds.
- `actions/checkout` and `actions/cache` Node 20 deprecation — Both actions bumped to their major version that supports Node 24 (`actions/checkout@v6`, `actions/cache@v5`). `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: "true"` added to workflow env as belt-and-suspenders. The deprecation warning no longer appears in CI logs.
- MSRV bumped to 1.88 — `rust-version` in `Cargo.toml` is now 1.88. All documentation files (EN and PT-BR) updated: `docs/INSTALL.md`, `docs/INSTALL.pt-BR.md`, `docs/HOW_TO_USE.md`, `docs/HOW_TO_USE.pt-BR.md`, `docs/CROSS_PLATFORM.md`, `docs/CROSS_PLATFORM.pt-BR.md`, `docs/COOKBOOK.md`, `docs/COOKBOOK.pt-BR.md`, `CONTRIBUTING.md`, `CONTRIBUTING.pt-BR.md`.

### Changed
- `build.rs:4-12` — Collapsed nested `if let + if` into `if let + &&` to satisfy clippy 1.94+ `collapsible_if` lint.
- `src/lib.rs` — Added `#![allow(clippy::collapsible_if)]` and `#![allow(clippy::needless_return)]` as deliberate project decisions to keep `if let` blocks on separate lines for readability. This avoids 25 separate refactor sites in subcommand handlers.
- Snapshot test platform-aware — `tests/snapshot_write.rs` and `tests/snapshots/snapshot_write__write_output_structure.snap` now use `[platform_fsync]` placeholder for the `platform.fsync` field, allowing the same snapshot to be valid on Linux (`sync_data`), macOS (`F_FULLFSYNC`), and Windows.

### Validation
- `cargo build --all-features`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `cargo test --all-features`: 302 of 303 tests PASS (1 pre-existing failure in `signal_test::shutdown_message_on_stderr` tracked as GAP 16, unrelated to GAP 15)
- `cargo fmt -- --check`: PASS
- `cargo audit`: PASS (no vulnerabilities, no `--ignore` flag)
- `cargo deny check`: PASS (advisories, bans, licenses, sources all OK)
- Cross-compile `x86_64-pc-windows-gnu`: PASS (build, clippy -D warnings, tests --no-run)
- Cross-compile `i686-pc-windows-gnu`: PASS (check --all-features)

### Notes
- This is a NON-BREAKING change for users on Rust 1.88 or later. Users on Rust 1.85-1.87 must upgrade.
- The `time` transitive dependency is now patched (0.3.47+), resolving RUSTSEC-2026-0009.
- Cross-compile Windows GNU and i686 targets are now explicitly validated by the local developer workflow; MSVC target requires Windows runner (CI windows-latest job covers it).

### Fixed (GAP 16 - signal_test)
- `signal_test::shutdown_message_on_stderr` no longer fails on macOS — Replaced `libc::write(2, SHUTDOWN_MSG.as_ptr().cast(), ...)` in the SIGINT and SIGTERM signal handlers with `eprintln!`. The Rust runtime's stderr is reliably captured by `Stdio::piped()` in the test process, while raw fd writes via libc were being lost under cargo test's process-group inheritance. The `SHUTDOWN_MSG` constant was removed as dead code.
- `tests/signal_test.rs` test reliability — Increased `thread::sleep` from 50ms to 2000ms before sending SIGINT. The 50ms was insufficient for the atomwrite child to fully initialize tracing, mimalloc, and signal handlers before receiving the signal. Increased per-file payload from 100 to 1000 lines so the search loop runs long enough to confirm graceful shutdown. The test is now stable across 5 consecutive runs.

## [0.1.6] - 2026-06-05

### Added (README Badges)
- docs.rs badge in README.md and README.pt-BR.md — Added `[![docs.rs](https://img.shields.io/docsrs/atomwrite)](https://docs.rs/atomwrite)` between the Crates.io and License badges. The badge was previously missing from the published README, even though the documentation was being built successfully on docs.rs. The badge now appears in the rendered README on crates.io and on the GitHub repository page.

### Notes
- v0.1.6 is NON-BREAKING. The change is purely visual (badge image in README).
- No code or public API changes.
- No CHANGELOG migration guide required.

## [0.1.5] - 2026-06-05

### Changed (Documentation Hygiene)
- `#![warn(missing_docs)]` promoted to `#![deny(missing_docs)]` — Missing public API documentation is now a hard build error instead of a warning. All public items were already documented in v0.1.4 (verified by `RUSTDOCFLAGS="-D warnings" cargo doc --all-features`), so no documentation was added in this change.
- `#![warn(rustdoc::broken_intra_doc_links)]` promoted to `#![deny(...)]` — Broken intra-doc links now fail the build instead of being silent warnings.
- `#![doc(html_root_url = "https://docs.rs/atomwrite/0.1.2")]` removed — The attribute was hardcoded to the 0.1.2 version, causing intra-doc-links generated for newer versions (0.1.3, 0.1.4) to point to 0.1.2. The attribute is deprecated since rustc 1.48 in favor of the `repository` field, which is already set in `Cargo.toml`. docs.rs now uses the current crate version for link resolution automatically.

### Changed (docs.rs Metadata)
- `[package.metadata.docs.rs]` cleaned up — Removed `all-features = true` (no `[features]` table exists, so the flag was a no-op) and `rustdoc-args = ["--cfg", "docsrs"]` (no `#[cfg(docsrs)]` markers exist in the source). Added `targets = ["x86_64-unknown-linux-gnu"]` to make the docs.rs build target explicit.

### Tests
- 302 tests pass with 0 failures (unchanged from v0.1.4)
- 3 ignored tests (cross-compile Windows tests, unchanged)
- `cargo doc --no-deps --all-features` with `RUSTDOCFLAGS="-D warnings"` passes cleanly

### Notes
- v0.1.5 is NON-BREAKING. The bumped deny lints are already satisfied by the existing code.
- v0.1.5 does not change any public API surface or behavior. It only tightens documentation enforcement and removes deprecated metadata.
- No CHANGELOG migration guide required.

## [0.1.4] - 2026-06-05

### Fixed (Windows Compilation - GAP 14)
- `cargo install atomwrite` no Windows 10/11 — Resolvidos três erros de compilação que bloqueavam a instalação em Windows desde v0.1.3. Erro `E0433` em `src/atomic.rs:404` (tipo `AtomwriteError` usado sem import), erro `E0308` em `src/platform.rs:116` (comparação de `*mut c_void` com literal `0`), e erro `E0507` em `src/atomic.rs:387` (assinatura `&NamedTempFile` mas chamada `.persist()` requer ownership). Todos os três bugs estavam em blocos `#[cfg(windows)]` invisíveis ao CI Linux.

### Fixed (FFI Correctness - GAP 14)
- `src/platform.rs:116` — Substituída comparação `handle != 0` por `!handle.is_null()` para conformidade com o padrão idiomático de raw pointer check em Rust. O `HANDLE` retornado por `GetStdHandle` é um `*mut c_void`; compará-lo com literal inteiro `0` viola o sistema de tipos e disparava `E0308`. Padrão agora é `is_null()` para nulidade e `!= INVALID_HANDLE_VALUE` (que já é `HANDLE`) para validade.

### Fixed (Error Suggestions - GAP 13)
- `WorkspaceJail` suggestion is now context-aware — When the user already supplied a workspace root via `--workspace` or `ATOMWRITE_WORKSPACE`, the suggestion now says "use a path inside the workspace (<root>)" instead of re-prompting the `--workspace` flag. Fixed the phantom `--force-text` flag that did not exist and caused cascading exit 2 errors.
- All 20 error variants now have actionable suggestions — Added suggestions for `InvalidInput`, `Io`, `ConfigInvalid`, `FileImmutable`, `NoMatches`, and `InternalError`. Only `BrokenPipe` (SIGPIPE, not actionable) remains without a suggestion.
- New `ErrorContext` struct — Carries `workspace_provided: bool` and `workspace: Option<PathBuf>`. `ErrorJson::from_error_with_context()` and `output::write_error_json_with_context()` use the context to produce precise suggestions.
- `FileImmutable` suggestion — Now mentions `chattr -i` (Unix) and `fsutil` (Windows) for clearing the immutable attribute.
- `NoMatches` suggestion — Guides the user to broaden the pattern, check `--include`/`--exclude` filters, and verify file content.
- `InternalError` suggestion — Asks the user to report the bug with the reason context.

### Added (Cross-Platform Validation - GAP 14)
- `tests/cross_compile_check.rs` — New cross-compile gate running `cargo check` against `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, and `x86_64-pc-windows-msvc`. Fails if `E0433`, `E0308`, or `E0507` reappear in any `cfg(windows)` block. Tests marked `#[ignore]` to skip gracefully on hosts without Windows targets.
- `output::write_error_json_with_context()` — New helper accepting `&ErrorContext` so the main entry point can propagate `--workspace` provenance to the error output.
- Windows installation documentation — New `docs/INSTALL.md` (EN) and `docs/INSTALL.pt-BR.md` (PT-BR) with Windows 10/11 prerequisites, `cargo install` commands, and troubleshooting.

### Changed
- `src/atomic.rs:13-15` — Moved `use crate::error::AtomwriteError` into a `#[cfg(windows)]` block to avoid `unused_imports` warning on Linux/macOS. The type is only referenced inside `persist_with_retry`.
- `src/atomic.rs:386-409` — `persist_with_retry` now takes `NamedTempFile` by value and recovers the file from `e.file` in the retry branch. Caller updated to pass `temp` by value.
- `src/main.rs:93-105` — Error reporting now constructs an `ErrorContext` with `workspace_provided: cli.global.workspace.is_some()` so the `WorkspaceJail` suggestion adapts to the user's invocation.

### Tests
- 7 new GAP 13 tests in `src/error.rs::tests`: `gap13_workspace_jail_suggestion_when_workspace_not_provided`, `gap13_workspace_jail_suggestion_when_workspace_provided`, `gap13_all_variants_have_suggestion`, `gap13_binary_file_suggestion_does_not_mention_force_text_wrong_flag`, `gap13_file_immutable_suggestion_mentions_chattr`, `gap13_no_matches_suggestion_mentions_filters`, `gap13_error_context_default_matches_legacy_behavior`.
- 1 new GAP 13 integration test in `tests/cli_v012_regressions.rs`: `gap13_jail_suggestion_when_workspace_supplied_says_inside`.
- Existing test `jail_suggestion_mentions_workspace_flag` updated to assert the suggestion mentions `--workspace` only when workspace is NOT provided (GAP 13 fix).

### Notes
- GAPs 01-12 (previously resolved) re-audited via `cargo test --all-features` — all 300+ tests still pass.
- Atomic decision `atomwrite-no-github-actions` retained: release is manual via `cargo publish` local after 8 official gates and the cross-compile gate. CI matrix in `.github/workflows/ci.yml` exists for reference only and is not executed.


## [0.1.3] - 2026-06-03

### Changed (BREAKING)
- Atomic write default behavior for `edit` and `replace` — `AtomicWriteOptions::default()` now sets `preserve_timestamps: false` (was `true`). The mtime of an edited or replaced file is now updated to the moment the write completes, which is the correct default for build systems that use mtime to detect source changes (cargo, make, cmake, gradle, sbt, bazel, ninja, msbuild). For backup, snapshot, or reproducible-build scenarios where the original timestamp must be preserved, use the new `--preserve-timestamps` flag on `edit` and `replace`. Cargo's fingerprint module compares the mtime of source files against the mtime of `target/.fingerprint/<unit>/dep-info` files; with the old default, cargo would skip the rebuild silently (the "Finished in 0.29s" no-op) because the source appeared older than the binary. See the v0.1.2 → v0.1.3 migration guide in `docs/MIGRATION.md` for the upgrade path.

### Added (Build System Awareness)
- `--preserve-timestamps` flag on `edit` and `replace` to opt back into the v0.1.2 behavior of keeping the original file mtime
- `mtime_preserved` field in `EditOutput` and `ReplaceResult` NDJSON responses so consumers can verify whether the timestamp was kept or updated (always present; `true` when `--preserve-timestamps` was passed, `false` by default)
- New regression tests in `src/atomic.rs::tests`: `atomic_write_updates_mtime_by_default` and `atomic_write_preserves_mtime_when_opted_in`

### Added (Documentation)
- New section "Modification Time And Build Systems" in `docs/HOW_TO_USE.md` explaining how cargo, make, cmake, gradle, sbt, bazel, ninja, and msbuild detect source changes via mtime and why the default was changed
- Portuguese equivalent in `docs/HOW_TO_USE.pt-BR.md`
- New recipe "How to Edit and Trigger a Build Without Manual Touch" in `docs/COOKBOOK.md` showing the `atomwrite edit && cargo build` workflow that no longer requires `touch`
- Portuguese equivalent in `docs/COOKBOOK.pt-BR.md`
- All v0.1.2 → v0.1.3 changes documented in `gaps.md` section "Atomic Edit Preserva mtime E Quebra Detecção De Mudança Pelo Cargo" (GAP 12)

### Test Coverage
- 2 new regression tests in `src/atomic.rs` for the default-updates-mtime and opt-in-preserves-mtime contract
- 2 new tests in `src/ndjson_types.rs::tests` updated for the new `mtime_preserved` field in `EditOutput` and `ReplaceResult`
- 2 snapshot files updated: `tests/snapshots/snapshot_write__edit_output_structure.snap` and `tests/snapshots/snapshot_write__replace_result_structure.snap` now include the new `mtime_preserved: false` field in their JSON output
- Total: 33 test suites, 294 tests passing (was 292+ in v0.1.2)

### Validation Gates
- `cargo fmt --check` clean
- `cargo clippy --all-targets --all-features -- -D warnings` zero warnings
- `cargo test --all-features` 33 suites passing
- `cargo doc --no-deps --all-features` zero warnings
- End-to-end behavior verified: file with mtime=2024-01-01 → `atomwrite edit` (default) → mtime=now → `cargo build` rebuilds correctly; `--preserve-timestamps` keeps the 2024-01-01 mtime as expected


## [0.1.2] - 2026-06-02

### Fixed (CRITICAL)
- macOS compilation failure — `nix::fcntl::posix_fadvise` is restricted to `cfg(target_os = "linux")` so atomwrite now compiles on macOS arm64/Intel (the nix crate gates the symbol under `linux_android | emscripten | fuchsia | freebsd` only, breaking macOS previously)
- `batch --transaction` rollback is now real — pre-existing files are restored AND files newly created by `write` operations are removed. The NDJSON `rollback` event now reports `files_restored`, `files_removed`, and `total_reverted` so LLMs can verify the ACID contract. Previously files created mid-transaction were never cleaned up.
- `replace` no longer inflates counters on jail violations — `total_replacements` is incremented only AFTER workspace jail validation passes. Violations now emit a `JailViolation` error event with `error_class: permanent` and `retryable: false`.
- `search` parallel events are grouped by path — parallel walker threads no longer interleave `begin`/`match`/`end` events from different files in NDJSON output. Consumers (LLM and humans) see contiguous event sequences per file.
- `scope --delete` Rust comments no longer leaves orphan whitespace — prepared query for Rust comments now matches trailing whitespace so deletion produces clean code.
- `search` invalid regex emits structured JSON envelope — invalid patterns now fail with `AtomwriteError::InvalidInput` which propagates through `write_error_json` to stdout, not raw stderr.

### Fixed (HIGH)
- `batch --file <PATH>` is now functional — the flag is wired through `cmd_batch` to read the NDJSON manifest from a file (validated against workspace jail) instead of stdin only.
- `backup --output-dir` is now respected — the flag plumbs through `AtomicWriteOptions.backup_output_dir` to `create_backup_in`, which creates the directory if missing and prunes old backups in that directory.

### Fixed (UX)
- Workspace jail error message corrected — `WORKSPACE_JAIL` errors now suggest `--workspace <root>` or `ATOMWRITE_WORKSPACE=<path>` instead of the misleading "use an absolute path" (which was wrong when the path was already absolute).
- Proptest backup retention bug fixed — `cleanup_old_backups_in` now correctly prunes old backups when using `create_backup_in` with a custom output directory.

### Changed (Dependencies)
- `nix` updated from 0.29 to 0.31 (latest stable)
- `signal-hook` updated from 0.3 to 0.4 (latest stable)
- `windows-sys` updated from 0.59 to 0.61 (latest stable)
- `rust-i18n` updated from 3 to 4 (latest stable)
- `nix::fcntl::posix_fadvise` signature changed from `AsRawFd` to `AsFd` in 0.31 — code adapted accordingly

### Added (Agent-First Features)
- `--timeout <SECONDS>` global flag for bounded execution time (at introduction: 0 = no timeout, default 0; since v0.1.33 default is 120, exit 124 on deadline)
- `--grep <REGEX>` flag on `read` to filter returned lines by regex
- `completions --install` to install completion scripts to XDG data directory (`~/.local/share/bash-completion/completions/atomwrite` for Bash, etc.)

### Security
- `cargo audit` baseline 1 vulnerability acknowledged: `RUSTSEC-2026-0009` in `time 0.3.45` (DoS via stack exhaustion). Fix requires `time >= 0.3.47` which needs Rust 1.88. Our MSRV is 1.85, and atomwrite uses `time` only via `tracing-appender` for log timestamps — not exploitable. Tracked for MSRV bump in 0.2.0.


## [0.1.1] - 2026-06-01

### Fixed
### Search and Replace Engine
- `search --include`/`--exclude` now correctly filter files via OverrideBuilder (was silently ignored)
- `replace --include`/`--exclude` now correctly filter files via OverrideBuilder
- `transform --include`/`--exclude` now correctly filter files via OverrideBuilder
- `search --context` now emits context lines via custom SearchSink (was missing entirely)
- `search --max-count` now limits matches per file via SearcherBuilder.max_matches() (was being ignored)
- `search --invert` now shows non-matching lines via SearcherBuilder.invert_match() (was inverted behavior)
- `search --sort` now sorts results by file path (was returning in unspecified order)
- `transform` now processes files in parallel via WalkParallel + crossbeam channel (was sequential)
- `search` regex special characters in patterns now properly escaped when `--literal` is set

### Atomic Writes and Backups
- `batch delete` backup now uses atomic create_backup() with fsync (was racing the write)
- `create_backup` now uses `fs::copy` instead of `fs::hard_link` to prevent backup corruption when original is overwritten in-place (hard links would diverge silently)
- Hardlink detection before atomic rename with `tracing::warn` when nlink > 1
- Same-file detection in `copy` and `move` to prevent source=destination data loss (was overwriting)

### Error Handling and Code Quality
- 12 broken intra-doc links in `error.rs` corrected (`DiskFull` to `Self::DiskFull` and similar)
- Six `unwrap()` calls in `edit.rs` multi-edit mode replaced with `ok_or_else` for safer error handling
- `scope.rs` thread join changed from `unwrap()` to `let _ = join()` to prevent panic propagation
- `rollback.rs` `unwrap()` replaced with descriptive `expect` message
- Exit codes in `output.rs`, `read.rs`, `batch.rs`, and `hash.rs` moved from magic numbers to named constants in `constants.rs`
- `DETECTION_SIZE` in `binary_detect.rs` centralized to `BINARY_DETECT_SIZE` in `constants.rs`

### Output Format
- `read` modified timestamp now returns ISO 8601 format instead of epoch seconds (LLM-readable)
- `# Errors` documentation added to three `output.rs` public functions returning `Result`
- Portuguese test data in `file_io.rs` replaced with English (code-in-english convention)

### Added
### New Subcommands
- `scope` subcommand for grammatical scoping: select AST categories (comments, functions, strings, etc.) and apply actions (delete, upper, lower, titlecase, squeeze, replace)
- `scope` supports Rust (30 prepared queries), Python (13), JavaScript/TypeScript (11), Go (8), and custom AST patterns via `--pattern`
- `backup` subcommand for creating timestamped file backups with BLAKE3 checksums and configurable retention
- `rollback` subcommand for restoring files from previous backups with optional BLAKE3 verification
- `apply` subcommand for applying patches from stdin with auto-detection of format (unified diff, SEARCH/REPLACE blocks, markdown-fenced diff, full file replacement)

### Batch Operations Expansion
- `batch` supports 7 operations: write, replace, delete, edit, hash, move, copy
- `batch --transaction` flag for all-or-none execution with automatic rollback
- `batch` move and copy operations now accept `source`, `from`, and `src` as field aliases for the source path
- `batch` write, delete, edit, and hash operations now accept `path` as alias for `target`

### Edit Engine Enhancements
- `edit --fuzzy` flag with cascade of 7 matching strategies (exact, line_trimmed, whitespace_normalized, indent_flexible, escape_normalized, trimmed_boundary, block_anchor)
- `edit --multi` flag for applying multiple NDJSON edit operations in a single atomic write
- `edit` NDJSON output includes `fuzzy`, `strategy`, `strategies_tried`, `similarity` fields when fuzzy matching is used

### Path Safety
- FIFO and device file detection in path validation (exit codes 85 and 86) — prevents atomic writes to special files

### Internationalization (i18n)
- `--lang` global flag for locale override (en, pt-BR) with `ATOMWRITE_LANG` environment variable
- i18n support via `rust-i18n` and `sys-locale`: automatic OS locale detection with en and pt-BR translations
- All user-facing strings now locale-aware (errors, warnings, info messages)

### Internationalization Documentation
- `ARCHITECTURE.md` bilingual documentation (en + pt-BR) describing module map, data flow, and key decisions
- SPDX license headers (`MIT OR Apache-2.0`) in all 64 `.rs` source files
- Module-level `//!` documentation in all 38 source modules
- Executable doctests for `is_binary`, `detect`, and `normalize` functions
- `[package.metadata.docs.rs]` configuration for docs.rs builds
- `documentation` field and `[badges.maintenance]` in `Cargo.toml`
- Rustdoc lints: `broken_intra_doc_links`, `private_intra_doc_links`, `clippy::doc_markdown`
- `doc(html_root_url)` for docs.rs cross-linking

### Supply Chain and Security
- `deny.toml` for cargo-deny license and advisory auditing
- Line ending detection and normalization module (`line_endings.rs`)
- `--line-ending lf|crlf|cr|auto` flag on `write` and `edit` for line ending normalization

### Test Infrastructure
- 282 tests across integration and unit test suites (was 5 tests in 1 module at v0.1.0)
- Integration tests for `backup`, `rollback`, `apply`, and `scope`
- 2 fuzz targets (`batch_parse`, `extract_json`) with `libfuzzer-sys` for parser security testing
- Optimistic locking integration tests for `write --expect-checksum` and `edit --expect-checksum`
- NDJSON validation tests expanded from 5 to 20 of 21 commands
- `jaq` interop tests validating NDJSON piped through `jaq` filter
- i18n integration test confirming `--lang` does not alter JSON output

### Security
- SPDX license headers ensure license clarity in all source files
- cargo-deny enforces license compliance and tracks security advisories
- FIFO and device file detection prevents accidental writes to special files
- Hardlink detection prevents silent data corruption when atomic rename breaks hard links

### Known Limitations (fixed in v0.1.2)
- `batch --file <PATH>` flag was declared in help but not wired through to command logic
- `batch --transaction` did not delete files created during failed transactions (only restored modified files)
- `replace` incremented counters BEFORE workspace jail validation, producing contradictory NDJSON counts
- `search` invalid regex produced raw stderr error instead of JSON envelope
- `search` parallel walker interleaved begin/match/end events for different files
- `scope --delete` for Rust comments left orphan whitespace
- macOS compilation failed (nix 0.29 gated `posix_fadvise` to non-macOS Unix)
- Default `--workspace` was CWD-silent (no warning when capturing everything)
- WORKSPACE_JAIL error message suggested "use an absolute path" even when path was already absolute
- `backup --output-dir` was declared but not plumbed through
- 4 dependencies frozen (nix 0.29, signal-hook 0.3, windows-sys 0.59, rust-i18n 3)
- `read` had no `--head`/`--tail`/`--grep` flags for LLM context window control
- `completions` did not auto-install
- No global `--timeout` flag for unbounded operation termination


## [0.1.0] - 2026-05-29
### Added
- 22 subcommands: `read`, `write`, `edit`, `search`, `replace`, `hash`, `delete`, `count`, `diff`, `move`, `copy`, `list`, `extract`, `calc`, `regex`, `transform`, `batch`, `completions`, `scope`, `backup`, `rollback`, `apply`
- Atomic write pipeline: tempfile + fsync + rename + directory fsync on every write operation
- BLAKE3 checksums on every `read` and `write` response
- Optimistic locking via `--expect-checksum` flag
- NDJSON output contract: every stdout line is a JSON object with a `"type"` discriminator
- Structured error responses on stdout with `error: true`, including `error_class`, `retryable`, and `suggestion` fields
- Parallel file search powered by the ripgrep engine (`grep-regex`, `grep-searcher`, `grep-matcher`)
- `.gitignore`-aware traversal via the `ignore` crate
- Structural AST search and rewrite via the ast-grep engine, over the built-in grammars of the pinned `ast-grep-language` crate (28 languages in 0.43.0 and 29 in 0.51.0, both counted in the crate source)
- Regex generation from examples via grex
- Math expression evaluation and unit conversions via fend-core
- Unified diff output via the `similar` crate
- Memory-mapped file reads via `memmap2` for large files
- Workspace jail enforcement via `--workspace` to prevent path escapes
- Symlink blocking to prevent traversal outside workspace boundaries
- Batch operations from NDJSON manifests supporting write, replace, and delete
- Shell completion generation for bash, zsh, fish, elvish, and PowerShell
- Signal handling for SIGINT, SIGPIPE, and SIGTERM with clean shutdown
- Cross-platform support: Unix (nix, libc) and Windows (windows-sys)
- 20 distinct exit codes for precise error classification
- `NO_COLOR` environment variable support
- `RUST_LOG` environment variable for log verbosity control
- Release profile with LTO, single codegen unit, symbol stripping, and panic=abort


[Unreleased]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.37...HEAD
[0.1.37]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.36...v0.1.37
[0.1.36]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.35...v0.1.36
[0.1.35]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.34...v0.1.35
[0.1.34]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.33...v0.1.34
[0.1.33]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.32...v0.1.33
[0.1.32]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.31...v0.1.32
[0.1.31]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.30...v0.1.31
[0.1.2]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/danilo-aguiar-br/atomwrite/releases/tag/v0.1.0
