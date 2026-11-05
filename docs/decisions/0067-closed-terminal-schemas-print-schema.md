# ADR-0067 — Closed terminal schemas and a faithful `--print-schema`

## Status
Accepted (2026-10-15), name list superseded by ADR-0078 (2026-10-31)

## Context
- The published schemas described a struct the verb did not serialize. `search-summary` and `replace-summary` shared one type, so the schema of a terminal summary was the schema of its event.
- `diff` was an `anyOf` with no `type` and no `required` (V1 FAIL-8), so a validator accepted objects that no verb can emit.
- The `config` schema required a `warnings` key that `config path`, `config set` and `config unset` do not emit (V1 FAIL-2).
- `docs/schemas/` files for some events had `additionalProperties: false` and no verdict keys, so a real event would have failed validation.

## Decision
- Each `--print-schema NAME` is generated from the Rust type the verb really serializes (`schemars::schema_for!` per arm in `src/schema_dispatch.inc.rs`), and `command_schema_name` maps every subcommand to its name so `--json-schema` and `--print-schema NAME` answer from one table.
- These terminal events are closed (`additionalProperties: false`, a non-null `required` with the keys that are always present): `search-summary`, `replace-summary`, `count`, `count-by-ext`, `count-by-size`, `backup-summary`, `list-summary`, `config`, `batch` and `doctor` (`CLOSED_TERMINAL_SCHEMAS`).
- `diff` is `type: "object"` with a `oneOf` of four closed branches (`change`, `summary`, `unified`, `stat`), each with `required`.
- The keys the writer stamps (`ok`, `code`, `exit`, `target_resolved`, `target_source`) are optional properties of every terminal schema.
- A summary that can end in its own failure (`search-summary`, `replace-summary`, `batch`) adds the rule "`ok: false` requires `code` and `exit`" (`FAILURE_REQUIRED`). The first version demanded `error_class`, `retryable`, `suggestion_lang` and `message` as well; those four left the default failure envelope and come back only under `-v`, so a schema that still demanded them rejected every failure the build really writes. `doctor` carries the detail as optional.
- Six new names, `backup-summary`, `count-by-ext`, `count-by-size`, `list-summary`, `replace-summary` and `search-summary`, bring the list to 58. An unknown name exits 65 with the name list.

## Consequences
- `atomwrite --print-schema` lists 55 names, and `docs/schemas/` has 54 files, 31 of them regenerated from the binary and 4 completed with the stamped keys.
- Two naming sets coexist: `--print-schema` serves short names (`read`, `edit`) while `docs/schemas/` publishes descriptive ones (`read-output`, `edit-output`). This is a declared residual risk (CHANGELOG, Security).
- Validation in the tests is a type-only check of `required` and names. No JSON Schema validator is a dependency of the product. A test validates 12 real invocations (failures of `search` and `replace` included), and a negative test proves the validator rejects a missing key, an undeclared key and a failure without `code`.
- `cancelled` carries neither `ok: false` nor `code`, by design of `docs/schemas/README.md` (gaps.md W44).

## Alternatives
- Keep one shared type for `search-summary` and `replace-summary`: rejected, their keys differ.
- Leave the schemas open (`additionalProperties` unset): rejected, a validator then accepts any object.

## Evidence
- `CHANGELOG.md:63-64,160,225-227,235` and `gaps.md:284-285` (W39), `gaps.md:305-307` (W48).
- `src/schema_terminal.inc.rs:11-22` (`CLOSED_TERMINAL_SCHEMAS`), `:25-41` (`never_emitted`, `always_emitted`), `:44-67` (`FAILURE_REQUIRED`, `ends_failure_with_summary`), `src/schema_dispatch.inc.rs:11-70` (`SCHEMA_NAMES`), `:219-270` (`command_schema_name`), `:272-310` (`writer_stamped_properties`, `with_envelope_keys`).
- `tests/cli_v0137_terminal_schema.rs` (9 tests).
- Binary 0.1.37: `atomwrite --print-schema` printed 55 NDJSON lines, and `agent-surface` publishes the same list in `discovery.schema_names`.

## Trigger to revisit
- A JSON Schema validator becomes a dependency, so the tests can validate the full schema and not only `required` and names.
- The two naming sets are unified.
