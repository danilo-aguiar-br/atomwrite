# ADR-0072 — Agent-native surface: stream shaping, `--print-schema` before clap, side-effect guards (E3, E4, E6)

## Status
Accepted (2026-10-15)

## Context
- Agents spent tokens on `jq` and `sed` pipes to cut, sort and sum NDJSON, and on guessing schema names.
- Destructive truncation of the output had no explicit opt-in.
- `--print-schema` inside clap could not answer on an install that cannot yet parse a full command line.
- A shaping flag could erase the very notice that the answer is incomplete.

## Decision
- Shape the stream inside the process, on the data channel, with global flags: `--sort-by` (`path-line`, `line-path` or a key), `--dedupe KEY`, `--aggregate KEY[:OP[:FIELD]]`, `--join KEY:SUFFIX`, `--diff-against FILE`, `--redact`, `--normalize-encoding` and `--max-output-bytes`. The flags `--select`, `--fields`, `--filter` and `--limit` from 0.1.36 (ADR-0055) stay.
- `--max-output-bytes` truncates every string value, `checksum` included, and requires `--allow-truncate` next to it. Without it the run exits 2.
- `--print-schema [NAME]` is answered before clap parses the subcommand (`scan_print_schema`, `plan_print_schema`, `prescan_print_schema`). Without a name it lists every schema, as one NDJSON line each. An unknown name exits 65 with the name list and an error envelope, never an empty document. `--print-schema=NAME` is accepted exactly as the separated form, and the flag placed after the command with no value does not replace the command.
- `--non-interactive` and `--confirm-side-effect` are accepted globally. `--confirm-side-effect` acknowledges a side effect only where the contract of the command asks for it.
- `agent-surface` publishes a `discovery` entry generated from `SCHEMA_NAMES`, so the manifest and the door cannot disagree.
- Errors are always emitted: `--filter` never hides `ok: false`, and the terminal cancel record is written unshaped (ADR-0073).
- `-n` stays reserved for `replace`. The shape flags use `--limit`.
- A credential-shaped top-level key is redacted on every envelope with or without `--redact`, and `--redact` also reaches the error envelope with a widened list of issuance prefixes.
- `serde_json` is built with `preserve_order`, so an envelope serialized through `Value` keeps the key order its struct declares. Measured cost: +2 ms on 47 ms for 3000 events (about 4%).

## Consequences
- A larger global flag surface: 33 global args in `atomwrite commands --include-globals`.
- `--max-output-bytes` is destructive for `checksum` (20 characters in a hand measurement), so it must not be used when the whole `checksum` or `content` is needed.
- `--aggregate` keeps `ok`, `code` and `exit` of the failing terminal event, and `--select` no longer emits an event it left empty.
- `--join` and `--aggregate` alone had no execution test when gaps.md was written (gaps.md E4). `--aggregate type:count` was run by hand and returned `{"type":"entry","count":53}` plus the summary.
- With a tail pass active (`--sort-by` and friends) the failure event enters the buffer and `--filter` can still reach it (gaps.md W25, P3).
- `search` without `--sort-by` has no ordering guarantee between runs when multithreaded, as in the older binary (gaps.md W44 B11, P3).

## Alternatives
- Leave the shaping to `jq` and `sed` pipes: rejected, it is what cost the tokens.
- Handle `--print-schema` inside clap: rejected, discovery has to work before a full command line parses.
- Silent truncation by `--max-output-bytes`: rejected, it needs the explicit `--allow-truncate`.

## Evidence
- `CHANGELOG.md:14-17,23,112-114,123,125-127,221,325-328` and `gaps.md:64-68,71` (E3, E4, E6), `gaps.md:277` (W36).
- `src/cli_global.inc.rs:140` (`print_schema`), `:154,167` (`non_interactive`, `confirm_side_effect`), `:177-181` (`allow_truncate` requires `max_output_bytes`), `:269-381` (shape flags), `src/schema_dispatch.inc.rs:103-148` (`scan_print_schema`, `plan_print_schema`), `src/runtime_cli.inc.rs:132-140`, `src/main.rs:71-80`, `src/commands/agent_surface.rs:59-135` (`discovery`), `Cargo.toml:109`.
- `tests/cli_v0137_agent_native.rs` (22 tests, including `print_schema_*` at lines 49-160, `max_output_bytes_requires_the_truncation_opt_in` at 357 and `non_interactive_is_accepted_globally` at 589).
- Binary 0.1.37: `commands --include-globals` lists 33 global args, `agent-surface` has a `discovery` object with 54 `schema_names`, and `--print-schema` lists 55 names.

## Trigger to revisit
- A global flag collides with a local one of a verb.
- The tail pass is changed so that `--filter` never reaches a failure event.
