# atomwrite -- Agent Integration Contract


[Leia em Português](AGENTS.pt-BR.md)


## What's New in v0.1.38 (2026-11-04)
- The failure envelope carries `ok`, `code` and `exit`, and `error: true` left it because it duplicated `ok: false` plus `code`, cost 13 bytes on every failure and no caller branched on it alone; a consumer that read it switches to `.ok == false`
- `read --format raw` stops at 24,000 bytes and prints one stderr line with the `--lines` that reads the rest, and `--all` removes the cap
- `read --format raw` on a single target honors `--max-filesize` and exits 65 `FILE_TOO_LARGE` instead of streaming a file above the ceiling
- A picture arrives as one `data:` URI, `--high-res` raises the tier to 2 576 pixels and 4 784 visual tokens, and a picture already sent in this session answers with a short token line unless `--force` is given
- `read doc.pdf` returns the text of every page, `--pages` selects windows, `needs_vision` names a page that carries no text, and `--ocr` recognizes characters on a picture or a rendered page in pure Rust
- `search --format text` writes one `path:line:col:text` per match, and a tree where every file was binary exits `89 ALL_SKIPPED_BINARY` instead of `NO_MATCHES`
- `apply` matches whole lines, applies several files in one invocation, publishes the changed lines with `--show`, and sorts the targets before it takes the locks
- The lock targets the inode with `std::fs::File::try_lock`, and `delete` takes the same lock before it hashes and unlinks, so a deletion and a write to one target no longer interleave
- `delete -r <dir>` owns the whole named tree, including hidden, ignored, linked, fifo, socket and device entries, and a link is never followed
- `target_resolved` is always an array in invocation order, because a path may contain the comma that used to join several targets
- `[defaults].backup = true` reaches `delete` and `batch delete`, and `[delete].backup` wins over it for `delete` only
- `output.profile = "lean"` drops the digests, the byte counts and the timing from a successful mutation record, which took one `apply` on a three-line file from 433 bytes to 196
- A file whose image signature is damaged fails `read` with exit 65 `INVALID_INPUT` instead of returning its bytes as text
- `outline` reads a Markdown heading as a `section` that runs to the end of its own section, and `read --symbol Part` returns that whole section with its sub-sections
- `config` lists 43 keys, and `atomwrite commands --brief` prints one line per subcommand in 5 020 bytes against 67 492 for the full tree
- Pin agents to `^0.1.38`. 47 subcommands (SSOT)


## What's New in v0.1.37 (2026-10-31)
- SSOT surface: 47 subcommands (`CANONICAL_SUBCOMMAND_COUNT` / `atomwrite agent-surface`). The new one is `config`
- `config`: `config path|list|get|set|unset|doctor`; `--scope global|workspace` is required on `path`, `set` and `unset` (there is no default); `list` and `get` show the effective value with its origin (`default`, `global`, `workspace` or `explicit`); 43 keys, all in the Configuration section below
- One terminal event: a failing summary of `search`, `replace`, `transform`, `scope`, `edit-loop`, `semantic-merge` or `recipe` carries `ok:false`, `code`, `exit` and the error detail in the same line; no second error envelope follows. A broken pipe and an already published `hash_verify` verdict are the only exceptions
- `MATCH_FAILED`: an `edit-loop` or multi-pair `edit` pair with no match exits 65 with code `MATCH_FAILED` and reads `best_candidate` like `edit`
- `files_failed` / `errors[]`: `search`, `transform`, `replace`, `scope`, `count`, `list`, `sparse`, `semantic-search` and `case` list the files they could not read or write (each entry has `path`, `code` and `message`, `path` is the marker `<walk>` when the walk failed without naming a file of its own; at most 50, `files_failed` keeps the full count, and `partial: true` accompanies them whenever a file failed); a root named in the invocation that does not exist fails the run with exit 4 even when it matched, `search`, `replace`, `transform`, `scope`, `case`, `sparse`, `semantic-search` and `backup` keep exit 0 when they answered and some file failed, while `count`, `list`, `hash`, `copy`, `delete` and `prune-backups` fail the run over a partial tree, because the answer of those six is a total over the whole tree, `--format text` writes every failure to stderr with its path and reason, and a run with no match and a failure exits with the first failure
- Schemas: one name per verb now serves the whole stream: `recipe`, `codemod`, `batch`, `search`, `replace`, `list`, `backup`, `semantic-search`, `sparse`, `semantic-merge`, `case` and `watch` each answer with a union of every record they write, the terminal event and the error envelope; `recipe` tags reemitted child records with `step` and `codemod` with `rule_id`; the four `*-summary` names are gone; `count` serves the total-count schema; `--print-schema` lists 55 names
- Fuzzy ceiling: a file above `[fuzzy].max_content_bytes` (256 MiB) fails under fuzzy with exit 65 (`INVALID_INPUT`); `--fuzzy off` stays exact-only; the `escape_normalized` strategy is skipped above 32 MiB
- Cancellation: every scanning verb ends its data channel on the `cancelled` envelope, whose `deadline_exceeded` field separates an elapsed `--timeout-secs` budget (exit 124) from a signal (exit 130, 143 or 129 for SIGHUP) without reading `reason`; the record is written unshaped, so `--filter`, `--select` and `--limit` cannot drop it
- Success envelope: every success event carries `ok: true`; verbs with side effects also carry `target_resolved`, always an array even when one path was resolved, and `target_source` (`argv`, or `stdin` when the patch or manifest names it), and `search`, `read`, `count`, `list`, `hash` and `diff` carry both too (`workspace` only for the `.` default of `count` and `list`, which is the ambient root); `suggestion_lang` names the language of the `suggestion`; top-level `null` values are gone from stdout
- Agent-hosted runs: `--non-interactive` (never prompts, never reads a confirmation) and `--confirm-side-effect` join the global flags, with the stream-shape flags `--sort-by`, `--dedupe`, `--aggregate`, `--join`, `--truncate-content`, `--redact`, `--normalize-encoding`, `--diff-against` and `--max-output-bytes`
- BREAKING -- `--max-output-bytes`: it requires `--allow-truncate` (each flag is rejected without the other, exit 2) and it cuts every string value, `checksum` included
- BREAKING -- `apply`: a unified diff is compared with the file (context and removed lines, shift up to 64 lines); a mismatch exits 65 `MATCH_FAILED` with `best_candidate` and writes nothing, where it used to write wrongly with exit 0; an insertion-only hunk inserts AFTER the named line; a SEARCH/REPLACE batch with a missing block fails whole, and a block that matches in two places is `MATCH_AMBIGUOUS`
- BREAKING -- `semantic-merge`: with `--fail-on-conflict` a merge with conflicts exits 84 (`MERGE_CONFLICT`), not 65
- BREAKING -- `STATE_DRIFT`: read-modify-write verbs (`edit`, `edit-loop`, `set`, `del`, `config set`, `apply`, `write --append`) serialize on the automatic advisory lock; `case`, `transform`, `scope` and `codemod` fail a file that changed since they read it with `STATE_DRIFT` (82, retryable), so run them again and never assume the write happened
- BREAKING -- invalid input is 65, not 255: an invalid regex or glob in any verb (`sparse` and `hash -r` included), an AST pattern, a YAML rule, `query -Q`, `read --grep`, `locale --set` or an `edit --multi` operation exits 65 `INVALID_INPUT`; 255 `INTERNAL_ERROR` is a real internal failure only
- BREAKING -- `sparse list`: `--include` and `--exclude` are applied as real globs
- BREAKING -- `--hidden` in `hash`, `copy -r` and `semantic-search`: the three verbs hide dotfiles by default and only visit them under `--hidden`, so a walk that never passed the flag returns the smaller inventory; a path named in the invocation is still read without the flag, and `sparse list` and `sparse outline` take their own `--all` for the same visit
- BREAKING -- fuzzy tie: a missing block that only resembles several lines is `MATCH_FAILED` with `best_candidate`; `MATCH_AMBIGUOUS` needs two or more candidates that pass the dual gate
- Other exit changes: `doctor --strict` with a failed or warned check exits 1 (`DOCTOR_CHECKS_FAILED`); `write` interrupted by a signal or the deadline exits 143, 130 or 124, not 255; the SIGPIPE handler is cooperative and ends with exit 141 after cleaning up. A write that fails because the reader closed exits 141 (`BROKEN_PIPE`) with no panic and no crash report: `completions`, `--print-schema`, `--help` and every verb. Very short output that fits in the pipe buffer (for example `--version | head -1`) can finish before the reader closes and then exits 0
- Build: `watch` is not in the default feature set; the default binary answers `watch` with exit 78 (`CONFIG_INVALID`), and a build with `--features watch` runs it. Tree-sitter parsers are downloaded at run time the first time a language is needed (for example `outline`, `query` and `--syntax-check`); the download respects the proxy settings of the host and needs network access once
- Exit codes to branch on: 65 (read `best_candidate`), 78 (config or missing feature in a slim build), 82 (drift), 83 (lock), 84 (merge conflict), 87 (`edit-loop --partial`), 124 (deadline), 129 (SIGHUP), 141 (broken pipe), 143 (SIGTERM); the full table is in Exit Codes
- Pin agents to `^0.1.37`


## What's New in v0.1.36 (2026-07-31)
- Agent-contract residual (GAP-2026-019…033 + CAPA): content-source jail, `scratch init`, global agent-native stdout shape
- Stdout shape (global): `--select`/`--fields` (CSV keys), `--filter KEY=VAL` (repeatable AND, top-level only), `--limit N` (long-only — no global `-n`; `replace` owns `-n`)
- Always-emit errors: failure envelopes always print NDJSON with `ok:false`; `--filter` never drops them; optional `--select` may thin keys; `--limit` applies to success events only
- Historical (v0.1.36): 46 subcommands (`CANONICAL_SUBCOMMAND_COUNT` / `atomwrite agent-surface`). Do not document 41/42 as current
- `extract`: positional fields or local `--field` (repeatable). NEVER `extract --select` — global `--select` only projects the emitted line (local name collides with clap)
- Pin agents to `^0.1.37`. Config CLI + XDG / `.atomwrite.toml` only. Suite: `cli_v0136_gaps`. ADR-0055


## What's New in v0.1.35 (2026-07-19)
- Residual A-* gaps closed (gaps.md of that release, §20): suite green without `delete --yes`; large overwrite default-deny (`--ack-overwrite`); `watch` always emits `watch_summary`; semantic-merge conflict markers default ON; monólitos `include!` SRP splits
- Write large files: existing targets above XDG `[write].confirm_large_bytes` (default 100 KiB) require `--ack-overwrite` (agent one-shot; no interactive Y/N). `--confirm` is a legacy awareness flag; `--require-large-ack` is independent (no alias collision)
- Delete: use `--plan` or `--dry-run` for plan-only; `delete --confirm` and `delete --yes`/`-y` are rejected (fail-closed). Run again without those flags to delete
- Watch: final NDJSON `type:watch_summary` on every exit path; idle default 500 ms (`--idle-exit-ms` / XDG `[watch].idle_exit_ms`); debounce via CLI or XDG `[watch].debounce_ms`
- Semantic-merge: conflict markers written by default; opt-out `--no-conflict-markers`
- Local DoD: `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`, `cargo check --target x86_64-pc-windows-gnu`, `cargo install --path . --force`
- Contract suite: `cargo test --test cli_e2e_v0135`
- Historical pin `^0.1.35` (superseded by `^0.1.36`). Configuration is CLI + XDG / `.atomwrite.toml` only (no product `ATOMWRITE_*` runtime knobs; no product telemetry)


## What's New in v0.1.34
- Docs-complete publish of the v0.1.33 one-shot runtime (same binary behavior)
- Global `--timeout-secs` (alias `--timeout`) default 120 (was 0); `0` disables; deadline → exit 124
- Fuzzy multi-apply is one-pass L→R on original content (`apply_fuzzy_one_pass`); never re-scans inserted text
- Default max fuzzy applies = 1 if `--max-replacements` omitted; hard ceiling 10_000
- If replacement contains pattern (fixed-string), force single apply (safe section expansion)
- Cooperative cancel polled mid-fuzzy cascade; caps: pattern 64 KiB, lev 8192 chars, max windows 4096, growth max(4×,+16 MiB)
- Fuzzy caps (constants in `src/constants.rs`): `FUZZY_MAX_PATTERN_BYTES` (64 KiB), `FUZZY_MAX_LEVENSHTEIN_CHARS` (8192), `FUZZY_MAX_WINDOWS` (4096), `FUZZY_MAX_BUFFER_GROWTH_FACTOR`/`FUZZY_MAX_BUFFER_GROWTH_BYTES` (max 4× / +16 MiB), hard ceiling `FUZZY_HARD_MAX_REPLACEMENTS` (10_000)
- Regression suite: `tests/cli_v0133_oneshot_fuzzy.rs` — hang < 2s, embeds, timeout
- Historical (at v0.1.33): 41 subcommands; fuzzy still only `auto|aggressive` (`off` rejected exit 65). Superseded in v0.1.35: `off` = exact-only (G-010).
- See [ADR-0054](decisions/0054-v0-1-34-oneshot-fuzzy-timeout.md) for one-shot fuzzy + timeout contract (exit 124 vs 143)


