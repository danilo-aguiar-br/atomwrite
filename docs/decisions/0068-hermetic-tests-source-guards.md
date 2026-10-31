# ADR-0068 — Hermetic tests and source guards

## Status
Accepted (2026-10-15)

## Context
- After the `config` verb (ADR-0058), a test that runs the binary would depend on the `config.toml` and the locale of the machine that runs it (gaps.md W38).
- Production code had accumulated time and sidecar literals, silent `let _ =` that swallow errors, and `expect`, `unreachable!` and static-regex panics (gaps.md W11, W12, W13).

## Decision
- `common::hermetic` (`tests/common/mod.rs`) gives every child process its own empty `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME` and `XDG_STATE_HOME`, fixes `LANG` and `LC_ALL` to `en_US.UTF-8`, and removes `LANGUAGE` and `LC_MESSAGES`.
- `XDG_CACHE_HOME` is left on the real cache on purpose, because the `ast` feature downloads tree-sitter parsers there at run time and an empty cache would send `outline`, `query` and `scope` to the network (`real_cache_home`).
- A test that needs its own config redirects `HOME` and `XDG_*` after calling `hermetic`, and the later `env` call wins.
- Two source guards fail the build, both with an empty allowance list:
  - `cli_v0137_hardcode_guard` scans `src/` and rejects `Duration::from_*` with a numeric literal, the `.bak.` infix, sidecar and probe names, time-unit numbers (86400, 604800, 3600), `env::var*` and `Command::new`. It skips `constants*` files, test files and comments, and it also fails if a `.sh` file exists.
  - `cli_v0137_let_underscore_guard` rejects `let _ =` in production code and `.unwrap()`, `.expect(`, `unreachable!(`, `panic!(`, `todo!(` and `unimplemented!(` outside tests.
- Cleanup whose failure must not change the result goes through `src/best_effort.rs` (logged at debug), and the other discards propagate when they change the result.

## Consequences
- Tests no longer pass or fail by the developer's machine.
- Hermeticity is a fact only for the code of the helper, because the environment was not poisoned on purpose (gaps.md W38). `outline`, `query` and `scope` tests still depend on a filled parser cache (gaps.md W18, P3).
- 33 test files use `hermetic` (`rg -l hermetic tests/*.rs`). `CHANGELOG.md` and gaps.md W38 say 31.
- 44 tests fail under `--features core` because they need the `ast` feature and lack `#[cfg(feature = "ast")]` (gaps.md W19, P3).
- `build.rs` calls `git` and the source guard covers only `src/` (gaps.md W44 B12, accepted).
- A production `.unwrap()` or `let _ =` now fails the build, so a new one needs a typed error or a `best_effort` call.

## Alternatives
- Poison the environment on purpose to prove isolation: not done (gaps.md W38).
- Isolate `XDG_CACHE_HOME` too: rejected, it would send the AST tests to the network. On macOS the cache follows `HOME` and was not handled, because macOS was not run.

## Evidence
- `CHANGELOG.md:73,80` and `gaps.md:225-234` (W11, W12), `gaps.md:235-239` (W13), `gaps.md:281-283` (W38).
- `tests/common/mod.rs:15-83` (`hermetic_home`, `real_cache_home`, `Hermetic`, `hermetic`).
- `tests/cli_v0137_hardcode_guard.rs:14-58` (rules and the empty `ALLOWED`), `:146-168` (`violations_in`), `tests/cli_v0137_let_underscore_guard.rs:29` (empty `ALLOWED`), `:177-216` (`PANIC_NEEDLES`, empty `ALLOWED_PANICS`), `src/best_effort.rs` (133 lines).
- `tests/cli_v0137_hardcode_guard.rs` (5 tests) and `tests/cli_v0137_let_underscore_guard.rs` (5 tests).

## Trigger to revisit
- A literal or a panic must be allowed with a reason. It then enters `ALLOWED` with that reason.
- The suite runs on macOS and Windows hosts.
