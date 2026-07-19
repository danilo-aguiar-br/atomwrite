# ADR-0048: unified BackupOpts and single resolve_backup() resolver

> **Historical (pre-0.1.35):** product `ATOMWRITE_*` / env knobs described below are **superseded**. Runtime config is CLI flags + XDG `config.toml` / `atomwrite set|get` only.


- **Status**: Accepted
- **Date**: 2026-07-06
- **Context**: Backup-related flags (`--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`) were hand-declared separately in each of 15 mutating subcommand structs (`write`, `edit`, `edit-loop`, `replace`, `transform`, `scope`, `apply`, `set`, `del`, `case`, `batch`, `delete`, `move`, `copy`, `rollback`), with divergent presence and defaults across structs (GAP-CLI-SURFACE-DRIFT). This caused `replace --retention` and `delete --no-backup` to return exit 2 `ARGUMENT_PARSE_ERROR` because the flag was missing or declared with an incompatible type in those specific structs. `--backup` and `--no-backup` being passed together silently let the last flag win instead of failing fast. `retention: 5` was hardcoded at 9 call sites instead of flowing from a single source of truth, and `.atomwrite.toml` `[defaults]` `backup`/`retention` keys were parsed but never consulted (GAP-CONFIG-DEFAULTS-DEAD) — see ADR-0049.
- **Decision**: Extract a single `BackupOpts` struct (`--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`) flattened via `#[command(flatten)]` into all 15 mutating subcommands. `--backup` and `--no-backup` are now mutually exclusive via clap `conflicts_with` (exit 2) instead of silently letting the last flag win. A single `resolve_backup(opts: &BackupOpts, defaults: &DefaultsSection) -> ResolvedBackup` function centralizes the resolution with precedence: `ATOMWRITE_BACKUP` env var > CLI flags (`--backup`/`--no-backup`/`--retention`) > `.atomwrite.toml` `[defaults]` > built-in default (`true` / `5`). Three documented exceptions to the unified default-true contract:
  1. **`rollback`** keeps its pre-rollback safety snapshot opt-in via explicit `--backup` (it already reads from an existing backup, so a default-true pre-step would be redundant).
  2. **`delete --keep-backup`** is redundant because deletion backups are always preserved and never auto-removed; passing it now surfaces a `warnings` field in the NDJSON envelope instead of a silent no-op.
  3. **`move`/`copy`** overwriting an existing destination now require `--force` OR an explicit `--backup` — previously the two subcommands enforced this inconsistently.
- **Consequences**:
  - **+** `replace --retention` and `delete --no-backup` now parse successfully across all 15 subcommands.
  - **+** `--backup`/`--no-backup` conflict fails fast at parse time (exit 2) instead of silently picking the last flag.
  - **+** Zero hardcoded `retention: 5` literals remain; `Default` impls in `config.rs` and `atomic.rs` read `constants::DEFAULT_BACKUP_RETENTION`.
  - **+** `batch --retention <N>` and `batch --backup` are now effective end-to-end, including the transactional pre-backup step and the `delete` operation inside a batch.
  - **-** `delete`/`move`/`copy` now have a BREAKING behavior change: `delete` creates a backup by default (was opt-in), and `move`/`copy` overwrite requires `--force` or `--backup`.
- **Alternatives considered**:
  1. **Keep per-struct backup flags, fix each divergence individually.** Rejected: the drift would recur every time a new mutating subcommand is added; a single flattened struct is the only way to guarantee parity by construction.
  2. **Make `--backup`/`--no-backup` last-flag-wins (status quo).** Rejected: silent precedence is a footgun in agent pipelines that may pass both flags across composed command lines.
- **Trigger to revisit**: If a 16th mutating subcommand is added, confirm it flattens `BackupOpts` rather than re-declaring the flags by hand.