## What Was New in v0.1.30
- BREAKING residual: `--fuzzy off` is rejected (exit 65); only `auto` and `aggressive`. Superseded in v0.1.35: Off = exact-only (G-010).
- Edit success NDJSON includes `match_count` and optional `indent_adjusted`
- `platform.backup_method` is `reflink_or_copy` (never hardlink of live file)
- Recipe hash excludes `*.bak.*`; sparse outline emits real AST items


## What Was New in v0.1.29
- BREAKING: `replace` default is `fuzzy=auto` after zero exact matches; as of 0.1.30 exact-only via `--fuzzy off` is rejected. Superseded in v0.1.35: Off = exact-only (G-010).
- 8 new agent-facing subcommands: `recipe`, `sparse`, `semantic-merge`, `agent-surface`, `watch`, `codemod`, `semantic-search`, `stat`
- Shared fuzzy cascade on `replace` (`--fuzzy`, `--fuzzy-threshold`); match errors may include `best_candidate`
- `write --durability full|fast|auto`; NDJSON `platform.durability`, `platform.rename_method`, `platform.backup_method`
- Linux `renameat2` with rename fallback
- Cargo features: `core` / `ast` / `lang-*` / `watch` / `semantic` / `full` (slim core ~7.7 MiB; PRD 5–8 MiB = core only)
- `agent-surface` is the CLI tool manifesto — integration by subprocess, CLI with NDJSON output
- `recipe` runs real multi-step dispatch (search→replace→hash)
- Cooperative cancel: SIGINT/SIGTERM aborts stdin and chunked work (the `cancelled` envelope with `deadline_exceeded` arrived in v0.1.37, see above)
- `semantic-search` is offline token/index ranking with Jaccard foundation (not OpenRouter embeddings)
- Historical (at v0.1.29): 41 subcommands total, 38 JSON schemas, 683 tests listed


## What's New in v0.1.28
- BREAKING: `delete` now creates a backup by default (was opt-in) — pass `--no-backup` to disable; `--keep-backup` on `delete` is redundant (deletion backups are always preserved and never auto-removed) and now surfaces a `warnings` field in the NDJSON envelope instead of a silent no-op
- BREAKING: `move`/`copy` overwriting an existing destination now require `--force` OR an explicit `--backup`
- Unified `BackupOpts` struct (`--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`) flattened via `#[command(flatten)]` into all 15 mutating subcommands: `write`, `edit`, `edit-loop`, `replace`, `transform`, `scope`, `apply`, `set`, `del`, `case`, `batch`, `delete`, `move`, `copy`, `rollback` (GAP-CLI-SURFACE-DRIFT, ADR-0048)
- `replace --retention` and `delete --no-backup` now parse successfully — previously returned exit 2 `ARGUMENT_PARSE_ERROR` from divergent per-struct flag declarations
- `--backup`/`--no-backup` are now mutually exclusive via `conflicts_with` (exit 2) instead of the last flag silently winning
- `rollback` keeps its pre-rollback safety snapshot opt-in via explicit `--backup` — documented exception to the unified default-true contract
- Zero hardcoded `retention: 5` literals remain; `Default` impls in `config.rs`/`atomic.rs` read `constants::DEFAULT_BACKUP_RETENTION`
- `.atomwrite.toml` `[defaults]` `backup`/`retention` keys are now effective end-to-end across every mutating subcommand (GAP-CONFIG-DEFAULTS-DEAD, ADR-0049) — precedence: CLI flags (`--backup`/`--no-backup`/`--retention`) > `.atomwrite.toml` `[defaults]` > XDG config > built-in default (`true`/`5`) — no product env knobs
- `batch --retention <N>` and `batch --backup` are now effective end-to-end, including the transactional pre-backup step and the `delete` operation inside a batch
- New stdin-tty guard on `edit`'s stdin-consuming modes (`--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between`, `--multi`): fails fast with exit 65 (`INVALID_INPUT`) and an actionable `suggestion` when stdin is a terminal, instead of blocking indefinitely (ADR-0050)
- `edit`'s stdin-consuming modes now reject `--old`/`--new`/`--old-file`/`--new-file` at parse time via `conflicts_with_all` (exit 2) instead of silently dispatching into `edit_by_marker`
- 3 new ADRs: 0048 (unified BackupOpts), 0049 (live config plumbing), 0050 (stdin-tty guard)
- 661 tests passing (0 failures, 3 ignored); new test files: `cli_v0128_backup_matrix.rs` (12), `cli_v0128_config_defaults.rs` (6), `cli_v0128_edit_stdin_guard.rs` (3), `cli_v0128_batch_backup.rs` (6)


## What's New in v0.1.27
- SECURITY FIX (BUG-SEC-001): symlink-directory escape from workspace jail fixed — `canonicalize_existing_prefix` now resolves symlinks in path components before jail check. Affects: write, read, edit, edit-loop, apply, set, del, copy, move. Exit 126 on violation.
- CRITICAL FIX (BUG-SCOPE-004): `scope --query comments --delete` no longer destroys code on lines with inline comments — `expand_to_full_line` now preserves code before the comment
- `get` for missing key returns exit 65 (`INVALID_INPUT`) — reverses v0.1.26 revert to exit 4 (BUG-GET)
- `edit-loop` hardened: empty stdin returns exit 65 (BUG-001), invalid JSON emits NDJSON error (BUG-002), `pair_results` includes `old`/`new` fields (BUG-008)
- `scope` queries `fn`, `struct`, `enum`, `trait`, `const`, `static`, `type-alias`, `mod`, `use`, `async-fn`, `unsafe-fn` now capture `pub` variants (BUG-SCOPE-002/003)
- Go `scope --query var` matches declarations without explicit type (BUG-SCOPE-005)
- `write --syntax-check` suggestion corrected to "remove --syntax-check" (BUG-005)
- Known limitations: `test-fn` (GAP-01), `doc-comment` (GAP-02), JS `export` (GAP-03) — ast-grep multi-node patterns
- 631+ tests passing, 33 subcommands, 29 ADRs


## What's New in v0.1.25
- 49 gaps resolved (GAP-071 through GAP-134) in 6 rounds of end-to-end audit (~505 scenarios)
- NEW SUBCOMMAND: `verify <PATH> <BLAKE3>` (the hash is positional) — dedicated checksum verification (33 subcommands total)
- NEW: `.atomwrite.toml` config file — hierarchy at the time: CLI > env > local `.atomwrite.toml` > XDG `~/.config/atomwrite/config.toml` > defaults (superseded in v0.1.35: there are no environment variables, see the Configuration section)
- NEW FLAGS: `delete --older-than <DURATION>` (s/m/h/d/w), `delete --plan` (plan-only; `--confirm`/`--yes` rejected since v0.1.35), `replace --preserve-case` (UPPER/lower/Title adaptation), `search --pcre2`, `edit --fuzzy-threshold <FLOAT>`, `scope --action symbols|normalize`
- NEW FLAGS: `copy --no-reflink`, `copy --preserve-xattr`, `move --preserve-hardlinks`
- FUZZY MATCHING: Jaro-Winkler (`context_aware_jw` strategy) added to the 9-strategy cascade; `diff_preview` field in responses when fuzzy match used
- CRITICAL FIXES: `write --backup` no longer reports phantom `backup_path` (GAP-101), `set` no longer misroutes key into scalar TOML (GAP-102), `size_delta_pct` overflow fixed (GAP-120)
- HIGH FIXES: `copy --preserve` now preserves permissions AND mtime (GAP-103/133), `copy --backup`/`replace --backup` now retain `.bak` files (GAP-104/105; superseded in v0.1.37 for `replace`, which removes the `.bak` on success unless `--keep-backup`, per ADR-0038), `write --require-backup` enforces retention (GAP-106), I/O errors now emit NDJSON envelope (GAP-098)
- MEDIUM FIXES: `hash` output field renamed `value` → `checksum` (GAP-107), `list --long` ISO 8601 dates (GAP-116), `outline --positions` now emits byte offsets (GAP-109), `list` on nonexistent dir returns exit 4 (GAP-110), `get`/`del` missing key returns exit 65 INVALID_INPUT (GAP-111), `batch move/copy` respects `--force` (GAP-108), `scope --query comments` captures block comments (GAP-123)
- BEHAVIOR CHANGE: `case` without `--subvert` returns exit 65 with helpful message (GAP-127)
- BEHAVIOR CHANGE: `scope --query test-fn` returns actionable error with alternatives (GAP-134)
- Property-based tests for fuzzy matching via proptest (GAP-086)
- Flag conflict validation: `append+prepend`, `fixed+regex`, `literal+regex` rejected at parse time (GAP-093)
- 631 tests passing (10 new), 0 clippy warnings, 0 fmt diffs
- 3 JSON schemas updated (del-result, write-output, read-output)


## What's New in v0.1.24
- 52 bugs fixed (GAP-2026-019 through GAP-2026-070) in comprehensive end-to-end audit
- TYPED ERROR AUDIT: ALL `anyhow::bail!()` in user-facing paths converted to `AtomwriteError` variants. Every error now emits structured JSON on stdout with the correct exit code. No more silent exit 1 without JSON envelope.
- Exit code changes: paths that returned exit 1 now return exit 4 (NotFound) or exit 65 (InvalidInput). Agents MUST update error parsers.
- `delete --recursive` NOW WORKS (was no-op for directories)
- `hash --recursive` NOW WORKS (was accepted but never walked directories)
- `search --multiline` NOW WORKS (flag was not propagated to SearcherBuilder)
- `replace` REJECTS empty pattern (was silently destroying all files)
- `batch --transaction` properly reverts `move`/`copy` on rollback
- `scope`/`count`/`transform`/`diff` now resolve walk roots against `--workspace`
- `get`/`set`/`del` values no longer doubly-quoted (raw value in JSON response)
- Backup timestamp: `YYYYMMDD_HHMMSS_mmm` (millisecond resolution prevents collision)
- `rollback --timestamp` accepts PREFIX match (backward-compatible with old format)
- `prune-backups --max-count` sorts by filename (lexicographic) instead of mtime
- `case --subvert` changed from greedy `num_args=2..` to exact `num_args=2`
- `edit --multi` field `op` now optional (inferred as "exact" when `old`+`new` present)
- `hash --stdin` no longer requires PATHS argument
- `regex` removed `allow_hyphen_values`; use POSIX `--` for hyphen-starting examples
- ARGUMENT_PARSE_ERROR (exit 2) gains context-aware `suggestion` field (ADR-0045)
- 3 new ADRs: 0045 (clap suggestion), 0046 (diff resolve-first), 0047 (scope read-only)
- 621 tests passing (12 new)


