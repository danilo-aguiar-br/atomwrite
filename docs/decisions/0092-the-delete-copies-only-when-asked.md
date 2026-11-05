# ADR-0092 — the delete copies only when the call asks


## Status
- Accepted (2026-11-03)
- Records the default the 0.1.38 code and test already fixed
- Clarifies ADR-0038 for `delete` and for the `batch` operations
- Amended by ADR-0093: the written key now reaches `delete` and the `batch`


## Context
- `delete` passed `silent_default = args.backup_opts.backup == Some(true)`, so without `--backup` it created no copy
- `delete_default_leaves_no_bak_file` and `delete_keep_backup_emits_warning_in_envelope` already fixed that contract in the tests
- The shared help still said the backup was on by default, and the guides, the `llms-full` file and the skill repeated it
- `--keep-backup` alone created no copy on the standalone `delete`, but the `batch` delete counted it as a request and created one
- `[defaults].backup` never reached the `delete`, because the verb passes its own silent default


## Decision
- `delete` creates a copy only when the call asks, through `--backup` or through the written `[defaults].backup = true` (ADR-0093); `--no-backup` is accepted and wins
- `--keep-backup` only preserves a copy that exists, and the warning says that `--backup` is the flag that creates one
- The `batch` delete follows the same rule: `--keep-backup` alone does not create a copy
- `[defaults].backup` did not govern `delete` or the `batch` operations in this round; ADR-0093 supersedes this line, and an absent key keeps the behavior recorded here
- The help of `--backup`, the help of `--keep-backup`, the `about` of `delete`, the comment in `delete.rs`, the guides and the skill carry the real rule


## Consequences
- An agent that reads `delete --help` learns that no copy is created without `--backup`
- A `delete` copy, once created, is never auto-removed, so `--keep-backup` keeps one meaning across the standalone verb and the batch
- The `[defaults].backup` key stops being a silent promise on `delete`, and the exception is written where the key is documented
- The history of `docs/MIGRATION.md` keeps the line written for 0.1.28, and the correction arrives as a new bullet


## Alternatives
- Turn the copy back on by default: rejected, because deleting is the request and a copy the caller did not ask for keeps bytes on disk
- Read `[defaults].backup` in `delete`: rejected for this round and accepted in ADR-0093, which adds the absent state instead of changing the meaning of the key
- Keep the shared help and document the exception only in the guides: rejected, because the first line an agent reads is the help


## Trigger to revisit
- Closed by ADR-0093: `backup` is now `Option<bool>`, so an absent key and a written `true` are different states
- A request to make `--keep-backup` create the copy on `delete`
