# ADR-0066 — `DoctorChecksFailed`: `doctor --strict` exits 1 with one terminal line

## Status
Accepted (2026-10-15)

## Context
- `doctor --strict` with a failing check returned an error without a type. That error falls to exit 255 (`INTERNAL_ERROR`) and a second error envelope followed the report (gaps.md W56, finding F-1 of the V3 review).
- The same review found two siblings: an interrupted `write` also exited 255, and `batch` without `--transaction` emitted its `summary` and then a second error envelope.
- An agent that sees 255 treats it as a crash, but a failing host check is a settled verdict, not a bug.

## Decision
- Add `AtomwriteError::DoctorChecksFailed { failed, warned }`: code `DOCTOR_CHECKS_FAILED`, exit 1 (the generic failure status the command always returned), class `precondition_failed`, because the host must change before a re-run.
- The `doctor_report` itself is the single terminal line, published through `publish_failure` (ADR-0061). It carries the failure detail, and the error carries only the totals.
- `ok` in the report counts only `fail` in the default mode and also rejects `warn` under `--strict`. The status `info` never fails. The failure detail is optional in the `doctor` schema, because the report says `ok: false` for a failing check without failing the run when `--strict` is absent.
- An interrupted `write` now exits with the real status of the signal or the deadline (143, 130 or 124). `batch` without `--transaction` publishes the failure on its own `summary`.
- The `doctor` and `batch` schemas are closed (`additionalProperties: false`), and the name `--print-schema` serves for the second is `batch` (ADR-0067).

## Consequences
- Agents branch on exit 1 plus `code` instead of treating 255 as a crash.
- Exit 1 is now shared by `NO_MATCHES` and `DOCTOR_CHECKS_FAILED`. The `code` disambiguates them.
- It was recorded as a correction, not a behaviour change: the exit had been 255 because the error had no type.

## Alternatives
- Keep exit 255: rejected, it is the internal-error status and hides a normal verdict.
- A dedicated exit number: not chosen, exit 1 is what the command always returned and the `code` carries the distinction.

## Evidence
- `CHANGELOG.md:243-246` and `gaps.md:206-208` (W56).
- `src/error/variants.inc.rs:290-301` (variant and rationale), `src/error/impl_methods.inc.rs:44` (exit), `:84` (class), `:159` (code), `src/commands/doctor.rs:230-258` (strict handling and `publish_failure`), `src/commands/agent_surface_exit.inc.rs:128,263`, `src/schema_terminal.inc.rs:57-67`.
- `tests/cli_v0137_doctor_strict.rs` (4 tests), `tests/cli_v0137_terminal_event.rs:540` (`batch_with_a_failed_operation_ends_with_one_failed_summary`), `tests/cli_v0137_stdin_cancel.rs:137` (`r2a_write_with_open_stdin_exits_143_on_sigterm`).

## Trigger to revisit
- Exit 1 collides with another verdict that agents must tell apart without reading `code`.