## What's New in v0.1.23
- GAP-2026-015 closed — `allow_hyphen_values = true` added to 12 CLI text-accepting fields in 8 structs; the three positionals (`search` and `replace` pattern, `replace` replacement) require `--` since 0.1.37. Values starting with `-` (Markdown bullets `- item`, negative numbers `-5`, YAML entries `- key: val`, diff content `--- a/file`) are now accepted as data, not parsed as flags. Excluded: `CaseArgs.subvert` (incompatible with `num_args = 2..`). See ADR-0041.
- GAP-2026-016 closed — backup is now enabled by default in 9 content-mutating structs: `WriteArgs`, `EditArgs`, `EditLoopArgs`, `ReplaceArgs`, `TransformArgs`, `ApplyArgs`, `SetArgs`, `DelArgs`, `CaseArgs`. Backup is auto-deleted after success (`keep_backup: false` default unchanged). Opt-out: `--no-backup` flag. Precedence: CLI flag > `.atomwrite.toml` / XDG > default (true). No product env knobs. 4 non-content structs unchanged: `DeleteArgs`, `MoveArgs`, `CopyArgs`, `RollbackArgs`. See ADR-0042.
- GAP-2026-017 closed — writes that shrink file size by >50% are now BLOCKED when `--expect-checksum` is active. Returns exit 65 (`INVALID_INPUT`) with suggestion to pass `--allow-shrink`. The `risk_assessment` (L1) becomes blocking (not just informative) when `--expect-checksum` is active and the file shrinks. Without `--expect-checksum`, behavior is unchanged. See ADR-0043.
- GAP-2026-018 closed — new `--old-file <PATH>` and `--new-file <PATH>` flags on `edit`. Content is read from files inside the atomwrite process, bypassing shell expansion and kernel ARG_MAX (~131 KB). `conflicts_with` prevents mixing `--old` with `--old-file` (exit 2). Cross-mixing guard rejects `--old` + `--new-file` and `--old-file` + `--new` (exit 65). `strip_file_trailing_newline()` strips one trailing newline for argv parity. `PairResult.source` reports "arg" or "file". See ADR-0044.
- 609+ tests passing (31 new: 12 hyphen + 7 backup + 4 shrink + 8 old-file)
- 4 new ADRs: 0041, 0042, 0043, 0044


## What's New in v0.1.22
- GAP-2026-012 Front 3 closed — new subcommand `edit-loop [PATH]` applies N pairs `{old, new}` in 1 invocation via NDJSON on stdin. Reduces 5 sequential `edit` calls (5 subprocess spawns, 5 checksum recaptures) to a single atomic write. Supports `--partial`, `--backup`, `--keep-backup`, `--line-ending`, `--preserve-timestamps`, `--fuzzy`, `--expect-checksum`. See `tests/cli_v0121_edit_loop.rs` and ADR-0039.
- GAP-2026-013 Front 2 closed — new subcommand `prune-backups [PATHS]...` provides manual cleanup of legacy `.bak.YYYYMMDD_HHMMSS` files from v0.1.20 and earlier. Flags: `--max-age-secs <SECONDS>`, `--max-count <N>`, `--dry-run` (a switch, off by default; see the prune-backups section). Reuses `cleanup_old_backups_in` from `src/atomic.rs`. See `tests/cli_v0121_prune_backups.rs` and ADR-0040.
- 2 new NDJSON schemas: `edit-loop-output.schema.json` (with `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`) and `prune-backups-output.schema.json` (with `action`, `path`, `reason`, `total`, `elapsed_ms`).
- 32 subcommands total (added `edit-loop` and `prune-backups` to the previous 30).


## What's New in v0.1.21
- GAP-2026-012 closed — new flag `--allow-sequential-drift` on `edit` opts into accepting checksUM drift between sequential edits on the same file, eliminating the false-positive `STATE_DRIFT` (exit 82) when the same agent owns the file across iterations. Default behavior (without flag) is unchanged: re-capture checksum between edits.
- GAP-2026-013 Front 4 closed — `edit` and `rollback` now expose `--backup` and `--retention` flags for parity with `write` and `replace`. Default `backup: false` preserves existing behavior.
- GAP-2026-014 v2 closed — backup default changed to delete after success. Previously, backups accumulated with the `retention: u8` (default 5) policy. Now, `--backup` creates a `.bak.{timestamp}` which is `rm`'d inline on success. New opt-in `--keep-backup` preserves the backup. Failed operations always preserve the backup for inspection. See ADR-0038 and `tests/cli_v0121_backup_keep_flag.rs`.
- New pattern documented: capture checksum at the top of each loop iteration via `atomwrite read --json | jaq -r '.checksum'`, then pass to `--expect-checksum`. Eliminates STATE_DRIFT for sequential edits by the same agent.


## What's New in v0.1.15 (extended in v0.1.18)
- G117: `edit` multi-pair `--old/--new` now runs the same 9-strategy fuzzy cascade as the single path, per pair. Success envelopes gain `pairs_total` and `pair_results[{index, matched, strategy, similarity}]`; failures gain `failed_pair_index` (exit 65, file intact). New opt-in `--partial` applies the pairs that match and reports the unmatched ones.
- G118: `write` resolves the target against `--workspace` BEFORE append/prepend, automatic line ending detection, and `--expect-checksum`. With diverging CWD, checksum drift now returns exit 82 (`STATE_DRIFT`) instead of silently overwriting, and targets outside the jail return exit 126 early.
- 542 tests passing; ADRs 0031-0037 document the closure of seven gaps between v0.1.19-v0.1.20.


## What's New in v0.1.12

This section summarizes the v0.1.12 changes most relevant to AI agents using atomwrite as a tool. All 13 gaps closed in the PRD audit in v0.1.11+v0.1.12 are listed below.

### Subcommands Added (v14 Tier 3)
- `set <PATH> <KEY_PATH> <VALUE>` — writes a value at a dotted path in a TOML or JSON file, preserving comments and key order via `toml_edit`. Use this instead of rewriting the entire config file (saves tokens, preserves formatting).
- `get <PATH> <KEY_PATH>` — reads a value at a dotted path. NDJSON: `{"type":"get","key_path","value","found","format"}`. Use this instead of reading the entire config file.
- `del <PATH> <KEY_PATH>` — removes a key. The `--force-missing` flag treats absent keys as no-op success. Use this for idempotent cleanup scripts.
- `case <PATHS...> --subvert OLD NEW --to <style>` — renames identifiers across multiple files via `heck`. Styles: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`.
- `query <PATH> [--kinds|-Q <PATTERN>|--tree] [--positions]` — walks a tree-sitter AST and emits nodes as NDJSON. 305 languages via `tree-sitter-language-pack`.
- `outline <PATH> [--kind <KIND>] [--positions]` — extracts high-level structure (functions, classes, structs, enums, traits, modules) as NDJSON.

### Flags Added (Critical for Agents)
- `--format raw|ndjson` on `read` — emits raw bytes for Unix composability with `sed`, `awk`, `diff`, `patch`. G81.
- `--syntax-check` on `write` — invokes the tree-sitter parser to validate code. Exit 88 on syntax error. G72.
- `--max-filesize <BYTES>` on `search` — skips files larger than the limit (default 10 MiB). G68.
- `--max-columns <N>` on `search` — truncates matches wider than N columns (default 500). G68.
- `--literal` (alias `-F`) on `replace` — disables regex interpretation. G66.
- `--rules <file.yaml>` and `--inline-rules <YAML>` on `transform` — multi-rule YAML for cascading refactors. G44.
- `--batch-size <N>` on `batch` — controls memory peak (default 100). G77.
- `--no-reflink` on `backup`/`copy` — disables CoW for filesystems without support. G64.
- `--include-fifo` on `search` — opts back into opening named pipes, which the walker skips by default. G56.
- Cross-device (EXDEV) renames fall back to copy-then-rename. G90. `--strict-atomic` on the writing verb refuses that fallback and exits 91.
- Advisory write lock via `std::fs::File::try_lock`, which wraps `flock` on Unix and `LockFileEx` on Windows. G54. Since 0.1.37 the lock is automatic, takes no flag and waits `lock.acquire_timeout_ms` (default 5000) before exit 83.

### Error Codes Added (5 New)
- 83 `LockTimeout` (G54 advisory lock via flock exceeded)
- 88 `SyntaxError` (G72 `--syntax-check` via tree-sitter parser)
- 91 `ExdevFallbackDisabled` (G90: `--strict-atomic` on a writing verb refuses the Docker/NFS copy fallback; the library option `WriteOptions.strict_atomic` reaches the same path)
- 92 `CopyBackBlake3Failed` (G114 in-place write lost checksum integrity)
- 93 `OrphanJournal` (G114 WAL sidecar left by crash)
- See REQUIRED -- Exit Codes below for the full table: 41 rows over 33 distinct exit numbers.

### Crash Recovery (G114)
- `atomic_write` writes `.atomwrite.journal.<target>.atomwrite.journal.json` with `Started`/`Committed` entries.
- `recover_orphan_journals(dir)` is consultive (no auto-replay, no auto-delete).
- The agent receives `{"type":"wal_recovery","orphan_journals":[...]}` events and decides.

### Gaps Closed (13 of Top 20 from PRD)

G39 xattr, G41 binary detect (content_inspector), G54 advisory lock, G56 FIFO skip, G58 line endings, G64 reflink CoW, G66 --literal, G68 --max-filesize, G72 syntax check, G74 --threads, G76 `diff --algorithm`, G77 --batch-size, G80 SIGPIPE, G81 --format raw, G90 EXDEV fallback, G116 fuzzy match, v14 Tier 3 (set/get/del/case/query/outline).

### Dependencies Added
- `tree-sitter-language-pack = "1.8"` (305 languages; parsers are downloaded at run time on first use, about 19 MiB in the cache, then loaded dynamically)
- `toml_edit` (preserves TOML formatting)
- `heck = "0.5"` (case conversion)
- `reflink-copy = "0.1"` (backup CoW)
- `content_inspector = "0.2"` (UTF-16 detection)
- `xattr = "1"` (extended attributes)

### Test Coverage
- 542 tests passing (461 baseline v0.1.15 + 8 G117 edge cases v0.1.18 + 2 G118 replace pre-validation v0.1.18 + 16 cross-platform/WAL/audit increments v0.1.16-v0.1.18)
- 9 ADRs in `docs/decisions/` (0019-0027)
- 7 new JSON schemas in `docs/schemas/` (set, get, del, case, query, outline, wal-recovery)
- See [docs/decisions/README.md](decisions/README.md) for architectural decisions


## Why atomwrite
- Your agent makes dozens of tool calls to read, write, search, and replace files
- Each call costs tokens, latency, and context window space
- atomwrite replaces all of this with one CLI that handles every file operation
- Every write is atomic: tempfile, fsync, rename, fsync-dir
- Every output is NDJSON: one JSON object per line on stdout
- Every response includes a BLAKE3 checksum
- The checksum in the response eliminates verification reads


## Objectives
- Spend fewer tokens: one NDJSON object per call replaces the shell pipeline, and `commands --brief` answers in 5 020 bytes where the full tree costs 67 492
- Lower the financial cost, because the bill follows the tokens and the round trips, and one call replaces the several shell calls a file task needs
- Reduce errors with an atomic write and a BLAKE3 checksum on every response, so a stale or partial result is caught instead of assumed
- Reduce hallucinations with a `type` discriminator, a closed schema per verb and an exit code that names the failure
- Reduce the time spent on each task, because the read, the search and the write run in one process instead of a chain of shell tools
- Reduce latency with local execution and no network on the file path, measured at medians of 5.6 ms on `read` and 6.7 ms on `write` for small files
- Multiply the speed of operations with `batch`, `edit-loop`, `replace`, `case`, `transform` and `codemod` settling many files in one call
- Cut the lead time between the request and the verified result, because the response already carries the checksum, the `target_resolved` and the next step
- Make the agent faster, more effective and more assertive, since the exit code, the `code` and the `suggestion` say what to do next without a retry guess


## Economy
### Token Economy
- Each subcommand costs ~50-200 output tokens
- A batch of 100 writes costs 1 bash call instead of 100 tool calls
- The checksum in write responses saves one read per write
- A typical refactoring session saves 500+ tool calls

### Context Window
- NDJSON output is compact and structured
- No verbose human formatting to interpret
- Agents consume the output directly without extraction steps


## Sovereignty
- atomwrite is a standalone Rust binary with zero runtime dependencies
- No cloud service, no API key, no network access required
- All operations execute locally with sub-millisecond latency
- The agent controls all aspects of file operations
- No vendor lock-in to any agent framework


## Compatible Agents
- Claude Code (Anthropic)
- Cursor (Anysphere)
- Windsurf (Codeium)
- Aider
- OpenAI Codex CLI
- Any agent that invokes bash commands and interprets JSON


## Quickstart

```bash
cargo install atomwrite --locked
WS="$PWD"
echo "hello" | atomwrite --workspace "$WS" write src/hello.txt
atomwrite --workspace "$WS" read src/hello.txt
atomwrite --workspace "$WS" search 'hello' src/
atomwrite --workspace "$WS" replace 'hello' 'world' src/
atomwrite calc "2 hours + 30 minutes to seconds"
```


## 47 Subcommands (SSOT)
- The list is the manifest of `atomwrite agent-surface --format json`; every entry below has a one-line summary and an example that was run against the 0.1.38 binary
- Examples assume `WS` is the absolute workspace root (`WS="$PWD"`) and the fixtures of the project under test
- `read` -- reads files with metadata, checksum and optional content; `--stat` skips the content, `--head N`, `--tail N`, `--lines START:END`, `--grep <REGEX>` filters the returned lines, `--verify-checksum HASH` exits 81 on a mismatch, `--format raw|ndjson` (`raw` emits the bytes), `--symbol NAME`, `--line N` with `--context N` and `--enclosing`, `--number`, `--no-comments`, `--bytes`, `--all`, `--crop WxH`, `--dpi N`, `--max-tokens N`, `--pages`, `--ocr`, `--high-res` and `--force`
  - Example: `atomwrite --workspace "$WS" read --head 2 src/a.rs`
- `write` -- creates or overwrites files atomically from stdin; `--expect-checksum` (exit 82 on drift), `--durability full|fast|auto`, `--syntax-check` validates with tree-sitter (exit 88), `--allow-shrink` for a rewrite that shrinks more than 50%, `--append`, `--prepend`; large existing targets need `--ack-overwrite` (default-deny above `write.confirm_large_bytes`, exit 65); `--max-size BYTES`, `--wal-policy auto|always|never`, `--no-checksum-when-empty`, `--auto-rotate`, `--risk-threshold PERCENT` and `--require-large-ack` complete the local surface
  - Example: `printf 'x\n' | atomwrite --workspace "$WS" write notes/new.txt`
- `edit` -- edits surgically by line number (`--after-line`, `--before-line`, `--range`, `--delete-range`), text marker (`--after-match`, `--before-match`, `--between`) or exact match (`--old`/`--new`, repeatable pairs, or `--old-file`/`--new-file`); `--fuzzy auto|aggressive|off` where `off` = exact-only (default `auto`), `--fuzzy-threshold`, `--replace-all`, `--multi` for NDJSON multi-edit; success NDJSON carries `match_count`, `indent_adjusted` and `checksum_after`
  - Example: `atomwrite --workspace "$WS" edit src/b.rs --old beta --new gamma`
- `search` -- searches file content in parallel (ripgrep engine); `-C`/`--context N`, `-m`/`--max-count N`, `--invert`, `--sort path`, `-F`/`--fixed`, `-w`/`--word`, `-i`/`--case-insensitive`, `-g`/`--include`, `--exclude`, `-P`/`--pcre2`, `-U`/`--multiline`, `--include-fifo` (opens FIFOs, which are skipped by default); zero matches exit 1 with `NO_MATCHES`; `--regex`, `--smart-case`, `--binary`, `--files`, `--offset N`, `--limit N`, `--format text|ndjson` and `--no-begin-end` complete the surface
  - Example: `atomwrite --workspace "$WS" search alpha src/`
- `replace` -- replaces text across multiple files with atomic writes; `--fuzzy auto|aggressive|off` where `off` = exact-only (default `auto`), `--fuzzy-threshold`, `--progress-every`, `--dry-run`; fuzzy multi-apply is one-pass L→R (`apply_fuzzy_one_pass`, never re-scans inserted text); default max applies 1 if `--max-replacements` (`-n`) is omitted (hard ceiling 10_000); if the replacement contains the pattern (fixed-string), a single apply is forced; match failures may emit `best_candidate`, and the local `--regex` treats the pattern as a regular expression
  - Example: `atomwrite --workspace "$WS" replace --dry-run alpha omega src/`
- `hash` -- computes BLAKE3 checksums (read the `checksum` field); `--verify HASH`, `--stdin`, `-r`/`--recursive`, `--exclude`
  - Example: `atomwrite --workspace "$WS" hash src/a.rs`
- `delete` -- deletes files, copying one only when the call asks (`--backup` or `[defaults].backup = true`); `--plan` for plan-only listing, `-r` for directories, `--older-than`, `-g`/`--include`; `delete --confirm` / `--yes` / `-y` are rejected
  - Example: `atomwrite --workspace "$WS" delete --plan tmp.txt`
- `count` -- counts lines and files by extension (`--by-extension`) or by size (`--by-size --top N`)
  - Example: `atomwrite --workspace "$WS" count --by-extension src/`
- `diff` -- compares two files (`--unified`, `--stat`, `-C N`, `--algorithm myers|patience|lcs`); second path `-` reads stdin
  - Example: `atomwrite --workspace "$WS" diff src/a.rs src/b.rs --unified`
- `move` -- moves or renames files atomically; overwriting a destination needs `--force` or `--backup`
  - Example: `atomwrite --workspace "$WS" move src/tmp.rs src/tmp2.rs`
- `copy` -- copies files with checksum verification; `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr`; overwriting a destination needs `--force` or `--backup`
  - Example: `atomwrite --workspace "$WS" copy src/a.rs src/a_copy.rs`
- `list` -- lists project file structure with metadata; `--long`, `--depth N`, `--count-by-ext`, `--all`, `-g`/`--include`, `--exclude`
  - Example: `atomwrite --workspace "$WS" list --long --depth 1 src/`
- `extract` -- extracts fields from NDJSON on stdin or from text columns; positional fields or the local `--field` (repeatable), `--delimiter`, `--stdin`; the global `--select` projects the output and is never a local flag
  - Example: `printf '{"a":1}\n' | atomwrite --workspace "$WS" extract --field a`
- `extract-artifact` -- extracts bounded signal from PDF, OOXML and image headers (never raw artifact bytes); `--max-sample-rows N` bounds the data rows echoed per spreadsheet sheet (0 = header only) and `--max-text-chars N` bounds the document text
  - Example: `atomwrite --workspace "$WS" extract-artifact img.png`
- `calc` -- evaluates mathematical expressions and unit conversions (fend engine); expressions are English (`2 hours to seconds`), a Portuguese unit name exits 65
  - Example: `atomwrite --workspace "$WS" calc "2 hours to seconds"`
- `regex` -- generates a regex from examples (grex engine); flags such as `--digits`, `--words`, `--spaces`, `--repetitions N`, `--case-insensitive` and `--no-anchors` generalize the pattern
  - Example: `atomwrite --workspace "$WS" regex abc-12 xyz-99 id-7 --digits`
- `transform` -- structural AST search and rewrite (ast-grep, 29 built-in languages measured on the shipped binary); `-p`/`--pattern` with `-r`/`--rewrite`, `-l`/`--language`, `--rules`, `--inline-rules`, `--dry-run`, `--verify-parse`, `--include` and `--exclude`; zero matches exit 1
  - Example: `atomwrite --workspace "$WS" transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- `scope` -- grammatical scope over code categories; `--delete` to remove matches; `--action upper|lower|titlecase|squeeze` for text transformations; `--replace-with "text"` for custom substitution; `--query` for prepared queries (comments, fn, strings, struct, etc); `--pattern` for custom AST patterns; supports Rust (30 queries), Python (13), JS/TS (11), Go (8)
  - Example: `atomwrite --workspace "$WS" scope src/ --language rust --query comments --delete --dry-run`
