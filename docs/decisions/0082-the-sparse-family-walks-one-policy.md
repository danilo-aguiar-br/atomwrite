# ADR-0082 — the sparse family walks one policy


## Status
- Accepted (2026-11-03)
- Amends ADR-0077, which owns the shared walk policy, and refines the measurement quoted by ADR-0080
- Amended by ADR-0083, which brought `hash`, `copy -r` and `semantic-search` into the same rule and gave the sparse verbs their own `--all`


## Context
- `sparse list` walked with `WalkerFlavor::AlwaysVisitHidden` in `src/commands/sparse.rs`, and that flavor reaches `src/commands/mod.rs` as `.hidden(false)`, so `--hidden` changed nothing in that verb
- On the folder ADR-0080 measured, `sparse list` answered 3 while `sparse outline` answered 2, and both exited 0, so the divergence was silent
- `sparse outline` and `sparse list` kept a timestamped `.bak.` copy while `semantic-search` and `recipe` already dropped it through `is_backup_path` in `src/commands/backup_names.rs`
- The `files_scanned` description spoke only about the `--max-files` budget, so a reader of the schema could not predict the difference between a run with and without `--hidden`
- `hash` and `copy` also ignore `--hidden`, and their comment says why: a hash or a tree copy that skipped `.env` would answer a question the caller did not ask


## Decision
- `sparse list` uses `RespectGlobalFlags`, so the common hidden rule decides and `--hidden` is the caller asking for every entry
- `sparse list` and `sparse outline` drop any path `is_backup_path` accepts, before the `--include` match, under and without `--hidden`
- The exclusion is the same predicate the other scanning verbs use, so the rule lives in one function instead of two
- The `files_scanned` description says a dotfile counts only under `--hidden`
- `hash` and `copy` keep their flavor, because their answers are about the whole tree and this decision does not reach them


## Consequences
- A default `sparse list` now hides dotfiles, which is a behavior change for a caller that relied on the old listing to inventory a journal or a `.env`
- The three sparse verbs answer the same tree, so a count on one predicts the other two
- A backup copy no longer occupies a `--max-files` slot nor reaches a reader as if it were source
- The schema description answers the question the number raises, and `docs/schemas/sparse.schema.json` is regenerated from the binary
- The ADR-0080 sentence that reported `sparse list` as 3 describes the tree before this change, and `sparse list --hidden` still answers 3 on it


## Alternatives
- Keep `AlwaysVisitHidden` and document that `--hidden` is inert: rejected, because a flag that changes nothing is the silent divergence this round closes
- Add `--no-hidden` as a second switch: rejected, because `--hidden` already exists and a pair of switches for one rule is a new surface
- Exclude backups only in `sparse outline`: rejected, because it would create between the two verbs exactly the divergence the first decision removes


## Trigger to revisit
- A sparse verb that needs to see dotfiles by default, which would split the family again and need its own reason
- A backup naming scheme `is_backup_path` cannot recognize, which would make the exclusion leak copies
