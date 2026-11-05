# ADR-0093 — the backup key distinguishes unset from false


## Status
- Accepted (2026-11-03)
- Amends ADR-0092 and closes the trigger it recorded


## Context
- `DefaultsSection.backup` was a `bool` with a `true` default, so an absent key and a written `false` carried the same value
- `delete` passed its own silent default and never read the key, and the `batch` computed `backup_explicit` from the flag alone
- ADR-0092 recorded the exception and named the condition to revisit it: a config change that lets the key govern `delete` without breaking the published contract


## Decision
- `DefaultsSection.backup` is `Option<bool>`, and `None` means the key is not written
- An absent key keeps each verb's own default: `resolve_backup` passes `true` to the rewrites, and `delete` passes `false`
- A written `true` reaches `delete` and the `batch`, and a written `false` turns the copy off for the rewrites
- `--no-backup` still wins over the key, and `--backup` still forces the copy
- `--keep-backup` still only preserves a copy that exists, and the warning appears only when no copy was created
- `config get` keeps `origin: default` and omits `value` while the key is not written, so the published shape does not gain `null`


## Consequences
- A user who writes `backup = true` gets a copy on `delete`, and a user who never writes the key keeps the delete without a copy
- `config unset defaults.backup` returns to the verb defaults
- The `batch` delete, the `batch` rewrite, `move` and `copy` see the written key the same way they see the global `--backup`
- The schema of the key keeps `boolean`, because the TOML still accepts only `true` and `false`
- The warning of `--keep-backup` is computed per file, so a target that is itself a backup still gets the advice even when the key requested a copy


## Alternatives
- Keep `bool` and detect presence with a second parse of the TOML: rejected, because it duplicates the source of truth and `Option` is the idiom
- Make `delete` copy by default: rejected, because deleting is the request and the copy must stay opt-in
- Add a second key for the delete: rejected, because the existing key already names the meaning


## Trigger to revisit
- A request to make `--keep-backup` create the copy on `delete`
- A config format that needs a third state beyond absent, true and false
