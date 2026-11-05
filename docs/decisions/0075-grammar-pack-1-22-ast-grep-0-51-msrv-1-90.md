# ADR-0075 — Grammar pack 1.22.1, ast-grep 0.51, tree-sitter 0.27 and MSRV 1.90

## Status
- Accepted (2026-10-28)
- Amends ADR-0019 on the pack version (1.8.1 to 1.22.1), on the grammar count (305 to 372) and on the cache path (`v1.8.1` to `v1.22.1`), so the 1.8 figures in that record stay as dated history
- Amends ADR-0057 on the MSRV, which rises from 1.89 to 1.90, because `ast-grep-core 0.51` and `tree-sitter 0.27` both declare that floor

## Context
- `cargo audit --deny warnings` reported 4 vulnerabilities and 10 unsound warnings against `zstd 0.13.3`, `zstd-safe 7.2.4` and `zstd-sys 2.0.16`, and `cargo deny check` failed the `advisories` gate with the same set.
- The reported identifiers were RUSTSEC-2026-0391 (an abandoned stream leaks into the next one), 0398 (`Seekable::decompress` sets a `Vec` length past its capacity), 0403 (uninitialized memory on a failed legacy frame), 0410 (the wasm allocator shim corrupts memory), plus 0395, 0396, 0397, 0399, 0400, 0401, 0402, 0404, 0405 and 0406 as unsound.
- None of those three crates is a direct dependency. They arrive only through `tree-sitter-language-pack 1.8.1`, which enables the pack's `download` feature and with it `ureq`, `rustls`, `ring` and the `zstd` compression path.
- Upstream fixed the flaws in `zstd 0.14.3`, `zstd-safe 8.2.0` and `zstd-sys 2.1.2`, so no version of the old line is safe.

## Decision
### 1. Move to `tree-sitter-language-pack` 1.22.1
- That is the highest published version, and it pulls `zstd 0.14.3`, which chains `zstd-safe ^8.2.0` and `zstd-sys ^2.1.2`, so the whole advisory set clears without a `[patch]` and without pinning an old pack.
- The pack keeps `get_parser`, `get_language`, `has_language`, `cache_dir`, `init`, `Error::LanguageNotFound` and the re-exported `Parser`, `Node`, `TreeCursor` and `tree_sitter::Language`, so `src/grammar.rs`, `src/syntax_check.rs`, `src/commands/query.rs`, `src/commands/outline.rs` and `src/commands/doctor.rs` do not change.
- The pack adds `fd-lock 4` and `dirs 7`. `dirs 7` resolves through the same `dirs-sys 0.5` that `directories 6` already uses, so the duplicate check stays clean.
- The grammar cache is keyed by pack version, so the bump moves the cache from `.../tree-sitter-language-pack/v1.8.1/libs` to `.../v1.22.1/libs` and the grammars download again on first use. That is the documented cost of the change, not a defect.

### 2. Move `ast-grep-core` and `ast-grep-language` to 0.51
- The pack 1.19 and later requires `tree-sitter ^0.27`, and `ast-grep` 0.43 requires `tree-sitter ^0.26`, so the three move together or the graph holds two `tree-sitter` majors and `multiple-versions = "deny"` fails.
- `ast-grep-core 0.51` keeps `AstGrep::new`, `replace_all`, `Pattern::try_new`, `Edit { position, deleted_length, inserted_text }` and `Language`, so `src/commands/transform.rs`, `src/commands/scope/mod.rs` and `src/lang_utils.rs` do not change.

### 3. Move `tree-sitter` to 0.27 and adjust the one call that changed
- `Query::new(&Language, &str)`, `Parser::set_language`, `QueryCursor::matches` and `StreamingIterator` keep their shape.
- The one break is `QueryMatch::captures`, which became a private field behind a `captures()` method; `src/commands/query.rs` now calls the method.

### 4. Raise `rust-version` to 1.90
- `ast-grep-core 0.51` and `tree-sitter 0.27` both declare 1.90 as their floor. A `rust-version` below that would claim a toolchain the graph cannot build with.
- `rust-toolchain.toml` keeps channel 1.94, so the fix does not change the development toolchain.

## Consequences
- `cargo audit --deny warnings` and `cargo deny check` pass with no ignore list entry, so the gate keeps its meaning.
- A host that already downloaded grammars pays one re-download of the missing ones after the upgrade, into the new versioned directory. `atomwrite doctor` reports the count before and after.
- Any future pack bump repeats that re-download. It is the price of a cache that cannot mix grammars built against different `tree-sitter` majors.
- The MSRV statement moves in every document that carries it: both readmes, `CONTRIBUTING`, `SECURITY`, `ARCHITECTURE`, `docs/INSTALL`, `docs/CROSS_PLATFORM`, `docs/HOW_TO_USE`, `docs/COOKBOOK`, both `llms` pairs, `llms-full` pair and `src/lib.rs`.
