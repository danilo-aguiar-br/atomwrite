# ADR-0089 — the target is always an array, and the batch names its copies


## Status
- Accepted (2026-11-03)
- Amends ADR-0088, which kept the string-for-one shape in every verb except the rollback record
- Amended by ADR-0090, which implements the two follow-ups this record left for a later round


## Context
- `set_target` wrote a string, `set_targets` wrote a string for one subject and an array for several, and the error envelope repeated the same rule in `targets_value`, so one field carried two types
- A consumer had to read the count before choosing how to read the field, and the schema declared the union of both shapes
- `BatchOpResult` had no `warnings`, so the redundant `--keep-backup` warned in the standalone `delete` and vanished inside the batch
- `execute_delete` and `execute_move` discarded the `PathBuf` that `create_backup` returned, so the copy existed on disk and no record named it
- `execute_move` skipped the copy when the destination was itself a backup and filled no `backup_skipped`, while the `delete` of the same batch declared that refusal
- `destructive_paths` matched only `delete`, so the commit discarded the snapshot of a destination that `move` or `copy` overwrote
- The summary of a committed transaction named neither the snapshots it kept nor the copies it removed, and the caller listed the directory to find them
- The rollback of `apply` restored content and removed nothing, so a `.bak` made for a committed file survived an unnamed


## Decision
- `target_resolved` is always an array, produced by `set_target` and `set_targets` alike, and `set_targets_array` no longer exists
- `targets_value` always returns an array, and the published schema declares one `array` of strings instead of the union
- `BatchOpOutcome` and `BatchOpResult` carry `backup_path` and `warnings`, and the operation record publishes both
- `execute_delete` fills `warnings` from `KEEP_BACKUP_REDUNDANT_FOR_DELETE` before the preview return, so `--dry-run` answers the flag too
- `execute_move` fills `backup_skipped` with `BACKUP_OF_BACKUP_SKIPPED` when the destination is a backup, and `backup_path` when it made a copy
- `destructive_paths` matches the targets of `delete` and the destination of `move` and `copy`; the source of a `move` stays out, because its content lives on at the destination
- The commit collects the snapshots that survived it, and `BatchSummary` publishes them as `backups_preserved`
- `rollback_after_failure` receives the `WriteResult` of every committed file, collects the `backup_path` whose file still exists, and publishes it as `ApplyRollbackEvent.backups_preserved`


## Consequences
- A consumer of any terminal envelope reads one shape for `target_resolved`, and the count no longer changes the type
- An agent finds the undo of a batch operation and the copies a failed `apply` left without listing the directory
- A transaction reports the snapshot it kept, so the recovery path travels in the output instead of in the filesystem alone
- An overwritten destination inside a transaction keeps its previous content, because the commit no longer treats it as residue
- The change is breaking for an external consumer of the old string shape, which `0.y.z` permits and `docs/MIGRATION.md` covers with the reading recipe


## Alternatives
- Keep the union and accept both shapes: rejected, because a field whose type depends on a count the caller has not read forces a branch in every consumer
- Keep the string for the standalone verbs and the array only for the rollback: rejected by the owner, who chose one shape for every verb
- Suppress the copy of the operation when the transaction already snapshotted the same file: rejected for this round, because it would change what `--backup` means inside a transaction, and the duplicate is declared as a follow-up
- Publish the failed write's copy in the `apply` error branch: rejected for this round, because `commit_target` does not return its `WriteResult` on failure and the value is not reachable without a wider change
- Keep the snapshot of the source of a `move`: rejected, because the content is readable at the destination and the copy would double bytes without adding a way back


## Trigger to revisit
- A caller that still needs the string shape, which would reopen the contract with a versioned field
- A transaction that pays two copies for the same file, which reopens the `--backup` meaning inside a transaction
- An `apply` failure that leaves an unnamed `.bak`, which reopens the error branch of `commit_target`
