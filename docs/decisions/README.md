# ADRs (Architecture Decision Records) — Index

- **Format**: Michael Nygard's ADR template (Status, Context, Decision, Consequences, Alternatives, Trigger to revisit).
- **Authoring rule**: Every non-trivial change to atomwrite (a new subcommand, a new error variant, a new dependency) MUST be accompanied by an ADR or by an update to an existing ADR.
- **Storage**: One file per ADR in `docs/decisions/`. Filename is `NNNN-kebab-case-slug.md`. Numbering is monotonic.

## Index

- [0019 — tree-sitter-language-pack](0019-tree-sitter-language-pack.md) — v0.1.12 chose `tree-sitter-language-pack` over direct `tree-sitter` deps to keep binary small.
- [0020 — WAL sidecar](0020-wal-sidecar.md) — G114 WAL sidecar path is `.atomwrite.journal.<basename>.atomwrite.journal.json`.
- [0021 — v14 query/outline no S-expr](0021-v14-query-outline-no-s-expr.md) — v0.1.12 `query` accepts only kind names, not tree-sitter S-expressions.
- [0022 — G72 replaces heuristic](0022-g72-tree-sitter-replaces-heuristic.md) — G72 tree-sitter REAL syntax check replaces (not adds to) the bracket heuristic.
- [0023 — G114 WAL consultive](0023-g114-wal-consultive.md) — G114 recovery is consultative (no auto-replay, no auto-delete).
- [0024 — get_toml_path manual](0024-get-toml-path-manual.md) — `get/del` TOML dotted path navigation is manual `Table` descent, not `toml_edit::Table::get`.
- [0025 — positions opt-in](0025-positions-only-in-query-tree.md) — `query --positions` is valid in `--query`/`--tree`, silently ignored in `--kinds`.
