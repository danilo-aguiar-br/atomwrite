# 0032 — G122: Real S-expression matching in `query` subcommand

> Status: Superseded in v0.1.37 for the error exit code. An invalid S-expression pattern exits 65 `INVALID_INPUT`, not exit 1 (measured with `query m.rs --query '(unclosed'`).

- Status: Accepted
- Date: 2026-06-14
- Scope: `src/commands/query.rs`, `Cargo.toml`, `tests/cli_v019_query_sexpr.rs`

## Context

The `query` subcommand (v14 Tier 3, introduced in v0.1.12) always promised
S-expression support. The documentation in `COOKBOOK.md`, `HOW_TO_USE.md`
and in the bilingual skills (`skill/atomwrite-{en,pt}/SKILL.md`) shows
examples such as:

```bash
atomwrite --workspace . query src/main.rs \
  --query "(function_item name: (identifier) @name)"
```

In practice, `cmd_query` called `walk_kind_filter`, which does
`wanted.iter().any(|w| w == &kind)` — a literal STRING comparison with
`node.kind()`. The whole kind
`"(function_item name: (identifier) @name)"` never matched any real
`node.kind()`, so the S-expression feature never worked.

`tree-sitter` 0.26.9 was already available through `tree-sitter-language-pack`
1.8. All that was missing was routing S-expression patterns to
`tree_sitter::Query::new` + `QueryCursor`.

## Decision

Add automatic S-expression detection on the pattern, and route to the
real `tree_sitter` path when it is detected:

1. New enum `QueryType { KindFilter, SExpression }` in
   `src/commands/query.rs`.
2. New function `classify_pattern(pattern) -> QueryType` that detects an
   S-expression by the presence of `(`, `)`, or `@`.
3. New function `walk_sexpr` that compiles the pattern through
   `tree_sitter::Query::new`, runs it through `QueryCursor::matches`, and emits
   NDJSON `query_match` with the extra field `capture_name` for each
   `@capture` of the pattern.
4. `cmd_query` branches on `classify_pattern` to call
   `walk_kind_filter` (legacy) or `walk_sexpr` (new).
5. Add `tree-sitter = "0.26"` as a direct dependency in
   `Cargo.toml` (the language-pack re-exports `Language`, but
   `Query`/`QueryCursor`/`StreamingIterator` require the `tree-sitter`
   crate itself).
6. `walk_sexpr` re-parses the source through a direct `tree_sitter::Parser`
   instead of the language-pack wrapper, because the `QueryCursor::matches`
   API requires `tree_sitter::Node<'_>` (not `tree_sitter_language_pack::Node`).

## Alternatives Considered
- Keep only the kind-filter (it matched the code): rejected because it
  breaks the promise documented for 6 releases and is the most useful
  `query` feature for LLM agents (capturing function/struct names).
- **Separate `--sexp` flag**: rejected because it adds a second flag
  with no real benefit, and auto-detection by structural characters
  is trivial and unambiguous.
- **Force the kind-filter to be the default for `SExpression`**: rejected
  because it would break the v0.1.12 pattern `atomwrite --query function_item`
  that is used in existing pipelines.

## Consequences
- A pattern with `(`, `)`, or `@` is now routed to `tree_sitter::Query::new`.
  S-expression parsing errors (e.g. `(unclosed`) return exit 1 with the
  message `invalid S-expression pattern: ...` (through `anyhow::Context`).
- The NDJSON response includes the `capture_name` field for each capture
  (e.g. `name` for `@name`).
- The legacy kind-filter path is preserved bit for bit: users who
  passed `--query function_item` keep receiving the same results
  as before.
- The re-parse inside `walk_sexpr` adds minimal overhead per
  call (in-memory parser, small source), acceptable for a
  query on a single file. For file batches, consider caching the
  `Query` in a future iteration (YAGNI for now).

## Validation
- 4 new tests in `tests/cli_v019_query_sexpr.rs`:
  - `sexpr_function_item_returns_main` — pattern
    `(function_item name: (identifier) @name)` matches a file with
    `fn main`.
  - `sexpr_with_capture_returns_captured_text` — the `text` field of the
    capture is exactly `"main"`.
  - `sexpr_struct_item` — pattern
    `(struct_item name: (type_identifier) @name)` extracts `"Point"`.
  - `kind_filter_still_works` — pattern `function_item` (no S-expression)
    keeps using the legacy path and does not emit `capture_name`.
- `cargo check` and `cargo build` pass with `tree-sitter = "0.26"`
  added as a dependency.
