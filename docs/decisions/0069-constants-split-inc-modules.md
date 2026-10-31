# ADR-0069 — Split `constants.rs` and the large modules into `.inc.rs` parts

## Status
Accepted (2026-10-15)

## Context
- `src/constants.rs` had grown past a readable size, and the hardcode guard (ADR-0068) made it the only legal home of every literal in production code.
- `output_shape.inc.rs` was 1951 lines, and `run_replace`, `cmd_search` and `run` were long. No production source file may be longer than 500 lines (CHANGELOG R40). The `tests.inc.rs` test modules are outside that limit.

## Decision
- Split `constants.rs` by responsibility into three `include!` modules: `constants_concurrency.inc.rs` (threads, retry, search budget), `constants_config.inc.rs` (ranges and relations of the config keys) and `constants_literals.inc.rs` (the named literals moved out of production code, starting with time units and the calendar conversion).
- Give `config`, `fuzzy`, `ndjson` and `replace` their own `.inc.rs` files (`config_tuning.inc.rs`, `config_cmd_keys.inc.rs`, `config_cmd_write.inc.rs`, `part3.inc.rs`, `replace_file.inc.rs`), split `output_shape.inc.rs` into nine files, and move the closing verdict of `replace` to `replace_finish.inc.rs`.
- Public paths do not change: `crate::constants::NAME` still resolves, because the parts are included into the same module.
- Compile-time asserts sit next to the values (for example `MAX_AUTO_THREADS <= CONCURRENCY_MAX_THREADS_CEILING`).

## Consequences
- No behaviour change.
- The hardcode guard skips `constants*` files by name, so a new constants module must keep the `constants` prefix.
- The literals of the inventory (sidecar and backup names with one `BACKUP_INFIX`, time units, risk thresholds, buffer and block sizes, `/proc` and container probe paths, errno values, the BOM, thread names and the calendar constants of `time_fmt.rs`) live in `constants*`.
- `list` no longer carries its own copy of the calendar algorithm: it calls `atomic::epoch_to_utc`.
- `locale.rs` was split earlier by the same pattern (`locale_persist.inc.rs`).
- `src/constants.rs` still has 447 lines and `CANONICAL_SUBCOMMAND_COUNT` (47) lives there.

## Alternatives
- Keep one `constants.rs`: rejected, it was too large to read and review.
- Turn each part into a separate `mod`: not done, the parts are included to keep the existing `crate::constants::NAME` paths.

## Evidence
- `CHANGELOG.md:131-133,186-188` (Changed) and `gaps.md:235-236` (W13).
- `src/constants.rs` (447 lines) with `src/constants_concurrency.inc.rs` (105 lines), `src/constants_config.inc.rs` (71 lines) and `src/constants_literals.inc.rs` (271 lines); `src/config_tuning.inc.rs` (369 lines), `src/commands/config_cmd_keys.inc.rs` (451 lines), `src/commands/config_cmd_write.inc.rs` (317 lines).
- Correction, 2026-10-28: the sizes above are the ones measured after the 0.1.37 key table grew to 35 keys; the earlier figures in this list were stale.
- `src/constants_config.inc.rs:66` and `:62-70` (asserts), `tests/cli_v0137_hardcode_guard.rs:8-10` (skips `constants*`).

## Trigger to revisit
- A constants part grows past 500 lines.
- The guard changes how it recognizes the constants files.
