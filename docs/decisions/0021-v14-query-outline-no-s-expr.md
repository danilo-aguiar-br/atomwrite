# ADR-0021: v14 query/outline accept only kind names, not S-expressions

- **Status**: Accepted
- **Date**: 2026-06-07
- **Context**: tree-sitter exposes two query styles: kind-name match (e.g. `function_item`) and S-expression query (e.g. `(function_item name: (identifier) @func)`). `tree-sitter-language-pack` re-exports `Node` and `TreeCursor` but does NOT re-export `tree_sitter::Query` (the S-expression parser is in a different crate feature).
- **Decision**: v0.1.12 `query` and `outline` accept only kind names. `query --kinds` enumerates kinds with counts. `query --query <KIND>` emits nodes matching that kind. `query --tree` dumps every named node. `outline --kind <KIND>` filters structural items.
- **Consequences**:
  - **+** No S-expression parser needed; the language pack API suffices.
  - **+** Users get a useful subset of AST navigation without learning tree-sitter query syntax.
  - **+** Output is stable across tree-sitter-language-pack minor versions.
  - **-** Power users cannot write `(call_expression function: (identifier) @fn)` to find function calls. They must `query --kinds` first, then filter with `rg`.
  - **-** We expose a kind-name API that may feel "limited" to AST power users.
- **Alternatives considered**:
  1. Add `tree-sitter` as a direct dep alongside the language pack. Rejected for v0.1.12 (adds a dep, more compile time, more surface to test). Consider for v0.1.13.
  2. Use `tree-sitter-cli` to pre-compile queries at build time. Rejected: requires `cc` at build time; fails on `cargo install` from source tarball.
  3. Defer the whole `query` subcommand to v0.1.13. Rejected: kind-name match is useful enough on its own; we lose momentum if we wait for S-expression support.
- **Trigger to revisit**: When users ask "how do I find all function calls?" or when the v0.1.13+ S-expression support is feasible.
