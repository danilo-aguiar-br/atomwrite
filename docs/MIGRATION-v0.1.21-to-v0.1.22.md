# Migration Guide: v0.1.21 to v0.1.22

- **Target audience**: operators and CI scripts that call `atomwrite` from shell, Make, or other automation layers
- **Scope**: 2 new subcommands (`edit-loop`, `prune-backups`) and 2 new ADRs (0039, 0040)
- **Reading time**: 5 minutes
- **Action required**: None for existing scripts; optional adoption of new subcommands for sequential-edit and backup-cleanup use cases

## What Is New

This release adds 2 new subcommands to v0.1.22. Both are additive: existing scripts that do not invoke the new subcommands continue to behave exactly as they did under v0.1.21.

### `edit-loop` — N pairs in one invocation

- **Use case**: applying a batch of textual transformations to one file (rename an identifier in 7 places, sweep obsolete type aliases, refactor import paths) where today you invoke `edit` N times in a shell loop.
- **Before (v0.1.21)**: shell loop with N `edit` calls; each invocation pays the full subprocess startup cost (clap parse, BLAKE3 read, `--expect-checksum` re-validation, write pipeline, NDJSON envelope emission).
- **After (v0.1.22)**: 1 `edit-loop` invocation reads the file once, applies all N pairs in memory, and writes once.

```bash
# v0.1.21 — 5 edit calls, 5 process spawns, 5 checksum re-reads
for pair in "$@"; do
  CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
  echo "${pair#*=}" | atomwrite --workspace . edit \
    --expect-checksum "$CS" \
    --old "${pair%%=*}" --new "${pair#*=}" src/foo.rs
done

# v0.1.22 — 1 invocation, 1 read, 1 write
printf '%s\n' \
  '{"old":"foo","new":"bar"}' \
  '{"old":"baz","new":"qux"}' \
  | atomwrite --workspace . edit-loop src/foo.rs
```

- **NDJSON input format**: one `{"old":"...","new":"..."}` per line
- **NDJSON output**: one `pair_result` line per input line (`matched: true|false`) plus a `summary` line with `pairs_total`, `pairs_matched`, `pairs_unmatched`
- **Flags**: `--workspace`, `--expect-checksum`, `--partial`, `--fuzzy`, `--line-ending`, `--preserve-timestamps`, `--backup`, `--keep-backup`, `--retention`
- **Exit codes**: 0 if all matched (or `--partial` with ≥1 matched), 1 if zero matched (NO_MATCHES), 65 if any precondition failed

### `prune-backups` — manual cleanup of legacy backup rings

- **Use case**: operators who upgraded from v0.1.20 inherit `.bak.<timestamp>` siblings from every `--backup` write issued under v0.1.20 (the audit classified these as transient trash that the operator can clean up at leisure).
- **Before (v0.1.21)**: no command to clean up legacy `.bak.*` files. The v0.1.21 lifecycle handles only post-success deletion of the just-created backup (ADR-0038).
- **After (v0.1.22)**: explicit `prune-backups` subcommand with `--dry-run true` default for safety.

```bash
# v0.1.22 — list what would be removed (safe; default dry-run true)
atomwrite --workspace . prune-backups --max-age 86400 .

# v0.1.22 — actually remove backups older than 24 hours
atomwrite --workspace . prune-backups --max-age 86400 --dry-run false .

# v0.1.22 — keep only the 3 most recent backups per directory
atomwrite --workspace . prune-backups --max-count 3 --dry-run false .
```

- **Flags**: `--max-age <SECONDS>` (delete backups older than N; default 0 = all), `--max-count <N>` (keep at most N most-recent per directory; default 0 = unlimited), `--dry-run` (default `true`; pass `false` to actually delete)
- **NDJSON output**: one line per inspected backup (`path`, `age_secs`, `size_bytes`, `action: deleted|kept|would_delete`) plus a `summary` line with `scanned`, `deleted`, `kept`, `elapsed_ms`, `dry_run`
- **Exit codes**: 0 if scan completed, 1 if no backups found (NO_MATCHES), 65 if precondition failed
- **Safety**: refusing to run without `--max-age` or `--max-count` (bails with `InvalidInput`); `--dry-run` default `true` makes the command safe to paste from a chat thread

## Non-Breaking Notes

- All 32 subcommands from v0.1.21 remain available with identical signatures and behavior.
- All flag semantics are preserved. No flag was renamed, removed, or had its default changed.
- NDJSON envelope schemas for existing subcommands are unchanged. The two new schemas (`edit-loop-output.schema.json`, `prune-backups-output.schema.json`) are additive.
- `keep_backup: false` default behavior from v0.1.21 is preserved (backups deleted after success unless `--keep-backup` is passed).
- `--allow-sequential-drift` opt-in from v0.1.21 remains the recommended pattern for sequential edits that prefer shell loops over `edit-loop`.

## Field Reference

- `EditLoopArgs { path: PathBuf }` — new struct in `src/cli_args.rs`
- `PruneBackupsArgs { paths: Vec<PathBuf>, max_age: Option<u32>, max_count: Option<u8>, dry_run: bool }` — new struct in `src/cli_args.rs`
- `EditLoopSummary` — new struct in `src/ndjson_types.rs`
- `PruneBackupSummary` — new struct in `src/ndjson_types.rs`

## Verification

After upgrading, run the smoke test in a scratch directory to confirm the new subcommands work as expected:

```bash
# Setup — create a target file
echo "hello world" > /tmp/v0122-test.txt

# v0.1.22 — edit-loop applies 2 pairs in 1 invocation
printf '%s\n' '{"old":"hello","new":"Olá"}' '{"old":"world","new":"Rust"}' \
  | atomwrite --workspace /tmp edit-loop /tmp/v0122-test.txt

# Expect "Olá Rust" in the file
cat /tmp/v0122-test.txt

# v0.1.22 — prune-backups with default dry-run
echo "original" > /tmp/v0122-prune.txt
echo "new" | atomwrite --workspace /tmp write --backup --keep-backup /tmp/v0122-prune.txt
atomwrite --workspace /tmp prune-backups --max-age 0 /tmp
# Expect: summary line shows dry_run=true, action="would_delete", no actual deletion

# v0.1.22 — prune-backups with --dry-run false to actually delete
atomwrite --workspace /tmp prune-backups --max-age 0 --dry-run false /tmp
fd '*.bak.*' /tmp | wc -l
# Expect: 0
```

## See Also

- `docs/decisions/0039-edit-loop-helper.md` — full ADR rationale, alternatives considered, and trigger to revisit
- `docs/decisions/0040-prune-backups-subcommand.md` — full ADR rationale, alternatives considered, and trigger to revisit
- `CHANGELOG.md` — v0.1.22 release notes
- `skill/atomwrite-en/SKILL.md` and `skill/atomwrite-pt/SKILL.md` — Padrão Correto sections for sequential edits with edit-loop and backup cleanup
- `docs/HOW_TO_USE.md` — usage examples for both subcommands
