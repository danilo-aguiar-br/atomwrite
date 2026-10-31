# ADR-0065 — Fuzzy content ceiling and cooperative cancel polls

## Status
Accepted (2026-10-15)

## Context
- Every fuzzy strategy costs O(lines), so a huge file could run for minutes past `--timeout-secs` (gaps.md W4). With a 2 s budget on a pattern that never matches, `replace` took 6.16 s on 50 MB and 18.6 s on 150 MB, and `edit` took 5.73 s and 17.7 s.
- The loops of `strategies`, `windows`, `collect`, `rank`, `cascade`, `util` and `escape_map` never polled for cancellation. `windows.rs` and `rank.inc.rs` returned `None`, which the cascade turned into `MatchFailed` (exit 65) even when the deadline had already passed.
- The cancel checks were copied into each loop.

## Decision
- `[fuzzy].max_content_bytes` (default 256 MiB, `FUZZY_MAX_CONTENT_BYTES`, config key `fuzzy.max_content_bytes`) is a ceiling above which the cascade does not run.
- Above it, `edit` exits 65 `INVALID_INPUT` naming the key and `--fuzzy off`, and `replace` records the file in `files_failed`. An exact match and `--fuzzy off` still work above the ceiling.
- `escape_normalized` is skipped above `FUZZY_ESCAPE_MAP_MAX_CONTENT_BYTES` (32 MiB) without building its offset map, and the cascade goes on with the other strategies.
- One cancellation primitive, `CancelCheck` (block granularity, `src/signal_cancel.inc.rs`), is shared by the scan loops. The fuzzy loops use `CancelPoll` (a local counter plus a mask, `src/fuzzy/util.rs`), and `check_cancel()?` runs after each strategy and before `MatchFailed`.
- Only a real miss (`MatchFailed`, `MatchAmbiguous`) becomes zero matches. `Cancelled`, `InvalidInput` and internal errors reach the caller in `edit`, `edit-loop`, `replace` and `batch` with their own code.
- Two dedicated localized hints, `suggestion.fuzzy-content-cap` and `suggestion.fuzzy-pattern-cap`, explain the refusals and no longer embed the English `reason`.
- In the single-line Jaro-Winkler branch only a candidate that passes the dual gate counts as a tie (gaps.md W52): an absent block among similar lines is `MATCH_FAILED` with `best_candidate`, and a real tie stays `MATCH_AMBIGUOUS`.

## Consequences
- With a 2 s budget the same fixtures take 2.02 s on `replace` and `edit` (50 MB and 150 MB), all with exit 124 and a `cancelled` envelope (debug binary, gaps.md W4).
- A 896.4 MB file is refused by the ceiling: `edit --old XXNOPE` exits 65 `INVALID_INPUT` in 1808 ms in release (V1), inside the 3.5 s target. The refused file is still read whole before the refusal (RSS equal to the file size, gaps.md W17, P3).
- Behaviour change in `batch`: a `replace` op whose fuzzy match is ambiguous now fails the op, where it used to be reported as "no matches".
- The pt-BR hint is entirely in pt-BR and cites `atomwrite config set fuzzy.max_content_bytes <BYTES> --scope global` and `--fuzzy off`. The `gaps.md` line W23 still lists the older P3, written before the dedicated hints existed (V1 FAIL-5 is closed in W39).

## Alternatives
- No ceiling, only polls: rejected, on 900 MB `edit` took 4.19 s with exit 124 on the debug binary, above the 3.5 s target, because the file is read, hashed and split into lines before the cascade starts. The ceiling refuses the file in 1808 ms in release.
- Copy the cancel check into each loop: rejected, replaced by one primitive.

## Evidence
- `CHANGELOG.md:18,58-60,79,148-150,224,234,261` and `gaps.md:165-168` (W4), `gaps.md:243` (W23), `gaps.md:247-248` (W17), `gaps.md:314-316` (W52).
- `src/constants.rs:205` (`FUZZY_MAX_CONTENT_BYTES`) and `:215-219` (32 MiB, asserted under the ceiling), `src/config.rs:234-246`, `src/fuzzy/types.inc.rs:54`, `src/fuzzy/escape_map.inc.rs:65`, `src/fuzzy/util.rs:68-110` (`check_cancel`, `CancelPoll`), `src/signal_cancel.inc.rs:63` (`CancelCheck`).
- `tests/cli_v0137_cancel.rs` (10 tests, including the fuzzy deadline tests of gaps.md W4).
- Binary 0.1.37: `config get fuzzy.max_content_bytes` returned 268435456.

## Trigger to revisit
- A `stat` before the read is added, to avoid the read peak of a refused file.
- The 256 MiB default is changed.
