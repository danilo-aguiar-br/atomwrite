# ADR-0074 — Serialized read-modify-write and a strict `apply`

## Status
Accepted (2026-10-15)

## Context
- A write that is computed from the file it replaces lost updates under concurrency. `atomic_write` took the advisory lock only at write time, after the verb had read the file, so two writers read the same base and the second `rename` erased the first (gaps.md W31, review finding V2 A1).
- Measured before the fix: 30 parallel `edit` on distinct lines of one file all exited 0 with `ok: true` and only 4 to 10 of the 30 changes survived. 30 parallel `set` kept 1 key, 30 `replace` kept 1, and 19 parallel `config set` kept 2.
- `apply` with a unified diff used only the line number of the `@@` header. A patch `@@ -1 +1 @@ -zzz +yyy` over a file that started with `alpha` produced `yyy` with exit 0, and a hunk with wrong context erased three lines. The `.bak` of a success is removed, so no copy was left (gaps.md W29, V2 C1, critical, present since 0.1.36).
- `apply` with SEARCH/REPLACE swallowed `Cancelled`, `InvalidInput` and `MatchAmbiguous` (an `else if let Ok(...)`), and a missing block vanished with `ok: true` and `hunks_applied: 1`, leaving the file half applied (gaps.md W30, V2 A2).

## Decision
- A read-modify-write verb takes the advisory lock BEFORE it reads and holds the guard until the write is committed: `acquire_for_update` in `src/lock.rs`, with the wait budget `lock.acquire_timeout_ms`. The verbs are `edit`, `edit-loop`, `set`, `del`, `config set`, `config unset`, `apply`, `write --append`, `write --prepend`, and the `replace` and `edit` ops of `batch`.
- `edit-loop` reads stdin before taking the lock, so it never holds the lock while waiting for input.
- `replace` reads each file without the lock. At write time it takes the lock, compares the checksum and, if the file changed, redoes the replacement over the new content, so every writer succeeds and no update is lost.
- The lock is re-entrant per thread (the inner `atomic_write` of the same verb shares it), and `LockGuard` is no longer `Send`. A second thread of the same process asks the OS and waits, as another process would.
- `atomic_write_based(target, content, opts, workspace, expected_base)` is the net for verbs that do not hold the lock while reading. Under the lock it hashes the current file and compares it with the checksum of the content the new content came from. A different or missing file fails with `STATE_DRIFT` (exit 82, retryable) and writes nothing, before any journal is opened. `case`, `transform`, `scope`, `replace` and `edit` call it. The loser re-runs.
- `apply` with a unified diff compares the context and the removed lines of each hunk with the file (exact, trailing spaces ignored). A hunk is found at the position of its header or at the nearest exact neighbour within `PATCH_MAX_HUNK_SHIFT` (64) lines. All hunks are applied in memory and written once, and context lines keep the text of the file.
- A mismatch is `MATCH_FAILED` (exit 65) naming the hunk, the first line that disagrees, with `best_candidate` (strategy `unified-hunk`), and nothing is written. An unreadable `@@` header and a patch with no hunk are `INVALID_INPUT`. An insertion-only hunk inserts AFTER the named line.
- `apply` with SEARCH/REPLACE tries an exact match first and then the fuzzy cascade. Only `MatchFailed` marks a block as missing. Any missing block fails the whole batch with `MATCH_FAILED` naming the blocks, with `best_candidate` when the text resembles some line, and nothing is written. `Cancelled`, `InvalidInput` and an ambiguous fuzzy match reach the caller. A patch with no block is `INVALID_INPUT`.
- There is no `--partial` on `apply` and none was created.