- `backup` -- creates timestamped backups with BLAKE3 checksums; `--retention` for the retention count, `--output-dir`, `--dry-run` for preview
  - Example: `atomwrite --workspace "$WS" backup src/a.rs --retention 3`
- `rollback` -- restores a file from a backup; `--timestamp` or `--latest` selects the backup, `--verify` validates the checksum, `--dry-run` previews
  - Example: `atomwrite --workspace "$WS" rollback src/a.rs --latest --verify`
- `apply` -- applies patches from stdin with automatic format detection (unified diff, SEARCH/REPLACE blocks, markdown-fenced, full file); `--format` forces the format; a unified diff is compared with the file (shift up to 64 lines) and a mismatch exits 65 `MATCH_FAILED` with nothing written; `--strict-atomic` exits 91 instead of the cross-device copy fallback, and `--no-syntax-check` skips the parse check that refuses a patch which breaks the file
  - Example: `printf 'new body\n' | atomwrite --workspace "$WS" apply --format full notes.txt`
- `batch` -- executes multiple operations from an NDJSON manifest on stdin (write, replace, delete, edit, hash, move, copy); `--transaction` for all-or-nothing, `--dry-run`, `--batch-size`, `--file PATH` and `--input-schema`
  - Example: `printf '{"op":"write","target":"batch.txt","content":"x"}\n' | atomwrite --workspace "$WS" batch --transaction`
  - `--transaction` copies every target before the first mutation, so a `write`, `replace`, `edit`, `copy`, `delete` or `move` inside it publishes that copy in `backup_path` and states in `warnings` that no second copy was made; outside a transaction the operation writes its own copy
- `completions` -- generates shell completions (bash, zsh, fish, elvish, powershell); the script goes to stdout as plain text; `--install` writes it to the platform data directory of the shell (never to a path from argv, and the `[storage].home` setting relocates it while the process environment does not) and emits `{"type":"installed","path":"<completion file>","ok":true,"target_resolved":["<same path>"],"target_source":"argv"}`, where `target_source` is `argv` by the target-evidence contract
  - Example: `atomwrite --workspace "$WS" completions bash`
- `config` -- persistent tuning keys since v0.1.37: `path`, `list`, `get`, `set`, `unset` over 43 keys of the XDG `config.toml` and the workspace `.atomwrite.toml`; `--scope global|workspace` required on `path`, `set` and `unset` (see Configuration)
  - Example: `atomwrite --workspace "$WS" config get fuzzy.threshold`
- `set` -- writes a value at a dotted path in a TOML or JSON file via `toml_edit`, keeping comments; auto-coerces int, bool, float and string
  - Example: `atomwrite --workspace "$WS" set config.toml package.name demo2`
- `get` -- reads a value at a dotted path; NDJSON: `{"type":"get","key_path","value","found","format"}`; an invalid TOML or JSON file exits 65
  - Example: `atomwrite --workspace "$WS" get config.toml package.name`
- `del` -- removes a key; `--force-missing` treats an absent key as a no-op success
  - Example: `atomwrite --workspace "$WS" del config.toml features.x`
- `case` -- renames identifiers in FILES via `heck` (a directory exits 0 and does nothing); `--subvert OLD NEW`, `--to snake|camel|pascal|kebab|screaming-snake`, `--dry-run`
  - Example: `atomwrite --workspace "$WS" case --subvert user_id UserId --to pascal --dry-run src/c.rs`
- `query` -- walks a tree-sitter AST and emits nodes as NDJSON; 372 languages via `tree-sitter-language-pack`; modes: `--kinds`, `-Q`/`--query <PATTERN>`, `--tree`, `--positions`
  - Example: `atomwrite --workspace "$WS" query --kinds src/a.rs`
- `outline` -- extracts the high-level structure (functions, classes, structs, enums, traits, modules) of ONE source file as NDJSON; `--kind` (repeatable), `--positions`
  - Example: `atomwrite --workspace "$WS" outline src/a.rs`
- `wal-stats` -- inspects the WAL journal state for local diagnostics; read-only; scope via `--workspace <DIR>`; report with `total_journals`, `by_state`, `oldest_age_secs`
  - Example: `atomwrite --workspace "$WS" wal-stats`
- `wal-heal` -- removes terminal (committed or aborted) journals older than `--threshold-secs` (default 3600) inside a required PATH; wall-clock budget via `--max-duration-ms` (default 100); a journal left by a crash (`Started`) is never removed
  - Example: `atomwrite --workspace "$WS" wal-heal --threshold-secs 3600 .`
- `edit-loop` -- applies N `{old, new}` pairs in one write, all or nothing, NDJSON on stdin (a pair with no match exits 65); `--partial` writes the matching pairs and exits 87; also `--backup`, `--keep-backup`, `--fuzzy`, `--expect-checksum`
  - Example: `printf '{"old":"gamma","new":"delta"}\n' | atomwrite --workspace "$WS" edit-loop src/b.rs`
