# ADR-0053: Clap rules audit exceptions (v0.1.32)

## Status

Accepted

## Context

Audit against `rules_rust_cli_com_clap` / GraphRAG memory `rules-rust-cli-com-clap` closed most OBRIGATÓRIO gaps (thin `main`, explicit clap features, `author`, `value_hint`, `ArgAction::SetTrue`, `help_heading` on Backup, dedicated `debug_assert` test, CONTRIBUTING/CODE_OF_CONDUCT).

Two intentional deviations remain.

## Decision

### 1. `BackupOpts.backup: Option<bool>` (tri-state)

Rule text forbids `Option<bool>` for boolean flags. Atomwrite needs three states:

| CLI | Meaning |
|-----|---------|
| (absent) | Use `.atomwrite.toml` `[defaults].backup` or built-in default `true` |
| `--backup` | Force on (`Some(true)` via `ArgAction::SetTrue`) |
| `--no_backup` | Force off (`conflicts_with = "backup"`) |

A plain `bool` cannot represent “defer to config”. Documented on the field; covered by `tests/cli_v0128_backup_matrix.rs`.

### 2. `runtime::prescan_json_schema` uses `std::env::args` before Clap

Rule text forbids manual argv parsing before Clap. Exception: agents must dump JSON Schema for a subcommand via `--json-schema` even when remaining argv would fail required-arg validation. Normal command paths still use `Cli::try_parse()` exclusively. Documented in `src/runtime.rs`.

## Consequences

- Future audits must treat these as **documented exceptions**, not silent violations.
- Prefer not to expand pre-Clap argv scanning beyond schema prescan.
- If Clap gains first-class “schema dump without required args”, remove the prescan path.

## Related

- `src/cli_args.rs` (`BackupOpts`)
- `src/runtime.rs` (`prescan_json_schema`)
- `tests/cli_clap_debug_assert.rs`
- `xtask` man page generation via `clap_mangen`
