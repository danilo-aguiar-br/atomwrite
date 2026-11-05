# ADR-0094 — the recursive delete owns the whole named tree


## Status
- Accepted (2026-11-03)
- Amends ADR-0083, which owns the shared walk policy, and ADR-0092 and ADR-0093, which own the delete copy


## Context
- `delete --recursive` built its walker through `build_scoped_walker`, which is `WalkerFlavor::RespectGlobalFlags`, so the walk hid every dot entry and every path an ignore file listed
- The directory removal pass then used a different walker and swallowed every `remove_dir` error, and the verb published a synthetic `InvalidInput` with exit 65 after it had already deleted the visible files
- The caller that received exit 65 found the directory still on disk with `.git`, `kept.log` and hidden entries inside, and had to remove it with `rm -rf`
- The shared helper touched only `hidden`, `git_ignore` and `follow_links`, so `.ignore`, the global gitignore and `info/exclude` kept hiding entries even with `--no-gitignore`
- The collection accepted only files and links, so a fifo, socket or device inside the tree survived and kept the parent on disk
- The hash and the backup ran by path, so a fifo could block the open and a link could copy the content of its target
- A backup copy written next to a file inside the tree outlived the removal and kept the root on disk


## Decision
- `delete --recursive <dir>` without a narrowing filter owns the whole named tree, and removes hidden, ignored, linked, fifo, socket and device entries
- `WalkerFlavor::DeleteTree` turns off every ignore source, hidden filtering, parent ignore lookup and link following, and it stays on one file system
- The traversal never follows a link, including a link named as the root, which is unlinked as an entry and never as its target
- A fifo, socket or device is unlinked without being opened and publishes no `checksum_before`, because there is no content to hash
- A requested backup for a link or for a special entry is refused per event with `backup_skipped` naming the reason
- A requested backup with a directory root is refused before the first unlink, because the copy would be created inside the tree and outlive the removal
- The directory cleanup uses the same `DeleteTree` walker and removes empty directories deepest first, tolerating a `NotFound` and recording every other error with its path
- A run without narrowing publishes a real I/O failure with the surviving path in `errors[]` when a directory survives, and never the synthetic exit 65
- A run narrowed by `--include`, `--exclude`, `--older-than` or `--limit` exits 0 and publishes `dirs_left`, `entries_left` and a hint that names the filters
- `--plan` and `--dry-run` publish no leftover fields, because they never removed anything
- Overlapping roots collapse into the outermost root and a repeated root is deduplicated
- An entry that disappears between collection and removal counts as skipped, so a repeated run reaches the same end state
- `--hidden`, `--no-gitignore` and `--follow-symlinks` are accepted as no-ops on `delete` and emit one diagnostic line
- `copy -r` keeps its promise and stops reading `.ignore`, the global gitignore and `info/exclude`


## Consequences
- A consumer that treated exit 65 from a filtered run as failure now receives exit 0 with `dirs_left` and `entries_left`
- A consumer that expected `INVALID_INPUT` when a directory survived without a filter now receives the measured I/O code, such as 74 or 13, with the surviving path in `errors[]`
- A caller that relied on `--follow-symlinks` to delete a target through a link now removes the link alone
- Deleting `.git` inside the named root becomes possible, which is the direct effect of owning the tree
- A mount point inside the tree is not crossed and becomes a named failure, which differs from `rm -rf`
- A fifo, socket or device named as the root is still refused by `validate_path` with exit 85, because the root is not an entry inside a tree
- The tree is collected before the removal, so memory grows with the number of entries
- Windows and macOS were not executed for this decision and stay declared as not validated


## Alternatives
- Add `--force` or a second flag to include hidden entries: rejected, because a delete that hides part of the named tree answers a different question, and a flag that repairs the verb hides the defect
- Keep `remove_dir_all` for the final removal: rejected, because it would destroy what `--exclude` kept out while no counter saw it
- Keep the synthetic exit 65 for a surviving directory: rejected, because it names an argument mistake after a partial deletion and hides the measured I/O error
- Create the backup outside the tree: rejected for now, because no verb accepts a backup destination and the copy would need a new contract
- Follow links to remove their targets: rejected, because the named tree owns the entries inside it and never what the entries point at


## Trigger to revisit
- A request for a backup destination, which would make `--backup` usable with a directory root
- A platform where `same_file_system` is not supported, which would need a different mount guard
