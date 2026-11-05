# ADR-0081 — the verdict names the mechanism and the subject


## Status
- Accepted (2026-11-03)
- Amends ADR-0078, which closes every terminal branch and therefore makes a new field ship with its regenerated schema


## Context
- `write` carried `write_strategy` inside the write result and dropped it in `src/commands/write.rs`, so a caller had to infer the strategy from the inode or from `platform.rename_method`
- `platform.rename_method` answers `renameat2`, `rename` or `MoveFileEx`, which names the syscall and cannot separate an in-place write from a swap
- `apply` ended `commit_target` with `.map(|_| ())` in `src/commands/apply.rs`, so the `backup_skipped` the atomic layer filled for a refused copy never reached the event
- Measured on a folder under `setfacl -m u::r-x`: `apply --backup` exited 0 with `ok:true` and no `backup_skipped` in the JSON, while the same case on `write` published the reason
- A new file in that folder exited 13 and named `.novo.txt.atomwrite.lock`, a sidecar the run creates and unlinks, so the retry the message invites reaches nothing
- `src/lock.rs` classified the failure with `from_io_at(&lock_path, error)` because the sidecar was the only handle a new file had


## Decision
- `write` publishes `write_strategy` as a required field, and `apply` publishes the same field plus an optional `backup_skipped`
- Both come from the write result the atomic layer already returns, and no new computation is added
- The lock failure is classified against the target, because the target is the file the caller named and the sidecar is the mechanism
- The sidecar path and the I/O error stay on a `tracing::warn!` line in stderr, where diagnosis belongs
- The exit code does not change, because `from_io_at` keeps choosing the variant by `io::ErrorKind`


## Consequences
- A caller proves the strategy from one field, and an in-place write is distinguishable from a swap without measuring the inode
- `apply` no longer hides a refused copy, so stdout alone carries the whole verdict
- The error path names a file that exists, and the sidecar is still traceable in the log
- `docs/schemas/write-output.schema.json` and `docs/schemas/apply-result.schema.json` are regenerated from the binary, because ADR-0078 closes the branch
- `batch`, `copy`, `move`, `delete`, `rollback`, `semantic-merge` and `config` also drop these fields, and they are left for another decision


## Alternatives
- Publish `write_strategy` only in `write` and leave `apply` silent: rejected, because the cause is the same and the sibling divergence is what this round closes
- Keep the sidecar path in `path` and add the target as a second key: rejected, because `path` is the subject by contract and two subjects in one error is a new ambiguity
- Derive the strategy from `platform.rename_method`: rejected, because it names the syscall and answers `renameat2` for an in-place write


## Trigger to revisit
- A verb that writes and needs a strategy field of its own, which would make this a stamp at the writer instead of a field per verb
- A platform whose in-place path cannot be named, which would make the three strategy values insufficient
