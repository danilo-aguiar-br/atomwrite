# ADR-0062 — WAL rate limit at journal creation and a bounded `startup_wal_heal`

## Status
Accepted (2026-10-15)

## Context
- The H3 rate limit (`h3_rate_limit`) was counted in the `Drop` of the journal. After 10 journals in 60 s every later journal was preserved after its commit: a `replace foo bar` over 120 files in one directory left 110 residual `.atomwrite.journal.*` files (gaps.md W1).
- `startup_wal_heal` walked the whole workspace on every run, `target/` and `.git` included, ignored `--threads` and checked its 100 ms budget only after the walk (gaps.md W9). A `read` of one file in a workspace whose `target/` has 40203 entries took 73 to 90 MB.
- A cooperative SIGTERM or SIGINT in the middle of `replace` left a `started` journal in 16 of 100 runs, although the target was intact (gaps.md W46).
- `inplace.rs` removed every `.atomwrite-*` file of the directory, so it could delete the tempfile of another process (gaps.md W3).

## Decision
- H3 left `heuristics_should_preserve`. It applies only at journal creation (`creation_throttled`), in the `Auto` policy: after 10 journals opened in 60 s, the next `Auto` writes open none and stay atomic through tempfile and rename. `--wal-policy always` is never limited.
- A journal that committed is always removed.
- `startup_wal_heal` runs only for verbs that mutate and only on the directories named in argv (the subtree for a directory target, the parent only for a file target). It skips `.git`, `target` and `node_modules`, honours `.gitignore` and `.ignore`, and checks its 100 ms budget inside the walk (`walk_journal_paths_bounded`).
- `wal-heal` keeps its full-scan behaviour. `--no-auto-heal` turns the startup heal off.
- `JournalGuard::discard()` removes the journal when the rename arm fails with `Cancelled`, so a cancelled run leaves none. A real failure and a crash keep the journal.
- `inplace.rs` removes only the tempfile of its own operation.

## Consequences
- 120 files with `replace foo bar`: 110 residual journals before, 0 after.
- In a `replace` over 120 files only the first 10 get a crash-recovery journal.
- `semantic-merge` and `batch` are not in the explicit-target list, so they get no automatic heal, only the explicit `wal-heal` (gaps.md W28, P3).
- Correction, 2026-10-28: only `batch` is still outside the list. `semantic-merge` was added to `explicit_target_roots` as `a.output`, so it now gets the automatic heal; the P3 that remains is `batch` alone.
- A `kill -9` still leaves a journal (46 of 100 runs), by design. `wal-heal` is the recovery path, and a `started` journal of a dead process is not removed by it (`removed:0, preserved:1`, gaps.md W44, P3).
- SIGTERM series after the fix: 0 of 100 and 0 of 100 runs left a journal, measured on the debug binary without a release repeat (gaps.md W46).
- The H4 sentinel is `.atomwrite_keep_wal`, the name that says what it does: a file in the target directory preserves the journal (gaps.md W15, closed in 0.1.37). The old `.atomwrite_no_wal` name is still honoured, because a directory that holds it keeps the behaviour it always had.

## Alternatives
- Keep H3 in the `Drop`: rejected, it kept the journal of a write that had already committed.
- Walk the whole workspace at startup: rejected, the cost grew with the size of `target/`.

## Evidence
- `CHANGELOG.md:161-167,262-263,286` and `gaps.md:158-173` (W1, W3, W9, W28), `gaps.md:296-301` (W46).
- `src/wal/policy.rs:84-96` (`creation_throttled`), `src/atomic/write.rs:106` (the only caller), `src/wal/heuristics.rs:46-62` (H3 counter) and `:146-147`, `src/wal/journal.rs:227` (comment that H3 is not consulted) and `:329`.
- `src/lib_dispatch.inc.rs:8` (`startup_wal_heal`), `:124` (`explicit_target_roots`), `src/lib.rs:218`, `src/wal/stats.rs:203` (`walk_journal_paths_bounded`), `src/wal/heal.rs:61-69`.
- `tests/cli_v0137_wal_journals.rs` (2 tests), `tests/cli_v0137_wal_startup_heal.rs` (4 tests), `tests/cli_v0137_wal_cancel.rs` (1 test).

## Trigger to revisit
- `semantic-merge` or `batch` is given an explicit-target list.
- The H4 sentinel is renamed or its meaning changes.
