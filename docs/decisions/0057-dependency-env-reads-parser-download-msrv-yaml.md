# ADR-0057 — Dependency environment reads, parser download, MSRV 1.89 and YAML stack

## Status
Accepted (2026-10-19)

## Context
- The product code reads no environment variable at runtime (G-007). Configuration is CLI flags plus XDG `config.toml`.
- Three groups of dependency code still read the process environment. They are listed here so that no reader finds them by surprise.
- `cargo audit` reported RUSTSEC-2024-0449 (`unsafe-libyaml`, unmaintained) and RUSTSEC-2024-0384 (`instant`, unmaintained).
- `tree-sitter-language-pack` is compiled with `download`, which brings `ureq`, `rustls`, `ring` and `zstd-sys`.

## Decision
### 1. Minimal environment-read exceptions (third-party code only)
- `sys-locale` 0.3.2 reads `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` and `LANG` on Unix, to detect the OS language for automatic i18n (`src/locale.rs`).
- `directories` 6 reads `HOME` and `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME` on Linux, to resolve the XDG paths of `.atomwrite.toml` (`src/storage.rs`).
- `tree-sitter-language-pack` (through `dirs`) reads `HOME` and `XDG_CACHE_HOME` for the parser cache, and `ureq` 3.3 reads `ALL_PROXY`, `HTTPS_PROXY`, `HTTP_PROXY` and `NO_PROXY` (with lowercase variants) when a parser is downloaded.
- Both i18n and XDG are required by the project rules (automatic OS language, XDG layout), so the first two are accepted exceptions.
- No `ATOMWRITE_*` variable exists, and the product code never calls `std::env::var`.

### 2. Keep the `download` feature of `tree-sitter-language-pack`
- Version 1.8.1 compiles no grammar in. Every parser is either downloaded on first use or already present in the local cache.
- Measured with the feature removed (debug build, empty cache): `outline` and `query` failed for all six languages tried (Rust, TypeScript, Python, Go, Java, Ruby) with `Language '<name>' not found`.
- Measured with the feature kept (fresh empty cache, network available): `outline` and `query` worked for the same six languages.
- `transform --dry-run` behaved the same in both builds, because it uses `ast-grep-language` (compiled in), not the pack.
- So `download` is the only way `outline` and `query` obtain a grammar (measured). `--syntax-check` also calls `get_parser` in `src/syntax_check.rs` (read from source, not run). It stays.
- The CLI never calls the download API (`DownloadManager`, `init`, `download`) itself. The download happens inside `get_language` and `get_parser` on first use of a language.
- The proxy variables of `ureq` are the documented exception for that path. A host with no network can pre-populate the cache directory.

### 3. MSRV 1.89
- `rust-version` moves from 1.88 to 1.89, as `rust-i18n` 4.2 requires it. The host toolchain is 1.94 (`rust-toolchain.toml`).
- `README.md`, `README.pt-BR.md`, `docs/INSTALL*.md` and `docs/CROSS_PLATFORM*.md` state 1.89.

### 4. YAML stack
- `rust-i18n` moves to 4.2 (4.2.4 resolved). It replaces `serde_yaml` by `serde-saphyr`.
- The YAML rules of `transform` are read with `serde-saphyr` 1.3 instead of `serde_yaml_ng`. Version 1.x is the major that `rust-i18n-support` already uses, so the graph keeps one copy. Version 2.0 would add a duplicate and was not chosen.
- Measured on ten rule files (valid, empty, missing field, unknown field, multi-document, malformed, anchors, empty list, numeric language, flow style): accept or reject, exit code and summary counts are identical between the two crates. Only the text of the parse-error message differs.
- The four YAML tests in `tests/cli_v012_audit_regressions.rs` now parse into `serde_json::Value` with `serde_saphyr`, with the same assertions.
- `unsafe-libyaml` (RUSTSEC-2024-0449) is gone from `Cargo.lock`. No exception is needed.

### 5. `notify` 8
- `notify` (optional `watch` feature) moves from 7 to 8.2.0, which drops `instant` (RUSTSEC-2024-0384).
- `cargo tree -d --target all` is unchanged for the default feature set (193 entries before, 193 after).
- Under `--features watch` it adds nine duplicated Windows crates (`windows-targets` and eight `windows_*`), now listed in the `deny.toml` skip table with the reason.

## Consequences
- `cargo audit` reports no advisory: neither `RUSTSEC-2024-0449` (`unsafe-libyaml`) nor `RUSTSEC-2024-0384` (`instant`) appears.
- The default binary still needs network once per language for `outline` and `query`, and for `--syntax-check` by source reading. This is a known and documented trade-off.
- A new environment read by a dependency must be added to section 1 or rejected.

## Alternatives
- Remove `download`: rejected, it disables `outline`, `query` and syntax checks for every language.
- `serde-saphyr` 2.0: rejected for now, it adds a second copy of the crate.
- Keep `notify` 7 with an advisory exception: rejected, the fix is available and affects only the opt-in feature.

## Trigger to revisit
- `tree-sitter-language-pack` ships compiled-in grammars as features.
- `rust-i18n-support` moves to `serde-saphyr` 2.
