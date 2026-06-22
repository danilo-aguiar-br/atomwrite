# ADR-0046: diff resolve-first retrofit

- **Status**: Accepted
- **Date**: 2026-06-21
- **Context**: The `diff` subcommand was implemented before ADR-0027 (resolve-first convention) was established in v0.1.18. When ADR-0027 was adopted, the following commands were retrofitted: write, edit, copy, apply, move, rollback, set, del, case, replace. The `diff` command was omitted from this list because it was considered read-only and non-mutating. However, the resolve-first convention applies to ALL commands that perform file I/O — not just writes — because: (1) relative paths must resolve against `--workspace` for API consistency, (2) paths escaping the workspace jail must be rejected with `WORKSPACE_JAIL` (exit 126) regardless of read/write intent, and (3) agents expect uniform path semantics across all subcommands. The bug manifested as `FILE_NOT_FOUND` (exit 4) when calling `diff a.txt b.txt` with `--workspace /path/to/dir` even though both files existed inside the workspace.

- **Decision**: Add `global.resolve_workspace()` + `path_safety::validate_path()` for both `file_a` and `file_b` at the top of `cmd_diff()`, before any call to `read_file_string()`. This is the same 3-line pattern used by all other commands since v0.1.18.

- **Consequences**:
  - **+** `diff` now resolves relative paths against `--workspace`, consistent with all other subcommands.
  - **+** Paths escaping the workspace jail are rejected with `WORKSPACE_JAIL` (exit 126) instead of confusing `FILE_NOT_FOUND`.
  - **+** Agents no longer need special path-prefixing logic for `diff`.
  - **+** Full compliance with ADR-0027 and ADR-0030.
  - **-** (none) The change is purely additive and backwards-compatible: absolute paths inside the workspace continue to work.

- **Alternatives considered**:
  1. **Leave diff as-is and document the exception.** Rejected: inconsistency is a recurring source of agent errors and the fix is trivial (3 lines).
  2. **Only resolve but skip jail validation for read-only commands.** Rejected: jail validation prevents symlink-based path escape attacks even in read-only mode.

- **Trigger to revisit**: If a new read-only subcommand is added, apply the same resolve-first pattern from the first commit (lesson learned from this gap).
