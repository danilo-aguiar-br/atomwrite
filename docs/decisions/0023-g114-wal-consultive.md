# ADR-0023: G114 WAL recovery is consultative, not automatic

- **Status**: Accepted
- **Date**: 2026-06-07
- **Context**: When a process is killed between `Started` and `Committed`, the WAL sidecar has a dangling `Started` entry. The naive fix is to auto-replay or auto-rollback the write on next start. We chose to expose `recover_orphan_journals(dir)` as a public function that returns a report without touching the filesystem.
- **Decision**: Recovery is consultative. `recover_orphan_journals(dir)` reads all `.atomwrite.journal.<basename>.atomwrite.journal.json` files in a directory and returns `OrphanJournalReport { entries: Vec<JournalEntry>, target, op_id, started_at_unix, pid, age_secs }`. The caller (a CI hook, a developer, a future `atomwrite wal-recover` CLI) decides what to do.
- **Consequences**:
  - **+** Zero risk of "helpful" auto-recovery making a wrong decision (e.g. replaying a write that the user already manually fixed).
  - **+** Library users can wire the report into their own UI / log / metrics.
  - **+** The recovery function is pure and easy to test.
  - **-** There is no `atomwrite wal-recover` CLI yet; users have to script the recovery themselves (deferred to v0.1.13).
  - **-** Orphan journals accumulate if nobody runs recovery.
- **Alternatives considered**:
  1. Auto-replay on next atomwrite start. Rejected: dangerous; user may have manually fixed the file.
  2. Auto-delete orphan journals older than 7 days. Rejected: silent data loss; consultative is safer.
  3. Reject all `atomic_write` calls if an orphan exists. Rejected: too aggressive; legitimate use cases include the user wanting to ignore orphans.
- **Trigger to revisit**: When `atomwrite wal-recover` CLI is implemented (v0.1.13), we can add a `--cleanup` flag that deletes orphans with user confirmation.
