# ADR-0042: backup-by-default for content-mutating commands

- **Status**: Accepted
- **Date**: 2026-06-19
- **Context**: `AtomicWriteOptions::default()` had `backup: false`. The `write` command (and 8 other content-mutating commands) did NOT create backups by default. In the incident of 2026-06-15, an agent used `atomwrite write` instead of `atomwrite edit --before-match`, destroying 122,994 bytes of `gaps.md` with no backup to recover from. ADR-0035 (v0.1.20) added 6 defense-in-depth layers (L1-L6) but ALL were opt-in. The field `keep_backup: false` meant that even when `--backup` was explicitly passed, the backup was deleted silently after a successful write via `delete_backup_quietly`. The result: the default execution path (no flags) preserved ZERO recoverable copies. The Unix convention (cp, mv, dd) does not create backups by default, but this convention is inadequate for a tool whose primary audience is LLM agents that make semantic command errors (confusing `write` with `edit`).

- **Decision**: Change `backup` default from `false` to `true` in 9 content-mutating argument structs: `WriteArgs`, `EditArgs`, `EditLoopArgs`, `ReplaceArgs`, `TransformArgs`, `ApplyArgs`, `SetArgs`, `DelArgs`, `CaseArgs`. Change `AtomicWriteOptions::default().backup` from `false` to `true`. Add `--no-backup` flag to all 9 structs for explicit opt-out. Add `ATOMWRITE_BACKUP=0` environment variable for global opt-out. Add `resolve_backup()` helper in `src/commands/mod.rs` that implements the precedence chain: `--no-backup` (CLI) > `ATOMWRITE_BACKUP=0` (env) > default `true`. Keep `keep_backup: false` unchanged — the backup serves as a temporary safety net, not permanent storage. The backup is created BEFORE the atomic write and auto-deleted after success. 4 non-content structs (`DeleteArgs`, `MoveArgs`, `CopyArgs`, `RollbackArgs`) keep `backup: false` because they do not overwrite file content via stdin.

- **Consequences**:
  - **+** Automatic safety net for ALL content mutations — agents that confuse `write` with `edit` can recover via `rollback --latest`.
  - **+** ~1ms overhead on SSD (`fs::copy` + `fs::remove_file` in the success path). Zero overhead when target does not exist (new file, no backup needed).
  - **+** `--no-backup` provides explicit opt-out for performance-critical pipelines.
  - **+** `ATOMWRITE_BACKUP=0` provides global opt-out for CI environments that manage their own backup strategy.
  - **+** Aligns atomwrite with the "secure-by-default" principle — protection is automatic, not opt-in.
  - **-** (acceptable) The `--backup` flag becomes redundant since backup is now the default. The flag is preserved for backward compatibility and explicitness.
  - **-** (acceptable) Existing tests that tested `--require-backup` without `--backup` now need `--no-backup` to trigger the guard, because `--require-backup` checks whether `backup` is true, and the new default makes it always true.

- **Alternatives considered**:
  1. **Keep default `false` with better documentation.** Rejected: documentation does not protect LLM agents that make semantic errors. The 2026-06-15 incident proves that opt-in safety is insufficient — the agent had access to the documentation and still used `write` instead of `edit`.
  2. **Only change `WriteArgs`.** Rejected: all 9 content-mutating commands share the same risk profile. An agent can destroy data via `edit --range 1:9999` or `replace` with an empty replacement just as easily as via `write`. Partial coverage creates a false sense of safety.
  3. **Change `keep_backup` to `true` (permanent backups).** Rejected: permanent backups accumulate disk usage without bound. The `--retention N` mechanism already exists for users who want persistent backups. The temporary backup (created and deleted in the same syscall path) provides crash recovery without disk bloat.

- **Trigger to revisit**: If the ~1ms overhead causes measurable regression in batch operations (>10,000 files), add a `--fast` mode that disables backup. If `ATOMWRITE_BACKUP=0` adoption in CI exceeds 50% of invocations, reconsider whether the default should be environment-aware.
