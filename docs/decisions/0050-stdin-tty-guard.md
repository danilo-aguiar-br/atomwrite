# ADR-0050: stdin-tty guard for edit's stdin-consuming modes

- **Status**: Accepted
- **Date**: 2026-07-06
- **Context**: `edit`'s stdin-consuming modes (`--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between`, and `--multi`) read new content from stdin. When stdin was a terminal (interactive session, no pipe or redirect), these modes blocked indefinitely waiting for input the user never intended to type, previously requiring an external `timeout` wrapper and exiting 143 (SIGTERM) instead of failing with an actionable error.
- **Decision**: `main.rs` computes `stdin_is_tty = stdin.is_terminal()` once, using the standard library's `IsTerminal` trait (stable since Rust 1.70), and passes it as a plain `bool` through `atomwrite::run()` down to `cmd_edit` and its stdin-consuming helpers (`edit_by_line`, `edit_by_marker`, and the individual mode handlers), which call the shared `read_stdin_text_guarded(stdin, max_size, stdin_is_tty, mode_name)` helper. When `stdin_is_tty` is `true`, the guard returns `AtomwriteError::InvalidInput` (exit 65) with an actionable `suggestion` instead of blocking. `is_terminal()` itself is called exactly once, in `main.rs`; `edit.rs` and its helpers only ever receive the already-computed `bool`, never call `is_terminal()` directly — this keeps the stdin-consuming code path testable without a real terminal (tests can pass `stdin_is_tty: false` with a `Cursor` as stdin).
- **Consequences**:
  - **+** Stdin-consuming edit modes fail fast (exit 65) with a `suggestion` when invoked interactively, instead of hanging and requiring an external `timeout` + SIGTERM (exit 143).
  - **+** `edit.rs` remains testable with `Cursor`-backed stdin in unit/integration tests, since `is_terminal()` is never called inside it.
  - **-** `cmd_edit` and its helpers carry an additional `stdin_is_tty: bool` parameter threaded from `main.rs` through `lib.rs::run()`.
- **Alternatives considered**:
  1. **Call `is_terminal()` directly inside `edit.rs`.** Rejected: couples the command logic to a real stdin handle and breaks `Cursor`-based tests that don't back a real terminal.
  2. **Rely on an external `timeout` wrapper as the only safeguard (status quo).** Rejected: silently blocking with no actionable error is a poor agent-pipeline experience; the exit 143 signature gives no hint about the root cause.
  3. **Prompt interactively when a terminal is detected.** Rejected: `atomwrite` is designed for non-interactive/agent use; an interactive prompt would reintroduce blocking behavior in a different form.
- **Trigger to revisit**: If a non-`edit` subcommand gains a new stdin-consuming mode, propagate `stdin_is_tty` to it the same way rather than calling `is_terminal()` locally.
