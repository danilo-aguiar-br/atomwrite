# atomwrite Migration Guide


[Leia em Português](MIGRATION.pt-BR.md)


## Current Version
- atomwrite is at v0.1.37
- This document covers migration from v0.1.0 through v0.1.37
- See the sections below for additive changes and breaking changes in each version


## Work folded into v0.1.37: read expansion and search ceiling (Current)
### What Changes
- `read` expands a pattern whose directory prefix runs several levels deep, and only a wildcard inside a directory name exits 65, because the old check counted every component of the path instead of asking which one is a pattern
- An operand with `*`, `?` or `[` that exists on disk is read literally instead of being expanded, so `read` and `cat` agree on a file such as `report[1].txt`
- `search` derives the line buffer ceiling from `search.max_filesize` instead of a fixed cap, so a file the caller already allowed to be read is no longer refused for having one line above that cap, and the error names the key to raise
- Pin: `cargo install atomwrite --locked --version "^0.1.37"`
- Surface: 47 subcommands, unchanged from v0.1.36, and the single source of truth is still `atomwrite agent-surface`


## Work folded into v0.1.37: read batching and the workspace jail
### What Changes
- `read` takes one or more paths. Explicit paths are read in argv order, and an operand that carries `*`, `?` or `[` is a pattern the binary expands itself against one level of its own directory, in `LC_ALL=C` byte order, skipping names that start with a dot. The shell is no longer needed for the expansion
- `--format raw` concatenates every target with no header, no separator and no newline of its own, so `atomwrite read --format raw '*.rs' | cmp - <(LC_ALL=C cat *.rs)` exits 0 with an empty diff. One target in this format is byte-identical to 0.1.37, and the default NDJSON format is untouched
- The batch copies in 64 KiB blocks and checks `--max-filesize` per target, so memory stays flat instead of holding the sum of the files, and every block advances the cancel check, so `--timeout-secs` cuts a batch with exit 124
- A target that fails prints one line on stderr and the run continues with the next one. The final status is the semantic code of the first failure: 4 for a missing path, 13 for a permission, 126 outside the `--workspace`. `cat` would answer 1 for all of them, so a consumer that tests for 1 must test for the code the envelope names
- A `read --format raw` error now goes to stderr and never to stdout, because that channel promised bytes and a JSON envelope there was a defect
- `--lines`, `--line`, `--context`, `--head`, `--tail`, `--grep`, `--stat` and `--verify-checksum` require a single target and exit 65 with more than one, and `read` with several targets without `--format raw` also exits 65
- A pattern with `**` or with more than one separator exits 65 with its own message, because recursing is out of scope and matching something else in silence is worse than refusing
- Pin: `cargo install atomwrite --locked --version "^0.1.37"`
- Surface: 47 subcommands, unchanged from v0.1.37, and the single source of truth is still `atomwrite agent-surface`


## v0.1.37 Release Notes
### What Changes
- Risk assessment: `edit`, `edit-loop`, `replace`, `case`, `transform` and `codemod` now publish `risk_assessment` when the payload they would write matches a content-risk pattern, exactly as `write` already did. The field is absent when no pattern matches, and the size barriers stay out of these verbs on purpose, because they rewrite a region and a shorter result is the operation working
- New exit status: `69` `GRAMMAR_UNAVAILABLE` with class `precondition_failed`. A tree-sitter grammar that is missing and cannot be downloaded used to arrive as 255 `INTERNAL_ERROR` with a suggestion telling the user to file a bug; it now names the language and says to run the verb once with the network up
- Config surface: `write.content_risk_patterns` is now reachable with `config get` and `config set` as a list value, so the risk patterns are XDG configuration instead of a compiled literal. The table holds 37 keys
- `log.file` and `log.rotation` are accepted in the workspace scope too: the `[log]` section of `.atomwrite.toml` now starts the log sink, where before it was validated and discarded
- `--version` no longer appends `-dirty`: the identifier is the short commit id of the tracked tree, so an untracked file no longer changes the string
- `batch --transaction` publishes one verdict: the `rollback` event carries `ok: false` with the `code` and `exit` of the failure that caused it, instead of `ok: true` before the failure
- Pin: `cargo install atomwrite --locked --version "^0.1.37"`
- Surface: 47 subcommands, unchanged from v0.1.37, and the single source of truth is still `atomwrite agent-surface`


## v0.1.36 to v0.1.37

### What Changes
- Stream shaping: `--sort-by` (`path-line` / `line-path` / `key`), `--dedupe KEY`, `--aggregate`, `--join`, `--diff-against FILE`, `--redact`, `--normalize-encoding` and `--max-output-bytes` join the global shape. Ordering, dedupe and the stream comparison buffer the whole run and are applied at the end, so they work on every subcommand without per-command flags
- Projection key: every success envelope carries `ok: true`; `verified` stays a checksum verdict and still requires `--verify`
- `transform` subject: the positionals after `transform` are required, because a verb with side effects receives its target in its own invocation line
- Continuous mode: `watch` left the default feature set. The default binary answers with the closed-feature error (exit 78, `code: CONFIG_INVALID`); `agent-surface` reports `available: false` and the `--features watch` install hint. Install with `--features watch` to keep it
- Schema discovery: `--print-schema` lists every schema name, or emits one by name, without a subcommand. The flag must precede the command name; placed after it, the call fails with a usage error instead of replacing the command with a listing. Both spellings work, `--print-schema NAME` and `--print-schema=NAME`. An unknown name leaves the data channel as an error envelope with `code: INVALID_INPUT` and exit 65, where it previously exited 1 with no payload at all
- Interrupted runs: every scanning verb now ends its data channel on the `cancelled` envelope, which carries `deadline_exceeded` so an elapsed `--timeout-secs` budget (exit 124) is distinguishable from a signal (exit 130/143) without parsing `reason`. `search` and `count` previously published nothing on a cancel and a reader could mistake the last partial line for a final result. The terminal cancel record is written unshaped, so `--filter`, `--select` and `--limit` cannot drop it
- Artifact signal: `extract-artifact` reads PDF, OOXML and image headers and emits bounded signal, never raw artifact bytes
- Rewrite path: the test that asserted `watch_summary` on idle is now gated on the `watch` feature, the backup matrix passes a subject to `transform`, and the jail-suggestion assertion pins `--locale en`
- `edit-loop` contract: all-or-nothing by default. A pair with no match exits 65 and nothing is written. `edit-loop --partial` writes the pairs that match and exits 87 (`EDIT_LOOP_PARTIAL`)
- Mandatory paths: `replace`, `scope`, `codemod`, `case`, `delete`, `backup` and `prune-backups` require their paths. `wal-heal` and `scratch init` take the path as a positional, and `recipe run` needs `--path` or `--target`. Without a target the exit is 2, and `replace foo src/` no longer reads `src/` as the new text
- Target evidence: the terminal event of every verb with side effects carries `target_resolved` and `target_source` (always `argv`)
- `doctor`: it reads no environment variable. The `ci`, `github_actions` and `sudo` fields are gone from its environment block, and the policy check about a tool-protocol server is gone from its report
- `agent-surface`: the tool-protocol key is gone from the manifest
- Strict config: an unknown key or an invalid file in `config.toml` or `.atomwrite.toml` exits 78 (`CONFIG_INVALID`). A `version` key (current value 1) is accepted, and a newer one is rejected
- Log file: off by default. Set `[log] file = true` in the global XDG `config.toml` to turn it on (daily or hourly rotation, 7 files, mode 0600)
- Write lock: automatic and cross-platform. `std::fs::File::try_lock` wraps `flock` on Unix and `LockFileEx` on Windows, so a write waits up to 5 s for the sidecar `.<target>.atomwrite.lock` and exits 83 (`LOCK_TIMEOUT`) on timeout, and Windows was not validated
- Lock sidecar name: it keeps the literal `.<target>.atomwrite.lock` up to 255 bytes, and past that the stem becomes a 16 character digest of the target name in the same directory, so a long target name no longer fails with `ENAMETOOLONG`
- `search`: it honours `--timeout-secs` at about 1x, and every `match` sits inside the `begin` / `end` pair of its own file. `files_skipped` on the summary counts files skipped for size
- Stdin: write, edit, edit-loop, apply, hash `--stdin`, extract, diff, calc and regex obey SIGTERM (143), SIGHUP (129) and the deadline (124) with stdin open
- `config` verb (the 47th subcommand): `config path|list|get|set|unset|doctor` reads and writes 37 persistent keys. `--scope global|workspace` is required on `path`, `set` and `unset`; `list` and `get` show the effective value and its origin. A write keeps the comments of the file, validates the whole file and lands atomically (mode 0600 for the global file on Unix)
- Config layering: the workspace `.atomwrite.toml` and the global `config.toml` are now merged key by key (flag, then workspace, then global, then built-in). Before, a workspace file replaced the global one as a whole
- One terminal event: a failing summary (`search`, `replace`, `transform`, `edit-loop`, `semantic-merge`, `recipe`, `scope`) carries `ok: false`, `code`, `exit` and the error detail itself, and no second error envelope follows. A consumer that waited for a trailing error line after the summary must read the summary instead
- `MATCH_FAILED` for a missing pair: a pair with no match in `edit-loop` or in a multi-pair `edit` now reports code `MATCH_FAILED` (exit 65, with `best_candidate`); before, the same failure reported `INVALID_INPUT`
- `files_failed` and `partial`: every summary that publishes `files_failed` publishes `partial: true` beside it, and that rule covers `search`, `transform`, `replace`, `scope`, `count`, `list`, `sparse`, `semantic-search` and `case`. Both fields come from the same count, so they can never disagree
- `errors[]`: each entry carries `path`, `code` and `message`, and the list stops at `summary.errors_max_entries` entries (50 by default) while `files_failed` keeps the full count. `search`, `transform`, `replace` and `scope` keep exit 0 when they answered and a failure was found during the walk, while `count`, `list`, `sparse`, `semantic-search`, `hash`, `copy`, `delete` and `case` fail the run over a partial tree, a root named in the invocation that does not exist exits 4 `FILE_NOT_FOUND` even when it matched, and a run with no match and a failure exits with the first failure
- Fuzzy ceiling: a file above `[fuzzy].max_content_bytes` (default 256 MiB) fails under fuzzy with exit 65 and a hint; `--fuzzy off` stays exact-only; the `escape_normalized` strategy is skipped above 32 MiB
- Broken pipe: exit 141 whether the OS delivers SIGPIPE or the write returns `BrokenPipe`
- Retry and limits: lock and Windows rename retries use backoff with jitter and obey `--timeout-secs`; the automatic worker count stops at 16 threads (an explicit `-j N` is honoured); `search` keeps match data in flight under a byte budget (8 MiB by default)
- Startup WAL heal: runs only on verbs that mutate, and only over the directories named in argv
- Slim build: `outline` and `sparse outline` exit 78 (`CONFIG_INVALID`) with a rebuild hint when the `ast` feature is missing
- Schemas: `docs/schemas/` has 54 files that follow `--print-schema` (the procedure to refresh them is in `docs/schemas/README.md`): success events list `ok`, `code`, `exit`, `target_resolved` and `target_source` as optional properties, and every verb that writes more than one record shape now serves one union instead (`recipe`, `codemod`, `batch`, `search`, `replace`, `list`, `backup`, `semantic-search`, `sparse`, `semantic-merge`, `case` and `watch`), while `count-by-ext` and `count-by-size` are new names
- Pin: `cargo install atomwrite --locked --version "^0.1.37"`

