# ADR-0022: G72 tree-sitter REAL replaces (does not add) the bracket heuristic

- **Status**: Accepted
- **Date**: 2026-06-07
- **Context**: v0.1.11 had a bracket-balance heuristic in `atomic_write` to catch obvious syntax errors before committing a write. It was useful but it returned many false positives (Python indentation, JS template literals) and false negatives (Python `import` of missing module). v0.1.12 adds G72, real tree-sitter syntax check.
- **Decision**: G72 REPLACES the heuristic. `--syntax-check` on `write` invokes tree-sitter for known languages (24 covered) and falls back to the legacy heuristic for unknown extensions. The default for `write` is OFF (no syntax check) because it adds 50-200ms per write.
- **Consequences**:
  - **+** Users who opt in get accurate syntax errors with line/column/kind, not bracket counts.
  - **+** Fallback heuristic still works on languages without parsers (e.g. Dockerfile, .env).
  - **-** Default OFF means most users never see the benefit. We added a `cfg(debug_assertions)` warn-every-time branch in v0.1.12 to nudge users.
  - **-** Tree-sitter is parser-permissive: a file that tree-sitter parses cleanly is not necessarily semantically valid (e.g. Rust that uses an undefined name still parses). We document this.
- **Alternatives considered**:
  1. Keep both heuristic and tree-sitter side-by-side. Rejected: two code paths to maintain; users would have to choose between `--syntax-check=heuristic` and `--syntax-check=tree-sitter`.
  2. Always run tree-sitter. Rejected: 50-200ms per write is unacceptable for batch operations.
  3. Async pre-warm of parsers. Rejected: daemon-less design; first-use cost is amortized by tree-sitter-language-pack cache.
- **Trigger to revisit**: If we add a daemon (`atomwrite daemon --syntax-warmup`), we can make this the default and amortize the 50-200ms.
