# ADR-0058 — The `config` verb and typed tuning keys

## Status
Accepted (2026-10-15)

## Context
- Tuning values (fuzzy thresholds, lock timeout, retry shape, search budget, extract limits) were literals in code, so changing one meant editing the source (gaps.md W13).
- The only other way to change them was to edit `config.toml` by hand, which could corrupt the file and left the value unchecked until the next run.
- The v0.1.37 hardcode rule (ADR-0068) moved every literal into `constants*`, so a persistent way to override them had to exist outside the code.

## Decision
- Add the 47th subcommand, `config path|list|get|set|unset`, in `src/commands/config_cmd.rs`.
- One table of 37 typed keys feeds `list`, `get`, `set` and `unset`: `fuzzy.mode`, `fuzzy.threshold`, `fuzzy.threshold_aggressive`, `fuzzy.threshold_context`, `fuzzy.threshold_jw`, `fuzzy.soft_floor_delta`, `fuzzy.max_content_bytes`, `write.confirm_large_bytes`, `log.file`, `log.rotation`, `storage.home`, `locale`, `concurrency.max_threads`, `concurrency.ram_fraction_percent`, `search.max_inflight_bytes`, `lock.acquire_timeout_ms`, `retry.base_ms`, `retry.max_ms`, `retry.max_elapsed_ms`, `extract.max_sample_rows`, `extract.max_text_chars`, `extract.max_sheets` and `extract.max_slides`.
- Ranges and relations live in `src/constants_config.inc.rs` and are enforced by `config set` and by the startup load alike (for example `concurrency.max_threads` 1 to 256, `search.max_inflight_bytes` 4 MiB to 1 GiB, `retry.base_ms` not above `retry.max_ms`).
- `--scope global|workspace` is mandatory on `path`, `set` and `unset` and has no default: without it the verb exits 2 and touches nothing. `get` and `list` take no `--scope` and report the value with its `origin` (`default`, `global` or `workspace`).
- `log.file`, `log.rotation`, `storage.home` and `locale` are global-only. The workspace scope rejects them with exit 65, and an unknown key or an out-of-range value also exits 65 and writes nothing.
- `config set` reads the file, changes the key keeping the comments, validates the whole file and only then writes it through `atomic_write` (global file mode 0600, no backup).
- The terminal event is `type: "config"` and carries `target_resolved` and `target_source: "argv"`. The global XDG file lives outside the jail, so the dispatch has a stamp branch of its own for it.
- `config` is answered before the config load, so a broken `config.toml` does not block it. A file that is not valid TOML is read with a warning and never rewritten (`set` and `unset` exit 78 with the path).
- The global `config.toml` and the workspace `.atomwrite.toml` are merged key by key, with the workspace winning. Before this release a local file replaced the global one whole. `--config PATH` still replaces both on `get` and `list`.

## Consequences
- One source of truth for keys, ranges and defaults. A new tuning value is one row in the table and one constant.
- `defaults.line_ending`, `defaults.backup`, `defaults.retention` and `defaults.max_filesize` are config keys and change the default of every mutating verb (gaps.md W14, P3).
- `storage.home` is circular (it decides where the config lives), so `set` writes it straight to the bootstrap file and warns in the envelope where the global config moved.
- `locale --set` now writes its preference through `atomic_write` (mode 0600) instead of a plain write.
- The two extract limits that had flags keep working: `--max-sample-rows` and `--max-text-chars` still win over the config.

## Alternatives
- Keep the values as literals: rejected, a tuning change needed a rebuild and the hardcode rule left no legal place for them.
- Tell users to edit `config.toml` by hand: rejected, nothing checked the value, and a typo surfaced only on the next run.
- A default scope for `set`: rejected, a silent global write is the wrong default for a tool that agents call without a terminal.
- A `config doctor` verb: not built, listed as P3 in gaps.md.

## Evidence
- `CHANGELOG.md:48-57,134-135,222-224` (Added and Changed of 0.1.37) and `gaps.md:235-239` (W13), `gaps.md:278-280` (W37), `gaps.md:308` (W49).
- `src/commands/config_cmd.rs:277` (global-only check), `src/commands/config_cmd_keys.inc.rs`, `src/commands/config_cmd_write.inc.rs`, `src/config_tuning.inc.rs`, `src/constants_config.inc.rs`.
- `src/lib_dispatch.inc.rs:206-222` (`run_config_verb`, answered before the config load) and `src/lib.rs:201`.
- `tests/cli_v0137_config_verb.rs` (23 tests) and `docs/schemas/config.schema.json`.
- Binary 0.1.37 in a bench with `HOME` and `XDG_*` inside the scratchpad: `config get retry.max_ms` printed `value` 400, `default` 400, `origin` `default` (exit 0). `config set fuzzy.mode auto` without `--scope` exited 2 with `ARGUMENT_PARSE_ERROR`. `config set search.max_inflight_bytes 1048576 --scope workspace` exited 65 with "must be 4194304..=1073741824". `config set log.file true --scope workspace` exited 65. `config set fuzzy.max_content_bytes 1000000 --scope global` exited 0 with `target_source` `argv`, and the file mode was 600.

## Trigger to revisit
- A key must be added that no scope can hold, or a third scope is needed.
- `defaults.line_ending` or `defaults.max_filesize` gets a consumer and becomes a key.