- `prune-backups` -- manual cleanup of the `.bak.*` siblings of target FILES; needs `--max-age-secs <SECONDS>` or `--max-count <N>`; `--dry-run` is a switch that is off by default
  - Example: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run src/a.rs`
- `verify` -- verifies a file against an expected BLAKE3 hash (positional `PATH HASH`); exit 0 on a match, exit 81 on a mismatch
  - Example: `atomwrite --workspace "$WS" verify src/a.rs "$(atomwrite --workspace "$WS" hash src/a.rs | jaq -r .checksum)"`
- `recipe` -- lists and runs versioned multi-step pipelines (`search-replace-verify`, `edit-loop-syntax-check`); `recipe run` needs `--path` or `--target`; recursive hash skips `*.bak.*`
  - Example: `atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern alpha --replacement omega --path src --dry-run`
- `sparse` -- budgeted monorepo `list`, `read` and `outline`; `sparse list` takes `--max-files`, `--max-bytes`, `--include`, `--exclude` (real globs, an invalid one exits 65) and `--all` (every entry, hidden included); outline takes `--all` too and emits real AST kinds
  - Example: `atomwrite --workspace "$WS" sparse list src/ --max-files 10 --max-bytes 1048576 --all`
- `semantic-merge` -- three-way line-based merge for multi-agent conflicts (not AST); conflict markers default ON, `--write-conflict-markers true|false` states it and `--no-conflict-markers` opts out; `--fail-on-conflict` exits 84
  - Example: `atomwrite --workspace "$WS" semantic-merge --base base.txt --ours ours.txt --theirs theirs.txt --output merged.txt --fail-on-conflict`
- `agent-surface` -- the clap-derived tool manifest (CLI only; integration by subprocess, CLI with NDJSON output) and the SSOT of the 47 subcommands and of the exit codes
  - Example: `atomwrite --workspace "$WS" agent-surface --format json`
- `watch` -- filesystem events with `--debounce-ms N`, `--checksum` and `--gitignore`; always ends with the NDJSON `type:watch_summary`; needs a build with `--features watch` (the default binary exits 78 `CONFIG_INVALID`)
  - Example: `atomwrite --workspace "$WS" watch src/ --max-events 1 --idle-exit-ms 500`
- `codemod` -- multi-rule AST campaign from a YAML rules file, with a `by_rule_id` summary; run `--dry-run` first
  - Example: `atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/`
- `semantic-search` -- offline token ranking by Jaccard similarity (always compiled); `--k`, `--min-score`, `--index-dir` and `--include-backups`
  - Example: `atomwrite --workspace "$WS" semantic-search "unwrap value" src/ --k 5 --min-score 0.01`
- `stat` -- alias of `read --stat` (metadata only)
  - Example: `atomwrite --workspace "$WS" stat src/a.rs`
- `doctor` -- host readiness diagnostics; reads no environment variable; `--strict` exits 1 with `DOCTOR_CHECKS_FAILED` when a check fails or warns
  - Example: `atomwrite --workspace "$WS" doctor`
- `locale` -- shows, sets (`--set en|pt-BR`) or clears (`--clear`) the UI locale preference (XDG); distinct from the global `--locale`
  - Example: `atomwrite --workspace "$WS" locale`
- `commands` -- emits the full command tree as JSON for discovery; `--include-globals` adds the 33 global flags
  - Example: `atomwrite --workspace "$WS" commands --include-globals`
- `scratch` -- `scratch init <PATH>` creates the staging directory (canonical `.atomwrite/scratch`) inside the workspace for `--old-file`/`--new-file` (NEVER OS `/tmp` outside the workspace; exit 126 `WORKSPACE_JAIL`)
  - Example: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch`


## REQUIRED — Content staging (`--old-file` / `--new-file`)
Stage content files inside the workspace jail. `scratch init` takes the directory as a required positional (no default; without it the exit is 2). Prefer the skill formula:

```bash
WS="$PWD"
atomwrite --workspace "$WS" scratch init .atomwrite/scratch
# then write old/new under $WS/.atomwrite/scratch/
```

NEVER stage `--old-file`/`--new-file` in OS `/tmp` outside `--workspace` (exit 126 `WORKSPACE_JAIL`).


## REQUIRED — Edit hierarchy
- Prefer the most precise level that solves the task
- Order: `search` → `transform` → `scope` → `codemod` → `replace` → `edit` → `write`
- Full-file `write` is the last resort
- Use `transform`/`scope` for AST structure; `replace`/`edit` for text; `write` only when a full rewrite is required
- Intentional smaller full rewrite: `write --allow-shrink`


## REQUIRED -- Output Contract
- stdout: ALWAYS structured NDJSON (one JSON object per line)
- stderr: logs only (tracing format, only with `--verbose`)
- Every success event has a discriminator field `"type"` and `"ok":true`; an error envelope has `"ok":false` plus `"code"` instead, because the old `"error":true` duplicate cost every failure 13 bytes and no caller branched on it alone (see JSON Output Format for the few non-NDJSON outputs)
- Flush after each line
- NEVER interpret stderr as structured data
- ALWAYS interpret stdout line by line as JSON


## REQUIRED -- CRUD Contract
### Create (write)
- Send content via stdin
- Receive path, bytes_written, checksum, platform info
- Backup is on by default and is removed after a successful write; pass `--keep-backup` to preserve the previous version, `--no-backup` to disable it
- Use `--expect-checksum` for optimistic locking (the check runs before the write lock is taken, so two writers can still race between check and write)

### Read (read)
- Receive path, content, lines, bytes, checksum, permissions, modified, kind
- Use `--stat` to skip content (metadata only)
- Use `--lines START:END` for partial reads (1-based inclusive)
- Use `--head N` for first N lines, `--tail N` for last N lines
- Use `--grep <REGEX>` to filter returned lines to those matching the regex
- Binary files are auto-detected and content is omitted

### Update (edit, replace, transform)
- `edit` -- surgical: by line number, text marker, or exact match
- `replace` -- bulk: across multiple files with regex support
- `transform` -- structural: AST-based rewrite across codebases
- All three return checksums before and after modification
- All three support `--dry-run` for preview
- `edit` and `replace` support `--preserve-timestamps` to skip mtime update (default: mtime is updated to reflect the change, so build systems like cargo/make/cmake detect the source change without manual `touch`)
- The NDJSON output of `edit` and `replace` includes the `mtime_preserved: bool` field to verify which path was taken

### Delete (delete)
- Receive path, bytes, checksum_before
- A backup is made by default and kept on success (`--no-backup` disables it); restore with `rollback`
- Use `-r` / `--recursive` for directories
- Use `--plan` for plan-only listing (`--dry-run` also lists without deleting); `--older-than` filters by age


## REQUIRED -- JSON Output Format
- Every line is one JSON object; success events carry `type` and `ok: true`, error envelopes carry `ok: false` plus `code` and `exit`
- A few outputs are not NDJSON events: `completions SHELL` prints the raw shell script, and `read --format raw` prints raw bytes (`extract` prints the picked fields without a `type` key)
- Samples below are real output with `--workspace /ws`; checksums are shortened to `abc...`

### Write Response

```json
{"type":"write","status":"success","path":"sample.txt","bytes_written":12,"checksum":"abc...","elapsed_ms":0,"stdin_bytes_read":12,"wal_policy":"auto","platform":{"fsync":"sync_data","dir_fsync":"sync_all","durability":"fast","rename_method":"renameat2"},"ok":true,"target_resolved":["/ws/sample.txt"],"target_source":"argv"}
```

### Read Response

```json
{"type":"read","path":"/ws/sample.txt","content":"hello world\n","lines":1,"bytes":12,"checksum":"abc...","permissions":"0o600","modified":"2026-10-20T02:01:45Z","kind":"text","binary":false,"mode":"full","ok":true}
```

### Edit Response

```json
{"type":"edit","path":"/ws/sample.txt","edits":1,"mode":"exact","bytes_before":12,"bytes_after":9,"checksum_before":"abc...","checksum_after":"def...","lines_before":1,"lines_after":1,"elapsed_ms":0,"fuzzy":false,"strategy":"exact","strategies_tried":1,"diff_preview":" hello","mtime_preserved":false,"match_count":1,"indent_adjusted":false,"ok":true,"target_resolved":["/ws/sample.txt"],"target_source":"argv"}
```

### Search Match

```json
{"type":"begin","path":"/ws/sample.txt"}
{"type":"match","path":"/ws/sample.txt","line_number":1,"lines":"hi world","byte_offset":0,"submatches":[{"match":"hi","start":0,"end":2}]}
{"type":"end","path":"/ws/sample.txt","stats":{"matches":1,"lines_searched":1}}
{"type":"summary","files_visited":1,"files_matched":1,"total_matches":1,"elapsed_ms":3,"ok":true}
```

### Replace Result

```json
{"type":"replaced","path":"/ws/sample.txt","replacements":1,"bytes_before":9,"bytes_after":10,"checksum_before":"abc...","checksum_after":"def...","elapsed_ms":0,"mtime_preserved":false,"fuzzy":false,"strategy":"exact","strategies_tried":1}
{"type":"summary","files_visited":1,"files_matched":1,"files_modified":1,"files_skipped":0,"total_replacements":1,"elapsed_ms":4,"ok":true,"target_resolved":["/ws/sample.txt"],"target_source":"argv"}
```

### Cut counters in the terminal event
- `records_suppressed` counts the records the writer dropped before the terminal record: `--filter` rejections, a `--select` projection that emptied the record, dedupe duplicates and blocks cut by `--limit`
- `records_suppressed` counts records and never files, so it does not compare with `files_matched`, because one file can contribute several records
- `files_budget_dropped` counts the entries the `--limit` budget dropped, from `cap_collected` and from the writer alike
- `files_budget_dropped` names the unit the verb produces: files for a walking verb, records for a streaming one
- Both numbers are a floor, because workers may stop before producing the rest
- `files_planned` appears in `--dry-run` and `--preview` with the files the run would have changed, while `files_modified` stays zero without a real write
- `truncated` is stamped on the record a `--truncate-content` or `--max-output-bytes` cut, and `truncated_records` is the run total of those cuts
- `matches_suppressed` (search `--max-count`), `replacements_suppressed` (replace `--max-replacements`), `below_min_score` (semantic-search `--min-score`) and `items_omitted` (outline `--limit`) name their own cut, so no verb hides what its flag removed
- `matches_suppressed` is a ceiling on records and not on reading, because proving that a file held more costs at most one match read past `--max-count` in that file
- Every one of them is absent when the value is zero, and `--print-schema` declares all of them, so a validator never sees an undeclared key

### Error Envelope

Search/replace/transform/scope with zero matches (exit 1): the summary itself is the terminal event, with `ok:false` and the error detail on the same line:

```json
{"type":"summary","files_visited":1,"files_matched":0,"total_matches":0,"elapsed_ms":3,"ok":false,"code":"NO_MATCHES","exit":1,"target_resolved":["src"],"target_source":"argv","suggestion":"broaden the search pattern; check --include / --exclude filters; verify the file content","match_count":0}
```

Edit/fuzzy miss (exit 65): inspect the optional `best_candidate` and adjust `--old`:

```json
{"ok":false,"code":"MATCH_FAILED","exit":65,"target_resolved":["src/main.rs"],"target_source":"argv","match_count":0,"suggestion":"inspect best_candidate in the error NDJSON and adjust --old, or lower --fuzzy-threshold","best_candidate":{"text":"fn compute_total(items: &[u32]) -> u32 {","line":1,"column":1,"similarity":0.905,"strategy":"context_aware_jw","diff_preview":"-fn compute_totl(items+fn compute_total(items: &[u32]) -> u32 {"}}
```

- Every error envelope includes `"ok":false`, and a match failure adds `"match_count"` (0 on a miss, the real count for `MATCH_AMBIGUOUS`)
- `"suggestion_lang"` appears only with `-v`, the same flag that restores `"message"`, `"error_class"` and `"retryable"`
- The normalized fuzzy strategies (`line_trimmed`, `whitespace_normalized`, `punctuation_normalized`, `indent_flexible`, `trimmed_boundary`, `escape_normalized`) omit `similarity`; do not read a missing value as 1.0
- The `workspace` field only appears on `WORKSPACE_JAIL` errors and reports the resolved workspace root
- `suggestion` is context-aware: the `WORKSPACE_JAIL` suggestion changes based on whether `--workspace` was provided; if `--old-file`/`--new-file` is outside the jail, stage with `scratch init` under the workspace (do not only "adjust `--workspace`")
- See `docs/schemas/` for the JSON Schema definitions of all output types (`atomwrite --print-schema` lists the 55 names; `error-output.schema.json` describes the envelope and the `workspace` field, and `code` is a free string there)


