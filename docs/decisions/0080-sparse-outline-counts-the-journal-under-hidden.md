# ADR-0080 — `sparse outline` counts the atomwrite journal under `--hidden`

## Status
- Accepted (2026-11-03)
- Amends ADR-0077, which moved this verb to the common hidden policy and left a name filter behind it

## Context
- `sparse outline` walks with `WalkerFlavor::RespectGlobalFlags`, so without `--hidden` a dotfile is already out by the rule every scanning verb shares
- `src/commands/sparse_outline.inc.rs` also dropped every name starting with `WAL_JOURNAL_PREFIX`, and that filter ran after the walk without asking `global.hidden`
- Measured on a folder with `a.rs`, `b.txt` and one `.atomwrite.journal.x.atomwrite.journal.json`: `sparse outline` answered `files_scanned:2` and `sparse outline --hidden` also answered 2
- The same folder answered `count --hidden` as 3 files and `sparse list` as 3, so two verbs disagreed with the other two on the same tree

## Decision
- The journal is excluded by being a dotfile, and by nothing else
- The name filter is removed, so `--hidden` is the only switch that decides whether the tool's own sidecar appears in the answer
- No `!global.hidden` condition is added, because the walker already applies that rule and a second copy would be a second place to keep in sync
- A caller that wants the journal gone runs `wal-heal`, which already removes terminal journals and is idempotent

## Consequences
- `sparse outline --hidden` answers 3 on the measured folder, and `sparse outline` keeps answering 2
- The verbs agree again on the same tree, so an agent comparing their counts stops seeing a silent difference
- A stale journal costs tokens when the caller explicitly asks for hidden entries, and that cost is the caller's request being honored
- `files_unparsed` is unchanged by this, because the journal has a `.json` extension and resolves a grammar

## Alternatives
- Keep the filter and make it conditional on `!global.hidden`: rejected, because the walker already owns that rule and the duplicate would drift
- Keep the filter unconditional: rejected, because it makes `--hidden` lie, which is the failure this ADR closes
- Exclude both the journal and the lock sidecar by name: rejected, because the lock sidecar is unlinked on release and a name list grows with every sidecar the tool adds

## Trigger to revisit
- A second sidecar category that outlives a successful run and is not hidden by name
- A scanning verb that needs to hide the journal even under `--hidden`, which would be a different contract and a new ADR
