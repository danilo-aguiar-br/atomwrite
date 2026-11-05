# ADR-0087 — the transaction snapshot follows the backup policy and the rollback names its targets


## Status
- Accepted (2026-11-03)
- Extends ADR-0086, which published the rollback record and the backup path of `apply`
- Extended by ADR-0088, which keeps the snapshot of a target the transaction deleted and makes the rollback record name the copies a failed reversal preserved


## Context
- `snapshot_file` took a `create_backup` copy of every target before a transactional batch and never consulted `--no-backup`, `--keep-backup` or `--backup`
- Nothing removed those snapshots after a successful commit, so `batch --transaction --no-backup` left `f1.txt.bak.<stamp>` beside every file it had just written
- The snapshot is the only source `rollback_transaction` restores from, so it cannot be skipped at the start without losing the undo of the transaction
- `delete_backup_quietly` already treats a missing copy as success and never masks a committed write, so the removal needs no new rule
- The `rollback` record of `apply` and of `batch --transaction` carried no `target_resolved`, and the writer stamps a single value that neither verb had set before the rollback
- The writer already publishes several subjects as an array through `set_targets`, and the published schema already accepts a string or an array on `target_resolved`


## Decision
- A transaction always takes the pre-mutation snapshot, because it is the undo path, and removes it when the run ends
- The remove condition is `!keep_backup || no_backup`: `--keep-backup` keeps the copy, and `--no-backup` wins over it, the same precedence the operations already use
- The removal runs after every operation commits and after a rollback that restored everything, through `delete_backup_quietly`
- A rollback that fails keeps its snapshots, because they are the last known state of files whose recovery is no longer known
- `rollback_after_failure` of `apply` sets the reverted targets on the writer before publishing, and so does the batch rollback, with the origin `argv` or `stdin` of the manifest
- The targets are sorted and deduplicated and travel as an array, in the order the rollback touched them
- No struct gains the field, because the writer is already the single place that stamps `target_resolved`, and the schema already describes the array


## Consequences
- `batch --transaction --no-backup` leaves the workspace with exactly the files the manifest changed, and no residue
- The default transaction and every other verb agree on one rule: a backup is transient unless the invocation asks to keep it
- An agent that receives a rollback record finds every reverted path in `target_resolved`, without a second call to list the directory
- A `target_resolved` with one element is still a string, which is the shape the writer already publishes


## Alternatives
- Skip the snapshot when `--no-backup` is present: rejected, because the snapshot is the only source of the transaction undo, and a transaction that cannot be undone is worse than a transient file
- Capture the original content in memory: rejected, because it puts the size of every target in RAM and breaks the bounded-memory rule the batch follows
- Add `target_resolved` to both rollback structs: rejected, because no output struct declares a stamped key, and the writer already publishes an array
- Keep the snapshot always and document it: rejected, because it contradicts ADR-0038 and the promise that a backup is removed on success


## Trigger to revisit
- A snapshot kept in a directory outside the workspace, which would change where the undo copy lives
- A rollback that reports per-file targets rather than the resolved list, which would need a different field
- A transaction that runs on a target whose directory denies the copy but allows the write
