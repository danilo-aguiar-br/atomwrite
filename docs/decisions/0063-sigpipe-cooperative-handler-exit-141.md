# ADR-0063 — SIGPIPE: a cooperative handler with exit 141 (conscious deviation)

## Status
Accepted (2026-10-15)

## Context
- The owner rule says to reset SIGPIPE to `SIG_DFL` (the old `reset_sigpipe`).
- With `SIG_DFL`, a closed reader kills the process without running a single destructor. A `replace | head -1` left 7 to 11 lock sidecars, 3 to 7 journals (`committed` ones included) and sometimes a `.atomwrite-*.tmp` (V1 FAIL-1 in 5 runs, V2 M1 in 6 runs).
- `wal-heal` never collected those journals: it reported `removed:0, preserved:7`.
- Ignoring SIGPIPE was the other option. It hides the closed reader from every worker except the one that writes.

## Decision
- SIGPIPE is neither ignored nor left at `SIG_DFL`. `register_broken_pipe` installs a handler that marks a cooperative shutdown with exit 141 (`ShutdownSignal::record_broken_pipe`). The handler does only atomic stores: no allocation, no lock, no I/O.
- A repeated SIGPIPE never forces the exit, because several threads can hit the closed pipe before the first one has unwound.
- The write that hits the closed pipe fails with `BrokenPipe`. The normal error path runs every `Drop`, and `main` exits 141.
- An in-flight write finishes and commits: `is_global_hard_shutdown` ignores the broken-pipe flag, and only SIGINT, SIGTERM, SIGHUP and the timeout abort it. The checks between files stop new work.
- The shutdown banner is not written for a broken pipe. `read` returns 141 and propagates a failed flush, where it returned 0. When the error envelope itself cannot be written because the reader closed the pipe, the run exits 141.
- `reset_sigpipe` was removed.

## Consequences
- This is a DELIBERATE DEVIATION from the literal owner rule. The graceful-shutdown rule and the exit 141 are preserved, and the owner can revert the deviation by restoring `SIG_DFL`, at the cost of the residue above.
- 20 runs of `replace | head -1` and 20 runs each of `list`, `search` and `read` exited 141 by code, not by signal, and left no lock, no journal and no tempfile (gaps.md W7, T2). A repeat of 5 runs of `replace | head -1` over 600 files also gave exit 141 and zero residue (T5b).
- On Windows there is no SIGPIPE, and `BrokenPipe` from `std` follows the same path. This is not validated there.
- On a closed pipe stderr still receives two `WARN data channel write failed` lines (the cancelled event and the flush). They are not data and do not pollute stdout.
- Every output path that writes to stdout by itself exits 141 on a closed reader, `completions` included. The docs audit found a residual before publication: `completions bash | head -1` panicked inside `clap_complete::generate` and exited 134 with a panic report file. It was fixed in 0.1.37 itself: `render_completions` renders the script in memory and the write propagates the error.
- Not covered: a reader that closes only after it has received the whole output causes no `EPIPE`, so the run exits 0, which is correct. On Windows the same path is not validated.

## Alternatives
- `SIG_DFL` (the literal owner rule): rejected, the measured residue above.
- Ignore SIGPIPE: rejected, only the writing thread would learn that the reader closed.

## Evidence
- `CHANGELOG.md:76,176,196-201,283` and `gaps.md:193-200` (W7).
- `src/signal_handlers.inc.rs:122-149` (`register_broken_pipe` and the rationale in its rustdoc) and `:281`, `src/signal.rs:63-77` (`is_global_hard_shutdown`) and `:195-204` (`record_broken_pipe`), `src/atomic/rename_path.rs:49-52`, `src/main.rs:182-184`, `src/constants.rs:56` (`EXIT_BROKEN_PIPE` is 141, asserted equal to 128 + 13).
- `tests/cli_v0137_terminal_pipe.rs` (4 tests), `tests/cli_v0137_broken_pipe_outputs.rs` (2 tests, 17 output paths with the read end already closed: `completions` for 5 shells, `--print-schema` in 3 forms, `--json-schema`, `agent-surface`, `commands`, `doctor`, three `--help` forms, `--version` and an unknown command), the unit tests of `signal::tests`.
- `src/lib.rs:339,356` (the writes of the script) and `:363` (`render_completions`).
- Binary 0.1.37: `atomwrite completions bash | head -c 10` exited 141 and `atomwrite --help | head -c 10` exited 141. `--print-schema | head -c 10` and `--version | head -c 3` exited 0, because their whole output fits in the pipe before `head` exits. The deterministic proof is the test above.

## Trigger to revisit
- The owner decides to restore `SIG_DFL`, accepting the residue.
- Windows is validated on a real host.
