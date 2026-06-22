# ADR-0045: Actionable suggestions for clap parse errors

- **Status**: Accepted
- **Date**: 2026-06-21
- **Context**: `ARGUMENT_PARSE_ERROR` (exit 2) was the only error code without a context-aware `suggestion` field in the JSON error envelope. When clap rejected arguments involving `--old`/`--new` (e.g., content with leading hyphens passed via shell expansion), the suggestion field was either `null` or contained clap's generic tip (`to pass '-' as a value, use '-- -'`). This tip is incorrect for multi-pair edit mode (the `--` separator terminates all flag parsing, breaking subsequent `--old`/`--new` pairs) and does not mention `--old-file`/`--new-file` — the safe alternative that reads content from files inside the atomwrite process, bypassing shell expansion entirely. In LLM agent pipelines, the cascading failure was: exit 2 masked by jaq pipe → agent retry loops with ineffective workarounds → agent falls back to truncating `write` → catastrophic data loss (documented in ADR-0041). The `--old-file`/`--new-file` flags (ADR-0044, v0.1.23) solve the root cause but were not discoverable at the moment of error.

- **Decision**: Replace the direct call to `extract_clap_tip()` with `enrich_clap_suggestion()` in the clap error handling path (main.rs:67). The new function applies a heuristic: if the error message mentions `--old`, `--new`, `--after-match`, `--before-match`, `--between`, or contains patterns indicating a hyphen-value parsing failure (`wasn't expected`, `unexpected argument`, tip mentioning `'--'`), the suggestion is enriched with actionable guidance pointing to `--old-file`/`--new-file`. The original clap tip is preserved as a parenthetical suffix. Additionally, the help text for `--old` and `--new` in `EditArgs` now includes a hint: "for content >1KB or with special chars, prefer --old-file/--new-file".

- **Consequences**:
  - **+** Agents receive actionable guidance on the FIRST error, eliminating retry loops.
  - **+** Consistency with all other 25 error codes that already have context-aware `suggestion`.
  - **+** Eliminates the cascading failure chain to data loss documented in ADR-0041.
  - **+** Zero schema change — `suggestion` field already exists in `error-output.schema.json`.
  - **+** Help text provides discoverability even before an error occurs.
  - **-** (acceptable) False positives possible if a future subcommand introduces a flag named `--old` or `--new`. Likelihood is extremely low given the existing command vocabulary.
  - **-** (acceptable) The heuristic matches on string content of the error message, which could change across clap versions. Mitigation: regression tests verify the suggestion content.

- **Alternatives considered**:
  1. **Modify clap upstream to support custom suggestion injection.** Rejected: clap v4 has no API for per-field error suggestions; the `Error` type is opaque after generation.
  2. **Document-only approach (update AGENTS.md).** Rejected: agents do not read documentation at the moment of error; they consume the `suggestion` field programmatically.
  3. **Always inject the --old-file suggestion regardless of error context.** Rejected: would produce irrelevant suggestions for errors in subcommands that don't have `--old`/`--new` (e.g., `search`, `hash`).

- **Trigger to revisit**: If clap v5 introduces a `suggestion_fn` callback or per-argument error customization API.
