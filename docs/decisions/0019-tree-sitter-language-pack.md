# ADR-0019: tree-sitter-language-pack over direct tree-sitter dependency

- **Status**: Accepted
- **Date**: 2026-06-07
- **Context**: v0.1.12 introduces the `query` and `outline` subcommands (v14 Tier 3) plus `--syntax-check` for `write` (G72). All three features need a tree-sitter parser. We needed to decide which crate to depend on.
- **Decision**: Adopt `tree-sitter-language-pack = "1.8"` with `default-features = false` and `features = ["download", "dynamic-loading"]`. The `download` feature lets the pack fetch individual parser `.so`/`.dylib`/`.dll` files on first use. The `dynamic-loading` feature keeps them off the binary's static footprint. The 305 grammars are NOT bundled.
- **Consequences**:
  - **+** Binary stays small (~5-10 MB on Linux, vs ~50+ MB if all 305 parsers were bundled via `bundled` feature).
  - **+** Single dependency exposes 305 languages; no need to declare 5-10 individual `tree-sitter-{rust,python,js,...}` crates.
  - **+** First-use download is cached in `~/.cache/tree-sitter-language-pack/parsers/`, so second run is offline.
  - **-** First call to a given language pays a network round-trip + parser compile/install time (~50-200ms per parser).
  - **-** Air-gapped systems need a manual pre-population of the cache.
  - **-** `tree-sitter-language-pack` does NOT re-export `tree_sitter::Query` (S-expression support). v14 `query` accepts only kind names, not S-expressions. Real S-expression queries are deferred to v0.1.13 if we add a direct `tree-sitter` dependency.
- **Alternatives considered**:
  1. `tree-sitter = "0.25"` + 5 individual `tree-sitter-{rust,python,javascript,typescript,go,toml,json,css,bash,markdown,yaml,html,...}` crates. Rejected: 8+ deps, manual feature flag management, no bundling story.
  2. `tree-sitter-language-pack` with `bundled` feature. Rejected: 1+ GB parser set, defeats cargo install.
  3. Hand-rolled lexer per language. Rejected: re-inventing 305 wheels is absurd.
- **Trigger to revisit**: If a user reports latency problems for first-use per language, or if a corporate environment blocks the download endpoint, we may need to ship a pre-populated parser bundle as a separate `atomwrite-parsers` crate.
