# ADR-0044: --old-file/--new-file for edit command

- **Status**: Accepted
- **Date**: 2026-06-19
- **Context**: `edit --old`/`--new` accept content only via CLI arguments. Shell expansion `$(cat file)` injects content into argv before `execve(2)`. Linux kernel ARG_MAX is 2,097,152 bytes for argv+envp combined; effective limit for a single argument is ~131 KB. Content above this limit causes E2BIG (errno 7) — the atomwrite process never starts and the agent receives exit 126 from the shell with no JSON envelope. The workaround `edit --multi` with NDJSON stdin exists but requires JSON encoding that is error-prone. Agents default to `$(cat)` for simplicity and hit E2BIG silently on large files. The documentation does not mention ARG_MAX or suggest `--multi` as an alternative.

- **Decision**: Add `--old-file <PATH>` and `--new-file <PATH>` to `EditArgs` as alternatives to `--old` and `--new`. Files are read inside the atomwrite process, bypassing shell expansion and ARG_MAX. Use clap `conflicts_with` to prevent mixing `--old` with `--old-file` (Silent Argument Discard prevention per rules-rust-cli-stdin-stdout-silent-discard). Validate paths against workspace jail. Add `source: "arg"|"file"` field to `PairResult` for traceability. Add runtime validation in `resolve_edit_pairs()` to reject cross-mixing of `--old` with `--new-file` or `--old-file` with `--new` — exit 65 (`INVALID_INPUT`) with message 'cannot mix --old with --new-file or --old-file with --new; use both from the same source'. Add `strip_file_trailing_newline()` to strip exactly one trailing newline (`\n` or `\r\n`) from file content for parity with argv behavior — files created by `echo` include a trailing newline that argv values never have.

- **Consequences**:
  - **+** Eliminates ARG_MAX limit for agents — content of any size can be used.
  - **+** Zero shell expansion issues ($, backticks, quotes, etc.).
  - **+** `conflicts_with` prevents silent discard of `--old` when `--old-file` is present.
  - **+** `PairResult.source` provides traceability of content origin.
  - **+** Cross-mixing guard catches the case clap `conflicts_with` cannot: `--old` paired with `--new-file` (or vice versa).
  - **+** Trailing newline stripping ensures `--old-file old.txt` matches the same content as `--old "text"` even when `old.txt` was created by `echo "text" > old.txt`.
  - **-** (acceptable) Agents must write temporary files before edit (two-step workflow).
  - **-** (acceptable) Cannot mix `--old` and `--old-file` in same invocation (deliberate safety tradeoff).

- **Alternatives considered**:
  1. **Read content from stdin with a protocol separator.** Rejected: conflicts with `--multi` and `--between` which already consume stdin.
  2. **Increase ARG_MAX via sysctl.** Rejected: requires root, not portable, doesn't solve the fundamental design issue.
  3. **Only document `--multi` as workaround.** Rejected: JSON encoding is error-prone for agents; `--old-file` is simpler.

- **Trigger to revisit**: If stdin multiplexing becomes viable (e.g., via named pipes or fd passing).
