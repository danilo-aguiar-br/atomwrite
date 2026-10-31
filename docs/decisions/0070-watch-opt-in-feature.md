# ADR-0070 — `watch` becomes an opt-in feature (E7)

## Status
Accepted (2026-10-15)

## Context
- atomwrite is a one-shot binary: it is born, runs and dies. The default binary carried `notify` and a long-lived event loop that this model does not need.
- The `watch` event channel was unbounded.

## Decision
- `watch` is outside the `default` feature set: `default = ["core", "ast", "lang-rust", "lang-ts", "lang-py"]`. The feature `watch = ["dep:notify"]` brings it back, and `full` includes it.
- The verb stays on the surface through a stub in `src/commands/watch.rs` (`#[cfg(not(feature = "watch"))]`) that fails closed with exit 78 `CONFIG_INVALID` and the hint "watch requires rebuild: cargo install atomwrite --features watch (or cargo build --features watch)".
- `agent-surface` still lists `watch` among the 47 tools, with `available: false` and an `install_hint`, so a build never hides a verb.
- With the feature, `watch` uses a bounded channel of 4096 events, where it was unbounded.
- `cargo xtask check-features` runs `cargo check --locked` on the `core` and `core,watch` feature sets and stops at the first error.

## Consequences
- The surface has 47 verbs on every build, but `watch` works only with `--features watch` or `--features full`. Docs must not say that the default build has `watch`.
- Without the feature the verb never starts a loop, so the default binary stays one-shot.
- `watch` built with the feature ends with a `watch_summary` event (`src/commands/watch.rs:198`).

## Alternatives
- Keep `watch` in `default`: rejected, it carries `notify` and a long-lived loop in every install.
- Remove the verb when the feature is off: rejected, the SSOT of 47 names would change from build to build.

## Evidence
- `CHANGELOG.md:70,124,147` and `gaps.md:72` (E7).
- `Cargo.toml:302-318` (the comment on E7, `default`, `watch`, `full`), `src/commands/watch.rs:54,209,218` (feature code) and `:289-300` (stub), `xtask/src/main.rs:105-133` (`check_features`).
- `tests/cli_v0137_artefato.rs:555` (`default_binary_does_not_carry_the_continuous_mode`).
- Binary 0.1.37 (installed without the feature): `atomwrite watch DIR --max-events 1` exited 78 with `CONFIG_INVALID` and the rebuild hint. `agent-surface` printed `available: false` for `watch` with `install_hint` "cargo install atomwrite --features watch (or cargo build --features watch)".

## Trigger to revisit
- A long-lived mode becomes part of the product contract.
- `notify` leaves the tree or the feature set is reorganized.
