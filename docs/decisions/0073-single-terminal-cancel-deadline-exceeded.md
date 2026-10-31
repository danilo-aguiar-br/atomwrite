# ADR-0073 — One terminal cancel point and `deadline_exceeded` (A1, A3)

## Status
Accepted (2026-10-15). Amends ADR-0054, which gave exit 143 as the exit of the cooperative cancel and described the 124 deadline as a plain error envelope.

## Context
- Each scanning verb chose by itself between an envelope, silence and a partial summary when it was interrupted.
- A consumer that read the last line of a cancelled `search` took it as a final result.
- A consumer could not tell an elapsed budget from a signal without reading the human text.
- The cancel reason of `delete`, `copy`, `hash`, `backup` and the stdin reader was a fixed sentence, `cancelled by signal`, even when the global deadline fired.
- The deadline interrupted a scan between files only. A huge file with no match ran to the end.

## Decision
- One terminal-cancel point in the dispatch (`finish_run` in `src/lib_dispatch.inc.rs`): when the run was cancelled, the writer drops what it still holds for the tail pass, writes the `cancelled` event unshaped, and flushes. Every scanning verb emits the same envelope, and `cancelled` is the last line of the output also with `--sort-by`, `--dedupe` and `--aggregate`.
- The terminal cancel record is written unshaped, so `--filter`, `--select` and `--limit` cannot drop it. A shaping flag describes how to present a complete answer and must not erase the notice that the answer is incomplete.
- `deadline_exceeded` is a boolean always serialized and never skipped. It comes from `shutdown.is_timeout()`.
- The exit status follows the live cause: SIGHUP 129 (`signal: "SIGHUP"`), SIGINT 130, SIGTERM 143, the deadline 124. The reason comes from the live signal through `cancel_reason`.
- A single cancellation primitive with block granularity, `CancelCheck`, is shared by the scan loop. `CancelRead` checks the flag on every read, so the deadline and SIGTERM are seen inside one huge file (`search_file_cancellable`).
- The cancel notice stays off the data channel and diagnostics travel only on the diagnostic channel.
- `src/stdin_cancel.rs` is a cancellable stdin reader, so `write`, `edit`, `edit-loop`, `apply`, `hash --stdin`, `extract`, `diff`, `calc` and `regex` obey SIGTERM (143) and the deadline (124) while stdin is still open.

## Consequences
- Data-channel contract change: `search`, `count` and the other scanning verbs emit the `cancelled` envelope when interrupted, where they used to emit silence or a partial summary.
- Docs that describe exit 143 as "the cancel" need the four-way table: 129, 130, 143 and 124. The failure path with a closed pipe is a fifth case, exit 141 (ADR-0063).
- On a 492 MB file with no match, `--timeout-secs 1` exits 124 in 1.01 s where it took 1.63 s, and SIGTERM at 0.3 s exits 143 in 0.307 s where it took 1.60 s (T3b).
- `cancelled` carries `deadline_exceeded` but no `ok: false` and no `code`, by design of `docs/schemas/README.md` (gaps.md W44).
- A `kill -9` is not a cancel. It still leaves a journal, and `wal-heal` is the recovery path (ADR-0062).

## Alternatives
- Let each verb decide what to print when interrupted: rejected, it is the behaviour that left consumers guessing.
- A fixed `cancelled by signal` reason: rejected, it hid the deadline case.

## Evidence
- `CHANGELOG.md:18-22,91-92,128-130,239-240,261` and `gaps.md:165-168` (W4), `gaps.md:20-29` (A1 to A3).
- `src/lib_dispatch.inc.rs:85-110` (`finish_run`), `src/ndjson/part2.inc.rs:112-156` (`CancelledEvent`, `deadline_exceeded`), `src/signal_cancel.inc.rs:55-110` (`CancelCheck`), `src/signal.rs:179` (`cancel_reason`), `src/constants.rs:131-149` (`EXIT_TIMEOUT` 124, `EXIT_SIGHUP` 129, `EXIT_SIGINT` 130, `EXIT_SIGTERM` 143), `src/stdin_cancel.rs` (245 lines), `docs/schemas/cancelled-event.schema.json:29-37`.
- `tests/cli_v0137_cancel.rs` (10 tests, `deadline_exceeded` asserted at lines 521, 612 and 828), `tests/cli_v0137_safety.rs:426` (`r22_search_sighup_exits_cleanly_with_cancelled_envelope`), `tests/cli_v0137_stdin_cancel.rs` (13 tests), `tests/cli_v0137_search_cancel_file.rs`.
- Binary 0.1.37: `write x.txt` with a stdin that never closes and `--timeout-secs 1` exited 124 and printed `{"type":"cancelled","deadline_exceeded":true,...}`.

## Trigger to revisit
- A new cancel cause appears that the four exit codes do not cover.
- `cancelled` is given `ok: false` and `code`.
