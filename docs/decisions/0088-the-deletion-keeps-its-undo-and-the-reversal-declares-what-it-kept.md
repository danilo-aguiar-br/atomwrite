# ADR-0088 — the deletion keeps its undo, the reversal declares what it kept, and the apply names the stream


## Status
- Accepted (2026-11-03)
- Extends ADR-0087, which removed every snapshot after a commit and named the reverted paths
- Amended by ADR-0089, which makes `target_resolved` an array in every verb and fills the batch records with the copies they made


## Context
- `collect_target_paths` snapshots every target of a transaction, including the target of an operation that deletes it
- The commit discarded every snapshot through `discard_snapshots`, so a transaction that deleted a file removed the file and its only copy, even without `--no-backup`
- The standalone `delete` treats its copy as the recovery artifact and warns that `--keep-backup` is redundant, so the batch and the standalone verb disagreed about what a deletion copy is for
- A reversal that stopped halfway published no `rollback` record at all: the process answered with the generic error envelope, and the snapshots left on disk had no name in the output
- `rollback_transaction` returned only an error, so the partial count of files that did come back was lost
- `cmd_apply` always reads the patch from standard input, and a patch can carry its own path line, yet all three stamps credited `argv`
- `set_targets` publishes one subject as a string, so the rollback shape depended on how many files came back


## Decision
- The commit of a transaction discards the snapshot of every target it rewrote and keeps the snapshot of every target it deleted, because that file lives nowhere else
- `--no-backup` still wins and removes every snapshot, and `--keep-backup` still keeps them all, so the flag keeps its meaning
- A successful reversal discards every snapshot, because the deleted file is back on disk and the copy became residue again
- The reversal reports partial truth: `rollback_transaction` returns the counts it reached, the first path it could not restore and the diagnostic, instead of only an error
- A reversal that cannot restore a file publishes a `rollback` record through the failure path, with `failed_paths` naming what did not come back and `backups_preserved` naming the snapshots that stayed
- `apply` stamps `stdin` when the patch names its own file and `argv` when `--file` names the target on the invocation line, for the plan record, the applied records and the rollback record alike
- The `rollback` record of `apply` and of `batch --transaction` always publishes `target_resolved` as an array, through `set_targets_array`; every other verb keeps the string-for-one shape of `set_targets`


## Consequences
- A transaction that deletes a file leaves `f1.txt.bak.<stamp>` beside the hole, which is the way back the standalone `delete` already promised
- A reversal that fails names the copies it kept, so the agent reads the recovery paths from the record instead of listing the directory
- A consumer of the rollback record handles one shape and no longer branches on the count
- An agent that reads `target_source` learns where the target really came from, and no envelope credits a line the caller never wrote
- The published schema of `batch` gains two array fields, and the published schema of `apply` is unchanged because the shape it already allowed stays allowed


## Alternatives
- Remove the deletion copy on success for uniformity: rejected, because the file would exist nowhere and the run would destroy data to satisfy a rule written for rewrites
- Keep every snapshot after the commit: rejected, because a rewrite would leave residue that `--no-backup` and ADR-0038 forbid
- Publish only the error envelope when the reversal fails: rejected, because the caller cannot tell which copies survived, and a hidden state is the failure this project punishes
- Make every verb publish an array: rejected, because five existing tests and several consumers read the field as a string, and the change would break the contract of forty verbs to fix one event
- Stamp `argv` for a patch that names its own file: rejected, because the invocation line carried no path at all and the caller cannot verify an origin that never existed


## Trigger to revisit
- A batch operation that destroys a file and exposes the same snapshot through a named field of its own record
- A consumer that needs the deleted file's copy path without reading `backups_preserved`
- A move whose source removal would need the same treatment as a deletion
