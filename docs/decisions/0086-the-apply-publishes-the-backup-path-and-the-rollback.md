# ADR-0086 — the apply publishes the backup path and the rollback


## Status
- Accepted (2026-11-03)
- Extends ADR-0081, which named the verdict of `apply`, and ADR-0085, which carried the same verdict to the verbs that rewrite content
- Extended by ADR-0087, which makes the transaction snapshot follow the backup policy and names the reverted targets in the rollback record


## Context
- `apply` read `write_strategy` and `backup_skipped` from the `WriteResult` and dropped `backup_path`, although the same value arrived in the same variable
- v0.1.24 recorded the same defect on `move` as an incident: the `.bak` reached the disk and the event never said where it was
- A patch that names several files prepares all of them and writes them in order, and the `applied` records are emitted only after the last commit, so a failure in the middle left the stream with no record of what had been written or undone
- `rollback_committed` restored the files and reported nothing, and a failed restoration only reached `tracing::error!`, so the caller read the commit failure as if the disk were back to the state before the run
- The rollback ran with `AtomicWriteOptions::default()`, which has `backup: true` and `keep_backup: false`, so every restored file paid a full copy that was deleted right after
- `docs/schemas/apply-result.schema.json` described only the `applied` record, although the verb also writes a `plan` record under `--dry-run`
- Fifteen optional fields of the write verdict declared `["string", "null"]` while the binary omits them instead of emitting `null`


## Decision
- `ApplyResult` publishes `backup_path`, taken from the `WriteResult`, and omits the key when no copy was made
- An `apply` that fails after writing at least one file restores the written files and publishes one `rollback` record with `operation` equal to `apply`, the number of files restored and how long the restoration took
- The record goes through `publish_failure`, so it carries `ok: false` and the very `code` and `exit` of the failure that forced the rollback, and `main` writes no second envelope
- A restoration that fails names the file in `failed_paths` and turns the returned error into `TransactionRollbackFailed`, the same outcome `batch` uses, because the disk state is no longer known
- A run that fails before writing anything publishes no rollback record, because nothing was undone and the error envelope already says the call failed
- The rollback writes without a backup, because a copy of the content the run produced would be created and deleted in the same call
- The restoration passes the digest of the content it finds on disk, so the atomic writer compares bytes instead of inodes: the run holds the lock of the inode its own commit replaced, and the inode comparison would refuse every restoration with `StateDrift`
- `batch --transaction` takes the same digest, because its rollback hit the same refusal and left the tree partially reverted
- The schema of `apply` becomes the union of `plan`, `applied` and `rollback`, in the file `apply-result.schema.json`, which needs no new name
- Every optional field of the write verdict declares a plain `string`, and the schemars `extend` changes the type without adding the field to `required`
- A vector the binary omits when it is empty carries `serde(default)`, so the published schema stops demanding a key the run does not write; `failed_paths` and the `submatches` of a search match follow that rule


## Consequences
- An agent that receives a failed `apply` learns from one record that the files came back, how many came back and which ones did not
- The `.bak` of a patch is discoverable without listing the directory
- A consumer validating the stream against the published schema accepts the dry-run plan and the rollback record
- The published contract stops promising `null` on a field the binary never emits


## Alternatives
- Publish the `applied` records before the rollback: rejected, because those records describe a content that no longer exists after the rollback, and the counters of the rollback record already say how many came back
- Reuse the `RollbackEvent` of `batch`: rejected, because that record counts the files a transaction removed and the rollback of `apply` restores files and fails per file, so `failed_paths` would be a field `batch` could never fill
- Emit a rollback record with zero restored files: rejected, because the error envelope already reports the failure and a record with nothing undone is noise
- Take a copy of the patched content during the rollback: rejected, because `keep_backup` is false in that path, so the copy would be written and deleted in the same call


## Trigger to revisit
- A `--format full` that accepts more than one target, which would let the rollback create and remove files
- A commit path that fails after a partial write, which the atomic writer does not have today
- An event that needs to carry more than one resolved target on its terminal line