## REQUIRED -- Exit Codes
- Error envelopes on stdout are always NDJSON with `ok: false`; read `exit` and `code` before treating a line as success
- The table below has 41 rows over 33 distinct exit numbers: 35 error-variant entries plus `SUCCESS`, `ARGUMENT_PARSE_ERROR`, `TIMEOUT`, `SIGHUP`, `SIGINT` and `SIGTERM`; it is generated from `atomwrite agent-surface --format json | jaq '.exit_codes'` and matches `atomwrite --help`
- The 35 variants in `src/error/variants.inc.rs` collapse to 33 entries because `EditPairFailed` reuses `MATCH_FAILED` on 65 and `MissingFlag` reuses `INVALID_INPUT` on 65
- Each row ends with the `error_class` and the `retryable` flag, which `-v` puts on the envelope
- Branch on `exit` first and on `code` second, because several codes share one number (1, 65 and 130)
<!-- exit-codes:begin -->
- `0` SUCCESS: success (class `none`, retryable `false`)
- `1` NO_MATCHES: no matches found (search, replace, transform, scope; not a crash) (class `permanent`, retryable `false`)
- `1` DOCTOR_CHECKS_FAILED: `doctor --strict` found failed or warned checks (the report event carries every check) (class `precondition_failed`, retryable `false`)
- `2` UNSUPPORTED_COMBINATION: flags that cannot be combined were passed together (class `permanent`, retryable `false`)
- `2` ARGUMENT_PARSE_ERROR: command line rejected by the parser (class `permanent`, retryable `false`)
- `4` FILE_NOT_FOUND: file not found (class `permanent`, retryable `false`)
- `4` GLOB_NO_MATCHES: the glob matched no file (class `permanent`, retryable `false`)
- `13` PERMISSION_DENIED: permission denied (class `permanent`, retryable `false`)
- `28` DISK_FULL: disk full (no space left on device) (class `transient`, retryable `true`)
- `30` QUOTA_EXCEEDED: quota exceeded (class `transient`, retryable `true`)
- `65` INVALID_INPUT: invalid input (bad arguments or malformed data, including empty stdin without `--allow-empty-stdin` and a file above `[fuzzy].max_content_bytes` under fuzzy) (class `permanent`, retryable `false`)
- `65` MATCH_FAILED: pattern did not match, or an `edit-loop` pair has no match (read `best_candidate`) (class `permanent`, retryable `false`)
- `65` MATCH_AMBIGUOUS: pattern matched more than once (read `best_candidate`) (class `permanent`, retryable `false`)
- `65` BINARY_FILE: binary content rejected (class `precondition_failed`, retryable `false`)
- `65` FILE_TOO_LARGE: file above the size limit (class `precondition_failed`, retryable `false`)
- `69` GRAMMAR_UNAVAILABLE: the tree-sitter grammar for a language is not on this host (download it once with the network up) (class `precondition_failed`, retryable `false`)
- `73` CROSS_DEVICE: cross-device rename (filesystem boundary) (class `conflict`, retryable `true`)
- `74` IO_ERROR: I/O error (class `transient`, retryable `true`)
- `78` CONFIG_INVALID: configuration invalid, or a verb that needs a feature missing from this build (slim) (class `permanent`, retryable `false`)
- `80` TRANSACTION_ROLLBACK_FAILED: batch transaction rollback failed (backups kept) (class `permanent`, retryable `false`)
- `81` CHECKSUM_VERIFY_FAILED: checksum verify failed (class `precondition_failed`, retryable `false`)
- `82` STATE_DRIFT: state drift (checksum mismatch, optimistic lock failed) (class `conflict`, retryable `true`)
- `83` LOCK_TIMEOUT: lock not acquired within `lock.acquire_timeout_ms` (default 5000) (class `conflict`, retryable `true`)
- `84` MERGE_CONFLICT: `semantic-merge` found conflicts (class `permanent`, retryable `false`)
- `85` FIFO_DETECTED: FIFO detected (named pipe cannot be atomically written) (class `precondition_failed`, retryable `false`)
- `86` DEVICE_FILE: device file detected (block or character device) (class `precondition_failed`, retryable `false`)
- `87` EDIT_LOOP_PARTIAL: `edit-loop --partial` wrote the matching pairs and some pairs missed (class `permanent`, retryable `false`)
- `88` SYNTAX_ERROR_DETECTED: syntax error detected (`--syntax-check` failed) (class `precondition_failed`, retryable `false`)
- `91` EXDEV_FALLBACK_DISABLED: EXDEV fallback disabled (raised when `--strict-atomic` is passed and the target lives on another filesystem than its temp file) (class `precondition_failed`, retryable `false`)
- `92` COPY_BACK_BLAKE3_FAILED: copy-back BLAKE3 verification failed (class `conflict`, retryable `true`)
- `93` ORPHAN_JOURNAL: orphan journal detected (class `precondition_failed`, retryable `false`)
- `124` TIMEOUT: global `--timeout-secs` deadline exceeded (default 120; pass `0` to disable) (class `precondition_failed`, retryable `false`)
- `126` WORKSPACE_JAIL: workspace jail violated (path escapes workspace) (class `precondition_failed`, retryable `false`)
- `127` SYMLINK_BLOCKED: symlink blocked (target outside workspace) (class `precondition_failed`, retryable `false`)
- `128` IMMUTABLE_FILE: file immutable (cannot modify) (class `precondition_failed`, retryable `false`)
- `129` SIGHUP: terminal hangup (SIGHUP) (class `none`, retryable `false`)
- `130` CANCELLED: run cancelled before it finished (status follows the cause) (class `permanent`, retryable `false`)
- `130` SIGINT: interrupted by SIGINT (class `none`, retryable `false`)
- `141` BROKEN_PIPE: broken pipe (SIGPIPE or a `BrokenPipe` write error) (class `permanent`, retryable `false`)
- `143` SIGTERM: terminated by SIGTERM (class `none`, retryable `false`)
- `255` INTERNAL_ERROR: internal error (class `permanent`, retryable `false`)
<!-- exit-codes:end -->
- `IO_ERROR` (74) is `transient` and retryable, except the cases of a directory used as a file, a non-directory used as a directory and an invalid argument, which are `precondition_failed` and not retryable
- `edit-loop` and multi-pair `edit` report a pair with no match as `MATCH_FAILED` (65) with `best_candidate`; `edit-loop --partial` writes the matching pairs and exits 87
- `semantic-merge --fail-on-conflict` exits 84; without the flag a merge with conflicts writes the markers and exits 0 with `status` `conflict`
- A failed run ends in EXACTLY ONE terminal event with `ok: false`; the `code`, `exit` and `suggestion` fields sit inside it, so do not wait for a second error line


## REQUIRED -- Error Handling
- Errors emit JSON on stdout with `ok: false`, `code`, `exit`, `message`, `error_class`, `retryable` and `suggestion`
- Add `-v` to get `message`, `error_class`, `retryable` and `suggestion_lang`, which the default envelope drops to keep every failure small
- Optional fields appear only when they apply: `path`, `suggestion`, `workspace`, `match_count`, `best_candidate`, `candidates`, `similar_paths`, `failed_pair_index`, `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results`, `conflict_count` (conflicting regions of a three-way merge) and `missing_flag` (the flag the command required)
- A success summary of `search`, `replace`, `transform`, `scope`, `case`, `sparse`, `semantic-search` and `backup` that could not read or write some files carries `files_failed` and `errors[]` (at most `summary.errors_max_entries` entries (50 by default) of `path`, `code` and `message`); read `files_failed` before trusting a result; the family contract is ADR-0077 (`docs/decisions/0077-answer-family-and-walk-honesty.md`)
- Top-level `null` values are no longer emitted on stdout (11 fields): a key is either present with a value or absent
- Every success event of a verb with side effects carries `ok: true`, `target_resolved` (the absolute path that ran) and `target_source`, which is `argv` when the invocation line names the file, `stdin` when a patch stream or a manifest names it, and `workspace` for the `.` default of `count` and `list`; confirm the path before trusting the effect
- The `rollback` record of `apply` and of `batch --transaction` always publishes `target_resolved` as an array, even when a single file came back, because the caller cannot know the count before reading the field
- `error_class` values: `permanent`, `transient`, `conflict`, `precondition_failed` (the manifest also uses `none` for `SUCCESS` and the signals)
- `retryable` is true for the `transient` and `conflict` classes only
- `suggestion` follows the UI locale and `suggestion_lang` names the language it was rendered in; `message` and `code` stay English
- The `workspace` field appears only on `WORKSPACE_JAIL` errors and reports the resolved workspace root
- The 35 error variants carry an actionable `suggestion`, except `BrokenPipe` (SIGPIPE), which is not user-actionable
- The `WorkspaceJail` suggestion is context-aware: when `--workspace` is already set it says "use a path inside the workspace (<root>)"; if content-source files (`--old-file`/`--new-file`) are outside the jail, stage them with `scratch init` under the workspace
- The `BinaryFile` suggestion recommends `read --stat` for metadata-only reads
- The `FileImmutable` suggestion mentions `chattr -i` (Unix) and `fsutil` (Windows)
- The `NoMatches` suggestion guides pattern broadening and reviewing `--include`/`--exclude` filters
- `code` is a free string in `docs/schemas/error-output.schema.json`; the table above is the list to branch on

### Context-Aware Suggestion API (v0.1.4)
- New Rust API: `ErrorJson::from_error_with_context(err, &ErrorContext)` accepts workspace provenance
- The `ErrorContext` struct has `workspace_provided: bool` and `workspace: Option<PathBuf>`
- The legacy `ErrorJson::from_error(err)` still works and produces the same output as the new API with default context
- Programmatic consumers can call `from_error_with_context` to influence the suggestion text


## REQUIRED -- Retry Strategy
- Read `retryable` and `error_class` from the envelope under `-v`; the groups below are the rows of the table above
- NEVER retry `permanent` or `precondition_failed` without fixing the cause

### Transient Errors (retryable: true)
- `DISK_FULL` (exit 28) -- wait for space and retry
- `QUOTA_EXCEEDED` (exit 30) -- wait for quota reset and retry
- `IO_ERROR` (exit 74) -- retry with exponential backoff

### Conflict Errors (retryable: true)
- `STATE_DRIFT` (exit 82) -- re-read the file, get the new checksum, retry with updated `--expect-checksum`; `case`, `transform`, `scope` and `codemod` also raise it for a file that changed since they read it, so run them again and never assume the write happened
- `LOCK_TIMEOUT` (exit 83) -- the automatic advisory lock was not acquired within `lock.acquire_timeout_ms` (default 5000); retry with backoff or raise the key
- `COPY_BACK_BLAKE3_FAILED` (exit 92) -- the in-place copy-back lost checksum integrity; re-read the file and retry
- `CROSS_DEVICE` (exit 73) -- atomwrite handles it internally via copy-then-delete

### Permanent Errors (retryable: false)
- `NO_MATCHES` (exit 1) -- not a crash; broaden the pattern or the filters
- `ARGUMENT_PARSE_ERROR` (exit 2) -- fix the command line using `suggestion`
- `FILE_NOT_FOUND` (exit 4) -- verify the path exists before retrying
- `PERMISSION_DENIED` (exit 13) -- do not retry without fixing permissions
- `INVALID_INPUT` (exit 65) -- correct the input and retry; an invalid regex, glob, AST pattern or YAML rule also lands here, never on 255
- `MATCH_FAILED` (exit 65) -- read `best_candidate` and adjust `--old`, or lower `--fuzzy-threshold`
- `MATCH_AMBIGUOUS` (exit 65) -- read `best_candidate` and `match_count`, widen `--old` or use `--replace-all` (edit); never guess
- `CONFIG_INVALID` (exit 78) -- correct the configuration, or install a build with the missing feature (for example `--features watch`), and retry
- `TRANSACTION_ROLLBACK_FAILED` (exit 80) -- the `batch --transaction` rollback failed; the backups were kept, inspect them
- `MERGE_CONFLICT` (exit 84) -- resolve the conflict markers of `semantic-merge`
- `EDIT_LOOP_PARTIAL` (exit 87) -- `edit-loop --partial` wrote the matching pairs; fix or drop the missed ones
- `CANCELLED` (exit 130) -- the run was cancelled before it finished; the `cancelled` event says why
- `BROKEN_PIPE` (exit 141) -- the reader closed the pipe and the output is incomplete; re-check the state
- `INTERNAL_ERROR` (exit 255) -- report as a bug; not user-actionable

