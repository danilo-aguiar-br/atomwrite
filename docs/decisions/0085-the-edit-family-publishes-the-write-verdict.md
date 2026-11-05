# ADR-0085 — the edit family publishes the write verdict


## Status
- Accepted (2026-11-03)
- Extends ADR-0084, which made the copying and unlinking verbs name the mechanism, to the verbs that rewrite content
- Extended by ADR-0086, which carries the same verdict to `backup_path` and makes the rollback of `apply` publish what it restored


## Context
- `atomic_write_based` returns the same `WriteResult` that `atomic_write` returns, and the nine verbs of this round called it and consumed only the error arm or the checksum
- `edit`, `replace`, `transform`, `scope`, `case`, `set`, `del`, `edit-loop` and `locale` each publish an event per file, and every one of those events stayed silent about the path the write took
- `scope` and `transform` have a read-only branch that builds the same struct without writing anything, and `case`, `transform` and `scope` also answer a preview, so a single required field with no absence value would have let a preview claim a write it never performed
- `del` returns early when the key never existed, and the sibling `config unset` already omits the field in that case
- `batch delete` copied a backup of a backup, because the no-copy-of-a-copy rule lived in `src/commands/delete.rs` instead of a module both verbs share


## Decision
- `edit`, `replace`, `transform`, `scope`, `case`, `set`, `del`, `edit-loop` and `locale` publish `write_strategy` and `backup_skipped`, so a consumer reads the mechanism from the event that reports it
- `write_strategy` is required on every event that reports a write, and `backup_skipped` stays optional and is skipped in the JSON when absent
- A preview and a read-only branch publish `"none"`, by the same rule ADR-0084 fixed for `batch --dry-run`, because the run took no write path
- `del` on an absent key omits `write_strategy` instead of publishing `none`, because the sibling `config unset` omits it and a verdict for a key that was never there would be false
- `locale --clear` publishes `"none"`, because it removes the preference file and writes no content, and the plain `locale` report omits both fields because it writes nothing
- `edit-loop` publishes the verdict of the single file it rewrites, because it writes once per invocation and not once per pair
- `batch delete` refuses to copy a copy, by the shared `backup_names::is_backup_path` predicate, and both verbs read the `BACKUP_OF_BACKUP_SKIPPED` sentence from that module so the text cannot drift
- The target named as a backup is still removed, because the refusal is about the copy and not about the removal


## Consequences
- The nine verbs answer the same shape the seven verbs of ADR-0084 answer, so a record from one is compared with a record from another
- A preview and a write stop looking alike to a consumer, because the preview publishes `none`
- A consumer that fails on an unknown field must allow the two names, and the migration guide says so
- The nine schemas are regenerated from the binary by `cargo xtask schemas`, so the file and the event cannot drift
- The `delete` and the `batch delete` share one sentence, so the same tree no longer answers differently depending on the verb


## Alternatives
- Extract one `WriteVerdict` with `#[serde(flatten)]`: rejected, because the serde book measures that `flatten` needs a named-field struct or a map and does not coexist with `deny_unknown_fields`, and the saving would be two lines per struct
- Publish `none` on the `del` of an absent key: rejected, because the sibling `config unset` omits it and two brothers would carry different meanings for the same field
- Require `write_strategy` on the plain `locale` report: rejected, because that report writes nothing and `none` would read as if it had
- Make `backup`, `prune-backups` and `wal-heal` publish the verdict: rejected for this round, because they never call the atomic writer and have no `WriteResult` to drop


## Trigger to revisit
- A verb that rewrites content through a path the `WriteResult` does not describe, which would need a fourth strategy value
- A preview that starts writing, which would make `none` wrong
- A `batch` operation that copies a backup of a copy under a condition the shared predicate does not cover