### Breaking Changes for Consumers
- Reading guide: each item says what a consumer saw before 0.1.37, what it sees now and the exit code
- `apply` with a unified diff: each hunk now compares its context and removed lines with the file (trailing spaces ignored) and is found at the header line or at the nearest exact neighbour within 64 lines. A mismatch exits 65 `MATCH_FAILED` with `best_candidate` and writes nothing. Before, a patch `@@ -1 +1 @@ -zzz +yyy` turned line 1 into `yyy` with exit 0
- `apply`, insertion-only hunk: it inserts AFTER the named line, where it used to insert one line above. An unreadable `@@` header and a patch with no hunk exit 65 `INVALID_INPUT`
- `apply` with SEARCH/REPLACE: any missing block fails the whole batch with `MATCH_FAILED` (exit 65) and nothing is written, and a block that matches in two places exits 65 `MATCH_AMBIGUOUS` with `match_count`. Before, a missing block left the file half applied with exit 0 and `hunks_applied: 1`. `apply` has no `--partial`
- `semantic-merge` with `--fail-on-conflict` and a conflict: exit 84 `MERGE_CONFLICT` (`error_class: permanent`, with `conflict_count`), where it exited 65 `INVALID_INPUT`. The merge result with `status: "conflict"` is the only terminal line
- `STATE_DRIFT` (exit 82, `retryable: true`) is now possible in `case`, `transform`, `scope` and `codemod`: a file that changed between the read and the write fails for that file and the run must be repeated. Before, the later writer overwrote the change without notice
- Read-modify-write verbs serialise on the advisory lock: `edit`, `edit-loop`, `set`, `del`, `config set`, `config unset`, `apply`, `write --append`, `write --prepend` and the `replace` and `edit` ops of `batch`. Thirty parallel `edit` on distinct lines used to exit 0 with `ok: true` and keep 4 to 10 of the 30 changes
- Where `flock` does not exist (NFS) the lock degrades, and only `edit`, `replace`, `case`, `transform`, `scope` and `codemod` keep the checksum net; this was not validated on NFS
- Invalid user input is exit 65 `INVALID_INPUT` and no longer 255 `INTERNAL_ERROR`: the regex of `replace` and `search`, the globs of include and exclude in `search`, `replace`, `transform`, `scope`, `list`, `count`, `delete`, `sparse` and `hash -r`, AST patterns, YAML rules (file and inline), `query -Q`, `read --grep` (it was ignored in silence), `locale --set` and the ops of `edit --multi`
- `sparse list --include '['` and `hash -r --exclude '['` exited 0 in silence and now exit 65. `sparse list --include` is applied (it used to be ignored) and `--exclude` is a real glob
- Fuzzy tie: an absent block among similar lines is `MATCH_FAILED` (exit 65, `match_count: 0`, with `best_candidate`), where it was `MATCH_AMBIGUOUS`. A real tie stays `MATCH_AMBIGUOUS` with `match_count: 2`. It affects `edit`, `replace`, `apply` and `batch`
- `edit-loop` and a multi-pair `edit`: a pair with no match is `MATCH_FAILED` (it was `INVALID_INPUT`), `edit-loop` writes nothing unless `--partial` is passed, and `--partial` exits 87 `EDIT_LOOP_PARTIAL`
- Every summary that publishes `files_failed` reports unreadable or unwritable files as `files_failed` and `errors[]` (at most `summary.errors_max_entries` entries (50 by default)), plus `partial: true` whenever a file failed. `search`, `transform`, `replace` and `scope` keep exit 0 when they answered and a failure was found during the walk, while `count`, `list`, `sparse`, `semantic-search`, `hash`, `copy`, `delete` and `case` fail the run over a partial tree, so read `partial` and `files_failed` before trusting a result. A root named in the invocation that does not exist exits 4 `FILE_NOT_FOUND` even when it matched, and `--format text` writes every failure to stderr with its path and reason. A run with no match and a failure exits with the first failure, where it exited 1 `NO_MATCHES`
- `null` values are no longer written at the top level of the stdout events: a key whose value would be `null` is absent (test `r9_no_null_top_level_values`). A consumer that compared a field with `null` must test for the missing key, for example `jaq '.field // empty'`
- `--max-output-bytes` needs `--allow-truncate`, and each flag is rejected without the other with exit 2 `ARGUMENT_PARSE_ERROR`. The cut applies to every string value, `checksum` included, so do not use both when the whole `checksum` or `content` is needed
- `doctor --strict` with a failing check: exit 1 `DOCTOR_CHECKS_FAILED` and one terminal line (the `doctor_report`), where it exited 255. Exit 1 is shared with `NO_MATCHES`, so branch on `code`
- Broken pipe: exit 141 when the reader closes the pipe, also for `read`, which exited 0 before. Unix installs a cooperative SIGPIPE handler (a deliberate deviation, ADR-0063): the write that hits the closed pipe fails with `BrokenPipe`, every `Drop` runs and no lock, journal or tempfile is left. The output is incomplete, so check the state before assuming that the work finished
- An interrupted `write` exits with the real status of the signal or of the deadline (143, 130 or 124), where it exited 255
- `get`, `set` and `del` on an invalid TOML or JSON file: exit 65 `INVALID_INPUT` with only line, column and a short cause, where they exited 1 without NDJSON and quoted the whole file in the message
- A failure with no type publishes an `INTERNAL_ERROR` envelope (exit 255) on stdout, where it printed one stderr line with exit 1
- `read --verify-checksum` with a wrong hash emits an error envelope and exits 81, where stdout and stderr were empty
- `search.max_inflight_bytes`: the minimum rose from 1 MiB to 4 MiB, and a `.atomwrite.toml` that still says 1 MiB is refused when it loads (exit 65 `INVALID_INPUT`, with the accepted range 4194304..=1073741824 in the message)
- Backup retention deletes only `<name>.bak.<valid stamp>`, where `f.txt.bak.0-user-note` could be deleted
- `semantic-search --index-dir` outside the workspace jail exits 126, where it read and wrote outside the jail
- `case --subvert` rejects an OLD identical to NEW with exit 65, and `edit --multi` rejects overlapping ranges with exit 65 and leaves the file untouched

### Library API
- The supported interface of atomwrite is the CLI and its NDJSON contract. The crate also publishes a library, and its surface is NOT covered by SemVer in this release
- `cargo semver-checks --baseline-version 0.1.36` (cargo-semver-checks 0.50.0) on this tree exits 100 with 10 major checks failed and 0 minor
- `auto_trait_impl_removed`: `LockGuard` is no longer `Send` and `Sync`
- `constructible_struct_adds_field`: new public fields in `GlobalArgs`, `OutputShape`, `AtomwriteConfig`, `ErrorJson`, `CancelledEvent`, `EditLoopArgs`, `WalHealArgs`, `SearchSection`, `FuzzySection`, `LocaleState`, `MatchOpts` and `ScratchInitEvent`
- `enum_missing`: `locale::Idioma`, now `locale::Language`
- `enum_no_repr_variant_discriminant_changed`: `LocaleSource::Persisted`, `LocaleSource::System` and `LocaleSource::Default` changed their discriminant
- `enum_variant_added`: `Commands::ExtractArtifact` and `Commands::Config` on an exhaustive enum
- `enum_variant_missing`: `LocaleSource::EnvVar`
- `feature_not_enabled_by_default`: `watch`
- `function_missing`: `signal::reset_sigpipe`, `locale::negotiate_to_idioma` and `locale::resolved_idioma`
- `pub_module_level_const_missing`: `LOCK_POLL_FAST_ATTEMPTS`, `PERSIST_RETRY_DELAYS_MS`, `LOCK_POLL_SLOW_MS` and `LOCK_POLL_FAST_MS`
- `struct_pub_field_missing`: `RuntimeEnvironment.ci`, `RuntimeEnvironment.sudo` and `LocaleState.idioma`
- A project that depends on the `atomwrite` crate as a library should pin `=0.1.36` until it has been ported. A project that only runs the binary is not affected by this list

### Rollback
- Return to the previous release with an exact version: `cargo install atomwrite --locked --version 0.1.36 --force`
- A requirement such as `^0.1.36` is not a rollback, because Cargo resolves it to the newest 0.1.x, which is 0.1.37
- Rolling back brings back the old behaviours listed above, among them the unchecked `apply` of a unified diff, which can destroy content with exit 0

### Validation (v0.1.37)
- Reproduced on `atomwrite 0.1.37 (no-git) x86_64-unknown-linux-gnu` in an isolated workspace with `HOME` and `XDG_*` pointing inside it
- `apply` of `@@ -1 +1 @@ -zzz +yyy` on a file without `zzz`: `MATCH_FAILED`, exit 65
- `semantic-merge --fail-on-conflict` with both sides changing the same line: exit 84
- `replace --regex '('`, `list --include '['`, `hash -r --exclude '['` and `sparse list --include '['`: exit 65 `INVALID_INPUT`
- `get` on a TOML file that does not parse: exit 65
- `--max-output-bytes 10 read FILE`: exit 2, and with `--allow-truncate` it exits 0 and cuts `checksum` to 10 characters
- `read FILE | head -c 5` and `search PATTERN FILE | head -1` on a 50000-line file: exit 141
- `write` with an open stdin and `--timeout-secs 1`: exit 124 with a `cancelled` event carrying `deadline_exceeded: true`
- `.atomwrite.toml` with `[search] max_inflight_bytes = 1048576`: exit 65 `INVALID_INPUT`
- `semantic-search "unwrap value" src/ --index-dir /tmp/OUTSIDE`: exit 126 `WORKSPACE_JAIL`
- `case --subvert foo foo`: exit 65, and `read FILE --verify-checksum deadbeef`: exit 81 `CHECKSUM_VERIFY_FAILED`
- `--print-schema` lists 55 names and `--print-schema nope` exits 65
- Not reproduced here: `STATE_DRIFT` under a real race, `doctor --strict` with a failing check (this host has none), interrupted `write` by SIGTERM and SIGINT, NFS, macOS and Windows

### The 47 Subcommands in 0.1.37
- One example per subcommand, each run against `atomwrite 0.1.37` in an isolated workspace; the list is the one in `atomwrite agent-surface`
- Setup: `WS` is the absolute path of an empty workspace that holds `src/a.rs` (an `unwrap()` and a comment), `src/b.rs` (the word `beta` once), `src/c.rs` (with `user_id`), `src/tmp.rs`, `config.toml`, `notes.txt`, `tmp.txt`, `base.txt`, `ours.txt`, `theirs.txt`, `img.png` and `rules.yaml`; `HASH_A` is the `checksum` that `hash src/a.rs` prints
- The examples run in the order below (`rollback` needs the `backup` before it and `edit-loop` needs the `edit` before it). 46 of the 47 exit 0, and `watch` exits 78 because the default build has no `watch` feature
- The full setup text is in `docs/INSTALL.md`

