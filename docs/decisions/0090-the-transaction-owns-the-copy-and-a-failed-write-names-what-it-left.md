# ADR-0090 — the transaction owns the copy, and a failed write names what it left


## Status
- Accepted (2026-11-03)
- Amends ADR-0089, which declared the duplicate copy and the unnamed failed copy as follow-ups for a later round


## Context
- A transactional `batch` snapshots every target before the first mutation, and `execute_delete` and `execute_move` created a copy of the same file again, so two timestamped copies of one file survived the commit
- The copy of the operation was not part of the snapshot list, so no commit rule removed it, and `backup_path` named only that one while the summary named the other
- `atomic_write_based` created the backup in the middle of the write and only attached it to `WriteResult` at the end, so a failure in between left the file on disk with no record naming it
- `rollback_after_failure` received only the `WriteResult` of the files that committed, so the copy of the file that failed never reached `backups_preserved`
- The `apply` prepares every target before the first write, so a block that does not match never reaches the commit, and the second-file failure the check-points needed had no permitted inducer


## Decision
- `BatchOpOptions` carries the snapshot list of the run, borrowed rather than owned, so the fan-out closure stays `Copy` and no path is cloned per operation
- `execute_delete` and `execute_move` look up the target they would copy and, when the transaction already holds it, publish that path in `backup_path` and answer `TRANSACTION_SNAPSHOT_SUPERSEDES_COPY` in `warnings` instead of writing a second file
- Outside a transaction the list is empty, so the operation writes its own copy exactly as before
- `atomic_write_based` collects the copy it creates in an out-parameter and attaches it to any failure as an anyhow context, so the typed error underneath stays reachable by downcast and every other caller keeps propagating it unchanged
- `apply` reads the path from the failure, adds a sentence to the error naming where the copy stayed, and passes the path to `rollback_after_failure`, which publishes it after the committed copies
- The description of `backups_preserved` covers the failed file's copy, and the 54 published schemas were regenerated from the binary


## Consequences
- A transactional `delete` or `move` leaves one copy on disk, and the run names it twice with the same path, once in the operation record and once in the summary
- `--keep-backup` inside a transaction keeps one copy instead of two, which is the case where the bytes matter most
- An `apply` that fails after making a copy names it in the error message, and also in the rollback record when some file had already been committed
- The destination of `write` and of `replace` still keeps no copy, by the owner's decision in this round, so the asymmetry with the destructive operations stays as ADR-0088 fixed it


## Alternatives
- Remove `delete` from `destructive_paths`, so the commit discards the transaction snapshot and the copy of the operation survives: rejected, because two copies would remain whenever `--keep-backup` is set, which is exactly the case the rule exists for
- Delete the copy when the write fails: rejected, because the in-place strategy can fail midway with the target already modified, and that copy is then the only way back
- Name the failed copy by scanning the directory: rejected, because a timestamped name cannot be reconstructed, and a scan would name a copy another run created
