# ADR-0025: --positions is opt-in and limited to --query/--tree modes

- **Status**: Accepted
- **Date**: 2026-06-07
- **Context**: v14 `query` has four modes: `--kinds` (count kinds), `--query <KIND>` (emit matching nodes), `--tree` (dump all nodes), and `--positions` (add `start_line`, `start_column`, `byte_offset` to each node). The user might want positions in any mode, but emitting them for `--kinds` would be silly (kinds are counts, not positions).
- **Decision**: `--positions` is valid in `--query <KIND>` and `--tree` modes. It is silently ignored in `--kinds` (with a stderr warning in `cfg(debug_assertions)` builds). Each NDJSON line for a matched node gains a `positions` field when `--positions` is on.
- **Consequences**:
  - **+** NDJSON output is concise by default.
  - **+** Power users can get byte ranges for further processing (e.g. extracting a function body to a separate file).
  - **+** Wire format is stable: `positions` field is present-when-enabled, absent-otherwise.
  - **-** The "silently ignored in --kinds" behavior is surprising; we add a debug warning.
  - **-** Users who want positions in `--kinds` output have to switch to `--query <KIND> --positions` and filter.
- **Alternatives considered**:
  1. Always emit positions. Rejected: bloats output for the 95% case where users just want kind names.
  2. Separate `--positions` and `--kinds --positions` as two flags. Rejected: combinatorial explosion of flags.
  3. Make `--positions` work everywhere, even `--kinds`. Rejected: `--kinds` output is `{kind, count}` tuples; positions make no sense.
- **Trigger to revisit**: If users ask for "give me a histogram of functions by file", we can add a dedicated `--function-locations` mode.
