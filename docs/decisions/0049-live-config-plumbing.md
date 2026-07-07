# ADR-0049: live config plumbing for `.atomwrite.toml` `[defaults]`

- **Status**: Accepted
- **Date**: 2026-07-06
- **Context**: `.atomwrite.toml` `[defaults]` `backup`/`retention` keys were parsed by `load_config` but never actually consulted by any mutating command (GAP-CONFIG-DEFAULTS-DEAD) — the resolver that existed at the time took only `(backup: bool, no_backup: bool)` as arguments and had no path to reach the parsed config values. Users who set `[defaults]` in their project config saw it silently ignored.
- **Decision**: `load_config` is now called exactly once, in `lib.rs::run()`, and its resulting `DefaultsSection` is threaded through and propagated to every mutating command handler. `resolve_backup()` (ADR-0048) now takes `&DefaultsSection` as its second argument and consults `defaults.backup`/`defaults.retention` as the third precedence tier: `ATOMWRITE_BACKUP` env var > CLI flags (`--backup`/`--no-backup`/`--retention`) > `.atomwrite.toml` `[defaults]` > built-in default (`true` / `5`).
- **Consequences**:
  - **+** `.atomwrite.toml` `[defaults]` `backup`/`retention` are now effective across every mutating subcommand, closing GAP-CONFIG-DEFAULTS-DEAD.
  - **+** Config is loaded once per invocation instead of being re-parsed or ignored per-handler, avoiding redundant I/O and drift between handlers.
  - **-** Handlers that mutate content now carry an extra `&DefaultsSection` parameter; this is a one-time signature change absorbed by the `BackupOpts` refactor (ADR-0048) in the same release.
- **Alternatives considered**:
  1. **Load config independently inside each handler.** Rejected: duplicates I/O and risks re-introducing per-handler drift, the exact failure mode this ADR closes.
  2. **Keep `[defaults]` parsed-but-unused and document it as reserved for a future release.** Rejected: the config already documented the keys as effective; leaving them dead is a documentation/behavior mismatch, not a deferral.
- **Trigger to revisit**: If `[defaults]` grows additional keys beyond `backup`/`retention`, confirm `DefaultsSection` and its propagation path are extended consistently rather than adding a parallel resolution path.
