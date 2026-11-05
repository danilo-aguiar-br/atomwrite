# ADR-0091 — the transaction lends its copy to a rewrite


## Status
- Accepted (2026-11-03)
- Amends ADR-0088 and ADR-0090, which lent the snapshot to `delete` and `move` only


## Context
- A transactional run snapshots every target before its first mutation, and that snapshot is the way back for every operation
- `execute_delete` and `execute_move` already asked the transaction for the copy instead of writing a second one
- `execute_write`, `execute_replace`, `execute_edit` and `execute_copy` never received the snapshot list, so each one created its own copy
- With `--keep-backup` the run left two copies of the same bytes, and the operation published one path while `backups_preserved` published the other
- `collect_target_paths` also snapshotted the source of `move` and `copy`, which no rollback path reads
- `destructive_paths` named only `delete`, `move` and `copy`, so the rewrite survived the commit by accident of the `else if`


## Decision
- One helper, `copy_decision`, answers the copy question for every operation that rewrites a target
- When a snapshot exists and `--keep-backup` asks for the copy, the writer runs with `backup: false`, the record publishes the snapshot path, and `warnings` says the transaction already preserved the target
- When a snapshot exists and the copy is temporary, the writer still runs with `backup: false`, the record publishes no path, and the commit discards the snapshot
- When no snapshot exists, the writer creates the copy as before, and `backup_path` comes from the writer
- A target that is itself a backup gets no second copy, by the same rule `delete` already applied
- The commit always calls `discard_residue`, and `destructive_paths` names a rewrite only when `--keep-backup` asked for it and `--no-backup` did not forbid it
- The transaction snapshots the destination of `move` and `copy` and no longer snapshots their source


## Consequences
- `--keep-backup` on a transactional rewrite leaves one file, and `backup_path` equals the surviving path in `backups_preserved`
- Without `--keep-backup` no copy survives, and the record no longer promises a path the commit removes
- A `move` or `copy` run stops writing a source snapshot that the reversal never reads
- Writing into a backup file stays a single operation on that file, with no `.bak.bak` chain
- The published description of `backup_path` now covers both origins of the field


## Measurement
- Scenario: one `write` of an 8 MB file through `batch --transaction --keep-backup`, run on Linux with the binary the install placed in `~/.cargo/bin/atomwrite`
- Before this change, the same scenario left 2 files matching `.bak.` in the directory and reported `elapsed_ms` 11 in the summary
- After this change, the same scenario leaves 1 file of 8.0 MB and reports `elapsed_ms` 1 in the operation and 7 in the summary
- The earlier binary is no longer on disk, because the install replaced `~/.cargo/bin/atomwrite` and `target/release/atomwrite` at 2026-11-03 18:34, and the repository holds no earlier source
- The baseline above is the execution recorded in the round that measured it, and the copy count is the number the decision rests on


## Alternatives
- Keep the per-operation copy and remove the snapshot: rejected, because the snapshot is the transaction's undo path and exists before the first mutation
- Publish the snapshot path even when the commit removes it: rejected, because the agent would chase a path that no longer exists
- Leave `destructive_paths` implicit: rejected, because the rule would depend on the shape of an `if` rather than on a named decision
- Snapshot the source of `move` and `copy` for symmetry: rejected, because no rollback path reads it and the copy costs a full file


## Trigger to revisit
- An operation that mutates its source without a rename the reversal can undo
- A verb that needs the transaction copy without a `--keep-backup` keep
