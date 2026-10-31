# ADR-0051 — v0.1.29 gap close, agent surface and binary honesty

> **Status: Superseded in v0.1.37 for `--fuzzy off`.** `--fuzzy off` is accepted and means exact-only; a miss exits 65 `MATCH_FAILED` (measured with `edit a.txt --old zzz --new bar --fuzzy off`).

- Status: Accepted
- Date: 2026-07-13
- Deciders: atomwrite maintainer


## Context
- The 0.1.28 gaps.md listed P0–P3 items and audit residuals
- The AST monolith produced about 52 MB in the default release
- The PRD promised a self-contained 5–8 MB binary
- Agents asked for fuzzy in replace, match diagnostics and named pipelines
- The surface is subprocess integration, a CLI with NDJSON output through agent-surface


## Decision
- Close P0–P3 and the residuals in 0.1.29 in the working tree
- Extract the shared fuzzy cascade into src/fuzzy.rs for edit and replace
- Emit best_candidate on match failures (exit 65)
- Slice the Cargo features: core, ast, lang-*, watch, semantic, full
- Declare the PRD goal of 5–8 MB as the target of the core build, not of the default build with AST
- Local size gate: slim core at most 15 MiB
- Implement recipe with real search-replace-hash dispatch
- Implement write --durability, Linux renameat2, stat, sparse, semantic-merge
- P3 foundation: watch with debounce/checksum/gitignore, offline Jaccard semantic-search, codemod with by_rule_id
- Regenerate the NDJSON schemas and update the EN/PT documentation at the root and in docs/


## Consequences
- Agents must pin `cargo install --path . --force` until 0.1.29 is published on crates.io
- Exact-only pipelines in replace needed --fuzzy off in 0.1.29, and in 0.1.30 off is rejected (see ADR-0052)
- The slim build omits AST and returns exit 78 in the stubs
- Historical documentation that cites 33 subcommands stays in sections for old versions
- The Current sections at the time of 0.1.29 pointed to 0.1.29, and the 0.1.30 residual superseded Current through ADR-0052


## Rejected alternatives
- OpenRouter embeddings in semantic-search — P3 stays offline and local
- Claiming that the default release is 5–8 MB — honesty in the CHANGELOG and README


## See also
- [gaps.md](../../gaps.md)
- [CHANGELOG.md](../../CHANGELOG.md)
- [MIGRATION.md](../MIGRATION.md)
- [INSTALL.md](../INSTALL.md)
- [HOW_TO_USE.md](../HOW_TO_USE.md)
- [schemas/README.md](../schemas/README.md)