### Precondition Failed (retryable: false)
- `DOCTOR_CHECKS_FAILED` (exit 1) -- `doctor --strict` found failed or warned checks; fix them and run again
- `BINARY_FILE` (exit 65) -- use `--stat` mode to read metadata without content
- `FILE_TOO_LARGE` (exit 65) -- increase `--max-filesize` or process a smaller file
- `CHECKSUM_VERIFY_FAILED` (exit 81) -- the hash passed to `verify`, `hash --verify` or `read --verify-checksum` did not match; re-read the file
- `FIFO_DETECTED` (exit 85) -- skip this file or use stdin redirection
- `DEVICE_FILE` (exit 86) -- skip this file or use stdin redirection
- `SYNTAX_ERROR_DETECTED` (exit 88) -- `--syntax-check` rejected the content; fix the syntax
- `EXDEV_FALLBACK_DISABLED` (exit 91) -- `--strict-atomic` refused the cross-device copy fallback; move the target into the same filesystem as the workspace, or drop the flag
- `ORPHAN_JOURNAL` (exit 93) -- an orphan WAL journal was detected; inspect it with `wal-stats` and `wal-heal`
- `TIMEOUT` (exit 124) -- the `--timeout-secs` deadline passed; raise it or narrow the path
- `WORKSPACE_JAIL` (exit 126) -- do not retry with the same outside path; stage content-source files with `scratch init` under the workspace, or set `--workspace` to the allowed root (for example a multi-root host skill under home)
- `SYMLINK_BLOCKED` (exit 127) -- do not retry with symlinks disabled
- `IMMUTABLE_FILE` (exit 128) -- remove the immutability flag first (`chattr -i` on Unix, `fsutil` on Windows)

### Not Errors (class none)
- `SUCCESS` (exit 0), plus `SIGHUP` (129), `SIGINT` (130) and `SIGTERM` (143): the process was stopped by a signal and ended cleanly, with no tempfile, journal or lock left behind


## REQUIRED -- Global Flags
- 33 global flags (`atomwrite --help`, `atomwrite commands --include-globals`); every subcommand accepts them, before or after the verb name, except `--print-schema` (put it BEFORE the command). The hidden `--json` is accepted and ignored (output is always NDJSON)
- `--workspace <PATH>` -- ALWAYS pass, absolute, to restrict operations to the project root (the jail is anchored on this value)
- `--config <PATH>` -- explicit config file; it replaces the global and workspace files for `config get` and `config list`; fails if missing or malformed
- `-v` / `--verbose` -- repeatable (`-v` info, `-vv` debug, `-vvv` trace), tracing on stderr
- `-q` / `--quiet` -- repeatable (`-q` error, `-qq` off)
- `--color <auto|always|never>` -- controls colored output; default `auto`
- `--no-color` -- disables colored output (equivalent to `--color never`)
- `--ready-file <PATH>` -- writes the PID after the signal handlers are installed; test harness only, one-shot agents omit it
- `--no-gitignore` -- does not respect .gitignore rules
- `--hidden` -- includes hidden files and directories
- `--follow-symlinks` -- follows symbolic links
- `--threads <N>` / `-j <N>` (alias `--max-concurrency`) -- omitted or `0` is automatic: all logical CPUs up to a built-in ceiling of 16, capped by RAM; `N` is exactly N workers, also above 16
- `--max-filesize <BYTES>` -- default 1 GiB; a larger file is REJECTED (`read`, `edit`, `hash` and `write` exit 65 `FILE_TOO_LARGE`), and on `write` the same ceiling bounds the stdin payload, so an oversized payload is refused before the pipe is drained; `search` has its own local `--max-filesize` (default 10 MiB) that SKIPS larger files and counts them in `files_skipped`
- `--timeout-secs <SECONDS>` (alias `--timeout`) -- global operation timeout; default 120; `0` disables; deadline → exit 124. Use to bound long searches, batches, and replace operations
- `--no-progress` -- disables NDJSON progress events on long operations
- `--json-schema` -- emits the JSON schema for the subcommand's output and exits
- `--print-schema [NAME]` -- without a name lists the 55 schema names, with a name emits that schema; it must precede the command name; an unknown name exits 65
- `--non-interactive` -- declares agent-hosted execution: never prompts, never reads a confirmation from stdin
- `--confirm-side-effect` -- acknowledges a side effect where a command contract asks for it; accepted on the verbs that change bytes on disk, and rejected with exit 65 `INVALID_INPUT` on every other verb, so the flag never reads as a successful no-op
- `--allow-truncate` -- opt-in required by `--max-output-bytes`; each is rejected without the other (exit 2)
- `--locale <en|pt-BR>` -- overrides the display locale; precedence is `--locale`, the XDG preference (`atomwrite locale`), the OS locale, `en`; any other tag exits 2 and lists `available`; use `--locale` only (no product env knobs)
- `--no-auto-heal` -- skips the startup WAL heal (default: threshold 3600 s, budget 100 ms)
- `--select` / `--fields <KEYS>` -- projects NDJSON top-level keys (CSV)
- `--filter <EXPR>` -- keeps events matching `key=value`, `key==value` or `key!=value` (repeatable AND, top-level only)
- `--limit <N>` -- work budget: the target list is cut before the side effect and the walk stops at the record that reaches the cap, so `--limit 1` touches exactly one target (long-only; no global `-n`; `replace` owns `-n`)
- `--sort-by <KEY>` -- orders the buffered events by a key, `path-line` or `line-path`
- `--dedupe <KEYS>` -- keeps the first event of each duplicate group (CSV of keys)
- `--aggregate <KEY[:OP[:FIELD]]>` -- one summary line per group; `OP` is `count` (default), `sum`, `min` or `max`
- `--join <KEY:SUFFIX>` -- attaches the NDJSON table read from stdin to each event under `SUFFIX`
- `--truncate-content <KEY:LEN>` -- cuts the string values of `KEY` to `LEN` characters
- `--redact` -- replaces secret-shaped values with `[redacted]`
- `--normalize-encoding` -- normalizes every string value to Unicode NFC
- `--diff-against <PATH>` -- replaces the live events with one `stream_diff` line (`added`, `removed`, `unchanged`) against a baseline NDJSON file inside the jail
- `--max-output-bytes <BYTES>` -- caps every string value, `checksum` included, on a UTF-8 boundary; needs `--allow-truncate`; never use both when the full `checksum` or `content` is needed
- Note: error events always emit even under `--filter`; optional `--select`/`--fields` may thin keys; `--limit` applies to success events only


## REQUIRED -- Configuration
- Persistent tuning lives in 43 keys of two TOML files: the global XDG `config.toml` (mode 0600; print its path with `config path --scope global`) and the workspace `.atomwrite.toml` at the workspace root
- Precedence: flags, then the workspace `.atomwrite.toml`, then the global `config.toml` (the two files merge key by key), then the built-in default. There are no product environment variables
- Change a key only with the `config` verb: `config path|list|get|set|unset|doctor`; `set` keeps comments, validates the whole file and writes by tempfile-fsync-rename without backup
- `--scope global|workspace` is required on `path`, `set` and `unset` (no default, exit 2 without it); `list` and `get` take no `--scope`
- `list` and `get` show `value`, `origin` (`default`, `global`, `workspace`, or `explicit` when `--config` is used) and `default`
- The terminal event carries `target_resolved` and `target_source` (`argv`): check that the path is the file of the intended scope
- An unknown key or an out-of-range value exits 65 and writes nothing; a `config.toml` that is not valid TOML exits 78 and is never rewritten
- `log.file`, `log.rotation`, `storage.home` and `locale` are global only (scope `workspace` exits 65)
- `defaults.line_ending`, `defaults.backup`, `defaults.retention` and `defaults.max_filesize` are ordinary `config` keys, read and written like every other key; `backup` tells absent from `false`, so an absent key keeps each verb's own default

| Key | Type | Default | Range or values | Scopes |
|---|---|---|---|---|
| `fuzzy.mode` | string | `auto` | `auto`, `aggressive`, `off` | global, workspace |
| `fuzzy.threshold` | float | `0.7` | 0 to 1 | global, workspace |
| `fuzzy.threshold_aggressive` | float | `0.5` | 0 to 1 | global, workspace |
| `fuzzy.threshold_context` | float | `0.8` | 0 to 1 | global, workspace |
| `fuzzy.threshold_jw` | float | `0.85` | 0 to 1 | global, workspace |
| `fuzzy.soft_floor_delta` | float | `0.05` | 0 to 1 | global, workspace |
| `fuzzy.max_content_bytes` | integer | `268435456` | 1 to 9223372036854775807 | global, workspace |
| `write.confirm_large_bytes` | integer | `102400` | 1 to 9223372036854775807 | global, workspace |
| `log.file` | boolean | `false` | `true`, `false` | global |
| `log.rotation` | string | `daily` | `daily`, `hourly` | global |
| `storage.home` | string | none | a directory | global |
| `locale` | string | none | `en`, `pt-BR` | global |
| `defaults.backup` | boolean | none (true for rewrites, false for delete) | `true`, `false` | global, workspace |
| `delete.backup` | boolean | none (falls back to `defaults.backup`) | `true`, `false` | global, workspace |
| `defaults.line_ending` | string | `auto` | `auto`, `lf`, `cr-lf`, `cr` | global, workspace |
| `defaults.max_filesize` | integer | `1073741824` | integer | global, workspace |
| `defaults.retention` | integer | `5` | integer | global, workspace |
| `search.context` | integer | `0` | integer | global, workspace |
| `search.max_columns` | integer | `500` | 1 or more | global, workspace |
| `search.max_filesize` | integer | `10485760` | 1 or more | global, workspace |
| `search.smart_case` | boolean | `true` | `true`, `false` | global, workspace |
| `search.format` | string | `ndjson` | `ndjson`, `text` | global, workspace |
| `read.max_output_bytes` | integer | `24000` | 1 or more | global, workspace |
| `read.number` | boolean | `false` | `true`, `false` | global, workspace |
| `output.profile` | string | `full` | `full`, `lean` | global, workspace |
| `apply.max_ellipsis_lines` | integer | `200` | 1 or more | global, workspace |
| `watch.debounce_ms` | integer | `200` | integer | global, workspace |
| `watch.idle_exit_ms` | integer | `500` | integer | global, workspace |
| `write.auto_rotate_max_age_secs` | integer | `86400` | 1 or more | global, workspace |
| `write.content_risk_patterns` | list | the 14 built-in patterns | list of strings | global, workspace |
| `write.shrink_block_percent` | integer | `50` | 1 to 99 | global, workspace |
| `concurrency.max_threads` | integer | `16` | 1 to 256 | global, workspace |
| `concurrency.ram_fraction_percent` | integer | `50` | 1 to 100 | global, workspace |
| `search.max_inflight_bytes` | integer | `8388608` | 4194304 to 1073741824 | global, workspace |
| `summary.errors_max_entries` | integer | `50` | 1 to 1000 | global, workspace |
| `lock.acquire_timeout_ms` | integer | `5000` | 100 to 3600000 | global, workspace |
| `retry.base_ms` | integer | `50` | 1 to 60000, not above `retry.max_ms` | global, workspace |
| `retry.max_ms` | integer | `400` | 1 to 60000 | global, workspace |
| `retry.max_elapsed_ms` | integer | `2000` | 1 to 3600000 | global, workspace |
| `extract.max_sample_rows` | integer | `10` | 0 to 100000 | global, workspace |
| `extract.max_text_chars` | integer | `400` | 1 to 10000000 | global, workspace |
| `extract.max_sheets` | integer | `32` | 1 to 10000 | global, workspace |
| `extract.max_slides` | integer | `64` | 1 to 10000 | global, workspace |

```bash
WS="$PWD"
atomwrite --workspace "$WS" config get fuzzy.threshold
atomwrite --workspace "$WS" config set fuzzy.threshold 0.9 --scope workspace
atomwrite --workspace "$WS" config unset fuzzy.threshold --scope workspace
```


## PROHIBITED -- Common Pitfalls
- NEVER interpret stderr as data; it only contains tracing logs
- NEVER assume exit code 1 is a fatal error; it means zero matches in search, replace, transform or scope (only `doctor --strict` uses exit 1 for failed checks, with code `DOCTOR_CHECKS_FAILED`)
- NEVER omit `--workspace` when running as an agent
- NEVER omit `--dry-run` before destructive batch operations
- NEVER use unquoted expressions with `calc`; the shell will interpolate them
- NEVER ignore `checksum_before` and `checksum_after` in edit/replace responses
- NEVER retry `permanent` or `precondition_failed` errors without fixing the cause
- NEVER pass `delete --confirm` / `--yes` / `-y` (rejected fail-closed; use `--plan` or omit flags)
- NEVER write large existing targets without `--ack-overwrite` (default-deny above XDG `[write].confirm_large_bytes`)
- NEVER assume empty stdout on `watch` idle — always ends with `type:watch_summary`
- NEVER invent product `ATOMWRITE_*` runtime knobs (CLI + XDG / `.atomwrite.toml` only in `^0.1.38`)
- NEVER stage `--old-file`/`--new-file` in OS `/tmp` outside `--workspace` (use `scratch`)
- NEVER invent `edit --strategy` (use `--fuzzy auto|aggressive|off` only)
- NEVER pass unquoted shell `$VAR` in bodies when literal `$` must survive (use `<<'EOF'` or file staging)
- NEVER use fuzzy edit/replace for structural symbol rename (use transform/scope/codemod)
- NEVER run `--fuzzy aggressive` tree-wide without `--dry-run` first
- ALWAYS set `--workspace` to the allowed root when editing multi-root paths (e.g. host skills under home)
- Prefer file-based patches for content with `//`, jq, quotes, multiline
- Intentional smaller full rewrite: `write --allow-shrink`
- Prefer `--fuzzy auto` (default cascade for agents); use `--fuzzy off` only for intentional exact-only (G-010)


