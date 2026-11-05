# ADR-0084 — every mutating verb publishes the write verdict


## Status
- Accepted (2026-11-03)
- Extends ADR-0081, which made `write` and `apply` name the strategy, to the rest of the surface
- Extended by ADR-0085, which brought the nine verbs that rewrite content into the same shape


## Context
- `atomic_write` returns a `WriteResult` that carries `write_strategy` and `backup_skipped`, and `src/atomic/write.rs` is the only place that knows which path the write really took
- `copy`, `move`, `delete`, `rollback`, `semantic-merge`, `config set`, `config unset` and the operations of `batch` called it and dropped the result, so the event reported `bytes` and `checksum` while the mechanism stayed invisible
- A caller could only infer the mechanism from the syscall name or from the inode, and neither travels in the JSON
- `batch` runs its operations in a parallel closure, so a borrowed field would have tied the worker to the caller's lifetime
- `config` answers six actions, and four of them read and write nothing, so a required field would have made a read claim a mechanism it never used


## Decision
- `copy`, `move`, `delete`, `rollback` and `semantic-merge` publish `write_strategy` as a required field, because every one of them writes or unlinks the target it reports
- `delete` publishes the value `none`, because it removes the directory entry and takes no atomic write path
- `move` publishes the strategy of the syscall it took: `rename` on the same filesystem, and the value of the `WriteResult` on the cross-device fallback, which `cross_device` and `atomic` already separate
- `config set` and `config unset` publish the field only when they wrote, and the `locale` store publishes `rename` for the preference file and `none` for its removal
- `batch` publishes the verdict per operation, with `none` for `hash` and `delete` and the `WriteResult` value for `write`, `replace`, `edit` and `copy`
- A `batch --dry-run` operation publishes `none` too, because the event reports a plan and the run used no write path
- The field of `batch` is `&'static str`, so the parallel closure stays `Send` and the value is still the closed set the engine produces
- `backup_skipped` is optional everywhere and is skipped in the JSON when absent, so no event carries a `null` the reader cannot tell from a skipped copy


## Consequences
- A consumer reads the mechanism from the event that reports the write, and the inode and the syscall stop being the only evidence
- The seven verbs answer the same shape, so a record from one is compared with a record from another
- Analyses that validate the event against a closed schema must allow the two names, and the migration guide says so
- `write_strategy` on a read action of `config` stays absent instead of claiming `none`, which keeps a read and a unlink distinguishable
- The eight published schemas are regenerated from the binary, so the file and the event cannot drift


## Alternatives
- Require `write_strategy` in `config` for every action: rejected, because `path`, `list`, `get` and `doctor` write nothing and `none` would read as if they had
- Publish a free-form string built per verb: rejected, because the value is a closed set and a text field invites a variant the engine cannot produce
- Leave `batch` without the field and wrap the operation result: rejected, because it would report the verdict one level away from the operation that produced it


## Trigger to revisit
- A verb that takes a write path the `WriteResult` does not describe, which would need a fourth strategy value
- A `batch` operation that writes in a way the outcome enum cannot name, which would make `none` wrong
