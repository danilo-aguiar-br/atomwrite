# Privacy Policy — atomwrite

[Leia em Português](PRIVACY.pt-BR.md)

**Last updated:** 2026-07-19

## Summary

`atomwrite` is a local CLI. It does **not** phone home, collect telemetry, or upload file contents to any third party by default.

## What runs locally

- All file reads, writes, searches, backups, and WAL sidecars execute on the machine that invokes the binary.
- Configuration is CLI flags + XDG directories + `.atomwrite.toml` only (no product `ATOMWRITE_*` runtime knobs in v0.1.35). Host env may still supply OS-level signals such as `NO_COLOR`; OS locale is read once via `sys-locale` for UI suggestion language only.
- Optional features (AST, watch, semantic) still operate on local filesystem data only.
- **Locale preference (local only):** when you run `atomwrite locale --set <tag>`, atomwrite writes a single-line preference file under the XDG config directory (e.g. `~/.config/atomwrite/locale` on Linux) with mode `0600` when the platform allows. That file stores only a BCP 47 language tag (`en` or `pt-BR`). It is never uploaded. OS locale is not sent off-host.

## What is never collected by atomwrite itself

- No analytics, crash-reporting SaaS, or usage beacons are embedded in the binary.
- No network client is required for core read/write/edit/search/replace operations.
- NDJSON output may contain **file paths and content excerpts** that you (or an agent) chose to process — treat stdout as sensitive if those paths hold secrets.

## Data you may produce

- Backup files (`*.bak.*`), WAL journals, and temporary files in the workspace or OS temp directory.
- Shell completion scripts when using `atomwrite completions --install`.
- Install scripts may download release artifacts from GitHub Releases when you run them — that traffic is between your host and GitHub, not an atomwrite telemetry endpoint.

## Third-party dependencies

- Upstream crates are governed by their own licenses (see `deny.toml` / `cargo deny`).
- If you enable tooling that calls external services (unrelated wrappers, agent hosts), those systems have separate privacy policies.

## Contact

Maintainer: see `Cargo.toml` `authors` / repository listed in the package metadata.


## v0.1.35 configuration privacy

- Product configuration is CLI flags + XDG directories + `.atomwrite.toml` only.
- No product telemetry is collected or transmitted.
- Doctor may read host env for local diagnostics (CI indicators); that is not a remote phone-home channel.