## Consequences
- 30 parallel `edit`, 30 `replace`, 30 `set` and 16 `config set` all exit 0 and every change is in the file (`cli_v0137_rmw_concurrent`).
- Residual, declared: where `flock` does not exist (NFS) and on Windows (the lock is a no-op, because `LockFileEx` needs `unsafe` and the `Win32_System_IO` feature of `windows-sys`) only `edit`, `replace`, `case`, `transform`, `scope` and `codemod` keep the checksum net (CHANGELOG).
- `edit-loop` passes no base checksum, and `batch --transaction` locks per operation and not per batch.
- `--expect-checksum` runs outside the lock, so a TOCTOU window remains between the check and the write (gaps.md, P3).
- A file name above about 238 bytes can overflow the 255-byte limit with the sidecar name `.<name>.atomwrite.lock`, and an in-place write in a directory where the sidecar cannot be created runs without the lock (gaps.md, P3).
- A killed process (SIGKILL) can leave a lock sidecar behind. The `flock` is already released by the kernel and the next writer reuses and removes the file (gaps.md W44).
- Behaviour changes: an insertion-only hunk of `apply` used to insert one line above, and blank padding lines that the hunk header does not count are dropped.

## Alternatives
- Keep the lock at write time and rely on `--expect-checksum`: rejected, it detects a lost update after it happened and does not prevent it, and only 4 to 10 of 30 changes survived in the measurement.
- Let `apply` write the hunks that match: rejected, a missing block must not leave the file half applied, and `apply` has no `--partial`.
- A Windows lock through `LockFileEx`: not done, it needs `unsafe` and a feature that is not enabled, and no Windows host was available to prove it.

## Evidence
- `CHANGELOG.md:74,202-212,278` and `gaps.md:251-266` (W29, W30, W31 with its P3 cut), `gaps.md:321-324` (P3 list).
- `src/lock.rs:1-60` (what the lock covers and its limitations), `:150-152` (`acquire_for_update`), `:104` (`HELD`, the per-thread table) and `:176-230` (`acquire_exclusive`), `src/atomic/write.rs:54-100` (`atomic_write_based`, the base comparison and `StateDrift`).
- Callers of `acquire_for_update`: `src/commands/edit/single.inc.rs:33`, `src/commands/edit_loop.rs:287`, `src/commands/set.rs:63`, `src/commands/del.rs:53`, `src/commands/config_cmd_write.inc.rs:26`, `src/commands/apply.rs:62`, `src/commands/write.rs:74`, `src/commands/replace_run.inc.rs:282,386`, `src/commands/batch/ops.inc.rs:151,277`.
- Callers of `atomic_write_based`: `src/commands/case.rs:266`, `src/commands/transform.rs:290`, `src/commands/scope/mod.rs:320`, `src/commands/replace_run.inc.rs:417`, `src/commands/edit/single.inc.rs:185`, `src/commands/edit/multi.inc.rs:315`.
- `src/commands/apply_unified.inc.rs:49-100` (`apply_unified`), `:103-125` (`locate_hunk`), `:132-170` (`hunk_mismatch`), `src/commands/apply.rs:169-255` (`apply_search_replace`), `src/constants.rs:364-366` (`PATCH_MAX_HUNK_SHIFT` 64, `PATCH_ERROR_SNIPPET_CHARS` 120).
- `tests/cli_v0137_rmw_concurrent.rs` (5 tests, the stale base at line 182), `tests/cli_v0137_apply_strict.rs` (8 tests), `tests/cli_v0137_lock.rs` (3 tests).
- Binary 0.1.37 in a bench with `HOME` and `XDG_*` inside the scratchpad: 30 parallel `edit --fuzzy off` on 30 distinct tokens of one file gave 30 events with `ok: true`, 30 lines changed, 0 left, and no sidecar left in the workspace. `apply --format unified` with wrong context exited 65 `MATCH_FAILED` with `best_candidate` (strategy `unified-hunk`) and the file stayed `alpha`, `beta`, `gamma`. `apply --format search-replace` with one present block and one missing block exited 65 `MATCH_FAILED` and the file stayed unchanged. A correct unified patch exited 0 with `hunks_applied: 1`.

## Trigger to revisit
- A Windows lock is implemented, or Windows is validated on a real host.
- `edit-loop` starts to pass a base checksum, or `batch --transaction` locks per batch.
- `apply` gets a partial mode by explicit request.