- `read` — Read a file with checksum, size and lines, optionally only head/tail/range. Example: `atomwrite --workspace "$WS" read --head 2 src/a.rs`
- `write` — Create or overwrite a file atomically from stdin. Example: `printf 'x\n' | atomwrite --workspace "$WS" write notes/new.txt`
- `edit` — Edit a file by exact or fuzzy text match, line number or marker. Example: `atomwrite --workspace "$WS" edit src/b.rs --old beta --new gamma`
- `delete` — Delete files or trees with backup, or only print the plan. Example: `atomwrite --workspace "$WS" delete --plan tmp.txt`
- `copy` — Copy a file or tree with checksum verification. Example: `atomwrite --workspace "$WS" copy src/a.rs src/a_copy.rs`
- `move` — Move or rename a file atomically. Example: `atomwrite --workspace "$WS" move src/tmp.rs src/tmp2.rs`
- `apply` — Apply a patch (unified diff, SEARCH/REPLACE, full file) from stdin. Example: `printf 'new body\n' | atomwrite --workspace "$WS" apply --format full notes.txt`
- `search` — Search file contents in parallel with the ripgrep engine. Example: `atomwrite --workspace "$WS" search alpha src/`
- `replace` — Replace text across files with atomic writes and fuzzy matching. Example: `atomwrite --workspace "$WS" replace --dry-run alpha omega src/`
- `transform` — Rewrite code by AST pattern (ast-grep). Example: `atomwrite --workspace "$WS" transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- `hash` — Print the BLAKE3 checksum of files. Example: `atomwrite --workspace "$WS" hash src/a.rs`
- `count` — Count lines and files by extension or by size. Example: `atomwrite --workspace "$WS" count --by-extension src/`
- `diff` — Compare two files (unified, stat or changes). Example: `atomwrite --workspace "$WS" diff src/a.rs src/b.rs --unified`
- `list` — List a directory tree with metadata. Example: `atomwrite --workspace "$WS" list --long --depth 1 src/`
- `extract` — Pick fields from NDJSON read on stdin. Example: `printf '{"a":1}\n' | atomwrite --workspace "$WS" extract --field a`
- `extract-artifact` — Bounded signal from PDF, OOXML and image headers (never raw bytes). Example: `atomwrite --workspace "$WS" extract-artifact img.png`
- `scope` — Grammatical scoping: act on comments, strings, functions and other code categories. Example: `atomwrite --workspace "$WS" scope src/ --language rust --query comments --delete --dry-run`
- `regex` — Generate a regex from example strings. Example: `atomwrite --workspace "$WS" regex abc-12 xyz-99 id-7 --digits`
- `calc` — Math and unit conversion. Example: `atomwrite --workspace "$WS" calc "2 hours to seconds"`
- `completions` — Print or install shell completions (bash, zsh, fish, elvish, powershell). Example: `atomwrite --workspace "$WS" completions bash`
- `backup` — Create timestamped backups with BLAKE3 checksums. Example: `atomwrite --workspace "$WS" backup src/a.rs --retention 3`
- `rollback` — Restore a file from a previous backup. Example: `atomwrite --workspace "$WS" rollback src/a.rs --latest --verify`
- `batch` — Run NDJSON operations from stdin, optionally as one transaction. Example: `printf '{"op":"write","target":"batch.txt","content":"x"}\n' | atomwrite --workspace "$WS" batch --transaction`
- `edit-loop` — Apply N old/new pairs in one write, all or nothing. Example: `printf '[{"old":"gamma","new":"delta"}]\n' | atomwrite --workspace "$WS" edit-loop src/b.rs`
- `prune-backups` — Remove old .bak files by age or count. Example: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run .`
- `wal-stats` — Show the state of WAL journals (read only). Example: `atomwrite --workspace "$WS" wal-stats`
- `wal-heal` — Remove orphan terminal WAL journals older than a threshold. Example: `atomwrite --workspace "$WS" wal-heal --threshold-secs 3600 .`
- `verify` — Check a file against an expected BLAKE3 hash (exit 0 match, 81 mismatch). Example: `atomwrite --workspace "$WS" verify src/a.rs HASH_A`
- `set` — Write a value at a dotted path in a TOML or JSON file. Example: `atomwrite --workspace "$WS" set config.toml package.name demo2`
- `get` — Read a value at a dotted path in a TOML or JSON file. Example: `atomwrite --workspace "$WS" get config.toml package.name`
- `del` — Remove a key from a TOML or JSON file. Example: `atomwrite --workspace "$WS" del config.toml features.x`
- `case` — Rename identifiers across files between case styles. Example: `atomwrite --workspace "$WS" case --subvert user_id UserId --to pascal --dry-run src/c.rs`
- `query` — Walk the tree-sitter AST and emit nodes. Example: `atomwrite --workspace "$WS" query --kinds src/a.rs`
- `outline` — Emit the high-level structure of a source file. Example: `atomwrite --workspace "$WS" outline src/a.rs`
- `semantic-merge` — Line-based three-way merge with conflict markers (not AST). Example: `atomwrite --workspace "$WS" semantic-merge --base base.txt --ours ours.txt --theirs theirs.txt --output merged.txt --fail-on-conflict`
- `sparse` — Budgeted list, read and outline for large trees. Example: `atomwrite --workspace "$WS" sparse list src/ --max-files 10 --max-bytes 1048576`
- `recipe` — List and run built-in multi-step recipes. Example: `atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern alpha --replacement omega --path src --dry-run`
- `stat` — Metadata only, alias of read --stat. Example: `atomwrite --workspace "$WS" stat src/a.rs`
- `agent-surface` — Tool manifest for agents (the SSOT of subcommands). Example: `atomwrite --workspace "$WS" agent-surface --format json`
- `watch` — Stream filesystem events with debounce; ends with watch_summary (needs a build with `--features watch`, the default build exits 78). Example: `atomwrite --workspace "$WS" watch src/ --max-events 1 --idle-exit-ms 500`
- `codemod` — Apply a YAML list of AST rewrite rules across a tree. Example: `atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/`
- `semantic-search` — Offline Jaccard ranking of files by query. Example: `atomwrite --workspace "$WS" semantic-search "unwrap value" src/ --k 5 --min-score 0.01`
- `doctor` — Host readiness diagnostics. Example: `atomwrite --workspace "$WS" doctor`
- `scratch` — Create the staging dir .atomwrite/scratch inside the workspace jail. Example: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch`
- `locale` — Show, set or clear the UI locale. Example: `atomwrite --workspace "$WS" locale`
- `commands` — Print the full command tree as JSON. Example: `atomwrite --workspace "$WS" commands --include-globals`
- `config` — Read and write persistent tuning keys (37 keys, scopes global and workspace). Example: `atomwrite --workspace "$WS" config get fuzzy.threshold`

### Step-by-Step Migration

1. Pin `^0.1.37`
2. If you rely on `watch`, install with `--features watch` or move to a polling `list` loop
3. Replace inline `sort` / `uniq` / `diff` steps with `--sort-by`, `--dedupe` and `--diff-against` so the CLI answers on the data channel
4. Pass a subject to `transform`; a call without paths is now a usage error instead of an implicit current-directory scan
5. Parsers that read a checksum verdict keep reading `verified`; parsers that read the operation outcome now find `ok` on every success envelope
6. Move `--print-schema` to the front of the line, before the command name, and read an unknown name from the error envelope instead of from an exit 1 with no payload
7. Read `deadline_exceeded` on a `cancelled` envelope to tell a deadline from a signal. A consumer that treated the last line of a cancelled `search` as a final result must now expect the cancel record there instead
8. In `edit-loop` callers, treat exit 65 as "nothing written" and pass `--partial` only when a partial write is acceptable, then handle exit 87
9. Pass an explicit path to `replace`, `scope`, `codemod`, `case`, `wal-heal`, `scratch init` and `recipe run`, and read `target_source` to confirm which target ran
10. Stop reading `ci`, `github_actions` and `sudo` from `doctor`, and stop reading the tool-protocol key from `agent-surface`
11. Validate every `config.toml` and `.atomwrite.toml` against the strict schema before upgrading, because an unknown key now stops the run with exit 78
12. Do not pass a lock flag or a lock-wait flag: the flags that older docs mentioned never existed, and the lock is automatic
13. Move any tuning that used to live in a hand-edited TOML to `config set KEY VALUE --scope global|workspace`, and check that a workspace file no longer hides the global keys it does not name
14. In scripts that read the last line of `search`, `replace` or `transform`, read `files_failed` and `errors[]` on the summary, and treat a summary with `ok: false` as the only error line of the run
15. Branch on `MATCH_FAILED` for a missing `edit-loop` pair and read `best_candidate` as you do for `edit`
16. Review every script that relied on exit 255 or exit 1 for a bad input: an invalid regex, glob, AST pattern or YAML rule exits 65, `doctor --strict` with a failing check exits 1 `DOCTOR_CHECKS_FAILED`, an interrupted `write` exits 143, 130 or 124, and `get`, `set` and `del` on an invalid file exit 65
17. Treat `apply` exit 65 as "nothing written": read `best_candidate`, fix the context of the patch and run again, and do not resend the same patch
18. Handle exit 84 (`MERGE_CONFLICT`) from `semantic-merge --fail-on-conflict`, and exit 82 (`STATE_DRIFT`, retryable) from `case`, `transform`, `scope` and `codemod`: repeat the run for the files that failed
19. If you pass `--max-output-bytes`, add `--allow-truncate`, and do not use the pair when you need the whole `checksum` or `content`
20. Stop comparing top-level fields with `null`: test for the missing key
21. Look for `[search] max_inflight_bytes` below 4 MiB in every `.atomwrite.toml`, because the file is refused with exit 65
22. If your code depends on the `atomwrite` crate as a library, pin `=0.1.36` or port it (see Library API)
23. Treat exit 141 as an incomplete output and check the state of the files before going on


### See Also
- `CHANGELOG.md`, section 0.1.37, for the complete list of changes, measurements and notes
- `docs/INSTALL.md` for the install profiles, the sizes, the XDG paths and the 37 config keys
- `docs/schemas/README.md` for the 54 schema files and the name-to-file map
- `docs/decisions/README.md` for ADR-0058 onward (the v0.1.37 decisions, among them the SIGPIPE handler in ADR-0063)
- `docs/CROSS_PLATFORM.md` for what was and was not validated on each platform


## v0.1.35 to v0.1.36 (superseded by v0.1.37)

### What Changes
- Staging: prefer `atomwrite scratch init .atomwrite/scratch` then `--old-file`/`--new-file` under `.atomwrite/scratch/` (WORKSPACE_JAIL 126 on OS `/tmp` remains)
- Error envelope: always `ok: false` on failures; `match_count` present on match failures
- Agent stdout: global `--select`/`--fields`, `--filter`, `--limit` (long-only — no global `-n`). Errors always emit even if `--filter` would drop a success event; `--limit` counts success events only; `--select` may thin error keys
- extract: local `--field` (repeatable) or positionals; global `--select` projects the emitted line (cannot name extract flag `--select` — collides with global). Legacy CAPA/plans that say `extract --select` → use `extract --field`
- Fuzzy: `similarity: null` on normalized strategies; dual-gate block_anchor; XDG `soft_floor_delta`
- Subcommands (historical, v0.1.36): 46 SSOT (`Commands` / `agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`) — not “41” or incomplete “42”
- Historical pin: `cargo install atomwrite --locked --version "^0.1.36"`

### Step-by-Step Migration

1. Pin `^0.1.37`
2. Replace `/tmp` old/new files with workspace scratch (`scratch init`)
3. Parsers: check exit and `ok`/`error` before success fields (stdout may still show an error line when exit ≠ 0 even under aggressive `--filter`)
4. Prefer `--select ok,error,code,match_count,suggestion` and `--limit N` instead of jq/`-n` for agent loops
5. Local gate: `cargo test --tests`


## v0.1.34 to v0.1.35

### What Changes
- Large overwrite default-deny (A-WRITE-001): overwriting an existing file larger than XDG `[write].confirm_large_bytes` (bootstrap default 100 KiB) now requires `--ack-overwrite`. Without it the command exits 65. No interactive Y/N prompt (one-shot).
- `--require-large-ack` is independent (A-WRITE-002): it is no longer a clap alias of `--confirm`. You may pass both; large targets still need `--ack-overwrite`.
- `delete --confirm` / `delete --yes` rejected (B-014/B-015 / A-TEST-001): use `delete --plan` or `--dry-run` for plan-only; omit flags to actually delete. Update scripts and tests that still pass `--yes` or `-y`.
- `watch` always emits `watch_summary` (A-WATCH-001): even with zero events / idle exit, stdout includes a final NDJSON line `type:watch_summary`.
- Watch idle default 500 ms (A-ONESHOT-001): was ~3000 ms. Override with `--idle-exit-ms` or XDG / `.atomwrite.toml` `[watch].idle_exit_ms`.
- Semantic-merge markers default ON (A-MERGE-001): conflict output writes `<<<<<<< ours` markers. Prefer-ours without markers: `--no-conflict-markers` or `--write-conflict-markers false`.
- XDG watch debounce (A-XDG-002): `[watch].debounce_ms` + CLI `--debounce-ms`.
- Honest multi-OS (A-XPLAT-001): Linux full e2e; Windows-gnu `cargo check` on Linux host; macOS needs a real Mac or osxcross. Local gates only.
- Config surface: product knobs are CLI + XDG / `.atomwrite.toml` only (no `ATOMWRITE_*` runtime knobs). Since 0.1.37 `doctor` reads no environment variable at all (file probes only).

### Step-by-Step Migration

1. Historical (0.1.34 → 0.1.35 pin at that release): `cargo install atomwrite --locked --version "^0.1.35"` (or `cargo install --path . --force` from source). Current line is `^0.1.37`.
2. Agents that overwrite large files: add `--ack-overwrite` (and optionally `--require-large-ack` for explicitness).
3. Agents that used `delete --confirm` as “preview”: switch to `delete --plan`.
4. Agents that used `delete --yes` / `-y`: remove those flags; they now fail with exit 65.
5. Parsers that assumed empty stdout on idle `watch`: accept `type:watch_summary`.
6. Pipelines that expected clean prefer-ours files on merge conflict: pass `--no-conflict-markers` if markers are unwanted.
7. Re-run local gates: `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`, `cargo test --test cli_e2e_v0135`.

### Compatibility Notes
- Non-large writes and the fuzzy one-shot contract from 0.1.33/0.1.34 remain.
- Historical (at v0.1.35): the documented surface was 41 subcommands. Current (v0.1.37): SSOT is 47 (`doctor`, `locale`, `commands`, `scratch`, `extract-artifact` and `config` complete the surface) — do not treat “41 remain” as current.
- See [ADR-0055](decisions/0055-v0-1-36-agent-contract-gaps.md) for the A-* closeout matrix; see [v0.1.35 to v0.1.36](#v0135-to-v0136-superseded-by-v0137) for extract `--field`, global shape, and scratch staging.

### Rollback
- Pin back to `^0.1.34` if you must restore the previous large-write opt-in and watch empty-stdout behavior (not recommended for agents).


## v0.1.35 — no product environment knobs

Breaking (runtime config): the CLI no longer reads `ATOMWRITE_*` or `RUST_LOG`/`NO_COLOR` as product configuration.
Use only:
- CLI flags (`--workspace`, `--locale`, `--no-backup`, `-v`/`-q`, `--color`, `--threads`, …)
- `atomwrite set` / `get` + XDG `config.toml` (`[storage].home`, `[fuzzy].threshold_*`, …)
- optional workspace `.atomwrite.toml`

Rewrite older examples that used `export` of product workspace/backup knobs (removed) to `--workspace` and `--no-backup`.
Logging: `init_tracing` + CLI; optional file tee under XDG state — no product log environment knobs.
`wal-stats` is local diagnostics (not product telemetry).


## v0.1.33 to v0.1.34

### Docs alignment (same runtime as 0.1.33 oneshot)
- No new runtime behavior relative to v0.1.33. Code fix was logged as 0.1.33; 0.1.34 is the docs-complete publish.
- Documentation (AGENTS, HOW_TO_USE, COOKBOOK, TESTING, INSTALL, MIGRATION, ADRs) now documents:
  - Global `--timeout-secs` / `--timeout` default 120, `0` disables, deadline exit 124
  - Fuzzy multi-apply one-pass L→R (`apply_fuzzy_one_pass`); never re-scans inserted text
  - Default max fuzzy applies 1; hard ceiling 10_000; embeds force single apply; caps and cooperative cancel mid-cascade
- Regression suite remains `tests/cli_v0133_oneshot_fuzzy.rs`

### Migration Action
- Pin `^0.1.34`: `cargo install atomwrite --locked --version "^0.1.34"` or `cargo install --path . --locked --force`
- If you already run 0.1.33, no behavioral change is required — only version pin and docs
- Run: `cargo test --test cli_v0133_oneshot_fuzzy`


## v0.1.32 to v0.1.33

### BREAKING Changes
- Global `--timeout-secs` default is 120 (was 0 = disabled). Long monorepo jobs must pass `--timeout-secs 0` or a higher value. Deadline → exit 124. Alias `--timeout`.
- Fuzzy multi-apply default is 1 when `--max-replacements` is omitted (was unbounded). Explicit `--max-replacements N` still multi-hits one-pass (never re-scans inserted text).
- If `replacement` contains `pattern` (fixed-string), fuzzy forces a single apply even with a huge `--max-replacements` (prevents infinite growth).
- Hard ceiling on fuzzy applies: 10_000 (`FUZZY_HARD_MAX_REPLACEMENTS`), even if `--max-replacements` is larger.
- Caps: pattern 64 KiB; lev 8192 chars; max windows 4096; growth max(4×, +16 MiB).

### Fixed (critical)
- `replace --fuzzy` infinite loop / multi-day CPU hang when NEW embeds OLD (agent expand-section edits).

### Migration Action
- Agents: keep using `replace --fuzzy auto`; hang is fixed. Prefer unique OLD blocks; NEW may embed OLD for expansions.
- CI/scripts that relied on unlimited runtime: add `--timeout-secs 0` only if intentional.
- Prefer pin `^0.1.34` (docs-complete); `^0.1.33` has the same runtime.
- Regression: `cargo test --test cli_v0133_oneshot_fuzzy`


## v0.1.29 to v0.1.30

### BREAKING Changes
- `--fuzzy off` is rejected (exit 65) with a migration note; only `auto` and `aggressive` remain. Superseded in v0.1.35: Off = exact-only (G-010 CLOSED).
- `.atomwrite.toml` with `[fuzzy] mode = "off"` fails parse with INVALID_INPUT. Superseded in v0.1.35: `mode = "off"` is exact-only.
- Multi-match without `--replace-all` fails with MATCH_AMBIGUOUS (exit 65)

### Added (migration-relevant)
- EditOutput NDJSON fields: `match_count` and optional `indent_adjusted`
- Escape-drift guard; unicode_normalized strategy; ranked best_candidate and candidates[]
- search --target content|files|both with --offset and --limit; similar_paths on path miss
- recipe hash excludes *.bak.*; sparse outline emits real AST outline_item under budget
- config [fuzzy] applied to edit/replace/loop/batch; platform.backup_method is reflink_or_copy
- Contract suite tests/cli_v0130_agent_contract.rs

### Migration Action
- Remove `--fuzzy off` from scripts and agent prompts; use auto or aggressive (historical 0.1.30 advice; as of v0.1.35 `off` is exact-only again)
- Remove mode = "off" from .atomwrite.toml [fuzzy] (historical 0.1.30; as of v0.1.35 exact-only is allowed)
- For bulk multi-occurrence edits pass --replace-all and parse match_count with jaq
- Prefer jaq on match_count and indent_adjusted over full file re-reads after edit
- Update version pin: cargo install atomwrite --locked --version "^0.1.30"
- Run: cargo test --test cli_v0130_agent_contract


## v0.1.28 to v0.1.29

### BREAKING Changes
- Default fixed-string `replace` now uses `fuzzy=auto` after exact multi-match finds zero hits — previously exact-only failure returned exit 1 with no fuzzy fallback
- Pipelines that required exact-only failure (exit 1 on zero exact hits) must not rely on exact-only (as of v0.1.30 `--fuzzy off` is rejected). Superseded in v0.1.35: `--fuzzy off` = exact-only (G-010); exact miss → `MATCH_FAILED` exit 65.

### Added (migration-relevant)
- Shared `fuzzy` module: 9-strategy cascade for `edit`, `replace`, `batch`, `edit-loop`
- `best_candidate` structured diagnostics on match failure (error envelopes, exit 65 paths)
- `replace --fuzzy auto|off|aggressive` and `--fuzzy-threshold` (in 0.1.29 only; `off` rejected since 0.1.30)
- `replace --progress-every` and batch progress heartbeats
- Cargo features: `core`, `ast`, `lang-*`, `watch`, `semantic`, `full` — slim install without AST (historical: the `semantic` feature was removed later and `semantic-search` is always compiled)
- New subcommands: `semantic-merge`, `sparse list|read`, `recipe list|run`, `stat` (alias of `read --stat`), `agent-surface`, `watch` (feature `watch`), `codemod`, `semantic-search`
- `write --durability full|fast|auto` with NDJSON `platform.durability`
- Linux `renameat2` in `atomic_rename` with ENOSYS fallback
- Backup uses reflink_or_copy (never hardlink of the live file)
- Cooperative cancel during stdin read and chunked atomic write
- Versioned recipe docs under `recipes/*.yaml`
- Local gates: size-gate (slim core ≤ 15 MiB), core-test, schema-diff

### Changed
- Strategy numbering normalized to nine named strategies
- Historical (at v0.1.29): skill surface documents 41 subcommands
- Slim core release ~7.7 MiB (local size-gate ≤ 15 MiB); default/full with AST ~52 MB — PRD 5–8 MB applies to core only

### Migration Action
- Update version pin: `cargo install atomwrite --locked --version "^0.1.30"`
- BREAKING (v0.1.30): exact-only via `--fuzzy off` was rejected; only auto/aggressive remained. Superseded in v0.1.35 (G-010): `--fuzzy off` = exact-only again; exact miss on edit → `MATCH_FAILED` exit 65 (not the same as tree-wide `NO_MATCHES` exit 1)
- On match failures, inspect optional `best_candidate` in the NDJSON error envelope for near-miss diagnostics
- Slim install (no AST): `cargo install --path . --locked --no-default-features --features core`
- Full install (default features): `cargo install --path . --locked` or `cargo install atomwrite --locked --version "^0.1.30"`
- Use `agent-surface` for clap-derived tool inventory (integration by subprocess, CLI with NDJSON output)
- Optional durability: `write --durability full|fast|auto` — read `platform.durability` in NDJSON
- Linux operators: atomic rename may use `renameat2` (falls back on ENOSYS)
- Feature-gated: enable `watch` for `watch` (historical: there is no `semantic` feature in 0.1.37, `semantic-search` is always compiled)
- MSRV unchanged at Rust 1.88

### Test Coverage
- Core-only build works without AST crates (stubs exit 78)
- Recipe run dispatches real search→replace→hash (no longer plan-only stub)
- Regenerated schemas: write durability/rename_method, replace fuzzy*, error best_candidate, recipe/progress/cancelled


## v0.1.27 to v0.1.28 (2026-07-06)

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

### Bug Fixes (High) — GAP-CONFIG-DEFAULTS-DEAD
- `.atomwrite.toml` `[defaults]` `backup`/`retention` keys are now effective across every mutating subcommand — `resolve_backup()` previously took only `(backup: bool, no_backup: bool)` and never consulted the parsed config
- Precedence is now CLI flags (`--backup`/`--no-backup`/`--retention`) > `.atomwrite.toml` `[defaults]` > XDG config > built-in default (`true` / `5`) — no product env knobs (v0.1.35)

### Migration Action
- Update version pin: `cargo install atomwrite --locked --version "^0.1.28"`
- BREAKING: If you rely on `delete` never creating a backup by default, pass `--no-backup` explicitly — the default flipped from opt-in to opt-out
- BREAKING: If you script `move`/`copy` to overwrite an existing destination, pass `--force` or `--backup` — the two subcommands now enforce this consistently
- If you previously worked around `replace --retention` or `delete --no-backup` returning exit 2: both now parse successfully
- If you pass both `--backup` and `--no-backup` together: this now fails fast with exit 2 instead of the last flag silently winning
- If your `.atomwrite.toml` sets `[defaults]` `backup`/`retention`: these are now honored across every mutating subcommand — verify your project config reflects the intended behavior
- If you invoke `edit`'s stdin-consuming modes (`--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between`, `--multi`) interactively without piping stdin: this now fails fast with exit 65 `INVALID_INPUT` instead of blocking indefinitely
- MSRV unchanged at Rust 1.88

### Test Coverage
- 661 tests passing, 0 failures, 3 ignored
- 33 subcommands, 32 ADRs in `docs/decisions/`


## v0.1.26 to v0.1.27 (2026-06-24)

### Security Fixes (CRITICAL)
- BUG-SEC-001: symlink-directory escape from workspace jail — `ln -s /tmp $WS/link && atomwrite write link/file.txt` could write OUTSIDE the workspace. Fixed with `canonicalize_existing_prefix` in `path_safety.rs`. Affects: write, read, edit, edit-loop, apply, set, del, copy, move. Exit 126 on detection.

### Bug Fixes (CRITICAL)
- BUG-SCOPE-004: `scope --query comments --delete` on lines with inline comments (e.g., `fn foo() {} // comment`) destroyed the entire line including valid code. Fixed: `expand_to_full_line` now preserves code before inline comments.

### Bug Fixes (Medium)
- BUG-GET: `get` for missing key now returns exit 65 (`INVALID_INPUT`) with message "key 'X' not found in Y" — reverses v0.1.26 revert to exit 4
- BUG-001: `edit-loop` with empty stdin now returns exit 65 (`INVALID_INPUT`) instead of silent exit 0
- BUG-002: `edit-loop` with invalid JSON now emits NDJSON error envelope on stdout instead of unstructured text on stderr
- BUG-005: `write --syntax-check` suggestion changed from "--syntax-check=false" to "remove --syntax-check"
- BUG-008: `edit-loop` `pair_results` now includes `old` and `new` fields for each pair
- BUG-SCOPE-002: `scope` queries `fn`, `struct`, `enum`, `trait`, `async-fn`, `unsafe-fn` now include `pub` variants
- BUG-SCOPE-003: `scope` queries `const`, `static`, `type-alias`, `mod`, `use` now include `pub` variants
- BUG-SCOPE-005: Go `scope --query var` now matches declarations without explicit type (`var x = 0`)

### Known Limitations
- GAP-01: `scope --query test-fn` unavailable — ast-grep cannot match `#[test] fn` across 2 AST nodes
- GAP-02: `scope --query doc-comment` unavailable — tree-sitter parses `///` as `line_comment` (no structural distinction)
- GAP-03: `scope --query export` in JS/TS unavailable — pattern crosses multiple AST nodes

### Migration Action
- Update version pin: `cargo install atomwrite --locked --version "^0.1.27"`
- CRITICAL: If you use symlinks inside workspace directories, test that they resolve within the jail. Symlinks pointing outside the workspace now correctly return exit 126.
- If you parse `get` exit codes for missing keys: exit is now 65 (`INVALID_INPUT`) again (was 4 in v0.1.26, 65 in v0.1.25)
- If you parse `edit-loop` `pair_results`: new `old` and `new` fields are present
- If you rely on `scope` queries matching only non-pub items: `fn`, `struct`, `enum`, `trait`, `const`, `static`, `type-alias`, `mod`, `use` now capture both pub and non-pub
- MSRV unchanged at Rust 1.88

### Test Coverage
- 631+ tests passing, 0 failures
- 33 subcommands, 29 ADRs in `docs/decisions/`


## v0.1.25 to v0.1.26 (2026-06-23)

### Bug Fixes (May Affect Behavior)
- `replace` with zero matches now returns exit 1 (`NO_MATCHES`) instead of exit 0 (GAP-141)
- `transform` with zero matches now returns exit 1 (`NO_MATCHES`) instead of exit 0 (GAP-145)
- `scope` with zero matches now returns exit 1 (`NO_MATCHES`) instead of exit 0 (GAP-146)
- `read --verify-checksum` with wrong hash now returns exit 81 (`CHECKSUM_VERIFY_FAILED`) instead of exit 82 (GAP-136)
- `get` for missing key REVERTED to exit 4 (`FILE_NOT_FOUND`) — reverses v0.1.25 GAP-111 change to exit 65 (GAP-137)
- `search --sort path` now guarantees deterministic global ordering via BTreeMap buffer (GAP-147)
- `scope` Rust queries now use dual AST patterns to capture generics and return types (GAP-144)
- `verify` checksum is now a positional argument: `verify <PATH> <HASH>` (GAP-135)
- `edit-loop` now accepts JSON array input in addition to NDJSON (GAP-142)

### Documentation Clarifications (BY-DESIGN)
- `query --kinds` output is NDJSON stream (one object per kind), not single JSON array (GAP-138)
- `case` requires `--subvert OLD NEW` — global scanning not implemented (GAP-139)
- `write` with empty stdin requires `--allow-empty-stdin` flag (GAP-140)
- `prune-backups` flag is `--max-age-secs` (not `--max-age`) to make unit explicit (GAP-143)

### Migration Action
- Update version pin: `cargo install atomwrite --locked --version "^0.1.26"`
- If you parse `get` exit codes for missing keys: exit is now 4 (was 65 in v0.1.25, was 4 in v0.1.24)
- If you check `replace`/`transform`/`scope` exit codes: zero matches now returns exit 1 (was exit 0)
- If you pass the expected hash to `verify` through a flag: pass it as the positional argument instead, `verify <PATH> <HASH>`
- If you use `prune-backups --max-age`: change to `--max-age-secs`
- MSRV unchanged at Rust 1.88

### Test Coverage
- 631+ tests passing
- 13 gaps resolved (9 CODE + 4 BY-DESIGN) across 5 audit rounds
- See `gaps.md` for the full audit


## v0.1.24 to v0.1.25 (2026-06-22)

### Additive (Non-Breaking)
- `.atomwrite.toml` configuration file with hierarchy: CLI > env > local > XDG global > defaults
- New `verify` subcommand (delegates to `hash --verify`)
- Jaro-Winkler fuzzy matching strategy for short strings in edit
- `edit --fuzzy-threshold <FLOAT>` for configurable match sensitivity
- `scope` actions `symbols` and `normalize` (NFC)
- `delete --older-than` with human-readable duration
- `delete --confirm` as preview mode (superseded in v0.1.35: use `--plan`; `--confirm` rejected)
- `replace --preserve-case` with case adaptation
- `search --pcre2` flag (returns exit 65 when feature not enabled)
- `transform --verify-parse` re-validates output via tree-sitter
- `scope --query comments` now captures block comments in Rust
- `copy --preserve` now copies source permissions and timestamps
- `outline --positions` now emits byte offsets and column positions
- Property-based fuzzy tests via proptest

### Bug Fixes (May Affect Behavior)
- `write --backup` no longer reports phantom `backup_path` (critical data integrity fix)
- `set` rejects descent into scalar TOML values (was silently misrouting)
- `case` without `--subvert` now exits 65 (was silent no-op)
- I/O errors via anyhow now emit NDJSON envelopes (7 subcommands affected)
- `hash` output field renamed from `value` to `checksum`
- `batch` move/copy now requires `"force":true` to overwrite existing targets
- `get`/`del` on missing key returns INVALID_INPUT (exit 65) instead of FILE_NOT_FOUND (exit 4)
- `list --long` modified field now emits ISO 8601 format
- `size_delta_pct` in risk_assessment changed from u8 to u32
- Risk assessment default changed to disabled (255)

### Migration Action
- Update version pin: `cargo install atomwrite --locked --version "^0.1.25"`
- If you parse `hash` output: field `value` is now `checksum`
- If you use `case` without `--subvert`: now exits 65 (add `--subvert` pairs explicitly)
- If you parse `get`/`del` exit codes for missing keys: exit changed from 4 to 65
- If you parse `batch` move/copy: existing targets now require `"force":true`
- MSRV unchanged at Rust 1.88

### Test Coverage
- 631 tests passing (621 in v0.1.24 + 10 new for v0.1.25)
- ~505 e2e scenarios across 6 audit rounds
- See `gaps.md` for the full audit of 64 gaps


## v0.1.23 to v0.1.24 (2026-06-21)

### Behavioral Changes (Breaking)
- Backup timestamp format: `YYYYMMDD_HHMMSS` changed to `YYYYMMDD_HHMMSS_mmm` (millisecond suffix)
- `rollback --timestamp` now accepts PREFIX match (e.g. `--timestamp 20260621_120000` matches `file.bak.20260621_120000_042`)
- `prune-backups --max-count` now sorts by FILENAME (lexicographic) instead of mtime
- `replace` rejects empty pattern with exit 65 (previously destroyed files silently)
- `get` of non-existent key returns exit 4 (was exit 0 with `found: false`)
- `hash` of non-existent file returns exit 4 (was exit 0 silently)
- `regex` removed `allow_hyphen_values` on examples field; use POSIX `--` for hyphen-starting examples

### Error Handling (Breaking for parsers)
- 20 `anyhow::bail!()` calls converted to typed `AtomwriteError` variants
- ALL user-facing errors now emit structured JSON on stdout with correct exit codes
- Exit codes changed: many paths that returned exit 1 now return exit 4 (NotFound) or exit 65 (InvalidInput)
- Commands affected: `set`, `del`, `get`, `query`, `outline`, `edit`, `batch`, `write`, `prune-backups`, `extract`

### Bug Fixes (May Affect Existing Behavior)
- `delete --recursive` NOW WORKS (was a no-op for directories)
- `hash --recursive` NOW WORKS (was accepted but never walked)
- `search --multiline` NOW WORKS (was not propagated to SearcherBuilder)
- `scope --query comments --delete` now removes entire line_comment nodes (was removing only `//`)
- `scope`/`count`/`transform` now resolve walk roots against `--workspace` (was using CWD)
- `diff` now resolves paths against `--workspace`
- `batch --transaction` now reverts `move`/`copy` operations on rollback
- `case --subvert` changed from greedy `num_args=2..` to exact `num_args=2`
- `get` JSON/TOML values no longer doubly-quoted
- `set`/`del` `old_value`/`removed_value` no longer doubly-quoted

### Additive (Non-Breaking)
- `write` risk_assessment skipped for `--append`/`--prepend`
- `scope` summary emits `files_modified: null` in read-only mode
- `wal-stats`/`wal-heal` NDJSON output now includes `type` field
- `hash --stdin` no longer requires PATHS argument
- `edit --multi` field `op` is now optional (inferred as `"exact"`)
- `read --format raw` skips binary heuristic
- `read --line/--lines` no longer panics on out-of-range indices
- `read --lines/--head/--tail` empty range returns empty content instead of `"\n"`
- `set` TOML nested keys now correctly return `old_value`
- `regex` emits warning when examples look like flags
- `--require-backup` now detects `--no-backup` override
- ARGUMENT_PARSE_ERROR (exit 2) gains context-aware `suggestion` field

### ADRs Added
- ADR-0045: Actionable suggestion for clap parse errors
- ADR-0046: diff resolve-first retrofit
- ADR-0047: scope read-only mode fix

### Migration Action
- Update version pin: `cargo install atomwrite --locked --version "^0.1.24"`
- If you parse exit codes: update handlers for paths that changed from exit 1 to exit 4 or exit 65
- If you use `rollback --timestamp`: old format still works (prefix match is backward-compatible)
- If you use `prune-backups --max-count`: results may differ (lexicographic vs mtime ordering)
- If you match backup filenames with regex: update to accept optional `_\d{3}` suffix
- If you pass empty pattern to `replace`: this is now rejected (was a silent data-destruction bug)
- MSRV unchanged at Rust 1.88

### Test Coverage
- 621 tests passing (609 in v0.1.23 + 12 new for v0.1.24)
- 3 ADRs added (0045-0047)
- See `gaps.md` for the full audit of 52 resolved issues


## v0.1.22 to v0.1.23

### Behavioral Changes (Action Required)
- backup-by-default: all 9 content-mutating commands (`write`, `edit`, `edit-loop`, `replace`, `transform`, `apply`, `set`, `del`, `case`) now create a backup BEFORE writing by default. The backup is auto-deleted after success (existing `keep_backup: false` default unchanged). If your pipeline depends on NO backup file being created (e.g., checks for `.bak.*` files), add `--no-backup` to the command
- shrink guard: `write --expect-checksum` now BLOCKS writes that shrink the file by more than 50%. If your pipeline legitimately truncates files while using `--expect-checksum`, add `--allow-shrink` to the command. Without `--expect-checksum`, behavior is unchanged

### Additive Changes (No Action Required)
- `allow_hyphen_values`: 12 CLI fields across 8 structs now accept values starting with `-`, and the three positionals (`search` pattern, `replace` pattern and replacement) require `--` since 0.1.37. Previously these caused exit 2 (ARGUMENT_PARSE_ERROR). No migration needed — this fixes a bug
- `edit --old-file <PATH> --new-file <PATH>`: new flags that read match/replacement content from files instead of CLI arguments. Bypasses kernel ARG_MAX (~131 KB). Cross-mixing `--old` with `--new-file` (or vice versa) returns exit 65. No migration needed — these are new opt-in flags

### Migration Checklist
- If using `write` without `--backup`: no action needed (backup auto-deletes after success)
- If checking for `.bak.*` file absence in automated local runs: add `--no-backup`
- If using `write --expect-checksum` to legitimately truncate files: add `--allow-shrink`
- If passing values starting with `-` to `edit --old`, `search`, `replace`, `calc`, `regex`, `transform`, `read --grep`, `query --query`: the fix is automatic, no migration needed


## v0.1.22 — What Is New

This release adds 2 new subcommands to address the legacy backup cleanup and the N-edits-in-1-invocation operator pattern. Both are additive: no flag, schema, or default behavior changed for existing commands.

### Subcommands Added
- `prune-backups [PATHS]...` — manual cleanup of legacy `.bak.YYYYMMDD_HHMMSS` siblings
  - Flags: `--max-age-secs <SECONDS>` (remove backups older than N seconds), `--max-count <N>` (keep the N most recent), `--dry-run` (preview without removing anything)
  - NDJSON output with per-backup lines and a summary
  - See `docs/decisions/0040-prune-backups-subcommand.md`
- `edit-loop <PATH>` — apply N `{old, new}` substitution pairs from NDJSON on stdin in 1 invocation
  - Flags: `--workspace`, `--partial`, `--allow-sequential-drift`, `--syntax-check`, `--line-ending`, `--backup`, `--keep-backup`, `--retention`; it does not accept `--expect-checksum`, `--fuzzy` or `--preserve-timestamps`
  - NDJSON output: one `result` event with `pairs_total`, `pairs_applied`, `pairs_unmatched` and `pair_results[]`
  - Since v0.1.37 a pair with no match exits 65 `MATCH_FAILED` and nothing is written; `--partial` writes the pairs that match and exits 87
  - See `docs/decisions/0039-edit-loop-helper.md`

### Validation
- Historical (at v0.1.22): 575+ tests passing, 2 new ADRs (0039, 0040)
- 2 new NDJSON schemas: `edit-loop-output.schema.json`, `prune-backups-output.schema.json`
- Cross-compile of the 3 Windows targets was reported then and was not re-run for v0.1.37


## v0.1.21 — What Is New

This release closes 3 GAP-2026 items (012, 013 Problem C, 014 v2) and adds 1 ADR (0038). The most visible change is that `--backup` operations now DELETE the backup after success by default; add `--keep-backup` to preserve it. The second visible change is that `edit` and `rollback` now accept `--backup`, closing the API parity hole from v0.1.20. The third change is `--allow-sequential-drift` on `edit` for sequential pipelines.

### Backup Operations
- `write --backup` and `replace --backup` DELETE the backup after success by default (at v0.1.21)
- `edit --backup` and `rollback --backup` are NEW in v0.1.21; the flag is honored on all 4 mutating subcommands
- `--keep-backup` is the OPT-IN flag to preserve the backup after success on `write`, `edit`, `replace`, `rollback`, `apply`, and `batch`
- `apply --keep-backup` and `batch --keep-backup` are NEW in v0.1.21 for parity
- Backups are ALWAYS preserved on the FAILURE path, regardless of `--keep-backup`

### Sequential Edit Pattern
- Chaining multiple `edit` calls on the same file without re-capturing `checksum_after` produces `STATE_DRIFT` (exit 82) on every call after the first
- Two valid patterns: re-capture checksum (Pattern A) or pass `--allow-sequential-drift` (Pattern B)
- Default behavior is unchanged: `STATE_DRIFT` still fires on checksum mismatch when the flag is absent

### Example — Pattern A

```bash
# Initial checksum
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')

# Edit 1 — pass the captured checksum
printf 'line 2\n' | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --after-line 1

# Re-capture the checksum after the edit
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')

# Edit 2 — use the new checksum
printf 'line 3\n' | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --after-line 2
```

### Example — Pattern B

```bash
# Edit 1 — initial checksum
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')
printf 'line 2\n' | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --after-line 1

# Edit 2 — drift allowed, the file no longer matches CS (exit 0 with a warning on stderr)
printf 'line 3\n' | atomwrite --workspace . edit --expect-checksum "$CS" --allow-sequential-drift src/main.rs --after-line 2
```

- Both patterns were run on 0.1.37 with a file that holds `line 1`; the second edit of Pattern B without `--allow-sequential-drift` exits 82 `STATE_DRIFT`

### Migration Notes
- Breaking change — `write --backup` and `replace --backup` no longer leave a `.bak` sibling on disk after a successful write. Add `--keep-backup` to any script that relies on the backup surviving the operation, or read the backup before the write completes
- Breaking change — `edit` and `rollback` now accept `--backup`, and ignore it without complaint if the preconditions of the atomic layer reject it. The explicit `--backup` flag is the new opt-in; old scripts that call `edit` and assume no backup still get no backup by default
- Non-breaking — `apply --keep-backup` and `batch --keep-backup` are additive. The default behavior (no backup) is unchanged

### Validation
- Historical (at v0.1.21): 555+ tests passing (542 baseline v0.1.20 + 13 new), 1 new ADR (0038), 3 GAP-2026 items closed (012, 013 Problem C, 014 v2)
- Cross-compile of the 3 Windows targets was reported then and was not re-run for v0.1.37


## v0.1.20 — What Is New

This release introduces a new safety layer called intention guards and renames the global `--lang` flag to `--locale` to disambiguate from the tree-sitter `--lang` selector used by `scope` and `transform`.

### Intention Guards (5 OPT-IN flags)
- `--require-backup` — require `--backup`: abort when it is missing and the target file exists (takes no value)
- `--confirm` — legacy awareness flag on `write`, with no effect by itself: a large overwrite needs `--ack-overwrite`, and `delete --confirm` is rejected (use `--plan`)
- `--auto-rotate` — force an automatic rotation backup for recently modified files (age from `[write].auto_rotate_max_age_secs`, default 24 h; takes no value)
- `--risk-threshold <PERCENT>` — size-delta threshold in percent for the risk warning (default 255 = off; for example `--risk-threshold 50`)
- `--locale <en|pt-BR>` — renamed from `--lang` to disambiguate from the tree-sitter `--lang`

### Other Additions
- `count --by-size` — list the largest files in the tree with sizes and line counts
- `read --format raw|ndjson` — select between byte-stream output and structured NDJSON envelope (an old `--mode` spelling never existed)
- `search --no-begin-end` — disable the implicit `^` and `$` anchor decoration in regex output
- `write --preserve-timestamps` — keep the source file mtime when overwriting
- `scope --lang rust` — explicit alias of `scope --language`; `transform` takes `-l`/`--language` only

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

# Or via ruplacer (a dry run until you add --go)
ruplacer -- '--lang' '--locale' .
```


## v0.1.12 to v0.1.15

### Additive (G117)
- `edit` multi-pair `--old`/`--new` runs the full 9-strategy fuzzy cascade per pair (was exact-only).
- The success envelope gains `pairs_total` and `pair_results`; the error envelope gains `failed_pair_index`, `pairs_total`, `pair_results` (still `INVALID_INPUT`, exit 65, file untouched).
- New opt-in `edit --partial` applies the matching pairs and reports the rest; zero matches maps to `NO_MATCHES` (exit 1) with no write.

### Behavioral Fix (G118)
- `write` resolves the target via the workspace jail BEFORE append/prepend, line-ending auto-detection, and `--expect-checksum`.
- With a CWD outside the workspace: `--append`/`--prepend` no longer truncate, `--line-ending auto` detects the existing file again, and a divergent `--expect-checksum` exits 82 (`STATE_DRIFT`) instead of silently overwriting. Out-of-jail targets exit 126 early.
- Breaking only for callers that depended on the buggy silent overwrite.

### Migration Action (v0.1.15)
- Update version pin: `cargo install atomwrite --locked --version "^0.1.15"`
- MSRV unchanged at Rust 1.88. No code changes required for compliant callers.
- See ADR-0026 and ADR-0027 in `docs/decisions/` for the full rationale.


## What's New in v0.1.12

This section summarizes the migration-relevant changes in v0.1.12. See the [v0.1.11 to v0.1.12](#v0111-to-v0112) section below for the v0.1.12 migration guide, the [v0.1.32 to v0.1.33](#v0132-to-v0133) section for the one-shot runtime break, and the [v0.1.36 to v0.1.37](#v0136-to-v0137) section for the latest transition.

### New Subcommands (6)
- `set` -- write a value at a dotted path in TOML/JSON. Use instead of rewriting entire config files.
- `get` -- read a value at a dotted path. Use instead of reading entire config files.
- `del` -- remove a key at a dotted path. `--force-missing` for idempotent scripts.
- `case` -- rename identifiers in 5 case styles via `heck`.
- `query` -- walk a tree-sitter AST. 305 languages at v0.1.12, and the tree now pins the pack 1.22.1 with 372.
- `outline` -- extract top-level definitions. 305 languages at v0.1.12, and the tree now pins the pack 1.22.1 with 372.

All 6 subcommands are additive. No existing code is affected.

### New Flags (15 Total)
- `write --syntax-check` (G72)
- `search --include-fifo` (G56): FIFOs are skipped by default and the flag opens them again; `write` to a FIFO exits 85 `FIFO_DETECTED`
- G90 EXDEV fallback: `--strict-atomic` on a writing verb sets `WriteOptions.strict_atomic`, so the command line reaches exit 91 `EXDEV_FALLBACK_DISABLED` when the target sits on another filesystem
- Advisory write lock (G54), automatic since 0.1.37: there is no lock flag and no lock-wait flag
- `read --format raw` (G81); there is no `--raw` alias
- `read --head N`, `--tail N`, `--line N`, `--grep <REGEX>`
- `search --max-filesize` and `--max-columns` (G68)
- `replace --literal` and `-F` (G66)
- `transform --rules` and `--inline-rules` (G44)
- `batch --batch-size` (G77)
- `backup/copy --no-reflink` (G64)
- `diff --algorithm patience|myers|lcs` (G76)

All flags are additive with default values that preserve v0.1.11 behavior.

### New Error Codes (5)
- 83 `LockTimeout` (G54)
- 88 `SyntaxError` (G72)
- 91 `ExdevFallbackDisabled` (G90)
- 92 `CopyBackBlake3Failed` (G114)
- 93 `OrphanJournal` (G114)

Total error codes: 25 (was 20 in v0.1.4). All new codes have bilingual messages and actionable suggestions.

### Dependencies Added
- `tree-sitter-language-pack = "1.8"` -- 305 languages, dynamic loading, at that version; the tree now pins 1.22.1 with 372
- `toml_edit` -- preserves TOML formatting
- `heck = "0.5"` -- case conversion
- `reflink-copy = "0.1"` -- CoW backup
- `content_inspector = "0.2"` -- UTF-16 detection
- `xattr = "1"` -- extended attributes

All additive. No existing dependency removed.

### Behavior Changes
- None. v0.1.12 is fully backward-compatible with v0.1.11.
- New subcommands are opt-in: existing scripts keep working.
- Default values for new flags preserve v0.1.11 behavior.
- Error code additions do not change existing exit codes.

### Migration Action
- Update version pin: `cargo install atomwrite --locked --version "^0.1.12"`
- New subcommands and flags are opt-in. No code changes required for existing callers.
- See the [work folded into v0.1.37](#work-folded-into-v0137-read-expansion-and-search-ceiling-current) section for the latest migration steps.

### Test Coverage
- 542 tests passing (445 in v0.1.12 + 2 in v0.1.14 + 8 G117 + 6 G118 in v0.1.15 + 40 v0.1.16-v0.1.18 cross-platform + 21 v0.1.19 + 20 v0.1.20)
- 9 ADRs in `docs/decisions/` (0019-0027)
- 7 new JSON schemas in `docs/schemas/`
- See [docs/decisions/README.md](decisions/README.md) for architectural decisions


## v0.1.11 to v0.1.12
### v0.1.12

The v0.1.12 release closes 13 of the Top 20 gaps from the PRD v5-v16 audit (`gaps.md`). It is additive: all v0.1.11 behavior is preserved.

### Added (New Subcommands -- v14 Tier 3)
- `set <PATH> <KEY_PATH> <VALUE>` -- write a value at a dotted path in a TOML or JSON file, preserving comments and key order via `toml_edit`.
- `get <PATH> <KEY_PATH>` -- read a value at a dotted path. NDJSON: `{"type":"get","key_path","value","found","format"}`.
- `del <PATH> <KEY_PATH>` -- remove a key. `--force-missing` flag treats missing keys as a no-op success.
- `case <PATHS...> --subvert OLD NEW --to <style>` -- rename identifiers via `heck`.
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` -- walk a tree-sitter AST.
- `outline <PATH> [--kind <KIND>] [--positions]` -- extract high-level structure.

### Added (G72 REAL syntax check)
- `atomwrite write --syntax-check` invokes tree-sitter (24 languages). Exit 88.

### Added (G114 WAL sidecar)
- `.atomwrite.journal.<target>.atomwrite.journal.json` with `Started`/`Committed`.
- `recover_orphan_journals(dir)` is consultive.

### Added (Other Gaps)
- G54 advisory flock -- exit 83. Automatic since 0.1.37 (5 s wait on the hidden sidecar `.<target>.atomwrite.lock`), no lock flag and no lock-wait flag exists.
- G39 xattr -- macOS quarantine, Linux SELinux, POSIX capabilities preserved.
- G41 content_inspector -- UTF-8, UTF-16LE, UTF-16BE, Binary correctly detected.
- G64 reflink CoW -- `reflink_or_copy` in APFS/btrfs/XFS.
- G90 EXDEV fallback -- copy fallback for Docker/NFS. `--strict-atomic` on a writing verb sets `WriteOptions.strict_atomic` and refuses the fallback with exit 91.
- G44 transform multi-rule -- `--rules <file.yaml>` and `--inline-rules <YAML>`.
- G68 `--max-filesize` and `--max-columns` -- search skip/truncate.
- G80 SIGPIPE -- broken pipe → exit 0 (historical at v0.1.12; since v0.1.37 a closed pipe exits 141).
- G81 `--format raw` -- read emits raw bytes for Unix composability (there is no `--raw` alias).

### Added (5 New Error Codes)
- 83 `LockTimeout`
- 88 `SyntaxError`
- 91 `ExdevFallbackDisabled`
- 92 `CopyBackBlake3Failed`
- 93 `OrphanJournal`

### Migration Action
- No code changes required
- New subcommands are opt-in
- Update version pin: `cargo install atomwrite --locked --version "^0.1.12"`


## v0.1.3 to v0.1.4 (Historical)

### v0.1.4 (Historical)

### Fixed (Windows Compilation - GAP 14)

Three compilation errors in `#[cfg(windows)]` blocks prevented `cargo install atomwrite` from succeeding on Windows 10/11 since v0.1.3:

- `E0433` in `src/atomic.rs:404` — `persist_with_retry` used `AtomwriteError::PermissionDenied` without importing it. The `use crate::error::AtomwriteError;` is now gated under `#[cfg(windows)]` to avoid `unused_imports` on Linux/macOS.
- `E0507` in `src/atomic.rs:387` — `persist_with_retry` took `&NamedTempFile` but called `temp.persist()` which requires ownership. Signature changed to `fn persist_with_retry(mut temp: NamedTempFile, target: &Path) -> Result<()>`. The retry branch now recovers the file from `e.file` (PersistError exposes the original NamedTempFile on failure).
- `E0308` in `src/platform.rs:116` — `GetStdHandle` returns `HANDLE` which is `*mut c_void` in windows-sys 0.61. The literal `0` is a `usize`; comparing a raw pointer to an integer is a type error. Replaced `handle != 0` with `!handle.is_null()`. The `handle != INVALID_HANDLE_VALUE` comparison is unchanged because `INVALID_HANDLE_VALUE` is already typed as `HANDLE` (`-1i32 as _`).

Migration impact:
- No API or behavior change for end users on Linux or macOS
- Windows users: `cargo install atomwrite` now succeeds; no need to apply manual patches or compile from source
- All atomic write semantics, exit codes, NDJSON output, and CLI flags are unchanged

### Fixed (Error Suggestions - GAP 13)

Error suggestions are now context-aware and actionable:

- `WorkspaceJail` suggestion adapts: when the user has supplied `--workspace`, the suggestion now says "use a path inside the workspace (<root>)" instead of re-prompting the flag.
- All 20 error variants now carry `suggestion` text. Previously 6 variants (InvalidInput, Io, ConfigInvalid, FileImmutable, NoMatches, InternalError) returned `None`. Only `BrokenPipe` (SIGPIPE, not actionable) remains without a suggestion.
- Phantom `--force-text` flag reference removed from BinaryFile suggestion.
- New `ErrorContext` struct (`workspace_provided`, `workspace`) and `ErrorJson::from_error_with_context()` API. The legacy `from_error()` is preserved.

New suggestions:
- `FileImmutable` — mentions `chattr -i` (Unix) and `fsutil` (Windows) to clear the immutable attribute
- `NoMatches` — guides the user to broaden the pattern and review `--include`/`--exclude` filters
- `InternalError` — requests a bug report with the reason context
- `InvalidInput` — asks the user to review the input and check arguments
- `Io` — points to the underlying I/O error message
- `ConfigInvalid` — points to the configuration reason

Migration impact:
- No API breakage: `ErrorJson::from_error()` still works with the same output
- If you parse the `suggestion` field of error envelopes, the text may now differ for the affected variants. The semantics (actionable hint) are preserved or improved.

### Added (Cross-Platform Validation - GAP 14)
- `tests/cross_compile_check.rs` — 3 gated cross-compile tests for `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, and `x86_64-pc-windows-msvc`. Fails on any regression of `E0433`, `E0308`, or `E0507` in `cfg(windows)` blocks. Run with `cargo test --test cross_compile_check -- --ignored` before releases that touch Windows-only code.
- `output::write_error_json_with_context()` — propagates `ErrorContext` from the CLI parser to the NDJSON output.
- `docs/INSTALL.md` and `docs/INSTALL.pt-BR.md` — Windows 10/11 installation prerequisites, `cargo install` commands, and troubleshooting.

### Reference

See `gaps.md` sections "GAP 13" and "GAP 14" for the full root-cause analysis and design rationale.


## v0.1.2 to v0.1.3

### v0.1.3 (Previous)

### Changed (BREAKING)

### Atomic write default no longer preserves mtime

The `edit` and `replace` subcommands now update the file modification time (mtime) to the moment the write completes, instead of preserving the original mtime. This is the correct default for build systems that use mtime to detect source changes.

Before v0.1.3:
- `edit` and `replace` hardcoded `AtomicWriteOptions::preserve_timestamps = true`
- The mtime of the file was restored to the value it had BEFORE the atomic rename
- Build systems that compare source mtime to dep-info mtime (cargo, make, cmake) would skip the rebuild silently when the source appeared older than the binary
- Workaround: agents had to run `touch <file>` after `atomwrite edit` to force cargo to detect the change

After v0.1.3:
- `edit` and `replace` use `AtomicWriteOptions::preserve_timestamps = false` by default
- The mtime is set to "now" automatically, so cargo detects the change without manual intervention
- Agents no longer need `touch` after editing a Rust source file before `cargo build`
- For backup, snapshot, or reproducible-build workflows that need the original timestamp, pass the new `--preserve-timestamps` flag

Affected consumers (LLM agents):
- If you build code after editing with atomwrite, the new default fixes a silent "Finished in 0.29s" no-op where cargo skips the rebuild
- If you depend on the old mtime-preservation behavior, add `--preserve-timestamps` to your `edit` and `replace` invocations

Diagnostic field:
- `edit` and `replace` NDJSON output now include `mtime_preserved: bool` so you can verify which path was taken (true = timestamp kept, false = timestamp updated)

### Added (Build System Awareness)
- `--preserve-timestamps` flag on `edit` and `replace` to opt back into the v0.1.2 mtime-preservation behavior
- `mtime_preserved` field in `EditOutput` and `ReplaceResult` NDJSON responses

### Reference

See `gaps.md` section "Atomic Edit Preserva mtime E Quebra Detecção De Mudança Pelo Cargo" (GAP 12) for the full root-cause analysis and design rationale.


## v0.1.1 to v0.1.2

### v0.1.2

### Fixed (Bug Fixes)

### `batch --transaction` rollback is now real
Previously, files created by `write` operations during a transaction were never removed on rollback. Now:
- `RollbackEvent` includes `files_restored`, `files_removed`, and `total_reverted`
- New files created mid-transaction are deleted on rollback
- Pre-existing modified files are restored from backup

Affected consumers (LLM agents): trust the NDJSON `rollback` event — disk state matches it.

### `replace` no longer inflates counters on jail violations
Previously, `total_replacements` was incremented for files outside the workspace jail. Now:
- Jail validation runs BEFORE counter increment
- Violations emit `ReplaceErrorEvent` with `kind: jail_violation`, `error_class: permanent`, `retryable: false`
- `total_replacements` reflects only in-jail matches

### `search` parallel events are now grouped by path
The parallel walker no longer interleaves `begin`/`match`/`end` events from different files. Event sequences for a given path are now contiguous in NDJSON output.

### `scope --delete` Rust comments no longer leaves orphan whitespace
The `comments` prepared query for Rust now matches trailing whitespace so deletion produces clean code.

### `search` invalid regex emits structured JSON envelope
Invalid patterns now fail with `AtomwriteError::InvalidInput` which is serialized as `error.json` on stdout, not raw stderr.

### `batch --file <PATH>` is now functional
The flag now actually reads the NDJSON manifest from a file (validated against workspace jail) instead of being ignored.

### `backup --output-dir <DIR>` is now respected
The flag now places backups in the custom directory (created if missing) and prunes old backups in that directory.

### WORKSPACE_JAIL error message corrected
The misleading "use an absolute path" suggestion is now "set --workspace <root> (XDG/config only; no environment variables)".

### Added (Agent-First Features)
- `--timeout <SECONDS>` global flag for bounded execution (at introduction: 0 = no timeout, default 0; since v0.1.33 default is 120, exit 124 on deadline)
- `read --grep <REGEX>` filter to return only lines matching a regex
- `completions --install` to install completion scripts to XDG data directory

### Changed (Dependencies)
- `nix` 0.29 → 0.31
- `signal-hook` 0.3 → 0.4
- `windows-sys` 0.59 → 0.61
- `rust-i18n` 3 → 4

### Cross-Platform

atomwrite v0.1.2 now compiles on macOS arm64 (Apple Silicon) and macOS x86_64. The `posix_fadvise` syscall is now correctly gated to `target_os = "linux"` only.


## v0.1.0 to v0.1.1
### Summary
- ZERO breaking changes
- All v0.1.0 commands, flags and JSON output remain unchanged
- No migration action required for existing consumers

### Fixed Behaviors (silent failures corrected)
- `search --include` and `search --exclude` now actually filter files (was silently ignored)
- `replace --include` and `replace --exclude` now actually filter files
- `transform --include` and `transform --exclude` now actually filter files
- `search --context N` now emits context lines around matches
- `search --max-count N` now limits matches per file
- `search --invert` now shows non-matching lines (was inverted)
- `search --sort path` now sorts results by file path
- `transform` now processes files in parallel (was sequential)
- `read` modified timestamp now returns ISO 8601 instead of epoch seconds
- `batch delete` backup now uses atomic create_backup() with fsync (was racing the write)
- `create_backup` now uses `fs::copy` instead of `fs::hard_link` (hard links would diverge silently)
- 12 broken intra-doc links in `error.rs` corrected
- Magic exit code numbers replaced with named constants in `constants.rs`
- Six `unwrap()` calls in `edit.rs` multi-edit mode replaced with `ok_or_else`
- `scope.rs` thread join no longer panics on failure

### Additive Changes
### New Subcommands
- `scope` subcommand for grammatical scoping with AST-based actions (delete, upper, lower, titlecase, squeeze, replace)
- `scope` supports Rust (30 prepared queries), Python (13), JavaScript/TypeScript (11), Go (8)
- `backup` subcommand for timestamped backups with BLAKE3 checksums and configurable retention
- `rollback` subcommand for restoring from backups with optional BLAKE3 verification
- `apply` subcommand for patch application with auto-format detection (unified diff, SEARCH/REPLACE, markdown-fenced, full file)

### New Flags
- `batch --transaction` flag for all-or-nothing execution with rollback
- `edit --fuzzy` flag with 7-strategy cascade for approximate text matching
- `edit --multi` flag for multiple NDJSON edits in one atomic write
- `--line-ending lf|crlf|cr|auto` flag on `write` and `edit`
- `--lang <LOCALE>` global flag for locale override (en, pt-BR)
- `batch` move and copy accept `source`, `from`, `src` as field aliases
- `batch` write, delete, edit, hash accept `path` as alias for `target`

### Internationalization
- i18n support via `rust-i18n` with automatic OS locale detection
- All user-facing strings now locale-aware (errors, warnings, info messages)
- Bilingual documentation (en + pt-BR) for all major docs

### Security
- FIFO and device file detection in path validation (exit codes 85 and 86)
- Hardlink detection before atomic rename with `tracing::warn` when nlink > 1
- Same-file detection in `copy` and `move` to prevent source=destination data loss
- SPDX license headers in all 64 `.rs` source files
- `deny.toml` for cargo-deny license and advisory auditing

### Test Infrastructure
- 282 tests (was 5 in v0.1.0)
- Integration tests for `backup`, `rollback`, `apply`, and `scope`
- 2 fuzz targets (`batch_parse`, `extract_json`) with `libfuzzer-sys`
- Optimistic locking integration tests
- NDJSON validation tests expanded from 5 to 20 of 21 commands
- `jaq` interop tests validating NDJSON piped through filter
- i18n integration test

### JSON Output Changes
- `edit` output includes new optional fields: `fuzzy`, `strategy`, `strategies_tried`, `similarity` (only when fuzzy matching used)
- `read` timestamp changed from epoch seconds to ISO 8601 format (breaking for consumers reading `modified` as number)
- New output types added for `scope`, `backup`, `rollback`, `apply`
- All existing fields remain unchanged

### JSON Schema Changes Example

```json
// Before (v0.1.0)
{"type":"read","path":"/abs/file","content":"...","modified":1704067200}

// After (v0.1.1)
{"type":"read","path":"/abs/file","content":"...","modified":"2024-01-01T00:00:00Z"}
```

### Known Limitations Fixed in v0.1.2
- `batch --file <PATH>` flag was declared but not wired (now reads manifest from file)
- `batch --transaction` did not delete files created mid-transaction
- `replace` inflated counters on jail violations
- `search` parallel walker interleaved events from different files
- `search` invalid regex produced raw stderr instead of JSON envelope
- `scope --delete` for Rust comments left orphan whitespace
- macOS compilation failed (nix 0.29 gated `posix_fadvise` to non-macOS Unix)
- `backup --output-dir` was declared but not plumbed through
- No `--timeout`, `--grep`, `completions --install` flags

### Migration Action
- No action required for v0.1.0 to v0.1.1
- Existing `jaq` filters and JSON parsing code continue to work for all fields except `read.modified` (epoch → ISO 8601)
- Update consumers that read `read.modified` as a numeric value
- New fields are additive and safe to ignore
- Recommended: upgrade to v0.1.2 next, which fixes 14 issues introduced in v0.1.1


## Compatibility Notes
### v0.1.15
- G117: `edit` multi-pair gains fuzzy parity, `pair_results`, `failed_pair_index`, and opt-in `--partial` -- envelope fields are additive
- G118: `write` resolves the target against the workspace before append/prepend, line-ending auto-detection, and `--expect-checksum` -- exits 82/126 now fire where a silent overwrite happened
- No new error codes; MSRV stays at Rust 1.88
### v0.1.12
- 6 new subcommands: `set`, `get`, `del`, `case`, `query`, `outline` (v14 Tier 3)
- G72 REAL syntax check via tree-sitter (`atomwrite write --syntax-check`, exit 88)
- G114 WAL sidecar for crash recovery (`.atomwrite.journal.<target>.atomwrite.journal.json`)
- G54 Advisory file lock via `flock` (exit 83 on timeout)
- G39 xattr preservation (macOS quarantine, Linux SELinux, POSIX capabilities)
- G41 content_inspector for UTF-16/BOM/binary detection
- G64 reflink CoW for backup/copy in APFS/btrfs/XFS
- G90 EXDEV fallback for Docker/NFS (exit 91 comes from `--strict-atomic` on the writing verb, which sets `strict_atomic`)
- G44 transform multi-rule YAML (`--rules` / `--inline-rules`)
- G68 `--max-filesize` and `--max-columns` for search
- G80 SIGPIPE handling (broken pipe → exit 0)
- G81 `--format raw` for read (Unix composability)
- 5 new error codes: 83, 88, 91, 92, 93
- All v0.1.11 behavior preserved
- 445 tests (was 320 baseline, +125 new)

### v0.1.3 (Historical)
- BREAKING: `edit` and `replace` no longer preserve the original file mtime by default
- New `--preserve-timestamps` flag on `edit` and `replace` restores the v0.1.2 behavior
- New `mtime_preserved` field in `EditOutput` and `ReplaceResult` NDJSON responses
- All v0.1.2 behavior preserved otherwise (macOS build fix, batch transaction fix, search event grouping, etc)

### v0.1.2 (Previous)
- All v0.1.1 behavior preserved
- 6 critical bug fixes including macOS build, batch transaction, replace counter
- 2 high-priority fixes (batch --file, backup --output-dir)
- 3 agent-first flags (--timeout, --grep, completions --install)
- 4 dependency updates (nix 0.31, signal-hook 0.4, windows-sys 0.61, rust-i18n 4)

### v0.1.1
- All v0.1.0 behavior preserved
- New subcommands and flags are additive only
- Exit codes unchanged from v0.1.0

### v0.1.0
- First public release
- All JSON schemas are defined in `docs/schemas/`
- Use `--json-schema` on any subcommand to introspect at runtime
- Exit codes follow sysexits conventions
- Pre-1.0 releases do not guarantee output stability
- Post-1.0 releases will maintain backward compatibility within major versions


## Rollback Plan
- Keep the previous version binary available before upgrading
- Use `cargo install atomwrite@0.x.y` to pin a specific version
- Verify rollback by running `atomwrite --version`
- Test the new version in a staging environment before production
- Monitor exit codes and NDJSON output for unexpected changes
- Revert to the previous version if agent tests fail
- Revert agent configuration to match the older CLI version


## What Changes
### SemVer Commitment
- atomwrite follows Semantic Versioning 2.0.0
- MAJOR version: breaking changes to CLI flags, exit codes or JSON output schema
- MINOR version: new subcommands, new flags, new JSON fields (additive only)
- PATCH version: bug fixes with no API changes

### What Counts as Breaking
- Removing or renaming a CLI flag
- Changing the meaning of an exit code
- Removing a field from JSON output
- Changing the type of an existing JSON field
- Renaming a JSON field
- Changing the default behavior of an existing flag

### What Does NOT Break
- Adding a new subcommand
- Adding a new optional flag
- Adding a new field to JSON output
- Adding a new exit code
- Improving error messages
- Performance improvements

### Planned Stabilizations for 1.0
- Historical (at that release): NDJSON output schemas for all 41 subcommands
- Exit code assignments
- Error code strings (`FILE_NOT_FOUND`, `STATE_DRIFT`, etc)
- Global flag names and behavior
- Batch manifest format

### Potential Breaking Changes Before 1.0
- Field names in NDJSON output may change before 1.0
- New required fields may be added to output types
- Exit code values may shift to align with sysexits
- The `--json-schema` output format may evolve


## Step-by-Step Migration Template
- Use this template when migrating between versions

### Step 1 -- Read the Changelog
- Review `CHANGELOG.md` for the target version
- Identify all entries marked BREAKING

### Step 2 -- Check Your Commands
- List every atomwrite invocation in your agent or scripts
- Compare each flag against the migration notes

### Step 3 -- Compare JSON Schemas
- Run `atomwrite <subcommand> --json-schema` with both versions
- Identify field additions, removals and type changes

### Step 4 -- Update JSON Parsing
- Update your `jaq` filters or JSON parsing code
- Handle new fields gracefully (additive changes)
- Remove references to deleted fields

### Step 5 -- Update Exit Code Handling
- Review any `case` or `if` blocks that handle exit codes
- Add handling for new exit codes
- Remove handling for deprecated exit codes

### Step 6 -- Test in Dry-Run Mode
- Run every modified invocation with `--dry-run` first
- Verify output structure matches expectations

### Step 7 -- Deploy
- Update the binary via `cargo install atomwrite`
- Run your test suite
- Verify agent behavior in a staging environment


## JSON Schema Changes Template
- Use this format to document field changes between versions

### Before (vX.Y.Z)

```json
{"type":"write","status":"ok","path":"/abs/path","bytes_written":42,"checksum":"abc..."}
```

### After (vX.Y.Z)

```json
{"type":"write","status":"ok","path":"/abs/path","bytes_written":42,"checksum":"abc...","new_field":"value"}
```

### Migration Action
- New field `new_field` is additive and OPTIONAL
- No action required for existing consumers
- Update parsing to consume the new field if useful
