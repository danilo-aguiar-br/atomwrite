# ADR-0061 — One terminal event per failed run (`publish_failure` and `VerdictPublished`)

## Status
Accepted (2026-10-15)

## Context
- `NdjsonWriter::write_event_failed` had 7 callers and none marked the verdict as published, so `main` wrote the error envelope again. A failed run ended with two terminal lines (gaps.md W6).
- The case showed in `search` without a match, `semantic-merge` with a conflict and `edit-loop --partial`. A parser that counts one terminal event per run got it wrong.
- `search`, `transform` and `replace` also swallowed read, open, probe and walk errors, so `NO_MATCHES` came out for a tree they could not fully read (gaps.md W5).

## Decision
- `NdjsonWriter::publish_failure` writes the terminal summary with `ok: false`, `code`, `exit` and the whole error detail (`error_class`, `retryable`, `suggestion`, `suggestion_lang`, `conflict_count`, `match_count`, `pairs_*`).
- It then returns the error tagged with `VerdictPublished`, which `main` reads to skip the second envelope. `write_event_failed` was removed.
- A failure event ignores `--filter` and `--limit` and honours `--select` and `--redact`.
- `search`, `transform` and `replace` report unreadable files as `files_failed` and `errors[]` in the summary, with `errors[]` capped at `SUMMARY_ERRORS_MAX_ENTRIES` (50) while `files_failed` keeps the full count. Zero matches with a failure exits with the error of the first failure, and a match with a failure exits 0 with `files_failed` visible. A file that is too large, not UTF-8 or binary is still skipped by design.
- The success envelope carries `ok: true`, stamped once by the writer. A summary is no longer followed by a contradicting error.
- `EditPairFailed` now has the code `MATCH_FAILED`, with the same exit 65.

## Consequences
- Exactly one terminal event per failed run, except for a broken pipe and a `hash --verify` verdict that was already published.
- The summary schemas carry the failure detail as optional properties (`FAILURE_REQUIRED`, ADR-0067).
- `verify` with a wrong checksum emits one event with one verdict instead of two contradicting events.
- A tail pass (`--sort-by` and friends) can still let `--filter` reach the failure event (gaps.md W25, P3), and the multi-rule `transform` still writes the child `ok: false` summary before `rule_error` (W24, P3).

## Alternatives
- Keep `write_event_failed` and have each caller mark the verdict: rejected, 7 callers had already forgotten it.

## Evidence
- `CHANGELOG.md:42,61,156-160,262,265,300` and `gaps.md:184-192` (W5, W6).
- `src/output.rs:96-110` (`publish_failure`), `src/output_verdict.inc.rs:161-181` (`VerdictPublished`), `src/main.rs:147-153` (skips the second envelope), `src/schema_terminal.inc.rs:43-67`.
- `rg -n -F write_event_failed src` returns no match.
- `tests/cli_v0137_terminal_event.rs` (17 tests) and `tests/cli_v0137_envelope.rs` (26 tests).

## Trigger to revisit
- A verb needs two terminal events by design.
- The tail pass is reworked and `--filter` must never reach a failure event.
