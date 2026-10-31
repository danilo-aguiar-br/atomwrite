# ADR-0056 — Remove the compile-time `release_max_level_info` cap from `tracing`

## Status
Accepted (2026-10-19)

## Context
- `Cargo.toml` compiled `tracing` with `features = ["release_max_level_info"]`.
- That feature removes every `debug!` and `trace!` call site from release binaries at compile time.
- The CLI maps `-vv` to `debug` and `-vvv` to `trace` (`runtime::cli_log_level`) and builds an `EnvFilter` from that level.
- In a release binary the filter asked for levels that the binary no longer contained.
- `tracing-subscriber` then printed a coloured, non-JSON warning on stderr ("some trace filter directives would enable traces that are disabled statically").
- Measured on the pre-change release binary: `-vv read` and `-vvv read` wrote that ANSI warning instead of JSON records, and no `DEBUG` record existed.
- The stderr channel is documented as one JSON object per line, so the warning broke the agent contract (BUG-3).
- The project rule `rules-rust-tracing-desempenho-testes-observabilidade` favours compile-time caps for release builds. This ADR records the deliberate deviation.

## Decision
1. Declare `tracing = "0.1"` with no `release_max_level_*` feature.
2. Keep the runtime filter as the only level gate (`-v`, `-vv`, `-vvv`, `-q`, `-qq`).
3. Extend `diagnostic_records_are_distinguishable_from_payload` (`tests/cli_v0137_agent_native.rs`) to run `-vv` and `-vvv`, require JSON-only stderr, and require at least one record with `level` equal to `DEBUG`.
4. Run that test in debug and with `--release`, because only the release profile ever applied the cap.

## Measurement
- Two release binaries were built from the same source snapshot in separate target directories, differing only in the `tracing` feature.
- Workload: `search foo tree400 -j 8 --no-progress` on the 10.7 million line fixture, output to `/dev/null`.
- Seven runs, interleaved (before, after, before, after, …), because other processes were using the 72-core host.
- Wall time in seconds: `before` min 40.47, median 42.44, max 53.30. `after` min 39.77, median 43.12, max 53.29.
- DERIVADO: (43.12 - 42.44) / 42.44 × 100 = 1.60 % on the median, below the 2 % trigger and well inside the 12.83 s spread of the `before` runs alone.
- Binary size: 54 358 528 bytes before, 54 375 744 bytes after. DERIVADO: 54 375 744 - 54 358 528 = 17 216 bytes (0.03 %).
- Plan B (`release_max_level_debug`, with `-vvv` limited to debug) was therefore NOT needed.

## Consequences
- `-vv` and `-vvv` emit JSON records only, including `DEBUG` (and `TRACE` for `-vvv`), in release builds.
- Disabled call sites still cost one atomic load each at runtime. The measurement shows no cost outside the noise.
- A future cap must also change `cli_log_level`, or the warning returns.

## Alternatives
- `release_max_level_debug`: keeps `-vv`, still breaks `-vvv`. Rejected because it needs a measured regression to justify it.
- Silence the subscriber warning: hides the symptom while the binary still cannot honour `-vvv`.

## Trigger to revisit
- A measured median regression above 2 % outside the noise range on a representative tree.
