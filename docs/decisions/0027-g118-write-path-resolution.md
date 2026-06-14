# ADR-0027: write resolves the target against the workspace before all pre-steps (G118)

- **Status**: Accepted
- **Date**: 2026-06-11
- **Context**: `cmd_write` resolved the workspace but handed the RAW CLI path (relative to the process CWD) to three pre-steps — `handle_append_prepend`, `normalize_line_endings` (auto mode), and `verify_checksum` — while only `atomic_write` resolved it via `path_safety::validate_path` (src/atomic.rs:143). With a relative target and a CWD different from `--workspace`, each pre-step's `if !target.exists()` guard saw a non-existent path: append/prepend silently became a full overwrite (truncation), line-ending auto-detection was skipped, and `--expect-checksum` was skipped entirely — optimistic locking accepted ANY hash with exit 0. This is CWE-367 check-then-act on divergent path identities. A real incident truncated this repository's `gaps.md`; data was recovered via `rollback --latest --verify`. The forensic clue was `checksum_before` in the NDJSON envelope: `atomic_write` saw the real file the pre-steps never did. write.rs was the ONLY mutating command outside the convention — edit, copy, apply, move, rollback, set, del, and case all validate before any `exists()`/read.
- **Decision**: Resolve the target once at the top of `cmd_write` (`let resolved = validate_path(&args.target, &workspace)?`) and pass `&resolved` to all three pre-steps and to `atomic_write` (whose internal re-validation is idempotent for an in-jail absolute path). The NDJSON `path` field keeps echoing the user-supplied path: display identity ≠ operation identity. Project convention, now explicit: every mutating command MUST resolve its target via `validate_path` BEFORE any `exists()` check or read. A conformance guard test asserts `&args.target` appears exactly once in write.rs.
- **Consequences**:
  - **+** Append/prepend preserve existing content regardless of CWD; line-ending auto-detection works; `--expect-checksum` mismatch returns STATE_DRIFT (exit 82) instead of silently overwriting.
  - **+** Out-of-jail targets now fail early with WORKSPACE_JAIL (exit 126) before pre-steps run, not only at `atomic_write`.
  - **+** The `exists()` guards inside the helpers regain their intended "new file" semantics.
  - **-** Behavioral change for callers that depended on the bug (relative target + divergent CWD): drift now fails with exit 82 and out-of-jail paths fail earlier. Both were silent data-loss paths before; failing is correct.
  - **-** The test suite previously used only ABSOLUTE targets (immune to the bug); five regression tests now use a RELATIVE target with `current_dir` pointing outside the workspace.
- **Alternatives considered**:
  1. Patch each helper to take workspace and resolve internally. Rejected: three resolutions instead of one re-introduce the multi-identity risk the fix removes.
  2. Make `exists()` failures in pre-steps hard errors. Rejected: "target does not exist yet" is a legitimate state for `write` (file creation).
  3. Forbid relative targets. Rejected: breaks every documented recipe (`atomwrite --workspace . write target.rs`).
- **Trigger to revisit**: If a future mutating subcommand lands without the resolve-first convention, promote the textual conformance guard into a shared lint/test harness covering all commands.