## REQUIRED -- Token Budget
- Each subcommand: 1 bash call, ~50-200 output tokens
- Batch mode: 1 bash call for N operations
- Checksum in response eliminates 1 verification read per write
- A typical agent session saves 500+ calls versus individual operations


## REQUIRED -- Optimistic Locking
- Read a file and capture its `checksum` from the response
- Pass the checksum via `--expect-checksum` on the next write or edit
- If the file changed between read and write, atomwrite returns exit 82 (`STATE_DRIFT`)
- Re-read the file to get the current checksum and try again
- This prevents lost updates in concurrent agent workflows


## Backup Operations v0.1.21
- By default, backups are DELETED after the operation completes successfully
- Use `--keep-backup` to preserve the backup after success
- Backups from FAILED operations are always preserved for inspection
- `delete` keeps its `.bak.<timestamp>` after a successful run, while `replace` deletes it like the other mutators
- `cleanup_old_backups_in` keeps N backups only for `--keep-backup` cases
- New subcommand `prune-backups` for manual cleanup of legacy backups


## Sequential Pattern v0.1.21
- 5 sequential edits WITHOUT re-capture = 4 fail with `STATE_DRIFT` (exit 82)
- 5 sequential edits WITH re-capture = all pass (canonical pattern)
- 5 sequential edits WITH `--allow-sequential-drift` = all pass with warning
- `edit-loop` applies N pairs in 1 invocation (no internal STATE_DRIFT)


## Subcommands v0.1.22

Two new subcommands close the rejected fronts from previous plans (`gaps.md` of that release, lines 82-83 and 201):

### `edit-loop` — N Pairs in 1 Invocation
- When to use: applying a batch of textual transformations to one file where today you would invoke `edit` N times in a shell loop
- Input: NDJSON via stdin with one `{old, new}` object per line
- Behavior: reads the file ONCE, applies all pairs in memory, writes atomically ONCE
- Advantage over shell loop: 1 CLI invocation instead of N; 1 read + 1 write instead of N+N; no internal `STATE_DRIFT` between pairs

```bash
WS="$PWD"

# Apply 3 pairs in 1 invocation (all or nothing: a pair with no match exits 65 and writes nothing)
printf '%s\n' \
  '{"old":"v0_1_20","new":"v0_1_22"}' \
  '{"old":"foo","new":"bar"}' \
  '{"old":"baz","new":"qux"}' \
  | atomwrite --workspace "$WS" edit-loop src/version.rs

# Keep the backup (forensic timeline)
printf '%s\n' '{"old":"foo","new":"bar"}' \
  | atomwrite --workspace "$WS" edit-loop --backup --keep-backup src/foo.rs

# Best-effort with --partial: writes the pairs that match, exits 87 (EDIT_LOOP_PARTIAL) if any missed
printf '%s\n' '{"old":"exists","new":"X"}' '{"old":"missing","new":"Y"}' \
  | atomwrite --workspace "$WS" edit-loop --partial src/foo.rs
```

- NDJSON Schema: `docs/schemas/edit-loop-output.schema.json` -- fields `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`
- ADR: `docs/decisions/0039-edit-loop-helper.md`

### `prune-backups` — Manual Cleanup of Legacy Files
- When to use: operators who upgraded from v0.1.20 inherit `.bak.YYYYMMDD_HHMMSS` siblings that v0.1.21 no longer creates (and therefore no longer cleans up automatically)
- Algorithm: `PATHS...` are the target FILES, not directories; for each file it lists the siblings named `<file>.bak.*` and applies the age or count filter (age first, then count on the survivors)
- Required filter: pass `--max-age-secs` or `--max-count`; without one the run exits 2
- Safety: `--dry-run` is a switch with no value and it is OFF by default, so a run without it deletes; preview first with `--dry-run`, and never write `--dry-run false` (the word `false` is read as one more path)

```bash
WS="$PWD"

# Preview: list the backups of src/a.rs older than 24 hours (nothing is removed)
atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run src/a.rs

# Remove them: omit --dry-run to apply
atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 src/a.rs

# Keep only the 3 most recent backups of src/a.rs
atomwrite --workspace "$WS" prune-backups --max-count 3 src/a.rs

# Assert that no backup of the file is left after the cleanup
atomwrite --workspace "$WS" prune-backups --max-age-secs 0 src/a.rs \
  && fd -c never -g 'a.rs.bak.*' src | wc -l | jaq -e '. == 0'
```

- NDJSON Schema: `docs/schemas/prune-backups-output.schema.json` -- fields `path`, `reason`, `action`, `total`, `elapsed_ms`
- ADR: `docs/decisions/0040-prune-backups-subcommand.md`
- Operational note: automatic retroactive cleanup was explicitly rejected (see ADR-0038 Addendum); operators must run `prune-backups` on demand


## Known Limitations (v0.1.38)
- `--expect-checksum` is checked before the write lock is taken, so a TOCTOU window remains between the check and the write
- A target name above 255 bytes turns the stem of the lock sidecar into a 16 character digest of that name, so the sidecar stays inside the filesystem limit instead of failing with `ENAMETOOLONG`
- An in-place write in a directory that forbids creating the sidecar still proceeds on the inode lock, and the run is refused only when the target cannot be opened for locking and the sidecar cannot be created either
- The write lock is automatic on Unix and on Windows, and the Windows lock was not run for this release
- The RAM cap of the automatic thread count exists on Linux only and does not read cgroup limits
- The global `--limit` makes the walk quit at the record that reaches the cap, and the terminal summary carries `walker_stopped_early: true` only when the walker itself quit, so a total below the tree size is never read as the tree size
- `files_skipped` counts both the files above `--max-filesize` and the binary files the default run refuses to search, so a file missing from the answer is named in one field instead of vanishing
- With `--sort-by` and the other buffered shape flags the failure event enters the buffer, and `--filter`, `--select` and `--limit` never drop it
- `transform` in multi-rule mode ends with one summary of the whole invocation; a rule that fails adds a `rule_error` and the aggregate stays the single terminal record
- The `wal-heal` verb never removes a `Started` journal left by a killed process (`kill -9`), and the lock sidecar of that process stays on disk
- `count` fails closed on the first read failure and still publishes `files_failed`, `errors[]` and the counts of the files it could read, with `ok:false`, `code:"PERMISSION_DENIED"` and `exit:13`
- The verbs that answer from part of the tree (`search`, `replace`, `transform`, `scope`, `sparse`, `semantic-search` and `case`) keep `ok:true` and name the files they could not read, and ADR-0077 fixes the two families
- The Rust panic reporter (`human-panic`) writes its report as a local file in the temporary directory and never transmits it


## REQUIRED -- Rule Zero and Gates
- Rule Zero: run a command against the real binary before you say it works, and report a failure with its exit code and `code`, never with "should work"
- Precedence of instructions: the owner's message in the current turn wins, then an `AGENTS.override.md` next to the file you edit, then the nearest nested `AGENTS.md`, then this file
- Edit project files with atomwrite itself (`edit`, `write`, `replace`), stage patches under `$WS/.atomwrite/scratch/`, and pass `--workspace "$WS"` on every file operation
- Every gate runs on the local machine: the project has no remote automation, no CI and no workflow files
- Wrap each long command in `timeout <SECONDS>` with a positive integer, so a hung build or test cannot block the session
- NEVER commit, tag or publish unless the owner orders it in the current message

```bash
# Build
timeout 900 cargo build --release --locked
timeout 1800 cargo build --release --features full

# Format and lint
cargo fmt --check
timeout 900 cargo clippy --locked --all-targets --all-features -- -D warnings

# Docs
RUSTDOCFLAGS="-D warnings" timeout 600 cargo doc --no-deps --locked

# Test
timeout 1800 cargo test --locked
timeout 600 cargo test --test docs_consistency --test docs_exit_codes

# Features: proves the slim core and core+watch builds compile
timeout 900 cargo xtask check-features

# Audit and deny: before the version bump, not after the docs are written
timeout 300 cargo audit
timeout 300 cargo deny check

# Package: list the files that would ship (docs/AGENTS.md must be in the list)
timeout 600 cargo package --list --locked

# Install the build you just tested
cargo install --path . --force
```

- Build: the default `cargo build --release --locked` is the gate; the `full` profile adds every language and `watch` (needs `--features watch` for the `watch` verb)
- Test: `cargo test --no-default-features --features core` compiles but is NOT green (44 tests in 11 targets need the AST features), so use `cargo xtask check-features` for the slim profile
- Coverage: there is no coverage gate; the integration suite under `tests/` is the metric (`docs/TESTING.md`)
- Package: the tarball must contain `docs/AGENTS.md` and `docs/AGENTS.pt-BR.md`; check the list before any release
- Publish: a human decision; an agent never runs `cargo publish`


## REQUIRED -- Paths by Platform (XDG)
- Every path below hangs off `ProjectDirs::from("", "", "atomwrite")` of the `directories` crate (`src/storage.rs`); the key `storage.home` relocates them to `{home}/config`, `{home}/data`, `{home}/cache` and `{home}/state`, while the bootstrap `config.toml` that holds `storage.home` stays at the platform config path
- Ask the binary instead of guessing: `atomwrite config path --scope global`, `atomwrite config path --scope workspace` and `atomwrite locale` (field `preference_path`) print the real files

| Item | Linux (measured) | macOS (not validated) | Windows (compiles under `cargo check`, never run) |
|---|---|---|---|
| Global `config.toml` | XDG config dir, `atomwrite/config.toml` (`~/.config/atomwrite/config.toml`) | `~/Library/Application Support/atomwrite/config.toml` | `%APPDATA%\atomwrite\config\config.toml` |
| Workspace file | `<workspace>/.atomwrite.toml` | same | same |
| Locale preference | `atomwrite/locale` in the config dir | same dir | same dir |
| Log file (`log.file = true`) | XDG state dir, `atomwrite/logs/atomwrite.<date>.jsonl` | falls back to the data dir | falls back to the data dir |
| `completions bash --install` | XDG data dir, `bash-completion/completions/atomwrite` | not validated | not validated |
| Tree-sitter parsers | XDG cache dir, `tree-sitter-language-pack/v1.22.1/` (bundle of 23,031,807 bytes and about 4 MiB of extracted libraries, downloaded on first use) | cache dir | cache dir |

- Linux was run in a bench whose HOME and XDG directories sat inside a temporary directory; `config path --scope global` printed `<config dir>/atomwrite/config.toml` with `exists: false` before the first `config set`, and `config path --scope workspace` printed `<workspace>/.atomwrite.toml`
- The global file is created with mode 0600 on Unix; the log file is 0600 in a 0700 directory
- macOS and Windows entries were read from the code and the crate convention, not run; confirm them with `config path --scope global` on the host


## REQUIRED -- Locale Precedence
- Order: `--locale` flag, then the persisted preference (`atomwrite locale --set en|pt-BR`, stored in the file `locale` of the config dir), then the OS locale (`sys-locale`), then `en`
- Measured with the OS locale `pt-BR`: `atomwrite locale` resolved `pt-BR` with `source: system`; after `locale --set en` it resolved `en` with `source: persisted`; with `--locale pt-BR` it resolved `pt-BR` with `source: cli` while `persisted` stayed `en`; after `locale --clear` it went back to `pt-BR` and `system`
- `locale --set` and `locale --clear` report the state of the CURRENT run and a `note` says the change applies from the next invocation; read `locale` again to see the new state
- Any tag other than `en` and `pt-BR` exits 2 and lists `available`; a locale the OS reports but atomwrite does not ship resolves to `en`
- Only the `suggestion` text is translated; `code`, `message` and every JSON key stay English
