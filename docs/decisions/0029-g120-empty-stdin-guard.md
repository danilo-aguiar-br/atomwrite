# ADR-0029: G120 — Explicit cross-validation of `--append` + `--expect-checksum` + empty stdin

- Status: Accepted
- Date: 2026-06-13
- Context: G120 closed L1 (reject empty stdin by default), L2 (reject append/prepend with empty stdin) and L4 (field `stdin_bytes_read` in the `WriteOutput` envelope) in v0.1.15. L3 was missing: structured cross-validation. The problematic case: the caller combines `--append --expect-checksum <HASH>` and passes empty stdin by accident (the upstream pipe broke, a malformed heredoc). Today the checksum is compared against the pre-mutation file (it passes if the HASH matches) and the append is a no-op (0 bytes). The result: a silent exit 0, but the caller's intent was clearly wrong — the caller passed `--append` expecting to APPEND something. Without L3, the agent has no way to tell "intentional no-op" from "upstream broke and I did not notice". G118 closed path resolution; G120 L1+L2 closed input validation; L3 closes the last piece — cross-flag validation that detects ambiguous intent.

- Decision: Implement L3 as a structured warning through `tracing` + an opt-out flag `--no-checksum-when-empty` on `write`:
  1. Detection: in `cmd_write`, when `stdin_bytes_read == 0 && (args.append || args.prepend) && args.expect_checksum.is_some()`, activate the L3 logic.
  2. Default behavior: `tracing::info!` on stderr documents the cross-flag combination and proceeds with `verify_checksum` (which still validates the pre-mutation state). The caller that passed `--allow-empty-stdin` declared explicit intent, and L3 only logs.
  3. Explicit opt-out: `--no-checksum-when-empty` suppresses `verify_checksum` when stdin is empty. `tracing::warn!` records the decision.
  4. Failure kept: L1 (`read_stdin_content` rejects empty stdin by default) and L2 (`handle_append_prepend` rejects append+empty) stay unchanged. L3 is ADDITIVE: it only runs when L1/L2 were explicitly bypassed through `--allow-empty-stdin`.
  5. Telemetry: the `stdin_bytes_read` field (L4), already present, allows late detection through grep in logs, and L3 adds the explicit cross-flag signal on stderr.

- Consequences:
  - + L3 catches the last class of ambiguity of the compound G120 bug: a caller that combines `--append --expect-checksum` AND empty stdin gets a structured warning on stderr.
  - + The explicit opt-out (`--no-checksum-when-empty`) preserves the "intentional no-op" option without surprise.
  - + `tracing::info` on the default path is observable but not blocking — a caller that does not monitor stderr sees no difference, and one that does sees the signal.
  - + L3 introduces NO new exit code: the exit behavior stays the same. The difference is visibility, not control.
  - - An operator who monitors stderr will see more lines. Mitigated: it uses `tracing::info` (not warn), filterable through `--verbose` if desired.
  - - L3 only handles append/prepend + checksum + empty. It does not cover plain `write` + empty + checksum (but L1 already rejects that). It does not cover `--prepend + empty` (but L2 already rejects it). Full coverage comes with L1+L2+L3.

- Alternatives considered:
  1. A fatal error in L3 (`return Err(InvalidInput)`). Rejected: L1+L2 already protect, and L3 is an observational complement, not the main defense. Forcing an exit code would break the rule "opt-in L1 covers 99% of the cases".
  2. Add a field `cross_flag_warning: Option<String>` to the `WriteOutput` envelope instead of tracing. Considered but rejected: structured warnings in NDJSON pollute the output for agents that parse fields. `tracing` on stderr is the right layer for non-blocking signals.
  3. Auto-apply `--no-checksum-when-empty` when empty stdin + append + checksum are detected. Rejected: an implicit decision hides intent, and an explicit opt-out is more transparent.
  4. Add `--strict-empty-stdin`, which fails on ANY combination of empty stdin. Rejected: redundant with L1.

- Trigger to revisit: If production telemetry shows that agents consistently ignore the L3 warnings (an audit log with 0 reads in 30 days), consider promoting the warning to first-class stderr or a distinct exit code. If the combination `--append --expect-checksum` + empty stdin proves common in legitimate pipelines, revert L3 to an implicit opt-out.
